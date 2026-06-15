# Architektur — spoke-widget

_Automatisch generiert von graphify-kira aus dem Code-Graphen. Nicht von Hand editieren — wird beim nächsten Lauf überschrieben._

**Umfang:** 217 Knoten, 310 Kanten, 12 größere Module, 0 zirkuläre Abhängigkeiten.

## Modulkarte

- **Service Status Monitoring** (35): `status_poller.rs`
- **Media Asset Management** (25): `tauri.conf.json`
- **Tauri App Configuration** (21): `tauri.conf.json`
- **UI Component Handling** (19): `tray.rs`
- **Widget Configuration** (18): `config.rs`
- **URL Opening Mechanism** (18): `opener.rs`
- **Vue Project Setup** (17): `package.json`
- **API Client Management** (15): `agent_actions.rs`
- **TypeScript Config** (15): `tsconfig.json`
- **App Configuration** (14): `lib.rs`
- **Icon Generation** (6): `gen_icons.py`
- **App Permissions** (6): `default.json`

## Zentrale Bausteine (God Nodes)

_Hohe Zentralität ist nicht automatisch ein Defekt (zentrale Stores/Modelle sind oft legitim). Konkrete Refactoring-Prioritäten siehe Optimierungs-Report._

- `make_status() (src-tauri/src/status_poller.rs)` — Grad 7 (ein 5/aus 2)
- `restart_all() (src-tauri/src/agent_actions.rs)` — Grad 10 (ein 4/aus 6)
- `status_poller.rs (src-tauri/src/status_poller.rs)` — Grad 20 (ein 0/aus 20)
- `Image (scripts/gen_icons.py)` — Grad 2 (ein 2/aus 0)
- `AgentStatusDto (src-tauri/src/status_poller.rs)` — Grad 5 (ein 3/aus 2)
- `poll_once() (src-tauri/src/status_poller.rs)` — Grad 9 (ein 5/aus 4)
- `compilerOptions (ui/tsconfig.json)` — Grad 13 (ein 1/aus 12)
- `SpokeState (src-tauri/src/status_poller.rs)` — Grad 7 (ein 5/aus 2)
- `Self (src-tauri/src/config.rs)` — Grad 2 (ein 2/aus 0)
- `.default() (src-tauri/src/config.rs)` — Grad 5 (ein 4/aus 1)

## Schnittstellen / Brücken (Betweenness)

- `poll_once() (src-tauri/src/status_poller.rs)` — Betweenness 0.001
- `AgentStatusDto (src-tauri/src/status_poller.rs)` — Betweenness 0.001
- `restart_all() (src-tauri/src/agent_actions.rs)` — Betweenness 0.001
- `bundle (src-tauri/tauri.conf.json)` — Betweenness 0.001
- `SpokeState (src-tauri/src/status_poller.rs)` — Betweenness 0.000
- `classify() (src-tauri/src/status_poller.rs)` — Betweenness 0.000
- `make_status() (src-tauri/src/status_poller.rs)` — Betweenness 0.000
- `compilerOptions (ui/tsconfig.json)` — Betweenness 0.000
- `.new() (src-tauri/src/status_poller.rs)` — Betweenness 0.000
- `windows (src-tauri/tauri.conf.json)` — Betweenness 0.000

## Empfohlene Spezialisten

Passend zu Stack/Domäne dieses Projekts (Claude-Code-Agents/Skills):

`/deutsche-formulierung`, `@git-workflow`, `/auto-verify`, `@e2e-browser-tester`, `/modern-gui-builder`, `/ux-completeness-check`, `/vue3-gui-builder`, `@rust-expert`, `/understanding-tauri-architecture`, `/setting-up-tauri-projects`, `/understanding-tauri-ipc`, `/calling-rust-from-tauri-frontend`.

## Hinweis für Änderungen

Vor dem Ändern eines zentralen Bausteins die Abhängigen prüfen — am schnellsten über den **graphify-MCP** (globaler Graph): „Was hängt an `<datei>`?". Brücken-Knoten stabil halten.

