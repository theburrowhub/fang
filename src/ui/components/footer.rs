use ratatui::{
    prelude::*,
    widgets::Paragraph,
};
use crate::app::state::{AppMode, AppState};
use crate::ui::utils::key_hint;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if area.height < 2 {
        return;
    }

    let line1_area = Rect { x: area.x, y: area.y, width: area.width, height: 1 };
    let line2_area = Rect { x: area.x, y: area.y + 1, width: area.width, height: 1 };

    // CommandInput gets its own rendering: vim-style command line on line 1, hints on line 2.
    if let AppMode::CommandInput { cmd } = &state.mode {
        let cmd_line = Line::from(vec![
            Span::styled(": ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(cmd.as_str().to_owned(), Style::default().fg(Color::White)),
            Span::styled("\u{2588}", Style::default().fg(Color::Cyan)),  // block cursor indicator
        ]);
        frame.render_widget(
            Paragraph::new(cmd_line).style(Style::default().bg(Color::Reset)),
            line1_area,
        );

        let mut hint_spans = Vec::new();
        hint_spans.extend(key_hint("Enter", "Run"));
        hint_spans.extend(key_hint("Esc", "Cancel"));
        hint_spans.extend(key_hint("Ctrl+C", "Quit"));
        frame.render_widget(
            Paragraph::new(Line::from(hint_spans)).style(Style::default().bg(Color::Reset)),
            line2_area,
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
            s.extend(key_hint("m", "Make"));
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
        // CommandInput is handled above via early return; this arm is unreachable but required
        // by the exhaustiveness checker.
        AppMode::CommandInput { .. } => vec![],
    };

    frame.render_widget(
        Paragraph::new(Line::from(keybinding_spans)).style(Style::default().bg(Color::Reset)),
        line1_area,
    );

    // Line 2: status message only (path is now shown in the header bar).
    let line2_spans: Vec<Span<'_>> = if let Some(msg) = &state.status_message {
        vec![Span::styled(msg.as_str().to_owned(), Style::default().fg(Color::Yellow))]
    } else {
        vec![]
    };

    frame.render_widget(Paragraph::new(Line::from(line2_spans)), line2_area);
}
