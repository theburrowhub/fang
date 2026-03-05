use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use crate::app::state::FileEntry;

/// Filter entries using fuzzy matching against the query.
/// Returns indices of matching entries, sorted by score descending.
pub fn filter(entries: &[FileEntry], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..entries.len()).collect();
    }

    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(usize, i64)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            matcher
                .fuzzy_match(&entry.name, query)
                .map(|score| (i, score))
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().map(|(i, _)| i).collect()
}
