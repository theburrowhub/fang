use crate::app::state::PreviewState;
use std::path::Path;

const BINARY_EXTENSIONS: &[&str] = &[
    "exe", "bin", "dll", "so", "dylib", "obj", "o", "png", "jpg", "jpeg", "gif", "webp", "bmp",
    "ico", "tiff", "mp4", "mkv", "avi", "mov", "webm", "mp3", "wav", "flac", "ogg", "aac", "zip",
    "tar", "gz", "bz2", "7z", "rar", "xz", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    "dmg", "iso", "img", "class", "jar", "war", "pyc", "pyo", "wasm", "ttf", "otf", "woff",
    "woff2", "db", "sqlite", "sqlite3",
];

const MIME_HINTS: &[(&str, &str)] = &[
    ("png", "PNG image"),
    ("jpg", "JPEG image"),
    ("jpeg", "JPEG image"),
    ("gif", "GIF image"),
    ("webp", "WebP image"),
    ("mp4", "MP4 video"),
    ("mp3", "MP3 audio"),
    ("pdf", "PDF document"),
    ("zip", "ZIP archive"),
    ("tar", "TAR archive"),
    ("gz", "GZip archive"),
    ("exe", "Windows executable"),
    ("dmg", "macOS disk image"),
    ("wasm", "WebAssembly module"),
];

pub fn is_binary_by_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            BINARY_EXTENSIONS
                .iter()
                .any(|&ext| ext.eq_ignore_ascii_case(e))
        })
        .unwrap_or(false)
}

pub fn is_binary_by_content(data: &[u8]) -> bool {
    // Check first 8KB for null bytes — their presence reliably indicates binary content
    let check_len = data.len().min(8192);
    data[..check_len].iter().any(|&b| b == 0)
}

pub fn is_binary_file(path: &Path) -> bool {
    if is_binary_by_extension(path) {
        return true;
    }
    // Check content
    std::fs::read(path)
        .map(|data| is_binary_by_content(&data))
        .unwrap_or(false)
}

pub fn get_mime_hint(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(|ext| {
            MIME_HINTS
                .iter()
                .find(|(e, _)| e.eq_ignore_ascii_case(ext))
                .map(|(_, h)| h.to_string())
        })
        .unwrap_or_else(|| "Binary file".to_string())
}

/// Format a hex dump in the style of `hexdump -C`.
/// Each line: "00000000  48 65 6c 6c 6f 20 57 6f  72 6c 64 0a           |Hello World.|"
/// The hex column is always 49 chars wide (consistent for full and partial rows).
pub fn format_hex_dump(data: &[u8], max_lines: usize) -> Vec<String> {
    let mut lines = Vec::with_capacity(data.len().div_ceil(16).min(max_lines));

    for (i, chunk) in data.chunks(16).take(max_lines).enumerate() {
        let offset = i * 16;

        // Build hex column directly into a String, no intermediate Vec
        // Format: "xx xx xx xx xx xx xx xx  xx xx xx xx xx xx xx xx" (49 chars when full)
        let mut hex_col = String::with_capacity(49);
        for (j, byte) in chunk.iter().enumerate() {
            if j == 8 {
                hex_col.push(' '); // extra space between the two groups of 8
            } else if j > 0 {
                hex_col.push(' ');
            }
            hex_col.push_str(&format!("{:02x}", byte));
        }

        let ascii_part: String = chunk
            .iter()
            .map(|&b| {
                if (32..127).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        // {:<49} pads hex_col to exactly 49 chars, keeping partial rows aligned
        lines.push(format!("{:08x}  {:<49} |{}|", offset, hex_col, ascii_part));
    }

    lines
}

pub async fn load_binary_preview(path: &Path) -> PreviewState {
    let size = match std::fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) => return PreviewState::Error(format!("Cannot read: {}", e)),
    };

    let mime_hint = get_mime_hint(path);
    PreviewState::Binary { size, mime_hint }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_binary_extension() {
        assert!(is_binary_by_extension(std::path::Path::new("file.png")));
        assert!(is_binary_by_extension(std::path::Path::new("file.exe")));
        assert!(!is_binary_by_extension(std::path::Path::new("file.rs")));
        assert!(!is_binary_by_extension(std::path::Path::new("file.txt")));
    }

    #[test]
    fn test_is_binary_content_with_null() {
        let data = b"Hello\x00World";
        assert!(is_binary_by_content(data));
    }

    #[test]
    fn test_is_binary_content_plain_text() {
        let data = b"Hello, World!\nThis is plain text\n";
        assert!(!is_binary_by_content(data));
    }

    #[test]
    fn test_hex_dump_format() {
        let data = b"Hello World!";
        let lines = format_hex_dump(data, 10);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("48 65 6c 6c")); // "Hell" in hex
        assert!(lines[0].contains("Hello")); // ASCII representation
        assert!(lines[0].starts_with("00000000")); // offset
    }

    #[test]
    fn test_hex_dump_empty() {
        let data = b"";
        let lines = format_hex_dump(data, 10);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_hex_dump_multiline() {
        let data: Vec<u8> = (0..32).collect();
        let lines = format_hex_dump(&data, 10);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("00000000"));
        assert!(lines[1].starts_with("00000010"));
    }

    #[test]
    fn test_get_mime_hint() {
        assert_eq!(
            get_mime_hint(std::path::Path::new("image.png")),
            "PNG image"
        );
        assert_eq!(
            get_mime_hint(std::path::Path::new("video.mp4")),
            "MP4 video"
        );
        assert_eq!(
            get_mime_hint(std::path::Path::new("unknown.xyz")),
            "Binary file"
        );
    }
}
