//! Markdown renderer for the preview panel.
//!
//! Converts Markdown source into a flat list of [`StyledLine`]s that ratatui
//! can display directly, applying visual formatting:
//!
//! - `# Heading` → bold + cyan, preceded by a blank separator
//! - `**bold**` / `__bold__` → bold white
//! - `*italic*` / `_italic_` → italic gray
//! - `` `code` `` → yellow mono-style text
//! - `> blockquote` → dark-gray italic with leading `│ `
//! - `- item` / `* item` / `1. item` → indented with `• ` / `  1. `
//! - `---` / `***` (thematic break) → a line of `─` chars
//! - Fenced code blocks → plain white (syntax highlighting kept for text.rs)
//! - Plain paragraphs → white, wrapped at the source line boundaries

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};

use crate::app::state::StyledLine;

/// Parse `source` and return a list of styled lines suitable for the preview panel.
pub fn render_markdown(source: &str, panel_width: u16) -> Vec<StyledLine> {
    let opts = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(source, opts);

    let mut lines: Vec<StyledLine> = Vec::new();
    // Accumulate spans for the current visual line
    let mut current: Vec<(Style, String)> = Vec::new();
    // Stack of active inline styles
    let mut style_stack: Vec<Style> = vec![Style::default().fg(Color::White)];
    // Whether we're inside a code block
    let mut in_code_block = false;
    // Whether we're inside a blockquote
    let mut in_blockquote = false;
    // List nesting depth and ordered-list counters
    let mut list_depth: usize = 0;
    let mut ordered_counter: Vec<u64> = Vec::new();

    let push_line = |lines: &mut Vec<StyledLine>, spans: Vec<(Style, String)>| {
        lines.push(StyledLine { spans });
    };

    let flush = |lines: &mut Vec<StyledLine>, current: &mut Vec<(Style, String)>| {
        let spans = std::mem::take(current);
        push_line(lines, spans);
    };

    for event in parser {
        match event {
            // ── Block starts ─────────────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                flush(&mut lines, &mut current);
                lines.push(StyledLine { spans: vec![] }); // blank before heading
                let color = match level {
                    HeadingLevel::H1 => Color::Cyan,
                    HeadingLevel::H2 => Color::LightCyan,
                    HeadingLevel::H3 => Color::LightBlue,
                    _ => Color::Blue,
                };
                let prefix = match level {
                    HeadingLevel::H1 => "# ",
                    HeadingLevel::H2 => "## ",
                    HeadingLevel::H3 => "### ",
                    HeadingLevel::H4 => "#### ",
                    HeadingLevel::H5 => "##### ",
                    HeadingLevel::H6 => "###### ",
                };
                current.push((
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                    prefix.to_string(),
                ));
                style_stack.push(Style::default().fg(color).add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                flush(&mut lines, &mut current);
            }

            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush(&mut lines, &mut current);
                lines.push(StyledLine { spans: vec![] }); // blank after paragraph
            }

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
                flush(&mut lines, &mut current);
            }

            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                flush(&mut lines, &mut current);
                style_stack.push(Style::default().fg(Color::White));
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                style_stack.pop();
                flush(&mut lines, &mut current);
            }

            Event::Start(Tag::List(start_num)) => {
                list_depth += 1;
                if let Some(n) = start_num {
                    ordered_counter.push(n);
                } else {
                    ordered_counter.push(0); // sentinel for unordered
                }
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                ordered_counter.pop();
            }
            Event::Start(Tag::Item) => {
                flush(&mut lines, &mut current);
                let indent = "  ".repeat(list_depth.saturating_sub(1));
                let counter = ordered_counter.last_mut();
                let bullet = if let Some(c) = counter {
                    if *c == 0 {
                        // unordered
                        format!("{}• ", indent)
                    } else {
                        let s = format!("{}{}. ", indent, c);
                        *c += 1;
                        s
                    }
                } else {
                    format!("{}• ", indent)
                };
                current.push((Style::default().fg(Color::Yellow), bullet));
            }
            Event::End(TagEnd::Item) => {
                flush(&mut lines, &mut current);
            }

            Event::Rule => {
                flush(&mut lines, &mut current);
                let width = (panel_width as usize).saturating_sub(2).max(4);
                lines.push(StyledLine {
                    spans: vec![(Style::default().fg(Color::DarkGray), "─".repeat(width))],
                });
            }

            // ── Inline starts ─────────────────────────────────────────────────
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

            Event::Start(Tag::Link { dest_url, .. }) => {
                style_stack.push(
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED),
                );
                // We'll append the URL after the link text in End
                let _ = dest_url; // stored by pulldown-cmark until End
            }
            Event::End(TagEnd::Link) => {
                style_stack.pop();
            }

            Event::Start(Tag::Image { .. }) => {
                current.push((Style::default().fg(Color::DarkGray), "[image]".to_string()));
            }
            Event::End(TagEnd::Image) => {}

            // ── Inline content ────────────────────────────────────────────────
            Event::Text(text) => {
                let style = *style_stack.last().unwrap_or(&Style::default());
                if in_code_block {
                    // Code block: emit each source line as its own StyledLine
                    for line in text.lines() {
                        current.push((style, line.to_string()));
                        flush(&mut lines, &mut current);
                    }
                } else if in_blockquote {
                    // Blockquote: prefix each line with │
                    let mut first = true;
                    for line in text.lines() {
                        if !first {
                            flush(&mut lines, &mut current);
                            current.push((Style::default().fg(Color::DarkGray), "│ ".to_string()));
                        }
                        current.push((style, line.to_string()));
                        first = false;
                    }
                } else {
                    current.push((style, text.into_string()));
                }
            }

            Event::Code(code) => {
                current.push((
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                    code.into_string(),
                ));
            }

            Event::SoftBreak => {
                current.push((Style::default(), " ".to_string()));
            }

            Event::HardBreak => {
                flush(&mut lines, &mut current);
            }

            Event::Html(_) | Event::InlineHtml(_) => {
                // Raw HTML — show as-is in dim color
                // (pulldown-cmark gives us the raw tag, skip it for cleanliness)
            }

            _ => {}
        }
    }

    // Flush any remaining content
    if !current.is_empty() {
        flush(&mut lines, &mut current);
    }

    lines
}
