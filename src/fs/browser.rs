use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::app::state::FileEntry;

/// Reads the entries in a directory and returns them as FileEntry list.
pub fn read_dir(path: &Path) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let name = entry.file_name().to_string_lossy().to_string();
        let file_path: PathBuf = entry.path();

        entries.push(FileEntry {
            path: file_path,
            name,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }

    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
    });

    Ok(entries)
}
