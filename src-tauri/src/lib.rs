//! Vis desktop shell — spawns a local `opencode serve`, waits for it to come
//! up, then injects its URL into the bundled Vue 3 SPA via the existing
//! `window.vis-api-base` meta tag convention (see `app/composables/useCredentials.ts`).
//!
//! The frontend is served through `tauri-plugin-localhost` on a random
//! loopback port, NOT via the default `tauri://localhost` custom protocol.
//! The reason: in production on Linux, WebKitGTK treats the custom protocol
//! as a sandboxed origin where `fetch` to `http://127.0.0.1:*` randomly
//! fails with `TypeError: Load failed`, and SharedWorkers inherit an even
//! more restricted context where `fetch` never works at all. Serving the
//! frontend itself over `http://localhost:<port>` puts both the main thread
//! and any SharedWorker into a standard HTTP security context, which makes
//! both `fetch` and the existing SSE SharedWorker architecture work the
//! same as they do in dev mode.

use std::sync::Mutex;

use tauri::{webview::WebviewWindowBuilder, Manager, RunEvent, WebviewUrl, WebviewWindow};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

mod process_group;

/// Bookkeeping for one spawned `opencode serve` process.
struct ChildInfo {
    /// Handle from `tauri-plugin-shell`; kept for `.kill()` fallback and to
    /// prevent the child from being accidentally reaped while still in use.
    child: CommandChild,
    /// OS pid of the opencode root process. Cached separately because
    /// `CommandChild::kill(self)` consumes the handle, but we still need
    /// the pid for `killpg`/proc-walk during shutdown even after `.kill()`
    /// was already called.
    pid: u32,
    /// URL opencode is listening on (`http://127.0.0.1:<port>`), populated
    /// once we parse it from the child's stdout. `None` means the spawn
    /// hasn't surfaced its listening line yet.
    url: Option<String>,
}

/// Shared state: every `opencode serve` we've spawned during this session.
/// A `Vec` rather than a single slot because Phase 2 will add multi-project
/// support where each project gets its own opencode instance — and we must
/// be able to shut them all down cleanly on exit.
#[derive(Default)]
struct OpencodeState {
    children: Vec<ChildInfo>,
}

