use std::path::Path;
use crate::app::state::PreviewState;

const BINARY_EXTENSIONS: &[&str] = &[
    "exe","bin","dll","so","dylib","obj","o",
    "png","jpg","jpeg","gif","webp","bmp","ico","tiff",
    "mp4","mkv","avi","mov","webm","mp3","wav","flac","ogg","aac",
    "zip","tar","gz","bz2","7z","rar","xz",
    "pdf","doc","docx","xls","xlsx","ppt","pptx",
    "dmg","iso","img","class","jar","war","pyc","pyo","wasm",
    "ttf","otf","woff","woff2","db","sqlite","sqlite3",
];

const MIME_HINTS: &[(&str, &str)] = &[
    ("png","PNG image"),("jpg","JPEG image"),("jpeg","JPEG image"),("gif","GIF image"),
    ("webp","WebP image"),("mp4","MP4 video"),("mp3","MP3 audio"),("pdf","PDF document"),
    ("zip","ZIP archive"),("tar","TAR archive"),("gz","GZip archive"),
    ("exe","Windows executable"),("dmg","macOS disk image"),("wasm","WebAssembly module"),
];

pub fn is_binary_by_extension(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str())
        .map(|e| BINARY_EXTENSIONS.iter().any(|&ext| ext.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

pub fn is_binary_by_content(data: &[u8]) -> bool {
    let check_len = data.len().min(8192);
    data[..check_len].iter().any(|&b| b == 0)
}

pub fn is_binary_file(path: &Path) -> bool {
    if is_binary_by_extension(path) { return true; }
    std::fs::read(path).map(|data| is_binary_by_content(&data)).unwrap_or(false)
}

pub fn get_mime_hint(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str())
        .and_then(|ext| MIME_HINTS.iter().find(|(e, _)| e.eq_ignore_ascii_case(ext)).map(|(_, h)| h.to_string()))
        .unwrap_or_else(|| "Binary file".to_string())
}

pub fn format_hex_dump(data: &[u8], max_lines: usize) -> Vec<String> {
    let mut lines = Vec::with_capacity(data.len().div_ceil(16).min(max_lines));
    for (i, chunk) in data.chunks(16).take(max_lines).enumerate() {
        let offset = i * 16;
        let mut hex_col = String::with_capacity(49);
        for (j, byte) in chunk.iter().enumerate() {
            if j == 8 || j > 0 { hex_col.push(' '); }
            hex_col.push_str(&format!("{:02x}", byte));
        }
        let ascii_part: String = chunk.iter().map(|&b| if (32..127).contains(&b) { b as char } else { '.' }).collect();
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
        assert!(!is_binary_by_extension(std::path::Path::new("file.rs")));
    }
    #[test]
    fn test_is_binary_content_with_null() { assert!(is_binary_by_content(b"Hello\x00World")); }
    #[test]
    fn test_is_binary_content_plain_text() { assert!(!is_binary_by_content(b"Hello, World!\n")); }
    #[test]
    fn test_hex_dump_format() {
        let lines = format_hex_dump(b"Hello World!", 10);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("48 65 6c 6c"));
        assert!(lines[0].starts_with("00000000"));
    }
    #[test]
    fn test_hex_dump_empty() { assert!(format_hex_dump(b"", 10).is_empty()); }
    #[test]
    fn test_get_mime_hint() {
        assert_eq!(get_mime_hint(std::path::Path::new("image.png")), "PNG image");
        assert_eq!(get_mime_hint(std::path::Path::new("unknown.xyz")), "Binary file");
    }
}
