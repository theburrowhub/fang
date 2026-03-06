use crate::app::events::Event;
use anyhow::Result;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::UnboundedSender;

// ── Static parameter definitions ─────────────────────────────────────────────

/// How a parameter contributes to the final `git` command-line.
#[derive(Debug, Clone, Copy)]
pub enum GitParamKind {
    /// A text field.
    /// `flag`: if Some, the text is passed as `flag value` (e.g. `-m "msg"`).
    ///          if None, the value is appended as a positional arg.
    Text {
        placeholder: &'static str,
        flag: Option<&'static str>,
    },
    /// A boolean checkbox.  When true, `flag` is appended to the args.
    Bool { flag: &'static str, default: bool },
    /// A subcommand selector.  When true, `subcommand` is inserted at
    /// position 1 in the args (e.g. `git stash pop`).
    SubCmd {
        subcommand: &'static str,
        default: bool,
    },
}

/// Static definition of one parameter in a git form.
#[derive(Debug, Clone, Copy)]
pub struct GitParamDef {
    pub label: &'static str,
    pub kind: GitParamKind,
}

/// A git operation.  `params` is empty → execute immediately;
/// non-empty → show the form before running.
#[derive(Debug, Clone, Copy)]
pub struct GitOperation {
    pub label: &'static str,
    /// Base args, e.g. `&["commit"]`.  Form params are appended at runtime.
    pub base_args: &'static [&'static str],
    pub params: &'static [GitParamDef],
}

impl GitOperation {
    pub fn has_form(self) -> bool {
        !self.params.is_empty()
    }
}

// ── Dynamic form state (owned by AppMode::GitForm) ────────────────────────────

/// Current value for one parameter while the form is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitParamValue {
    Text(String),
    Bool(bool),
}

impl GitParamValue {
    #[allow(dead_code)]
    pub fn as_text(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
    #[allow(dead_code)]
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
}

/// Initialise a `Vec<GitParamValue>` from a static param slice.
pub fn default_values(params: &[GitParamDef]) -> Vec<GitParamValue> {
    params
        .iter()
        .map(|p| match p.kind {
            GitParamKind::Text { .. } => GitParamValue::Text(String::new()),
            GitParamKind::Bool { default, .. } => GitParamValue::Bool(default),
            GitParamKind::SubCmd { default, .. } => GitParamValue::Bool(default),
        })
        .collect()
}

/// Build the final argument list for `git` from base args + form values.
pub fn build_args(op: GitOperation, values: &[GitParamValue]) -> Vec<String> {
    let mut args: Vec<String> = op.base_args.iter().map(|&s| s.to_string()).collect();

    // SubCmd params are processed first: when checked, insert the subcommand
    // at position 1 and return immediately (subcommands are mutually exclusive).
    for (def, val) in op.params.iter().zip(values.iter()) {
        if let (GitParamKind::SubCmd { subcommand, .. }, GitParamValue::Bool(true)) =
            (&def.kind, val)
        {
            args.insert(1, subcommand.to_string());
            return args;
        }
    }

    for (def, val) in op.params.iter().zip(values.iter()) {
        match (&def.kind, val) {
            (GitParamKind::Text { flag: Some(f), .. }, GitParamValue::Text(s)) if !s.is_empty() => {
                args.push(f.to_string());
                args.push(s.clone());
            }
            (GitParamKind::Text { flag: None, .. }, GitParamValue::Text(s)) if !s.is_empty() => {
                args.push(s.clone());
            }
            (GitParamKind::Bool { flag, .. }, GitParamValue::Bool(true)) => {
                args.push(flag.to_string());
            }
            _ => {}
        }
    }
    args
}

// ── Operation catalogue ───────────────────────────────────────────────────────

// Static param definitions for each form operation.
static COMMIT_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Message",
        kind: GitParamKind::Text {
            placeholder: "Commit message…",
            flag: Some("-m"),
        },
    },
    GitParamDef {
        label: "Amend last commit",
        kind: GitParamKind::Bool {
            flag: "--amend",
            default: false,
        },
    },
    GitParamDef {
        label: "Allow empty commit",
        kind: GitParamKind::Bool {
            flag: "--allow-empty",
            default: false,
        },
    },
    GitParamDef {
        label: "No edit (keep last message)",
        kind: GitParamKind::Bool {
            flag: "--no-edit",
            default: false,
        },
    },
];

