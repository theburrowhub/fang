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

    // CommandInput / ExternalCommand: show a vim-style prompt on the footer line.
    let prompt_line: Option<Line<'_>> = match &state.mode {
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
    if let Some(line) = prompt_line {
        frame.render_widget(
            Paragraph::new(line).style(Style::default().bg(Color::Reset)),
            line_area,
        );
        return;
    }

    // Line 1: keybindings — varies by mode
    let keybinding_spans: Vec<Span<'_>> = match &state.mode {
        AppMode::Normal if state.focused_panel == FocusedPanel::AiChat => {
            let mut s = Vec::new();
            s.extend(key_hint("j/k", "Scroll"));
            s.extend(key_hint("i", "Ask AI"));
            s.extend(key_hint("I", "Provider"));
            s.extend(key_hint("Ctrl+R", "Reset"));
            s.extend(key_hint("a", "Hide"));
            s.extend(key_hint("Tab", "Panel"));
            s.extend(key_hint("q", "Quit"));
            s
        }
        AppMode::Normal => {
            let mut s = Vec::new();
            s.extend(key_hint("j/k", "Nav"));
            s.extend(key_hint("h", "Up"));
            s.extend(key_hint("l", "Enter"));
            s.extend(key_hint("/", "Search"));
            s.extend(key_hint(":", "Cmd"));
            s.extend(key_hint(";", "Split"));
            s.extend(key_hint("m", "Make"));
            s.extend(key_hint("g", "Git"));
            s.extend(key_hint("o", "Open"));
            s.extend(key_hint("n", "New"));
            s.extend(key_hint("a", "AI Panel"));
            s.extend(key_hint("i", "AI"));
            s.extend(key_hint("I", "AI Cfg"));
            s.extend(key_hint("Ctrl+R", "AI Reset"));
            s.extend(key_hint("Tab", "Panel"));
            s.extend(key_hint("q", "Quit"));
            s
        }
        AppMode::Search { .. } => {
            let mut s = Vec::new();
            s.extend(key_hint("Enter", "Select"));
            s.extend(key_hint("Esc", "Cancel"));
            s.extend(key_hint("\u{2191}\u{2193}", "Navigate"));
            s
        }
        AppMode::MakeTarget => {
            let mut s = Vec::new();
            s.extend(key_hint("Enter", "Run"));
            s.extend(key_hint("Esc", "Cancel"));
            s.extend(key_hint("j/k", "Navigate"));
            s
        }
        AppMode::GitMenu { .. } => {
            let mut s = Vec::new();
            s.extend(key_hint("Enter", "Run"));
            s.extend(key_hint("Esc", "Cancel"));
            s.extend(key_hint("j/k", "Navigate"));
            s
        }
        AppMode::AiProviderSelect { .. } => {
            let mut s = Vec::new();
            s.extend(key_hint("Enter", "Select"));
            s.extend(key_hint("Esc", "Cancel"));
            s.extend(key_hint("j/k", "Navigate"));
            s
        }
        // CommandInput / ExternalCommand / NewFile / AiPrompt handled above via early return.
        AppMode::CommandInput { .. }
        | AppMode::ExternalCommand { .. }
        | AppMode::NewFile { .. }
        | AppMode::AiPrompt { .. } => vec![],
    };

    frame.render_widget(
        Paragraph::new(Line::from(keybinding_spans)).style(Style::default().bg(Color::Reset)),
        line_area,
    );
}
