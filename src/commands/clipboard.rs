//! Read content from the system clipboard.
//!
//! Priority:
//! 1. If a file is copied (macOS Finder "Copy"), read that file's bytes.
//! 2. Otherwise, read text/binary content from the clipboard.
//!
//! macOS: uses `osascript` for file URLs, `pbpaste` for text.
//! Linux: tries wl-paste (Wayland), xclip, xsel in order.

/// Read clipboard content as raw bytes.
/// Returns empty vec on failure rather than an error, to allow graceful degradation.
pub fn read_clipboard() -> Result<Vec<u8>, String> {
    #[cfg(target_os = "macos")]
    {
        macos_read_clipboard()
    }

    #[cfg(target_os = "linux")]
    {
        linux_read_clipboard()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err("Clipboard not supported on this platform".to_string())
    }
}

#[cfg(target_os = "macos")]
fn macos_read_clipboard() -> Result<Vec<u8>, String> {
    // 1. Try: is a file URL on the clipboard? (files copied in Finder)
    let file_url_script = r#"try
    POSIX path of (the clipboard as «class furl»)
on error
    ""
end try"#;
    if let Ok(out) = std::process::Command::new("osascript")
        .args(["-e", file_url_script])
        .output()
    {
        if out.status.success() {
            let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path_str.is_empty() && std::path::Path::new(&path_str).exists() {
                return std::fs::read(&path_str)
                    .map_err(|e| format!("Failed to read clipboard file: {}", e));
            }
        }
    }

    // 2. Fallback: text content via pbpaste
    std::process::Command::new("pbpaste")
        .output()
        .map(|o| o.stdout)
        .map_err(|e| format!("pbpaste failed: {}", e))
}

#[cfg(target_os = "linux")]
fn linux_read_clipboard() -> Result<Vec<u8>, String> {
    // Try wl-paste (Wayland), xclip, xsel in order
    let tools: &[&[&str]] = &[
        &["wl-paste", "--no-newline"],
        &["xclip", "-o", "-selection", "clipboard"],
        &["xsel", "--clipboard", "--output"],
    ];
    for tool in tools {
        if let Ok(out) = std::process::Command::new(tool[0]).args(&tool[1..]).output() {
            if out.status.success() {
                return Ok(out.stdout);
            }
        }
    }
    Err("No clipboard tool found. Install wl-paste (Wayland), xclip, or xsel.".to_string())
}
