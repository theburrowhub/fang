//! Footer — one-line bar at the bottom of the screen.
//!
//! Prompt modes (CommandInput, ExternalCommand, NewFile) replace the bar with
//! a vim-style inline prompt. All other modes render hints from
//! [`crate::app::keybindings::footer_bindings`] — the single source of truth
//! for key bindings, so the footer stays in sync automatically.

use crate::app::keybindings;
use crate::app::state::{AppMode, AppState};
use crate::ui::utils::key_hint;
use ratatui::{prelude::*, widgets::Paragraph};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if area.height == 0 {
        return;
    }
    let line_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };

    // ── Prompt modes: show an inline input line ───────────────────────────
    let prompt: Option<Line<'_>> = match &state.mode {
        AppMode::CommandInput { cmd } => Some(Line::from(vec![
            Span::styled(
                ": ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(cmd.as_str().to_owned(), Style::default().fg(Color::White)),
            Span::styled("\u{2588}", Style::default().fg(Color::Cyan)),
        ])),
        AppMode::ExternalCommand { cmd } => Some(Line::from(vec![
            Span::styled(
                "; ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(cmd.as_str().to_owned(), Style::default().fg(Color::White)),
            Span::styled("\u{2588}", Style::default().fg(Color::Green)),
        ])),
        AppMode::NewFile {
            name,
            from_clipboard,
        } => {
            let prefix = if *from_clipboard {
                "new (clipboard): "
            } else {
                "new: "
            };
            Some(Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(name.as_str().to_owned(), Style::default().fg(Color::White)),
                Span::styled("\u{2588}", Style::default().fg(Color::Yellow)),
            ]))
        }
        _ => None,
    };

    if let Some(line) = prompt {
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(Color::Reset)),
            line_area,
        );
        return;
    }

    // ── Hint bar: derived from keybindings registry ───────────────────────
    let mode_name = mode_str(&state.mode);
    let bindings = keybindings::footer_bindings(mode_name);

    let spans: Vec<Span<'_>> = if bindings.is_empty() {
        // Fallback for modes not listed in the registry
        vec![Span::styled(
            format!(" {} mode", mode_name),
            Style::default().fg(Color::DarkGray),
        )]
    } else {
        let mut s = Vec::new();
        for b in bindings {
            // Use the compact `short` label in the footer; full `description` is in Help.
            s.extend(key_hint(b.key, b.short));
        }
        s
    };

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Reset)),
        line_area,
    );
}

/// Map `AppMode` to the mode name used in the keybindings registry.
fn mode_str(mode: &AppMode) -> &'static str {
    match mode {
        AppMode::Normal => "Normal",
        AppMode::Search { .. } => "Search",
        AppMode::MakeTarget => "Make",
        AppMode::GitMenu { .. } => "Git",
        AppMode::GitForm { .. } => "GitForm",
        AppMode::CommandInput { .. } => "Command", // handled by prompt above
        AppMode::ExternalCommand { .. } => "Split", // handled by prompt above
        AppMode::NewFile { .. } => "NewFile",      // handled by prompt above
        AppMode::Settings { .. } => "Settings",
        AppMode::Help { .. } => "Help",
    }
}