static ADD_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Path",
        kind: GitParamKind::Text {
            placeholder: ". (all files)",
            flag: None,
        },
    },
    GitParamDef {
        label: "Stage all tracked + untracked",
        kind: GitParamKind::Bool {
            flag: "-A",
            default: true,
        },
    },
    GitParamDef {
        label: "Interactive patch mode",
        kind: GitParamKind::Bool {
            flag: "-p",
            default: false,
        },
    },
];

static SWITCH_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Branch name",
        kind: GitParamKind::Text {
            placeholder: "branch-name",
            flag: None,
        },
    },
    GitParamDef {
        label: "Create new branch (-c)",
        kind: GitParamKind::Bool {
            flag: "-c",
            default: false,
        },
    },
];

static MERGE_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Branch to merge",
        kind: GitParamKind::Text {
            placeholder: "branch-name",
            flag: None,
        },
    },
    GitParamDef {
        label: "No fast-forward (always create merge commit)",
        kind: GitParamKind::Bool {
            flag: "--no-ff",
            default: false,
        },
    },
    GitParamDef {
        label: "Squash commits into one",
        kind: GitParamKind::Bool {
            flag: "--squash",
            default: false,
        },
    },
];

static REBASE_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Onto branch/commit",
        kind: GitParamKind::Text {
            placeholder: "main",
            flag: None,
        },
    },
    GitParamDef {
        label: "Interactive (-i)",
        kind: GitParamKind::Bool {
            flag: "-i",
            default: false,
        },
    },
];

static RESET_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Commit (default HEAD)",
        kind: GitParamKind::Text {
            placeholder: "HEAD",
            flag: None,
        },
    },
    GitParamDef {
        label: "--soft  (keep staged changes)",
        kind: GitParamKind::Bool {
            flag: "--soft",
            default: false,
        },
    },
    GitParamDef {
        label: "--hard  (discard all changes)",
        kind: GitParamKind::Bool {
            flag: "--hard",
            default: false,
        },
    },
];

static TAG_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Tag name",
        kind: GitParamKind::Text {
            placeholder: "v1.0.0",
            flag: None,
        },
    },
    GitParamDef {
        label: "Annotated tag (-a)",
        kind: GitParamKind::Bool {
            flag: "-a",
            default: false,
        },
    },
    GitParamDef {
        label: "Message (for annotated tags)",
        kind: GitParamKind::Text {
            placeholder: "Tag message…",
            flag: Some("-m"),
        },
    },
];

// ── Log / diff / branch forms ────────────────────────────────────────────────

static LOG_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Limit (number of commits)",
        kind: GitParamKind::Text {
            placeholder: "20",
            flag: Some("-n"),
        },
    },
    GitParamDef {
        label: "One line per commit (--oneline)",
        kind: GitParamKind::Bool {
            flag: "--oneline",
            default: true,
        },
    },
    GitParamDef {
        label: "Show branch graph (--graph)",
        kind: GitParamKind::Bool {
            flag: "--graph",
            default: false,
        },
    },
    GitParamDef {
        label: "All branches (--all)",
        kind: GitParamKind::Bool {
            flag: "--all",
            default: false,
        },
    },
];

static DIFF_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Path or branch (empty = working tree)",
        kind: GitParamKind::Text {
            placeholder: "",
            flag: None,
        },
    },
    GitParamDef {
        label: "Summary only (--stat)",
        kind: GitParamKind::Bool {
            flag: "--stat",
            default: true,
        },
    },
    GitParamDef {
        label: "Staged / cached changes (--staged)",
        kind: GitParamKind::Bool {
            flag: "--staged",
            default: false,
        },
    },
    GitParamDef {
        label: "Word-level diff (--word-diff)",
        kind: GitParamKind::Bool {
            flag: "--word-diff",
            default: false,
        },
    },
];

