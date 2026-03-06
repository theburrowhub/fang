//! AI provider selection modal.
//!
//! Renders a floating modal listing all detected AI providers, allowing the user
//! to pick one with j/k + Enter.  Follows the same visual pattern as `git_modal`.

use crate::app::state::AppState;
use crate::ui::utils::key_hint;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

/// Number of providers currently detected (used for modal sizing in layout.rs).
pub fn provider_count(state: &AppState) -> usize {
    state.ai_providers.len()
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let selected = if let crate::app::state::AppMode::AiProviderSelect { selected } = state.mode {
        selected
    } else {
        0
    };

    // Clear area behind modal
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(
            " AI Provider ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

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

    // Instructions
    let mut inst_spans = Vec::new();
    inst_spans.extend(key_hint("Enter", "Select"));
    inst_spans.extend(key_hint("Esc", "Cancel"));
    inst_spans.extend(key_hint("j/k", "Navigate"));
    frame.render_widget(Paragraph::new(Line::from(inst_spans)), instructions_area);

    // Provider list
    if state.ai_providers.is_empty() {
        let msg = Paragraph::new(Span::styled(
            "  Detecting providers...",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));
        frame.render_widget(msg, list_area);
        return;
    }

    let items: Vec<ListItem> = state
        .ai_providers
        .iter()
        .enumerate()
        .map(|(i, provider)| {
            let is_selected = i == selected;

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            let type_label = match provider.provider_type {
                crate::commands::ai::AiProviderType::ClaudeCli => "CLI",
                crate::commands::ai::AiProviderType::Ollama => "HTTP",
                crate::commands::ai::AiProviderType::OpenAiApi => "API",
                crate::commands::ai::AiProviderType::AnthropicApi => "API",
            };

            let lines = vec![
                Line::from(vec![
                    Span::styled("\u{2023} ", Style::default().fg(Color::Magenta)),
                    Span::styled(&provider.display_name, name_style),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        format!("[{}] {}", type_label, provider.model),
                        Style::default().fg(Color::DarkGray),
                    ),
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
