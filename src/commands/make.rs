use std::path::Path;
use anyhow::Result;
use crate::app::state::MakeTarget;

/// Parse Make targets from a Makefile in the given directory.
/// Returns an empty list if no Makefile is present.
pub async fn parse_targets(dir: &Path) -> Result<Vec<MakeTarget>> {
    let makefile_path = dir.join("Makefile");

    let content = match tokio::fs::read_to_string(&makefile_path).await {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut targets = Vec::new();
    let mut prev_comment: Option<String> = None;

    for (line_number, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Capture comment lines as potential descriptions
        if trimmed.starts_with('#') {
            prev_comment = Some(trimmed.trim_start_matches('#').trim().to_string());
            continue;
        }

        // Match target lines: "target_name: ..."
        if let Some(colon_pos) = trimmed.find(':') {
            let target_name = trimmed[..colon_pos].trim();
            // Skip phony lines and variables
            if target_name.is_empty()
                || target_name.contains('=')
                || target_name.starts_with('.')
                || target_name.contains(' ')
            {
                prev_comment = None;
                continue;
            }

            targets.push(MakeTarget {
                name: target_name.to_string(),
                description: prev_comment.take().unwrap_or_default(),
                line_number,
            });
        } else {
            prev_comment = None;
        }
    }

    Ok(targets)
}

/// Run a Make target in the given directory, capturing output
pub async fn run_target(dir: &Path, target: &str) -> Result<Vec<String>> {
    let output = tokio::process::Command::new("make")
        .arg(target)
        .current_dir(dir)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut lines: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
    if !stderr.is_empty() {
        lines.push("--- stderr ---".to_string());
        lines.extend(stderr.lines().map(|s| s.to_string()));
    }

    Ok(lines)
}
