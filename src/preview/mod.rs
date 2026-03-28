use crate::app::state::{FileEntry, PreviewState};
use std::path::Path;

pub mod binary;
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

            // Markdown files: store the source and render lazily at draw time
            // so the text always uses the actual panel width.
            let ext = entry
                .extension
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase();
            if MARKDOWN_EXTENSIONS.contains(&ext.as_str()) {
                if let Ok(source) = String::from_utf8(data) {
                    let total_lines = source.lines().count();
                    return PreviewState::Markdown {
                        source,
                        total_lines,
                    };
                }
                // Not valid UTF-8 — fall through to syntax-highlighted text path
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
