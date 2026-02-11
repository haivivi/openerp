use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Cached entry for a user's expanded group list.
struct CacheEntry {
    groups: Vec<String>,
    inserted_at: Instant,
}

/// In-memory cache for user -> expanded groups with TTL.
pub struct GroupCache {
    ttl: Duration,
    entries: RwLock<HashMap<String, CacheEntry>>,
}

impl GroupCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Get cached groups for a user. Returns None if expired or missing.
    pub fn get(&self, user_id: &str) -> Option<Vec<String>> {
        let entries = self.entries.read().unwrap();
        entries.get(user_id).and_then(|entry| {
            if entry.inserted_at.elapsed() < self.ttl {
                Some(entry.groups.clone())
            } else {
                None
            }
        })
    }

    /// Get stale groups for a user (expired but still cached). Returns None if missing.
    pub fn get_stale(&self, user_id: &str) -> Option<Vec<String>> {
        let entries = self.entries.read().unwrap();
        entries.get(user_id).map(|entry| entry.groups.clone())
    }

    /// Store groups for a user.
    pub fn set(&self, user_id: &str, groups: Vec<String>) {
        let mut entries = self.entries.write().unwrap();
        entries.insert(
            user_id.to_string(),
            CacheEntry {
                groups,
                inserted_at: Instant::now(),
            },
        );
    }

    /// Invalidate a specific user's cache entry.
    pub fn invalidate(&self, user_id: &str) {
        let mut entries = self.entries.write().unwrap();
        entries.remove(user_id);
    }

    /// Invalidate all cache entries.
    pub fn invalidate_all(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.clear();
    }
}
