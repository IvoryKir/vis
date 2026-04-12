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

#[cfg(not(dev))]
use tauri::ipc::CapabilityBuilder;
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

/// Resolve the path to the `opencode` binary.
///
/// On macOS, GUI apps launched via Finder/Dock/Spotlight get a minimal PATH
/// (typically just `/usr/bin:/bin:/usr/sbin:/sbin`). User-installed tools in
/// `~/.opencode/bin`, `~/.local/bin`, or Homebrew/nix paths are invisible.
/// We probe well-known locations first, then fall back to bare `"opencode"`.
fn resolve_opencode_binary(home: &str) -> String {
    use std::path::PathBuf;

    let candidates = [
        PathBuf::from(home).join(".opencode/bin/opencode"),
        PathBuf::from(home).join(".local/bin/opencode"),
        PathBuf::from("/usr/local/bin/opencode"),
        PathBuf::from("/opt/homebrew/bin/opencode"),  // Apple Silicon Homebrew
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin/opencode"),
    ];

    for path in &candidates {
        if path.is_file() {
            return path.display().to_string();
        }
    }

    // Fall back to bare name — works if opencode is on PATH (e.g. Linux
    // terminal launch, or user has configured their environment properly).
    "opencode".to_string()
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

    // Resolve the opencode binary path. On macOS, GUI apps launched from
    // Finder/Dock inherit a minimal PATH that doesn't include ~/.opencode/bin
    // or paths added in .zshrc/.bash_profile. We check well-known locations
    // and fall back to bare "opencode" (hoping it's on PATH).
    let opencode_bin = resolve_opencode_binary(&home);
    eprintln!("[vis] using opencode binary: {opencode_bin}");

    // On Linux, wrap the spawn in `setsid` so opencode becomes the leader
    // of a new session/pgroup. This is the workaround for sst/opencode#20899:
    // `opencode serve` does not forward SIGTERM to its children.
    // On macOS, `setsid` doesn't exist — we spawn opencode directly and
    // rely on killpg with the child's PID as pgroup leader (macOS creates a
    // new pgroup for each child by default when spawned from a GUI app).
    #[cfg(target_os = "linux")]
    let cmd = shell
        .command("setsid")
        .args([
            opencode_bin.as_str(),
            "serve",
            "--hostname=127.0.0.1",
            "--port=0",
            "--cors",
            cors_origin.as_str(),
        ])
        .current_dir(&home);

    #[cfg(not(target_os = "linux"))]
    let cmd = shell
        .command(&opencode_bin)
        .args([
            "serve",
            "--hostname=127.0.0.1",
            "--port=0",
            "--cors",
            cors_origin.as_str(),
        ])
        .current_dir(&home);

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
    eprintln!("[vis] spawned opencode pid={pid}");

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

/// Install a `.desktop` file and icon into `~/.local/share/` so that Linux
/// desktop environments (GNOME, KDE, etc.) can resolve the app icon via
/// `WM_CLASS` / `app_id`.
///
/// AppImages are self-contained and don't install anything into the system,
/// so without this the DE shows a generic fallback icon.
#[cfg(target_os = "linux")]
fn install_desktop_entry() {
    use std::fs;
    use std::path::PathBuf;

    let Some(home) = std::env::var_os("HOME") else { return };
    let home = PathBuf::from(home);

    // --- Install icon ---
    let icon_dir = home.join(".local/share/icons/hicolor/128x128/apps");
    let _ = fs::create_dir_all(&icon_dir);
    let icon_path = icon_dir.join("vis-desktop.png");
    // Always overwrite so updates to the icon are picked up.
    let _ = fs::write(&icon_path, include_bytes!("../icons/128x128.png"));

    // --- Install .desktop file ---
    let apps_dir = home.join(".local/share/applications");
    let _ = fs::create_dir_all(&apps_dir);
    let desktop_path = apps_dir.join("vis-desktop.desktop");

    // Resolve our own executable path for the Exec= line.
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "vis-desktop".to_string());

    let desktop_content = format!(
        "[Desktop Entry]\n\
         Name=Vis\n\
         Comment=Beautiful OpenCode UI\n\
         Exec={exe}\n\
         Icon=vis-desktop\n\
         Terminal=false\n\
         Type=Application\n\
         Categories=Development;\n\
         StartupWMClass=vis-desktop\n"
    );
    let _ = fs::write(&desktop_path, desktop_content);
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK compositing/DMA-BUF performance workaround.
    // Paradoxically, disabling HW-accelerated compositing often *improves*
    // rendering performance in WebKitGTK — confirmed across Tauri, Wails,
    // GitButler and dozens of GitHub issues. The DMA-BUF renderer has known
    // bugs with certain GPU drivers and causes high CPU usage even on AMD/Intel.
    // See: tauri-apps/tauri#9394, gitbutlerapp/gitbutler#11602, wry#890
    #[cfg(target_os = "linux")]
    {
        // SAFETY: called before any other threads are spawned.
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
    // Pick the port for the embedded frontend HTTP server.
    //
    // CRITICAL: We use a FIXED preferred port so that WebKitGTK (Linux)
    // stores localStorage in the same SQLite file between app restarts.
    // WebKitGTK keys localStorage by origin — `http://localhost:<port>` —
    // so if the port changes each run, the user's theme, settings, and
    // session tabs are lost every time.
    //
    // If the preferred port is busy (e.g. two instances running), we
    // fall back to a random one. State won't persist in that second
    // instance, but at least it won't crash.
    const PREFERRED_FRONTEND_PORT: u16 = 14321;
    let frontend_port: u16 = if portpicker::is_free_tcp(PREFERRED_FRONTEND_PORT) {
        PREFERRED_FRONTEND_PORT
    } else {
        eprintln!(
            "[vis] preferred frontend port {} is busy, falling back to random",
            PREFERRED_FRONTEND_PORT
        );
        portpicker::pick_unused_port()
            .expect("failed to find an unused TCP port for the Vis frontend")
    };

    // On Linux (Wayland/X11) the desktop environment resolves the app icon
    // from a .desktop file matched by WM_CLASS / app_id. AppImages are
    // self-contained and do NOT install .desktop files into the system,
    // so GNOME/KDE/etc. fall back to a generic gear icon.
    //
    // Fix: on first run, install a .desktop file + icon into
    // ~/.local/share/applications and ~/.local/share/icons so the DE
    // can find and display our real icon.
    #[cfg(target_os = "linux")]
    install_desktop_entry();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_localhost::Builder::new(frontend_port).build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(Mutex::new(OpencodeState::default()))
        .invoke_handler(tauri::generate_handler![get_api_base])
        .setup(move |app| {
            // Create the main window pointing at the localhost-served frontend.
            // Window is hidden until opencode is up so the user never sees a
            // flash of the login form before auto-auth kicks in.
            let url: tauri::Url = format!("http://localhost:{frontend_port}")
                .parse()
                .expect("failed to build localhost url");

            // In release mode the frontend is served over http://localhost:<port>
            // which Tauri treats as a *remote* origin. We must explicitly grant
            // it the same IPC capabilities as the built-in tauri:// scheme,
            // otherwise `invoke()` calls and event listeners are silently blocked.
            // See: https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/localhost/README.md
            #[cfg(not(dev))]
            app.add_capability(
                CapabilityBuilder::new("localhost-remote")
                    .remote(url.to_string())
                    .window("main"),
            )?;

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.clone()))
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/128x128.png"))?)?
                .title("Vis \u{2014} OpenCode Visualizer")
                .inner_size(1400.0, 900.0)
                .min_inner_size(900.0, 600.0)
                .resizable(true)
                .decorations(true)
                .center()
                .visible(false)
                .build()?;

            // Disable HW acceleration via WebKitGTK settings API.
            // Environment variables (WEBKIT_DISABLE_COMPOSITING_MODE) are unreliable
            // when the webview is created via plugin-localhost. Using with_webview
            // gives us direct access to the underlying webkit2gtk::WebView.
            #[cfg(target_os = "linux")]
            {
                use webkit2gtk::{SettingsExt, WebViewExt};
                let main_window = app.get_webview_window("main").expect("main window");
                main_window.with_webview(move |webview| {
                    let wv = webview.inner();
                    if let Some(settings) = WebViewExt::settings(&wv) {
                        settings.set_hardware_acceleration_policy(
                            webkit2gtk::HardwareAccelerationPolicy::Never,
                        );
                        // Also enable page cache for faster back/forward
                        settings.set_enable_page_cache(true);
                        eprintln!("[vis] WebKitGTK: HW accel=NEVER, page_cache=ON");
                    }
                }).ok();
            }

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
