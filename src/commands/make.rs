use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::app::state::MakeTarget;

pub fn parse_targets(_path: &Path) -> Result<Vec<MakeTarget>> {
    Ok(vec![])
}

pub fn find_makefile(dir: &Path) -> Option<PathBuf> {
    let mf = dir.join("Makefile");
    if mf.exists() { Some(mf) } else { None }
}
