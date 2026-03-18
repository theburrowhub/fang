//! Footer — one-line bar at the bottom of the screen.
//!
//! Prompt modes (CommandInput, ExternalCommand, NewFile, AiPrompt) replace the
//! bar with a vim-style inline prompt. All other modes render hints from
//! [`crate::app::keybindings::footer_bindings`] — the single source of truth
//! for key bindings, so the footer stays in sync automatically.
//!
//! Special case: when the AI chat panel is focused in Normal mode, we show
//! AI-specific hints from the "AiChat" pseudo-mode instead of the full Normal
//! hints.

use crate::app::keybindings;
use crate::app::state::{AppMode, AppState, FocusedPanel};
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
        AppMode::ExternalCommand { cmd } => {
            let mut spans = vec![
                Span::styled(
                    "; ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(cmd.as_str().to_owned(), Style::default().fg(Color::White)),
                Span::styled("\u{2588}", Style::default().fg(Color::Green)),
                Span::raw("  "),
            ];
            spans.extend(key_hint("Enter", "Split"));
            spans.extend(key_hint("^P", "Popup"));
            spans.extend(key_hint("Esc", "\u{2715}"));
            Some(Line::from(spans))
        }
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
        AppMode::AiPrompt { prompt } => Some(Line::from(vec![
            Span::styled(
                "ai: ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                prompt.as_str().to_owned(),
                Style::default().fg(Color::White),
            ),
            Span::styled("\u{2588}", Style::default().fg(Color::Magenta)),
        ])),
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
    // While a make target is running, replace the Normal-mode hints with a
    // prominent cancel reminder so the user knows they can press Esc.
    if state.mode == AppMode::Normal && state.make_cancel_tx.is_some() {
        let mut spans = Vec::new();
        spans.extend(key_hint("Esc", "Cancel make"));
        spans.extend(key_hint("j/k", "Scroll"));
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Reset)),
            line_area,
        );
        return;
    }

    // When the AI chat panel is focused we show "AiChat" bindings instead of "Normal".
    let mode_name = if state.mode == AppMode::Normal && state.focused_panel == FocusedPanel::AiChat
    {
        "AiChat"
    } else {
        mode_str(&state.mode)
    };
    let bindings = keybindings::footer_bindings(mode_name);

    let spans: Vec<Span<'_>> = if bindings.is_empty() {
        vec![Span::styled(
            format!(" {} mode", mode_name),
            Style::default().fg(Color::DarkGray),
        )]
    } else {
        let mut s = Vec::new();
        for b in bindings {
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
        AppMode::CommandInput { .. } => "Command",
        AppMode::ExternalCommand { .. } => "Split",
        AppMode::NewFile { .. } => "NewFile",
        AppMode::Settings { .. } => "Settings",
        AppMode::Help { .. } => "Help",
        AppMode::AiPrompt { .. } => "AiPrompt",
        AppMode::AiProviderSelect { .. } => "AiProvider",
        AppMode::CommandPalette { .. } => "CommandPalette",
    }
}
