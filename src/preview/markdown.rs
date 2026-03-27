//! Markdown renderer for the preview panel.
//!
//! Converts Markdown source into styled [`StyledLine`]s.
//!
//! Changes vs previous version:
//! - Paragraphs are word-wrapped at `panel_width` preserving inline styles
//! - Tables are rendered with equal-width columns (two-pass layout)
//! - Links show only the anchor text (OSC 8 hyperlink where terminal supports it)
//! - Code blocks rendered in light-green to distinguish from prose
//! - Fenced code block lines are indented with a left-gutter mark

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};

use crate::app::state::StyledLine;

/// Rows collected for two-pass table layout: (is_header, cells × spans).
type TableRows = Vec<(bool, Vec<Vec<(ratatui::style::Style, String)>>)>;

// ── Public API ────────────────────────────────────────────────────────────────

pub fn render_markdown(source: &str, panel_width: u16) -> Vec<StyledLine> {
    let width = panel_width as usize;
    let opts = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(source, opts);

    let mut lines: Vec<StyledLine> = Vec::new();
    let mut current: Vec<(Style, String)> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default().fg(Color::White)];

    // Table state — collected for two-pass column layout
    let mut in_table = false;
    let mut _in_table_head = false;
    let mut row_cells: Vec<Vec<(Style, String)>> = Vec::new();
    let mut cell_spans: Vec<(Style, String)> = Vec::new();
    let mut in_cell = false;
    // All rows (header first) accumulated before rendering
    let mut table_rows: TableRows = Vec::new(); // (is_header, cells)

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
            // ── Headings ──────────────────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                do_flush_wrapped(&mut lines, &mut current, width);
                blank(&mut lines);
                let (color, prefix) = heading_props(level);
                let st = Style::default().fg(color).add_modifier(Modifier::BOLD);
                current.push((st, prefix.to_string()));
                style_stack.push(st);
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                do_flush_wrapped(&mut lines, &mut current, 0); // headings: no wrap
            }

            // ── Paragraphs ────────────────────────────────────────────────────
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                do_flush_wrapped(&mut lines, &mut current, width);
                blank(&mut lines);
            }

            // ── Blockquote ────────────────────────────────────────────────────
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
                do_flush_wrapped(&mut lines, &mut current, width);
            }

            // ── Code block ────────────────────────────────────────────────────
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                do_flush_wrapped(&mut lines, &mut current, width);
                // Push a distinct style for code blocks
                style_stack.push(Style::default().fg(Color::LightGreen));
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                style_stack.pop();
                do_flush_wrapped(&mut lines, &mut current, 0);
                blank(&mut lines);
            }

            // ── Lists ─────────────────────────────────────────────────────────
            Event::Start(Tag::List(start)) => {
                list_depth += 1;
                ordered_counter.push(start.unwrap_or(0));
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                ordered_counter.pop();
            }
            Event::Start(Tag::Item) => {
                do_flush_wrapped(&mut lines, &mut current, width);
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
                do_flush_wrapped(&mut lines, &mut current, width);
            }

            // ── Thematic rule ─────────────────────────────────────────────────
            Event::Rule => {
                do_flush_wrapped(&mut lines, &mut current, width);
                let w = (panel_width as usize).saturating_sub(2).max(4);
                lines.push(StyledLine {
                    spans: vec![(Style::default().fg(Color::DarkGray), "─".repeat(w))],
                });
            }

            // ── Tables (two-pass) ─────────────────────────────────────────────
            Event::Start(Tag::Table(_)) => {
                do_flush_wrapped(&mut lines, &mut current, width);
                blank(&mut lines);
                in_table = true;
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                render_table(&mut lines, &table_rows, panel_width);
                blank(&mut lines);
                table_rows.clear();
            }
            Event::Start(Tag::TableHead) => {
                _in_table_head = true;
                row_cells.clear();
            }
            Event::End(TagEnd::TableHead) => {
                _in_table_head = false;
                table_rows.push((true, std::mem::take(&mut row_cells)));
            }
            Event::Start(Tag::TableRow) => {
                row_cells.clear();
            }
            Event::End(TagEnd::TableRow) => {
                table_rows.push((false, std::mem::take(&mut row_cells)));
            }
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                cell_spans.clear();
            }
            Event::End(TagEnd::TableCell) => {
                in_cell = false;
                row_cells.push(std::mem::take(&mut cell_spans));
            }

            // ── Inline styles ─────────────────────────────────────────────────
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

            // ── Links — anchor text only, OSC 8 hyperlink ─────────────────────
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
                // Inject OSC 8 close sequence after the anchor text.
                // This is a no-op in terminals that don't support OSC 8.
                if let Some(url) = pending_link_url.take() {
                    if !url.is_empty() && !url.starts_with('#') {
                        // OSC 8 hyperlink: ESC ] 8 ; ; URL BEL  text  ESC ] 8 ; ; BEL
                        // We inject the closing sequence after the last span of the link.
                        // Supporting terminals (iTerm2, Kitty, WezTerm, foot, …) will
                        // make the anchor text clickable.
                        target!().push((Style::default(), "\x1b]8;;\x07".to_string()));
                        let _ = url; // URL was used to open OSC 8 before the text
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

            // ── Text content ──────────────────────────────────────────────────
            Event::Text(text) => {
                let style = *style_stack.last().unwrap_or(&Style::default());
                let t = text.into_string();

                if in_code_block {
                    // Each source line → its own StyledLine with left-gutter mark
                    let mut first = true;
                    for line in t.lines() {
                        if !first {
                            do_flush_wrapped(&mut lines, &mut current, 0);
                        }
                        current.push((Style::default().fg(Color::DarkGray), "  ".to_string()));
                        current.push((style, line.to_string()));
                        first = false;
                    }
                } else if in_blockquote {
                    let mut first = true;
                    for line in t.lines() {
                        if !first {
                            do_flush_wrapped(&mut lines, &mut current, width);
                            current.push((Style::default().fg(Color::DarkGray), "│ ".to_string()));
                        }
                        current.push((style, line.to_string()));
                        first = false;
                    }
                } else if in_cell {
                    cell_spans.push((style, t.replace('\n', " ")));
                } else if t.contains('\n') {
                    let mut first = true;
                    for line in t.split('\n') {
                        if !first {
                            do_flush_wrapped(&mut lines, &mut current, width);
                        }
                        current.push((style, line.to_string()));
                        first = false;
                    }
                } else {
                    // For link text: inject OSC 8 open before the text
                    if let Some(ref url) = pending_link_url {
                        if !url.is_empty() && !url.starts_with('#') {
                            target!().push((Style::default(), format!("\x1b]8;;{}\x07", url)));
                        }
                    }
                    target!().push((style, t));
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
                    do_flush_wrapped(&mut lines, &mut current, width);
                }
            }

            _ => {}
        }
    }

    if !current.is_empty() {
        do_flush_wrapped(&mut lines, &mut current, width);
    }
    lines
}

