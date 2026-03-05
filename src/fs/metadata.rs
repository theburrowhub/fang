use std::path::Path;
use anyhow::Result;

/// Get total size of a directory recursively
pub async fn dir_total_size(path: &Path) -> Result<(usize, u64)> {
    let mut total_size = 0u64;
    let mut count = 0usize;
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut rd = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = rd.next_entry().await? {
            let meta = entry.metadata().await?;
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total_size += meta.len();
                count += 1;
            }
        }
    }

    Ok((count, total_size))
}
