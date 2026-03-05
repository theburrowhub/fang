use std::path::Path;
use ratatui::style::{Color, Modifier, Style};
use crate::app::state::{PreviewState, StyledLine};

pub async fn load_makefile_preview(path: &Path) -> PreviewState {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return PreviewState::Error(format!("Cannot read Makefile: {}", e)),
    };
    let style_comment = Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC);
    let style_recipe = Style::default().fg(Color::White);
    let style_target = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let style_variable = Style::default().fg(Color::Yellow);
    let lines: Vec<StyledLine> = content.lines().map(|line| {
        let trimmed = line.trim_start();
        let style = if trimmed.starts_with('#') { style_comment }
            else if line.starts_with('\t') { style_recipe }
            else if trimmed.contains(":=") || trimmed.contains("?=") || trimmed.contains("+=") { style_variable }
            else if trimmed.contains(':') { style_target }
            else { style_recipe };
        StyledLine { spans: vec![(style, line.to_string())] }
    }).collect();
    let total_lines = lines.len();
    PreviewState::Text { lines, total_lines }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_load_makefile_preview_nonexistent() {
        let path = std::path::Path::new("/tmp/nonexistent_makefile_fang_test");
        assert!(matches!(load_makefile_preview(path).await, PreviewState::Error(_)));
    }
    #[tokio::test]
    async fn test_load_makefile_preview_ok() {
        let path = std::path::PathBuf::from("/tmp/test_makefile_preview_fang2.mk");
        std::fs::write(&path, "build:\n\tcargo build\n").unwrap();
        assert!(matches!(load_makefile_preview(&path).await, PreviewState::Text { .. }));
        std::fs::remove_file(&path).ok();
    }
}