static BRANCH_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Filter (name contains)",
        kind: GitParamKind::Text {
            placeholder: "",
            flag: None,
        },
    },
    GitParamDef {
        label: "Show all (local + remote) (-a)",
        kind: GitParamKind::Bool {
            flag: "-a",
            default: true,
        },
    },
    GitParamDef {
        label: "Remote branches only (-r)",
        kind: GitParamKind::Bool {
            flag: "-r",
            default: false,
        },
    },
    GitParamDef {
        label: "Show merged branches (--merged)",
        kind: GitParamKind::Bool {
            flag: "--merged",
            default: false,
        },
    },
    GitParamDef {
        label: "Verbose — show last commit (-v)",
        kind: GitParamKind::Bool {
            flag: "-v",
            default: false,
        },
    },
];

// ── Fetch / pull / push / stash forms ────────────────────────────────────────

static FETCH_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Remote",
        kind: GitParamKind::Text {
            placeholder: "origin",
            flag: None,
        },
    },
    GitParamDef {
        label: "Fetch all remotes (--all)",
        kind: GitParamKind::Bool {
            flag: "--all",
            default: false,
        },
    },
    GitParamDef {
        label: "Prune deleted remote branches",
        kind: GitParamKind::Bool {
            flag: "--prune",
            default: true,
        },
    },
];

static PULL_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Remote",
        kind: GitParamKind::Text {
            placeholder: "origin",
            flag: None,
        },
    },
    GitParamDef {
        label: "Branch",
        kind: GitParamKind::Text {
            placeholder: "(current)",
            flag: None,
        },
    },
    GitParamDef {
        label: "Rebase instead of merge (--rebase)",
        kind: GitParamKind::Bool {
            flag: "--rebase",
            default: false,
        },
    },
    GitParamDef {
        label: "Fast-forward only (--ff-only)",
        kind: GitParamKind::Bool {
            flag: "--ff-only",
            default: false,
        },
    },
];

static PUSH_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Remote",
        kind: GitParamKind::Text {
            placeholder: "origin",
            flag: None,
        },
    },
    GitParamDef {
        label: "Branch",
        kind: GitParamKind::Text {
            placeholder: "(current)",
            flag: None,
        },
    },
    GitParamDef {
        label: "Set upstream (-u / --set-upstream)",
        kind: GitParamKind::Bool {
            flag: "--set-upstream",
            default: false,
        },
    },
    GitParamDef {
        label: "Force with lease (safe force push)",
        kind: GitParamKind::Bool {
            flag: "--force-with-lease",
            default: false,
        },
    },
    GitParamDef {
        label: "Force (--force)",
        kind: GitParamKind::Bool {
            flag: "--force",
            default: false,
        },
    },
];

static STASH_PARAMS: &[GitParamDef] = &[
    GitParamDef {
        label: "Pop — apply and remove from stash",
        kind: GitParamKind::SubCmd {
            subcommand: "pop",
            default: false,
        },
    },
    GitParamDef {
        label: "Message (for git stash push)",
        kind: GitParamKind::Text {
            placeholder: "WIP: …",
            flag: Some("-m"),
        },
    },
    GitParamDef {
        label: "Include untracked files (-u)",
        kind: GitParamKind::Bool {
            flag: "-u",
            default: false,
        },
    },
];

// ── Full catalogue ────────────────────────────────────────────────────────────

/// Full list of git operations shown in the first screen.
///
/// Operations without `params` execute immediately on Enter.
/// Operations with `params` open the form (second screen).
pub fn git_operations() -> Vec<GitOperation> {
    vec![
        // ── Inspect (Status = direct; Log/Diff/Branch = forms) ──────────
        GitOperation {
            label: "Status",
            base_args: &["status"],
            params: &[],
        },
        GitOperation {
            label: "Log…",
            base_args: &["log"],
            params: LOG_PARAMS,
        },
        GitOperation {
            label: "Diff…",
            base_args: &["diff"],
            params: DIFF_PARAMS,
        },
        GitOperation {
            label: "Branches…",
            base_args: &["branch"],
            params: BRANCH_PARAMS,
        },
        // ── Fetch / pull / push — all through forms ───────────────────────
        GitOperation {
            label: "Fetch…",
            base_args: &["fetch"],
            params: FETCH_PARAMS,
        },
        GitOperation {
            label: "Pull…",
            base_args: &["pull"],
            params: PULL_PARAMS,
        },
        GitOperation {
            label: "Push…",
            base_args: &["push"],
            params: PUSH_PARAMS,
        },
        // ── Stash ─────────────────────────────────────────────────────────
        GitOperation {
            label: "Stash…",
            base_args: &["stash"],
            params: STASH_PARAMS,
        },
        // ── Write operations — all through forms ──────────────────────────
        GitOperation {
            label: "Add…",
            base_args: &["add"],
            params: ADD_PARAMS,
        },
        GitOperation {
            label: "Commit…",
            base_args: &["commit"],
            params: COMMIT_PARAMS,
        },
        GitOperation {
            label: "Switch…",
            base_args: &["switch"],
            params: SWITCH_PARAMS,
        },
        GitOperation {
            label: "Merge…",
            base_args: &["merge"],
            params: MERGE_PARAMS,
        },
        GitOperation {
            label: "Rebase…",
            base_args: &["rebase"],
            params: REBASE_PARAMS,
        },
        GitOperation {
            label: "Reset…",
            base_args: &["reset"],
            params: RESET_PARAMS,
        },
        GitOperation {
            label: "Tag…",
            base_args: &["tag"],
            params: TAG_PARAMS,
        },
    ]
}

