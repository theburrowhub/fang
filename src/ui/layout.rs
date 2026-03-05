/// Layout computation for the three-panel UI.
/// Stub — to be implemented in a later unit (U2).
use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Split the terminal `area` into three horizontal panels:
/// sidebar | file-list | preview
pub fn three_panel_layout(area: Rect) -> [Rect; 3] {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(40),
            Constraint::Percentage(40),
        ])
        .split(area);
    [chunks[0], chunks[1], chunks[2]]
}
