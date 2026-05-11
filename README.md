# spoke-widget

[![build](https://github.com/janpow77/spoke-widget/actions/workflows/build.yml/badge.svg)](https://github.com/janpow77/spoke-widget/actions/workflows/build.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Cross-platform system-tray frontend for the [spoke-agent](https://github.com/janpow77/spoke-agent),
written in **Tauri 2** (Rust + WebView). Runs on Linux, Windows and macOS.

The widget is intentionally _thin_: it shows a single tray icon whose colour
reflects the live status of your local Spoke-Stack and gives you one-click
access to the spoke-agent dashboard. The full management UI continues to
live inside the spoke-agent (`http://localhost:7700/admin/`).

## Architecture

```
+----------------------+        HTTP polling          +-----------------+
| spoke-widget (tray)  | --- GET  /api/status -----> | spoke-agent     |
|  (Tauri 2 / Rust)    | --- POST /api/services/... |  :7700          |
+----------------------+                              +--------+--------+
                                                              |
                                                              | registers
                                                              v
                                                      +-----------------+
                                                      |  llm-router     |
                                                      |  :7100 / :7101  |
                                                      +-----------------+
```

* The widget polls `GET /api/status` every 10/30/60 s (configurable).
* `router.connected` plus the per-service `status` field drive the tray
  icon colour:
  * green — router connected and *all* discovered services healthy
  * yellow — router connected but at least one service is degraded
  * red — router not connected
  * grey — spoke-agent itself unreachable (network error, refused, timeout)
  * `?` — initial state, no poll completed yet
* Left-click on the tray icon (or "Open Dashboard") opens
  `http://localhost:7700/admin/` in the user's default browser.

## Tray menu

| Item                  | Action                                                                |
| --------------------- | --------------------------------------------------------------------- |
| Open Dashboard        | Default action. Opens `<agent>/admin/` in the system browser.         |
| Status: \<text\>      | Read-only info item, reflects the last poll result.                   |
| Restart Spoke-Stack   | POST `<agent>/api/services/all/restart` (falls back per-service).     |
| Pause Heartbeat       | DELETE `<agent>/api/router/heartbeat` (toggled).                      |
| Settings…             | Opens a small Vue 3 settings window.                                  |
| About…                | Opens this repo on GitHub.                                            |
| Quit                  | Terminates the widget.                                                |

## Settings

Persisted to `~/.config/spoke-widget/config.json` (Linux),
`%APPDATA%\spoke-widget\config.json` (Windows) and
`~/Library/Application Support/spoke-widget/config.json` (macOS).

| Field                | Default                  | Description                              |
| -------------------- | ------------------------ | ---------------------------------------- |
| `agent_url`          | `http://localhost:7700`  | Spoke-Agent base URL.                    |
| `auth_token`         | _(none)_                 | Optional bearer for `SPOKE_AGENT_AUTH_TOKEN`. |
| `poll_interval_s`    | `30`                     | 10 / 30 / 60.                            |
| `autostart`          | `true`                   | Launch on user login.                    |
| `theme`              | `system`                 | `light` / `dark` / `system`.             |

Auto-start is wired through `tauri-plugin-autostart`
(LaunchAgent on macOS, Registry `Run` on Windows, `.desktop` autostart on Linux).

## Install

### Linux (Debian/Ubuntu)

```bash
sudo apt install ./spoke-widget_0.1.0_amd64.deb
```

…or use the portable AppImage:

```bash
chmod +x spoke-widget_0.1.0_amd64.AppImage
./spoke-widget_0.1.0_amd64.AppImage
```

> On GNOME, you may need the
> [AppIndicator and KStatusNotifierItem Support](https://extensions.gnome.org/extension/615/appindicator-support/)
> extension for the tray icon to appear.

### Windows

Install the `spoke-widget_0.1.0_x64-setup.exe` or the `.msi`. The installer
is **unsigned** in 0.x — Windows SmartScreen will warn; choose
"More info" → "Run anyway". A signed build is planned for v1.0.

### macOS

Mount `spoke-widget_0.1.0_aarch64.dmg` (or `_x86_64`) and drag the app to
`/Applications`. The build is **unsigned and unnotarised**, so the first
launch requires:

```bash
xattr -dr com.apple.quarantine /Applications/Spoke\ Widget.app
```

## Build from source

Requirements:

* Rust **1.77+** (Tauri 2 minimum)
* Node **18+** (for the optional Vue settings UI)
* Platform build deps — Tauri lists them per OS:
  [https://v2.tauri.app/start/prerequisites/](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/janpow77/spoke-widget
cd spoke-widget

# install UI deps
npm --prefix ui install

# dev (hot-reload)
cargo install tauri-cli --version "^2.0" --locked
cargo tauri dev

# release bundle
cargo tauri build
```

The release artefacts land under `src-tauri/target/release/bundle/`.

### Regenerating the icons

The bundled tray + app icons are generated by `scripts/gen_icons.py`
(needs Python 3 + Pillow). Re-run after changing colours:

```bash
python3 scripts/gen_icons.py
```

## Testing

```bash
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

The unit tests exercise:

* The `SpokeState` classifier (router-up + services-mix combinations).
* The `status_poller` HTTP layer (against `mockito`).
* The `restart_all` fallback to per-service restarts on 404.
* The bearer-token plumbing.
* The `MockOpener` (proxy for the production `tauri-plugin-shell` opener).

## Troubleshooting

* **Tray says "Agent unreachable"** — verify the spoke-agent is listening
  on the URL configured in Settings (`curl http://localhost:7700/api/status`).
  If the agent requires an auth token, set it in Settings.
* **Tray icon doesn't appear on GNOME** — install the AppIndicator
  extension (see above).
* **`Restart Spoke-Stack` fails with 404** — the widget will fall back to
  looping over `/api/services/{name}/restart`. If that also 404s, your
  spoke-agent is older than the widget expects.
* **Heartbeat-toggle menu greyed-out / errors** — the
  `DELETE /api/router/heartbeat` endpoint is part of the spoke-agent
  roadmap but not yet shipped in every release; the widget reports a
  clear error if it's missing.

## Roadmap

* v0.2: signed builds (macOS notarisation + Windows code-signing) and
  a built-in updater (`tauri-plugin-updater`).
* v0.3: richer per-service tray submenu (logs tail, individual restart).
* v0.4: tray badge with GPU memory used (from `discovery.gpu`).

## License

[MIT](LICENSE) © 2026 Jan Riener
