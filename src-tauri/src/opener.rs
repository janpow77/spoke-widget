//! Cross-platform "open URL in default browser" helper.
//!
//! Backed by `tauri-plugin-shell` v2's `opener` API. Wrapped behind a small
//! trait so unit tests can substitute a mock implementation.

use anyhow::{Context, Result};
use std::sync::Mutex;

/// Abstraction over "open an URL". Implemented by [`TauriShellOpener`] in
/// production and by [`MockOpener`] in unit tests.
pub trait UrlOpener: Send + Sync {
    fn open(&self, url: &str) -> Result<()>;
}

/// Production implementation: uses `tauri-plugin-shell` to honour the user's
/// default browser on all three desktop OSes.
pub struct TauriShellOpener {
    handle: tauri::AppHandle,
}

impl TauriShellOpener {
    pub fn new(handle: tauri::AppHandle) -> Self {
        Self { handle }
    }
}

impl UrlOpener for TauriShellOpener {
    fn open(&self, url: &str) -> Result<()> {
        use tauri_plugin_shell::ShellExt;
        // `Shell::open` is marked deprecated in tauri-plugin-shell 2.1+ in
        // favour of `tauri-plugin-opener`, but it is still the documented
        // path for the shell plugin and we have no reason to pull in a
        // second plugin for a one-liner.
        #[allow(deprecated)]
        self.handle
            .shell()
            .open(url, None)
            .with_context(|| format!("opening URL {url}"))?;
        Ok(())
    }
}

/// Test-only opener that records every URL it was asked to open.
#[cfg(test)]
pub struct MockOpener {
    pub calls: Mutex<Vec<String>>,
}

#[cfg(test)]
impl MockOpener {
    pub fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl UrlOpener for MockOpener {
    fn open(&self, url: &str) -> Result<()> {
        self.calls.lock().unwrap().push(url.to_string());
        Ok(())
    }
}

// Silence unused-import warnings of `Mutex` for non-test builds, since we
// only use it inside the `#[cfg(test)]` MockOpener above.
#[allow(dead_code)]
fn _unused_mutex_ref() -> Option<Mutex<()>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_opener_records_calls() {
        let m = MockOpener::new();
        m.open("http://localhost:7700/admin/").unwrap();
        m.open("https://github.com/janpow77/spoke-widget").unwrap();
        let calls = m.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], "http://localhost:7700/admin/");
        assert!(calls[1].starts_with("https://github.com"));
    }
}
