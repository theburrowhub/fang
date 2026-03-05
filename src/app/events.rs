/// Terminal events that the application reacts to.
/// Stub — to be implemented in a later unit.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// A crossterm key event.
    Key(crossterm::event::KeyEvent),
    /// Terminal was resized.
    Resize(u16, u16),
    /// Internal tick (for animations, status-bar clearing, etc.).
    Tick,
    /// Application should quit.
    Quit,
}
