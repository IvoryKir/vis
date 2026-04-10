//! Reliable "kill opencode and everything it spawned" helpers.
//!
//! The `tauri-plugin-shell` `CommandChild::kill()` sends a signal to the root
//! process only (SIGKILL on Unix). That's not enough for us because
//! `opencode serve` spawns a Bun/Node runtime and several worker processes of
//! its own: MCP servers, PTY shells, LSP servers, etc. Killing only the root
//! leaves those descendants orphaned under init, which is exactly how we
//! ended up with nine zombie `opencode` processes after a single debug
//! session.
//!
//! Strategy (POSIX):
//!   1. Look up the child's process-group id (PGID) via `/proc/<pid>/stat`.
//!   2. If `PGID == PID`, opencode is its own process-group leader. Sending
//!      `killpg(PGID, SIGKILL)` wipes the leader AND every descendant that
//!      stayed in the group, which is the typical case for server daemons.
//!   3. If `PGID != PID` (rare: happens when opencode was spawned without
//!      `setsid`/`setpgid`), we fall back to walking `/proc/*/stat` and
//!      collecting every descendant PID transitively, then SIGKILLing each.
//!
//! Windows is a no-op here: we rely on `CommandChild::kill()` via Tauri,
//! which uses `TerminateProcess`. A process-tree kill there needs
//! `JobObject` + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, which is out of
//! scope until we actually ship a Windows build.

#[cfg(unix)]
pub fn kill_tree(root_pid: u32) {
    use std::thread;
    use std::time::Duration;

    // Keep this binding for clarity — it documents that we interpret
    // root_pid as a signed pid (i32) for the libc calls below. Prefixed
    // with _ to satisfy the dead_code lint.
    let _pid = root_pid as i32;

    // Fast path: opencode is its own pgroup leader → killpg hits everything.
    if let Some(pgid) = read_pgid(root_pid) {
        if pgid as u32 == root_pid {
            unsafe {
                // SIGTERM first so well-behaved descendants get a chance to
                // clean up (flushing logs, removing sockets, etc).
                libc::killpg(pgid, libc::SIGTERM);
            }
            // Give them a brief grace period before the hammer.
            thread::sleep(Duration::from_millis(300));
            unsafe {
                libc::killpg(pgid, libc::SIGKILL);
            }
            eprintln!("[vis] killpg({pgid}, SIGKILL) sent for opencode group");
            return;
        }
        eprintln!(
            "[vis] opencode pid {root_pid} is not its own pgroup leader (pgid={pgid}); \
             falling back to /proc descendant walk"
        );
    } else {
        eprintln!(
            "[vis] could not read /proc/{root_pid}/stat; falling back to /proc descendant walk"
        );
    }

    // Slow path: walk /proc and collect every descendant.
    let mut victims = collect_descendants(root_pid);
    // Kill descendants first (bottom-up) so we don't race with reparenting
    // to the leftover root. Parent last.
    victims.push(root_pid);
    for pid in victims.iter().rev() {
        unsafe {
            libc::kill(*pid as i32, libc::SIGTERM);
        }
    }
    thread::sleep(Duration::from_millis(300));
    for pid in victims.iter().rev() {
        unsafe {
            libc::kill(*pid as i32, libc::SIGKILL);
        }
    }
    eprintln!(
        "[vis] /proc-walk SIGKILL sent to {} processes (opencode tree)",
        victims.len()
    );
}

#[cfg(not(unix))]
pub fn kill_tree(_root_pid: u32) {
    // Windows / other: handled by CommandChild::kill() upstream for now.
}

/// Parse PGID from `/proc/<pid>/stat`.
///
/// The stat file looks like:
///   "1234 (opencode) S 1200 1234 1200 ..."
/// Field 5 (1-indexed) is pgrp. We can't just split by whitespace because
/// the comm field (field 2) may contain spaces/parens. Find the last ')'
/// to skip past comm, then split the tail.
#[cfg(unix)]
fn read_pgid(pid: u32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = stat.rfind(')')?;
    let tail = stat[close + 1..].trim();
    let mut parts = tail.split_whitespace();
    let _state = parts.next()?; // field 3
    let _ppid = parts.next()?; // field 4
    let pgid = parts.next()?; // field 5
    pgid.parse().ok()
}

/// BFS through `/proc` collecting every descendant of `root_pid`.
/// Returns pids ordered roughly parent→child (so callers can reverse for
/// leaf-first signalling).
#[cfg(unix)]
fn collect_descendants(root_pid: u32) -> Vec<u32> {
    use std::collections::HashMap;

    // Build ppid → Vec<pid> map by scanning /proc once.
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        let Ok(pid) = name_str.parse::<u32>() else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let Some(close) = stat.rfind(')') else {
            continue;
        };
        let tail = stat[close + 1..].trim();
        let mut parts = tail.split_whitespace();
        let Some(_state) = parts.next() else {
            continue;
        };
        let Some(ppid_str) = parts.next() else {
            continue;
        };
        let Ok(ppid) = ppid_str.parse::<u32>() else {
            continue;
        };
        children.entry(ppid).or_default().push(pid);
    }

    // BFS from root.
    let mut out = Vec::new();
    let mut queue = vec![root_pid];
    while let Some(parent) = queue.pop() {
        if let Some(kids) = children.get(&parent) {
            for kid in kids {
                out.push(*kid);
                queue.push(*kid);
            }
        }
    }
    out
}
