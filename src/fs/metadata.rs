use std::path::PathBuf;
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    Directory,
    RegularFile,
    Symlink,
    Executable,
    Image,
    Video,
    Audio,
    Archive,
    Document,
    Code,
    Config,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub is_executable: bool,
    pub extension: Option<String>,
    pub file_type: FileType,
    pub modified: Option<std::time::SystemTime>,
}

impl FileEntry {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let metadata = std::fs::symlink_metadata(&path).ok()?;
        let name = path.file_name()?.to_string_lossy().to_string();
        let is_symlink = metadata.file_type().is_symlink();
        let is_dir = if is_symlink {
            std::fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false)
        } else {
            metadata.is_dir()
        };
        let size = if is_dir { 0 } else { metadata.len() };
        let is_executable = !is_dir && (metadata.permissions().mode() & 0o111 != 0);
        let extension: Option<String> = if !is_dir {
            path.extension().map(|e| e.to_string_lossy().to_lowercase())
        } else {
            None
        };
        let file_type = classify_file(is_dir, is_symlink, is_executable, extension.as_deref());
        let modified = metadata.modified().ok();
        Some(FileEntry { name, path, is_dir, is_symlink, size, is_executable, extension, file_type, modified })
    }
}

fn classify_file(is_dir: bool, is_symlink: bool, is_executable: bool, ext: Option<&str>) -> FileType {
    if is_symlink { return FileType::Symlink; }
    if is_dir { return FileType::Directory; }
    match ext {
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp") | Some("bmp") | Some("svg") | Some("ico") => FileType::Image,
        Some("mp4") | Some("mkv") | Some("avi") | Some("mov") | Some("webm") => FileType::Video,
        Some("mp3") | Some("wav") | Some("flac") | Some("ogg") | Some("aac") => FileType::Audio,
        Some("zip") | Some("tar") | Some("gz") | Some("bz2") | Some("7z") | Some("rar") | Some("xz") => FileType::Archive,
        Some("pdf") | Some("doc") | Some("docx") | Some("odt") | Some("epub") => FileType::Document,
        Some("rs") | Some("py") | Some("js") | Some("ts") | Some("go") | Some("c") | Some("cpp") | Some("h") | Some("java") | Some("kt") | Some("rb") | Some("php") | Some("swift") | Some("sh") | Some("bash") | Some("zsh") | Some("fish") | Some("lua") | Some("vim") | Some("el") | Some("clj") | Some("hs") | Some("ml") | Some("ex") | Some("exs") | Some("erl") | Some("cs") | Some("fs") | Some("scala") | Some("dart") => FileType::Code,
        Some("toml") | Some("yaml") | Some("yml") | Some("json") | Some("xml") | Some("ini") | Some("cfg") | Some("conf") | Some("env") => FileType::Config,
        _ => if is_executable { FileType::Executable } else { FileType::Unknown },
    }
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes == 0 { "-".to_string() }
    else if bytes < KB { format!("{} B", bytes) }
    else if bytes < MB { format!("{:.1} KB", bytes as f64 / KB as f64) }
    else if bytes < GB { format!("{:.1} MB", bytes as f64 / MB as f64) }
    else { format!("{:.1} GB", bytes as f64 / GB as f64) }
}

pub fn get_file_icon(entry: &FileEntry) -> &'static str {
    if entry.is_dir { return "\u{25B6}"; }
    if entry.is_symlink { return "\u{21AA}"; }
    match entry.file_type {
        FileType::Image => "\u{1F5BC}", FileType::Video => "\u{25B6}", FileType::Audio => "\u{266A}",
        FileType::Archive => "\u{1F4E6}", FileType::Document => "\u{1F4C4}",
        FileType::Executable => "\u{26A1}", FileType::Code => "{}", FileType::Config => "\u{2699}",
        _ => "\u{00B7}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_format_size_zero() { assert_eq!(format_size(0), "-"); }
    #[test]
    fn test_format_size_bytes() { assert_eq!(format_size(100), "100 B"); }
    #[test]
    fn test_format_size_kb() { assert_eq!(format_size(1024), "1.0 KB"); }
    #[test]
    fn test_format_size_mb() { assert_eq!(format_size(1024 * 1024), "1.0 MB"); }
    #[test]
    fn test_format_size_gb() { assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB"); }
    #[test]
    fn test_format_size_fractional() { let r = format_size(1536); assert!(r.contains("1.5") && r.contains("KB")); }
    #[test]
    fn test_classify_code() { assert_eq!(classify_file(false, false, false, Some("rs")), FileType::Code); }
    #[test]
    fn test_classify_image() { assert_eq!(classify_file(false, false, false, Some("png")), FileType::Image); }
    #[test]
    fn test_classify_dir() { assert_eq!(classify_file(true, false, false, None), FileType::Directory); }
    #[test]
    fn test_file_entry_from_current_dir() {
        let path = std::env::current_dir().unwrap();
        let entry = FileEntry::from_path(path).unwrap();
        assert!(entry.is_dir);
    }
}
