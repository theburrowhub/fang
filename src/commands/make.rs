use std::path::Path;
use anyhow::Result;
use crate::app::state::MakeTarget;

/// Parses Makefile targets from the given directory.
/// Returns an empty list as a stub — full implementation is in a later unit.
pub fn parse_makefile(_dir: &Path) -> Result<Vec<MakeTarget>> {
    Ok(vec![])
}

/// Executes a make target in the given directory.
/// Returns an empty output as a stub — full implementation is in a later unit.
pub async fn run_target(_dir: &Path, _target: &str) -> Result<Vec<String>> {
    Ok(vec![])
}
