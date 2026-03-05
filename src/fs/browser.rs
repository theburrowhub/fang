use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use crate::fs::metadata::FileEntry;

pub fn load_directory(path: &Path) -> Result<Vec<FileEntry>> {
    let read_dir = std::fs::read_dir(path).with_context(|| format!("Cannot read directory: {:?}", path))?;
    let mut entries: Vec<FileEntry> = read_dir
        .filter_map(|e| e.ok().and_then(|e| FileEntry::from_path(e.path())))
        .collect();
    entries.sort_by_key(|e| (!e.is_dir, e.name.to_lowercase()));
    Ok(entries)
}

pub fn is_hidden(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with('.')).unwrap_or(false)
}

pub fn get_dir_size(path: &Path, max_files: usize) -> u64 {
    let mut total = 0u64;
    let mut count = 0;
    fn recurse(path: &Path, total: &mut u64, count: &mut usize, max: usize) {
        if *count >= max { return; }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if *count >= max { return; }
                *count += 1;
                let path = entry.path();
                if let Ok(metadata) = std::fs::symlink_metadata(&path) {
                    if metadata.is_file() { *total += metadata.len(); }
                    else if metadata.is_dir() { recurse(&path, total, count, max); }
                }
            }
        }
    }
    recurse(path, &mut total, &mut count, max_files);
    total
}

pub fn parent_dir(path: &Path) -> Option<PathBuf> {
    path.parent().map(|p| p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_load_current_dir() {
        let dir = std::env::current_dir().unwrap();
        let entries = load_directory(&dir).unwrap();
        assert!(!entries.is_empty());
    }
    #[test]
    fn test_dirs_come_first() {
        let dir = std::env::current_dir().unwrap();
        let entries = load_directory(&dir).unwrap();
        let first_file_idx = entries.iter().position(|e| !e.is_dir);
        let last_dir_idx = entries.iter().rposition(|e| e.is_dir);
        if let (Some(ff), Some(ld)) = (first_file_idx, last_dir_idx) {
            assert!(ld < ff);
        }
    }
    #[test]
    fn test_is_hidden() {
        assert!(is_hidden(std::path::Path::new(".hidden")));
        assert!(!is_hidden(std::path::Path::new("visible")));
    }
    #[test]
    fn test_parent_dir() {
        let path = std::path::PathBuf::from("/tmp/a/b");
        assert_eq!(parent_dir(&path), Some(std::path::PathBuf::from("/tmp/a")));
    }
}