/// Total number of git operations — used for modal height calculation.
pub const N_GIT_OPS: usize = 15;

// ── Async runner ─────────────────────────────────────────────────────────────

fn find_git_binary() -> Option<std::path::PathBuf> {
    if std::process::Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
    {
        return Some(std::path::PathBuf::from("git"));
    }
    for path in &[
        "/usr/bin/git",
        "/usr/local/bin/git",
        "/opt/homebrew/bin/git",
        "/opt/local/bin/git",
    ] {
        if std::path::Path::new(path).exists() {
            return Some(std::path::PathBuf::from(path));
        }
    }
    None
}

pub async fn run_git(args: &[String], dir: &Path, tx: UnboundedSender<Event>) -> Result<()> {
    use tokio::process::Command;

    let git_bin = match find_git_binary() {
        Some(p) => p,
        None => {
            let _ = tx.send(Event::GitOutputLine(
                "Error: 'git' not found. Install via `brew install git` or `xcode-select --install`.".to_string(),
            ));
            let _ = tx.send(Event::GitDone { exit_code: -1 });
            return Ok(());
        }
    };

    let cmd_display = format!("git {}", args.join(" "));
    let _ = tx.send(Event::GitOutputLine(format!("$ {}", cmd_display)));

    let mut child = match Command::new(&git_bin)
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(Event::GitOutputLine(format!("Error: {}", e)));
            let _ = tx.send(Event::GitDone { exit_code: -1 });
            return Ok(());
        }
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
        assert_eq!(git_operations().len(), N_GIT_OPS);
    }

    #[test]
    fn test_direct_ops_have_no_params() {
        let ops = git_operations();
        assert!(!ops[0].has_form()); // status
        assert!(!ops[0].has_form()); // status — the only direct op
    }

    #[test]
    fn test_form_ops_have_params() {
        let ops = git_operations();
        let commit = ops.iter().find(|o| o.label.starts_with("Commit")).unwrap();
        assert!(commit.has_form());
        assert_eq!(commit.params.len(), 4);
    }

    #[test]
    fn test_build_args_commit() {
        let ops = git_operations();
        let op = *ops.iter().find(|o| o.label.starts_with("Commit")).unwrap();
        let mut values = default_values(op.params);
        values[0] = GitParamValue::Text("fix: typo".to_string());
        values[1] = GitParamValue::Bool(true); // --amend
        let args = build_args(op, &values);
        assert_eq!(args, vec!["commit", "-m", "fix: typo", "--amend"]);
    }

    #[test]
    fn test_build_args_switch_new_branch() {
        let ops = git_operations();
        let op = *ops.iter().find(|o| o.label.starts_with("Switch")).unwrap();
        let mut values = default_values(op.params);
        values[0] = GitParamValue::Text("feat/new".to_string());
        values[1] = GitParamValue::Bool(true); // -c
        let args = build_args(op, &values);
        assert_eq!(args, vec!["switch", "feat/new", "-c"]);
    }

    #[test]
    fn test_default_values() {
        let ops = git_operations();
        let op = *ops.iter().find(|o| o.label.starts_with("Add")).unwrap();
        let vals = default_values(op.params);
        assert_eq!(vals[0], GitParamValue::Text(String::new()));
        assert_eq!(vals[1], GitParamValue::Bool(true)); // -A default
    }
}
