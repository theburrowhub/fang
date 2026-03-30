use crate::app::state::{
    AppState, FocusedPanel, MarkdownItem, PendingImageRender, PreviewState, RenderedImage,
    StyledLine,
};
use crate::ui::utils::{format_size_verbose, panel_border_style};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Convert a StyledLine into a ratatui Line that is exactly `width` cells wide:
/// - Lines shorter than `width` are padded with trailing spaces so every
///   cell in the row is explicitly written (prevents stale cell artefacts).
/// - Lines longer than `width` are clipped at the right edge so they
///   cannot overflow into adjacent panels.
fn styled_line_to_line_padded(sl: &StyledLine, width: usize) -> Line<'static> {
    if width == 0 {
        return Line::from(vec![]);
    }

    let mut result: Vec<Span<'static>> = Vec::new();
    let mut remaining = width; // columns still available

    for (style, text) in &sl.spans {
        if remaining == 0 {
            break;
        }
        // Expand tabs to 4 spaces so char count matches terminal display width.
        // Without this, each '\t' counts as 1 char but the terminal renders it
        // as up to 8 columns, causing lines to overflow the panel boundary.
        let text = text.replace('\t', "    ");
        let chars: Vec<char> = text.chars().collect();
        if chars.len() <= remaining {
            result.push(Span::styled(text, *style));
            remaining -= chars.len();
        } else {
            // Clip this span at the panel boundary
            let clipped: String = chars[..remaining].iter().collect();
            result.push(Span::styled(clipped, *style));
            remaining = 0;
        }
    }

    // Pad any remaining space so the full row is written
    if remaining > 0 {
        result.push(Span::raw(" ".repeat(remaining)));
    }

    Line::from(result)
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
            let inner_width = inner.width as usize;

            // Scroll by source lines (no wrap).
            // Each line is padded/clipped to inner_width so every cell in every
            // row is written explicitly — this is the only reliable way to prevent
            // stale terminal cells from a previous file or scroll position bleeding
            // through. Paragraph::wrap + scroll leaves unwritten cells when the
            // visible slice is shorter than the panel height.
            let max_scroll = lines.len().saturating_sub(inner_height);
            let scroll = state.preview_scroll.min(max_scroll);

            let display: Vec<Line<'static>> = lines
                .iter()
                .skip(scroll)
                .take(inner_height)
                .map(|sl| styled_line_to_line_padded(sl, inner_width))
                .collect();

            frame.render_widget(Paragraph::new(display), inner);
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
            let title = if state.make_cancel_tx.is_some() {
                " Make Output  [Esc] cancel ".to_owned()
            } else {
                " Make Output ".to_owned()
            };
            let inner = render_block(frame, area, border_style, title);

            let inner_height = inner.height as usize;
            let scroll = state
                .preview_scroll
                .min(output.len().saturating_sub(inner_height));

            let inner_width = inner.width as usize;
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
                    // Expand tabs and clip to panel width
                    let expanded = s.replace('\t', "    ");
                    let clipped: String = expanded.chars().take(inner_width).collect();
                    Line::from(Span::styled(clipped, style))
                })
                .collect();

            frame.render_widget(Paragraph::new(lines), inner);
        }

        PreviewState::GitDiff { lines } => {
            let inner = render_block(frame, area, border_style, " Diff [d] preview ".to_owned());

            let inner_height = inner.height as usize;
            let inner_width = inner.width as usize;
            let max_scroll = lines.len().saturating_sub(inner_height);
            let scroll = state.preview_scroll.min(max_scroll);

            let display: Vec<Line<'static>> = lines
                .iter()
                .skip(scroll)
                .take(inner_height)
                .map(|sl| styled_line_to_line_padded(sl, inner_width))
                .collect();

            frame.render_widget(Paragraph::new(display), inner);
        }

        PreviewState::RichMarkdown {
            source,
            base_dir,
            images,
            total_lines,
        } => {
            let title = format!(" Preview ({} lines) ", total_lines);
            let inner = render_block(frame, area, border_style, title);
            if inner.height == 0 {
                return;
            }

            let inner_width = inner.width;
            let inner_height = inner.height as usize;
            // Image height: ≤ 30% of panel height, clamped to [3, 12] rows.
            let image_rows = (inner_height * 3 / 10).clamp(3, 12);

            // ── Re-render text at the actual panel width (cached by width) ──────
            // `render_markdown_rich` is fast (pure Rust, no I/O) so doing it at
            // draw time gives us perfectly-sized text for any terminal width.
            let items: Vec<MarkdownItem> = {
                let mut cache = state.markdown_text_cache.borrow_mut();
                if cache.as_ref().map(|(w, _)| *w) != Some(inner_width) {
                    // Width changed or first render — re-build.
                    let rich = crate::preview::markdown::render_markdown_rich(
                        source,
                        inner_width,
                        base_dir.as_deref(),
                    );
                    let new_items = build_rich_items(rich, images);
                    *cache = Some((inner_width, new_items));
                }
                cache.as_ref().unwrap().1.clone()
            };

            // ── Scroll ────────────────────────────────────────────────────────
            let total_visual: usize = items
                .iter()
                .map(|item| match item {
                    MarkdownItem::Text(lines) => lines.len(),
                    MarkdownItem::Image { .. } => image_rows,
                })
                .sum();
            let max_scroll = total_visual.saturating_sub(inner_height);
            let scroll = state.preview_scroll.min(max_scroll);

            // Ensure the image-protocol cache has a slot for every Image item.
            // Clear pending images from any previous frame before queueing new ones.
            state.pending_image_renders.borrow_mut().clear();

            // ── Render ────────────────────────────────────────────────────────
            let mut row = 0usize;
            let mut _img_idx = 0usize;

            'items: for item in &items {
                match item {
                    MarkdownItem::Text(lines) => {
                        let block_end = row + lines.len();
                        if block_end <= scroll {
                            row = block_end;
                            continue;
                        }
                        if row >= scroll + inner_height {
                            break 'items;
                        }
                        let skip = scroll.saturating_sub(row);
                        let take = inner_height.saturating_sub(row.saturating_sub(scroll));
                        let y = inner.y + row.saturating_sub(scroll) as u16;
                        let height = take.min(lines.len().saturating_sub(skip)) as u16;
                        if height > 0 {
                            let block_area = Rect {
                                x: inner.x,
                                y,
                                width: inner.width,
                                height,
                            };
                            let display: Vec<Line<'static>> = lines
                                .iter()
                                .skip(skip)
                                .take(height as usize)
                                .map(|sl| styled_line_to_line_padded(sl, inner_width as usize))
                                .collect();
                            frame.render_widget(Paragraph::new(display), block_area);
                        }
                        row = block_end;
                    }
                    MarkdownItem::Image { png, alt } => {
                        let img_top = row;
                        let img_bot = row + image_rows;

                        if img_top < scroll + inner_height && img_bot > scroll {
                            let vis_top = img_top.max(scroll);
                            let vis_height = (img_bot.min(scroll + inner_height) - vis_top) as u16;
                            let y = inner.y + (vis_top - scroll) as u16;

                            // Queue image for post-draw rendering — writing iTerm2 escape
                            // sequences directly to stdout AFTER terminal.draw() avoids the
                            // pre-clearing CUD loop in ratatui-image's buffer implementation
                            // that causes terminal scroll and layout corruption.
                            if !png.is_empty() && std::env::var("ITERM_SESSION_ID").is_ok() {
                                state
                                    .pending_image_renders
                                    .borrow_mut()
                                    .push(PendingImageRender {
                                        x: inner.x,
                                        y,
                                        cols: inner.width,
                                        rows: vis_height,
                                        png: png.clone(),
                                    });
                            } else {
                                // Non-iTerm2 terminal or no PNG — show placeholder
                                let img_area = Rect {
                                    x: inner.x,
                                    y,
                                    width: inner.width,
                                    height: vis_height,
                                };
                                frame.render_widget(
                                    Paragraph::new(Span::styled(
                                        format!("[img: {}]", alt),
                                        Style::default()
                                            .fg(Color::DarkGray)
                                            .add_modifier(Modifier::ITALIC),
                                    ))
                                    .alignment(Alignment::Center),
                                    img_area,
                                );
                            }
                        }
                        row += image_rows;
                        _img_idx += 1;
                    }
                }
            }
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

