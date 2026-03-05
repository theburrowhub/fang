use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use crate::app::state::{AppState, FocusedPanel, PreviewState, StyledLine};
use crate::ui::utils::{format_size_verbose, panel_border_style};

/// Convert a StyledLine into a ratatui Line.
fn styled_line_to_line(sl: &StyledLine) -> Line<'static> {
    let spans: Vec<Span<'static>> = sl
        .spans
        .iter()
        .map(|(style, text)| Span::styled(text.clone(), *style))
        .collect();
    Line::from(spans)
}

/// Render a titled block with a single centered message inside it.
fn render_centered_msg(
    frame: &mut Frame,
    area: Rect,
    border_style: Style,
    title: &str,
    msg: &'static str,
    msg_color: Color,
) {
    let block = Block::default()
        .title(Span::styled(title.to_owned(), Style::default().fg(Color::White)))
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Use ratatui's built-in vertical centering by padding with empty lines
    let top_pad = inner.height / 2;
    let mut lines = vec![Line::from(""); top_pad as usize];
    lines.push(Line::from(Span::styled(msg, Style::default().fg(msg_color))));
    let para = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(para, inner);
}

/// Render a titled block and return the inner rect for further content rendering.
fn render_block(
    frame: &mut Frame,
    area: Rect,
    border_style: Style,
    title: String,
) -> Rect {
    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(Color::White)))
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let border_style = panel_border_style(state.focused_panel == FocusedPanel::Preview);

    match &state.preview_state {
        PreviewState::None => {
            render_centered_msg(
                frame, area, border_style,
                " Preview ",
                "Select a file to preview",
                Color::DarkGray,
            );
        }

        PreviewState::Loading => {
            render_centered_msg(
                frame, area, border_style,
                " Preview ",
                "Loading...",
                Color::DarkGray,
            );
        }

        PreviewState::Text { lines, total_lines } => {
            let title = if *total_lines > lines.len() {
                format!(" Preview ({}/{} lines) ", lines.len(), total_lines)
            } else {
                format!(" Preview ({} lines) ", total_lines)
            };
            let inner = render_block(frame, area, border_style, title);

            let inner_height = inner.height as usize;
            let scroll = state
                .preview_scroll
                .min(lines.len().saturating_sub(inner_height));

            let visible_lines: Vec<Line<'static>> = lines
                .iter()
                .skip(scroll)
                .take(inner_height)
                .map(styled_line_to_line)
                .collect();

            frame.render_widget(Paragraph::new(visible_lines), inner);
        }

        PreviewState::Binary { size, mime_hint } => {
            let hint_str = mime_hint.as_deref().unwrap_or("unknown");
            let inner = render_block(
                frame, area, border_style,
                format!(" Binary File ({}) ", hint_str),
            );

            let text = vec![
                Line::from(vec![
                    Span::styled("Type:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(hint_str.to_owned(), Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled("Size:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format_size_verbose(*size), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Binary file \u{2014} cannot display as text.",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            frame.render_widget(Paragraph::new(text), inner);
        }

        PreviewState::Directory { entry_count, total_size } => {
            let inner = render_block(frame, area, border_style, " Directory Info ".to_owned());

            let text = vec![
                Line::from(vec![
                    Span::styled("Entries:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        entry_count.to_string(),
                        Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Total size:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format_size_verbose(*total_size), Style::default().fg(Color::White)),
                ]),
            ];
            frame.render_widget(Paragraph::new(text), inner);
        }

        PreviewState::MakeOutput { output } => {
            let inner = render_block(frame, area, border_style, " Make Output ".to_owned());

            let inner_height = inner.height as usize;
            let scroll = state
                .preview_scroll
                .min(output.len().saturating_sub(inner_height));

            let lines: Vec<Line<'static>> = output
                .iter()
                .skip(scroll)
                .take(inner_height)
                .map(|s| {
                    let style = if s.contains("error") || s.contains("Error") {
                        Style::default().fg(Color::Red)
                    } else if s.contains("warning") || s.contains("Warning") {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::from(Span::styled(s.clone(), style))
                })
                .collect();

            frame.render_widget(Paragraph::new(lines), inner);
        }

        PreviewState::TooLarge { size } => {
            let inner = render_block(frame, area, border_style, " Preview ".to_owned());

            let mb = *size as f64 / (1024.0 * 1024.0);
            let msg = format!("File too large to preview: {:.1} MB", mb);
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(msg, Style::default().fg(Color::Yellow))),
                Line::from(""),
                Line::from(Span::styled(
                    "Use an external editor to view this file.",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            frame.render_widget(Paragraph::new(text).alignment(Alignment::Center), inner);
        }

        PreviewState::Error(msg) => {
            let block = Block::default()
                .title(Span::styled(
                    " Preview \u{2014} Error ",
                    Style::default().fg(Color::Red),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red));
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let text = vec![
                Line::from(""),
                Line::from(Span::styled(msg.clone(), Style::default().fg(Color::Red))),
            ];
            frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), inner);
        }
    }
}
