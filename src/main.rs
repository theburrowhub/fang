use anyhow::Result;
use std::io;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

mod app;
mod ui;
mod fs;
mod preview;
mod search;
mod commands;

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = app::state::AppState::new(std::env::current_dir()?);

    // Populate with fake data for UI demonstration
    state.entries = vec![
        app::state::FileEntry {
            name: "src".into(),
            path: "src".into(),
            is_dir: true,
            is_symlink: false,
            size: 0,
            is_executable: false,
            extension: None,
        },
        app::state::FileEntry {
            name: "target".into(),
            path: "target".into(),
            is_dir: true,
            is_symlink: false,
            size: 0,
            is_executable: false,
            extension: None,
        },
        app::state::FileEntry {
            name: "Cargo.toml".into(),
            path: "Cargo.toml".into(),
            is_dir: false,
            is_symlink: false,
            size: 1234,
            is_executable: false,
            extension: Some("toml".into()),
        },
        app::state::FileEntry {
            name: "Makefile".into(),
            path: "Makefile".into(),
            is_dir: false,
            is_symlink: false,
            size: 567,
            is_executable: false,
            extension: None,
        },
        app::state::FileEntry {
            name: "README.md".into(),
            path: "README.md".into(),
            is_dir: false,
            is_symlink: false,
            size: 4567,
            is_executable: false,
            extension: Some("md".into()),
        },
        app::state::FileEntry {
            name: "fang".into(),
            path: "target/debug/fang".into(),
            is_dir: false,
            is_symlink: false,
            size: 5_000_000,
            is_executable: true,
            extension: None,
        },
    ];
    state.filtered_indices = (0..state.entries.len()).collect();
    state.preview_state = app::state::PreviewState::Text {
        lines: vec![
            app::state::StyledLine {
                spans: vec![(
                    ratatui::style::Style::default()
                        .fg(ratatui::style::Color::Cyan)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                    "[package]".to_string(),
                )],
            },
            app::state::StyledLine {
                spans: vec![(
                    ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
                    "name = \"fang\"".to_string(),
                )],
            },
            app::state::StyledLine {
                spans: vec![(
                    ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
                    "version = \"0.1.0\"".to_string(),
                )],
            },
            app::state::StyledLine {
                spans: vec![(
                    ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
                    "edition = \"2021\"".to_string(),
                )],
            },
            app::state::StyledLine {
                spans: vec![(
                    ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray),
                    "".to_string(),
                )],
            },
            app::state::StyledLine {
                spans: vec![(
                    ratatui::style::Style::default()
                        .fg(ratatui::style::Color::Cyan)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                    "[dependencies]".to_string(),
                )],
            },
            app::state::StyledLine {
                spans: vec![(
                    ratatui::style::Style::default().fg(ratatui::style::Color::Green),
                    "ratatui = \"0.29\"".to_string(),
                )],
            },
            app::state::StyledLine {
                spans: vec![(
                    ratatui::style::Style::default().fg(ratatui::style::Color::Green),
                    "crossterm = \"0.27\"".to_string(),
                )],
            },
            app::state::StyledLine {
                spans: vec![(
                    ratatui::style::Style::default().fg(ratatui::style::Color::Green),
                    "tokio = { version = \"1\", features = [\"full\"] }".to_string(),
                )],
            },
        ],
        total_lines: 25,
    };

    terminal.draw(|f| ui::layout::draw(f, &state))?;

    std::thread::sleep(std::time::Duration::from_secs(3));

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("UI rendered successfully for 3 seconds!");
    Ok(())
}
