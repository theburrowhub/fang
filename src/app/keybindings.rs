//! Single source of truth for all key bindings.
//!
//! Both the footer hint bar and the Help panel (`h`) read from [`all_bindings()`].
//! When you add a new binding, add it here — the rest updates automatically.

/// One entry in the key-binding registry.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// Mode in which this binding is active (e.g. "Normal", "Search").
    pub mode: &'static str,
    /// Key label shown to the user (e.g. "j / ↓", "Ctrl+S").
    pub key: &'static str,
    /// Short description of the action.
    pub description: &'static str,
    /// If true, show this entry in the one-line footer hint bar.
    pub show_in_footer: bool,
}

impl KeyBinding {
    const fn new(mode: &'static str, key: &'static str, description: &'static str) -> Self {
        Self {
            mode,
            key,
            description,
            show_in_footer: false,
        }
    }
    const fn footer(mode: &'static str, key: &'static str, description: &'static str) -> Self {
        Self {
            mode,
            key,
            description,
            show_in_footer: true,
        }
    }
}

/// Returns every registered key binding.
/// This list drives both the footer hints and the Help panel.
pub fn all_bindings() -> Vec<KeyBinding> {
    vec![
        // ── Normal mode — navigation ──────────────────────────────────────
        KeyBinding::new("Normal", "j / ↓", "Move down"),
        KeyBinding::new("Normal", "k / ↑", "Move up"),
        KeyBinding::footer("Normal", "u / ←", "Parent directory"),
        KeyBinding::new("Normal", "l / → / Enter", "Enter directory"),
        KeyBinding::new("Normal", "Tab", "Cycle panel focus"),
        KeyBinding::new("Normal", "PgUp / PgDn", "Scroll preview"),
        // ── Normal mode — panels ──────────────────────────────────────────
        KeyBinding::new("Normal", "s", "Toggle sidebar"),
        KeyBinding::new("Normal", "p", "Toggle preview"),
        // ── Normal mode — features ────────────────────────────────────────
        KeyBinding::footer("Normal", "/", "Fuzzy search"),
        KeyBinding::footer("Normal", ":", "Run shell command"),
        KeyBinding::footer("Normal", ";", "Open in terminal split"),
        KeyBinding::footer("Normal", "m", "Makefile targets"),
        KeyBinding::footer("Normal", "g", "Git operations"),
        KeyBinding::footer("Normal", "o", "Open with default app"),
        KeyBinding::footer("Normal", "n", "New empty file"),
        KeyBinding::new("Normal", "N", "New file from clipboard"),
        KeyBinding::footer("Normal", "Ctrl+S", "Settings"),
        KeyBinding::footer("Normal", "h", "Help"),
        KeyBinding::footer("Normal", "q / Ctrl+C", "Quit"),
        // ── Search mode ───────────────────────────────────────────────────
        KeyBinding::footer("Search", "Enter", "Open selected"),
        KeyBinding::footer("Search", "Esc", "Cancel"),
        KeyBinding::footer("Search", "↑ / ↓", "Navigate results"),
        KeyBinding::new("Search", "Backspace", "Delete character"),
        // ── Make targets modal ────────────────────────────────────────────
        KeyBinding::footer("Make", "Enter", "Run target"),
        KeyBinding::footer("Make", "Esc", "Close"),
        KeyBinding::footer("Make", "j / k", "Navigate"),
        // ── Git operations modal ──────────────────────────────────────────
        KeyBinding::footer("Git", "Enter", "Run operation"),
        KeyBinding::footer("Git", "Esc", "Close"),
        KeyBinding::footer("Git", "j / k", "Navigate"),
        // ── Shell command input ───────────────────────────────────────────
        KeyBinding::footer("Command", "Enter", "Execute"),
        KeyBinding::footer("Command", "Esc", "Cancel"),
        // ── External split ────────────────────────────────────────────────
        KeyBinding::footer("Split", "Enter", "Open in split"),
        KeyBinding::footer("Split", "Esc", "Cancel"),
        // ── New file dialog ───────────────────────────────────────────────
        KeyBinding::footer("NewFile", "Enter", "Create"),
        KeyBinding::footer("NewFile", "Esc", "Cancel"),
        // ── Settings modal ────────────────────────────────────────────────
        KeyBinding::footer("Settings", "+ / →", "Increase value"),
        KeyBinding::footer("Settings", "- / ←", "Decrease value"),
        KeyBinding::footer("Settings", "j / k", "Navigate"),
        KeyBinding::footer("Settings", "Esc", "Save & close"),
        // ── Help panel ────────────────────────────────────────────────────
        KeyBinding::footer("Help", "h / Esc", "Close help"),
        KeyBinding::new("Help", "j / k", "Scroll"),
        KeyBinding::new("Help", "PgUp/PgDn", "Scroll fast"),
    ]
}

/// Returns only the bindings that should appear in the footer for `mode`.
pub fn footer_bindings(mode: &str) -> Vec<&'static KeyBinding> {
    // We leak the static vector once and return references into it.
    // This avoids re-allocating on every frame.
    use std::sync::OnceLock;
    static BINDINGS: OnceLock<Vec<KeyBinding>> = OnceLock::new();
    let all = BINDINGS.get_or_init(all_bindings);
    all.iter()
        .filter(|b| b.show_in_footer && b.mode == mode)
        .collect()
}
