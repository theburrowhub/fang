use crate::app::state::{AppState, FocusedPanel, PreviewState, StyledLine};
use crate::ui::utils::{format_size_verbose, panel_border_style};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Convert a StyledLine into a ratatui Line, padded to `width` so every cell
/// in the row is explicitly written (prevents old content bleeding through).
fn styled_line_to_line_padded(sl: &StyledLine, width: usize) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = sl
        .spans
        .iter()
        .map(|(style, text)| Span::styled(text.clone(), *style))
        .collect();
    // Measure rendered width to know how many trailing spaces to add.
    let rendered_width: usize = sl.spans.iter().map(|(_, t)| t.chars().count()).sum();
    if rendered_width < width {
        spans.push(Span::raw(" ".repeat(width - rendered_width)));
    }
    Line::from(spans)
}

/// Fill every cell in `area` with a space using default style.
/// This guarantees stale terminal content (including syntect-coloured cells or
/// make-modal text that Paragraph::alignment(Center) left untouched) is erased.
fn fill_blank(frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let blank: String = " ".repeat(area.width as usize);
    let lines: Vec<Line<'static>> = (0..area.height as usize)
        .map(|_| Line::from(Span::raw(blank.clone())))
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
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
        .title(Span::styled(
            title.to_owned(),
            Style::default().fg(Color::White),
        ))
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Blank-fill the entire inner area first so no stale cells remain.
    fill_blank(frame, inner);

    // Then render the message centered.
    let top_pad = inner.height / 2;
    let mut lines = vec![Line::from(""); top_pad as usize];
    lines.push(Line::from(Span::styled(
        msg,
        Style::default().fg(msg_color),
    )));
    let para = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(para, inner);
}

/// Render a titled block, blank-fill its inner area, and return the inner rect.
fn render_block(frame: &mut Frame, area: Rect, border_style: Style, title: String) -> Rect {
    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(Color::White)))
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    // Blank-fill before any content is drawn over this rect.
    fill_blank(frame, inner);
    inner
}

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    // Guarantee a clean slate every frame — prevents stale cell artefacts when
    // transitioning between preview states (e.g. Text → MakeOutput).
    frame.render_widget(Clear, area);

    let border_style = panel_border_style(state.focused_panel == FocusedPanel::Preview);

    match &state.preview_state {
        PreviewState::None => {
            render_centered_msg(
                frame,
                area,
                border_style,
                " Preview ",
                "Select a file to preview",
                Color::DarkGray,
            );
        }

        PreviewState::Loading => {
            render_centered_msg(
                frame,
                area,
                border_style,
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

            let inner_width = inner.width as usize;
            let visible_lines: Vec<Line<'static>> = lines
                .iter()
                .skip(scroll)
                .take(inner_height)
                .map(|sl| styled_line_to_line_padded(sl, inner_width))
                .collect();

            // render_block already blank-filled inner; just draw the text on top.
            frame.render_widget(Paragraph::new(visible_lines), inner);
        }

        PreviewState::Binary { size, mime_hint } => {
            let hint_str = mime_hint.as_str();
            let inner = render_block(
                frame,
                area,
                border_style,
                format!(" Binary File ({}) ", hint_str),
            );

            let text = vec![
                Line::from(vec![
                    Span::styled("Type:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(hint_str.to_owned(), Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled("Size:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format_size_verbose(*size),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Binary file \u{2014} cannot display as text.",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            frame.render_widget(Paragraph::new(text), inner);
        }

        PreviewState::Directory {
            entry_count,
            total_size,
        } => {
            let inner = render_block(frame, area, border_style, " Directory Info ".to_owned());

            let text = vec![
                Line::from(vec![
                    Span::styled("Entries:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        entry_count.to_string(),
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Total size:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format_size_verbose(*total_size),
                        Style::default().fg(Color::White),
                    ),
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
