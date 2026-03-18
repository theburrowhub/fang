//! Spotlight-style command palette popup (Ctrl+K).
//!
//! A centered floating modal with a search box at the top and a filtered
//! list of all available commands below.  Selecting an item with Enter
//! closes the palette and dispatches the associated action.

use crate::app::palette::filtered_items;
use crate::app::state::{AppMode, AppState};
use crate::ui::utils::key_hint;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let (query, selected) = if let AppMode::CommandPalette { query, selected } = &state.mode {
        (query.as_str(), *selected)
    } else {
        return;
    };

    let items = filtered_items(query);

    // ── Size & position ───────────────────────────────────────────────────────
    let modal_width = (area.width * 3 / 4).clamp(52, 80);
    // 1 border + 1 search row + 1 sep + list rows + 1 sep + 1 hint + 1 border
    let list_rows = (items.len() as u16).min(12);
    let modal_height = (list_rows + 6).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 4; // sit in upper third
    let modal_area = Rect {
        x,
        y,
        width: modal_width,
        height: modal_height,
    };

    frame.render_widget(Clear, modal_area);

    // ── Outer border ─────────────────────────────────────────────────────────
    let block = Block::default()
        .title(Span::styled(
            " Command Palette ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    if inner.height < 4 {
        return;
    }

    // ── Layout: search | sep | list | sep | hints ─────────────────────────
    let search_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    let sep1_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: 1,
    };
    let hints_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };
    let sep2_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(2),
        width: inner.width,
        height: 1,
    };
    let list_area = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: inner.height.saturating_sub(4),
    };

    // ── Search row ────────────────────────────────────────────────────────────
    let search_line = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled("\u{1F50D} ", Style::default().fg(Color::DarkGray)),
        Span::styled(query.to_string(), Style::default().fg(Color::White)),
        Span::styled("\u{2588}", Style::default().fg(Color::Cyan)), // block cursor
    ]);
    frame.render_widget(Paragraph::new(search_line), search_area);

    // ── Separators ────────────────────────────────────────────────────────────
    let sep: String = "\u{2500}".repeat(inner.width as usize);
    let sep_style = Style::default().fg(Color::DarkGray);
    frame.render_widget(
        Paragraph::new(Span::styled(sep.clone(), sep_style)),
        sep1_area,
    );
    frame.render_widget(Paragraph::new(Span::styled(sep, sep_style)), sep2_area);

    // ── Hints ─────────────────────────────────────────────────────────────────
    let mut hint_spans = Vec::new();
    hint_spans.extend(key_hint("Enter", "Run"));
    hint_spans.extend(key_hint("↑↓ / j/k", "Navigate"));
    hint_spans.extend(key_hint("Esc", "Close"));
    frame.render_widget(Paragraph::new(Line::from(hint_spans)), hints_area);

    // ── Empty state ───────────────────────────────────────────────────────────
    if items.is_empty() {
        let msg = Paragraph::new(Span::styled(
            "  No matching commands",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));
        frame.render_widget(msg, list_area);
        return;
    }

    // ── Command list ──────────────────────────────────────────────────────────
    let hint_col_width = 10usize;
    let label_width = (inner.width as usize).saturating_sub(hint_col_width + 4);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_sel = i == selected;

            let label_style = if is_sel {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };
            let hint_style = if is_sel {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            // Truncate label to fit
            let label: String = if item.label.len() > label_width {
                format!("{}…", &item.label[..label_width.saturating_sub(1)])
            } else {
                item.label.to_string()
            };
            // Right-pad so the hint is right-aligned
            let padding = label_width.saturating_sub(label.len());

            let line = Line::from(vec![
                Span::styled(if is_sel { " ▶ " } else { "   " }, label_style),
                Span::styled(label, label_style),
                Span::styled(" ".repeat(padding + 1), label_style),
                Span::styled(item.hint.to_string(), hint_style),
                Span::styled(" ", label_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    let list = List::new(list_items).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_stateful_widget(list, list_area, &mut list_state);
}
