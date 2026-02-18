//! KvStore trait + KvOps CRUD operations.
//!
//! The model impls `KvStore` to declare KEY + hooks.
//! `KvOps<T>` provides the actual get/save/list/delete using a KVStore backend.

use openerp_core::ServiceError;
use openerp_types::Field;
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
    kv: Arc<dyn openerp_kv::KVStore>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: KvStore> KvOps<T> {
    pub fn new(kv: Arc<dyn openerp_kv::KVStore>) -> Self {
        Self {
            kv,
            _phantom: std::marker::PhantomData,
        }
    }

    fn make_key(id: &str) -> String {
        format!("{}{}", T::kv_prefix(), id)
    }

    fn kv_err(e: openerp_kv::KVError) -> ServiceError {
        match e {
            openerp_kv::KVError::ReadOnly(key) => {
                ServiceError::ReadOnly(format!("key '{}' is read-only", key))
            }
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

    /// List records with pagination (limit/offset).
    ///
    /// Scans all entries then slices in memory. For KV stores the full scan
    /// is unavoidable; pagination just controls how much is returned to the caller.
    pub fn list_paginated(
        &self,
        params: &openerp_core::ListParams,
    ) -> Result<openerp_core::ListResult<T>, ServiceError> {
        let all = self.list()?;
        let total = all.len();
        let offset = params.offset.min(total);
        let end = (offset + params.limit).min(total);
        let items: Vec<T> = all.into_iter().skip(offset).take(params.limit).collect();
        let has_more = end < total;
        Ok(openerp_core::ListResult { items, has_more })
    }

    /// Count all records with this prefix.
    pub fn count(&self) -> Result<usize, ServiceError> {
        let entries = self
            .kv
            .scan(T::kv_prefix())
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        Ok(entries.len())
    }

    /// Create a new record. Calls before_create hook, checks for duplicates.
    pub fn save_new(&self, mut record: T) -> Result<T, ServiceError> {
        record.before_create();

        let id = record.key_value();
        let key = Self::make_key(&id);

        if self.kv.get(&key).map_err(Self::kv_err)?.is_some() {
            return Err(ServiceError::Conflict(format!(
                "{} '{}' already exists",
                T::KEY.name, id
            )));
        }

        let bytes = serde_json::to_vec(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        self.kv.set(&key, &bytes).map_err(Self::kv_err)?;

        Ok(record)
    }

    /// Update an existing record with optimistic locking on `updatedAt`.
    ///
    /// Compares the incoming record's `updatedAt` with the stored value.
    /// If they don't match, another writer has modified the record and
    /// we return `ServiceError::Conflict` (409).
    /// On success, `before_update()` has already set a fresh `updatedAt`.
    pub fn save(&self, mut record: T) -> Result<T, ServiceError> {
        let id = record.key_value();
        let key = Self::make_key(&id);

        // Read existing for optimistic locking check BEFORE calling
        // before_update (which overwrites updatedAt with "now").
        let existing_bytes = self.kv.get(&key).map_err(Self::kv_err)?;
        if let Some(existing_bytes) = &existing_bytes {
            let existing: serde_json::Value = serde_json::from_slice(existing_bytes)
                .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
            let incoming = serde_json::to_value(&record)
                .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

            let existing_ts = existing.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("");
            let incoming_ts = incoming.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("");

            if incoming_ts != existing_ts {
                return Err(ServiceError::Conflict(format!(
                    "updatedAt mismatch: stored {}, got {}",
                    existing_ts, incoming_ts
                )));
            }
        }

        // Now apply before_update which sets fresh updatedAt.
        record.before_update();

        let bytes = serde_json::to_vec(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        self.kv.set(&key, &bytes).map_err(Self::kv_err)?;

        Ok(record)
    }

    /// Partially update a record using RFC 7386 JSON Merge Patch.
    ///
    /// Reads the existing record, applies the patch, and saves.
    /// Include `updatedAt` from the GET response in the patch for
    /// optimistic locking — the server returns 409 if it doesn't match.
    pub fn patch(&self, id: &str, patch: &serde_json::Value) -> Result<T, ServiceError> {
        let existing = self.get_or_err(id)?;
        let mut base = serde_json::to_value(&existing)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

        // Check updatedAt from patch if provided.
        if let Some(patch_ts) = patch.get("updatedAt").and_then(|v| v.as_str()) {
            let base_ts = base.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("");
            if patch_ts != base_ts {
                return Err(ServiceError::Conflict(format!(
                    "updatedAt mismatch: stored {}, got {}",
                    base_ts, patch_ts
                )));
            }
        }

        openerp_core::merge_patch(&mut base, patch);

        // Set fresh updatedAt after merge.
        if let Some(obj) = base.as_object_mut() {
            obj.insert(
                "updatedAt".into(),
                serde_json::json!(chrono::Utc::now().to_rfc3339()),
            );
        }

        let record: T = serde_json::from_value(base)
            .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;

        let key = Self::make_key(id);
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

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct Thing {
        id: String,
        name: String,
        count: u32,
        #[serde(default)]
        updated_at: String,
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
            let now = chrono::Utc::now().to_rfc3339();
            if self.updated_at.is_empty() {
                self.updated_at = now;
            }
        }

        fn before_update(&mut self) {
            self.updated_at = chrono::Utc::now().to_rfc3339();
        }
    }

    fn make_ops() -> (KvOps<Thing>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> =
            Arc::new(openerp_kv::RedbStore::open(&dir.path().join("test.redb")).unwrap());
        (KvOps::new(kv), dir)
    }

    fn new_thing(id: &str, name: &str, count: u32) -> Thing {
        Thing { id: id.into(), name: name.into(), count, updated_at: String::new() }
    }

    #[test]
    fn crud_lifecycle() {
        let (ops, _dir) = make_ops();

        let created = ops.save_new(new_thing("", "Widget", 42)).unwrap();
        assert_eq!(created.id, "auto-id");

        let fetched = ops.get_or_err("auto-id").unwrap();
        assert_eq!(fetched.name, "Widget");
        assert_eq!(fetched.count, 42);

        let all = ops.list().unwrap();
        assert_eq!(all.len(), 1);

        let mut updated = fetched;
        updated.name = "Gadget".into();
        let updated = ops.save(updated).unwrap();
        assert_eq!(updated.name, "Gadget");

        ops.delete("auto-id").unwrap();
        assert!(ops.get("auto-id").unwrap().is_none());
    }

    #[test]
    fn duplicate_key_rejected() {
        let (ops, _dir) = make_ops();
        ops.save_new(new_thing("x", "A", 1)).unwrap();
        let err = ops.save_new(new_thing("x", "B", 2)).unwrap_err();
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

    #[test]
    fn readonly_key_rejected_on_write() {
        let dir = tempfile::tempdir().unwrap();
        let redb = openerp_kv::RedbStore::open(&dir.path().join("ro.redb")).unwrap();
        let overlay = openerp_kv::OverlayKV::new(redb);

        let data = serde_json::to_vec(&new_thing("ro1", "ReadOnly", 1)).unwrap();
        overlay.insert_file_entry("test:thing:ro1".into(), data);

        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(overlay);
        let ops = KvOps::<Thing>::new(kv);

        let fetched = ops.get("ro1").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "ReadOnly");

        let t = new_thing("ro1", "Changed", 2);
        let err = ops.save(t.clone()).unwrap_err();
        assert!(err.to_string().contains("read-only") || err.to_string().contains("ReadOnly"),
            "Save to readonly key should fail, got: {}", err);

        let err = ops.delete("ro1").unwrap_err();
        assert!(err.to_string().contains("read-only") || err.to_string().contains("ReadOnly"),
            "Delete of readonly key should fail, got: {}", err);

        let err = ops.save_new(new_thing("ro1", "Dup", 0)).unwrap_err();
        assert!(err.to_string().contains("already exists"),
            "save_new of existing readonly key should fail with duplicate, got: {}", err);
    }

    #[test]
    fn delete_nonexistent_returns_not_found() {
        let (ops, _dir) = make_ops();
        let err = ops.delete("ghost").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn list_paginated_basic() {
        let (ops, _dir) = make_ops();

        for i in 0..5u32 {
            ops.save_new(new_thing(&format!("p{}", i), &format!("Item {}", i), i)).unwrap();
        }

        let params = openerp_core::ListParams { limit: 2, offset: 0, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(result.has_more);

        let params = openerp_core::ListParams { limit: 2, offset: 2, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(result.has_more);

        let params = openerp_core::ListParams { limit: 2, offset: 4, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 1);
        assert!(!result.has_more);

        let params = openerp_core::ListParams { limit: 10, offset: 100, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 0);
        assert!(!result.has_more);
    }

    #[test]
    fn count_returns_total() {
        let (ops, _dir) = make_ops();
        assert_eq!(ops.count().unwrap(), 0);

        for i in 0..3u32 {
            ops.save_new(new_thing(&format!("c{}", i), "N", i)).unwrap();
        }
        assert_eq!(ops.count().unwrap(), 3);

        ops.delete("c1").unwrap();
        assert_eq!(ops.count().unwrap(), 2);
    }

    #[test]
    fn updated_at_set_on_create() {
        let (ops, _dir) = make_ops();
        let created = ops.save_new(new_thing("v1", "A", 1)).unwrap();
        assert!(!created.updated_at.is_empty(), "updatedAt should be set after create");

        let fetched = ops.get_or_err("v1").unwrap();
        assert_eq!(fetched.updated_at, created.updated_at);
    }

    #[test]
    fn updated_at_changes_on_update() {
        let (ops, _dir) = make_ops();
        ops.save_new(new_thing("v2", "A", 1)).unwrap();

        let fetched = ops.get_or_err("v2").unwrap();
        let old_ts = fetched.updated_at.clone();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut w = fetched;
        w.name = "B".into();
        let updated = ops.save(w).unwrap();
        assert_ne!(updated.updated_at, old_ts, "updatedAt should change on update");

        let re_read = ops.get_or_err("v2").unwrap();
        assert_eq!(re_read.updated_at, updated.updated_at);
        assert_eq!(re_read.name, "B");
    }

    #[test]
    fn optimistic_lock_conflict_returns_409() {
        let (ops, _dir) = make_ops();
        ops.save_new(new_thing("v3", "A", 1)).unwrap();

        let read1 = ops.get_or_err("v3").unwrap();
        let read2 = ops.get_or_err("v3").unwrap();
        assert_eq!(read1.updated_at, read2.updated_at);

        // First write succeeds — updatedAt matches stored.
        let mut w1 = read1;
        w1.name = "Updated by w1".into();
        ops.save(w1).unwrap();

        // Second write fails — its updatedAt is stale.
        let mut w2 = read2;
        w2.name = "Updated by w2".into();
        let err = ops.save(w2).unwrap_err();
        assert!(err.to_string().contains("updatedAt mismatch"),
            "Expected updatedAt conflict, got: {}", err);

        let final_read = ops.get_or_err("v3").unwrap();
        assert_eq!(final_read.name, "Updated by w1");
    }

    #[test]
    fn patch_partial_update() {
        let (ops, _dir) = make_ops();
        let created = ops.save_new(new_thing("p1", "Original", 10)).unwrap();
        let ts = &created.updated_at;

        let patch = serde_json::json!({ "name": "Patched", "updatedAt": ts });
        let patched = ops.patch("p1", &patch).unwrap();
        assert_eq!(patched.name, "Patched");
        assert_eq!(patched.count, 10, "count should be unchanged");
        assert_ne!(patched.updated_at, *ts, "updatedAt should be refreshed");

        let re_read = ops.get_or_err("p1").unwrap();
        assert_eq!(re_read.name, "Patched");
        assert_eq!(re_read.count, 10);
    }

    #[test]
    fn patch_without_updated_at_no_conflict() {
        let (ops, _dir) = make_ops();
        ops.save_new(new_thing("p2", "A", 1)).unwrap();

        let patch = serde_json::json!({ "name": "B" });
        let patched = ops.patch("p2", &patch).unwrap();
        assert_eq!(patched.name, "B");
    }

    #[test]
    fn patch_stale_updated_at_returns_409() {
        let (ops, _dir) = make_ops();
        let created = ops.save_new(new_thing("p3", "A", 1)).unwrap();
        let old_ts = &created.updated_at;

        let patch1 = serde_json::json!({ "name": "B", "updatedAt": old_ts });
        ops.patch("p3", &patch1).unwrap();

        // Second patch with stale updatedAt should fail.
        let patch2 = serde_json::json!({ "name": "C", "updatedAt": old_ts });
        let err = ops.patch("p3", &patch2).unwrap_err();
        assert!(err.to_string().contains("updatedAt mismatch"),
            "Expected updatedAt conflict, got: {}", err);
    }

    #[test]
    fn patch_nonexistent_returns_not_found() {
        let (ops, _dir) = make_ops();
        let patch = serde_json::json!({ "name": "X" });
        let err = ops.patch("ghost", &patch).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
