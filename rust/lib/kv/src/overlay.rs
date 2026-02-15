use std::collections::{BTreeMap, BTreeSet};
use std::sync::RwLock;

use crate::error::KVError;
use crate::traits::KVStore;

/// OverlayKV is a two-layer KV store:
///
/// - **File layer** (read-only, higher priority): loaded from data-dir YAML files.
/// - **DB layer** (read-write): backed by a concrete KVStore (e.g. redb).
///
/// When reading, the file layer is checked first. If a key exists in the file
/// layer, the DB layer value is shadowed. When writing, only the DB layer is
/// writable — attempts to write a file-layer key return `KVError::ReadOnly`.
///
/// `scan` merges both layers, with file-layer entries taking priority for
/// duplicate keys.
pub struct OverlayKV<DB: KVStore> {
    file_layer: RwLock<BTreeMap<String, Vec<u8>>>,
    db: DB,
}

impl<DB: KVStore> OverlayKV<DB> {
    /// Create a new OverlayKV with an empty file layer and the given DB backend.
    pub fn new(db: DB) -> Self {
        Self {
            file_layer: RwLock::new(BTreeMap::new()),
            db,
        }
    }

    /// Insert a key-value pair into the read-only file layer.
    /// This is called by FileLoader during initialization.
    pub fn insert_file_entry(&self, key: String, value: Vec<u8>) {
        let mut layer = self.file_layer.write().unwrap();
        layer.insert(key, value);
    }

    /// Get the number of entries in the file layer.
    pub fn file_layer_len(&self) -> usize {
        self.file_layer.read().unwrap().len()
    }
}

impl<DB: KVStore> KVStore for OverlayKV<DB> {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, KVError> {
        // File layer takes priority.
        {
            let layer = self.file_layer.read().unwrap();
            if let Some(value) = layer.get(key) {
                return Ok(Some(value.clone()));
            }
        }
        // Fall through to DB layer.
        self.db.get(key)
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), KVError> {
        if self.is_readonly(key) {
            return Err(KVError::ReadOnly(key.to_string()));
        }
        self.db.set(key, value)
    }

    fn delete(&self, key: &str) -> Result<(), KVError> {
        if self.is_readonly(key) {
            return Err(KVError::ReadOnly(key.to_string()));
        }
        self.db.delete(key)
    }

    fn batch_set(&self, entries: &[(&str, &[u8])]) -> Result<(), KVError> {
        // Check all keys for read-only before delegating — fail fast, no partial writes.
        for (key, _) in entries {
            if self.is_readonly(key) {
                return Err(KVError::ReadOnly(key.to_string()));
            }
        }
        self.db.batch_set(entries)
    }

    fn batch_delete(&self, keys: &[&str]) -> Result<(), KVError> {
        // Check all keys for read-only before delegating — fail fast, no partial deletes.
        for key in keys {
            if self.is_readonly(key) {
                return Err(KVError::ReadOnly(key.to_string()));
            }
        }
        self.db.batch_delete(keys)
    }

    fn scan(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>, KVError> {
        // Collect keys from both layers, file layer wins on conflict.
        let file_layer = self.file_layer.read().unwrap();
        let db_entries = self.db.scan(prefix)?;

        // Use a BTreeSet to track keys we've already emitted from file layer.
        let mut seen_keys = BTreeSet::new();
        let mut results = Vec::new();

        // File layer entries matching prefix.
        for (key, value) in file_layer.range(prefix.to_string()..) {
            if !key.starts_with(prefix) {
                break;
            }
            seen_keys.insert(key.clone());
            results.push((key.clone(), value.clone()));
        }

        // DB layer entries not shadowed by file layer.
        for (key, value) in db_entries {
            if !seen_keys.contains(&key) {
                results.push((key, value));
            }
        }

        // Sort by key for deterministic output.
        results.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(results)
    }

    fn is_readonly(&self, key: &str) -> bool {
        let layer = self.file_layer.read().unwrap();
        layer.contains_key(key)
    }
}
