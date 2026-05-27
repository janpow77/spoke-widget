//! Persistent widget configuration.
//!
//! Stored as JSON under the platform-appropriate config dir
//! (e.g. `~/.config/spoke-widget/config.json` on Linux).

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WidgetConfig {
    /// Base URL of the spoke-agent.
    pub agent_url: String,
    /// Optional bearer token (spoke-agent's `SPOKE_AGENT_AUTH_TOKEN`).
    #[serde(default)]
    pub auth_token: Option<String>,
    /// Polling interval in seconds. Allowed: 10 / 30 / 60.
    pub poll_interval_s: u64,
    /// Auto-start with system login.
    pub autostart: bool,
    /// Theme (forwarded to the settings WebView).
    pub theme: Theme,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            agent_url: "http://localhost:7844".to_string(),
            auth_token: None,
            poll_interval_s: 30,
            autostart: true,
            theme: Theme::System,
        }
    }
}

impl WidgetConfig {
    /// Returns the canonical config file path.
    pub fn path() -> Result<PathBuf> {
        let base = dirs::config_dir().context("could not resolve user config dir")?;
        Ok(base.join("spoke-widget").join("config.json"))
    }

    /// Loads the config from disk, or returns defaults if the file does not exist.
    pub fn load() -> Result<Self> {
        let p = Self::path()?;
        if !p.exists() {
            return Ok(Self::default());
        }
        let raw =
            fs::read_to_string(&p).with_context(|| format!("reading config {}", p.display()))?;
        let cfg: Self = serde_json::from_str(&raw)
            .with_context(|| format!("parsing config {}", p.display()))?;
        Ok(cfg)
    }

    /// Persists the config to disk, creating parent dirs as needed.
    pub fn save(&self) -> Result<()> {
        let p = Self::path()?;
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating config dir {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(self).context("serialising config")?;
        fs::write(&p, raw).with_context(|| format!("writing config {}", p.display()))?;
        Ok(())
    }

    /// Returns the `/api/status` URL for the configured agent.
    pub fn status_url(&self) -> String {
        format!("{}/api/status", self.agent_url.trim_end_matches('/'))
    }

    /// Returns the `/admin/` dashboard URL.
    pub fn dashboard_url(&self) -> String {
        format!("{}/admin/", self.agent_url.trim_end_matches('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_urls_are_well_formed() {
        let cfg = WidgetConfig::default();
        assert_eq!(cfg.status_url(), "http://localhost:7844/api/status");
        assert_eq!(cfg.dashboard_url(), "http://localhost:7844/admin/");
    }

    #[test]
    fn trailing_slash_is_normalised() {
        let cfg = WidgetConfig {
            agent_url: "http://example.com:7700/".into(),
            ..WidgetConfig::default()
        };
        assert_eq!(cfg.status_url(), "http://example.com:7700/api/status");
        assert_eq!(cfg.dashboard_url(), "http://example.com:7700/admin/");
    }

    #[test]
    fn roundtrip_serde() {
        let cfg = WidgetConfig {
            agent_url: "http://nuc.local:7700".into(),
            auth_token: Some("secret".into()),
            poll_interval_s: 60,
            autostart: false,
            theme: Theme::Dark,
        };
        let s = serde_json::to_string(&cfg).unwrap();
        let back: WidgetConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(cfg, back);
    }
}
