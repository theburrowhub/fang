use crate::app::state::{FileEntry, MarkdownItem, PreviewState};
use std::path::Path;

pub mod binary;
pub mod images;
pub mod makefile;
pub mod markdown;
pub mod text;

const MAX_PREVIEW_SIZE: u64 = 10 * 1024 * 1024; // 10MB

/// Markdown extensions rendered with formatting instead of raw syntax highlighting.
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdx", "mdown", "mkd", "mkdn"];

pub async fn load_preview(entry: &FileEntry) -> PreviewState {
    if entry.is_dir {
        return load_directory_preview(&entry.path);
    }

    let size = match std::fs::metadata(&entry.path) {
        Ok(m) => m.len(),
        Err(e) => return PreviewState::Error(format!("Cannot stat: {}", e)),
    };

    if size > MAX_PREVIEW_SIZE {
        return PreviewState::TooLarge { size };
    }

    // Check if it's a Makefile (case-insensitive, no allocation)
    if entry.name.eq_ignore_ascii_case("makefile") {
        return makefile::load_makefile_preview(&entry.path).await;
    }

    // Check if binary by extension first (fast path — no I/O needed)
    if binary::is_binary_by_extension(&entry.path) {
        return binary::load_binary_preview(&entry.path).await;
    }

    // Read file once; use the bytes for both binary detection and text highlighting
    match std::fs::read(&entry.path) {
        Ok(data) => {
            if binary::is_binary_by_content(&data) {
                return binary::load_binary_preview(&entry.path).await;
            }

            // Render Markdown files with formatting instead of raw syntax highlighting
            let ext = entry
                .extension
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase();
            if MARKDOWN_EXTENSIONS.contains(&ext.as_str()) {
                if let Ok(source) = String::from_utf8(data) {
                    let total_lines = source.lines().count();
                    let base_dir = entry.path.parent();
                    // Parse markdown into intermediate items (text + image placeholders)
                    // Use a narrower render width so tables fit visible panel widths.
                    // The actual panel is usually 80-120 chars; 100 is a good compromise.
                    const RENDER_WIDTH: u16 = 100;
                    let rich = markdown::render_markdown_rich(&source, RENDER_WIDTH, base_dir);
                    // Render images async and build the final item list
                    let mut items: Vec<MarkdownItem> = Vec::new();
                    let mut has_images = false;
                    for item in rich {
                        match item {
                            markdown::RichItem::Text(lines) => {
                                items.push(MarkdownItem::Text(lines));
                            }
                            markdown::RichItem::Mermaid(src) => {
                                has_images = true;
                                let png = images::render_mermaid_to_png(&src);
                                let alt = "Mermaid diagram".to_string();
                                if let Some(png) = png {
                                    items.push(MarkdownItem::Image { png, alt });
                                } else {
                                    // Fallback: show source as code block
                                    let fallback = markdown::render_markdown(
                                        &format!("```mermaid\n{}\n```", src),
                                        RENDER_WIDTH,
                                    );
                                    items.push(MarkdownItem::Text(fallback));
                                }
                            }
                            markdown::RichItem::ImageFile { path, alt } => {
                                has_images = true;
                                if let Some(png) = images::load_image_to_png(&path) {
                                    items.push(MarkdownItem::Image { png, alt });
                                }
                                // If loading fails, silently skip the image
                            }
                        }
                    }
                    if has_images {
                        return PreviewState::RichMarkdown { items, total_lines };
                    }
                    // Pure text markdown — use the flat Text variant (cheaper rendering)
                    let lines = items
                        .into_iter()
                        .flat_map(|i| match i {
                            MarkdownItem::Text(l) => l,
                            _ => vec![],
                        })
                        .collect();
                    return PreviewState::Text { lines, total_lines };
                }
                // If not valid UTF-8 fall through to normal text path
                return text::highlight_bytes(
                    &entry.path,
                    std::fs::read(&entry.path).unwrap_or_default(),
                );
            }

            // Pass already-read bytes to avoid a second file read
            text::highlight_bytes(&entry.path, data)
        }
        Err(e) => PreviewState::Error(format!("Cannot read: {}", e)),
    }
}

fn load_directory_preview(path: &Path) -> PreviewState {
    let mut entry_count = 0;
    let mut total_size = 0u64;

    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.flatten() {
            entry_count += 1;
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }
    }

    PreviewState::Directory {
        entry_count,
        total_size,
    }
}
