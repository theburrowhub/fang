use anyhow::Result;
use crossterm::{
    event::{EventStream, Event as CrosstermEvent},
    execute,
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::prelude::*;
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::time::interval;

mod app;
mod ui;
mod fs;
mod preview;
mod search;
mod commands;

use app::events::Event;
use app::actions::{Action, map_key_to_action};
use app::state::AppState;

fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn schedule_preview(state: &AppState, tx: &UnboundedSender<Event>) {
    if let Some(entry) = state.selected_entry() {
        let entry = entry.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let preview = preview::load_preview(&entry).await;
            let _ = tx.send(Event::PreviewReady(preview));
        });
    }
}

fn schedule_directory_load(path: PathBuf, tx: &UnboundedSender<Event>) {
    let tx = tx.clone();
    tokio::spawn(async move {
        match fs::browser::load_directory(&path) {
            Ok(entries) => { let _ = tx.send(Event::DirectoryLoaded { path, entries }); }
            Err(e) => { tracing::warn!("Failed to load directory: {}", e); }
        }
    });
}

fn build_sidebar_tree(current_dir: &PathBuf) -> Vec<app::state::SidebarNode> {
    use std::path::Component;
    let mut nodes = Vec::new();
    let mut accumulated = PathBuf::new();
    for (depth, component) in current_dir.components().enumerate() {
        match component {
            Component::RootDir => accumulated.push("/"),
            _ => accumulated.push(component.as_os_str()),
        }
        nodes.push(app::state::SidebarNode { path: accumulated.clone(), depth, is_expanded: true, is_dir: true });
    }
    nodes
}

fn navigate_to_dir(state: &mut AppState, path: PathBuf, tx: &UnboundedSender<Event>) {
    state.current_dir = path.clone();
    state.selected_index = 0;
    state.file_list_scroll = 0;
    state.search_query.clear();
    state.mode = app::state::AppMode::Normal;
    state.preview_state = app::state::PreviewState::Loading;
    state.sidebar_tree = build_sidebar_tree(&path);
    schedule_directory_load(path, tx);
}

fn apply_search_update(state: &mut AppState, tx: &UnboundedSender<Event>) {
    if let app::state::AppMode::Search { query } = &mut state.mode {
        *query = state.search_query.clone();
    }
    state.selected_index = 0;
    search::fuzzy::apply_search(state);
    // Only schedule preview if there is a matching entry to preview.
    if state.visible_count() > 0 {
        schedule_preview(state, tx);
    }
}

fn handle_action(action: &Action, state: &mut AppState, tx: &UnboundedSender<Event>) {
    match action {
        Action::Quit => { state.should_quit = true; }
        Action::NavDown => {
            let count = state.visible_count();
            if count > 0 && state.selected_index < count - 1 { state.selected_index += 1; schedule_preview(state, tx); }
        }
        Action::NavUp => {
            if state.selected_index > 0 { state.selected_index -= 1; schedule_preview(state, tx); }
        }
        Action::NavLeft => {
            if let Some(parent) = state.current_dir.parent().map(|p| p.to_path_buf()) { navigate_to_dir(state, parent, tx); }
        }
        Action::NavRight => {
            if let Some(entry) = state.selected_entry().cloned() {
                if entry.is_dir { navigate_to_dir(state, entry.path, tx); }
            }
        }
        Action::ToggleSidebar => { state.sidebar_visible = !state.sidebar_visible; }
        Action::TogglePreview => { state.preview_visible = !state.preview_visible; }
        Action::OpenSearch => {
            state.mode = app::state::AppMode::Search { query: String::new() };
            state.search_query.clear();
            state.filtered_indices = (0..state.entries.len()).collect();
            state.selected_index = 0;
        }
        Action::SearchInput(c) => { state.search_query.push(*c); apply_search_update(state, tx); }
        Action::SearchBackspace => { state.search_query.pop(); apply_search_update(state, tx); }
        Action::CloseSearch => {
            state.mode = app::state::AppMode::Normal;
            state.search_query.clear();
            state.filtered_indices = (0..state.entries.len()).collect();
            state.selected_index = 0;
        }
        Action::OpenMakeModal => {
            if let Some(makefile) = commands::make::find_makefile(&state.current_dir) {
                match commands::make::parse_targets(&makefile) {
                    Ok(targets) if !targets.is_empty() => {
                        state.make_targets = targets;
                        state.make_target_selected = 0;
                        state.mode = app::state::AppMode::MakeTarget;
                    }
                    Ok(_) => { state.status_message = Some("No targets found in Makefile".to_string()); }
                    Err(e) => { state.status_message = Some(format!("Error reading Makefile: {}", e)); }
                }
            } else {
                state.status_message = Some("No Makefile found in current directory".to_string());
            }
        }
        Action::CloseMakeModal => { state.mode = app::state::AppMode::Normal; }
        Action::MakeNavDown => {
            if state.make_target_selected < state.make_targets.len().saturating_sub(1) { state.make_target_selected += 1; }
        }
        Action::MakeNavUp => {
            if state.make_target_selected > 0 { state.make_target_selected -= 1; }
        }
        Action::RunMakeTarget => {
            if let Some(target) = state.make_targets.get(state.make_target_selected) {
                let target_name = target.name.clone();
                let dir = state.current_dir.clone();
                let tx = tx.clone();
                state.preview_state = app::state::PreviewState::MakeOutput { output: vec![] };
                state.mode = app::state::AppMode::Normal;
                tokio::spawn(async move { let _ = commands::make::run_target(&target_name, &dir, tx).await; });
            }
        }
        Action::PreviewScrollUp => { state.preview_scroll = state.preview_scroll.saturating_sub(3); }
        Action::PreviewScrollDown => { state.preview_scroll += 3; }
        Action::FocusNext => {
            state.focused_panel = match state.focused_panel {
                app::state::FocusedPanel::Sidebar => app::state::FocusedPanel::FileList,
                app::state::FocusedPanel::FileList => {
                    if state.preview_visible { app::state::FocusedPanel::Preview }
                    else if state.sidebar_visible { app::state::FocusedPanel::Sidebar }
                    else { app::state::FocusedPanel::FileList }
                }
                app::state::FocusedPanel::Preview => {
                    if state.sidebar_visible { app::state::FocusedPanel::Sidebar } else { app::state::FocusedPanel::FileList }
                }
            };
        }
        Action::Noop => {}
    }
}

