//! Single source of truth for all key bindings.
//!
//! Both the footer hint bar and the Help panel (`h`) read from [`all_bindings()`].
//! When you add a new binding, add it here — the rest updates automatically.
//!
//! Each entry has two descriptions:
//! - `description` — full text shown in the Help panel
//! - `short`       — compact label used in the one-line footer

/// One entry in the key-binding registry.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// Mode in which this binding is active (e.g. "Normal", "Search").
    pub mode: &'static str,
    /// Key label shown to the user (e.g. "u / ←", "Ctrl+S").
    pub key: &'static str,
    /// Full description for the Help panel.
    pub description: &'static str,
    /// Compact label for the footer hint bar (one or two words).
    pub short: &'static str,
    /// If true, show this entry in the one-line footer hint bar.
    pub show_in_footer: bool,
}

impl KeyBinding {
    /// Entry shown only in the Help panel, not in the footer.
    const fn help_only(mode: &'static str, key: &'static str, description: &'static str) -> Self {
        Self {
            mode,
            key,
            description,
            short: description,
            show_in_footer: false,
        }
    }

    /// Entry shown in both the footer and the Help panel.
    const fn footer(
        mode: &'static str,
        key: &'static str,
        description: &'static str,
        short: &'static str,
    ) -> Self {
        Self {
            mode,
            key,
            description,
            short,
            show_in_footer: true,
        }
    }
}

