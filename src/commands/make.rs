use std::path::{Path, PathBuf};
use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;
use crate::app::state::MakeTarget;
use crate::app::events::Event;

pub fn find_makefile(dir: &Path) -> Option<PathBuf> {
    let mf = dir.join("Makefile");
    if mf.exists() { Some(mf) } else { None }
}

pub fn parse_targets(_path: &Path) -> Result<Vec<MakeTarget>> {
    Ok(vec![])
}

pub async fn run_target(_target: &str, _dir: &PathBuf, tx: UnboundedSender<Event>) -> Result<()> {
    let _ = tx.send(Event::MakeDone { exit_code: 0 });
    Ok(())
}
