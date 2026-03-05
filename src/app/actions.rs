/// High-level actions that can be dispatched from key-bindings.
/// Stub — to be implemented in a later unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    JumpToTop,
    JumpToBottom,
    Enter,
    GoUp,
    GoBack,
    GoForward,
    ToggleHidden,
    ToggleHelp,
    StartSearch,
    StopSearch,
    CycleFocus,
    Quit,
}
