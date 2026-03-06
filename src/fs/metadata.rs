use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

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
        // Usa std::fs::symlink_metadata para no seguir symlinks
        // Si falla (permisos), retorna None
        let metadata = std::fs::symlink_metadata(&path).ok()?;
        let name = path.file_name()?.to_string_lossy().to_string();
        let is_symlink = metadata.file_type().is_symlink();
        let is_dir = if is_symlink {
            // Para symlinks, sigue el target para ver si es dir
            std::fs::metadata(&path)
                .map(|m| m.is_dir())
                .unwrap_or(false)
        } else {
            metadata.is_dir()
        };

        let size = if is_dir { 0 } else { metadata.len() };
        let is_executable = !is_dir && (metadata.permissions().mode() & 0o111 != 0);
        // Compute extension as Option<String> once; borrow it for classify_file.
        // to_string_lossy() returns a Cow<str>; to_lowercase() produces a String
        // directly, so no further conversion is needed.
        let extension: Option<String> = if !is_dir {
            path.extension().map(|e| e.to_string_lossy().to_lowercase())
        } else {
            None
        };

        let file_type = classify_file(is_dir, is_symlink, is_executable, extension.as_deref());
        let modified = metadata.modified().ok();

        Some(FileEntry {
            name,
            path,
            is_dir,
            is_symlink,
            size,
            is_executable,
            extension,
            file_type,
            modified,
        })
    }
}

fn classify_file(
    is_dir: bool,
    is_symlink: bool,
    is_executable: bool,
    ext: Option<&str>,
) -> FileType {
    if is_symlink {
        return FileType::Symlink;
    }
    if is_dir {
        return FileType::Directory;
    }
    match ext {
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp") | Some("bmp")
        | Some("svg") | Some("ico") => FileType::Image,
        Some("mp4") | Some("mkv") | Some("avi") | Some("mov") | Some("webm") => FileType::Video,
        Some("mp3") | Some("wav") | Some("flac") | Some("ogg") | Some("aac") => FileType::Audio,
        Some("zip") | Some("tar") | Some("gz") | Some("bz2") | Some("7z") | Some("rar")
        | Some("xz") => FileType::Archive,
        Some("pdf") | Some("doc") | Some("docx") | Some("odt") | Some("epub") => FileType::Document,
        Some("rs") | Some("py") | Some("js") | Some("ts") | Some("go") | Some("c")
        | Some("cpp") | Some("h") | Some("java") | Some("rb") | Some("php") | Some("swift")
        | Some("kt") | Some("scala") | Some("hs") | Some("lua") | Some("r") | Some("jl")
        | Some("ex") | Some("exs") | Some("erl") | Some("ml") | Some("clj") | Some("cs")
        | Some("fs") | Some("vb") => FileType::Code,
        Some("toml") | Some("yaml") | Some("yml") | Some("json") | Some("xml") | Some("ini")
        | Some("cfg") | Some("conf") | Some("env") | Some("editorconfig") | Some("gitignore")
        | Some("dockerignore") | Some("dockerfile") => FileType::Config,
        _ if is_executable => FileType::Executable,
        _ => FileType::Unknown,
    }
}

pub fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "-".to_string();
    }
    const UNITS: &[(&str, u64)] = &[
        ("TB", 1024 * 1024 * 1024 * 1024),
        ("GB", 1024 * 1024 * 1024),
        ("MB", 1024 * 1024),
        ("KB", 1024),
    ];
    for (unit, threshold) in UNITS {
        if bytes >= *threshold {
            let val = bytes as f64 / *threshold as f64;
            return format!("{:.1} {}", val, unit);
        }
    }
    format!("{} B", bytes)
}

pub fn get_file_icon(entry: &FileEntry) -> &'static str {
    // Usa caracteres que funcionen sin Nerd Fonts
    if entry.is_dir {
        return "▶";
    }
    if entry.is_symlink {
        return "→";
    }
    match entry.file_type {
        FileType::Image => "🖼",
        FileType::Video => "▶",
        FileType::Audio => "♪",
        FileType::Archive => "📦",
        FileType::Document => "📄",
        FileType::Executable => "⚡",
        FileType::Code => "{}",
        FileType::Config => "⚙",
        _ => "·",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_zero() {
        assert_eq!(format_size(0), "-");
    }
    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(100), "100 B");
    }
    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(1024), "1.0 KB");
    }
    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
    }
    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }
    #[test]
    fn test_format_size_fractional() {
        let result = format_size(1536); // 1.5 KB
        assert!(result.contains("1.5") && result.contains("KB"));
    }

    #[test]
    fn test_classify_code() {
        assert_eq!(
            classify_file(false, false, false, Some("rs")),
            FileType::Code
        );
        assert_eq!(
            classify_file(false, false, false, Some("py")),
            FileType::Code
        );
    }

    #[test]
    fn test_classify_image() {
        assert_eq!(
            classify_file(false, false, false, Some("png")),
            FileType::Image
        );
        assert_eq!(
            classify_file(false, false, false, Some("jpg")),
            FileType::Image
        );
    }

    #[test]
    fn test_classify_dir() {
        assert_eq!(classify_file(true, false, false, None), FileType::Directory);
    }

    #[test]
    fn test_file_entry_from_current_dir() {
        let path = std::env::current_dir().unwrap();
        let entry = FileEntry::from_path(path.clone());
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert!(entry.is_dir);
        assert!(!entry.is_symlink);
    }
}
