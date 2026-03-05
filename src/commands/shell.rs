//! External shell command execution: opens the command in a new terminal split/pane
//! using the current terminal emulator or multiplexer.

use std::path::Path;
use std::process::Stdio;

/// Detect the running terminal/multiplexer and open `cmd` in a new horizontal
/// split/pane with `cwd` as the working directory.
///
/// Returns `Ok(())` if the split was launched (fire-and-forget), or an error
/// string if no supported environment was detected.
pub fn open_in_split(cmd: &str, cwd: &Path) -> Result<(), String> {
    let cwd_str = cwd.to_str().unwrap_or(".");
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    // ── 1. Zellij ────────────────────────────────────────────────────────────
    if std::env::var("ZELLIJ").is_ok() || std::env::var("ZELLIJ_SESSION_NAME").is_ok() {
        // new-pane down keeps the layout sane; --hold keeps the pane open after exit
        return fire(&[
            "zellij", "action", "new-pane", "--direction", "down",
            "--cwd", cwd_str, "--", &shell, "-i", "-c", cmd,
        ]);
    }

    // ── 2. Tmux ───────────────────────────────────────────────────────────────
    if std::env::var("TMUX").is_ok() {
        // -h = horizontal split (side by side); -c = start directory
        let cmd_str = format!("{} -i -c {}", shell, shlex_quote(cmd));
        return fire(&["tmux", "split-window", "-h", "-c", cwd_str, &cmd_str]);
    }

    // ── 3. Kitty ──────────────────────────────────────────────────────────────
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        // Requires allow_remote_control yes (or kitty started with --listen-on)
        return fire(&[
            "kitty", "@", "launch",
            "--type=horizontal",
            &format!("--cwd={}", cwd_str),
            "--hold",
            &shell, "-i", "-c", cmd,
        ]);
    }

    // ── 4. WezTerm ────────────────────────────────────────────────────────────
    if std::env::var("WEZTERM_UNIX_SOCKET").is_ok()
        || std::env::var("WEZTERM_PANE").is_ok()
    {
        return fire(&[
            "wezterm", "cli", "split-pane",
            "--horizontal",
            "--cwd", cwd_str,
            "--", &shell, "-i", "-c", cmd,
        ]);
    }

    // ── 5. Ghostty ───────────────────────────────────────────────────────────
    if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok()
        || matches!(std::env::var("TERM_PROGRAM").as_deref(), Ok("ghostty"))
    {
        return fire(&[
            "ghostty",
            &format!("+split-right={}:{}", cwd_str, cmd),
        ])
        .or_else(|_| {
            // Newer Ghostty CLI format
            fire(&["ghostty", "+open", "--split", "--command", cmd, "--working-directory", cwd_str])
        });
    }

    // ── 6. iTerm2 (macOS) ────────────────────────────────────────────────────
    if std::env::var("ITERM_SESSION_ID").is_ok() {
        let script = format!(
            r#"tell application "iTerm2"
                tell current window
                    tell current session of current tab
                        split horizontally with default profile
                    end tell
                    tell second session of current tab
                        write text "cd {cwd} && {cmd}"
                    end tell
                end tell
            end tell"#,
            cwd = cwd_str,
            cmd = cmd.replace('"', "\\\""),
        );
        return fire(&["osascript", "-e", &script]);
    }

    // ── 7. macOS Terminal.app ─────────────────────────────────────────────────
    if matches!(std::env::var("TERM_PROGRAM").as_deref(), Ok("Apple_Terminal")) {
        let script = format!(
            r#"tell application "Terminal"
                do script "cd {cwd} && {cmd}"
                activate
            end tell"#,
            cwd = cwd_str,
            cmd = cmd.replace('"', "\\\""),
        );
        return fire(&["osascript", "-e", &script]);
    }

    // ── 8. Linux fallback: try common terminal emulators ──────────────────────
    // gnome-terminal
    if command_exists("gnome-terminal") {
        let exec_cmd = format!("{}; exec {}", cmd, shell);
        return fire(&[
            "gnome-terminal", "--working-directory", cwd_str,
            "--", &shell, "-i", "-c", &exec_cmd,
        ]);
    }
    // konsole (KDE)
    if command_exists("konsole") {
        return fire(&[
            "konsole", "--workdir", cwd_str,
            "-e", &shell, "-i", "-c", cmd,
        ]);
    }
    // foot (Wayland, common on sway/river)
    if command_exists("foot") {
        return fire(&["foot", "--working-directory", cwd_str, &shell, "-i", "-c", cmd]);
    }
    // xterm (universal fallback)
    if command_exists("xterm") {
        let exec_cmd = format!("cd {} && {}; read -p 'Press Enter...'", cwd_str, cmd);
        return fire(&["xterm", "-e", &shell, "-i", "-c", &exec_cmd]);
    }

    Err("No supported terminal emulator or multiplexer detected.\n\
         Supported: zellij, tmux, kitty, wezterm, ghostty, iTerm2, \
         Terminal.app, gnome-terminal, konsole, foot, xterm.".to_string())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Spawn `args[0]` with `args[1..]`, detached (no wait, no output capture).
fn fire(args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        return Err("empty command".to_string());
    }
    std::process::Command::new(args[0])
        .args(&args[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to launch `{}`: {}", args[0], e))
}

/// Minimal shell quoting: wrap in single quotes, escaping any `'` inside.
fn shlex_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Check whether `cmd` exists on `PATH`.
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
