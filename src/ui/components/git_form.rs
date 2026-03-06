//! Second-screen git form: parameter inputs for operations that need them.
//! Shown when the user selects an operation with `params` from git_modal.

use crate::app::state::{AppMode, AppState};
use crate::commands::git::{GitParamKind, GitParamValue, git_operations};
use crate::ui::utils::key_hint;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let AppMode::GitForm { op_index, ref values, focused } = state.mode else {
        return;
    };

    let ops = git_operations();
    let Some(op) = ops.get(op_index) else { return };

    // Modal size
    let rows = op.params.len() as u16;
    let modal_width = area.width.clamp(44, 68);
    let modal_height = (rows + 6).clamp(8, area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect { x, y, width: modal_width, height: modal_height };

    frame.render_widget(Clear, modal_area);

    let title = format!(" git {} ", op.base_args.join(" "));
    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Layout: param rows + separator + instructions
    let param_height = rows.min(inner.height.saturating_sub(2));
    let params_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: param_height };
    let sep_area   = Rect { x: inner.x, y: inner.y + param_height, width: inner.width, height: 1 };
    let inst_area  = Rect { x: inner.x, y: inner.y + param_height + 1, width: inner.width, height: 1 };

    // Separator
    frame.render_widget(
        Paragraph::new(Span::styled("─".repeat(sep_area.width as usize), Style::default().fg(Color::DarkGray))),
        sep_area,
    );

    // Instructions
    let mut inst = Vec::new();
    inst.extend(key_hint("Tab", "Next"));
    inst.extend(key_hint("Space", "Toggle"));
    inst.extend(key_hint("Enter", "Run"));
    inst.extend(key_hint("Esc", "Back"));
    frame.render_widget(Paragraph::new(Line::from(inst)), inst_area);

    // Parameter rows
    for (i, (param_def, value)) in op.params.iter().zip(values.iter()).enumerate() {
        let row = Rect {
            x: params_area.x,
            y: params_area.y + i as u16,
            width: params_area.width,
            height: 1,
        };
        if row.y >= inner.y + inner.height { break; }

        let is_focused = i == focused;

        match (&param_def.kind, value) {
            // Text input
            (GitParamKind::Text { placeholder, .. }, GitParamValue::Text(text)) => {
                let label_style = if is_focused {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                let label = format!("{:<22}", param_def.label);
                let display = if text.is_empty() {
                    Span::styled(placeholder.to_string(), Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
                } else {
                    Span::styled(text.clone(), Style::default().fg(Color::White))
                };
                let cursor = if is_focused {
                    Span::styled("█", Style::default().fg(Color::Yellow))
                } else {
                    Span::raw("")
                };
                let line = Line::from(vec![
                    Span::styled(if is_focused { "> " } else { "  " }, Style::default().fg(Color::Yellow)),
                    Span::styled(label, label_style),
                    display,
                    cursor,
                ]);
                frame.render_widget(Paragraph::new(line), row);
            }
            // Bool toggle / checkbox
            (GitParamKind::Bool { .. }, GitParamValue::Bool(checked)) => {
                let checkbox = if *checked { "[x]" } else { "[ ]" };
                let box_style = if *checked {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let label_style = if is_focused {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                let line = Line::from(vec![
                    Span::styled(if is_focused { "> " } else { "  " }, Style::default().fg(Color::Yellow)),
                    Span::styled(checkbox, box_style),
                    Span::raw(" "),
                    Span::styled(param_def.label, label_style),
                ]);
                frame.render_widget(Paragraph::new(line), row);
            }
            _ => {}
        }
    }
}
