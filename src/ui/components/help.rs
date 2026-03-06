//! Full-screen Help panel — shows all registered key bindings grouped by mode.
//! Activated with `h`, closed with `h` or `Esc`.

use crate::app::keybindings;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// How many lines does the help content span?
pub fn content_line_count() -> usize {
    build_lines().len()
}

pub fn render(frame: &mut Frame, area: Rect, scroll: usize) {
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(
            " Help — Key Bindings ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " [h / Esc] close   [j/k  PgUp/PgDn] scroll ",
            Style::default().fg(Color::DarkGray),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = build_lines();
    let max_scroll = lines.len().saturating_sub(inner.height as usize);
    let scroll = scroll.min(max_scroll);

    let visible: Vec<Line<'static>> = lines.into_iter().skip(scroll).collect();
    frame.render_widget(Paragraph::new(visible).wrap(Wrap { trim: false }), inner);
}

// ── Build the content lines dynamically from keybindings::all_bindings() ─────

fn build_lines() -> Vec<Line<'static>> {
    let all = keybindings::all_bindings();

    // Collect unique mode names preserving insertion order
    let mut modes: Vec<&'static str> = Vec::new();
    for b in &all {
        if !modes.contains(&b.mode) {
            modes.push(b.mode);
        }
    }

    let key_col = 20usize; // width of the key column

    let mut lines: Vec<Line<'static>> = Vec::new();

    for mode in modes {
        // Section header
        lines.push(Line::from(vec![])); // blank separator
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", mode),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", "─".repeat(36)),
            Style::default().fg(Color::DarkGray),
        )]));

        for b in all.iter().filter(|b| b.mode == mode) {
            let key_padded = format!("  {:<width$}", b.key, width = key_col);
            lines.push(Line::from(vec![
                Span::styled(key_padded, Style::default().fg(Color::Yellow)),
                Span::styled(b.description, Style::default().fg(Color::White)),
            ]));
        }
    }

    // Remove the leading blank line
    if lines.first().map(|l| l.spans.is_empty()).unwrap_or(false) {
        lines.remove(0);
    }

    lines
}
