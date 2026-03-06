use crate::app::state::{MakeTarget, PreviewState, StyledLine};
use ratatui::style::{Color, Modifier, Style};
use std::path::Path;

/// Parse targets from a Makefile
pub fn parse_makefile_targets(content: &str) -> Vec<MakeTarget> {
    let mut targets = Vec::new();
    let mut pending_description: Option<String> = None;

    for (line_number, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Check for description comment (## Description)
        if trimmed.starts_with("## ") {
            pending_description = Some(trimmed[3..].trim().to_string());
            continue;
        } else if trimmed.starts_with('#') || trimmed.is_empty() {
            // Regular comment or empty line - clear description
            if !trimmed.starts_with("##") {
                pending_description = None;
            }
            continue;
        }

        // Skip lines starting with TAB (recipe lines)
        if line.starts_with('\t') {
            continue;
        }

        // Skip variable assignments
        if trimmed.contains(":=") || trimmed.contains("?=") || trimmed.contains("+=") {
            pending_description = None;
            continue;
        }

        // Check if this looks like a target: "name:" or "name: deps"
        if let Some(colon_pos) = trimmed.find(':') {
            let target_name = trimmed[..colon_pos].trim();

            // Valid target: non-empty, no spaces, doesn't start with "."
            if !target_name.is_empty()
                && !target_name.starts_with('.')
                && !target_name.contains(' ')
                && !target_name.contains('=')
                && target_name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                targets.push(MakeTarget {
                    name: target_name.to_string(),
                    description: pending_description.take(),
                    line_number,
                });
                continue;
            }
        }

        pending_description = None;
    }

    targets
}

pub async fn load_makefile_preview(path: &Path) -> PreviewState {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return PreviewState::Error(format!("Cannot read Makefile: {}", e)),
    };

    // Precompute shared styles to avoid repeated construction
    let style_comment = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::ITALIC);
    let style_recipe = Style::default().fg(Color::White);
    let style_target = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let style_variable = Style::default().fg(Color::Yellow);

    // Single pass: build styled lines; total_lines equals the resulting vec length
    // (Makefiles are small and are never truncated at MAX_LINES here)
    let lines: Vec<StyledLine> = content
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();

            let style = if trimmed.starts_with('#') {
                style_comment
            } else if line.starts_with('\t') {
                style_recipe
            } else if trimmed.contains(":=") || trimmed.contains("?=") || trimmed.contains("+=") {
                style_variable
            } else if trimmed.ends_with(':') || trimmed.contains(':') {
                style_target
            } else {
                style_recipe
            };

            StyledLine {
                spans: vec![(style, line.to_string())],
            }
        })
        .collect();

    let total_lines = lines.len();
    PreviewState::Text { lines, total_lines }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_targets() {
        let content = "build:\n\tcargo build\n\ntest:\n\tcargo test\n";
        let targets = parse_makefile_targets(content);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].name, "build");
        assert_eq!(targets[1].name, "test");
    }

    #[test]
    fn test_parse_with_descriptions() {
        let content = "## Build the project\nbuild:\n\tcargo build\n";
        let targets = parse_makefile_targets(content);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "build");
        assert_eq!(
            targets[0].description,
            Some("Build the project".to_string())
        );
    }

    #[test]
    fn test_parse_ignores_variables() {
        let content = "CC := gcc\nVAR ?= default\nbuild:\n\t$(CC) main.c\n";
        let targets = parse_makefile_targets(content);
        assert!(targets.iter().all(|t| t.name != "CC" && t.name != "VAR"));
        assert!(targets.iter().any(|t| t.name == "build"));
    }

    #[test]
    fn test_parse_ignores_phony() {
        let content = ".PHONY: build test\nbuild:\n\tcargo build\n";
        let targets = parse_makefile_targets(content);
        assert!(!targets.iter().any(|t| t.name.starts_with('.')));
    }

    #[test]
    fn test_parse_with_dependencies() {
        let content = "test: build\n\tcargo test\n";
        let targets = parse_makefile_targets(content);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "test");
    }

    #[test]
    fn test_parse_hyphenated_targets() {
        let content = "build-release:\n\tcargo build --release\n";
        let targets = parse_makefile_targets(content);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "build-release");
    }
}
