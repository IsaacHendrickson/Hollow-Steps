//! Window-free filtering and selection for editor lists.

/// Return source indices, with prefixes before other substrings and each group stable.
pub fn filter_names(names: &[impl AsRef<str>], query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    let mut prefixes = Vec::new();
    let mut substrings = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let name = name.as_ref().to_lowercase();
        if name.starts_with(&query) {
            prefixes.push(index);
        } else if name.contains(&query) {
            substrings.push(index);
        }
    }
    prefixes.extend(substrings);
    prefixes
}

#[derive(Default)]
pub struct SearchList {
    pub query: String,
    /// An index into the source list, never into the filtered results.
    pub selected: Option<usize>,
}

impl SearchList {
    pub fn matches(&mut self, names: &[impl AsRef<str>]) -> Vec<usize> {
        let matches = filter_names(names, &self.query);
        self.selected = self.selected.filter(|index| matches.contains(index));
        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_or_whitespace_query_returns_everything_in_order() {
        for query in ["", " \t\n "] {
            assert_eq!(filter_names(&["Beta", "Alpha", ""], query), [0, 1, 2]);
            assert!(filter_names(&[] as &[&str], query).is_empty());
        }
    }

    #[test]
    fn query_is_trimmed_and_case_insensitive() {
        assert_eq!(
            filter_names(&["Patrol", "CHASE", "patrol"], " PaTrOl "),
            [0, 2]
        );
    }

    #[test]
    fn prefixes_precede_substrings_with_original_order_in_both_groups() {
        let names = [
            "slow patrol",
            "Patrol B",
            "fast patrol",
            "patrol A",
            "chase",
        ];
        assert_eq!(filter_names(&names, "patrol"), [1, 3, 0, 2]);
    }

    #[test]
    fn no_matches() {
        assert!(filter_names(&["patrol", "chase"], "idle").is_empty());
    }

    #[test]
    fn unicode_lowercase_and_substrings() {
        assert_eq!(
            filter_names(&["小雪", "ÉCLAIR", "雪", "雪山", "école"], " 雪 "),
            [2, 3, 0]
        );
        assert_eq!(
            filter_names(&["ÉCLAIR", "un éclair", "école"], "é"),
            [0, 2, 1]
        );
        assert_eq!(filter_names(&["İstanbul", "other"], "İS"), [0]);
    }

    #[test]
    fn selection_survives_filter_reordering_and_clears_only_when_hidden() {
        let names = ["slow patrol", "Patrol B", "chase", "patrol A"];
        let mut list = SearchList {
            selected: Some(0),
            ..SearchList::default()
        };
        assert_eq!(list.matches(&names), [0, 1, 2, 3]);
        list.query = "PATROL".into();
        assert_eq!(list.matches(&names), [1, 3, 0]);
        assert_eq!(list.selected, Some(0));
        list.query = " slow ".into();
        assert_eq!(list.matches(&names), [0]);
        assert_eq!(list.selected, Some(0));
        list.query = "chase".into();
        assert_eq!(list.matches(&names), [2]);
        assert_eq!(list.selected, None);
        list.query.clear();
        list.matches(&names);
        assert_eq!(list.selected, None);
    }

    #[test]
    fn absent_or_removed_selection_does_not_select_another_item() {
        let mut list = SearchList::default();
        list.matches(&["idle"]);
        assert_eq!(list.selected, None);
        list.selected = Some(10);
        list.matches(&["idle"]);
        assert_eq!(list.selected, None);
        list.selected = Some(0);
        list.matches(&[] as &[&str]);
        assert_eq!(list.selected, None);
    }
}
