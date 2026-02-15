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
    /// Sets version to 1.
    pub fn save_new(&self, mut record: T) -> Result<T, ServiceError> {
        record.before_create();

        let id = record.key_value();
        let key = Self::make_key(&id);

        // Check duplicate.
        if self.kv.get(&key).map_err(Self::kv_err)?.is_some() {
            return Err(ServiceError::Conflict(format!(
                "{} '{}' already exists",
                T::KEY.name, id
            )));
        }

        // Set initial revision.
        let mut json_val = serde_json::to_value(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        if let Some(obj) = json_val.as_object_mut() {
            obj.insert("rev".into(), serde_json::json!(1));
        }
        let record: T = serde_json::from_value(json_val)
            .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;

        let bytes = serde_json::to_vec(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        self.kv.set(&key, &bytes).map_err(Self::kv_err)?;

        Ok(record)
    }

    /// Update an existing record with optimistic locking.
    ///
    /// Compares the incoming record's `version` with the stored version.
    /// If they don't match, returns `ServiceError::Conflict` (409).
    /// On success, increments version by 1 and saves.
    pub fn save(&self, mut record: T) -> Result<T, ServiceError> {
        record.before_update();

        let id = record.key_value();
        let key = Self::make_key(&id);

        // Read existing for version check.
        let existing_bytes = self.kv.get(&key).map_err(Self::kv_err)?;
        if let Some(existing_bytes) = &existing_bytes {
            let existing: serde_json::Value = serde_json::from_slice(existing_bytes)
                .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
            let incoming = serde_json::to_value(&record)
                .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

            let existing_rev = existing.get("rev").and_then(|v| v.as_u64()).unwrap_or(0);
            let incoming_rev = incoming.get("rev").and_then(|v| v.as_u64()).unwrap_or(0);

            if incoming_rev != existing_rev {
                return Err(ServiceError::Conflict(format!(
                    "rev mismatch: expected {}, got {}",
                    existing_rev, incoming_rev
                )));
            }
        }

        // Bump revision.
        let mut json_val = serde_json::to_value(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        if let Some(obj) = json_val.as_object_mut() {
            let rev = obj.get("rev").and_then(|v| v.as_u64()).unwrap_or(0);
            obj.insert("rev".into(), serde_json::json!(rev + 1));
        }
        let record: T = serde_json::from_value(json_val)
            .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;

        let bytes = serde_json::to_vec(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        self.kv.set(&key, &bytes).map_err(Self::kv_err)?;

        Ok(record)
    }

    /// Partially update a record using RFC 7386 JSON Merge Patch.
    ///
    /// Reads the existing record, applies the patch, checks version,
    /// bumps rev, and saves. The patch JSON should include `rev` from
    /// the GET response for optimistic locking.
    pub fn patch(&self, id: &str, patch: &serde_json::Value) -> Result<T, ServiceError> {
        let existing = self.get_or_err(id)?;
        let mut base = serde_json::to_value(&existing)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

        // Check rev from patch if provided.
        if let Some(patch_rev) = patch.get("rev").and_then(|v| v.as_u64()) {
            let base_rev = base.get("rev").and_then(|v| v.as_u64()).unwrap_or(0);
            if patch_rev != base_rev {
                return Err(ServiceError::Conflict(format!(
                    "rev mismatch: expected {}, got {}",
                    base_rev, patch_rev
                )));
            }
        }

        openerp_core::merge_patch(&mut base, patch);

        // Bump rev.
        if let Some(obj) = base.as_object_mut() {
            let rev = obj.get("rev").and_then(|v| v.as_u64()).unwrap_or(0);
            obj.insert("rev".into(), serde_json::json!(rev + 1));
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

    // A minimal test model (hand-built, no macro).
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Thing {
        id: String,
        name: String,
        count: u32,
        #[serde(default)]
        rev: u64,
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
        let kv: Arc<dyn openerp_kv::KVStore> =
            Arc::new(openerp_kv::RedbStore::open(&dir.path().join("test.redb")).unwrap());
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
            rev: 0,
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

        let t1 = Thing { id: "x".into(), name: "A".into(), count: 1, rev: 0 };
        ops.save_new(t1).unwrap();

        let t2 = Thing { id: "x".into(), name: "B".into(), count: 2, rev: 0 };
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

    #[test]
    fn readonly_key_rejected_on_write() {
        let dir = tempfile::tempdir().unwrap();
        let redb = openerp_kv::RedbStore::open(&dir.path().join("ro.redb")).unwrap();
        let overlay = openerp_kv::OverlayKV::new(redb);

        // Insert a key into the read-only file layer.
        let data = serde_json::to_vec(&Thing { id: "ro1".into(), name: "ReadOnly".into(), count: 1, rev: 0 }).unwrap();
        overlay.insert_file_entry("test:thing:ro1".into(), data);

        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(overlay);
        let ops = KvOps::<Thing>::new(kv);

        // Can read.
        let fetched = ops.get("ro1").unwrap();
        assert!(fetched.is_some(), "Should be able to read file-layer key");
        assert_eq!(fetched.unwrap().name, "ReadOnly");

        // Can't update.
        let mut t = Thing { id: "ro1".into(), name: "Changed".into(), count: 2, rev: 0 };
        let err = ops.save(t.clone()).unwrap_err();
        assert!(err.to_string().contains("read-only") || err.to_string().contains("ReadOnly"),
            "Save to readonly key should fail, got: {}", err);

        // Can't delete.
        let err = ops.delete("ro1").unwrap_err();
        assert!(err.to_string().contains("read-only") || err.to_string().contains("ReadOnly"),
            "Delete of readonly key should fail, got: {}", err);

        // Can't save_new (duplicate + readonly).
        t.id = "ro1".into();
        let err = ops.save_new(t).unwrap_err();
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

        // Insert 5 items.
        for i in 0..5 {
            let t = Thing { id: format!("p{}", i), name: format!("Item {}", i), count: i, rev: 0 };
            ops.save_new(t).unwrap();
        }

        // First page: limit=2, offset=0.
        let params = openerp_core::ListParams { limit: 2, offset: 0, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(result.has_more);

        // Second page: limit=2, offset=2.
        let params = openerp_core::ListParams { limit: 2, offset: 2, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(result.has_more);

        // Third page: limit=2, offset=4.
        let params = openerp_core::ListParams { limit: 2, offset: 4, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 1);
        assert!(!result.has_more);

        // Beyond range.
        let params = openerp_core::ListParams { limit: 10, offset: 100, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 0);
        assert!(!result.has_more);
    }

    #[test]
    fn count_returns_total() {
        let (ops, _dir) = make_ops();
        assert_eq!(ops.count().unwrap(), 0);

        for i in 0..3 {
            let t = Thing { id: format!("c{}", i), name: "N".into(), count: i, rev: 0 };
            ops.save_new(t).unwrap();
        }
        assert_eq!(ops.count().unwrap(), 3);

        ops.delete("c1").unwrap();
        assert_eq!(ops.count().unwrap(), 2);
    }

    #[test]
    fn version_set_on_create() {
        let (ops, _dir) = make_ops();
        let t = Thing { id: "v1".into(), name: "A".into(), count: 1, rev: 0 };
        let created = ops.save_new(t).unwrap();
        assert_eq!(created.rev, 1, "rev should be 1 after create");

        let fetched = ops.get_or_err("v1").unwrap();
        assert_eq!(fetched.rev, 1, "re-read should also show rev 1");
    }

    #[test]
    fn version_bumped_on_update() {
        let (ops, _dir) = make_ops();
        let t = Thing { id: "v2".into(), name: "A".into(), count: 1, rev: 0 };
        ops.save_new(t).unwrap();

        let mut fetched = ops.get_or_err("v2").unwrap();
        assert_eq!(fetched.rev, 1);
        fetched.name = "B".into();
        let updated = ops.save(fetched).unwrap();
        assert_eq!(updated.rev, 2, "rev should be 2 after one update");

        let re_read = ops.get_or_err("v2").unwrap();
        assert_eq!(re_read.rev, 2);
        assert_eq!(re_read.name, "B");
    }

    #[test]
    fn version_conflict_returns_409() {
        let (ops, _dir) = make_ops();
        let t = Thing { id: "v3".into(), name: "A".into(), count: 1, rev: 0 };
        ops.save_new(t).unwrap();

        // Simulate two concurrent reads (both see version=1).
        let read1 = ops.get_or_err("v3").unwrap();
        let read2 = ops.get_or_err("v3").unwrap();
        assert_eq!(read1.rev, 1);
        assert_eq!(read2.rev, 1);

        // First write succeeds (version 1 -> 2).
        let mut w1 = read1;
        w1.name = "Updated by w1".into();
        let saved = ops.save(w1).unwrap();
        assert_eq!(saved.rev, 2);

        // Second write should fail (it has version 1, but DB now has version 2).
        let mut w2 = read2;
        w2.name = "Updated by w2".into();
        let err = ops.save(w2).unwrap_err();
        assert!(err.to_string().contains("rev mismatch"),
            "Expected rev conflict, got: {}", err);

        // Verify the data wasn't overwritten.
        let final_read = ops.get_or_err("v3").unwrap();
        assert_eq!(final_read.name, "Updated by w1");
        assert_eq!(final_read.rev, 2);
    }

    #[test]
    fn patch_partial_update() {
        let (ops, _dir) = make_ops();
        let t = Thing { id: "p1".into(), name: "Original".into(), count: 10, rev: 0 };
        let created = ops.save_new(t).unwrap();
        assert_eq!(created.rev, 1);

        // Patch: only change name, include rev for locking.
        let patch = serde_json::json!({ "name": "Patched", "rev": 1 });
        let patched = ops.patch("p1", &patch).unwrap();
        assert_eq!(patched.name, "Patched");
        assert_eq!(patched.count, 10, "count should be unchanged");
        assert_eq!(patched.rev, 2, "rev should be bumped");

        // Verify via re-read.
        let re_read = ops.get_or_err("p1").unwrap();
        assert_eq!(re_read.name, "Patched");
        assert_eq!(re_read.count, 10);
    }

    #[test]
    fn patch_without_rev_no_conflict() {
        let (ops, _dir) = make_ops();
        let t = Thing { id: "p2".into(), name: "A".into(), count: 1, rev: 0 };
        ops.save_new(t).unwrap();

        // Patch without rev field — no version check, still bumps rev.
        let patch = serde_json::json!({ "name": "B" });
        let patched = ops.patch("p2", &patch).unwrap();
        assert_eq!(patched.name, "B");
        assert_eq!(patched.rev, 2);
    }

    #[test]
    fn patch_stale_rev_returns_409() {
        let (ops, _dir) = make_ops();
        let t = Thing { id: "p3".into(), name: "A".into(), count: 1, rev: 0 };
        ops.save_new(t).unwrap();

        // First patch succeeds.
        let patch1 = serde_json::json!({ "name": "B", "rev": 1 });
        ops.patch("p3", &patch1).unwrap();

        // Second patch with stale rev=1 should fail.
        let patch2 = serde_json::json!({ "name": "C", "rev": 1 });
        let err = ops.patch("p3", &patch2).unwrap_err();
        assert!(err.to_string().contains("rev mismatch"),
            "Expected rev conflict, got: {}", err);
    }

    #[test]
    fn patch_nonexistent_returns_not_found() {
        let (ops, _dir) = make_ops();
        let patch = serde_json::json!({ "name": "X" });
        let err = ops.patch("ghost", &patch).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
