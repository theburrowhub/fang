use crate::fs::metadata::FileEntry;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Carga las entradas de un directorio.
/// Orden: directorios primero (alfabético), luego archivos (alfabético), case-insensitive.
/// Maneja errores de permisos gracefully (devuelve lo que puede leer).
pub fn load_directory(path: &Path) -> Result<Vec<FileEntry>> {
    let read_dir =
        std::fs::read_dir(path).with_context(|| format!("Cannot read directory: {:?}", path))?;

    let mut entries: Vec<FileEntry> = read_dir
        .filter_map(|entry_result| {
            entry_result
                .ok()
                .and_then(|entry| FileEntry::from_path(entry.path()))
        })
        .collect();

    // Sort: dirs first, then files, both alphabetically (case-insensitive).
    // sort_by_key allocates the key tuple once per element (O(n)) rather than
    // two lowercase Strings per comparison (O(n log n)).
    entries.sort_by_key(|e| (!e.is_dir, e.name.to_lowercase()));

    Ok(entries)
}

/// Verifica si un path está oculto (nombre empieza con ".")
pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}

/// Calcula el tamaño total de un directorio recursivamente (best-effort).
/// Limita a 10000 archivos para no ser bloqueante.
pub fn get_dir_size(path: &Path, max_files: usize) -> u64 {
    let mut total = 0u64;
    let mut count = 0;

    fn recurse(path: &Path, total: &mut u64, count: &mut usize, max: usize) {
        if *count >= max {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if *count >= max {
                    return;
                }
                *count += 1;
                let path = entry.path();
                if let Ok(metadata) = std::fs::symlink_metadata(&path) {
                    if metadata.is_file() {
                        *total += metadata.len();
                    } else if metadata.is_dir() {
                        recurse(&path, total, count, max);
                    }
                }
            }
        }
    }

    recurse(path, &mut total, &mut count, max_files);
    total
}

/// Obtiene el directorio padre de un path dado.
pub fn parent_dir(path: &Path) -> Option<PathBuf> {
    path.parent().map(|p| p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_current_dir() {
        let dir = std::env::current_dir().unwrap();
        let entries = load_directory(&dir).unwrap();
        assert!(!entries.is_empty(), "Current dir should have files");
    }

    #[test]
    fn test_dirs_come_first() {
        let dir = std::env::current_dir().unwrap();
        let entries = load_directory(&dir).unwrap();

        // Find the first non-dir entry
        let first_file_idx = entries.iter().position(|e| !e.is_dir);
        // Find the last dir entry
        let last_dir_idx = entries.iter().rposition(|e| e.is_dir);

        // If we have both dirs and files, dirs should all come before files
        if let (Some(first_file), Some(last_dir)) = (first_file_idx, last_dir_idx) {
            assert!(
                last_dir < first_file,
                "All dirs should come before all files"
            );
        }
    }

    #[test]
    fn test_alphabetical_sort_within_dirs() {
        let dir = std::env::current_dir().unwrap();
        let entries = load_directory(&dir).unwrap();

        let dirs: Vec<&str> = entries
            .iter()
            .filter(|e| e.is_dir)
            .map(|e| e.name.as_str())
            .collect();
        let mut sorted_dirs = dirs.clone();
        sorted_dirs.sort_by_key(|s| s.to_lowercase());
        assert_eq!(dirs, sorted_dirs, "Dirs should be alphabetically sorted");

        let files: Vec<&str> = entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.name.as_str())
            .collect();
        let mut sorted_files = files.clone();
        sorted_files.sort_by_key(|s| s.to_lowercase());
        assert_eq!(files, sorted_files, "Files should be alphabetically sorted");
    }

    #[test]
    fn test_is_hidden() {
        assert!(is_hidden(Path::new(".gitignore")));
        assert!(is_hidden(Path::new(".hidden")));
        assert!(!is_hidden(Path::new("main.rs")));
        assert!(!is_hidden(Path::new("Cargo.toml")));
    }

    #[test]
    fn test_load_nonexistent_dir() {
        let result = load_directory(Path::new("/nonexistent/path/fang"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parent_dir() {
        let path = PathBuf::from("/home/user/docs/file.txt");
        let parent = parent_dir(&path);
        assert_eq!(parent, Some(PathBuf::from("/home/user/docs")));
    }

    #[test]
    fn test_parent_dir_root() {
        let path = PathBuf::from("/");
        let parent = parent_dir(&path);
        assert!(parent.is_none());
    }

    #[test]
    fn test_get_dir_size_current() {
        let dir = std::env::current_dir().unwrap();
        // Should not panic and should return a value
        let size = get_dir_size(&dir, 100);
        // We just verify it doesn't panic; size may be 0 or positive
        let _ = size;
    }
}
