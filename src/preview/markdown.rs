//! Markdown renderer for the preview panel.
//!
//! Converts Markdown source into styled [`StyledLine`]s:
//! - H1–H6 headings, bold/italic/strikethrough, inline code
//! - Tables: header + separator + data rows, cells joined with  │
//! - Blockquotes with │ prefix, fenced code blocks
//! - Lists (unordered • and ordered), thematic rules
//! - Links: underlined text + (url) appended in dim color

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};

use crate::app::state::StyledLine;

pub fn render_markdown(source: &str, panel_width: u16) -> Vec<StyledLine> {
    let opts = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(source, opts);

    let mut lines: Vec<StyledLine> = Vec::new();
    let mut current: Vec<(Style, String)> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default().fg(Color::White)];

    // Table state
    let mut in_table = false;
    let mut _in_table_head = false;
    let mut row_cells: Vec<Vec<(Style, String)>> = Vec::new();
    let mut cell_spans: Vec<(Style, String)> = Vec::new();
    let mut in_cell = false;

    // Other state
    let mut in_code_block = false;
    let mut in_blockquote = false;
    let mut list_depth: usize = 0;
    let mut ordered_counter: Vec<u64> = Vec::new();
    let mut pending_link_url: Option<String> = None;

    macro_rules! target {
        () => {
            if in_cell {
                &mut cell_spans
            } else {
                &mut current
            }
        };
    }

    for event in parser {
        match event {
            // ─── Headings ──────────────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                do_flush(&mut lines, &mut current);
                lines.push(StyledLine { spans: vec![] });
                let (color, prefix) = heading_props(level);
                let st = Style::default().fg(color).add_modifier(Modifier::BOLD);
                current.push((st, prefix.to_string()));
                style_stack.push(st);
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                do_flush(&mut lines, &mut current);
            }

            // ─── Paragraph ────────────────────────────────────────────────
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                do_flush(&mut lines, &mut current);
                lines.push(StyledLine { spans: vec![] });
            }

            // ─── Blockquote ───────────────────────────────────────────────
            Event::Start(Tag::BlockQuote(_)) => {
                in_blockquote = true;
                style_stack.push(
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                );
                current.push((Style::default().fg(Color::DarkGray), "│ ".to_string()));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_blockquote = false;
                style_stack.pop();
                do_flush(&mut lines, &mut current);
            }

            // ─── Code block ───────────────────────────────────────────────
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                do_flush(&mut lines, &mut current);
                style_stack.push(Style::default().fg(Color::White));
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                style_stack.pop();
                do_flush(&mut lines, &mut current);
            }

            // ─── Lists ────────────────────────────────────────────────────
            Event::Start(Tag::List(start)) => {
                list_depth += 1;
                ordered_counter.push(start.unwrap_or(0));
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                ordered_counter.pop();
            }
            Event::Start(Tag::Item) => {
                do_flush(&mut lines, &mut current);
                let indent = "  ".repeat(list_depth.saturating_sub(1));
                let bullet = match ordered_counter.last_mut() {
                    Some(c) if *c > 0 => {
                        let s = format!("{}{}. ", indent, c);
                        *c += 1;
                        s
                    }
                    _ => format!("{}• ", indent),
                };
                current.push((Style::default().fg(Color::Yellow), bullet));
            }
            Event::End(TagEnd::Item) => {
                do_flush(&mut lines, &mut current);
            }

            // ─── Thematic rule ────────────────────────────────────────────
            Event::Rule => {
                do_flush(&mut lines, &mut current);
                let w = (panel_width as usize).saturating_sub(2).max(4);
                lines.push(StyledLine {
                    spans: vec![(Style::default().fg(Color::DarkGray), "─".repeat(w))],
                });
            }

            // ─── Tables ───────────────────────────────────────────────────
            Event::Start(Tag::Table(_)) => {
                do_flush(&mut lines, &mut current);
                lines.push(StyledLine { spans: vec![] });
                in_table = true;
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                lines.push(StyledLine { spans: vec![] });
            }
            Event::Start(Tag::TableHead) => {
                _in_table_head = true;
                row_cells.clear();
            }
            Event::End(TagEnd::TableHead) => {
                _in_table_head = false;
                let head_style = Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD);
                lines.push(cells_to_line(&row_cells, head_style));
                let w = (panel_width as usize).saturating_sub(2).max(4);
                lines.push(StyledLine {
                    spans: vec![(Style::default().fg(Color::DarkGray), "─".repeat(w))],
                });
                row_cells.clear();
            }
            Event::Start(Tag::TableRow) => {
                row_cells.clear();
            }
            Event::End(TagEnd::TableRow) => {
                lines.push(cells_to_line(&row_cells, Style::default().fg(Color::White)));
            }
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                cell_spans.clear();
            }
            Event::End(TagEnd::TableCell) => {
                in_cell = false;
                row_cells.push(std::mem::take(&mut cell_spans));
            }

            // ─── Inline styles ────────────────────────────────────────────
            Event::Start(Tag::Strong) => {
                style_stack.push(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }
            Event::Start(Tag::Emphasis) => {
                style_stack.push(
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                );
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }
            Event::Start(Tag::Strikethrough) => {
                style_stack.push(
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::CROSSED_OUT),
                );
            }
            Event::End(TagEnd::Strikethrough) => {
                style_stack.pop();
            }

            // ─── Links ────────────────────────────────────────────────────
            Event::Start(Tag::Link { dest_url, .. }) => {
                pending_link_url = Some(dest_url.into_string());
                style_stack.push(
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED),
                );
            }
            Event::End(TagEnd::Link) => {
                style_stack.pop();
                if let Some(url) = pending_link_url.take() {
                    if !url.is_empty() && !url.starts_with('#') {
                        target!()
                            .push((Style::default().fg(Color::DarkGray), format!(" ({})", url)));
                    }
                }
            }

            Event::Start(Tag::Image { .. }) => {
                target!().push((Style::default().fg(Color::DarkGray), "[img] ".to_string()));
                style_stack.push(Style::default().fg(Color::DarkGray));
            }
            Event::End(TagEnd::Image) => {
                style_stack.pop();
            }

            // ─── Text content ─────────────────────────────────────────────
            Event::Text(text) => {
                let style = *style_stack.last().unwrap_or(&Style::default());
                let t = text.into_string();

                if in_code_block {
                    // Each source line → its own StyledLine
                    let mut first = true;
                    for line in t.lines() {
                        if !first {
                            do_flush(&mut lines, &mut current);
                        }
                        current.push((style, line.to_string()));
                        first = false;
                    }
                } else if in_blockquote {
                    let mut first = true;
                    for line in t.lines() {
                        if !first {
                            do_flush(&mut lines, &mut current);
                            current.push((Style::default().fg(Color::DarkGray), "│ ".to_string()));
                        }
                        current.push((style, line.to_string()));
                        first = false;
                    }
                } else if in_cell {
                    // Inside a table cell: newlines → space
                    cell_spans.push((style, t.replace('\n', " ")));
                } else if t.contains('\n') {
                    // Preserve paragraph-internal newlines
                    let mut first = true;
                    for line in t.split('\n') {
                        if !first {
                            do_flush(&mut lines, &mut current);
                        }
                        current.push((style, line.to_string()));
                        first = false;
                    }
                } else {
                    current.push((style, t));
                }
            }

            Event::Code(code) => {
                let st = Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD);
                target!().push((st, code.into_string()));
            }

            Event::SoftBreak => {
                if in_cell {
                    cell_spans.push((Style::default(), " ".to_string()));
                } else {
                    current.push((Style::default(), " ".to_string()));
                }
            }

            Event::HardBreak => {
                if !in_table {
                    do_flush(&mut lines, &mut current);
                }
            }

            _ => {}
        }
    }

    if !current.is_empty() {
        do_flush(&mut lines, &mut current);
    }
    lines
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn do_flush(lines: &mut Vec<StyledLine>, current: &mut Vec<(Style, String)>) {
    lines.push(StyledLine {
        spans: std::mem::take(current),
    });
}

fn heading_props(level: HeadingLevel) -> (Color, &'static str) {
    match level {
        HeadingLevel::H1 => (Color::Cyan, ""),
        HeadingLevel::H2 => (Color::LightCyan, ""),
        HeadingLevel::H3 => (Color::LightBlue, ""),
        _ => (Color::Blue, ""),
    }
}

/// Join `cells` into one [`StyledLine`] with `  │  ` separators.
fn cells_to_line(cells: &[Vec<(Style, String)>], row_style: Style) -> StyledLine {
    let sep = (Style::default().fg(Color::DarkGray), "  │  ".to_string());
    let mut spans: Vec<(Style, String)> = Vec::new();
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            spans.push(sep.clone());
        }
        for (st, text) in cell {
            let effective = if st.fg.is_none() { row_style } else { *st };
            spans.push((effective, text.clone()));
        }
    }
    StyledLine { spans }
}
