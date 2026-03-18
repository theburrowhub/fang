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
