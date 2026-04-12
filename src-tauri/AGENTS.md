# src-tauri/ — Tauri 2 Desktop Shell

Thin Rust wrapper for desktop deployment. All business logic in Vue/TypeScript — Rust only handles process lifecycle.

## FILES

| File | Lines | Role |
|------|-------|------|
| src/main.rs | 6 | Entry point, delegates to `lib.rs::run()` |
| src/lib.rs | 555 | Tauri app: spawn opencode, inject URL, signal handling |
| src/process_group.rs | ~60 | POSIX process group kill via `/proc` walk |
| tauri.conf.json | 34 | App config: CSP, bundle targets (AppImage, deb) |
| Cargo.toml | ~40 | Deps: tauri 2, plugin-shell, plugin-localhost, signal-hook |
| build.rs | 3 | Standard Tauri build script |

## CRITICAL CONSTRAINTS

- **Port 14321 is hardcoded** — WebKitGTK keys localStorage by origin; changing port loses all user state between restarts
- **Uses `tauri-plugin-localhost`** not `tauri://` protocol — WebKitGTK blocks `fetch()` to `http://127.0.0.1` from custom protocol origins
- **Process group kill** — `setsid` on spawn, kill `-pid` on shutdown; opencode doesn't forward SIGTERM to children (sst/opencode#20899)
- **`windows_subsystem = "windows"`** in main.rs — DO NOT REMOVE, prevents console window on Windows

## BOOTSTRAP FLOW

```
main.rs → lib.rs::run()
  → spawn: opencode serve --hostname=127.0.0.1 --port=0
  → parse listening URL from stdout (regex)
  → inject URL: window.__VIS_API_BASE__ + <meta name="vis-api-base">
  → serve frontend via tauri-plugin-localhost:14321
  → install signal handlers (Unix: SIGINT/SIGTERM/SIGHUP → kill process group)
  → on exit: SIGTERM process group → wait 3s → SIGKILL fallback
```

## TODO

- Windows signal handler (`SetConsoleCtrlHandler`) not implemented — see `#[cfg(not(unix))]` block in lib.rs
- Desktop builds only target Linux (AppImage, deb) — no Windows/macOS bundles yet

## BUILD

```bash
pnpm tauri:dev       # Dev mode (Vite HMR + Tauri window)
pnpm tauri:build     # Release: NO_STRIP=true tauri build → src-tauri/target/release/bundle/
```

Release profile: LTO enabled, panic=abort, opt-level=s (size), strip=true in Cargo.
