use anyhow::Result;
#[cfg(unix)]
extern crate libc;
use crossterm::{
    event::{Event as CrosstermEvent, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::prelude::*;
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::time::interval;

mod app;
mod commands;
mod config;
mod fs;
mod preview;
mod search;
mod ui;

use app::actions::{map_key_to_action, Action};
use app::events::Event;
use app::state::AppState;

/// Sets up a panic hook that restores the terminal before printing the panic message.
fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal state so the panic message is readable
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

/// Initializes the terminal (raw mode + alternate screen).
fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

/// Restores the terminal to its previous state.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Schedules an async preview load for the currently selected entry.
/// Sends result as Event::PreviewReady via the internal channel.
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

/// Collects git branch and dev environment info for the given directory.
/// Runs dev env probes concurrently via tokio::join! and sends HeaderInfoReady when done.
fn schedule_header_info_load(dir: &std::path::Path, tx: &UnboundedSender<Event>) {
    let dir = dir.to_path_buf();
    let tx = tx.clone();
    tokio::spawn(async move {
        let mut info = app::state::HeaderInfo::default();

        // Git branch — sequential: SHA lookup only needed on detached HEAD
        if let Ok(output) = tokio::process::Command::new("git")
            .args([
                "-C",
                dir.to_str().unwrap_or("."),
                "rev-parse",
                "--abbrev-ref",
                "HEAD",
            ])
            .output()
            .await
        {
            if output.status.success() {
                let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !branch.is_empty() && branch != "HEAD" {
                    info.git_branch = Some(branch);
                } else if branch == "HEAD" {
                    // Detached HEAD — show short SHA
                    if let Ok(sha_out) = tokio::process::Command::new("git")
                        .args([
                            "-C",
                            dir.to_str().unwrap_or("."),
                            "rev-parse",
                            "--short",
                            "HEAD",
                        ])
                        .output()
                        .await
                    {
                        let sha = String::from_utf8_lossy(&sha_out.stdout).trim().to_string();
                        if !sha.is_empty() {
                            info.git_branch = Some(format!("@{}", sha));
                        }
                    }
                }
            }
        }

        // Dev env probes — all independent, run concurrently
        let venv_python = dir.join(".venv").join("bin").join("python");
        let venv_python_win = dir.join(".venv").join("Scripts").join("python.exe");
        let python_path = if venv_python.exists() {
            Some(venv_python)
        } else if venv_python_win.exists() {
            Some(venv_python_win)
        } else {
            None
        };

        let has_go_mod = dir.join("go.mod").exists();
        let has_package_json = dir.join("package.json").exists();
        let has_cargo_toml = dir.join("Cargo.toml").exists();

        let (py_result, go_result, node_result, rs_result) = tokio::join!(
            async {
                let py_path = python_path?;
                let out = tokio::process::Command::new(&py_path)
                    .arg("--version")
                    .output()
                    .await
                    .ok()?;
                // Python 3.x: version on stdout; Python 2.x: version on stderr
                let ver_str = String::from_utf8_lossy(&out.stdout).to_string()
                    + &String::from_utf8_lossy(&out.stderr);
                let version = ver_str.trim().trim_start_matches("Python ").to_string();
                if version.is_empty() {
                    None
                } else {
                    Some(("py".to_string(), version))
                }
            },
            async {
                if !has_go_mod {
                    return None;
                }
                let out = tokio::process::Command::new("go")
                    .arg("version")
                    .output()
                    .await
                    .ok()?;
                // "go version go1.22.1 darwin/arm64" — extract "1.22.1"
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                let ver = s
                    .split_whitespace()
                    .nth(2)?
                    .trim_start_matches("go")
                    .to_string();
                if ver.is_empty() {
                    None
                } else {
                    Some(("go".to_string(), ver))
                }
            },
            async {
                if !has_package_json {
                    return None;
                }
                let out = tokio::process::Command::new("node")
                    .arg("--version")
                    .output()
                    .await
                    .ok()?;
                // "v20.11.0" — trim leading 'v'
                let v = String::from_utf8_lossy(&out.stdout)
                    .trim()
                    .trim_start_matches('v')
                    .to_string();
                if v.is_empty() {
                    None
                } else {
                    Some(("node".to_string(), v))
                }
            },
            async {
                if !has_cargo_toml {
                    return None;
                }
                let out = tokio::process::Command::new("rustc")
                    .arg("--version")
                    .output()
                    .await
                    .ok()?;
                // "rustc 1.75.0 (82e1608df 2023-12-21)" — take "1.75.0"
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                let ver = s.split_whitespace().nth(1)?.to_string();
                if ver.is_empty() {
                    None
                } else {
                    Some(("rs".to_string(), ver))
                }
            },
        );

        for result in [py_result, go_result, node_result, rs_result]
            .into_iter()
            .flatten()
        {
            info.dev_envs.push(result);
        }

        let _ = tx.send(Event::HeaderInfoReady(info));
    });
}

/// Loads a directory asynchronously.
/// Sends result as Event::DirectoryLoaded via the internal channel.
fn schedule_directory_load(path: PathBuf, tx: &UnboundedSender<Event>) {
    let tx = tx.clone();
    tokio::spawn(async move {
        match fs::browser::load_directory(&path) {
            Ok(entries) => {
                let _ = tx.send(Event::DirectoryLoaded { path, entries });
            }
            Err(e) => {
                tracing::warn!("Failed to load directory: {}", e);
            }
        }
    });
}

/// Refresh git file-status for the given directory asynchronously.
fn schedule_git_status_load(dir: &std::path::Path, tx: &UnboundedSender<Event>) {
    let dir = dir.to_path_buf();
    let tx = tx.clone();
    tokio::spawn(async move {
        let status = commands::git::file_status(&dir).await;
        let _ = tx.send(Event::GitStatusReady(status));
    });
}

/// Navigates to a new directory: resets list state, loads the directory.
/// Shared by NavLeft (parent) and NavRight (child directory).
fn navigate_to_dir(state: &mut AppState, path: PathBuf, tx: &UnboundedSender<Event>) {
    state.current_dir = path.clone();
    state.selected_index = 0;
    state.file_list_scroll = 0;
    state.search_query.clear();
    state.mode = app::state::AppMode::Normal;
    state.preview_state = app::state::PreviewState::Loading;
    state.preview_scroll = 0;
    state.needs_terminal_clear = true;
    schedule_directory_load(path.clone(), tx);
    schedule_header_info_load(&path, tx);
    commands::title::set_window_title(&path);
    schedule_git_status_load(&state.current_dir, tx);
}

/// Syncs mode.query with state.search_query, re-filters, and schedules a preview refresh.
/// Shared by SearchInput and SearchBackspace.
fn apply_search_update(state: &mut AppState, tx: &UnboundedSender<Event>) {
    if let app::state::AppMode::Search { query } = &mut state.mode {
        *query = state.search_query.clone();
    }
    state.selected_index = 0;
    search::fuzzy::apply_search(state);
    schedule_preview(state, tx);
}

/// Write `text` to the system clipboard using the platform clipboard tool.
fn copy_to_clipboard(text: &str) -> Result<(), String> {
    commands::clipboard::write_clipboard(text)
}

/// Handles an Action, updating state and spawning tasks as needed.
fn handle_action(action: &Action, state: &mut AppState, tx: &UnboundedSender<Event>) {
    match action {
        Action::Quit => {
            state.should_quit = true;
        }
        Action::NavDown => {
            let count = state.visible_count();
            if count > 0 && state.selected_index < count - 1 {
                state.selected_index += 1;
                // Clear immediately so stale preview cells don't persist.
                state.preview_state = app::state::PreviewState::None;
                state.preview_scroll = 0;
                state.needs_terminal_clear = true;
                schedule_preview(state, tx);
            }
        }
        Action::NavUp => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
                state.preview_state = app::state::PreviewState::None;
                state.preview_scroll = 0;
                state.needs_terminal_clear = true;
                schedule_preview(state, tx);
            }
        }
        Action::NavLeft => {
            if let Some(parent) = state.current_dir.parent().map(|p| p.to_path_buf()) {
                // starts_with checks path components, not byte strings, so
                // "/home/user/proj2" correctly does NOT start_with "/home/user/proj".
                if parent.starts_with(&state.root_dir) {
                    navigate_to_dir(state, parent, tx);
                }
            }
        }
        Action::NavRight => {
            if let Some(entry) = state.selected_entry().cloned() {
                if entry.is_dir {
                    navigate_to_dir(state, entry.path, tx);
                }
            }
        }
        Action::TogglePreview => {
            state.preview_visible = !state.preview_visible;
        }
        Action::OpenSearch => {
            state.mode = app::state::AppMode::Search {
                query: String::new(),
            };
            state.search_query.clear();
            // Show all entries initially
            state.filtered_indices = (0..state.entries.len()).collect();
            state.selected_index = 0;
        }
        Action::SearchInput(c) => {
            state.search_query.push(*c);
            apply_search_update(state, tx);
        }
        Action::SearchBackspace => {
            state.search_query.pop();
            apply_search_update(state, tx);
        }
        Action::CloseSearch => {
            state.mode = app::state::AppMode::Normal;
            state.search_query.clear();
            state.filtered_indices = (0..state.entries.len()).collect();
            state.selected_index = 0;
        }
        Action::OpenMakeModal => {
            // Load make targets for current directory
            if let Some(makefile) = commands::make::find_makefile(&state.current_dir) {
                match commands::make::parse_targets(&makefile) {
                    Ok(targets) if !targets.is_empty() => {
                        state.make_targets = targets;
                        state.make_target_selected = 0;
                        state.mode = app::state::AppMode::MakeTarget;
                    }
                    Ok(_) => {
                        state.status_message = Some("No targets found in Makefile".to_string());
                    }
                    Err(e) => {
                        state.status_message = Some(format!("Error reading Makefile: {}", e));
                    }
                }
            } else {
                state.status_message = Some("No Makefile found in current directory".to_string());
            }
        }
        Action::CloseMakeModal => {
            state.mode = app::state::AppMode::Normal;
        }
        Action::MakeNavDown => {
            if state.make_target_selected < state.make_targets.len().saturating_sub(1) {
                state.make_target_selected += 1;
            }
        }
        Action::MakeNavUp => {
            if state.make_target_selected > 0 {
                state.make_target_selected -= 1;
            }
        }
        Action::RunMakeTarget => {
            if let Some(target) = state.make_targets.get(state.make_target_selected) {
                let target_name = target.name.clone();
                let dir = state.current_dir.clone();
                let tx2 = tx.clone();
                state.make_output.clear();
                state.preview_state = app::state::PreviewState::MakeOutput { output: vec![] };
                state.preview_scroll = 0;
                state.mode = app::state::AppMode::Normal;

                let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
                state.make_cancel_tx = Some(cancel_tx);

                tokio::spawn(async move {
                    let _ = commands::make::run_target(&target_name, &dir, tx2, cancel_rx).await;
                });
            }
        }
        Action::CancelMake => {
            if let Some(cancel_tx) = state.make_cancel_tx.take() {
                let _ = cancel_tx.send(());
            }
        }
        Action::OpenCommandInput => {
            state.mode = app::state::AppMode::CommandInput { cmd: String::new() };
        }
        Action::CommandInputChar(c) => {
            if let app::state::AppMode::CommandInput { cmd } = &mut state.mode {
                cmd.push(*c);
            }
        }
        Action::CommandInputBackspace => {
            if let app::state::AppMode::CommandInput { cmd } = &mut state.mode {
                cmd.pop();
            }
        }
        Action::CloseCommandInput => {
            state.mode = app::state::AppMode::Normal;
        }
        Action::OpenExternalCommand => {
            state.mode = app::state::AppMode::ExternalCommand { cmd: String::new() };
        }
        Action::ExternalCommandChar(c) => {
            if let app::state::AppMode::ExternalCommand { cmd } = &mut state.mode {
                cmd.push(*c);
            }
        }
        Action::ExternalCommandBackspace => {
            if let app::state::AppMode::ExternalCommand { cmd } = &mut state.mode {
                cmd.pop();
            }
        }
        Action::CloseExternalCommand => {
            state.mode = app::state::AppMode::Normal;
        }
        Action::RunExternalCommand => {
            let cmd = if let app::state::AppMode::ExternalCommand { cmd } = &state.mode {
                cmd.clone()
            } else {
                String::new()
            };
            state.mode = app::state::AppMode::Normal;
            if !cmd.is_empty() {
                let cwd = state.current_dir.clone();
                match commands::shell::open_in_split(&cmd, &cwd, true) {
                    Ok(()) => {}
                    Err(e) => {
                        state.status_message = Some(format!("Split error: {}", e));
                    }
                }
            }
        }
        Action::RunExternalCommandPopup => {
            let cmd = if let app::state::AppMode::ExternalCommand { cmd } = &state.mode {
                cmd.clone()
            } else {
                String::new()
            };
            state.mode = app::state::AppMode::Normal;
            if !cmd.is_empty() {
                let cwd = state.current_dir.clone();
                match commands::shell::open_in_popup(&cmd, &cwd) {
                    Ok(()) => {}
                    Err(e) => {
                        state.status_message = Some(format!("Popup error: {}", e));
                    }
                }
            }
        }
        Action::RunCommand => {
            let cmd = if let app::state::AppMode::CommandInput { cmd } = &state.mode {
                cmd.clone()
            } else {
                String::new()
            };
            if !cmd.is_empty() {
                let dir = state.current_dir.clone();
                let tx = tx.clone();
                state.preview_state = app::state::PreviewState::MakeOutput {
                    output: vec![format!("$ {}", cmd)],
                };
                state.preview_scroll = 0;
                state.mode = app::state::AppMode::Normal;

                // Stdin pipe: keypresses typed while the command runs are forwarded here.
                let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
                state.command_stdin = Some(stdin_tx);

                tokio::spawn(async move {
                    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
                    use tokio::process::Command;

                    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
                    let mut cmd_builder = Command::new(&shell);
                    cmd_builder
                        .args(["-i", "-c", &cmd])
                        .current_dir(&dir)
                        .stdin(std::process::Stdio::piped()) // pipe, not null
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped());

                    // setsid() creates a new session with no controlling terminal so the
                    // interactive shell cannot call tcsetpgrp() against fang's terminal.
                    // Only available on Unix; on Windows we skip this (no SIGTTOU concept).
                    #[cfg(unix)]
                    {
                        unsafe {
                            cmd_builder.pre_exec(|| {
                                libc::setsid();
                                Ok(())
                            });
                        }
                    }

                    let mut child = match cmd_builder.spawn() {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = tx.send(Event::MakeOutputLine(format!("Error: {}", e)));
                            let _ = tx.send(Event::CommandOutput { exit_code: -1 });
                            return;
                        }
                    };

                    // Relay stdin channel → child stdin
                    let mut child_stdin = child.stdin.take().expect("stdin pipe");
                    let stdin_relay = tokio::spawn(async move {
                        while let Some(bytes) = stdin_rx.recv().await {
                            if child_stdin.write_all(&bytes).await.is_err() {
                                break;
                            }
                            let _ = child_stdin.flush().await;
                        }
                        // Channel closed → drop child_stdin → EOF sent to child
                    });

                    let stdout = child.stdout.take().expect("stdout");
                    let stderr = child.stderr.take().expect("stderr");
                    let mut stdout_reader = BufReader::new(stdout).lines();
                    let mut stderr_reader = BufReader::new(stderr).lines();
                    let tx_out = tx.clone();
                    let tx_err = tx.clone();
                    let stdout_task = tokio::spawn(async move {
                        while let Ok(Some(line)) = stdout_reader.next_line().await {
                            if tx_out.send(Event::MakeOutputLine(line)).is_err() {
                                break;
                            }
                        }
                    });
                    let stderr_task = tokio::spawn(async move {
                        while let Ok(Some(line)) = stderr_reader.next_line().await {
                            if tx_err.send(Event::MakeOutputLine(line)).is_err() {
                                break;
                            }
                        }
                    });
                    let status = child.wait().await.ok();
                    let _ = tokio::join!(stdout_task, stderr_task, stdin_relay);
                    let code = status.and_then(|s| s.code()).unwrap_or(-1);
                    let _ = tx.send(Event::CommandOutput { exit_code: code });
                });
            } else {
                state.mode = app::state::AppMode::Normal;
            }
        }
        Action::PreviewScrollUp => {
            state.preview_scroll = state.preview_scroll.saturating_sub(3);
        }
        Action::PreviewScrollDown => {
            state.preview_scroll += 3;
        }
        Action::FocusNext => {
            // Cycle forward: FileList → Preview → AiChat → FileList (skip hidden)
            state.focused_panel = match state.focused_panel {
                app::state::FocusedPanel::FileList => {
                    if state.preview_visible {
                        app::state::FocusedPanel::Preview
                    } else if state.ai_panel_visible {
                        app::state::FocusedPanel::AiChat
                    } else {
                        app::state::FocusedPanel::FileList
                    }
                }
                app::state::FocusedPanel::Preview => {
                    if state.ai_panel_visible {
                        app::state::FocusedPanel::AiChat
                    } else {
                        app::state::FocusedPanel::FileList
                    }
                }
                app::state::FocusedPanel::AiChat => app::state::FocusedPanel::FileList,
            };
        }
        Action::FocusPrev => {
            // Cycle backward — exact mirror of FocusNext:
            // FileList ← Preview ← AiChat ← FileList (skip hidden)
            state.focused_panel = match state.focused_panel {
                app::state::FocusedPanel::FileList => {
                    if state.ai_panel_visible {
                        app::state::FocusedPanel::AiChat
                    } else if state.preview_visible {
                        app::state::FocusedPanel::Preview
                    } else {
                        app::state::FocusedPanel::FileList
                    }
                }
                app::state::FocusedPanel::Preview => app::state::FocusedPanel::FileList,
                app::state::FocusedPanel::AiChat => {
                    if state.preview_visible {
                        app::state::FocusedPanel::Preview
                    } else {
                        app::state::FocusedPanel::FileList
                    }
                }
            };
        }
        // ── Git menu ─────────────────────────────────────────────────────────
        Action::OpenGitMenu => {
            state.mode = app::state::AppMode::GitMenu { selected: 0 };
            state.preview_state = app::state::PreviewState::None;
        }
        Action::CloseGitMenu => {
            state.mode = app::state::AppMode::Normal;
        }
        Action::GitNavDown => {
            if let app::state::AppMode::GitMenu { selected } = &mut state.mode {
                if *selected < commands::git::N_GIT_OPS - 1 {
                    *selected += 1;
                }
            }
        }
        Action::GitNavUp => {
            if let app::state::AppMode::GitMenu { selected } = &mut state.mode {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
        }
        Action::RunGitItem => {
            if let app::state::AppMode::GitMenu { selected } = state.mode {
                let ops = commands::git::git_operations();
                if let Some(&op) = ops.get(selected) {
                    if op.has_form() {
                        // Show the parameter form (second screen)
                        let values = commands::git::default_values(op.params);
                        state.mode = app::state::AppMode::GitForm {
                            op_index: selected,
                            values,
                            focused: 0,
                        };
                    } else {
                        // Execute directly
                        let args = commands::git::build_args(op, &[]);
                        let dir = state.current_dir.clone();
                        let tx2 = tx.clone();
                        state.make_output.clear();
                        state.preview_state =
                            app::state::PreviewState::MakeOutput { output: vec![] };
                        state.preview_scroll = 0;
                        state.mode = app::state::AppMode::Normal;
                        tokio::spawn(async move {
                            let _ = commands::git::run_git(&args, &dir, tx2).await;
                        });
                    }
                }
            }
        }
        // ── Copy path ──────────────────────────────────────────────────────────
        Action::CopyRelPath => {
            if let Some(entry) = state.selected_entry() {
                let path = entry.path.clone();
                let rel = path
                    .strip_prefix(&state.current_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                match copy_to_clipboard(&rel) {
                    Ok(()) => state.status_message = Some(format!("Copied: {}", rel)),
                    Err(e) => state.status_message = Some(format!("Copy error: {}", e)),
                }
            }
        }
        Action::CopyAbsPath => {
            if let Some(entry) = state.selected_entry() {
                let abs = entry.path.to_string_lossy().to_string();
                match copy_to_clipboard(&abs) {
                    Ok(()) => state.status_message = Some(format!("Copied: {}", abs)),
                    Err(e) => state.status_message = Some(format!("Copy error: {}", e)),
                }
            }
        }
        // ── Open with system default ──────────────────────────────────────────
        Action::OpenWithSystem => {
            if let Some(entry) = state.selected_entry() {
                let path = entry.path.clone();
                match commands::open::open_with_system(&path) {
                    Ok(()) => {
                        state.status_message = Some(format!("Opened: {}", path.display()));
                    }
                    Err(e) => {
                        state.status_message = Some(format!("Open error: {}", e));
                    }
                }
            }
        }
        // ── New file ──────────────────────────────────────────────────────────
        Action::OpenNewFile => {
            state.mode = app::state::AppMode::NewFile {
                name: String::new(),
                from_clipboard: false,
            };
        }
        Action::OpenNewFileFromClipboard => {
            state.mode = app::state::AppMode::NewFile {
                name: String::new(),
                from_clipboard: true,
            };
        }
        Action::NewFileChar(c) => {
            if let app::state::AppMode::NewFile { name, .. } = &mut state.mode {
                name.push(*c);
            }
        }
        Action::NewFileBackspace => {
            if let app::state::AppMode::NewFile { name, .. } = &mut state.mode {
                name.pop();
            }
        }
        Action::CloseNewFile => {
            state.mode = app::state::AppMode::Normal;
        }
        Action::CreateNewFile => {
            let (name, from_clipboard) = if let app::state::AppMode::NewFile {
                name,
                from_clipboard,
            } = &state.mode
            {
                (name.clone(), *from_clipboard)
            } else {
                return;
            };
            state.mode = app::state::AppMode::Normal;
            if name.is_empty() {
                return;
            }
            let file_path = state.current_dir.join(&name);
            let dir = state.current_dir.clone();
            let tx2 = tx.clone();
            tokio::spawn(async move {
                let content: Vec<u8> = if from_clipboard {
                    commands::clipboard::read_clipboard().unwrap_or_default()
                } else {
                    vec![]
                };
                match tokio::fs::write(&file_path, &content).await {
                    Ok(()) => {
                        schedule_directory_load(dir, &tx2);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create file: {}", e);
                    }
                }
            });
        }
        // ── Git form (second screen) ─────────────────────────────────────────
        Action::OpenGitForm => {} // triggered internally via RunGitItem
        Action::CloseGitForm => {
            if let app::state::AppMode::GitForm { op_index, .. } = state.mode {
                state.mode = app::state::AppMode::GitMenu { selected: op_index };
            }
        }
        Action::GitFormTabNext => {
            if let app::state::AppMode::GitForm {
                ref mut focused,
                op_index,
                ..
            } = state.mode
            {
                let ops = commands::git::git_operations();
                if let Some(op) = ops.get(op_index) {
                    *focused = (*focused + 1) % op.params.len().max(1);
                }
            }
        }
        Action::GitFormTabPrev => {
            if let app::state::AppMode::GitForm {
                ref mut focused,
                op_index,
                ..
            } = state.mode
            {
                let ops = commands::git::git_operations();
                if let Some(op) = ops.get(op_index) {
                    let n = op.params.len().max(1);
                    *focused = focused.checked_sub(1).unwrap_or(n - 1);
                }
            }
        }
        Action::GitFormToggle => {
            if let app::state::AppMode::GitForm {
                focused,
                ref mut values,
                ..
            } = state.mode
            {
                if let Some(commands::git::GitParamValue::Bool(b)) = values.get_mut(focused) {
                    *b = !*b;
                }
            }
        }
        Action::GitFormChar(ch) => {
            if let app::state::AppMode::GitForm {
                focused,
                ref mut values,
                ..
            } = state.mode
            {
                if let Some(commands::git::GitParamValue::Text(s)) = values.get_mut(focused) {
                    s.push(*ch);
                }
            }
        }
        Action::GitFormBackspace => {
            if let app::state::AppMode::GitForm {
                focused,
                ref mut values,
                ..
            } = state.mode
            {
                if let Some(commands::git::GitParamValue::Text(s)) = values.get_mut(focused) {
                    s.pop();
                }
            }
        }
        Action::RunGitForm => {
            if let app::state::AppMode::GitForm {
                op_index,
                ref values,
                ..
            } = state.mode
            {
                let ops = commands::git::git_operations();
                if let Some(&op) = ops.get(op_index) {
                    let args = commands::git::build_args(op, values);
                    let dir = state.current_dir.clone();
                    let tx2 = tx.clone();
                    state.make_output.clear();
                    state.preview_state = app::state::PreviewState::MakeOutput { output: vec![] };
                    state.preview_scroll = 0;
                    state.mode = app::state::AppMode::Normal;
                    tokio::spawn(async move {
                        let _ = commands::git::run_git(&args, &dir, tx2).await;
                    });
                }
            }
        }
        // ── Help panel ────────────────────────────────────────────────────────
        Action::OpenHelp => {
            state.mode = app::state::AppMode::Help { scroll: 0 };
        }
        Action::CloseHelp => {
            state.mode = app::state::AppMode::Normal;
        }
        Action::HelpScrollDown => {
            if let app::state::AppMode::Help { scroll } = &mut state.mode {
                let max = ui::components::help::content_line_count().saturating_sub(20);
                *scroll = (*scroll + 3).min(max);
            }
        }
        Action::HelpScrollUp => {
            if let app::state::AppMode::Help { scroll } = &mut state.mode {
                *scroll = scroll.saturating_sub(3);
            }
        }
        // ── Settings ─────────────────────────────────────────────────────────
        Action::OpenSettings => {
            let entries = config::entries_from_config(&state.config);
            state.mode = app::state::AppMode::Settings {
                selected: 0,
                entries,
            };
        }
        Action::SettingsNavDown => {
            if let app::state::AppMode::Settings { selected, entries } = &mut state.mode {
                if *selected < entries.len().saturating_sub(1) {
                    *selected += 1;
                }
            }
        }
        Action::SettingsNavUp => {
            if let app::state::AppMode::Settings { selected, .. } = &mut state.mode {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
        }
        Action::SettingsIncrease => {
            if let app::state::AppMode::Settings { selected, entries } = &mut state.mode {
                if let Some(e) = entries.get_mut(*selected) {
                    e.increment();
                }
                config::apply_entries(&mut state.config, entries);
                config::refresh_derived(entries, &state.config);
            }
        }
        Action::SettingsDecrease => {
            if let app::state::AppMode::Settings { selected, entries } = &mut state.mode {
                if let Some(e) = entries.get_mut(*selected) {
                    e.decrement();
                }
                config::apply_entries(&mut state.config, entries);
                config::refresh_derived(entries, &state.config);
            }
        }
        Action::CloseSettings => {
            if let app::state::AppMode::Settings { entries, .. } = &state.mode {
                let entries_clone = entries.clone();
                config::apply_entries(&mut state.config, &entries_clone);
                let cfg = state.config.clone();
                if let Err(e) = config::save(&cfg) {
                    state.status_message = Some(format!("Settings save error: {}", e));
                }
            }
            state.mode = app::state::AppMode::Normal;
        }
        // ── AI integration ────────────────────────────────────────────────────
        Action::OpenAiPrompt => {
            if state.ai_provider.is_some() {
                state.ai_panel_visible = true;
                state.mode = app::state::AppMode::AiPrompt {
                    prompt: String::new(),
                };
            } else {
                let tx2 = tx.clone();
                tokio::spawn(async move {
                    let providers = commands::ai::detect_providers().await;
                    let _ = tx2.send(Event::AiProvidersDetected(providers));
                });
                state.mode = app::state::AppMode::AiProviderSelect { selected: 0 };
            }
        }
        Action::AiPromptChar(c) => {
            if let app::state::AppMode::AiPrompt { prompt } = &mut state.mode {
                prompt.push(*c);
            }
        }
        Action::AiPromptBackspace => {
            if let app::state::AppMode::AiPrompt { prompt } = &mut state.mode {
                prompt.pop();
            }
        }
        Action::CloseAiPrompt => {
            state.mode = app::state::AppMode::Normal;
        }
        Action::RunAiPrompt => {
            let prompt_text = if let app::state::AppMode::AiPrompt { prompt } = &state.mode {
                prompt.clone()
            } else {
                String::new()
            };
            if !prompt_text.is_empty() {
                if let Some(config) = &state.ai_provider {
                    let config = config.clone();
                    let context = commands::ai::build_context(
                        &state.current_dir,
                        state.selected_entry(),
                        &state.ai_conversation,
                    );
                    let tx2 = tx.clone();
                    state.ai_conversation.push(app::state::AiMessage {
                        role: app::state::AiRole::User,
                        text: prompt_text.clone(),
                    });
                    state.ai_conversation.push(app::state::AiMessage {
                        role: app::state::AiRole::Assistant,
                        text: String::new(),
                    });
                    state.ai_streaming = true;
                    state.ai_scroll = usize::MAX;
                    state.mode = app::state::AppMode::Normal;
                    state.focused_panel = app::state::FocusedPanel::AiChat;
                    let prompt_owned = prompt_text.clone();
                    let handle = tokio::spawn(async move {
                        commands::ai::run_ai_prompt(&config, &prompt_owned, &context, tx2).await;
                    });
                    state.ai_task_handle = Some(handle.abort_handle());
                } else {
                    state.status_message =
                        Some("No AI provider configured. Press I to select one.".to_string());
                    state.mode = app::state::AppMode::Normal;
                }
            } else {
                state.mode = app::state::AppMode::Normal;
            }
        }
        Action::OpenAiProviderSelect => {
            let tx2 = tx.clone();
            tokio::spawn(async move {
                let providers = commands::ai::detect_providers().await;
                let _ = tx2.send(Event::AiProvidersDetected(providers));
            });
            state.mode = app::state::AppMode::AiProviderSelect { selected: 0 };
            state.ai_providers.clear();
        }
        Action::AiProviderNavDown => {
            if let app::state::AppMode::AiProviderSelect { selected } = &mut state.mode {
                if *selected < state.ai_providers.len().saturating_sub(1) {
                    *selected += 1;
                }
            }
        }
        Action::AiProviderNavUp => {
            if let app::state::AppMode::AiProviderSelect { selected } = &mut state.mode {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
        }
        Action::SelectAiProvider => {
            let selected = if let app::state::AppMode::AiProviderSelect { selected } = state.mode {
                selected
            } else {
                0
            };
            if let Some(provider) = state.ai_providers.get(selected) {
                let config = commands::ai::AiProviderConfig {
                    provider_type: provider.provider_type.clone(),
                    model: provider.model.clone(),
                    endpoint: provider.endpoint.clone(),
                };
                if let Err(e) = commands::ai::save_config(&config) {
                    tracing::warn!("Failed to save AI config: {}", e);
                }
                state.ai_provider = Some(config);
                state.status_message = Some(format!("AI provider: {}", provider.display_name));
                state.ai_panel_visible = true;
                state.mode = app::state::AppMode::AiPrompt {
                    prompt: String::new(),
                };
            }
        }
        Action::CloseAiProviderSelect => {
            state.mode = app::state::AppMode::Normal;
        }
        Action::ToggleAiPanel => {
            state.ai_panel_visible = !state.ai_panel_visible;
            if state.ai_panel_visible {
                state.focused_panel = app::state::FocusedPanel::AiChat;
            } else {
                state.focused_panel = app::state::FocusedPanel::FileList;
            }
        }
        Action::AiScrollUp => {
            let max_scroll = state.ai_max_scroll;
            if state.ai_scroll > max_scroll {
                state.ai_scroll = max_scroll;
            }
            state.ai_scroll = state.ai_scroll.saturating_sub(3);
        }
        Action::AiScrollDown => {
            let max_scroll = state.ai_max_scroll;
            if state.ai_scroll > max_scroll {
                // Already at bottom — nothing to do.
            } else {
                state.ai_scroll = state.ai_scroll.saturating_add(3);
                if state.ai_scroll >= max_scroll {
                    state.ai_scroll = usize::MAX;
                }
            }
        }
        Action::ResetAiSession => {
            if let Some(handle) = state.ai_task_handle.take() {
                handle.abort();
            }
            state.ai_conversation.clear();
            state.ai_streaming = false;
            state.ai_scroll = usize::MAX;
            state.status_message = Some("AI session cleared".to_string());
        }
        // ── Command palette ───────────────────────────────────────────────────
        Action::OpenCommandPalette => {
            state.mode = app::state::AppMode::CommandPalette {
                query: String::new(),
                selected: 0,
            };
        }
        Action::CommandPaletteChar(c) => {
            if let app::state::AppMode::CommandPalette { query, selected } = &mut state.mode {
                query.push(*c);
                *selected = 0; // reset selection on new input
            }
        }
        Action::CommandPaletteBackspace => {
            if let app::state::AppMode::CommandPalette { query, selected } = &mut state.mode {
                query.pop();
                *selected = 0;
            }
        }
        Action::CommandPaletteNavDown => {
            if let app::state::AppMode::CommandPalette { query, selected } = &mut state.mode {
                let count = app::palette::filtered_items(query).len();
                if *selected + 1 < count {
                    *selected += 1;
                }
            }
        }
        Action::CommandPaletteNavUp => {
            if let app::state::AppMode::CommandPalette { query, selected } = &mut state.mode {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
        }
        Action::CloseCommandPalette => {
            state.mode = app::state::AppMode::Normal;
        }
        Action::RunCommandPaletteItem => {
            // Extract the selected action before changing mode.
            let chosen =
                if let app::state::AppMode::CommandPalette { query, selected } = &state.mode {
                    app::palette::filtered_items(query)
                        .into_iter()
                        .nth(*selected)
                        .map(|item| item.action)
                } else {
                    None
                };
            state.mode = app::state::AppMode::Normal;
            if let Some(action) = chosen {
                handle_action(&action, state, tx);
            }
        }
        Action::Noop => {}
    }
}

/// Handles an internal Event, updating state.
fn handle_event(event: Event, state: &mut AppState, tx: &UnboundedSender<Event>) {
    match event {
        Event::Key(key_event) => {
            // If a command is running, relay keystrokes to its stdin instead of navigating.
            if let Some(ref stdin_tx) = state.command_stdin {
                use crossterm::event::{KeyCode, KeyModifiers};
                let bytes: Option<Vec<u8>> = match key_event.code {
                    // Ctrl+C → send ETX (0x03); many programs treat this as cancel
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        Some(vec![0x03])
                    }
                    KeyCode::Enter => Some(vec![b'\n']),
                    KeyCode::Backspace => Some(vec![b'\x7f']),
                    KeyCode::Char(c) => {
                        // Encode the character as UTF-8 bytes
                        let mut buf = [0u8; 4];
                        let s = c.encode_utf8(&mut buf);
                        // Echo it in the preview so the user sees what they typed
                        if let app::state::PreviewState::MakeOutput { output } =
                            &mut state.preview_state
                        {
                            match output.last_mut() {
                                Some(last) if !last.ends_with('\n') => last.push(c),
                                _ => output.push(s.to_string()),
                            }
                        }
                        Some(s.as_bytes().to_vec())
                    }
                    // Ignore all other keys (e.g. arrows, function keys)
                    _ => None,
                };
                if let Some(b) = bytes {
                    let _ = stdin_tx.send(b);
                }
                return;
            }
            let action = map_key_to_action(&key_event, &state.mode, &state.focused_panel);
            handle_action(&action, state, tx);
        }
        Event::Resize(_, _) => {
            // Terminal resize — ratatui handles this automatically on next draw
        }
        Event::Tick => {
            // Periodic tick — clear transient status messages
            // (status messages are informational and clear naturally on the next user action)
        }
        Event::PreviewReady(preview_state) => {
            state.preview_state = preview_state;
            state.preview_scroll = 0; // Reset scroll on new preview
        }
        Event::MakeOutputLine(line) => {
            // Append directly to the PreviewState output to avoid cloning make_output.
            if let app::state::PreviewState::MakeOutput { output } = &mut state.preview_state {
                output.push(line.clone());
            }
            state.make_output.push(line);
        }
        Event::MakeDone { exit_code } => {
            state.make_cancel_tx = None; // task is done; clear the cancel channel
            let msg = if exit_code == 0 {
                "make completed successfully".to_string()
            } else if exit_code == -1 {
                "make cancelled".to_string()
            } else {
                format!("make exited with code {}", exit_code)
            };
            let footer = format!("\n[{}]", msg);
            if let app::state::PreviewState::MakeOutput { output } = &mut state.preview_state {
                output.push(footer.clone());
            }
            state.make_output.push(footer);
            state.status_message = Some(msg);
        }
        Event::DirectoryLoaded { path, entries } => {
            if path == state.current_dir {
                state.entries = entries;
                state.filtered_indices = (0..state.entries.len()).collect();
                // Re-apply search filter if active
                if !state.search_query.is_empty() {
                    search::fuzzy::apply_search(state);
                }
                // Reset selection and schedule preview for the first entry
                state.selected_index = 0;
                schedule_preview(state, tx);
            }
        }
        Event::HeaderInfoReady(info) => {
            state.header_info = info;
        }
        Event::GitStatusReady(status_map) => {
            state.git_file_status = status_map;
        }
        Event::CommandOutput { exit_code, .. } => {
            // Command finished: release the stdin pipe (signals EOF to child) and
            // re-enable normal keyboard navigation.
            state.command_stdin = None;
            // Append exit status to the preview.
            let done_line = if exit_code == 0 {
                "\n[done]".to_string()
            } else {
                format!("\n[exited with code {}]", exit_code)
            };
            if let app::state::PreviewState::MakeOutput { output } = &mut state.preview_state {
                output.push(done_line);
            }
        }
        Event::GitOutputLine(line) => {
            if let app::state::PreviewState::MakeOutput { output } = &mut state.preview_state {
                output.push(line.clone());
            }
            state.make_output.push(line);
        }
        Event::GitDone { exit_code } => {
            let done_line = if exit_code == 0 {
                "\n[done]".to_string()
            } else {
                format!("\n[exited with code {}]", exit_code)
            };
            if let app::state::PreviewState::MakeOutput { output } = &mut state.preview_state {
                output.push(done_line.clone());
            }
            state.make_output.push(done_line);
        }
        Event::AiOutputLine(text) => {
            // Append streamed text to the last assistant message in conversation.
            if let Some(last_msg) = state.ai_conversation.last_mut() {
                if last_msg.role == app::state::AiRole::Assistant {
                    last_msg.text.push_str(&text);
                }
            }
            // Auto-scroll to bottom while streaming.
            state.ai_scroll = usize::MAX;
        }
        Event::AiDone => {
            state.ai_streaming = false;
            // Add a status message to mark completion.
            state.ai_conversation.push(app::state::AiMessage {
                role: app::state::AiRole::Status,
                text: "[done]".to_string(),
            });
            state.ai_scroll = usize::MAX;
        }
        Event::AiProvidersDetected(providers) => {
            state.ai_providers = providers;
            // If we're in the provider select modal, ensure selection is valid.
            if let app::state::AppMode::AiProviderSelect { selected } = &mut state.mode {
                if *selected >= state.ai_providers.len() {
                    *selected = 0;
                }
            }
            if state.ai_providers.is_empty() {
                state.status_message =
                    Some("No AI providers detected. Set OPENAI_API_KEY, ANTHROPIC_API_KEY, install Ollama, or install Claude Code.".to_string());
                state.mode = app::state::AppMode::Normal;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging to file (don't pollute the terminal)
    let file_appender = tracing_appender::rolling::daily("/tmp", "fang.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("fang=debug".parse()?),
        )
        .init();

    setup_panic_hook();

    // ── Parse CLI arguments ───────────────────────────────────────────────────
    // Usage: fang [directory] [--with="cmd"] ...
    //   directory  Optional path to open (defaults to current directory).
    //   --with=CMD Open CMD in a background split alongside fang on startup.
    //              May be repeated: fang --with="claudec!" --with="mactop"
    let mut initial_dir_opt: Option<PathBuf> = None;
    let mut with_commands: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        if let Some(cmd) = arg.strip_prefix("--with=") {
            with_commands.push(cmd.to_string());
        } else if !arg.starts_with('-') {
            initial_dir_opt = Some(PathBuf::from(&arg));
        }
        // Unknown flags are silently ignored for forward-compatibility.
    }

    let initial_dir = {
        let raw = initial_dir_opt
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        // Make the path absolute without resolving symlinks.
        // canonicalize() resolves symlinks (e.g. /Users -> /private/Users on macOS),
        // which would make root_dir inconsistent with the paths returned by readdir,
        // breaking starts_with() comparisons and blocking all upward navigation.
        std::path::absolute(&raw).unwrap_or(raw)
    };

    tracing::info!("Fang starting in {:?}", initial_dir);

    // Load AI config before entering raw mode to avoid blocking the TUI on slow filesystems.
    let ai_config = commands::ai::load_config();

    // Initialize state
    // Load persisted config (sync read before TUI starts — acceptable for startup)
    let cfg = config::load();
    let mut state = AppState::new(initial_dir.clone(), ai_config);
    // Apply persisted panel visibility — p toggles this in-session without saving.
    state.preview_visible = cfg.layout.preview_visible;
    state.config = cfg;

    // Setup internal event channel (for async results from background tasks)
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Load initial directory asynchronously
    schedule_directory_load(initial_dir.clone(), &tx);
    state.preview_state = app::state::PreviewState::Loading;

    // Load header info (git branch + dev envs) for initial directory
    schedule_header_info_load(&initial_dir, &tx);

    // Set initial window title
    commands::title::set_window_title(&initial_dir);

    // Initial git file status
    schedule_git_status_load(&initial_dir, &tx);

    // Open --with splits in background (fang keeps focus).
    for cmd in &with_commands {
        if let Err(e) = commands::shell::open_in_split(cmd, &initial_dir, false) {
            tracing::warn!("--with '{}' failed: {}", cmd, e);
        }
    }

    // Setup event sources
    let mut crossterm_events = EventStream::new();
    let mut tick_timer = interval(Duration::from_millis(250));

    // Main event loop
    loop {
        // When navigation or preview-state transitions happen, the syntect-coloured cells
        // from the previous preview can survive ratatui's diff algorithm.  Calling
        // terminal.clear() forces a full repaint, eliminating any stale artefacts.
        if state.needs_terminal_clear {
            terminal.clear()?;
            state.needs_terminal_clear = false;
        }

        // Update AI panel dimensions and scroll metrics before drawing,
        // so the render path never needs &mut AppState.
        let term_size = terminal.size()?;
        let term_rect = ratatui::prelude::Rect::new(0, 0, term_size.width, term_size.height);
        ui::layout::update_ai_panel_dimensions(&mut state, term_rect);
        ui::components::ai_panel::update_max_scroll(&mut state);

        // Render current state
        terminal.draw(|f| ui::layout::draw(f, &state))?;

        if state.should_quit {
            break;
        }

        // Wait for the next event from any source
        tokio::select! {
            // Internal events (preview ready, make output, directory loaded)
            Some(event) = rx.recv() => {
                handle_event(event, &mut state, &tx);
            }

            // Crossterm events (keyboard, resize, mouse)
            Some(Ok(ct_event)) = crossterm_events.next() => {
                match ct_event {
                    CrosstermEvent::Key(key) => {
                        handle_event(Event::Key(key), &mut state, &tx);
                    }
                    CrosstermEvent::Resize(w, h) => {
                        state.needs_terminal_clear = true;
                        handle_event(Event::Resize(w, h), &mut state, &tx);
                    }
                    _ => {}
                }
            }

            // Periodic tick (250ms)
            _ = tick_timer.tick() => {
                handle_event(Event::Tick, &mut state, &tx);
            }
        }
    }

    restore_terminal(&mut terminal)?;
    tracing::info!("Fang exited cleanly");

    Ok(())
}
