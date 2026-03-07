//! Settings modal — opened with `Ctrl+S`.
//!
//! Shows three rows (sidebar %, file list %, preview %) that always sum to 100.
//! Sidebar and file list are editable; preview is derived and shown in gray.
//! Navigation: j/k   |   +/→ increase   |   -/← decrease   |   Esc save & close

use crate::app::state::{AppMode, AppState};
use crate::ui::utils::key_hint;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let AppMode::Settings { selected, entries } = &state.mode else {
        return;
    };

    let modal_width = area.width.clamp(38, 52);
    let modal_height = ((entries.len() as u16) + 7).clamp(8, area.height.saturating_sub(4));
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
            " Panel sizes ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Span::styled(
            " total = 100 % ",
            Style::default().fg(Color::DarkGray),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let [list_area, sep_area, inst_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let name_width = 26usize;
    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            use crate::config::EntryKind;
            let is_sel = i == *selected;
            let is_editable = e.is_editable();

            let name_style = if is_editable {
                if is_sel {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                }
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let name_padded = format!("{:<width$}", e.description, width = name_width);

            let (value_span, tag_span) = match &e.kind {
                EntryKind::Toggle => {
                    let on = e.as_bool();
                    let (label, color) = if on {
                        (" on ", Color::Green)
                    } else {
                        ("off ", Color::DarkGray)
                    };
                    let style = if is_sel {
                        Style::default().fg(color).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(color)
                    };
                    (Span::styled(format!("[{}]", label), style), Span::raw(""))
                }
                EntryKind::Derived => {
                    let style = Style::default().fg(Color::DarkGray);
                    (
                        Span::styled(format!("{:>3} %", e.value), style),
                        Span::styled("  (auto)", style),
                    )
                }
                EntryKind::Editable { .. } => {
                    let style = if is_sel {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    (
                        Span::styled(format!("{:>3} %", e.value), style),
                        Span::raw(""),
                    )
                }
            };

            let line = Line::from(vec![
                Span::styled(name_padded, name_style),
                value_span,
                tag_span,
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
    frame.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(sep_area.width as usize),
            Style::default().fg(Color::DarkGray),
        )),
        sep_area,
    );

    // Instructions
    let mut inst = Vec::new();
    inst.extend(key_hint("+/→", "+1"));
    inst.extend(key_hint("-/←", "-1"));
    inst.extend(key_hint("j/k", "Nav"));
    inst.extend(key_hint("Esc", "Save"));
    frame.render_widget(
        Paragraph::new(Line::from(inst)).style(Style::default().bg(Color::Reset)),
        inst_area,
    );
}
