//! Background HTTP poller for the spoke-agent `/api/status` endpoint.
//!
//! Owns a small state machine that maps the agent response onto a
//! [`SpokeState`], which the tray module then renders as an icon colour
//! + menu label.

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::{watch, RwLock};

use crate::config::WidgetConfig;

/// Coarse-grained traffic-light state used by the tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpokeState {
    /// Router connected and *all* discovered services healthy.
    Connected,
    /// Router connected but at least one service is unhealthy.
    Degraded,
    /// Router not connected, or actively registering.
    Disconnected,
    /// The spoke-agent itself is unreachable (network error, refused, timeout).
    Unreachable,
    /// Initial state, before the first poll has completed.
    Unknown,
}

impl SpokeState {
    /// Human-readable label for menu items / tooltips.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Connected => "Connected",
            Self::Degraded => "Degraded",
            Self::Disconnected => "Disconnected",
            Self::Unreachable => "Agent unreachable",
            Self::Unknown => "Unknown",
        }
    }

    /// Filename (under `icons/`) of the tray-icon variant for this state.
    pub fn icon_filename(&self) -> &'static str {
        match self {
            Self::Connected => "tray-green.png",
            Self::Degraded => "tray-yellow.png",
            Self::Disconnected => "tray-red.png",
            Self::Unreachable => "tray-grey.png",
            Self::Unknown => "tray-unknown.png",
        }
    }
}

