use crate::app::state::AppState;
use crate::commands::git::git_operations;
use crate::ui::utils::key_hint;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let selected = if let crate::app::state::AppMode::GitMenu { selected } = state.mode {
        selected
    } else {
        0
    };

    let block = Block::default()
        .title(Span::styled(
            " Git ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 {
        return;
    }

    let instructions_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };
    let list_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height.saturating_sub(2),
    };
    let sep_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(2),
        width: inner.width,
        height: 1,
    };

    // Separator line
    let sep_line: String = "\u{2500}".repeat(sep_area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(sep_line, Style::default().fg(Color::DarkGray))),
        sep_area,
    );

    // Instructions built with the shared key_hint helper
    let mut inst_spans = Vec::new();
    inst_spans.extend(key_hint("Enter", "Run"));
    inst_spans.extend(key_hint("Esc", "Cancel"));
    inst_spans.extend(key_hint("j/k", "Navigate"));
    frame.render_widget(Paragraph::new(Line::from(inst_spans)), instructions_area);

    let ops = git_operations();

    let items: Vec<ListItem> = ops
        .iter()
        .enumerate()
        .map(|(i, op)| {
            let is_selected = i == selected;

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            let cmd_display = format!("git {}", op.args.join(" "));
            let lines = vec![
                Line::from(vec![
                    Span::styled("\u{2023} ", Style::default().fg(Color::Cyan)),
                    Span::styled(op.label, name_style),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(cmd_display, Style::default().fg(Color::DarkGray)),
                ]),
            ];

            ListItem::new(lines)
        })
        .collect();

    let highlight_style = Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);

    let list = List::new(items).highlight_style(highlight_style);
    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    frame.render_stateful_widget(list, list_area, &mut list_state);
}
