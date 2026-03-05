use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// A wrapper around the skim fuzzy matcher for filtering file entries.
pub struct FuzzySearch {
    matcher: SkimMatcherV2,
}

impl FuzzySearch {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Returns the score for matching `query` against `candidate`.
    /// Returns None if no match.
    pub fn score(&self, query: &str, candidate: &str) -> Option<i64> {
        self.matcher.fuzzy_match(candidate, query)
    }

    /// Filters and sorts a list of strings by fuzzy match score against `query`.
    /// Returns only items that match, sorted from best to worst.
    pub fn filter_and_sort<'a>(&self, query: &str, items: &'a [String]) -> Vec<&'a str> {
        let mut scored: Vec<(i64, &'a str)> = items
            .iter()
            .filter_map(|item| {
                self.score(query, item).map(|s| (s, item.as_str()))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, s)| s).collect()
    }
}

impl Default for FuzzySearch {
    fn default() -> Self {
        Self::new()
    }
}