/// Subset of the spoke-agent `AgentStatus` model we actually need.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentStatusDto {
    #[serde(default)]
    pub router: RouterStatusDto,
    #[serde(default)]
    pub discovery: DiscoveryDto,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RouterStatusDto {
    #[serde(default)]
    pub connected: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DiscoveryDto {
    #[serde(default)]
    pub services: Vec<ServiceDto>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServiceDto {
    #[serde(default)]
    pub name: String,
    /// `ok | down | unreachable | unknown`
    #[serde(default)]
    pub status: String,
}

/// Pure mapping function — kept side-effect free for easy unit testing.
pub fn classify(status: &AgentStatusDto) -> SpokeState {
    if !status.router.connected {
        return SpokeState::Disconnected;
    }
    let services = &status.discovery.services;
    if services.is_empty() {
        // No discovered services yet — treat as connected, the agent is up.
        return SpokeState::Connected;
    }
    let any_bad = services.iter().any(|s| s.status != "ok");
    if any_bad {
        SpokeState::Degraded
    } else {
        SpokeState::Connected
    }
}

/// Single-shot poll. Returns the resulting [`SpokeState`].
///
/// Exposed for unit tests against `mockito`.
pub async fn poll_once(client: &reqwest::Client, cfg: &WidgetConfig) -> SpokeState {
    let mut req = client
        .get(cfg.status_url())
        .timeout(Duration::from_secs(2));
    if let Some(token) = &cfg.auth_token {
        if !token.is_empty() {
            req = req.bearer_auth(token);
        }
    }

    match req.send().await {
        Err(err) => {
            log::debug!("poll: transport error: {err}");
            SpokeState::Unreachable
        }
        Ok(resp) => {
            if !resp.status().is_success() {
                log::debug!("poll: non-2xx status {}", resp.status());
                return SpokeState::Unreachable;
            }
            match resp.json::<AgentStatusDto>().await {
                Ok(dto) => classify(&dto),
                Err(err) => {
                    log::debug!("poll: json error: {err}");
                    SpokeState::Unreachable
                }
            }
        }
    }
}

/// Holds the live state and exposes a [`watch::Receiver`] for the UI.
pub struct StatusPoller {
    tx: watch::Sender<SpokeState>,
    rx: watch::Receiver<SpokeState>,
    config: Arc<RwLock<WidgetConfig>>,
}

impl StatusPoller {
    pub fn new(config: Arc<RwLock<WidgetConfig>>) -> Self {
        let (tx, rx) = watch::channel(SpokeState::Unknown);
        Self { tx, rx, config }
    }

    /// Subscribe to state transitions.
    pub fn subscribe(&self) -> watch::Receiver<SpokeState> {
        self.rx.clone()
    }

    /// Spawns the polling loop on Tauri's async runtime.
    pub fn spawn(self: Arc<Self>) {
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
            {
                Ok(c) => c,
                Err(err) => {
                    log::error!("could not build reqwest client: {err}");
                    return;
                }
            };
            loop {
                let cfg = this.config.read().await.clone();
                let new_state = poll_once(&client, &cfg).await;
                // Ignore send errors (no receivers) — tray may not yet be ready.
                let _ = this.tx.send(new_state);
                let interval = cfg.poll_interval_s.max(5);
                tokio::time::sleep(Duration::from_secs(interval)).await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_status(connected: bool, services: Vec<(&str, &str)>) -> AgentStatusDto {
        AgentStatusDto {
            router: RouterStatusDto { connected },
            discovery: DiscoveryDto {
                services: services
                    .into_iter()
                    .map(|(n, s)| ServiceDto {
                        name: n.into(),
                        status: s.into(),
                    })
                    .collect(),
            },
        }
    }

    #[test]
    fn connected_with_all_ok_is_connected() {
        let s = make_status(true, vec![("ollama", "ok"), ("comfyui", "ok")]);
        assert_eq!(classify(&s), SpokeState::Connected);
    }

    #[test]
    fn connected_with_zero_services_is_connected() {
        let s = make_status(true, vec![]);
        assert_eq!(classify(&s), SpokeState::Connected);
    }

    #[test]
    fn connected_with_one_bad_service_is_degraded() {
        let s = make_status(true, vec![("ollama", "ok"), ("comfyui", "unreachable")]);
        assert_eq!(classify(&s), SpokeState::Degraded);
    }

    #[test]
    fn router_disconnected_is_disconnected_even_if_services_ok() {
        let s = make_status(false, vec![("ollama", "ok")]);
        assert_eq!(classify(&s), SpokeState::Disconnected);
    }

    #[test]
    fn icon_filename_changes_per_state() {
        assert_ne!(
            SpokeState::Connected.icon_filename(),
            SpokeState::Disconnected.icon_filename()
        );
        assert_ne!(
            SpokeState::Degraded.icon_filename(),
            SpokeState::Unreachable.icon_filename()
        );
    }

    #[tokio::test]
    async fn poll_once_reads_valid_json() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/status")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "spoke_name":"test","spoke_tags":[],"version":"0","uptime_s":1,
                    "discovery":{"capabilities":[],"services":[
                        {"name":"ollama","status":"ok","type":"llm","base_url":"http://x","capabilities":[]}
                    ]},
                    "router":{"url":"x","connected":true,"app_id":"a"}
                }"#,
            )
            .create_async()
            .await;

        let cfg = WidgetConfig {
            agent_url: server.url(),
            ..WidgetConfig::default()
        };
        let client = reqwest::Client::new();
        let state = poll_once(&client, &cfg).await;
        assert_eq!(state, SpokeState::Connected);
    }

    #[tokio::test]
    async fn poll_once_on_connection_refused_is_unreachable() {
        let cfg = WidgetConfig {
            // port 1 is reserved + nothing listens => connection refused.
            agent_url: "http://127.0.0.1:1".into(),
            ..WidgetConfig::default()
        };
        let client = reqwest::Client::new();
        let state = poll_once(&client, &cfg).await;
        assert_eq!(state, SpokeState::Unreachable);
    }

    #[tokio::test]
    async fn poll_once_on_5xx_is_unreachable() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/status")
            .with_status(500)
            .create_async()
            .await;

        let cfg = WidgetConfig {
            agent_url: server.url(),
            ..WidgetConfig::default()
        };
        let client = reqwest::Client::new();
        let state = poll_once(&client, &cfg).await;
        assert_eq!(state, SpokeState::Unreachable);
    }
}
