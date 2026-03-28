use crate::app::state::{FileEntry, PreviewState, RenderedImage};
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
                    // Scan source for mermaid/image blocks only — text rendering is
                    // deferred to draw time so it uses the actual panel width.
                    let rich = markdown::render_markdown_rich(&source, 80, base_dir);
                    let mut rendered_images: Vec<RenderedImage> = Vec::new();
                    let mut has_images = false;
                    for item in rich {
                        match item {
                            markdown::RichItem::Text(_) => {}
                            markdown::RichItem::Mermaid(src) => {
                                has_images = true;
                                if let Some(png) = images::render_mermaid_to_png(&src) {
                                    rendered_images.push(RenderedImage {
                                        alt: "Mermaid diagram".to_string(),
                                        png,
                                    });
                                }
                                // Render failure → no slot added; draw code
                                // detects mismatch and falls back to source text.
                            }
                            markdown::RichItem::ImageFile { path, alt } => {
                                has_images = true;
                                if let Some(png) = images::load_image_to_png(&path) {
                                    rendered_images.push(RenderedImage { alt, png });
                                }
                            }
                        }
                    }
                    if has_images {
                        return PreviewState::RichMarkdown {
                            source,
                            base_dir: base_dir.map(|p| p.to_path_buf()),
                            images: rendered_images,
                            total_lines,
                        };
                    }
                    // Pure text — render now (no images).
                    let lines = markdown::render_markdown(&source, 200);
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