// ── Rich markdown (images + mermaid) ─────────────────────────────────────────

/// Intermediate item from the rich markdown parser.
/// Image rendering (PNG conversion) happens asynchronously in `mod.rs`.
pub enum RichItem {
    /// Already-processed styled text lines.
    Text(Vec<StyledLine>),
    /// A fenced ` ```mermaid ``` ` block — source to render to PNG.
    Mermaid(String),
    /// An `![alt](path)` image — resolved absolute path + alt text.
    ImageFile {
        path: std::path::PathBuf,
        alt: String,
    },
}

/// Parse `source` and split it into text blocks, mermaid diagrams, and image refs.
///
/// `base_dir` is used to resolve relative image paths in `![alt](path)` tags.
pub fn render_markdown_rich(
    source: &str,
    panel_width: u16,
    base_dir: Option<&std::path::Path>,
) -> Vec<RichItem> {
    use pulldown_cmark::CodeBlockKind;

    let width = panel_width as usize;
    let opts = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(source, opts);

    let mut items: Vec<RichItem> = Vec::new();

    // Accumulated styled lines for the current text block
    let mut lines: Vec<StyledLine> = Vec::new();
    let mut current: Vec<(Style, String)> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default().fg(Color::White)];

    // Mermaid block accumulator
    let mut in_mermaid = false;
    let mut mermaid_src = String::new();

    // Image state
    let mut image_url: Option<String> = None;
    let mut image_alt = String::new();

    // Table state
    let mut in_table = false;
    let mut _in_table_head = false;
    let mut row_cells: Vec<Vec<(Style, String)>> = Vec::new();
    let mut cell_spans: Vec<(Style, String)> = Vec::new();
    let mut in_cell = false;
    let mut table_rows: TableRows = Vec::new();

    // Other state (mirrors render_markdown)
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

    // Flush accumulated text lines into a Text item
    macro_rules! flush_text {
        () => {
            if !current.is_empty() {
                do_flush_wrapped(&mut lines, &mut current, width);
            }
            if !lines.is_empty() {
                items.push(RichItem::Text(std::mem::take(&mut lines)));
            }
        };
    }

    for event in parser {
        // Skip all events inside a mermaid block except Text and End
        if in_mermaid {
            match event {
                Event::Text(t) => mermaid_src.push_str(&t),
                Event::End(TagEnd::CodeBlock) => {
                    in_mermaid = false;
                    items.push(RichItem::Mermaid(std::mem::take(&mut mermaid_src)));
                }
                _ => {}
            }
            continue;
        }

        match event {
            // ── Headings ──────────────────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                do_flush_wrapped(&mut lines, &mut current, width);
                blank(&mut lines);
                let (color, prefix) = heading_props(level);
                let st = Style::default().fg(color).add_modifier(Modifier::BOLD);
                current.push((st, prefix.to_string()));
                style_stack.push(st);
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                do_flush_wrapped(&mut lines, &mut current, 0);
            }

            // ── Paragraphs ────────────────────────────────────────────────────
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                do_flush_wrapped(&mut lines, &mut current, width);
                blank(&mut lines);
            }

            // ── Blockquote ────────────────────────────────────────────────────
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
                do_flush_wrapped(&mut lines, &mut current, width);
            }

            // ── Code block ────────────────────────────────────────────────────
            Event::Start(Tag::CodeBlock(kind)) => {
                // Detect mermaid fenced blocks
                let is_mermaid = matches!(&kind,
                    CodeBlockKind::Fenced(info)
                    if info.split_whitespace().next() == Some("mermaid")
                );
                if is_mermaid {
                    // Flush pending text before the diagram
                    flush_text!();
                    in_mermaid = true;
                    mermaid_src.clear();
                } else {
                    in_code_block = true;
                    do_flush_wrapped(&mut lines, &mut current, width);
                    style_stack.push(Style::default().fg(Color::LightGreen));
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block {
                    in_code_block = false;
                    style_stack.pop();
                    do_flush_wrapped(&mut lines, &mut current, 0);
                    blank(&mut lines);
                }
                // mermaid end is handled in the early-exit above
            }

            // ── Lists ─────────────────────────────────────────────────────────
            Event::Start(Tag::List(start)) => {
                list_depth += 1;
                ordered_counter.push(start.unwrap_or(0));
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                ordered_counter.pop();
                if list_depth == 0 {
                    blank(&mut lines);
                }
            }
            Event::Start(Tag::Item) => {
                do_flush_wrapped(&mut lines, &mut current, width);
                let indent = "  ".repeat(list_depth.saturating_sub(1));
                let is_ordered = ordered_counter.last().copied().unwrap_or(0) > 0;
                let bullet = if is_ordered {
                    let n = ordered_counter.last_mut().unwrap();
                    let s = format!("{}{}. ", indent, n);
                    *n += 1;
                    s
                } else {
                    format!("{}• ", indent)
                };
                current.push((Style::default().fg(Color::Cyan), bullet));
            }
            Event::End(TagEnd::Item) => {
                do_flush_wrapped(&mut lines, &mut current, width);
            }

            // ── Rules ─────────────────────────────────────────────────────────
            Event::Rule => {
                do_flush_wrapped(&mut lines, &mut current, width);
                let w = width.saturating_sub(2).max(4);
                lines.push(StyledLine {
                    spans: vec![(Style::default().fg(Color::DarkGray), "─".repeat(w))],
                });
                blank(&mut lines);
            }

            // ── Tables ────────────────────────────────────────────────────────
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                do_flush_wrapped(&mut lines, &mut current, width);
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                render_table(&mut lines, &table_rows, panel_width);
                table_rows.clear();
                blank(&mut lines);
            }
            Event::Start(Tag::TableHead) => {
                _in_table_head = true;
            }
            Event::End(TagEnd::TableHead) => {
                _in_table_head = false;
                if !row_cells.is_empty() {
                    table_rows.push((true, std::mem::take(&mut row_cells)));
                }
            }
            Event::Start(Tag::TableRow) => {}
            Event::End(TagEnd::TableRow) => {
                if !row_cells.is_empty() {
                    table_rows.push((false, std::mem::take(&mut row_cells)));
                }
            }
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                cell_spans.clear();
            }
            Event::End(TagEnd::TableCell) => {
                in_cell = false;
                row_cells.push(std::mem::take(&mut cell_spans));
            }

            // ── Links ─────────────────────────────────────────────────────────
            Event::Start(Tag::Link { dest_url, .. }) => {
                let url = dest_url.into_string();
                target!().push((Style::default(), format!("\x1b]8;;{}\x07", url)));
                let st = Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED);
                style_stack.push(st);
                pending_link_url = Some(url);
            }
            Event::End(TagEnd::Link) => {
                style_stack.pop();
                if let Some(url) = pending_link_url.take() {
                    let _ = url;
                    target!().push((Style::default(), "\x1b]8;;\x07".to_string()));
                }
            }

            // ── Images ────────────────────────────────────────────────────────
            Event::Start(Tag::Image { dest_url, .. }) => {
                flush_text!();
                image_url = Some(dest_url.into_string());
                image_alt.clear();
                // Push style so alt-text events are captured via style_stack,
                // but we ignore them for rendering (alt goes to image_alt below).
                style_stack.push(Style::default());
            }
            Event::End(TagEnd::Image) => {
                style_stack.pop();
                if let Some(url) = image_url.take() {
                    // Skip HTTP/HTTPS images (can't load them without networking)
                    if !url.starts_with("http://") && !url.starts_with("https://") {
                        let path = if std::path::Path::new(&url).is_absolute() {
                            std::path::PathBuf::from(&url)
                        } else if let Some(base) = base_dir {
                            base.join(&url)
                        } else {
                            std::path::PathBuf::from(&url)
                        };
                        items.push(RichItem::ImageFile {
                            path,
                            alt: std::mem::take(&mut image_alt),
                        });
                    } else {
                        // Remote image: fall back to [img: alt] text
                        let alt = std::mem::take(&mut image_alt);
                        lines.push(StyledLine {
                            spans: vec![(
                                Style::default().fg(Color::DarkGray),
                                format!("[img: {}]", alt),
                            )],
                        });
                    }
                }
            }

            // ── Inline code ───────────────────────────────────────────────────
            Event::Code(code) => {
                let st = Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD);
                target!().push((st, code.into_string()));
            }

            // ── Text ──────────────────────────────────────────────────────────
            Event::Text(text) => {
                let style = *style_stack.last().unwrap_or(&Style::default());
                let t = text.into_string();

                // Capture alt text for images
                if image_url.is_some() {
                    image_alt.push_str(&t);
                    continue;
                }

                if in_code_block {
                    for line in t.lines() {
                        lines.push(StyledLine {
                            spans: vec![
                                (Style::default().fg(Color::DarkGray), "  ".to_string()),
                                (style, line.to_string()),
                            ],
                        });
                    }
                } else {
                    for (i, line) in t.lines().enumerate() {
                        if i > 0 {
                            do_flush_wrapped(&mut lines, &mut current, width);
                            if in_blockquote {
                                current
                                    .push((Style::default().fg(Color::DarkGray), "│ ".to_string()));
                            }
                        }
                        target!().push((style, line.to_string()));
                    }
                    if t.ends_with('\n') {
                        do_flush_wrapped(&mut lines, &mut current, width);
                    }
                }
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
                    do_flush_wrapped(&mut lines, &mut current, width);
                }
            }

            _ => {}
        }
    }

    // Flush any remaining text
    flush_text!();

    items
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn blank(lines: &mut Vec<StyledLine>) {
    lines.push(StyledLine { spans: vec![] });
}

