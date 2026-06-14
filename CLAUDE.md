# spoke-widget

Cross-Platform-System-Tray-Frontend für den [spoke-agent](https://github.com/janpow77/spoke-agent).
Ein bewusst dünnes Tauri-2-Widget: ein Tray-Icon, dessen Farbe den Live-Status des
lokalen Spoke-Stack spiegelt, plus Ein-Klick-Zugang zum spoke-agent-Dashboard
(`http://localhost:7844/admin/`). Die eigentliche Management-UI lebt im spoke-agent.

## Tech-Stack

- **Backend / App:** Rust 2021 (rust-version 1.77+), Tauri 2 (`tray-icon`, `image-png`)
- **Tauri-Plugins:** shell, autostart, notification, dialog, os
- **HTTP:** reqwest 0.12 (rustls-tls), async via tokio (multi-thread)
- **Settings-UI:** Vue 3 + TypeScript, gebaut mit Vite 5 (`vue-tsc`)
- **Tests:** `cargo test` + `mockito` (HTTP-Mocking)
- **Icon-Generierung:** Python 3 + Pillow (`scripts/gen_icons.py`)
- **Kommuniziert mit:** spoke-agent auf Port `7844` (`GET /api/status`,
  `POST /api/services/.../restart`, `DELETE /api/router/heartbeat`)

## Setup & Befehle

Alle Tauri-Befehle aus dem Repo-Root ausführen (die `beforeBuildCommand`
`npm --prefix ui` setzt das voraus).

```bash
# UI-Dependencies installieren
npm --prefix ui install

# Dev (Hot-Reload) — benötigt tauri-cli
cargo install tauri-cli --version "^2.0" --locked
cargo tauri dev

# Release-Bundle (Artefakte unter src-tauri/target/release/bundle/)
cargo tauri build

# Tray-/App-Icons neu generieren
python3 scripts/gen_icons.py
```

UI-Skripte (`ui/package.json`): `npm --prefix ui run dev` / `build` (`vue-tsc --noEmit && vite build`) / `preview`.

Tests & Lint (CI führt dieselben aus, `working-directory: src-tauri`):

```bash
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

## Struktur

```
Cargo.toml            Workspace-Root (members = ["src-tauri"], Release-Profil)
src-tauri/            Rust-Crate (lib: spoke_widget_lib, bin: spoke-widget)
  Cargo.toml          Crate-Manifest + Dependencies
  tauri.conf.json     Tauri-Config (identifier io.janpow.spokewidget, Tray, Settings-Window)
  capabilities/       Tauri-Permissions (default.json)
  icons/              App-Icons + Tray-Varianten (green/yellow/red/grey/unknown)
  build.rs            tauri-build
  src/
    main.rs           Thin Binary, ruft spoke_widget_lib::run()
    lib.rs            run(): Logging, Config, Poller, Tauri-Builder; Commands get_config/save_config/open_dashboard
    status_poller.rs  Background-Poller für GET /api/status; SpokeState-Klassifizierer + StatusPoller
    agent_actions.rs  Agent-Aktionen: restart_all() (Fallback per-Service), Heartbeat-Toggle
    tray.rs           Tray-Setup + Menü; apply_state() rendert SpokeState als Icon/Label
    config.rs         WidgetConfig (agent_url, auth_token, poll_interval_s, autostart, theme) + Persistenz
    opener.rs         Opener-Abstraktion; MockOpener für Tests
ui/                   Vue-3-Settings-Window (src/App.vue, main.ts), Vite + TS
  package.json, vite.config.ts, tsconfig.json
scripts/gen_icons.py  Icon-Generator (Pillow)
graphify-out/         Code-Graph-Analyse (graph.json/html)
.github/workflows/    build.yml (lint+test+matrix-build), release.yml
```

Zentrale Module (nach Graph-Degree): `status_poller.rs`, `agent_actions.rs`,
`config.rs` (`WidgetConfig`), `lib.rs` (`setup()`), `tray.rs` (`apply_state()`).

## Konventionen

- **Sprache:** Code, Doc-Kommentare und UI sind englisch (Repo-Konvention).
- **Format:** `cargo fmt` (rustfmt), erzwungen im CI.
- **Lint:** `cargo clippy --all-targets -- -D warnings` (Warnings = Fehler).
- **Tests:** `cargo test --all`; HTTP-Layer gegen `mockito`, Opener über `MockOpener`.
- **CI-Branch:** `master` (build.yml triggert auf push/PR gegen `master`).
- **Lizenz:** MIT.
