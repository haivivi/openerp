use std::any::Any;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::trie::Trie;
use crate::value::{StateValue, SubscriptionId};

/// Callback type for state change notifications.
pub type ChangeHandler = Arc<dyn Fn(&str, &StateValue) + Send + Sync>;

/// Per-path state store with Trie-based subscription routing.
///
/// - `set(path, value)` stores a value and notifies all matching subscribers.
/// - `get(path)` reads the current value (Arc clone, cheap).
/// - `scan(prefix)` lists all children under a prefix path.
/// - `subscribe(pattern, handler)` registers a change handler.
/// - `unsubscribe(pattern, id)` removes a handler.
///
/// Uses `BTreeMap` internally for ordered prefix scanning.
pub struct StateStore {
    /// Current state values, keyed by exact path. BTreeMap for ordered scan.
    values: RwLock<BTreeMap<String, StateValue>>,
    /// Trie mapping subscription patterns to handler entries.
    handlers: Trie<HandlerEntry>,
    /// Monotonic counter for subscription IDs.
    next_id: AtomicU64,
}

#[derive(Clone)]
struct HandlerEntry {
    id: SubscriptionId,
    handler: ChangeHandler,
}

impl StateStore {
    /// Create a new empty StateStore.
    pub fn new() -> Self {
        Self {
            values: RwLock::new(BTreeMap::new()),
            handlers: Trie::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Set a typed value at the given path and notify matching subscribers.
    ///
    /// Wraps the value in `StateValue` (Arc) internally.
    pub fn set<T: Any + Send + Sync>(&self, path: &str, value: T) {
        self.set_value(path, StateValue::new(value));
    }

    /// Set a pre-built StateValue at the given path and notify matching subscribers.
    pub fn set_value(&self, path: &str, value: StateValue) {
        {
            let mut values = self.values.write().unwrap();
            values.insert(path.to_string(), value.clone());
        }
        // Notify all subscribers whose pattern matches this path.
        let entries = self.handlers.match_topic(path);
        for entry in entries {
            (entry.handler)(path, &value);
        }
    }

    /// Get the current state value at the given path.
    ///
    /// Returns a cloned `StateValue` (Arc clone, cheap — no data copy).
    /// Returns `None` if no value is set at this path.
    pub fn get(&self, path: &str) -> Option<StateValue> {
        let values = self.values.read().unwrap();
        values.get(path).cloned()
    }

    /// Remove the state value at the given path.
    ///
    /// Returns the old value if present. Does NOT notify subscribers.
    pub fn remove(&self, path: &str) -> Option<StateValue> {
        let mut values = self.values.write().unwrap();
        values.remove(path)
    }

    /// Scan all entries whose path starts with `{prefix}/`.
    ///
    /// Does NOT include the exact `prefix` path itself — only children.
    /// Results are ordered by path (BTreeMap ordering).
    ///
    /// Example: `scan("home/devices/items")` returns entries at
    /// `home/devices/items/1`, `home/devices/items/2`, etc.
    pub fn scan(&self, prefix: &str) -> Vec<(String, StateValue)> {
        let values = self.values.read().unwrap();
        let scan_prefix = format!("{}/", prefix);
        values
            .range(scan_prefix.clone()..)
            .take_while(|(k, _)| k.starts_with(&scan_prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Check if a value exists at the given path.
    pub fn contains(&self, path: &str) -> bool {
        let values = self.values.read().unwrap();
        values.contains_key(path)
    }

    /// Get the total number of stored paths.
    pub fn len(&self) -> usize {
        let values = self.values.read().unwrap();
        values.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Subscribe to state changes matching the given Trie pattern.
    ///
    /// The handler is called synchronously whenever `set` or `set_value`
    /// is called on a path that matches the pattern.
    ///
    /// Returns a `SubscriptionId` that can be used to unsubscribe.
    pub fn subscribe<F>(&self, pattern: &str, handler: F) -> SubscriptionId
    where
        F: Fn(&str, &StateValue) + Send + Sync + 'static,
    {
        let id = SubscriptionId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let entry = HandlerEntry {
            id,
            handler: Arc::new(handler),
        };
        self.handlers.insert(pattern, entry);
        id
    }

    /// Unsubscribe a handler by its subscription ID and pattern.
    pub fn unsubscribe(&self, pattern: &str, id: SubscriptionId) {
        self.handlers.remove(pattern, |entry| entry.id == id);
    }

    /// Get a snapshot of all paths and values.
    ///
    /// Returns entries ordered by path.
    pub fn snapshot(&self) -> Vec<(String, StateValue)> {
        let values = self.values.read().unwrap();
        values.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Get all paths currently stored.
    pub fn paths(&self) -> Vec<String> {
        let values = self.values.read().unwrap();
        values.keys().cloned().collect()
    }
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    // ========================================================================
    // Basic get/set
    // ========================================================================

    #[test]
    fn set_and_get_u32() {
        let store = StateStore::new();
        store.set("counter", 42u32);

        let v = store.get("counter").unwrap();
        assert_eq!(v.downcast_ref::<u32>(), Some(&42));
    }

    #[test]
    fn set_and_get_string() {
        let store = StateStore::new();
        store.set("name", "hello".to_string());

        let v = store.get("name").unwrap();
        assert_eq!(v.downcast_ref::<String>(), Some(&"hello".to_string()));
    }

    #[test]
    fn set_and_get_struct() {
        #[derive(Debug, PartialEq)]
        struct AuthState {
            phase: String,
            busy: bool,
        }

        let store = StateStore::new();
        store.set(
            "auth/state",
            AuthState {
                phase: "authenticated".to_string(),
                busy: false,
            },
        );

        let v = store.get("auth/state").unwrap();
        let state = v.downcast_ref::<AuthState>().unwrap();
        assert_eq!(state.phase, "authenticated");
        assert!(!state.busy);
    }

    #[test]
    fn get_missing_returns_none() {
        let store = StateStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn set_overwrites_previous_value() {
        let store = StateStore::new();
        store.set("counter", 1u32);
        store.set("counter", 2u32);

        let v = store.get("counter").unwrap();
        assert_eq!(v.downcast_ref::<u32>(), Some(&2));
    }

    #[test]
    fn set_overwrites_different_type() {
        let store = StateStore::new();
        store.set("value", 42u32);
        store.set("value", "now a string".to_string());

        let v = store.get("value").unwrap();
        assert_eq!(v.downcast_ref::<u32>(), None);
        assert_eq!(
            v.downcast_ref::<String>(),
            Some(&"now a string".to_string())
        );
    }

    #[test]
    fn get_returns_arc_clone_not_data_clone() {
        let store = StateStore::new();
        let big = vec![0u8; 1_000_000];
        store.set("big", big);

        let v1 = store.get("big").unwrap();
        let v2 = store.get("big").unwrap();

        // Both point to the same underlying data (Arc shared).
        let p1 = v1.downcast_ref::<Vec<u8>>().unwrap().as_ptr();
        let p2 = v2.downcast_ref::<Vec<u8>>().unwrap().as_ptr();
        assert_eq!(p1, p2);
    }

    #[test]
    fn set_value_prebuilt() {
        let store = StateStore::new();
        let sv = StateValue::new(42u32);
        store.set_value("counter", sv.clone());

        let v = store.get("counter").unwrap();
        assert_eq!(v.downcast_ref::<u32>(), Some(&42));
    }

    // ========================================================================
    // Remove
    // ========================================================================

    #[test]
    fn remove_existing_returns_value() {
        let store = StateStore::new();
        store.set("counter", 42u32);

        let old = store.remove("counter").unwrap();
        assert_eq!(old.downcast_ref::<u32>(), Some(&42));
        assert!(store.get("counter").is_none());
    }

    #[test]
    fn remove_missing_returns_none() {
        let store = StateStore::new();
        assert!(store.remove("nonexistent").is_none());
    }

    #[test]
    fn remove_then_set_again() {
        let store = StateStore::new();
        store.set("counter", 1u32);
        store.remove("counter");
        store.set("counter", 2u32);

        assert_eq!(store.get("counter").unwrap().downcast_ref::<u32>(), Some(&2));
    }

    // ========================================================================
    // Scan
    // ========================================================================

    #[test]
    fn scan_returns_children() {
        let store = StateStore::new();
        store.set("home/devices/items/1", "device-1".to_string());
        store.set("home/devices/items/2", "device-2".to_string());
        store.set("home/devices/items/3", "device-3".to_string());

        let results = store.scan("home/devices/items");
        assert_eq!(results.len(), 3);
        assert_eq!(
            results[0].1.downcast_ref::<String>(),
            Some(&"device-1".to_string())
        );
        assert_eq!(
            results[1].1.downcast_ref::<String>(),
            Some(&"device-2".to_string())
        );
    }

    #[test]
    fn scan_does_not_include_exact_prefix() {
        let store = StateStore::new();
        store.set("home/devices", "parent".to_string());
        store.set("home/devices/1", "child-1".to_string());
        store.set("home/devices/2", "child-2".to_string());

        let results = store.scan("home/devices");
        assert_eq!(results.len(), 2);
        // "home/devices" itself should NOT be in results.
        assert!(results.iter().all(|(k, _)| k != "home/devices"));
    }

    #[test]
    fn scan_returns_nested_children() {
        let store = StateStore::new();
        store.set("a/b/c", 1u32);
        store.set("a/b/c/d", 2u32);
        store.set("a/b/c/d/e", 3u32);

        let results = store.scan("a/b");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn scan_no_matches() {
        let store = StateStore::new();
        store.set("auth/state", 1u32);

        let results = store.scan("home/devices");
        assert!(results.is_empty());
    }

    #[test]
    fn scan_does_not_match_similar_prefix() {
        let store = StateStore::new();
        store.set("auth/state", 1u32);
        store.set("authorization/state", 2u32);

        let results = store.scan("auth");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "auth/state");
    }

    #[test]
    fn scan_empty_store() {
        let store = StateStore::new();
        assert!(store.scan("any/prefix").is_empty());
    }

    #[test]
    fn scan_results_are_ordered() {
        let store = StateStore::new();
        store.set("items/c", 3u32);
        store.set("items/a", 1u32);
        store.set("items/b", 2u32);

        let results = store.scan("items");
        let paths: Vec<&str> = results.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(paths, vec!["items/a", "items/b", "items/c"]);
    }

    // ========================================================================
    // Contains / len / is_empty
    // ========================================================================

    #[test]
    fn contains_existing() {
        let store = StateStore::new();
        store.set("auth/state", 1u32);

        assert!(store.contains("auth/state"));
        assert!(!store.contains("auth/terms"));
    }

    #[test]
    fn len_and_is_empty() {
        let store = StateStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        store.set("a", 1u32);
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);

        store.set("b", 2u32);
        assert_eq!(store.len(), 2);

        store.remove("a");
        assert_eq!(store.len(), 1);
    }

    // ========================================================================
    // Snapshot / paths
    // ========================================================================

    #[test]
    fn snapshot_returns_all() {
        let store = StateStore::new();
        store.set("a", 1u32);
        store.set("b", 2u32);
        store.set("c", 3u32);

        let snap = store.snapshot();
        assert_eq!(snap.len(), 3);
        let paths: Vec<&str> = snap.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(paths, vec!["a", "b", "c"]);
    }

    #[test]
    fn snapshot_empty_store() {
        let store = StateStore::new();
        assert!(store.snapshot().is_empty());
    }

    #[test]
    fn paths_returns_all_keys() {
        let store = StateStore::new();
        store.set("auth/state", 1u32);
        store.set("app/route", 2u32);

        let mut paths = store.paths();
        paths.sort();
        assert_eq!(paths, vec!["app/route", "auth/state"]);
    }

    // ========================================================================
    // Subscribe — exact match
    // ========================================================================

    #[test]
    fn subscribe_exact_notifies_on_match() {
        let store = StateStore::new();
        let called = Arc::new(AtomicU64::new(0));
        let called_c = called.clone();

        store.subscribe("auth/state", move |path, _value| {
            assert_eq!(path, "auth/state");
            called_c.fetch_add(1, Ordering::Relaxed);
        });

        store.set("auth/state", 1u32);
        assert_eq!(called.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn subscribe_exact_does_not_notify_other_paths() {
        let store = StateStore::new();
        let called = Arc::new(AtomicU64::new(0));
        let called_c = called.clone();

        store.subscribe("auth/state", move |_path, _value| {
            called_c.fetch_add(1, Ordering::Relaxed);
        });

        store.set("auth/terms", 1u32);
        store.set("home/devices", 2u32);
        assert_eq!(called.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn subscribe_receives_correct_value() {
        let store = StateStore::new();
        let received = Arc::new(RwLock::new(None::<u32>));
        let received_c = received.clone();

        store.subscribe("counter", move |_path, value| {
            let v = *value.downcast_ref::<u32>().unwrap();
            *received_c.write().unwrap() = Some(v);
        });

        store.set("counter", 42u32);
        assert_eq!(*received.read().unwrap(), Some(42));
    }

    #[test]
    fn subscribe_called_on_every_set() {
        let store = StateStore::new();
        let count = Arc::new(AtomicU64::new(0));
        let count_c = count.clone();

        store.subscribe("counter", move |_path, _value| {
            count_c.fetch_add(1, Ordering::Relaxed);
        });

        store.set("counter", 1u32);
        store.set("counter", 2u32);
        store.set("counter", 3u32);
        assert_eq!(count.load(Ordering::Relaxed), 3);
    }

    // ========================================================================
    // Subscribe — wildcard patterns
    // ========================================================================

    #[test]
    fn subscribe_single_wildcard() {
        let store = StateStore::new();
        let paths_seen = Arc::new(RwLock::new(Vec::<String>::new()));
        let paths_c = paths_seen.clone();

        store.subscribe("auth/+", move |path, _value| {
            paths_c.write().unwrap().push(path.to_string());
        });

        store.set("auth/state", 1u32);
        store.set("auth/terms", 2u32);
        store.set("home/devices", 3u32); // should NOT trigger

        let paths = paths_seen.read().unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"auth/state".to_string()));
        assert!(paths.contains(&"auth/terms".to_string()));
    }

    #[test]
    fn subscribe_multi_wildcard() {
        let store = StateStore::new();
        let count = Arc::new(AtomicU64::new(0));
        let count_c = count.clone();

        store.subscribe("auth/#", move |_path, _value| {
            count_c.fetch_add(1, Ordering::Relaxed);
        });

        store.set("auth/state", 1u32);
        store.set("auth/terms", 2u32);
        store.set("auth/deep/nested/path", 3u32);
        store.set("home/devices", 4u32); // should NOT trigger

        assert_eq!(count.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn subscribe_root_wildcard() {
        let store = StateStore::new();
        let count = Arc::new(AtomicU64::new(0));
        let count_c = count.clone();

        store.subscribe("#", move |_path, _value| {
            count_c.fetch_add(1, Ordering::Relaxed);
        });

        store.set("auth/state", 1u32);
        store.set("home/devices", 2u32);
        store.set("any/path/at/all", 3u32);

        assert_eq!(count.load(Ordering::Relaxed), 3);
    }

    // ========================================================================
    // Multiple subscribers
    // ========================================================================

    #[test]
    fn multiple_subscribers_same_pattern() {
        let store = StateStore::new();
        let count_a = Arc::new(AtomicU64::new(0));
        let count_b = Arc::new(AtomicU64::new(0));
        let ca = count_a.clone();
        let cb = count_b.clone();

        store.subscribe("auth/state", move |_, _| {
            ca.fetch_add(1, Ordering::Relaxed);
        });
        store.subscribe("auth/state", move |_, _| {
            cb.fetch_add(1, Ordering::Relaxed);
        });

        store.set("auth/state", 1u32);

        assert_eq!(count_a.load(Ordering::Relaxed), 1);
        assert_eq!(count_b.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn multiple_subscribers_different_patterns() {
        let store = StateStore::new();
        let exact = Arc::new(AtomicU64::new(0));
        let wild = Arc::new(AtomicU64::new(0));
        let all = Arc::new(AtomicU64::new(0));
        let e = exact.clone();
        let w = wild.clone();
        let a = all.clone();

        store.subscribe("auth/state", move |_, _| {
            e.fetch_add(1, Ordering::Relaxed);
        });
        store.subscribe("auth/+", move |_, _| {
            w.fetch_add(1, Ordering::Relaxed);
        });
        store.subscribe("#", move |_, _| {
            a.fetch_add(1, Ordering::Relaxed);
        });

        store.set("auth/state", 1u32);

        assert_eq!(exact.load(Ordering::Relaxed), 1);
        assert_eq!(wild.load(Ordering::Relaxed), 1);
        assert_eq!(all.load(Ordering::Relaxed), 1);
    }

    // ========================================================================
    // Unsubscribe
    // ========================================================================

    #[test]
    fn unsubscribe_stops_notifications() {
        let store = StateStore::new();
        let count = Arc::new(AtomicU64::new(0));
        let count_c = count.clone();

        let id = store.subscribe("auth/state", move |_, _| {
            count_c.fetch_add(1, Ordering::Relaxed);
        });

        store.set("auth/state", 1u32);
        assert_eq!(count.load(Ordering::Relaxed), 1);

        store.unsubscribe("auth/state", id);
        store.set("auth/state", 2u32);
        assert_eq!(count.load(Ordering::Relaxed), 1); // not incremented
    }

    #[test]
    fn unsubscribe_one_keeps_others() {
        let store = StateStore::new();
        let count_a = Arc::new(AtomicU64::new(0));
        let count_b = Arc::new(AtomicU64::new(0));
        let ca = count_a.clone();
        let cb = count_b.clone();

        let id_a = store.subscribe("auth/state", move |_, _| {
            ca.fetch_add(1, Ordering::Relaxed);
        });
        let _id_b = store.subscribe("auth/state", move |_, _| {
            cb.fetch_add(1, Ordering::Relaxed);
        });

        store.unsubscribe("auth/state", id_a);
        store.set("auth/state", 1u32);

        assert_eq!(count_a.load(Ordering::Relaxed), 0); // unsubscribed
        assert_eq!(count_b.load(Ordering::Relaxed), 1); // still active
    }

    #[test]
    fn unsubscribe_wildcard() {
        let store = StateStore::new();
        let count = Arc::new(AtomicU64::new(0));
        let count_c = count.clone();

        let id = store.subscribe("auth/#", move |_, _| {
            count_c.fetch_add(1, Ordering::Relaxed);
        });

        store.set("auth/state", 1u32);
        assert_eq!(count.load(Ordering::Relaxed), 1);

        store.unsubscribe("auth/#", id);
        store.set("auth/state", 2u32);
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn unsubscribe_nonexistent_is_noop() {
        let store = StateStore::new();
        // Should not panic.
        store.unsubscribe("auth/state", SubscriptionId(999));
    }

    // ========================================================================
    // Subscription IDs are unique
    // ========================================================================

    #[test]
    fn subscription_ids_are_monotonic() {
        let store = StateStore::new();

        let id1 = store.subscribe("a", |_, _| {});
        let id2 = store.subscribe("b", |_, _| {});
        let id3 = store.subscribe("c", |_, _| {});

        assert!(id1 != id2);
        assert!(id2 != id3);
        assert!(id1 != id3);
    }

    // ========================================================================
    // Notification ordering
    // ========================================================================

    #[test]
    fn subscriber_sees_value_after_store_updated() {
        let store = Arc::new(StateStore::new());
        let store_c = store.clone();

        store.subscribe("counter", move |path, _value| {
            // Inside the notification, the store should already have the new value.
            let current = store_c.get(path).unwrap();
            assert!(current.downcast_ref::<u32>().is_some());
        });

        store.set("counter", 42u32);
    }

    // ========================================================================
    // set_value also notifies
    // ========================================================================

    #[test]
    fn set_value_triggers_subscription() {
        let store = StateStore::new();
        let called = Arc::new(AtomicU64::new(0));
        let called_c = called.clone();

        store.subscribe("test", move |_, _| {
            called_c.fetch_add(1, Ordering::Relaxed);
        });

        let sv = StateValue::new(42u32);
        store.set_value("test", sv);
        assert_eq!(called.load(Ordering::Relaxed), 1);
    }

    // ========================================================================
    // Thread safety
    // ========================================================================

    #[test]
    fn concurrent_set_and_get() {
        use std::thread;

        let store = Arc::new(StateStore::new());
        let mut handles = vec![];

        // Writer thread.
        let store_w = store.clone();
        handles.push(thread::spawn(move || {
            for i in 0u32..1000 {
                store_w.set(&format!("item/{}", i), i);
            }
        }));

        // Reader thread.
        let store_r = store.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                let _ = store_r.get("item/0");
                let _ = store_r.scan("item");
            }
        }));

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(store.len(), 1000);
    }

    #[test]
    fn concurrent_subscribe_and_set() {
        use std::thread;

        let store = Arc::new(StateStore::new());
        let total = Arc::new(AtomicU64::new(0));

        // Subscribe before spawning threads.
        let total_c = total.clone();
        store.subscribe("#", move |_, _| {
            total_c.fetch_add(1, Ordering::Relaxed);
        });

        let mut handles = vec![];
        for t in 0..4 {
            let store_c = store.clone();
            handles.push(thread::spawn(move || {
                for i in 0..100 {
                    store_c.set(&format!("thread/{}/{}", t, i), i as u32);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(total.load(Ordering::Relaxed), 400);
        assert_eq!(store.len(), 400);
    }

    // ========================================================================
    // Default trait
    // ========================================================================

    #[test]
    fn default_creates_empty_store() {
        let store = StateStore::default();
        assert!(store.is_empty());
    }
}
