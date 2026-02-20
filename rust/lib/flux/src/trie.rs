use std::collections::HashMap;
use std::sync::RwLock;

/// A thread-safe Trie for MQTT-style topic pattern matching.
///
/// Supports wildcards:
/// - `+` matches exactly one topic level
/// - `#` matches any number of remaining topic levels (must be last segment)
///
/// Patterns and topics use `/` as the level separator.
///
/// # Examples
///
/// ```ignore
/// let trie = Trie::new();
/// trie.insert("auth/state", 1);
/// trie.insert("auth/+", 2);
/// trie.insert("#", 3);
///
/// // "auth/state" matches exact, single-level wildcard, and root wildcard
/// let results = trie.match_topic("auth/state"); // [1, 2, 3]
/// ```
pub struct Trie<T> {
    root: RwLock<TrieNode<T>>,
}

struct TrieNode<T> {
    /// Exact-match children, keyed by segment string.
    children: HashMap<String, TrieNode<T>>,
    /// `+` wildcard child — matches exactly one level.
    single: Option<Box<TrieNode<T>>>,
    /// `#` wildcard child — matches any remaining levels.
    multi: Option<Box<TrieNode<T>>>,
    /// Values stored at this node (when pattern terminates here).
    values: Vec<T>,
}

impl<T> Default for TrieNode<T> {
    fn default() -> Self {
        Self {
            children: HashMap::new(),
            single: None,
            multi: None,
            values: Vec::new(),
        }
    }
}

impl<T: Clone> Trie<T> {
    /// Create a new empty Trie.
    pub fn new() -> Self {
        Self {
            root: RwLock::new(TrieNode::default()),
        }
    }

    /// Insert a value at the given pattern.
    ///
    /// Pattern examples: `"auth/state"`, `"auth/#"`, `"+/state"`, `"#"`.
    pub fn insert(&self, pattern: &str, value: T) {
        let mut root = self.root.write().unwrap();
        root.insert(pattern, value);
    }

    /// Return all values whose patterns match the given concrete topic path.
    ///
    /// For example, topic `"auth/state"` matches patterns:
    /// - `"auth/state"` (exact)
    /// - `"auth/+"` (single-level wildcard)
    /// - `"auth/#"` (multi-level wildcard)
    /// - `"#"` (match all)
    pub fn match_topic(&self, topic: &str) -> Vec<T> {
        let root = self.root.read().unwrap();
        let mut results = Vec::new();
        root.collect_matches(topic, &mut results);
        results
    }

    /// Remove values matching the predicate from the given pattern.
    ///
    /// Returns `true` if any values were removed.
    pub fn remove<F>(&self, pattern: &str, predicate: F) -> bool
    where
        F: Fn(&T) -> bool,
    {
        let mut root = self.root.write().unwrap();
        root.remove(pattern, &predicate)
    }

    /// Check if any values exist at the given pattern (exact pattern, not matching).
    pub fn has_pattern(&self, pattern: &str) -> bool {
        let root = self.root.read().unwrap();
        root.has_pattern(pattern)
    }
}

impl<T> Default for Trie<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> TrieNode<T> {
    fn insert(&mut self, pattern: &str, value: T) {
        if pattern.is_empty() {
            self.values.push(value);
            return;
        }

        let (first, rest) = split_first(pattern);

        match first {
            "+" => {
                let child = self
                    .single
                    .get_or_insert_with(|| Box::new(TrieNode::default()));
                child.insert(rest, value);
            }
            "#" => {
                // `#` must be the last segment — store value on the multi child.
                let child = self
                    .multi
                    .get_or_insert_with(|| Box::new(TrieNode::default()));
                child.values.push(value);
            }
            segment => {
                let child = self
                    .children
                    .entry(segment.to_string())
                    .or_insert_with(TrieNode::default);
                child.insert(rest, value);
            }
        }
    }

