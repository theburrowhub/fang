//! Settings modal — opened with `Ctrl+S`.
//!
//! Displays all editable settings as a list. Navigation: j/k, +/- or ←/→
//! to change values, Esc to save and close.

use crate::app::state::{AppMode, AppState};
use crate::config;
use crate::ui::utils::{key_hint, panel_border_style};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let AppMode::Settings { selected, entries } = &state.mode else {
        return;
    };

    let modal_width = area.width.min(64).max(40);
    // 2 header + entries + 2 footer + 2 borders = entries.len() + 6
    let modal_height = ((entries.len() as u16) + 6).min(area.height - 4).max(8);
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect {
        x,
        y,
        width: modal_width,
        height: modal_height,
    };

    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(Span::styled(
            " Settings ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Split inner: list area + instructions row
    let [list_area, sep_area, inst_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    // Build list items
    let value_col_start = 36usize;
    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let is_sel = i == *selected;
            let key_style = if is_sel {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            let val_style = if is_sel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            // Pad key to align value column
            let key_part = format!("{:<width$}", e.description, width = value_col_start - 2);
            let val_part = format!("{:>4}", e.value.0);
            let unit_part = if e.key.contains("pct") { " %" } else { " cols" };
            let line = Line::from(vec![
                Span::styled(key_part, key_style),
                Span::styled(val_part, val_style),
                Span::styled(unit_part, Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    list_state.select(Some(*selected));
    frame.render_stateful_widget(list, list_area, &mut list_state);

    // Separator
    let sep_width = sep_area.width as usize;
    frame.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(sep_width),
            Style::default().fg(Color::DarkGray),
        )),
        sep_area,
    );

    // Instructions
    let mut inst = Vec::new();
    inst.extend(key_hint("+/→", "Increase"));
    inst.extend(key_hint("-/←", "Decrease"));
    inst.extend(key_hint("j/k", "Navigate"));
    inst.extend(key_hint("Esc", "Save & close"));
    frame.render_widget(
        Paragraph::new(Line::from(inst)).style(Style::default().bg(Color::Reset)),
        inst_area,
    );
}
