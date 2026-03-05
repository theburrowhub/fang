use anyhow::Result;

mod app;
mod commands;
mod fs;
mod preview;
mod search;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    let dir = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    println!("Fang - Filesystem Demo");
    println!("Directory: {}", dir.display());
    println!("{}", "─".repeat(60));

    let entries = fs::browser::load_directory(&dir)?;
    for entry in &entries {
        let icon = fs::metadata::get_file_icon(entry);
        let size = if entry.is_dir {
            "    -".to_string()
        } else {
            format!("{:>8}", fs::metadata::format_size(entry.size))
        };
        let symlink = if entry.is_symlink { " → " } else { "   " };
        println!(
            "{} {}{:<40} {}",
            icon, symlink, entry.name, size
        );
    }
    println!("{}", "─".repeat(60));
    println!("{} items", entries.len());

    Ok(())
}
