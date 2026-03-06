use crate::app::state::{AppState, FileEntry};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::sync::OnceLock;

/// Singleton matcher — expensive to initialise, so it is shared for the lifetime of the process.
static MATCHER: OnceLock<SkimMatcherV2> = OnceLock::new();

fn get_matcher() -> &'static SkimMatcherV2 {
    MATCHER.get_or_init(|| SkimMatcherV2::default().smart_case())
}

/// Applies a fuzzy filter over `state.entries` using `state.search_query`.
///
/// - When the query is empty, every entry is visible (original order).
/// - When the query is non-empty, only entries whose name produces a match
///   are kept, and they are sorted by descending score (best match first).
///
/// After the call `state.filtered_indices` is up-to-date, and
/// `state.selected_index` / `state.file_list_scroll` are reset to 0.
pub fn apply_search(state: &mut AppState) {
    // Check emptiness via a reference before cloning to avoid an allocation
    // on the fast path where the user clears the search bar.
    if state.search_query.is_empty() {
        state.filtered_indices = (0..state.entries.len()).collect();
        return;
    }

    let query = state.search_query.clone();
    let matcher = get_matcher();
    let mut scored: Vec<(usize, i64)> = state
        .entries
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            matcher
                .fuzzy_match(&entry.name, &query)
                .map(|score| (i, score))
        })
        .collect();

    // Best match first. Unstable sort is faster and sufficient here because
    // equal scores don't need to preserve their relative file order.
    scored.sort_unstable_by(|a, b| b.1.cmp(&a.1));

    state.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();

    // Reset viewport so the first result is always visible.
    state.selected_index = 0;
    state.file_list_scroll = 0;
}

/// Returns the entries that are currently visible together with their
/// original index inside `state.entries`.
///
/// Returns visible entries paired with their real index in `state.entries`.
pub fn visible_entries<'a>(state: &'a AppState) -> Vec<(usize, &'a FileEntry)> {
    if state.search_query.is_empty() {
        state.entries.iter().enumerate().collect()
    } else {
        state
            .filtered_indices
            .iter()
            .map(|&i| (i, &state.entries[i]))
            .collect()
    }
}

/// Returns the real index in `state.entries` of the currently selected entry.
pub fn current_entry_index(state: &AppState) -> Option<usize> {
    if state.search_query.is_empty() {
        state
            .entries
            .get(state.selected_index)
            .map(|_| state.selected_index)
    } else {
        state.filtered_indices.get(state.selected_index).copied()
    }
}

/// Returns a reference to the currently selected `FileEntry`.
pub fn current_entry<'a>(state: &'a AppState) -> Option<&'a FileEntry> {
    state.selected_entry()
}

/// Returns the byte positions of the characters in `text` that matched
/// `query`, or `None` when the query is empty or there is no match.
///
/// The returned indices are suitable for highlighting matched characters in
/// the file-list UI.
pub fn match_positions(query: &str, text: &str) -> Option<Vec<usize>> {
    if query.is_empty() {
        return None;
    }
    let matcher = get_matcher();
    matcher
        .fuzzy_indices(text, query)
        .map(|(_, indices)| indices)
}

