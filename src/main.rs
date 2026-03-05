use anyhow::Result;

mod app;
mod ui;
mod fs;
mod preview;
mod search;
mod commands;

#[tokio::main]
async fn main() -> Result<()> {
    let dir = std::env::args().nth(1)
        .map(std::path::PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)?;

    println!("Fang - scaffolding complete, implementation pending");
    println!("Initial dir: {:?}", dir);
    Ok(())
}
