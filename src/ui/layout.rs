/// Layout helpers for the TUI.
/// Stub implementation — full TUI rendering to be implemented in a later unit.

pub struct LayoutConfig {
    pub sidebar_width: u16,
    pub preview_width: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            sidebar_width: 30,
            preview_width: 70,
        }
    }
}