/// Flush `current` into lines, word-wrapping at `max_width` (0 = no wrap).
fn do_flush_wrapped(
    lines: &mut Vec<StyledLine>,
    current: &mut Vec<(Style, String)>,
    max_width: usize,
) {
    if current.is_empty() {
        return;
    }
    let spans = std::mem::take(current);

    if max_width == 0 {
        lines.push(StyledLine { spans });
        return;
    }

    let total: usize = spans.iter().map(|(_, t)| visible_len(t)).sum();
    if total <= max_width {
        lines.push(StyledLine { spans });
        return;
    }

    // Word-wrap preserving per-span styles.
    let mut wrapped: Vec<StyledLine> = vec![StyledLine { spans: vec![] }];
    let mut col = 0usize;

    for (style, text) in spans {
        // Skip raw OSC escape sequences from link handling — they have zero width
        if text.starts_with('\x1b') {
            if let Some(last) = wrapped.last_mut() {
                last.spans.push((style, text));
            }
            continue;
        }

        let mut buf = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == ' ' || ch == '\t' {
                buf.push(ch);
                if col + visible_len(&buf) > max_width && col > 0 {
                    // Flush what we have before the space and wrap
                    let content = buf.trim_end().to_string();
                    if !content.is_empty() {
                        wrapped.last_mut().unwrap().spans.push((style, content));
                    }
                    wrapped.push(StyledLine { spans: vec![] });
                    col = 0;
                    buf.clear();
                }
            } else {
                buf.push(ch);
                // If single word overflows, force-break
                if col + visible_len(&buf) > max_width && col == 0 {
                    wrapped.last_mut().unwrap().spans.push((style, buf.clone()));
                    wrapped.push(StyledLine { spans: vec![] });
                    col = 0;
                    buf.clear();
                } else if col + visible_len(&buf) >= max_width && chars.peek() == Some(&' ') {
                    // We're at the end — flush and wrap on the next space
                    wrapped.last_mut().unwrap().spans.push((style, buf.clone()));
                    col += visible_len(&buf);
                    buf.clear();
                }
            }
        }

        if !buf.is_empty() {
            let buf_len = visible_len(&buf);
            if col + buf_len > max_width && col > 0 {
                let trimmed = buf.trim_start().to_string();
                if !trimmed.is_empty() {
                    wrapped.push(StyledLine {
                        spans: vec![(style, trimmed.clone())],
                    });
                    col = visible_len(&trimmed);
                } else {
                    wrapped.push(StyledLine { spans: vec![] });
                    col = 0;
                }
            } else {
                wrapped.last_mut().unwrap().spans.push((style, buf.clone()));
                col += buf_len;
            }
        }
    }

    for line in wrapped {
        if !line.spans.is_empty() {
            lines.push(line);
        }
    }
}

