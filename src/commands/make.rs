use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;
use tokio::io::{AsyncBufReadExt, BufReader};
use anyhow::{Result, Context};
use crate::app::state::MakeTarget;
use crate::app::events::Event;

/// Verifica si existe un Makefile en el directorio dado.
pub fn has_makefile(dir: &Path) -> bool {
    find_makefile(dir).is_some()
}

/// Encuentra el Makefile en el directorio (busca "Makefile", "makefile", "GNUmakefile").
pub fn find_makefile(dir: &Path) -> Option<PathBuf> {
    for name in &["Makefile", "makefile", "GNUmakefile"] {
        let path = dir.join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Parsea los targets de un Makefile.
///
/// Reglas:
/// - Línea válida de target: empieza en columna 0, tiene ":", nombre válido
/// - Nombre válido: caracteres alfanuméricos, guiones, underscores, puntos
/// - NO son targets: líneas con :=, ?=, +=  (son variables)
/// - NO son targets: líneas que empiezan con TAB (son recetas)
/// - NO son targets: líneas que empiezan con "." seguido de mayúsculas (.PHONY, .DEFAULT, etc.)
/// - Descripción: comentario ## en la línea inmediatamente anterior
///
/// Ejemplos válidos:
///   "build:"                -> target "build"
///   "build: clean deps"     -> target "build"
///   "test-e2e: build"       -> target "test-e2e"
///   "deploy_prod:"          -> target "deploy_prod"
///
/// Ejemplos inválidos:
///   "CC := gcc"             -> variable
///   "\tcargo build"         -> recipe
///   ".PHONY: build"         -> special target (ignorar)
///   "# comment"             -> comment
pub fn parse_targets(path: &Path) -> Result<Vec<MakeTarget>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read Makefile: {:?}", path))?;

    parse_targets_from_content(&content)
}

/// Parsea targets desde el contenido string de un Makefile.
pub fn parse_targets_from_content(content: &str) -> Result<Vec<MakeTarget>> {
    let mut targets = Vec::new();
    let mut pending_description: Option<String> = None;

    for (line_number, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Empty line: clear pending description
        if trimmed.is_empty() {
            pending_description = None;
            continue;
        }

        // Description comment "## ..."
        if trimmed.starts_with("## ") {
            pending_description = Some(trimmed[3..].trim().to_string());
            continue;
        }

        // Regular comment or recipe line (tab-indented): clear description and skip.
        // At this point we know the line does NOT start with "## " (handled above),
        // so any '#' line is a plain comment.
        if trimmed.starts_with('#') || line.starts_with('\t') {
            pending_description = None;
            continue;
        }

        // Skip variable assignments
        if trimmed.contains(":=") || trimmed.contains("?=") || trimmed.contains("+=") {
            pending_description = None;
            continue;
        }

        // Try to parse as target
        if let Some(colon_pos) = trimmed.find(':') {
            let target_name = trimmed[..colon_pos].trim();

            // Validate target name
            if !target_name.is_empty()
                && !target_name.starts_with('.')  // Skip .PHONY etc
                && is_valid_target_name(target_name)
            {
                targets.push(MakeTarget {
                    name: target_name.to_string(),
                    description: pending_description.take(),
                    line_number,
                });
                continue;
            }
        }

        // Not a target line, clear pending description
        pending_description = None;
    }

    Ok(targets)
}

fn is_valid_target_name(name: &str) -> bool {
    // Spaces are implicitly excluded by the char allowlist, so no extra check needed.
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Ejecuta `make <target>` en el directorio dado de forma asíncrona.
///
/// Streams stdout y stderr como Event::MakeOutputLine.
/// Al terminar, envía Event::MakeDone { exit_code }.
/// Resolve the `make` binary, trying the PATH first, then common system locations.
fn find_make_binary() -> Option<std::path::PathBuf> {
    // Fast path: let the OS resolve via PATH
    if std::process::Command::new("make").arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status().is_ok()
    {
        return Some(std::path::PathBuf::from("make"));
    }
    // Fallback: common installation locations (macOS, Linux)
    for path in &[
        "/usr/bin/make",
        "/usr/local/bin/make",
        "/opt/homebrew/bin/make",
        "/usr/gnu/bin/make",
    ] {
        if std::path::Path::new(path).exists() {
            return Some(std::path::PathBuf::from(path));
        }
    }
    None
}

pub async fn run_target(
    target: &str,
    dir: &Path,
    tx: UnboundedSender<Event>,
) -> Result<()> {
    use tokio::process::Command;

    let make_bin = match find_make_binary() {
        Some(p) => p,
        None => {
            let _ = tx.send(Event::MakeOutputLine(
                "Error: 'make' not found in PATH or common locations.".to_string(),
            ));
            let _ = tx.send(Event::MakeOutputLine(
                "  macOS: run `xcode-select --install` to install make.".to_string(),
            ));
            let _ = tx.send(Event::MakeOutputLine(
                "  Linux: run `sudo apt install make` or equivalent.".to_string(),
            ));
            let _ = tx.send(Event::MakeDone { exit_code: -1 });
            return Ok(());
        }
    };

    let mut child = match Command::new(&make_bin)
        .arg(target)
        .current_dir(dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let _ = tx.send(Event::MakeOutputLine(
                "Error: 'make' command not found".to_string(),
            ));
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

    // Stream stdout and stderr concurrently
    let stdout_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            if tx_stdout.send(Event::MakeOutputLine(line)).is_err() {
                break;
            }
        }
    });

    let stderr_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            if tx_stderr.send(Event::MakeOutputLine(format!("stderr: {}", line))).is_err() {
                break;
            }
        }
    });

    // Wait for process to finish and both readers to complete
    let exit_status = child.wait().await
        .with_context(|| "Failed to wait for make process")?;

    // Wait for readers to flush
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
        let path = write_temp_makefile(content, "simple");
        let targets = parse_targets(&path).unwrap();
        assert!(targets.iter().any(|t| t.name == "build"), "build target missing");
        assert!(targets.iter().any(|t| t.name == "test"), "test target missing");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_with_descriptions() {
        let content = "## Build the project\nbuild:\n\tcargo build\n";
        let path = write_temp_makefile(content, "desc");
        let targets = parse_targets(&path).unwrap();
        let build = targets.iter().find(|t| t.name == "build").unwrap();
        assert_eq!(build.description, Some("Build the project".to_string()));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_ignores_variables() {
        let content = "CC := gcc\nLD ?= ld\nbuild:\n\t$(CC) main.c\n";
        let path = write_temp_makefile(content, "vars");
        let targets = parse_targets(&path).unwrap();
        assert!(!targets.iter().any(|t| t.name == "CC" || t.name == "LD"));
        assert!(targets.iter().any(|t| t.name == "build"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_ignores_phony_lines() {
        let content = ".PHONY: build test\nbuild:\n\tcargo build\n";
        let path = write_temp_makefile(content, "phony");
        let targets = parse_targets(&path).unwrap();
        assert!(!targets.iter().any(|t| t.name.starts_with('.')));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_with_dependencies() {
        let content = "test: build\n\tcargo test\nbuild:\n\tcargo build\n";
        let path = write_temp_makefile(content, "deps");
        let targets = parse_targets(&path).unwrap();
        let test_target = targets.iter().find(|t| t.name == "test");
        assert!(test_target.is_some(), "test target should exist");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_hyphenated_and_underscored() {
        let content = "build-release:\n\tcargo build --release\n\nrun_tests:\n\tcargo test\n";
        let path = write_temp_makefile(content, "hyph");
        let targets = parse_targets(&path).unwrap();
        assert!(targets.iter().any(|t| t.name == "build-release"));
        assert!(targets.iter().any(|t| t.name == "run_tests"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_is_valid_target_name() {
        assert!(is_valid_target_name("build"));
        assert!(is_valid_target_name("build-release"));
        assert!(is_valid_target_name("test_suite"));
        assert!(!is_valid_target_name(""));
        assert!(!is_valid_target_name("build release")); // space
    }

    #[test]
    fn test_find_makefile_nonexistent_dir_returns_none() {
        let path = std::path::Path::new("/tmp/fang_this_dir_does_not_exist_xyzzy");
        assert!(find_makefile(path).is_none(), "Non-existent dir should return None");
    }

    #[test]
    fn test_has_makefile_false_for_empty_dir() {
        let dir = PathBuf::from("/tmp/fang_empty_dir_test");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!has_makefile(&dir));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_has_makefile_true_when_present() {
        let dir = PathBuf::from("/tmp/fang_has_makefile_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Makefile"), "build:\n\techo ok\n").unwrap();
        assert!(has_makefile(&dir));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_find_makefile_lowercase() {
        let dir = PathBuf::from("/tmp/fang_lower_makefile_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("makefile"), "build:\n\techo ok\n").unwrap();
        let found = find_makefile(&dir);
        // On case-insensitive filesystems (macOS APFS/HFS+), "Makefile" and "makefile"
        // resolve to the same inode, so find_makefile may return either name.
        // The important thing is that it finds *something*.
        assert!(found.is_some(), "Should find the lowercase makefile");
        let found_path = found.unwrap();
        let file_name = found_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(
            file_name.eq_ignore_ascii_case("makefile"),
            "Found file '{}' should be a makefile variant",
            file_name
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_find_makefile_gnumakefile() {
        let dir = PathBuf::from("/tmp/fang_gnu_makefile_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("GNUmakefile"), "build:\n\techo ok\n").unwrap();
        let found = find_makefile(&dir);
        assert!(found.is_some());
        assert!(found.unwrap().ends_with("GNUmakefile"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_parse_targets_from_content_direct() {
        let content = "## Run all tests\ntest:\n\tcargo test\n\n## Build release binary\nbuild-release:\n\tcargo build --release\n";
        let targets = parse_targets_from_content(content).unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].name, "test");
        assert_eq!(targets[0].description, Some("Run all tests".to_string()));
        assert_eq!(targets[1].name, "build-release");
        assert_eq!(targets[1].description, Some("Build release binary".to_string()));
    }

    #[test]
    fn test_parse_description_cleared_by_empty_line() {
        let content = "## This should not attach\n\nbuild:\n\tcargo build\n";
        let targets = parse_targets_from_content(content).unwrap();
        let build = targets.iter().find(|t| t.name == "build").unwrap();
        assert!(build.description.is_none(), "description should be cleared by empty line");
    }

    #[test]
    fn test_parse_line_numbers() {
        let content = "build:\n\tcargo build\n\ntest:\n\tcargo test\n";
        let targets = parse_targets_from_content(content).unwrap();
        let build = targets.iter().find(|t| t.name == "build").unwrap();
        let test = targets.iter().find(|t| t.name == "test").unwrap();
        assert_eq!(build.line_number, 0);
        assert_eq!(test.line_number, 3);
    }

    #[test]
    fn test_parse_real_world_makefile() {
        let content = r#"
## Build the project
build:
	cargo build

## Run unit tests
test:
	cargo test

## Run tests with race detector
test-race:
	cargo test -- --test-threads=1

CC := gcc
LD ?= ld
CFLAGS += -Wall

.PHONY: build test test-race clean

## Clean build artifacts
clean:
	cargo clean
"#;
        let targets = parse_targets_from_content(content).unwrap();
        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();

        assert!(names.contains(&"build"), "build missing");
        assert!(names.contains(&"test"), "test missing");
        assert!(names.contains(&"test-race"), "test-race missing");
        assert!(names.contains(&"clean"), "clean missing");
        assert!(!names.contains(&"CC"), "CC (variable) should not be a target");
        assert!(!names.contains(&"LD"), "LD (variable) should not be a target");
        assert!(!names.iter().any(|n| n.starts_with('.')), ".PHONY should not be a target");

        let build_target = targets.iter().find(|t| t.name == "build").unwrap();
        assert_eq!(build_target.description, Some("Build the project".to_string()));

        let clean_target = targets.iter().find(|t| t.name == "clean").unwrap();
        assert_eq!(clean_target.description, Some("Clean build artifacts".to_string()));
    }

    #[tokio::test]
    async fn test_run_target_echo() {
        let content = "hello:\n\t@echo 'Hello from Fang make!'\n";
        let dir = PathBuf::from("/tmp/fang_make_test_u4");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Makefile"), content).unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let dir_clone = dir.clone();
        tokio::spawn(async move {
            let _ = run_target("hello", &dir_clone, tx).await;
        });

        let mut output_lines = vec![];
        let mut got_done = false;
        let timeout = tokio::time::Duration::from_secs(10);

        let _ = tokio::time::timeout(timeout, async {
            while !got_done {
                match rx.recv().await {
                    Some(Event::MakeOutputLine(line)) => output_lines.push(line),
                    Some(Event::MakeDone { exit_code }) => {
                        assert_eq!(exit_code, 0, "make should succeed");
                        got_done = true;
                    }
                    None => break,
                    _ => {}
                }
            }
        }).await;

        assert!(got_done, "Should have received MakeDone event");
        assert!(
            output_lines.iter().any(|l| l.contains("Hello") || l.contains("hello")),
            "Expected echo output, got: {:?}",
            output_lines
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_run_target_failing() {
        let content = "fail:\n\texit 1\n";
        let dir = PathBuf::from("/tmp/fang_make_fail_test_u4");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Makefile"), content).unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let dir_clone = dir.clone();
        tokio::spawn(async move {
            let _ = run_target("fail", &dir_clone, tx).await;
        });

        let mut exit_code = None;
        let timeout = tokio::time::Duration::from_secs(10);

        let _ = tokio::time::timeout(timeout, async {
            loop {
                match rx.recv().await {
                    Some(Event::MakeDone { exit_code: code }) => {
                        exit_code = Some(code);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }).await;

        assert!(exit_code.is_some(), "Should have received MakeDone");
        assert_ne!(exit_code.unwrap(), 0, "Failed target should have non-zero exit code");

        std::fs::remove_dir_all(&dir).ok();
    }
}
