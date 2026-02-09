# CLAUDE.md

## Project Overview

Live Scratch is a Tauri v2 desktop app (macOS only) that provides bi-directional live sync between workspace files and the Scratch editor. Edit `project.json` directly with a text editor or AI agent, and see changes reflected instantly in the Scratch editor.

## Architecture

```
[Text Editor / AI Agent]
    ↕ edit files in ~/Documents/Live Scratch/
[Rust Backend (Tauri v2)]
    workspace.rs  — SB3 build/extract, CLAUDE.md deployment
    watcher.rs    — File watching (notify crate), emits sb3-updated events
    commands.rs   — Tauri IPC commands
    lib.rs        — App init, menu, watcher startup
    ↕ Tauri IPC (window.__TAURI__)
[Frontend: Scratch GUI + live-reload.js]
    ↕ vm.loadProject / vm.saveProjectSb3
[Scratch Editor (WebView)]
```

## Key Files

- `src-tauri/src/` — Rust backend
- `client/live-reload.js` — Frontend script injected into Scratch GUI (communicates via Tauri IPC)
- `default-project/` — Template files copied to workspace on first launch (includes CLAUDE.md)
- `setup.sh` — Setup script that clones, patches, and builds scratch-editor
- `scratch-editor/` — Scratch GUI built by setup.sh (gitignored)

## Development

```bash
npm install          # Builds scratch-editor (setup.sh runs automatically)
npm run tauri:dev    # Run in development mode
npm run tauri:build  # Release build (generates .dmg)
```

## Tauri v2 Notes

- `withGlobalTauri: true` goes in the `app` section, NOT `build`
- CSP requires `'unsafe-eval'` and `'unsafe-inline'` for Scratch GUI
- In dev mode, `resource_dir()` points to `target/debug/`, so fall back to `CARGO_MANIFEST_DIR`
- SB3 data is transferred as base64 strings via Tauri invoke/events

## Workspace

- Path: `~/Documents/Live Scratch/`
- On launch, template files are copied from `default-project/` (only if `project.json` does not exist)
- `CLAUDE.md` is overwritten on every launch to stay current
- `.md` files are excluded from SB3 builds and ignored by the file watcher

## Scratch Program Editing

The Scratch project files (`project.json` and asset files such as SVG, WAV, PNG, etc.) are located in `~/Documents/Live Scratch`. When given instructions about Scratch programs, directly edit the files in that folder. Do not use the Scratch GUI or any external tools — modify the files in place.
