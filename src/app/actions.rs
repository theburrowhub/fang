/// Actions that can be dispatched to update application state.
#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    MoveUp,
    MoveDown,
    Enter,
    Back,
    RunMakeTarget(String),
    Search(String),
    ClearSearch,
}