fn handle_event(event: Event, state: &mut AppState, tx: &UnboundedSender<Event>) {
    match event {
        Event::Key(key_event) => { let action = map_key_to_action(&key_event, &state.mode); handle_action(&action, state, tx); }
        Event::Resize(_, _) => {}
        Event::Tick => {}
        Event::PreviewReady(preview_state) => { state.preview_state = preview_state; state.preview_scroll = 0; }
        Event::MakeOutputLine(line) => {
            if let app::state::PreviewState::MakeOutput { output } = &mut state.preview_state {
                output.push(line);
            }
        }
        Event::MakeDone { exit_code } => {
            let msg = if exit_code == 0 { "make completed successfully".to_string() } else { format!("make exited with code {}", exit_code) };
            let footer = format!("\n[{}]", msg);
            if let app::state::PreviewState::MakeOutput { output } = &mut state.preview_state {
                output.push(footer);
            }
            state.status_message = Some(msg);
        }
        Event::DirectoryLoaded { path, entries } => {
            if path == state.current_dir {
                state.entries = entries;
                state.filtered_indices = (0..state.entries.len()).collect();
                if !state.search_query.is_empty() { search::fuzzy::apply_search(state); }
                state.selected_index = 0;
                schedule_preview(state, tx);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let file_appender = tracing_appender::rolling::daily("/tmp", "fang.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("fang=debug".parse()?))
        .init();
    setup_panic_hook();
    let initial_dir = std::env::args().nth(1).map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    tracing::info!("Fang starting in {:?}", initial_dir);
    let mut state = AppState::new(initial_dir.clone());
    state.sidebar_tree = build_sidebar_tree(&initial_dir);
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let mut terminal = init_terminal()?;
    schedule_directory_load(initial_dir, &tx);
    state.preview_state = app::state::PreviewState::Loading;
    let mut crossterm_events = EventStream::new();
    let mut tick_timer = interval(Duration::from_millis(250));
    loop {
        terminal.draw(|f| ui::layout::draw(f, &state))?;
        if state.should_quit { break; }
        tokio::select! {
            Some(event) = rx.recv() => { handle_event(event, &mut state, &tx); }
            Some(Ok(ct_event)) = crossterm_events.next() => {
                match ct_event {
                    CrosstermEvent::Key(key) => { handle_event(Event::Key(key), &mut state, &tx); }
                    CrosstermEvent::Resize(w, h) => { handle_event(Event::Resize(w, h), &mut state, &tx); }
                    _ => {}
                }
            }
            _ = tick_timer.tick() => { handle_event(Event::Tick, &mut state, &tx); }
        }
    }
    restore_terminal(&mut terminal)?;
    tracing::info!("Fang exited cleanly");
    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use std::path::PathBuf;
    #[test]
    fn test_app_state_initialization() {
        let state = crate::app::state::AppState::new(PathBuf::from("."));
        assert!(state.entries.is_empty());
        assert_eq!(state.selected_index, 0);
        assert!(!state.should_quit);
    }
    #[test]
    fn test_build_sidebar_tree() {
        let path = PathBuf::from("/tmp/a/b");
        let tree = super::build_sidebar_tree(&path);
        assert!(!tree.is_empty());
        assert!(tree[0].path.to_str().unwrap_or("").contains('/'));
    }
}