    fn collect_matches(&self, topic: &str, results: &mut Vec<T>) {
        if topic.is_empty() {
            // Pattern terminates here — collect exact values.
            results.extend(self.values.iter().cloned());
            // `#` at this level also matches zero remaining levels.
            if let Some(ref multi) = self.multi {
                results.extend(multi.values.iter().cloned());
            }
            return;
        }

        let (first, rest) = split_first(topic);

        // Exact segment match.
        if let Some(child) = self.children.get(first) {
            child.collect_matches(rest, results);
        }

        // Single-level wildcard `+` — matches this one segment.
        if let Some(ref single) = self.single {
            single.collect_matches(rest, results);
        }

        // Multi-level wildcard `#` — matches everything from here on.
        if let Some(ref multi) = self.multi {
            results.extend(multi.values.iter().cloned());
        }
    }

    fn remove<F>(&mut self, pattern: &str, predicate: &F) -> bool
    where
        F: Fn(&T) -> bool,
    {
        if pattern.is_empty() {
            let before = self.values.len();
            self.values.retain(|v| !predicate(v));
            return self.values.len() < before;
        }

        let (first, rest) = split_first(pattern);

        match first {
            "+" => {
                if let Some(ref mut child) = self.single {
                    return child.remove(rest, predicate);
                }
            }
            "#" => {
                if let Some(ref mut child) = self.multi {
                    let before = child.values.len();
                    child.values.retain(|v| !predicate(v));
                    return child.values.len() < before;
                }
            }
            segment => {
                if let Some(child) = self.children.get_mut(segment) {
                    return child.remove(rest, predicate);
                }
            }
        }

        false
    }

    fn has_pattern(&self, pattern: &str) -> bool {
        if pattern.is_empty() {
            return !self.values.is_empty();
        }

        let (first, rest) = split_first(pattern);

        match first {
            "+" => self
                .single
                .as_ref()
                .map_or(false, |child| child.has_pattern(rest)),
            "#" => self
                .multi
                .as_ref()
                .map_or(false, |child| !child.values.is_empty()),
            segment => self
                .children
                .get(segment)
                .map_or(false, |child| child.has_pattern(rest)),
        }
    }
}

