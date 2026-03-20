//! Command palette items — the source of truth for the Ctrl+K spotlight popup.

use crate::app::actions::Action;

/// A single entry in the command palette.
pub struct PaletteItem {
    /// Human-readable label shown in the list.
    pub label: &'static str,
    /// Key binding hint shown on the right side of each row.
    pub hint: &'static str,
    /// Action dispatched when the item is selected.
    pub action: Action,
}

/// What happens when the user confirms a palette selection.
pub enum PaletteResult {
    /// Dispatch an existing fang action.
    Action(Action),
    /// Run `cmd` as a shell command (equivalent to `:cmd` + Enter).
    RunShell(String),
    /// Open `cmd` in a terminal split (equivalent to `;cmd` + Enter).
    OpenSplit(String),
}

/// Returns the full list of palette items, in display order.
pub fn all_palette_items() -> Vec<PaletteItem> {
    vec![
        PaletteItem {
            label: "Git Operations",
            hint: "g",
            action: Action::OpenGitMenu,
        },
        PaletteItem {
            label: "Makefile Targets",
            hint: "m",
            action: Action::OpenMakeModal,
        },
        PaletteItem {
            label: "AI Prompt",
            hint: "i",
            action: Action::OpenAiPrompt,
        },
        PaletteItem {
            label: "Toggle AI Panel",
            hint: "a",
            action: Action::ToggleAiPanel,
        },
        PaletteItem {
            label: "Change AI Provider",
            hint: "I",
            action: Action::OpenAiProviderSelect,
        },
        PaletteItem {
            label: "Reset AI Session",
            hint: "Ctrl+R",
            action: Action::ResetAiSession,
        },
        PaletteItem {
            label: "Open in Terminal Split",
            hint: ";",
            action: Action::OpenExternalCommand,
        },
        PaletteItem {
            label: "Fuzzy Search",
            hint: "/",
            action: Action::OpenSearch,
        },
        PaletteItem {
            label: "Run Shell Command",
            hint: ":",
            action: Action::OpenCommandInput,
        },
        PaletteItem {
            label: "Open with System App",
            hint: "o",
            action: Action::OpenWithSystem,
        },
        PaletteItem {
            label: "New Empty File",
            hint: "n",
            action: Action::OpenNewFile,
        },
        PaletteItem {
            label: "New File from Clipboard",
            hint: "N",
            action: Action::OpenNewFileFromClipboard,
        },
        PaletteItem {
            label: "Copy Relative Path",
            hint: "c",
            action: Action::CopyRelPath,
        },
        PaletteItem {
            label: "Copy Absolute Path",
            hint: "C",
            action: Action::CopyAbsPath,
        },
        PaletteItem {
            label: "Toggle Preview",
            hint: "p",
            action: Action::TogglePreview,
        },
        PaletteItem {
            label: "Toggle Git Diff",
            hint: "d",
            action: Action::ToggleGitDiff,
        },
        PaletteItem {
            label: "Settings",
            hint: "Ctrl+S",
            action: Action::OpenSettings,
        },
        PaletteItem {
            label: "Help",
            hint: "h",
            action: Action::OpenHelp,
        },
        PaletteItem {
            label: "Parent Directory",
            hint: "u / ←",
            action: Action::NavLeft,
        },
        PaletteItem {
            label: "Quit",
            hint: "q",
            action: Action::Quit,
        },
    ]
}

/// Returns only items whose label contains `query` (case-insensitive).
pub fn filtered_items(query: &str) -> Vec<PaletteItem> {
    let q = query.to_lowercase();
    all_palette_items()
        .into_iter()
        .filter(|item| q.is_empty() || item.label.to_lowercase().contains(&q))
        .collect()
}

/// Total number of selectable rows given a query.
///
/// If there are matching actions, that count is returned.
/// If there are no matches but the query is non-empty, two fallback rows are
/// shown ("Run in shell" and "Open in split"), so the count is 2.
pub fn item_count(query: &str) -> usize {
    let n = filtered_items(query).len();
    if n > 0 {
        n
    } else if !query.is_empty() {
        2
    } else {
        0
    }
}

/// Resolve the selected row into a `PaletteResult`.
///
/// When there are matching action items the selection indexes into them.
/// When there are no matches (but the query is non-empty) the two shell
/// fallback rows are available: index 0 = run, index 1 = split.
pub fn resolve_selection(query: &str, selected: usize) -> Option<PaletteResult> {
    let items = filtered_items(query);
    if !items.is_empty() {
        items
            .into_iter()
            .nth(selected)
            .map(|i| PaletteResult::Action(i.action))
    } else if !query.is_empty() {
        match selected {
            0 => Some(PaletteResult::RunShell(query.to_string())),
            1 => Some(PaletteResult::OpenSplit(query.to_string())),
            _ => None,
        }
    } else {
        None
    }
}
