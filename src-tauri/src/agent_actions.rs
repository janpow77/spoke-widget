//! Thin HTTP-wrapper around a subset of spoke-agent endpoints used by
//! the tray menu (restart-all, heartbeat-toggle, …).

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;

use crate::config::WidgetConfig;

fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("building reqwest client")
}

fn authed(req: reqwest::RequestBuilder, cfg: &WidgetConfig) -> reqwest::RequestBuilder {
    match cfg.auth_token.as_deref() {
        Some(t) if !t.is_empty() => req.bearer_auth(t),
        _ => req,
    }
}

/// Restart every service known to the agent (`POST /api/services/all/restart`).
///
/// The spoke-agent does not (yet) expose this composite endpoint, so we
/// fall back to looping over `/api/services` if a 404 comes back. The
/// helper is best-effort: any error is reported back to the caller.
pub async fn restart_all(cfg: &WidgetConfig) -> Result<String> {
    let client = build_client()?;
    let url = format!(
        "{}/api/services/all/restart",
        cfg.agent_url.trim_end_matches('/')
    );
    let resp = authed(client.post(&url), cfg)
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;

    if resp.status().is_success() {
        return Ok(format!("restart-all OK ({})", resp.status()));
    }
    if resp.status().as_u16() == 404 {
        return restart_all_fallback(cfg).await;
    }
    Err(anyhow!(
        "restart-all failed: HTTP {} from {}",
        resp.status(),
        url
    ))
}

async fn restart_all_fallback(cfg: &WidgetConfig) -> Result<String> {
    let client = build_client()?;
    let list_url = format!("{}/api/services", cfg.agent_url.trim_end_matches('/'));
    let services: Vec<serde_json::Value> = authed(client.get(&list_url), cfg)
        .send()
        .await
        .with_context(|| format!("GET {list_url}"))?
        .error_for_status()?
        .json()
        .await
        .context("parsing /api/services")?;

    let mut restarted = Vec::new();
    for svc in services {
        let Some(name) = svc.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let restart_url = format!(
            "{}/api/services/{}/restart",
            cfg.agent_url.trim_end_matches('/'),
            name
        );
        let resp = authed(client.post(&restart_url), cfg)
            .send()
            .await
            .with_context(|| format!("POST {restart_url}"))?;
        if resp.status().is_success() {
            restarted.push(name.to_string());
        }
    }
    Ok(format!(
        "restart-all (fallback) restarted {} services",
        restarted.len()
    ))
}

/// Toggle the router heartbeat by issuing DELETE /api/router/heartbeat.
///
/// If the spoke-agent does not implement this endpoint (404), we surface
/// a clear error instead of failing silently.
pub async fn toggle_heartbeat(cfg: &WidgetConfig) -> Result<String> {
    let client = build_client()?;
    let url = format!(
        "{}/api/router/heartbeat",
        cfg.agent_url.trim_end_matches('/')
    );
    let resp = authed(client.delete(&url), cfg)
        .send()
        .await
        .with_context(|| format!("DELETE {url}"))?;
    if resp.status().is_success() {
        Ok(format!("heartbeat toggled ({})", resp.status()))
    } else if resp.status().as_u16() == 404 {
        Err(anyhow!(
            "spoke-agent does not expose /api/router/heartbeat (got 404). Upgrade the agent or remove this menu item."
        ))
    } else {
        Err(anyhow!("heartbeat toggle failed: HTTP {}", resp.status()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn restart_all_uses_composite_endpoint_when_available() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/api/services/all/restart")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;
        let cfg = WidgetConfig {
            agent_url: server.url(),
            ..WidgetConfig::default()
        };
        let res = restart_all(&cfg).await.unwrap();
        assert!(res.contains("restart-all OK"));
    }

    #[tokio::test]
    async fn restart_all_falls_back_to_individual_endpoints_on_404() {
        let mut server = mockito::Server::new_async().await;
        let _composite = server
            .mock("POST", "/api/services/all/restart")
            .with_status(404)
            .create_async()
            .await;
        let _list = server
            .mock("GET", "/api/services")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"[{"name":"ollama","base_url":"http://x","capabilities":[]}]"#)
            .create_async()
            .await;
        let _restart = server
            .mock("POST", "/api/services/ollama/restart")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;

        let cfg = WidgetConfig {
            agent_url: server.url(),
            ..WidgetConfig::default()
        };
        let res = restart_all(&cfg).await.unwrap();
        assert!(res.contains("restarted 1"));
    }

    #[tokio::test]
    async fn toggle_heartbeat_reports_404_clearly() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("DELETE", "/api/router/heartbeat")
            .with_status(404)
            .create_async()
            .await;
        let cfg = WidgetConfig {
            agent_url: server.url(),
            ..WidgetConfig::default()
        };
        let err = toggle_heartbeat(&cfg).await.unwrap_err();
        assert!(err.to_string().contains("does not expose"));
    }

    #[tokio::test]
    async fn auth_token_sets_bearer_header() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/api/services/all/restart")
            .match_header("authorization", "Bearer hunter2")
            .with_status(200)
            .with_body("{}")
            .create_async()
            .await;
        let cfg = WidgetConfig {
            agent_url: server.url(),
            auth_token: Some("hunter2".into()),
            ..WidgetConfig::default()
        };
        let res = restart_all(&cfg).await.unwrap();
        assert!(res.contains("OK"));
    }
}