impl OpencodeState {
    /// Returns the URL of the first opencode instance, or empty string if
    /// none are ready yet. Used by the `get_api_base` command so the SPA
    /// can recover the URL after Vite HMR reloads in dev mode.
    fn primary_url(&self) -> String {
        self.children
            .iter()
            .find_map(|info| info.url.clone())
            .unwrap_or_default()
    }
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
    // Tell the frontend where opencode is listening via both a DOM meta tag
    // and a window global. The SPA's `useCredentials` composable reads these
    // at boot and skips the login screen entirely when they are present.
    // We also dispatch `vis:api-base-changed` in case the SPA had already
    // booted with an empty value before opencode was ready.
    let script = format!(
        r#"(() => {{
    try {{
        const u = {url};
        window.__VIS_API_BASE__ = u;
        let meta = document.querySelector('meta[name=\"vis-api-base\"]');
        if (!meta) {{
            meta = document.createElement('meta');
            meta.setAttribute('name', 'vis-api-base');
            document.head.appendChild(meta);
        }}
        meta.setAttribute('content', u);
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
///
/// `frontend_port` is the port on which the Vis SPA is served by
/// `tauri-plugin-localhost`. We pass it to opencode as the `--cors` allowed
/// origin so the browser's preflight for `Authorization` / `x-opencode-directory`
/// headers succeeds. Without this, loopback-to-loopback still works on
/// WebKitGTK today, but stricter browsers / future webview engines would
/// block the cross-origin request.
fn spawn_opencode(app: &tauri::AppHandle, frontend_port: u16) -> Result<(), String> {
    let shell = app.shell();
    let cors_origin = format!("http://localhost:{frontend_port}");
    // Use $HOME (or fallback to /) as the working directory. When launched
    // as an AppImage the inherited cwd is the squashfs mount point inside
    // /tmp, which confuses opencode (no `.git`, wrong project detection).
    // The actual project directory is communicated per-request by the SPA
    // via the `x-opencode-directory` header, so the cwd only matters for
    // opencode's own config lookup and default project resolution.
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    // Wrap the spawn in `setsid` so opencode becomes the leader of a brand-
    // new session (and therefore a new process group). This is the workaround
    // for sst/opencode#20899: `opencode serve` does not forward SIGTERM to
    // its children (MCP / LSP / bash), and Bun doesn't implement the
    // `detached` child_process option. `setsid` creates a new session, then
    // execs opencode — so the PID we get from `child.pid()` is setsid's PID,
    // which becomes opencode's PID after exec. The session (and pgroup) is
    // this same PID, meaning `killpg(pid, SIGKILL)` wipes the entire tree.
    let cmd = shell
        .command("setsid")
        .args([
            "opencode",
            "serve",
            "--hostname=127.0.0.1",
            "--port=0",
            "--cors",
            cors_origin.as_str(),
        ])
        .current_dir(home);

    let (mut rx, child) = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn opencode: {e}"))?;
    let pid = child.pid();

    {
        let state = app.state::<Mutex<OpencodeState>>();
        let mut guard = state.lock().expect("OpencodeState poisoned");
        guard.children.push(ChildInfo {
            child,
            pid,
            url: None,
        });
    }
    eprintln!("[vis] spawned opencode pid={pid} (via setsid)");

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
                                eprintln!("[vis] opencode pid={pid} is up at {url}");
                                on_opencode_ready(&app_handle, pid, url);
                                break;
                            }
                        }
                    }
                }
                CommandEvent::Terminated(payload) => {
                    eprintln!("[vis] opencode pid={pid} terminated: {payload:?}");
                    // Remove from state so we don't try to kill a dead pid
                    // during shutdown (would just log a benign ESRCH).
                    let state_handle = app_handle.state::<Mutex<OpencodeState>>();
                    if let Ok(mut guard) = state_handle.lock() {
                        guard.children.retain(|info| info.pid != pid);
                    }
                    // If this was the last opencode, take the app down with
                    // it — the UI can't do anything useful without a backend.
                    let last_one = app_handle
                        .state::<Mutex<OpencodeState>>()
                        .lock()
                        .map(|g| g.children.is_empty())
                        .unwrap_or(true);
                    if last_one {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.close();
                        }
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
/// Called when we have parsed the opencode URL from its stdout. Persists it
/// on the matching `ChildInfo`, injects it into the (possibly already
/// loaded) webview, and reveals the main window which we create hidden.
fn on_opencode_ready(app: &tauri::AppHandle, pid: u32, url: String) {
    {
        let state = app.state::<Mutex<OpencodeState>>();
        let mut guard = state.lock().expect("OpencodeState poisoned");
        if let Some(info) = guard.children.iter_mut().find(|i| i.pid == pid) {
            info.url = Some(url.clone());
        }
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
        .map(|g| g.primary_url())
        .unwrap_or_default()
}

/// Shut down every opencode child this process has spawned, using the
/// reliable process-group / descendant-walk kill implemented in
/// `process_group::kill_tree`. Idempotent: calling it twice is safe, the
/// second call just finds an empty state vector.
fn shutdown_all_opencode(app_handle: &tauri::AppHandle) {
    // Drain the Vec under the lock, then drop the guard so the kill syscalls
    // below don't block anything else that might be waiting on the state.
    // Note: `state::<>()` returns a `State<'_, _>` that borrows from
    // `app_handle`. Its temporary extends to the end of the enclosing
    // expression, which in a match block includes the block's scope —
    // that trips the borrow checker if we also hold the `MutexGuard`
    // through the match. We defeat that by capturing the taken Vec into
    // a local `x` and making `x` the last expression so the `State`
    // temporary can drop before the outer `let children = ...` binding.
    let children: Vec<ChildInfo> = {
        let state_handle = app_handle.state::<Mutex<OpencodeState>>();
        let x = match state_handle.lock() {
            Ok(mut guard) => std::mem::take(&mut guard.children),
            Err(poisoned) => {
                eprintln!("[vis] OpencodeState mutex poisoned; recovering");
                std::mem::take(&mut poisoned.into_inner().children)
            }
        };
        x
    };
    if children.is_empty() {
        return;
    }
    eprintln!(
        "[vis] shutting down {} opencode instance(s)",
        children.len()
    );
    for info in children {
        // 1) Group-wide SIGTERM+SIGKILL via /proc-aware helper. This is what
        //    actually reaches every descendant opencode forked (Bun runtime,
        //    MCP servers, PTY shells, LSP processes, …).
        process_group::kill_tree(info.pid);
        // 2) Belt-and-braces: also call the original CommandChild::kill().
        //    kill_tree above may have already reaped the root via killpg, in
        //    which case this logs a harmless ESRCH. But if /proc was unusable
        //    (sandboxes, containers, WSL1) this is the one that actually
        //    stops the root process.
        if let Err(err) = info.child.kill() {
            eprintln!("[vis] CommandChild::kill fallback for pid={}: {err}", info.pid);
        }
    }
}

/// Install POSIX signal handlers so `Ctrl+C` / `kill <vis-pid>` / session
/// logout still drag opencode down with us. Without this, sending SIGINT
/// from the terminal where `vis-desktop` was launched leaves orphan
/// opencode processes because Tauri's `RunEvent::ExitRequested` never fires
/// on signal-induced exit.
#[cfg(unix)]
fn install_signal_handlers(app_handle: tauri::AppHandle) {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    use signal_hook::iterator::Signals;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // `signal_hook::iterator::Signals::new` installs process-wide `sigaction`
    // handlers that also block the signal in the main thread until we fetch
    // it from this dedicated background thread. This is the combination the
    // earlier hand-rolled `pthread_sigmask + sigwait` version got wrong:
    // pthread_sigmask only affects the calling thread, so SIGTERM raced past
    // our wait loop and hit Tauri's default handler instead. Using
    // `signal_hook` fixes this and is the pattern the docs explicitly
    // recommend for multi-threaded applications.
    let mut signals = match Signals::new([SIGINT, SIGTERM, SIGHUP]) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("[vis] failed to install signal handlers: {err}");
            return;
        }
    };

    let already_shutting_down = Arc::new(AtomicBool::new(false));
    std::thread::spawn(move || {
        for signum in signals.forever() {
            if already_shutting_down.swap(true, Ordering::SeqCst) {
                // Second term signal — user is impatient, just die.
                std::process::exit(130);
            }
            eprintln!("[vis] received signal {signum}, shutting down opencode");
            shutdown_all_opencode(&app_handle);
            // Ask Tauri to exit cleanly so window close and other cleanup
            // callbacks run normally.
            app_handle.exit(0);
            // If Tauri doesn't terminate within a reasonable window, fall
            // back to a hard exit so we never leave the process hanging.
            std::thread::sleep(std::time::Duration::from_millis(800));
            std::process::exit(128 + signum);
        }
    });
}

#[cfg(not(unix))]
fn install_signal_handlers(_app_handle: tauri::AppHandle) {
    // TODO: SetConsoleCtrlHandler on Windows when we ship a Windows build.
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Pick a random free port for the embedded frontend HTTP server. We use
    // the `portpicker` crate because `tauri-plugin-localhost::Builder::new`
    // wants a concrete u16, not a "pick one for me" option. This port serves
    // the bundled Vue SPA to the webview — opencode itself runs on its own
    // separate random port, chosen by opencode when we spawn it.
    let frontend_port: u16 = portpicker::pick_unused_port()
        .expect("failed to find an unused TCP port for the Vis frontend");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_localhost::Builder::new(frontend_port).build())
        .manage(Mutex::new(OpencodeState::default()))
        .invoke_handler(tauri::generate_handler![get_api_base])
        .setup(move |app| {
            // Create the main window pointing at the localhost-served frontend.
            // Window is hidden until opencode is up so the user never sees a
            // flash of the login form before auto-auth kicks in.
            let url: tauri::Url = format!("http://localhost:{frontend_port}")
                .parse()
                .expect("failed to build localhost url");
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("Vis \u{2014} OpenCode Visualizer")
                .inner_size(1400.0, 900.0)
                .min_inner_size(900.0, 600.0)
                .resizable(true)
                .decorations(true)
                .center()
                .visible(false)
                .build()?;

            // Install signal handlers on POSIX so Ctrl+C in the launching
            // terminal or a `kill` from the OS takes opencode down with us.
            install_signal_handlers(app.handle().clone());

            // Kick off opencode in the background. The window is revealed
            // only once we've parsed its listening URL.
            let handle = app.handle().clone();
            if let Err(err) = spawn_opencode(&handle, frontend_port) {
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
            match event {
                // Fired when the user closes the last window OR when we call
                // `app_handle.exit(0)` ourselves from the signal thread.
                RunEvent::ExitRequested { .. } | RunEvent::Exit => {
                    shutdown_all_opencode(app_handle);
                }
                _ => {}
            }
        });
}
