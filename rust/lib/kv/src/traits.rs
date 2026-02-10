use crate::error::KVError;

/// KVStore provides a key-value storage interface with read-only key support.
///
/// Keys follow a namespaced convention: `config:model:h106`, `config:segment:manufacturer:1`, etc.
/// Keys loaded from the file layer are read-only; DB-layer keys are read-write.
pub trait KVStore: Send + Sync {
    /// Get the value for a key. Returns None if the key does not exist.
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, KVError>;

    /// Set a key-value pair. Returns KVError::ReadOnly if the key is in the read-only layer.
    fn set(&self, key: &str, value: &[u8]) -> Result<(), KVError>;

    /// Delete a key. Returns KVError::ReadOnly if the key is in the read-only layer.
    fn delete(&self, key: &str) -> Result<(), KVError>;

    /// Scan all keys matching a prefix. Returns sorted (key, value) pairs.
    /// Merges both file-layer and DB-layer results when applicable.
    fn scan(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>, KVError>;

    /// Check whether a key is in the read-only (file) layer.
    fn is_readonly(&self, key: &str) -> bool;
}
