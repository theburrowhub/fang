//! External shell command execution: opens the command in a new vertical
//! split/pane (left|right) using the current terminal emulator or multiplexer.

use std::path::Path;
use std::process::Stdio;

/// Detect the running terminal/multiplexer and open `cmd` in a new **vertical**
/// split (side by side, left | right) with `cwd` as the working directory.
///
/// "Vertical split" = a vertical dividing line = left pane + right pane.
/// Note: naming conventions differ per terminal (see comments below).
///
/// Returns `Ok(())` on success, or an `Err(String)` describing the failure.
pub fn open_in_split(cmd: &str, cwd: &Path) -> Result<(), String> {
    let cwd_str = cwd.to_str().unwrap_or(".");
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    // Wrap cmd so aliases (defined in ~/.zshrc / ~/.bashrc) are available.
    let shell_cmd = format!("cd {} && {} -i -c {}", cwd_str, shell, shlex_quote(cmd));

    // ── 1. Zellij ────────────────────────────────────────────────────────────
    // "right" = add a new pane to the RIGHT → vertical divider, side by side.
    if std::env::var("ZELLIJ").is_ok() || std::env::var("ZELLIJ_SESSION_NAME").is_ok() {
        return fire(&[
            "zellij", "action", "new-pane",
            "--direction", "right",
            "--cwd", cwd_str,
            "--", &shell, "-i", "-c", cmd,
        ]);
    }

    // ── 2. Tmux ───────────────────────────────────────────────────────────────
    // In tmux: -h (horizontal flag) = split into left/right panes (vertical divider).
    // This is tmux's confusing naming: "-h" means the panes sit side by side.
    if std::env::var("TMUX").is_ok() {
        return fire(&[
            "tmux", "split-window", "-h",
            "-c", cwd_str,
            &shell_cmd,
        ]);
    }

    // ── 3. Kitty ──────────────────────────────────────────────────────────────
    // "hsplit" = kitty creates a vertical dividing line → panes are LEFT | RIGHT.
    // Requires: `allow_remote_control yes` in kitty.conf (or --listen-on at launch).
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return fire(&[
            "kitty", "@", "launch",
            "--type=hsplit",               // vertical divider, panes side by side
            &format!("--cwd={}", cwd_str),
            "--hold",                      // keep pane open after command exits
            &shell, "-i", "-c", cmd,
        ])
        .map_err(|e| format!(
            "{}\nHint: add 'allow_remote_control yes' to ~/.config/kitty/kitty.conf", e
        ));
    }

    // ── 4. WezTerm ────────────────────────────────────────────────────────────
    // --horizontal = adds a vertical dividing line → panes are left | right.
    if std::env::var("WEZTERM_UNIX_SOCKET").is_ok()
        || std::env::var("WEZTERM_PANE").is_ok()
    {
        return fire(&[
            "wezterm", "cli", "split-pane",
            "--horizontal",                // = vertical divider (WezTerm naming)
            "--cwd", cwd_str,
            "--", &shell, "-i", "-c", cmd,
        ]);
    }

    // ── 5. Ghostty ───────────────────────────────────────────────────────────
    if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok()
        || matches!(std::env::var("TERM_PROGRAM").as_deref(), Ok("ghostty"))
    {
        // Try the IPC approach first, fall back to new window
        return fire(&[
            "ghostty", "action", "new-split",
            "--direction", "right",
            "--command", &shell_cmd,
        ])
        .or_else(|_| fire(&[
            "ghostty", "action", "new-window",
            "--command", &shell_cmd,
        ]));
    }

    // ── 6. iTerm2 (macOS) ────────────────────────────────────────────────────
    // "split vertically" in AppleScript = adds a VERTICAL dividing line → LEFT | RIGHT.
    // We save the new session reference so we can write the command to it directly.
    if std::env::var("ITERM_SESSION_ID").is_ok() {
        let escaped_cmd = cmd.replace('\\', "\\\\").replace('"', "\\\"");
        let escaped_cwd = cwd_str.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            r#"tell application "iTerm2"
                activate
                tell current window
                    tell current session of current tab
                        set newSession to (split vertically with default profile)
                        tell newSession
                            write text "cd \"{cwd}\" && {cmd}"
                        end tell
                    end tell
                end tell
            end tell"#,
            cwd = escaped_cwd,
            cmd = escaped_cmd,
        );
        return fire(&["osascript", "-e", &script]);
    }

    // ── 7. macOS Terminal.app ─────────────────────────────────────────────────
    // Terminal.app has no split API; open a new window running the command.
    if matches!(std::env::var("TERM_PROGRAM").as_deref(), Ok("Apple_Terminal")) {
        let escaped_cmd = cmd.replace('\\', "\\\\").replace('"', "\\\"");
        let escaped_cwd = cwd_str.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            r#"tell application "Terminal"
                do script "cd \"{cwd}\" && {cmd}"
                activate
            end tell"#,
            cwd = escaped_cwd,
            cmd = escaped_cmd,
        );
        return fire(&["osascript", "-e", &script]);
    }

    // ── 8. Linux: try common terminal emulators ───────────────────────────────
    if command_exists("gnome-terminal") {
        return fire(&[
            "gnome-terminal", "--working-directory", cwd_str,
            "--", &shell, "-i", "-c", &format!("{}; exec {}", cmd, shell),
        ]);
    }
    if command_exists("konsole") {
        return fire(&["konsole", "--workdir", cwd_str, "-e", &shell, "-i", "-c", cmd]);
    }
    if command_exists("foot") {
        return fire(&["foot", "--working-directory", cwd_str, &shell, "-i", "-c", cmd]);
    }
    if command_exists("xterm") {
        return fire(&[
            "xterm", "-e", &shell, "-i", "-c",
            &format!("cd {} && {}; read -p 'Press Enter...'", cwd_str, cmd),
        ]);
    }

    Err("No supported terminal emulator or multiplexer detected.\n\
         Supported: zellij, tmux, kitty (allow_remote_control yes), \
         wezterm, ghostty, iTerm2, Terminal.app, gnome-terminal, konsole, foot, xterm.".to_string())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Spawn `args[0]` with `args[1..]`, fully detached (no output capture, no wait).
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

/// Wrap `s` in single quotes, escaping any embedded single quotes.
fn shlex_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Return true if `cmd` is found on `$PATH`.
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
