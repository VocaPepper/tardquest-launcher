# TQ Launcher

A cross-platform (Windows + Linux) launcher for **[TardQuest](https://github.com/packardbell95/tardquest)** and **[TardQuest Online](https://tardquest.online)**,
built with [Tauri v2](https://tauri.app) (Rust backend, Vanilla TypeScript + Vite frontend).

## Prerequisites

- Rust (stable) and the [Tauri CLI](https://tauri.app/start/prerequisites/)
- Node.js 20+ and [pnpm](https://pnpm.io)
- System webview: WebView2 (Windows) / WebKitGTK (Linux)

## Development

```bash
pnpm install
cargo tauri dev
```

## Build

```bash
cargo tauri build               # installers + updater artifacts (needs signing env vars)
cargo tauri build --no-bundle   # just the executable
```

Built executables are written to `src-tauri/target/release/`.

For signed updater artifacts, set `TAURI_SIGNING_PRIVATE_KEY` (or `_PATH`) and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` and enable the updater in `src-tauri/tauri.conf.json`.

## Project layout

- `src/` — frontend (TypeScript, Vite)
- `src-tauri/` — Rust backend and Tauri configuration
- `catalog.json` — edition → source configuration (GitHub repo / TQO update endpoint)
