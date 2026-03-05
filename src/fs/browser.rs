use std::path::Path;
use anyhow::Result;
use crate::app::state::FileEntry;

/// Loads directory entries from the given path.
/// Returns an empty list as a stub — full implementation is in a later unit.
pub fn load_directory(_path: &Path) -> Result<Vec<FileEntry>> {
    Ok(vec![])
}