/// Convert `RichItem` list into `MarkdownItem` list, substituting pre-rendered images.
///
/// Images appear in source order; each Mermaid/ImageFile RichItem consumes one
/// slot from `rendered_images`.
fn build_rich_items(
    rich: Vec<crate::preview::markdown::RichItem>,
    rendered_images: &[RenderedImage],
) -> Vec<MarkdownItem> {
    let mut items = Vec::new();
    let mut img_iter = rendered_images.iter();

    for item in rich {
        match item {
            crate::preview::markdown::RichItem::Text(lines) => {
                if !lines.is_empty() {
                    items.push(MarkdownItem::Text(lines));
                }
            }
            crate::preview::markdown::RichItem::Mermaid(_) => {
                // Consume the next pre-rendered image slot.
                if let Some(img) = img_iter.next() {
                    items.push(MarkdownItem::Image {
                        png: img.png.clone(),
                        alt: img.alt.clone(),
                    });
                }
                // If no slot (render failed at load time), the block is silently skipped.
            }
            crate::preview::markdown::RichItem::ImageFile { .. } => {
                if let Some(img) = img_iter.next() {
                    items.push(MarkdownItem::Image {
                        png: img.png.clone(),
                        alt: img.alt.clone(),
                    });
                }
            }
        }
    }
    items
}
