//! Read content from the system clipboard.
//!
//! Priority on macOS:
//!   1. Finder-copied file  → read that file's bytes directly
//!   2. Image data (PNG/TIFF/JPEG) in clipboard → write to temp file, read back
//!   3. Text/RTF → pbpaste
//!
//! Linux: wl-paste → xclip → xsel, with --mime image/* fallback.

/// Read clipboard content as raw bytes.
/// Returns `Ok(vec![])` on graceful failure so the caller can still
/// create an empty file rather than aborting.
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

// ─── macOS ────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn macos_read_clipboard() -> Result<Vec<u8>, String> {
    // 1. Finder-copied file: clipboard contains a file URL («class furl»).
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

    // 2. Image data copied from browser / app (PNG, TIFF, JPEG).
    //    AppleScript can read «class PNGf»/«class TIFF»/«class JPEG» and write
    //    the raw bytes to a temp file, which we then read back.
    //    Try PNG first (highest fidelity), then TIFF, then JPEG.
    let tmp = std::env::temp_dir().join("fang_clipboard_img");
    let tmp_str = tmp.display().to_string();

    for (as_type, suffix) in &[
        ("«class PNGf»", "png"),
        ("«class TIFF»", "tiff"),
        ("«class JPEG»", "jpg"),
    ] {
        let script = format!(
            r#"try
    set theData to (the clipboard as {as_type})
    set tmpPath to "{tmp}.{suffix}"
    set fileRef to open for access POSIX file tmpPath with write permission
    set eof of fileRef to 0
    write theData to fileRef
    close access fileRef
    return tmpPath
on error
    return ""
end try"#,
            as_type = as_type,
            tmp = tmp_str,
            suffix = suffix,
        );

        if let Ok(out) = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
        {
            if out.status.success() {
                let written_path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !written_path.is_empty() {
                    if let Ok(bytes) = std::fs::read(&written_path) {
                        let _ = std::fs::remove_file(&written_path);
                        if !bytes.is_empty() {
                            return Ok(bytes);
                        }
                    }
                }
            }
        }
    }

    // 3. Text / RTF fallback via pbpaste.
    std::process::Command::new("pbpaste")
        .output()
        .map(|o| o.stdout)
        .map_err(|e| format!("pbpaste failed: {}", e))
}

// ─── Linux ────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn linux_read_clipboard() -> Result<Vec<u8>, String> {
    // Try image MIME types first (for images copied from browser).
    for mime in &["image/png", "image/jpeg", "image/webp", "image/tiff"] {
        if let Some(data) = try_wl_paste_mime(mime)
            .or_else(|| try_xclip_mime(mime))
        {
            if !data.is_empty() {
                return Ok(data);
            }
        }
    }

    // Fallback: plain text/binary.
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

#[cfg(target_os = "linux")]
fn try_wl_paste_mime(mime: &str) -> Option<Vec<u8>> {
    std::process::Command::new("wl-paste")
        .args(["--no-newline", "--type", mime])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| o.stdout)
}

#[cfg(target_os = "linux")]
fn try_xclip_mime(mime: &str) -> Option<Vec<u8>> {
    std::process::Command::new("xclip")
        .args(["-o", "-selection", "clipboard", "-t", mime])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| o.stdout)
}
