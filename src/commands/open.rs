//! Open files and directories with the system's default application.
//!
//! - macOS: `open <path>` — uses Launch Services to pick the default app
//! - Linux: `xdg-open <path>` — uses XDG MIME type associations
//! - Windows: `explorer <path>`

use std::path::Path;
use std::process::Stdio;

/// Open `path` with the system's default application.
/// Fire-and-forget: the process is spawned detached; no output is captured.
pub fn open_with_system(path: &Path) -> Result<(), String> {
    let path_str = path.to_str().unwrap_or(".");

    #[cfg(target_os = "macos")]
    return fire(&["open", path_str]);

    #[cfg(target_os = "linux")]
    return fire(&["xdg-open", path_str]);

    #[cfg(target_os = "windows")]
    return fire(&["explorer", path_str]);

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    Err("open not supported on this platform".to_string())
}

fn fire(args: &[&str]) -> Result<(), String> {
    std::process::Command::new(args[0])
        .args(&args[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to launch `{}`: {}", args[0], e))
}
