use std::path::Path;
use std::sync::OnceLock;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::easy::HighlightLines;
use crate::app::state::{PreviewState, StyledLine};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
const MAX_LINES: usize = 2000;

fn get_syntax_set() -> &'static SyntaxSet { SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines) }
fn get_theme() -> &'static syntect::highlighting::Theme {
    let ts = THEME_SET.get_or_init(ThemeSet::load_defaults);
    ts.themes.get("base16-ocean.dark").or_else(|| ts.themes.values().next()).expect("at least one theme")
}

fn syntect_color_to_ratatui(c: syntect::highlighting::Color) -> ratatui::style::Color {
    ratatui::style::Color::Rgb(c.r, c.g, c.b)
}

fn syntect_style_to_ratatui(style: syntect::highlighting::Style) -> ratatui::style::Style {
    use ratatui::style::{Style, Modifier};
    use syntect::highlighting::FontStyle;
    let mut s = Style::default().fg(syntect_color_to_ratatui(style.foreground));
    if style.font_style.contains(FontStyle::BOLD) { s = s.add_modifier(Modifier::BOLD); }
    if style.font_style.contains(FontStyle::ITALIC) { s = s.add_modifier(Modifier::ITALIC); }
    if style.font_style.contains(FontStyle::UNDERLINE) { s = s.add_modifier(Modifier::UNDERLINED); }
    s
}

pub fn highlight_bytes(path: &Path, data: Vec<u8>) -> PreviewState {
    match String::from_utf8(data) {
        Ok(content) => highlight_content(path, content),
        Err(_) => PreviewState::Error("File is not valid UTF-8".to_string()),
    }
}

pub async fn load_text_preview(path: &Path) -> PreviewState {
    let size = match std::fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) => return PreviewState::Error(format!("Cannot read file: {}", e)),
    };
    const MAX_FILE_SIZE: u64 = 1024 * 1024;
    if size > MAX_FILE_SIZE { return PreviewState::TooLarge { size }; }
    match std::fs::read_to_string(path) {
        Ok(content) => highlight_content(path, content),
        Err(_) => PreviewState::Error("Cannot read file (possibly binary)".to_string()),
    }
}

pub fn highlight_content(path: &Path, content: String) -> PreviewState {
    let ss = get_syntax_set();
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let syntax = ss.find_syntax_by_extension(extension)
        .or_else(|| ss.find_syntax_by_first_line(content.lines().next().unwrap_or("")))
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let theme = get_theme();
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut total_lines = 0usize;
    let mut lines: Vec<StyledLine> = Vec::new();
    for line in content.lines() {
        total_lines += 1;
        if total_lines <= MAX_LINES {
            let spans = match highlighter.highlight_line(line, ss) {
                Ok(ranges) => ranges.into_iter().map(|(style, text)| (syntect_style_to_ratatui(style), text.to_string())).collect(),
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
    #[test]
    fn test_highlight_bytes_valid_utf8() {
        let data = b"fn main() {}\n".to_vec();
        let path = PathBuf::from("test.rs");
        match highlight_bytes(&path, data) {
            PreviewState::Text { lines, total_lines } => { assert_eq!(total_lines, 1); assert_eq!(lines.len(), 1); }
            other => panic!("Expected Text, got {:?}", other),
        }
    }
    #[test]
    fn test_highlight_bytes_invalid_utf8() {
        let data = vec![0xFF, 0xFE, 0x00];
        let path = PathBuf::from("test.txt");
        assert!(matches!(highlight_bytes(&path, data), PreviewState::Error(_)));
    }
    #[test]
    fn test_single_pass_counts_all_lines() {
        let content = "line1\nline2\nline3\n".to_string();
        let path = std::path::Path::new("test.txt");
        match highlight_content(path, content) {
            PreviewState::Text { total_lines, .. } => assert_eq!(total_lines, 3),
            other => panic!("Expected Text, got {:?}", other),
        }
    }
    #[test]
    fn test_syntect_color_conversion() {
        let color = syntect::highlighting::Color { r: 255, g: 128, b: 0, a: 255 };
        assert_eq!(syntect_color_to_ratatui(color), ratatui::style::Color::Rgb(255, 128, 0));
    }
}
