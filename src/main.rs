use anyhow::Result;

mod app;
mod ui;
mod fs;
mod preview;
mod search;
mod commands;

#[tokio::main]
async fn main() -> Result<()> {
    let path = std::env::args().nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("Cargo.toml"));

    let metadata = std::fs::metadata(&path)?;
    let entry = app::state::FileEntry {
        name: path.file_name().unwrap().to_string_lossy().to_string(),
        path: path.clone(),
        is_dir: metadata.is_dir(),
        is_symlink: std::fs::symlink_metadata(&path).map(|m| m.file_type().is_symlink()).unwrap_or(false),
        size: metadata.len(),
        is_executable: false,
        extension: path.extension().map(|e| e.to_string_lossy().to_string()),
    };

    println!("Preview for: {}", entry.name);

    let preview = preview::load_preview(&entry).await;

    match &preview {
        app::state::PreviewState::Text { lines, total_lines } => {
            println!("Type: Text ({} lines, showing {} highlighted)", total_lines, lines.len());
            for (i, line) in lines.iter().take(5).enumerate() {
                let text: String = line.spans.iter().map(|(_, s)| s.as_str()).collect();
                println!("  {:3}: {}", i + 1, text.trim_end());
            }
        }
        app::state::PreviewState::Binary { size, mime_hint } => {
            println!("Type: Binary ({} bytes, {})", size, mime_hint);
        }
        app::state::PreviewState::Directory { entry_count, total_size } => {
            println!("Type: Directory ({} entries, {} bytes)", entry_count, total_size);
        }
        app::state::PreviewState::TooLarge { size } => {
            println!("Type: Too large ({} bytes)", size);
        }
        app::state::PreviewState::Error(e) => {
            println!("Error: {}", e);
        }
        _ => println!("Other: {:?}", preview),
    }

    Ok(())
}