/// Returns every registered key binding.
pub fn all_bindings() -> Vec<KeyBinding> {
    vec![
        // ── Normal mode — navigation ──────────────────────────────────────
        KeyBinding::help_only("Normal", "j / ↓", "Move down"),
        KeyBinding::help_only("Normal", "k / ↑", "Move up"),
        KeyBinding::help_only("Normal", "u / ←", "Parent directory"),
        KeyBinding::help_only("Normal", "l / → / Enter", "Enter directory"),
        KeyBinding::footer("Normal", "Tab", "Focus next panel (→)", "Tab →"),
        KeyBinding::footer("Normal", "Shift+Tab", "Focus previous panel (←)", "Tab ←"),
        KeyBinding::help_only("Normal", "PgUp / PgDn", "Scroll preview"),
        // ── Normal mode — footer (essentials only) ────────────────────────
        KeyBinding::footer("Normal", "Ctrl+K", "Command palette", "Palette"),
        KeyBinding::footer("Normal", "/", "Fuzzy search", "Search"),
        KeyBinding::footer("Normal", ":", "Run shell command", "Run"),
        KeyBinding::footer("Normal", ";", "Open in terminal split", "Split"),
        KeyBinding::footer("Normal", "h", "Help (this panel)", "Help"),
        KeyBinding::footer("Normal", "q", "Quit", "Quit"),
        // ── Normal mode — help-only (shown in Help panel via h) ──────────
        KeyBinding::help_only("Normal", "p", "Toggle preview"),
        KeyBinding::help_only("Normal", "d", "Toggle git diff view"),
        KeyBinding::help_only("Normal", "g", "Git operations"),
        KeyBinding::help_only("Normal", "m", "Makefile targets"),
        KeyBinding::help_only("Normal", "Esc", "Cancel running make target"),
        KeyBinding::help_only("Normal", "c", "Copy relative path to clipboard"),
        KeyBinding::help_only("Normal", "C", "Copy absolute path to clipboard"),
        KeyBinding::help_only("Normal", "o", "Open with default app"),
        KeyBinding::help_only("Normal", "n", "New empty file"),
        KeyBinding::help_only("Normal", "N", "New file from clipboard"),
        KeyBinding::help_only("Normal", "Ctrl+S", "Settings"),
        KeyBinding::help_only("Normal", "Ctrl+C", "Quit"),
        // ── Search mode ───────────────────────────────────────────────────
        KeyBinding::footer("Search", "Enter", "Open selected", "Select"),
        KeyBinding::footer("Search", "Esc", "Cancel search", "Cancel"),
        KeyBinding::footer("Search", "↑ / ↓", "Navigate results", "Nav"),
        KeyBinding::help_only("Search", "Backspace", "Delete character"),
        // ── Make targets modal ────────────────────────────────────────────
        KeyBinding::footer("Make", "Enter", "Run selected target", "Run"),
        KeyBinding::footer("Make", "Esc", "Close modal", "Close"),
        KeyBinding::footer("Make", "j / k", "Navigate targets", "Nav"),
        // ── Git operations modal ──────────────────────────────────────────
        KeyBinding::footer("Git", "Enter", "Run selected operation", "Run"),
        KeyBinding::footer("Git", "Esc", "Close modal", "Close"),
        KeyBinding::footer("Git", "j / k", "Navigate operations", "Nav"),
        // ── Shell command input ───────────────────────────────────────────
        KeyBinding::footer("Command", "Enter", "Execute command", "Run"),
        KeyBinding::footer("Command", "Esc", "Cancel", "Cancel"),
        // ── External split ────────────────────────────────────────────────
        KeyBinding::footer("Split", "Enter", "Open command in split", "Open"),
        KeyBinding::help_only(
            "Split",
            "Ctrl+P",
            "Open command in tmux popup (fallback: split)",
        ),
        KeyBinding::footer("Split", "Esc", "Cancel", "Cancel"),
        // ── New file dialog ───────────────────────────────────────────────
        KeyBinding::footer("NewFile", "Enter", "Create file", "Create"),
        KeyBinding::footer("NewFile", "Esc", "Cancel", "Cancel"),
        // ── Settings modal ────────────────────────────────────────────────
        KeyBinding::footer("Settings", "+ / →", "Increase value", "+"),
        KeyBinding::footer("Settings", "- / ←", "Decrease value", "−"),
        KeyBinding::footer("Settings", "j / k", "Navigate settings", "Nav"),
        KeyBinding::footer("Settings", "Esc", "Save and close", "Save"),
        // ── Help panel ────────────────────────────────────────────────────
        KeyBinding::footer("Help", "h / Esc", "Close help panel", "Close"),
        KeyBinding::help_only("Help", "j / k", "Scroll line by line"),
        KeyBinding::help_only("Help", "PgUp/PgDn", "Scroll page"),
        // ── AI — Normal mode (help-only; accessible via Palette) ─────────
        KeyBinding::help_only("Normal", "a", "Toggle AI panel"),
        KeyBinding::help_only("Normal", "i", "AI prompt"),
        KeyBinding::help_only("Normal", "I", "Change AI provider"),
        KeyBinding::help_only("Normal", "Ctrl+R", "Reset AI session"),
        // ── AI chat panel focused (pseudo-mode) ──────────────────────────
        KeyBinding::footer("AiChat", "j/k", "Scroll AI chat", "Scroll"),
        KeyBinding::footer("AiChat", "i", "Ask AI", "Ask AI"),
        KeyBinding::footer("AiChat", "I", "Change provider", "Provider"),
        KeyBinding::footer("AiChat", "Ctrl+R", "Reset session", "Reset"),
        KeyBinding::footer("AiChat", "a", "Hide AI panel", "Hide"),
        KeyBinding::footer("AiChat", "Tab", "Focus next panel", "Panel"),
        KeyBinding::footer("AiChat", "q", "Quit", "Quit"),
        // ── AI provider selection modal ───────────────────────────────────
        KeyBinding::footer("AiProvider", "Enter", "Select provider", "Select"),
        KeyBinding::footer("AiProvider", "Esc", "Cancel", "Cancel"),
        KeyBinding::footer("AiProvider", "j/k", "Navigate", "Nav"),
    ]
}

/// Returns the footer bindings for `mode`, keeping references into a
/// `OnceLock`-cached `Vec` so no allocation happens on every frame.
pub fn footer_bindings(mode: &str) -> Vec<&'static KeyBinding> {
    use std::sync::OnceLock;
    static BINDINGS: OnceLock<Vec<KeyBinding>> = OnceLock::new();
    let all = BINDINGS.get_or_init(all_bindings);
    all.iter()
        .filter(|b| b.show_in_footer && b.mode == mode)
        .collect()
}