/// Character count excluding OSC escape sequences (zero display width).
fn visible_len(s: &str) -> usize {
    if s.starts_with('\x1b') {
        return 0;
    }
    s.chars().count()
}

fn heading_props(level: HeadingLevel) -> (Color, &'static str) {
    match level {
        HeadingLevel::H1 => (Color::Cyan, ""),
        HeadingLevel::H2 => (Color::LightCyan, ""),
        HeadingLevel::H3 => (Color::LightBlue, ""),
        _ => (Color::Blue, ""),
    }
}

// ── Two-pass table renderer ───────────────────────────────────────────────────

/// Render a collected table with column-aligned widths.
fn render_table(lines: &mut Vec<StyledLine>, rows: &TableRows, panel_width: u16) {
    if rows.is_empty() {
        return;
    }

    // Pass 1: determine max content width for each column
    let n_cols = rows.iter().map(|(_, cells)| cells.len()).max().unwrap_or(0);
    let mut col_widths: Vec<usize> = vec![0; n_cols];

    for (_, cells) in rows {
        for (i, cell) in cells.iter().enumerate() {
            let w: usize = cell.iter().map(|(_, t)| t.chars().count()).sum();
            if w > col_widths[i] {
                col_widths[i] = w;
            }
        }
    }

    // Cap total width to avoid overflowing the panel
    let sep_overhead = (n_cols + 1) * 3; // " │ " between cols + margins
    let available = (panel_width as usize).saturating_sub(sep_overhead);
    let total_content: usize = col_widths.iter().sum();
    if total_content > available && total_content > 0 {
        let scale = available as f64 / total_content as f64;
        for w in &mut col_widths {
            *w = ((*w as f64 * scale) as usize).max(1);
        }
    }

    let sep_style = Style::default().fg(Color::DarkGray);
    let head_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    // Pass 2: render each row
    for (row_idx, (is_header, cells)) in rows.iter().enumerate() {
        let mut spans: Vec<(Style, String)> = Vec::new();

        for (col_idx, cell) in cells.iter().enumerate() {
            if col_idx > 0 {
                spans.push((sep_style, "  │  ".to_string()));
            }

            let cell_text: String = cell.iter().map(|(_, t)| t.as_str()).collect();
            let cell_len = cell_text.chars().count();
            let target_w = col_widths.get(col_idx).copied().unwrap_or(cell_len);

            for (st, text) in cell {
                let effective_style = if *is_header {
                    head_style
                } else if st.fg.is_none() {
                    Style::default().fg(Color::White)
                } else {
                    *st
                };
                spans.push((effective_style, text.clone()));
            }

            // Pad to column width
            if cell_len < target_w {
                spans.push((Style::default(), " ".repeat(target_w - cell_len)));
            }
        }

        lines.push(StyledLine { spans });

        // Separator after header row
        if *is_header && row_idx == 0 {
            let sep_w = col_widths.iter().sum::<usize>() + (n_cols.saturating_sub(1)) * 5; // "  │  "
            lines.push(StyledLine {
                spans: vec![(sep_style, "─".repeat(sep_w.min(panel_width as usize)))],
            });
        }
    }
}
