use std::path::Path;
use anyhow::Result;
use crate::app::state::FileEntry;

pub fn load_directory(_path: &Path) -> Result<Vec<FileEntry>> {
    Ok(vec![])
}
