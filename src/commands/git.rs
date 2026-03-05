use std::path::Path;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::UnboundedSender;
use anyhow::Result;
use crate::app::events::Event;

/// A static git operation with a display label and the arguments to pass to `git`.
#[derive(Debug, Clone)]
pub struct GitOperation {
    pub label: &'static str,
    pub args: &'static [&'static str],
}

/// Total number of git operations (equals `git_operations().len()`).
/// Callers that only need the count can use this constant to avoid the Vec allocation.
pub const N_GIT_OPS: usize = 13;

/// Returns the full list of git operations available in the Git menu.
pub fn git_operations() -> Vec<GitOperation> {
    vec![
        GitOperation { label: "Status",                   args: &["status"] },
        GitOperation { label: "Fetch",                    args: &["fetch"] },
        GitOperation { label: "Fetch all (prune)",        args: &["fetch", "--all", "--prune"] },
        GitOperation { label: "Pull",                     args: &["pull"] },
        GitOperation { label: "Pull (rebase)",            args: &["pull", "--rebase"] },
        GitOperation { label: "Push",                     args: &["push"] },
        GitOperation { label: "Push (force-with-lease)",  args: &["push", "--force-with-lease"] },
        GitOperation { label: "Push new branch upstream", args: &["push", "-u", "origin", "HEAD"] },
        GitOperation { label: "Log (last 20)",            args: &["log", "--oneline", "-20"] },
        GitOperation { label: "List branches",            args: &["branch", "-a"] },
        GitOperation { label: "Stash",                    args: &["stash"] },
        GitOperation { label: "Stash pop",                args: &["stash", "pop"] },
        GitOperation { label: "Diff (stat)",              args: &["diff", "--stat"] },
    ]
}

/// Runs `git <args>` in `dir`, streaming each output line as `Event::GitOutputLine`
/// and signalling completion with `Event::GitDone`.
pub async fn run_git(args: &[&str], dir: &Path, tx: UnboundedSender<Event>) -> Result<()> {
    use tokio::process::Command;

    let mut child = match Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let _ = tx.send(Event::GitOutputLine("Error: 'git' not found in PATH".to_string()));
            let _ = tx.send(Event::GitDone { exit_code: -1 });
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    let tx_out = tx.clone();
    let tx_err = tx.clone();

    let stdout_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            if tx_out.send(Event::GitOutputLine(line)).is_err() {
                break;
            }
        }
    });

    let stderr_task = tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            if tx_err.send(Event::GitOutputLine(line)).is_err() {
                break;
            }
        }
    });

    let status = child.wait().await.ok();
    let _ = tokio::join!(stdout_task, stderr_task);
    let code = status.and_then(|s| s.code()).unwrap_or(-1);
    let _ = tx.send(Event::GitDone { exit_code: code });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_operations_count() {
        let ops = git_operations();
        assert_eq!(ops.len(), 13, "Expected 13 git operations");
        assert_eq!(ops.len(), N_GIT_OPS, "N_GIT_OPS constant must match git_operations().len()");
    }

    #[test]
    fn test_git_operations_labels() {
        let ops = git_operations();
        assert_eq!(ops[0].label, "Status");
        assert_eq!(ops[1].label, "Fetch");
        assert_eq!(ops[2].label, "Fetch all (prune)");
        assert_eq!(ops[3].label, "Pull");
        assert_eq!(ops[4].label, "Pull (rebase)");
        assert_eq!(ops[5].label, "Push");
        assert_eq!(ops[6].label, "Push (force-with-lease)");
        assert_eq!(ops[7].label, "Push new branch upstream");
        assert_eq!(ops[8].label, "Log (last 20)");
        assert_eq!(ops[9].label, "List branches");
        assert_eq!(ops[10].label, "Stash");
        assert_eq!(ops[11].label, "Stash pop");
        assert_eq!(ops[12].label, "Diff (stat)");
    }

    #[test]
    fn test_git_operations_args() {
        let ops = git_operations();
        assert_eq!(ops[0].args, &["status"]);
        assert_eq!(ops[2].args, &["fetch", "--all", "--prune"]);
        assert_eq!(ops[4].args, &["pull", "--rebase"]);
        assert_eq!(ops[6].args, &["push", "--force-with-lease"]);
        assert_eq!(ops[7].args, &["push", "-u", "origin", "HEAD"]);
        assert_eq!(ops[8].args, &["log", "--oneline", "-20"]);
        assert_eq!(ops[9].args, &["branch", "-a"]);
        assert_eq!(ops[11].args, &["stash", "pop"]);
        assert_eq!(ops[12].args, &["diff", "--stat"]);
    }

    #[tokio::test]
    async fn test_run_git_status_in_repo() {
        // Run git status in the project root (which is a git repo)
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let _ = run_git(&["status"], dir, tx).await;
        });

        let mut got_done = false;
        let mut lines = vec![];
        let timeout = tokio::time::Duration::from_secs(10);
        let _ = tokio::time::timeout(timeout, async {
            loop {
                match rx.recv().await {
                    Some(Event::GitOutputLine(line)) => lines.push(line),
                    Some(Event::GitDone { exit_code }) => {
                        assert_eq!(exit_code, 0, "git status should succeed in a git repo");
                        got_done = true;
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }).await;

        assert!(got_done, "Should have received GitDone event");
        assert!(!lines.is_empty(), "git status should produce output");
    }
}
