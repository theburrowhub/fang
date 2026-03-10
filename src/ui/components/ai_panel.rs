//! AI chat panel — renders the persistent conversation history.
//!
//! Displays user prompts, assistant responses, and status messages with
//! distinct colors. Supports scrolling with j/k when focused.

use crate::app::state::{AiMessage, AiRole, AppState, FocusedPanel};
use crate::ui::utils::panel_border_style;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Build the display lines from the AI conversation.
///
/// Extracted so it can be reused by both `render` and `update_max_scroll`.
fn build_display_lines(
    conversation: &[AiMessage],
    streaming: bool,
    inner_width: usize,
) -> Vec<Line<'static>> {
    let mut display: Vec<Line<'static>> = Vec::new();

    for msg in conversation {
        match msg.role {
            AiRole::User => {
                // Blank line before user message (unless first).
                if !display.is_empty() {
                    display.push(Line::from(""));
                }
                // User prompt header
                display.push(Line::from(vec![
                    Span::styled(
                        "\u{276f} ",
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        msg.text.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                // Separator after user prompt
                let sep: String = "\u{2500}".repeat(inner_width);
                display.push(Line::from(Span::styled(
                    sep,
                    Style::default().fg(Color::DarkGray),
                )));
            }
            AiRole::Assistant => {
                if msg.text.is_empty() && streaming {
                    display.push(Line::from(Span::styled(
                        "Thinking...",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    )));
                } else {
                    // Split response text into lines.
                    for line in msg.text.split('\n') {
                        display.push(Line::from(Span::styled(
                            line.to_string(),
                            Style::default().fg(Color::Cyan),
                        )));
                    }
                }
            }
            AiRole::Status => {
                let style = if msg.text.starts_with("[error") {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::ITALIC)
                };
                display.push(Line::from(Span::styled(msg.text.clone(), style)));
            }
        }
    }

    display
}

/// Recompute `ai_max_scroll` from the conversation and the cached panel dimensions.
///
/// Called from the event loop before each draw so the render path stays `&AppState`.
pub fn update_max_scroll(state: &mut AppState) {
    let width = state.ai_panel_width;
    let height = state.ai_panel_height;

    if width < 3 || height < 3 || state.ai_conversation.is_empty() {
        state.ai_max_scroll = 0;
        return;
    }

    let inner_width = (width - 2) as usize;
    let inner_height = (height - 2) as usize;

    let display = build_display_lines(&state.ai_conversation, state.ai_streaming, inner_width);
    let paragraph = Paragraph::new(display).wrap(Wrap { trim: false });
    let total_visual = paragraph.line_count(width - 2);

    state.ai_max_scroll = total_visual.saturating_sub(inner_height);
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    frame.render_widget(Clear, area);

    let is_focused = state.focused_panel == FocusedPanel::AiChat;
    let border_style = if is_focused {
        Style::default().fg(Color::Magenta)
    } else {
        panel_border_style(false)
    };

    let title = if state.ai_streaming {
        " AI (streaming...) "
    } else if state.ai_conversation.is_empty() {
        " AI "
    } else {
        " AI Chat "
    };

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let inner_width = inner.width as usize;

    // Empty state
    if state.ai_conversation.is_empty() {
        let hint_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Press [i] to ask the AI",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(Span::styled(
                "Press [I] to change provider",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(Span::styled(
                "Press [Ctrl+R] to reset session",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
        ];
        frame.render_widget(
            Paragraph::new(hint_lines).alignment(Alignment::Center),
            inner,
        );
        return;
    }

    // Build display lines from conversation.
    let display = build_display_lines(&state.ai_conversation, state.ai_streaming, inner_width);

    // Build paragraph with wrapping so we can query the real visual line count.
    let paragraph = Paragraph::new(display).wrap(Wrap { trim: false });

    // Read pre-computed max_scroll (updated by update_max_scroll before draw).
    let max_scroll = state.ai_max_scroll;

    // Scrolling — Paragraph::scroll uses visual (wrapped) line offsets.
    let scroll_y = if state.ai_scroll > max_scroll {
        // usize::MAX or beyond end → stick to bottom.
        max_scroll
    } else {
        state.ai_scroll
    };

    frame.render_widget(
        paragraph.scroll(((scroll_y.min(u16::MAX as usize)) as u16, 0)),
        inner,
    );
}