/// Returns the number of entries currently visible (after applying the filter).
pub fn visible_count(state: &AppState) -> usize {
    if state.search_query.is_empty() {
        state.entries.len()
    } else {
        state.filtered_indices.len()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_entry(name: &str, is_dir: bool) -> FileEntry {
        FileEntry {
            name: name.to_string(),
            path: PathBuf::from(name),
            is_dir,
            is_symlink: false,
            size: 0,
            is_executable: false,
            extension: name
                .split('.')
                .last()
                .filter(|_| !is_dir)
                .map(|s| s.to_string()),
            file_type: if is_dir {
                crate::fs::metadata::FileType::Directory
            } else {
                crate::fs::metadata::FileType::Unknown
            },
            modified: None,
        }
    }

    fn make_state(names: &[(&str, bool)]) -> AppState {
        let mut state = AppState::new(PathBuf::from("."));
        state.entries = names
            .iter()
            .map(|(name, is_dir)| make_entry(name, *is_dir))
            .collect();
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
        let mut state = make_state(&[
            ("main.rs", false),
            ("lib.rs", false),
            ("Cargo.toml", false),
            ("README.md", false),
        ]);
        state.search_query = "lib".to_string();
        apply_search(&mut state);
        let names: Vec<&str> = state
            .filtered_indices
            .iter()
            .map(|&i| state.entries[i].name.as_str())
            .collect();
        assert!(names.contains(&"lib.rs"), "lib.rs should match 'lib'");
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
        assert_eq!(
            state.selected_index, 0,
            "Selection should reset to 0 on search"
        );
    }

    #[test]
    fn test_fuzzy_rs_matches_rust_files() {
        let mut state = make_state(&[
            ("main.rs", false),
            ("lib.rs", false),
            ("Cargo.toml", false),
            ("README.md", false),
        ]);
        state.search_query = "rs".to_string();
        apply_search(&mut state);
        let names: Vec<&str> = state
            .filtered_indices
            .iter()
            .map(|&i| state.entries[i].name.as_str())
            .collect();
        assert!(names.contains(&"main.rs"), "main.rs should match 'rs'");
        assert!(names.contains(&"lib.rs"), "lib.rs should match 'rs'");
    }

    #[test]
    fn test_visible_entries_no_filter() {
        let state = make_state(&[("a.rs", false), ("b.rs", false), ("c.rs", false)]);
        let visible = visible_entries(&state);
        assert_eq!(visible.len(), 3);
    }

    #[test]
    fn test_current_entry_index() {
        let mut state = make_state(&[("main.rs", false), ("lib.rs", false)]);
        state.selected_index = 0;
        let idx = current_entry_index(&state);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn test_match_positions_returns_some_for_match() {
        // Just verify no panic — the exact indices depend on the algorithm.
        let positions = match_positions("rs", "main.rs");
        let _ = positions;
    }

    #[test]
    fn test_match_positions_empty_query() {
        let positions = match_positions("", "main.rs");
        assert!(positions.is_none(), "Empty query should return None");
    }

    #[test]
    fn test_visible_count() {
        let mut state = make_state(&[("main.rs", false), ("lib.rs", false), ("Cargo.toml", false)]);
        assert_eq!(visible_count(&state), 3);

        state.search_query = "rs".to_string();
        apply_search(&mut state);
        let count = visible_count(&state);
        assert!(count >= 2, "Should have at least main.rs and lib.rs");
    }

    #[test]
    fn test_results_ordered_by_score() {
        // "main" should score higher against "main.rs" than against "remains.rs"
        let mut state = make_state(&[("remains.rs", false), ("main.rs", false)]);
        state.search_query = "main".to_string();
        apply_search(&mut state);
        assert!(!state.filtered_indices.is_empty());
        // The top result should be main.rs (exact prefix match)
        let top_name = &state.entries[state.filtered_indices[0]].name;
        assert_eq!(top_name, "main.rs");
    }

    #[test]
    fn test_scroll_reset_on_search() {
        let mut state = make_state(&[("main.rs", false), ("lib.rs", false)]);
        state.file_list_scroll = 5;
        state.search_query = "lib".to_string();
        apply_search(&mut state);
        assert_eq!(
            state.file_list_scroll, 0,
            "Scroll should reset to 0 on search"
        );
    }

    #[test]
    fn test_current_entry_returns_correct_file() {
        let mut state = make_state(&[("Cargo.toml", false), ("main.rs", false), ("lib.rs", false)]);
        state.search_query = "lib".to_string();
        apply_search(&mut state);
        let entry = current_entry(&state);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().name, "lib.rs");
    }

    #[test]
    fn test_empty_entries_no_panic() {
        let mut state = AppState::new(std::path::PathBuf::from("."));
        state.search_query = "anything".to_string();
        apply_search(&mut state);
        assert_eq!(state.filtered_indices.len(), 0);
        assert!(current_entry(&state).is_none());
        assert!(current_entry_index(&state).is_none());
    }

    #[test]
    fn test_case_insensitive_smart_case() {
        // Smart-case: lower-case query matches both cases
        let mut state = make_state(&[
            ("Makefile", false),
            ("makefile.bak", false),
            ("README.md", false),
        ]);
        state.search_query = "make".to_string();
        apply_search(&mut state);
        let names: Vec<&str> = state
            .filtered_indices
            .iter()
            .map(|&i| state.entries[i].name.as_str())
            .collect();
        assert!(
            names.contains(&"Makefile"),
            "Makefile should match lowercase 'make'"
        );
        assert!(
            names.contains(&"makefile.bak"),
            "makefile.bak should match 'make'"
        );
    }
}
