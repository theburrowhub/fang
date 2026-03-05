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
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    println!("Fang - Make Commands Demo");
    println!("Directory: {}", dir.display());

    match commands::make::find_makefile(&dir) {
        Some(makefile) => {
            println!("Found: {}", makefile.display());
            match commands::make::parse_targets(&makefile) {
                Ok(targets) => {
                    println!("\nTargets ({}):", targets.len());
                    println!("{}", preview::makefile::format_targets(&targets));
                }
                Err(e) => println!("Parse error: {}", e),
            }
        }
        None => println!("No Makefile found"),
    }

    Ok(())
}
