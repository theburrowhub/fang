use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use anyhow::{Result, Context};
use crate::app::state::MakeTarget;
use crate::app::events::Event;

pub fn has_makefile(dir: &Path) -> bool { find_makefile(dir).is_some() }

pub fn find_makefile(dir: &Path) -> Option<PathBuf> {
    for name in &["Makefile", "makefile", "GNUmakefile"] {
        let path = dir.join(name);
        if path.exists() { return Some(path); }
    }
    None
}

pub fn parse_targets(path: &Path) -> Result<Vec<MakeTarget>> {
    let content = std::fs::read_to_string(path).with_context(|| format!("Cannot read Makefile: {:?}", path))?;
    parse_targets_from_content(&content)
}

pub fn parse_targets_from_content(content: &str) -> Result<Vec<MakeTarget>> {
    let mut targets = Vec::new();
    let mut pending_description: Option<String> = None;
    for (line_number, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() { pending_description = None; continue; }
        if trimmed.starts_with("## ") { pending_description = Some(trimmed[3..].trim().to_string()); continue; }
        if trimmed.starts_with('#') || line.starts_with('\t') { pending_description = None; continue; }
        if trimmed.contains(":=") || trimmed.contains("?=") || trimmed.contains("+=") { pending_description = None; continue; }
        if let Some(colon_pos) = trimmed.find(':') {
            let target_name = trimmed[..colon_pos].trim();
            if !target_name.is_empty() && !target_name.starts_with('.') && is_valid_target_name(target_name) {
                targets.push(MakeTarget { name: target_name.to_string(), description: pending_description.take(), line_number });
                continue;
            }
        }
        pending_description = None;
    }
    Ok(targets)
}

fn is_valid_target_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
}

pub async fn run_target(target: &str, dir: &Path, tx: tokio::sync::mpsc::UnboundedSender<Event>) -> Result<()> {
    use tokio::process::Command;
    let mut child = match Command::new("make").arg(target).current_dir(dir).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn() {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let _ = tx.send(Event::MakeOutputLine("Error: 'make' command not found".to_string()));
            let _ = tx.send(Event::MakeDone { exit_code: -1 });
            return Ok(());
        }
        Err(e) => return Err(e).with_context(|| format!("Failed to spawn make {}", target)),
    };
    let stdout = child.stdout.take().expect("stdout should be captured");
    let stderr = child.stderr.take().expect("stderr should be captured");
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    let tx_stdout = tx.clone();
    let tx_stderr = tx.clone();
    let stdout_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            if tx_stdout.send(Event::MakeOutputLine(line)).is_err() { break; }
        }
    });
    let stderr_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            if tx_stderr.send(Event::MakeOutputLine(format!("stderr: {}", line))).is_err() { break; }
        }
    });
    let exit_status = child.wait().await.with_context(|| "Failed to wait for make process")?;
    let _ = tokio::join!(stdout_task, stderr_task);
    let exit_code = exit_status.code().unwrap_or(-1);
    let _ = tx.send(Event::MakeDone { exit_code });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    fn write_temp_makefile(content: &str, suffix: &str) -> PathBuf {
        let path = PathBuf::from(format!("/tmp/test_Makefile_fang_{}", suffix));
        std::fs::write(&path, content).unwrap();
        path
    }
    #[test]
    fn test_parse_simple_targets() {
        let content = "build:\n\tcargo build\n\ntest:\n\tcargo test\n";
        let path = write_temp_makefile(content, "simple2");
        let targets = parse_targets(&path).unwrap();
        assert!(targets.iter().any(|t| t.name == "build"));
        assert!(targets.iter().any(|t| t.name == "test"));
        std::fs::remove_file(&path).ok();
    }
    #[test]
    fn test_parse_with_descriptions() {
        let content = "## Build the project\nbuild:\n\tcargo build\n";
        let path = write_temp_makefile(content, "desc2");
        let targets = parse_targets(&path).unwrap();
        assert_eq!(targets[0].description, Some("Build the project".to_string()));
        std::fs::remove_file(&path).ok();
    }
    #[test]
    fn test_parse_ignores_variables() {
        let content = "CC := gcc\nbuild:\n\t$(CC) main.c\n";
        let path = write_temp_makefile(content, "vars2");
        let targets = parse_targets(&path).unwrap();
        assert!(targets.iter().all(|t| t.name != "CC"));
        std::fs::remove_file(&path).ok();
    }
    #[test]
    fn test_parse_ignores_phony() {
        let content = ".PHONY: build test\nbuild:\n\tcargo build\n";
        let path = write_temp_makefile(content, "phony2");
        let targets = parse_targets(&path).unwrap();
        assert!(!targets.iter().any(|t| t.name.starts_with('.')));
        std::fs::remove_file(&path).ok();
    }
    #[test]
    fn test_parse_hyphenated_targets() {
        let content = "build-release:\n\tcargo build --release\n";
        let path = write_temp_makefile(content, "hyphen2");
        let targets = parse_targets(&path).unwrap();
        assert_eq!(targets[0].name, "build-release");
        std::fs::remove_file(&path).ok();
    }
    #[test]
    fn test_is_valid_target_name() {
        assert!(is_valid_target_name("build"));
        assert!(is_valid_target_name("test-e2e"));
        assert!(!is_valid_target_name(""));
        assert!(!is_valid_target_name("has space"));
    }
    #[test]
    fn test_find_makefile_none() {
        assert!(find_makefile(&PathBuf::from("/tmp/nonexistent_fang_test_dir")).is_none());
    }
}
