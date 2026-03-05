use ratatui::{
    prelude::*,
    widgets::Paragraph,
};
use crate::app::state::{AppMode, AppState};
use crate::ui::utils::key_hint;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if area.height == 0 {
        return;
    }

    let line_area = Rect { x: area.x, y: area.y, width: area.width, height: 1 };

    // CommandInput / ExternalCommand / NewFile: show a vim-style prompt on the footer line.
    let prompt_line: Option<Line<'_>> = match &state.mode {
        AppMode::CommandInput { cmd } => Some(Line::from(vec![
            Span::styled(": ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(cmd.as_str().to_owned(), Style::default().fg(Color::White)),
            Span::styled("\u{2588}", Style::default().fg(Color::Cyan)),
        ])),
        AppMode::ExternalCommand { cmd } => Some(Line::from(vec![
            Span::styled("; ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(cmd.as_str().to_owned(), Style::default().fg(Color::White)),
            Span::styled("\u{2588}", Style::default().fg(Color::Green)),
        ])),
        AppMode::NewFile { name, from_clipboard } => {
            let prefix = if *from_clipboard {
                "new (clipboard): "
            } else {
                "new: "
            };
            Some(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(name.as_str().to_owned(), Style::default().fg(Color::White)),
                Span::styled("\u{2588}", Style::default().fg(Color::Green)),
            ]))
        }
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
        AppMode::Normal => {
            let mut s = Vec::new();
            s.extend(key_hint("j/k", "Nav"));
            s.extend(key_hint("h", "Up"));
            s.extend(key_hint("l", "Enter"));
            s.extend(key_hint("/", "Search"));
            s.extend(key_hint(":", "Cmd"));
            s.extend(key_hint(";", "Split"));
            s.extend(key_hint("m", "Make"));
            s.extend(key_hint("n", "New"));
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
        // CommandInput / ExternalCommand / NewFile handled above via early return.
        AppMode::CommandInput { .. } | AppMode::ExternalCommand { .. } | AppMode::NewFile { .. } => vec![],
    };

    frame.render_widget(
        Paragraph::new(Line::from(keybinding_spans)).style(Style::default().bg(Color::Reset)),
        line_area,
    );
}
