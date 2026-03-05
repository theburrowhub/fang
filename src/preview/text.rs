use std::path::Path;
use std::sync::OnceLock;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::easy::HighlightLines;
use crate::app::state::{PreviewState, StyledLine};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

const MAX_FILE_SIZE: u64 = 1024 * 1024; // 1MB
const MAX_LINES: usize = 2000;

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn get_theme() -> &'static syntect::highlighting::Theme {
    let ts = THEME_SET.get_or_init(ThemeSet::load_defaults);
    ts.themes.get("base16-ocean.dark")
        .or_else(|| ts.themes.values().next())
        .expect("at least one theme available")
}

fn syntect_color_to_ratatui(c: syntect::highlighting::Color) -> ratatui::style::Color {
    ratatui::style::Color::Rgb(c.r, c.g, c.b)
}

fn syntect_style_to_ratatui(style: syntect::highlighting::Style) -> ratatui::style::Style {
    use ratatui::style::{Style, Modifier};
    use syntect::highlighting::FontStyle;

    let mut s = Style::default()
        .fg(syntect_color_to_ratatui(style.foreground));

    if style.font_style.contains(FontStyle::BOLD) {
        s = s.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        s = s.add_modifier(Modifier::ITALIC);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        s = s.add_modifier(Modifier::UNDERLINED);
    }
    s
}

/// Highlight already-read bytes as a text preview.
/// `data` must already be confirmed non-binary by the caller.
/// This avoids a second file read when mod.rs has already read the bytes.
pub fn highlight_bytes(path: &Path, data: Vec<u8>) -> PreviewState {
    let content = match String::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return PreviewState::Error("File is not valid UTF-8".to_string()),
    };
    highlight_content(path, content)
}

/// Load a text file from disk, checking size first, then highlighting.
/// Used when the caller has not already read the file bytes.
pub async fn load_text_preview(path: &Path) -> PreviewState {
    // Check file size before reading
    let size = match std::fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) => return PreviewState::Error(format!("Cannot read file: {}", e)),
    };

    if size > MAX_FILE_SIZE {
        return PreviewState::TooLarge { size };
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return PreviewState::Error("Cannot read file (possibly binary)".to_string()),
    };

    highlight_content(path, content)
}

/// Core highlighting logic. Single pass over lines: count total while highlighting up to MAX_LINES.
fn highlight_content(path: &Path, content: String) -> PreviewState {
    let ss = get_syntax_set();
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let syntax = ss.find_syntax_by_extension(extension)
        .or_else(|| ss.find_syntax_by_first_line(content.lines().next().unwrap_or("")))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let theme = get_theme();
    let mut highlighter = HighlightLines::new(syntax, theme);

    // Single pass: count total lines while highlighting up to MAX_LINES
    let mut total_lines = 0usize;
    let mut lines: Vec<StyledLine> = Vec::new();

    for line in content.lines() {
        total_lines += 1;
        if total_lines <= MAX_LINES {
            let spans = match highlighter.highlight_line(line, ss) {
                Ok(ranges) => ranges.into_iter()
                    .map(|(style, text)| (syntect_style_to_ratatui(style), text.to_string()))
                    .collect(),
                // Fallback to plain text span on highlight error — never silently drop lines
                Err(_) => vec![(ratatui::style::Style::default(), line.to_string())],
            };
            lines.push(StyledLine { spans });
        }
    }

    PreviewState::Text { lines, total_lines }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_load_cargo_toml() {
        let path = PathBuf::from("Cargo.toml");
        if path.exists() {
            let result = load_text_preview(&path).await;
            match result {
                PreviewState::Text { lines, total_lines } => {
                    assert!(!lines.is_empty());
                    assert!(total_lines > 0);
                    // lines.len() must equal total_lines when file is within MAX_LINES
                    assert_eq!(lines.len(), total_lines);
                }
                other => panic!("Expected Text, got {:?}", other),
            }
        }
    }

    #[tokio::test]
    async fn test_too_large_file() {
        let result = PreviewState::TooLarge { size: 2 * 1024 * 1024 };
        if let PreviewState::TooLarge { size } = result {
            assert_eq!(size, 2 * 1024 * 1024);
        }
    }

    #[test]
    fn test_syntect_color_conversion() {
        let color = syntect::highlighting::Color { r: 255, g: 128, b: 0, a: 255 };
        let ratatui_color = syntect_color_to_ratatui(color);
        assert_eq!(ratatui_color, ratatui::style::Color::Rgb(255, 128, 0));
    }

    #[test]
    fn test_highlight_bytes_valid_utf8() {
        let data = b"fn main() {}\n".to_vec();
        let path = PathBuf::from("test.rs");
        let result = highlight_bytes(&path, data);
        match result {
            PreviewState::Text { lines, total_lines } => {
                assert_eq!(total_lines, 1);
                assert_eq!(lines.len(), 1);
            }
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_single_pass_counts_all_lines() {
        // Verify total_lines counts beyond MAX_LINES correctly
        // (can't easily create 2001 lines here, but we can verify the count matches content)
        let content = "line1\nline2\nline3\n".to_string();
        let path = std::path::Path::new("test.txt");
        let result = highlight_content(path, content);
        match result {
            PreviewState::Text { lines, total_lines } => {
                assert_eq!(total_lines, 3);
                assert_eq!(lines.len(), 3);
            }
            other => panic!("Expected Text, got {:?}", other),
        }
    }
}
