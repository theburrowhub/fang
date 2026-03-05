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

    // Line 1: keybindings — varies by mode
    let keybinding_spans: Vec<Span<'_>> = match &state.mode {
        AppMode::Normal => {
            let mut s = Vec::new();
            s.extend(key_hint("j/k", "Nav"));
            s.extend(key_hint("h", "Up"));
            s.extend(key_hint("l", "Enter"));
            s.extend(key_hint("/", "Search"));
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
    };

    let line1_area = Rect { x: area.x, y: area.y, width: area.width, height: 1 };
    let line2_area = Rect { x: area.x, y: area.y + 1, width: area.width, height: 1 };

    frame.render_widget(
        Paragraph::new(Line::from(keybinding_spans)).style(Style::default().bg(Color::Reset)),
        line1_area,
    );

    // Line 2: current path + optional status message.
    // Use Cow from to_string_lossy directly — avoids an extra heap allocation.
    let path_cow = state.current_dir.to_string_lossy();
    let mut line2_spans: Vec<Span<'_>> = vec![
        Span::styled(path_cow.as_ref().to_owned(), Style::default().fg(Color::DarkGray)),
    ];

    if let Some(msg) = &state.status_message {
        line2_spans.push(Span::raw("  "));
        line2_spans.push(Span::styled("\u{2502} ", Style::default().fg(Color::DarkGray)));
        line2_spans.push(Span::styled(msg.as_str().to_owned(), Style::default().fg(Color::Yellow)));
    }

    frame.render_widget(Paragraph::new(Line::from(line2_spans)), line2_area);
}