/// Split a path into (first_segment, rest).
///
/// `"auth/state"` -> `("auth", "state")`
/// `"auth"` -> `("auth", "")`
/// `""` -> `("", "")`
fn split_first(path: &str) -> (&str, &str) {
    match path.find('/') {
        Some(idx) => (&path[..idx], &path[idx + 1..]),
        None => (path, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Exact match
    // ========================================================================

    #[test]
    fn exact_match_single_segment() {
        let trie = Trie::new();
        trie.insert("auth", 1);

        assert_eq!(trie.match_topic("auth"), vec![1]);
        assert!(trie.match_topic("home").is_empty());
    }

    #[test]
    fn exact_match_two_segments() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);
        trie.insert("auth/terms", 2);

        assert_eq!(trie.match_topic("auth/state"), vec![1]);
        assert_eq!(trie.match_topic("auth/terms"), vec![2]);
        assert!(trie.match_topic("auth/code").is_empty());
    }

    #[test]
    fn exact_match_three_segments() {
        let trie = Trie::new();
        trie.insert("home/devices/items", 1);

        assert_eq!(trie.match_topic("home/devices/items"), vec![1]);
        assert!(trie.match_topic("home/devices").is_empty());
        assert!(trie.match_topic("home/devices/items/123").is_empty());
    }

    #[test]
    fn exact_match_deep_path() {
        let trie = Trie::new();
        trie.insert("a/b/c/d/e/f", 1);

        assert_eq!(trie.match_topic("a/b/c/d/e/f"), vec![1]);
        assert!(trie.match_topic("a/b/c/d/e").is_empty());
        assert!(trie.match_topic("a/b/c/d/e/f/g").is_empty());
    }

    #[test]
    fn multiple_values_same_pattern() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);
        trie.insert("auth/state", 2);

        let results = trie.match_topic("auth/state");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
    }

    // ========================================================================
    // Single-level wildcard (+)
    // ========================================================================

    #[test]
    fn single_wildcard_matches_one_level() {
        let trie = Trie::new();
        trie.insert("auth/+", 10);

        assert_eq!(trie.match_topic("auth/state"), vec![10]);
        assert_eq!(trie.match_topic("auth/terms"), vec![10]);
        assert_eq!(trie.match_topic("auth/code"), vec![10]);
    }

    #[test]
    fn single_wildcard_does_not_match_zero_levels() {
        let trie = Trie::new();
        trie.insert("auth/+", 10);

        // "auth" alone has zero levels after "auth", so + shouldn't match.
        assert!(trie.match_topic("auth").is_empty());
    }

    #[test]
    fn single_wildcard_does_not_match_multiple_levels() {
        let trie = Trie::new();
        trie.insert("auth/+", 10);

        assert!(trie.match_topic("auth/a/b").is_empty());
        assert!(trie.match_topic("auth/a/b/c").is_empty());
    }

    #[test]
    fn single_wildcard_does_not_match_different_prefix() {
        let trie = Trie::new();
        trie.insert("auth/+", 10);

        assert!(trie.match_topic("home/state").is_empty());
        assert!(trie.match_topic("app/route").is_empty());
    }

    #[test]
    fn single_wildcard_in_middle() {
        let trie = Trie::new();
        trie.insert("home/+/items", 10);

        assert_eq!(trie.match_topic("home/devices/items"), vec![10]);
        assert_eq!(trie.match_topic("home/chats/items"), vec![10]);
        assert!(trie.match_topic("home/devices/other").is_empty());
        assert!(trie.match_topic("home/items").is_empty());
    }

    #[test]
    fn single_wildcard_at_start() {
        let trie = Trie::new();
        trie.insert("+/state", 10);

        assert_eq!(trie.match_topic("auth/state"), vec![10]);
        assert_eq!(trie.match_topic("home/state"), vec![10]);
        assert!(trie.match_topic("auth/terms").is_empty());
    }

    #[test]
    fn multiple_single_wildcards() {
        let trie = Trie::new();
        trie.insert("+/+", 10);

        assert_eq!(trie.match_topic("auth/state"), vec![10]);
        assert_eq!(trie.match_topic("home/devices"), vec![10]);
        assert!(trie.match_topic("auth").is_empty());
        assert!(trie.match_topic("a/b/c").is_empty());
    }

    // ========================================================================
    // Multi-level wildcard (#)
    // ========================================================================

    #[test]
    fn multi_wildcard_matches_one_level() {
        let trie = Trie::new();
        trie.insert("auth/#", 20);

        assert_eq!(trie.match_topic("auth/state"), vec![20]);
        assert_eq!(trie.match_topic("auth/terms"), vec![20]);
    }

    #[test]
    fn multi_wildcard_matches_multiple_levels() {
        let trie = Trie::new();
        trie.insert("auth/#", 20);

        assert_eq!(trie.match_topic("auth/a/b"), vec![20]);
        assert_eq!(trie.match_topic("auth/a/b/c/d"), vec![20]);
    }

    #[test]
    fn multi_wildcard_matches_zero_remaining_levels() {
        let trie = Trie::new();
        trie.insert("auth/#", 20);

        // "auth" alone — `#` matches zero remaining levels.
        assert_eq!(trie.match_topic("auth"), vec![20]);
    }

    #[test]
    fn multi_wildcard_does_not_match_different_prefix() {
        let trie = Trie::new();
        trie.insert("auth/#", 20);

        assert!(trie.match_topic("home/state").is_empty());
        assert!(trie.match_topic("app/route").is_empty());
    }

    #[test]
    fn root_wildcard_matches_everything() {
        let trie = Trie::new();
        trie.insert("#", 99);

        assert_eq!(trie.match_topic("auth/state"), vec![99]);
        assert_eq!(trie.match_topic("app/route"), vec![99]);
        assert_eq!(trie.match_topic("a/b/c/d"), vec![99]);
        assert_eq!(trie.match_topic("x"), vec![99]);
    }

    #[test]
    fn root_wildcard_matches_single_segment() {
        let trie = Trie::new();
        trie.insert("#", 99);

        assert_eq!(trie.match_topic("anything"), vec![99]);
    }

    // ========================================================================
    // Combined patterns
    // ========================================================================

    #[test]
    fn exact_plus_single_wildcard() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);
        trie.insert("auth/+", 2);

        let mut results = trie.match_topic("auth/state");
        results.sort();
        assert_eq!(results, vec![1, 2]);
    }

    #[test]
    fn exact_plus_multi_wildcard() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);
        trie.insert("auth/#", 3);

        let mut results = trie.match_topic("auth/state");
        results.sort();
        assert_eq!(results, vec![1, 3]);
    }

    #[test]
    fn all_wildcard_types_combined() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);
        trie.insert("auth/+", 2);
        trie.insert("auth/#", 3);
        trie.insert("#", 4);

        let mut results = trie.match_topic("auth/state");
        results.sort();
        assert_eq!(results, vec![1, 2, 3, 4]);
    }

    #[test]
    fn wildcard_does_not_cross_match() {
        let trie = Trie::new();
        trie.insert("auth/#", 1);
        trie.insert("home/#", 2);

        assert_eq!(trie.match_topic("auth/state"), vec![1]);
        assert_eq!(trie.match_topic("home/devices"), vec![2]);
    }

    #[test]
    fn single_and_multi_wildcard_together() {
        let trie = Trie::new();
        trie.insert("+/#", 1);

        assert_eq!(trie.match_topic("auth/state"), vec![1]);
        assert_eq!(trie.match_topic("home/devices/items"), vec![1]);
        assert_eq!(trie.match_topic("x"), vec![1]); // + matches "x", # matches zero
    }

    // ========================================================================
    // Edge cases
    // ========================================================================

    #[test]
    fn empty_topic_no_match() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);

        assert!(trie.match_topic("").is_empty());
    }

    #[test]
    fn empty_trie_no_match() {
        let trie: Trie<i32> = Trie::new();

        assert!(trie.match_topic("auth/state").is_empty());
        assert!(trie.match_topic("").is_empty());
    }

    #[test]
    fn topic_with_many_segments() {
        let trie = Trie::new();
        trie.insert("a/b/c/d/e", 1);
        trie.insert("a/#", 2);
        trie.insert("a/b/+/d/e", 3);

        let mut results = trie.match_topic("a/b/c/d/e");
        results.sort();
        assert_eq!(results, vec![1, 2, 3]);
    }

    #[test]
    fn similar_prefixes_do_not_interfere() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);
        trie.insert("authorization/state", 2);

        assert_eq!(trie.match_topic("auth/state"), vec![1]);
        assert_eq!(trie.match_topic("authorization/state"), vec![2]);
    }

    // ========================================================================
    // Remove
    // ========================================================================

    #[test]
    fn remove_exact_match_by_predicate() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);
        trie.insert("auth/state", 2);

        assert!(trie.remove("auth/state", |v| *v == 1));
        assert_eq!(trie.match_topic("auth/state"), vec![2]);
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);

        assert!(!trie.remove("auth/state", |v| *v == 99));
        assert_eq!(trie.match_topic("auth/state"), vec![1]);
    }

    #[test]
    fn remove_from_nonexistent_pattern() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);

        assert!(!trie.remove("home/state", |_| true));
    }

    #[test]
    fn remove_from_single_wildcard() {
        let trie = Trie::new();
        trie.insert("auth/+", 10);
        trie.insert("auth/+", 20);

        assert!(trie.remove("auth/+", |v| *v == 10));
        assert_eq!(trie.match_topic("auth/state"), vec![20]);
    }

    #[test]
    fn remove_from_multi_wildcard() {
        let trie = Trie::new();
        trie.insert("#", 10);
        trie.insert("#", 20);

        assert!(trie.remove("#", |v| *v == 10));
        assert_eq!(trie.match_topic("anything"), vec![20]);
    }

    #[test]
    fn remove_from_nested_multi_wildcard() {
        let trie = Trie::new();
        trie.insert("auth/#", 10);
        trie.insert("auth/#", 20);

        assert!(trie.remove("auth/#", |v| *v == 10));
        assert_eq!(trie.match_topic("auth/state"), vec![20]);
    }

    #[test]
    fn remove_all_values() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);

        assert!(trie.remove("auth/state", |_| true));
        assert!(trie.match_topic("auth/state").is_empty());
    }

    // ========================================================================
    // has_pattern
    // ========================================================================

    #[test]
    fn has_pattern_exact() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);

        assert!(trie.has_pattern("auth/state"));
        assert!(!trie.has_pattern("auth/terms"));
        assert!(!trie.has_pattern("auth"));
    }

    #[test]
    fn has_pattern_wildcard() {
        let trie = Trie::new();
        trie.insert("auth/+", 1);
        trie.insert("home/#", 2);

        assert!(trie.has_pattern("auth/+"));
        assert!(trie.has_pattern("home/#"));
        assert!(!trie.has_pattern("auth/#"));
        assert!(!trie.has_pattern("home/+"));
    }

    #[test]
    fn has_pattern_root_wildcard() {
        let trie = Trie::new();
        trie.insert("#", 1);

        assert!(trie.has_pattern("#"));
        assert!(!trie.has_pattern("+"));
    }

    #[test]
    fn has_pattern_after_remove_all() {
        let trie = Trie::new();
        trie.insert("auth/state", 1);
        trie.remove("auth/state", |_| true);

        assert!(!trie.has_pattern("auth/state"));
    }

    // ========================================================================
    // Thread safety
    // ========================================================================

    #[test]
    fn concurrent_insert_and_match() {
        use std::sync::Arc;
        use std::thread;

        let trie = Arc::new(Trie::new());
        let mut handles = vec![];

        // Spawn writers.
        for i in 0..10 {
            let trie = Arc::clone(&trie);
            handles.push(thread::spawn(move || {
                let path = format!("topic/{}", i);
                trie.insert(&path, i);
            }));
        }

        // Wait for all writes.
        for h in handles {
            h.join().unwrap();
        }

        // All values should be findable.
        for i in 0..10 {
            let path = format!("topic/{}", i);
            let results = trie.match_topic(&path);
            assert_eq!(results, vec![i]);
        }
    }

    #[test]
    fn concurrent_match_while_inserting() {
        use std::sync::Arc;
        use std::thread;

        let trie = Arc::new(Trie::new());

        // Pre-insert some values.
        for i in 0..100 {
            trie.insert(&format!("pre/{}", i), i);
        }

        let mut handles = vec![];

        // Concurrent readers.
        for i in 0..10 {
            let trie = Arc::clone(&trie);
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    let path = format!("pre/{}", j);
                    let results = trie.match_topic(&path);
                    assert!(!results.is_empty());
                }
                i // return thread id for tracking
            }));
        }

        // Concurrent writer.
        {
            let trie = Arc::clone(&trie);
            handles.push(thread::spawn(move || {
                for j in 100..200 {
                    trie.insert(&format!("new/{}", j), j);
                }
                999
            }));
        }

        for h in handles {
            h.join().unwrap();
        }
    }

    // ========================================================================
    // split_first
    // ========================================================================

    #[test]
    fn split_first_two_segments() {
        assert_eq!(split_first("auth/state"), ("auth", "state"));
    }

    #[test]
    fn split_first_three_segments() {
        assert_eq!(split_first("a/b/c"), ("a", "b/c"));
    }

    #[test]
    fn split_first_single_segment() {
        assert_eq!(split_first("auth"), ("auth", ""));
    }

    #[test]
    fn split_first_empty() {
        assert_eq!(split_first(""), ("", ""));
    }
}
