//! KvStore trait + KvOps CRUD operations.
//!
//! The model impls `KvStore` to declare KEY + hooks.
//! `KvOps<T>` provides the actual get/save/list/delete using a KVStore backend.

use oe_core::ServiceError;
use oe_types::Field;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

/// Trait implemented by models to declare KV storage behavior.
///
/// KEY is the field used as the KV key. Hooks have default no-op impls.
pub trait KvStore: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    /// The key field. Value is extracted from the model instance via `key_value()`.
    const KEY: Field;

    /// KV key prefix: "{module}:{resource}:".
    /// Provided by `#[model]` — override if needed.
    fn kv_prefix() -> &'static str;

    /// Extract the key value from this instance as a string.
    fn key_value(&self) -> String;

    /// Called before inserting a new record. Use for auto-fill (uuid, timestamps).
    fn before_create(&mut self) {}

    /// Called before updating an existing record.
    fn before_update(&mut self) {}

    /// Called after a record is deleted.
    fn after_delete(&self) {}
}

/// CRUD operations for a KvStore model. Holds a reference to the KV backend.
pub struct KvOps<T: KvStore> {
    kv: Arc<dyn oe_kv::KVStore>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: KvStore> KvOps<T> {
    pub fn new(kv: Arc<dyn oe_kv::KVStore>) -> Self {
        Self {
            kv,
            _phantom: std::marker::PhantomData,
        }
    }

    fn make_key(id: &str) -> String {
        format!("{}{}", T::kv_prefix(), id)
    }

    fn kv_err(e: oe_kv::KVError) -> ServiceError {
        match e {
            oe_kv::KVError::ReadOnly(msg) => ServiceError::ReadOnly(msg),
            other => ServiceError::Storage(other.to_string()),
        }
    }

    /// Get a record by key value. Returns None if not found.
    pub fn get(&self, id: &str) -> Result<Option<T>, ServiceError> {
        let key = Self::make_key(id);
        match self.kv.get(&key).map_err(Self::kv_err)? {
            Some(bytes) => {
                let record: T = serde_json::from_slice(&bytes)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Get a record or return NotFound error.
    pub fn get_or_err(&self, id: &str) -> Result<T, ServiceError> {
        self.get(id)?.ok_or_else(|| {
            ServiceError::NotFound(format!("{} '{}' not found", T::KEY.name, id))
        })
    }

    /// List all records with this prefix.
    pub fn list(&self) -> Result<Vec<T>, ServiceError> {
        let entries = self
            .kv
            .scan(T::kv_prefix())
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        let mut records = Vec::with_capacity(entries.len());
        for (_key, bytes) in entries {
            let record: T = serde_json::from_slice(&bytes)
                .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
            records.push(record);
        }
        Ok(records)
    }

    /// Create a new record. Calls before_create hook, checks for duplicates.
    pub fn save_new(&self, mut record: T) -> Result<T, ServiceError> {
        record.before_create();

        let id = record.key_value();
        let key = Self::make_key(&id);

        // Check duplicate.
        if self.kv.get(&key).map_err(Self::kv_err)?.is_some() {
            return Err(ServiceError::Validation(format!(
                "{} '{}' already exists",
                T::KEY.name, id
            )));
        }

        let bytes = serde_json::to_vec(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        self.kv.set(&key, &bytes).map_err(Self::kv_err)?;

        Ok(record)
    }

    /// Update an existing record. Calls before_update hook.
    pub fn save(&self, mut record: T) -> Result<T, ServiceError> {
        record.before_update();

        let id = record.key_value();
        let key = Self::make_key(&id);

        let bytes = serde_json::to_vec(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        self.kv.set(&key, &bytes).map_err(Self::kv_err)?;

        Ok(record)
    }

    /// Delete a record by key value.
    pub fn delete(&self, id: &str) -> Result<(), ServiceError> {
        let record = self.get_or_err(id)?;
        let key = Self::make_key(id);
        self.kv.delete(&key).map_err(Self::kv_err)?;
        record.after_delete();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    // A minimal test model (hand-built, no macro).
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Thing {
        id: String,
        name: String,
        count: u32,
    }

    impl KvStore for Thing {
        const KEY: Field = Field::new("id", "String", "text");

        fn kv_prefix() -> &'static str {
            "test:thing:"
        }

        fn key_value(&self) -> String {
            self.id.clone()
        }

        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = "auto-id".to_string();
            }
        }
    }

    fn make_ops() -> (KvOps<Thing>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn oe_kv::KVStore> =
            Arc::new(oe_kv::RedbStore::open(&dir.path().join("test.redb")).unwrap());
        (KvOps::new(kv), dir)
    }

    #[test]
    fn crud_lifecycle() {
        let (ops, _dir) = make_ops();

        // Create with auto-fill.
        let thing = Thing {
            id: String::new(),
            name: "Widget".into(),
            count: 42,
        };
        let created = ops.save_new(thing).unwrap();
        assert_eq!(created.id, "auto-id"); // before_create hook fired

        // Get.
        let fetched = ops.get_or_err("auto-id").unwrap();
        assert_eq!(fetched.name, "Widget");
        assert_eq!(fetched.count, 42);

        // List.
        let all = ops.list().unwrap();
        assert_eq!(all.len(), 1);

        // Update.
        let mut updated = fetched;
        updated.name = "Gadget".into();
        let updated = ops.save(updated).unwrap();
        assert_eq!(updated.name, "Gadget");

        // Delete.
        ops.delete("auto-id").unwrap();
        assert!(ops.get("auto-id").unwrap().is_none());
    }

    #[test]
    fn duplicate_key_rejected() {
        let (ops, _dir) = make_ops();

        let t1 = Thing { id: "x".into(), name: "A".into(), count: 1 };
        ops.save_new(t1).unwrap();

        let t2 = Thing { id: "x".into(), name: "B".into(), count: 2 };
        let err = ops.save_new(t2).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let (ops, _dir) = make_ops();
        assert!(ops.get("nope").unwrap().is_none());
    }

    #[test]
    fn get_or_err_returns_not_found() {
        let (ops, _dir) = make_ops();
        let err = ops.get_or_err("nope").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
