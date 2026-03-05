use std::path::Path;
use anyhow::Result;
use crate::app::state::FileEntry;

/// Read directory entries from a path
pub async fn read_dir(path: &Path) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    let mut read_dir = tokio::fs::read_dir(path).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        let path_buf = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = metadata.is_dir();
        let is_symlink = metadata.file_type().is_symlink();
        let size = if is_dir { 0 } else { metadata.len() };
        let extension = if is_dir {
            None
        } else {
            path_buf
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase())
        };

        #[cfg(unix)]
        let is_executable = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o111 != 0
        };
        #[cfg(not(unix))]
        let is_executable = false;

        entries.push(FileEntry {
            name,
            path: path_buf.to_string_lossy().to_string(),
            is_dir,
            is_symlink,
            size,
            is_executable,
            extension,
        });
    }

    // Sort: dirs first, then files, both alphabetically
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}
