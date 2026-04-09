//! Vis desktop shell — spawns a local `opencode serve`, waits for it to come
//! up, then injects its URL into the bundled Vue 3 SPA via the existing
//! `window.vis-api-base` meta tag convention (see `app/composables/useCredentials.ts`).
//!
//! The shell is intentionally minimal: it does not proxy HTTP; instead the
//! renderer talks directly to `http://127.0.0.1:<port>` over fetch/SSE/WebSocket.
//! opencode is launched with `--cors tauri://localhost https://tauri.localhost`
//! so the browser-level CORS check passes on Linux/macOS and Windows.

use std::sync::Mutex;

use tauri::{Manager, RunEvent, WebviewWindow};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// Shared state: the running opencode child (Some until we've asked it to
/// stop) and the URL it's listening on (None until we've parsed its stdout).
#[derive(Default)]
struct OpencodeState {
    child: Option<CommandChild>,
    url: Option<String>,
}

/// Extract the URL from an opencode "server listening" line such as:
///   "opencode server listening on http://127.0.0.1:40431"
fn parse_listening_url(line: &str) -> Option<String> {
    let idx = line.find("http://")?;
    let rest = &line[idx..];
    let end = rest
        .find(|c: char| c.is_whitespace())
        .unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

/// Inject `window.__VIS_API_BASE__` and the `<meta name="vis-api-base">` tag
/// into the current document so `useCredentials` skips the login screen and
/// points at our locally-spawned opencode. Runs via `eval` on the webview.
fn inject_api_base(window: &WebviewWindow, url: &str) {
    let script = format!(
        r#"(() => {{
    try {{
        const u = {url};
        window.__VIS_API_BASE__ = u;
        let meta = document.querySelector('meta[name="vis-api-base"]');
        if (!meta) {{
            meta = document.createElement('meta');
            meta.setAttribute('name', 'vis-api-base');
            document.head.appendChild(meta);
        }}
        meta.setAttribute('content', u);
        // Ensure the SPA picks up the new value if it already read an empty one.
        if (typeof window.dispatchEvent === 'function') {{
            window.dispatchEvent(new Event('vis:api-base-changed'));
        }}
    }} catch (err) {{
        console.error('Vis shell: failed to inject api-base', err);
    }}
}})();"#,
        url = serde_json::to_string(url).unwrap_or_else(|_| "\"\"".to_string()),
    );
    if let Err(err) = window.eval(&script) {
        eprintln!("[vis] failed to eval inject script: {err}");
    }
}

/// Spawn `opencode serve` on a random free port and pipe stdout/stderr into
/// the shell's own stderr so users can see what's happening.
fn spawn_opencode(app: &tauri::AppHandle) -> Result<(), String> {
    let shell = app.shell();
    // --port=0 lets opencode pick a free port.
    // --cors lines make fetch/SSE/WebSocket from tauri://localhost pass CORS.
    let cmd = shell
        .command("opencode")
        .args([
            "serve",
            "--hostname=127.0.0.1",
            "--port=0",
            "--cors",
            "tauri://localhost",
            "--cors",
            "https://tauri.localhost",
        ]);

    let (mut rx, child) = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn opencode: {e}"))?;

    {
        let state = app.state::<Mutex<OpencodeState>>();
        let mut guard = state.lock().expect("OpencodeState poisoned");
        guard.child = Some(child);
    }

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(bytes) | CommandEvent::Stderr(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    eprint!("[opencode] {text}");

                    // Try to parse the listening URL from each line.
                    for line in text.lines() {
                        if line.contains("server listening") {
                            if let Some(url) = parse_listening_url(line) {
                                eprintln!("[vis] opencode is up at {url}");
                                on_opencode_ready(&app_handle, url);
                                break;
                            }
                        }
                    }
                }
                CommandEvent::Terminated(payload) => {
                    eprintln!("[vis] opencode terminated: {payload:?}");
                    // Take the app down with it if we're not already shutting down.
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.close();
                    }
                }
                _ => {}
            }
        }
    });

    Ok(())
}

/// Called when we have finally parsed the opencode URL from its stdout.
/// Persists it into state, injects it into the (possibly-already-loaded)
/// webview, and reveals the main window which we create hidden.
fn on_opencode_ready(app: &tauri::AppHandle, url: String) {
    {
        let state = app.state::<Mutex<OpencodeState>>();
        let mut guard = state.lock().expect("OpencodeState poisoned");
        guard.url = Some(url.clone());
    }

    if let Some(window) = app.get_webview_window("main") {
        inject_api_base(&window, &url);
        if let Err(err) = window.show() {
            eprintln!("[vis] failed to show window: {err}");
        }
    }
}

/// Tauri command exposed to the frontend: returns the current opencode URL,
/// or an empty string if it isn't ready yet. The SPA can fall back to this
/// via `invoke('get_api_base')` when the meta tag hasn't been injected yet
/// (e.g. Vite dev server which rebuilds index.html on every HMR).
#[tauri::command]
fn get_api_base(state: tauri::State<'_, Mutex<OpencodeState>>) -> String {
    state
        .lock()
        .ok()
        .and_then(|g| g.url.clone())
        .unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(OpencodeState::default()))
        .invoke_handler(tauri::generate_handler![get_api_base])
        .setup(|app| {
            // Kick off opencode in the background. The window is created hidden
            // and only revealed once we've parsed the listening URL, so the
            // user never sees the login form flash before auto-auth kicks in.
            let handle = app.handle().clone();
            if let Err(err) = spawn_opencode(&handle) {
                eprintln!("[vis] {err}");
                eprintln!("[vis] make sure 'opencode' is installed and on PATH");
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                }
            }

            // If opencode takes too long to come up, reveal the window anyway
            // after 8 seconds so the user isn't left staring at nothing.
            let handle_for_timeout = handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(8));
                if let Some(window) = handle_for_timeout.get_webview_window("main") {
                    if !window.is_visible().unwrap_or(true) {
                        eprintln!("[vis] opencode slow to start, showing window anyway");
                        let _ = window.show();
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                // Kill the opencode child on app exit. tauri_plugin_shell's
                // Child::kill() sends SIGTERM on POSIX and TerminateProcess
                // on Windows.
                //
                // Note: `state.lock()` produces a MutexGuard that borrows from
                // the `State<'_, _>` returned by `state::<>()`. We scope both
                // tightly in a block so the temporary from `state::<>()` is
                // dropped together with the guard at the end of the block.
                let state_handle = app_handle.state::<Mutex<OpencodeState>>();
                let child = match state_handle.lock() {
                    Ok(mut guard) => guard.child.take(),
                    Err(_) => None,
                };
                if let Some(child) = child {
                    if let Err(err) = child.kill() {
                        eprintln!("[vis] failed to kill opencode: {err}");
                    }
                }
            }
        });
}
