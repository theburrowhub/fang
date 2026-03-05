use anyhow::Result;

mod app;
mod ui;
mod fs;
mod preview;
mod search;
mod commands;

#[tokio::main]
async fn main() -> Result<()> {
    use app::state::AppState;
    use std::path::PathBuf;

    let query = std::env::args().nth(1).unwrap_or_else(|| "rs".to_string());

    let mut state = AppState::new(PathBuf::from("."));
    state.entries = vec![
        app::state::FileEntry {
            name: "main.rs".into(),
            path: "main.rs".into(),
            is_dir: false,
            is_symlink: false,
            size: 1000,
            is_executable: false,
            extension: Some("rs".into()),
        },
        app::state::FileEntry {
            name: "lib.rs".into(),
            path: "lib.rs".into(),
            is_dir: false,
            is_symlink: false,
            size: 2000,
            is_executable: false,
            extension: Some("rs".into()),
        },
        app::state::FileEntry {
            name: "Cargo.toml".into(),
            path: "Cargo.toml".into(),
            is_dir: false,
            is_symlink: false,
            size: 500,
            is_executable: false,
            extension: Some("toml".into()),
        },
        app::state::FileEntry {
            name: "README.md".into(),
            path: "README.md".into(),
            is_dir: false,
            is_symlink: false,
            size: 3000,
            is_executable: false,
            extension: Some("md".into()),
        },
        app::state::FileEntry {
            name: "src".into(),
            path: "src".into(),
            is_dir: true,
            is_symlink: false,
            size: 0,
            is_executable: false,
            extension: None,
        },
    ];
    state.search_query = query.clone();

    search::fuzzy::apply_search(&mut state);

    println!("Query: '{}'", query);
    println!("Results: {}/{}", state.filtered_indices.len(), state.entries.len());
    for &i in &state.filtered_indices {
        let entry = &state.entries[i];
        let icon = if entry.is_dir { "▶" } else { "·" };
        println!("  {} {}", icon, entry.name);
    }

    Ok(())
}
