use crate::app::state::AppState;

pub fn apply_search(state: &mut AppState) {
    state.filtered_indices = (0..state.entries.len()).collect();
}
