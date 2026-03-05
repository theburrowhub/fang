use std::sync::OnceLock;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use crate::app::state::{AppState, FileEntry};

static MATCHER: OnceLock<SkimMatcherV2> = OnceLock::new();
fn get_matcher() -> &'static SkimMatcherV2 { MATCHER.get_or_init(|| SkimMatcherV2::default().smart_case()) }

pub fn apply_search(state: &mut AppState) {
    if state.search_query.is_empty() {
        state.filtered_indices = (0..state.entries.len()).collect();
        return;
    }
    let query = state.search_query.clone();
    let matcher = get_matcher();
    let mut scored: Vec<(usize, i64)> = state.entries.iter().enumerate()
        .filter_map(|(i, entry)| matcher.fuzzy_match(&entry.name, &query).map(|score| (i, score)))
        .collect();
    scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    state.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();
    state.selected_index = 0;
    state.file_list_scroll = 0;
}

pub fn current_entry(state: &AppState) -> Option<&FileEntry> {
    state.visible_entries().into_iter().nth(state.selected_index)
}

pub fn match_positions(query: &str, text: &str) -> Option<Vec<usize>> {
    if query.is_empty() { return None; }
    get_matcher().fuzzy_indices(text, query).map(|(_, indices)| indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    fn make_entry(name: &str, is_dir: bool) -> FileEntry {
        FileEntry {
            name: name.to_string(), path: PathBuf::from(name), is_dir, is_symlink: false,
            size: 0, is_executable: false,
            extension: name.split('.').last().filter(|_| !is_dir).map(|s| s.to_string()),
            file_type: crate::fs::metadata::FileType::Unknown, modified: None,
        }
    }
    fn make_state(names: &[(&str, bool)]) -> AppState {
        let mut state = AppState::new(PathBuf::from("."));
        state.entries = names.iter().map(|(name, is_dir)| make_entry(name, *is_dir)).collect();
        state.filtered_indices = (0..state.entries.len()).collect();
        state
    }
    #[test]
    fn test_empty_query_shows_all() {
        let mut state = make_state(&[("main.rs", false), ("lib.rs", false), ("Cargo.toml", false)]);
        state.search_query = String::new();
        apply_search(&mut state);
        assert_eq!(state.filtered_indices.len(), 3);
    }
    #[test]
    fn test_query_filters_entries() {
        let mut state = make_state(&[("main.rs", false), ("lib.rs", false), ("Cargo.toml", false)]);
        state.search_query = "lib".to_string();
        apply_search(&mut state);
        let names: Vec<&str> = state.filtered_indices.iter().map(|&i| state.entries[i].name.as_str()).collect();
        assert!(names.contains(&"lib.rs"));
    }
    #[test]
    fn test_no_match_returns_empty() {
        let mut state = make_state(&[("main.rs", false), ("lib.rs", false)]);
        state.search_query = "xyzzy_impossible".to_string();
        apply_search(&mut state);
        assert_eq!(state.filtered_indices.len(), 0);
    }
    #[test]
    fn test_selection_reset_on_search() {
        let mut state = make_state(&[("main.rs", false), ("lib.rs", false)]);
        state.selected_index = 1;
        state.search_query = "main".to_string();
        apply_search(&mut state);
        assert_eq!(state.selected_index, 0);
    }
    #[test]
    fn test_results_ordered_by_score() {
        let mut state = make_state(&[("remains.rs", false), ("main.rs", false)]);
        state.search_query = "main".to_string();
        apply_search(&mut state);
        assert!(!state.filtered_indices.is_empty());
        assert_eq!(state.entries[state.filtered_indices[0]].name, "main.rs");
    }
    #[test]
    fn test_scroll_reset_on_search() {
        let mut state = make_state(&[("main.rs", false), ("lib.rs", false)]);
        state.file_list_scroll = 5;
        state.search_query = "lib".to_string();
        apply_search(&mut state);
        assert_eq!(state.file_list_scroll, 0);
    }
    #[test]
    fn test_empty_entries_no_panic() {
        let mut state = AppState::new(std::path::PathBuf::from("."));
        state.search_query = "anything".to_string();
        apply_search(&mut state);
        assert_eq!(state.filtered_indices.len(), 0);
    }
    #[test]
    fn test_case_insensitive_smart_case() {
        let mut state = make_state(&[("Makefile", false), ("makefile.bak", false), ("README.md", false)]);
        state.search_query = "make".to_string();
        apply_search(&mut state);
        let names: Vec<&str> = state.filtered_indices.iter().map(|&i| state.entries[i].name.as_str()).collect();
        assert!(names.contains(&"Makefile"));
        assert!(names.contains(&"makefile.bak"));
    }
}
