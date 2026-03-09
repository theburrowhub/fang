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
    // After opening, focus-next-pane moves focus into the new pane.
    if std::env::var("ZELLIJ").is_ok() || std::env::var("ZELLIJ_SESSION_NAME").is_ok() {
        fire(&[
            "zellij",
            "action",
            "new-pane",
            "--direction",
            "right",
            "--cwd",
            cwd_str,
            "--",
            &shell,
            "-i",
            "-c",
            cmd,
        ])?;
        // Move focus into the freshly-opened pane.
        let _ = fire(&["zellij", "action", "focus-next-pane"]);
        return Ok(());
    }

    // ── 2. Tmux ───────────────────────────────────────────────────────────────
    // -h means the panes sit side by side (vertical divider).
    // We capture the new pane ID with -P -F '#{pane_id}' so we can focus it.
    if std::env::var("TMUX").is_ok() {
        return tmux_split_and_focus(cwd_str, &shell_cmd);
    }

    // ── 3. Kitty ──────────────────────────────────────────────────────────────
    // "hsplit" = kitty creates a vertical dividing line → panes are LEFT | RIGHT.
    // Requires: `allow_remote_control yes` in kitty.conf (or --listen-on at launch).
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return fire(&[
            "kitty",
            "@",
            "launch",
            "--type=hsplit", // vertical divider, panes side by side
            &format!("--cwd={}", cwd_str),
            "--hold", // keep pane open after command exits
            &shell,
            "-i",
            "-c",
            cmd,
        ])
        .map_err(|e| {
            format!(
                "{}\nHint: add 'allow_remote_control yes' to ~/.config/kitty/kitty.conf",
                e
            )
        });
    }

    // ── 4. WezTerm ────────────────────────────────────────────────────────────
    // --horizontal = adds a vertical dividing line → panes are left | right.
    if std::env::var("WEZTERM_UNIX_SOCKET").is_ok() || std::env::var("WEZTERM_PANE").is_ok() {
        return fire(&[
            "wezterm",
            "cli",
            "split-pane",
            "--horizontal", // = vertical divider (WezTerm naming)
            "--cwd",
            cwd_str,
            "--",
            &shell,
            "-i",
            "-c",
            cmd,
        ]);
    }

    // ── 5. Ghostty ───────────────────────────────────────────────────────────
    if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok()
        || matches!(std::env::var("TERM_PROGRAM").as_deref(), Ok("ghostty"))
    {
        // Try the IPC approach first, fall back to new window
        return fire(&[
            "ghostty",
            "action",
            "new-split",
            "--direction",
            "right",
            "--command",
            &shell_cmd,
        ])
        .or_else(|_| fire(&["ghostty", "action", "new-window", "--command", &shell_cmd]));
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
    if matches!(
        std::env::var("TERM_PROGRAM").as_deref(),
        Ok("Apple_Terminal")
    ) {
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
            "gnome-terminal",
            "--working-directory",
            cwd_str,
            "--",
            &shell,
            "-i",
            "-c",
            &format!("{}; exec {}", cmd, shell),
        ]);
    }
    if command_exists("konsole") {
        return fire(&[
            "konsole",
            "--workdir",
            cwd_str,
            "-e",
            &shell,
            "-i",
            "-c",
            cmd,
        ]);
    }
    if command_exists("foot") {
        return fire(&[
            "foot",
            "--working-directory",
            cwd_str,
            &shell,
            "-i",
            "-c",
            cmd,
        ]);
    }
    if command_exists("xterm") {
        return fire(&[
            "xterm",
            "-e",
            &shell,
            "-i",
            "-c",
            &format!("cd {} && {}; read -p 'Press Enter...'", cwd_str, cmd),
        ]);
    }

    Err("No supported terminal emulator or multiplexer detected.\n\
         Supported: zellij, tmux, kitty (allow_remote_control yes), \
         wezterm, ghostty, iTerm2, Terminal.app, gnome-terminal, konsole, foot, xterm."
        .to_string())
}

/// Open `cmd` in a floating tmux popup overlay.
///
/// The popup covers 80 % of the window and closes automatically when the
/// command exits (tmux `-EE` flag).  Falls back to a regular split when tmux
/// is not available.
pub fn open_in_popup(cmd: &str, cwd: &Path) -> Result<(), String> {
    let cwd_str = cwd.to_str().unwrap_or(".");

    if std::env::var("TMUX").is_ok() {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        // -d sets working directory; shell_cmd loads aliases via -i -c.
        let shell_cmd = format!("{} -i -c {}", shell, shlex_quote(cmd));
        // -EE: close popup even when command exits with non-zero.
        return fire(&[
            "tmux", "popup", "-EE", "-d", cwd_str, "-w", "80%", "-h", "80%", &shell_cmd,
        ]);
    }

    // Fallback: regular split when not in tmux.
    open_in_split(cmd, cwd)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Split the current tmux window horizontally, then focus the new pane.
///
/// Uses `-P -F '#{pane_id}'` to capture the new pane's ID so we can call
/// `select-pane` immediately after — giving the user's focus to the new split.
fn tmux_split_and_focus(cwd_str: &str, shell_cmd: &str) -> Result<(), String> {
    let output = std::process::Command::new("tmux")
        .args([
            "split-window",
            "-h",
            "-P",
            "-F",
            "#{pane_id}",
            "-c",
            cwd_str,
            shell_cmd,
        ])
        .output()
        .map_err(|e| format!("tmux: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("tmux split-window: {}", err));
    }

    let pane_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !pane_id.is_empty() {
        let _ = std::process::Command::new("tmux")
            .args(["select-pane", "-t", &pane_id])
            .status();
    }
    Ok(())
}

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
pub fn shlex_quote(s: &str) -> String {
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
