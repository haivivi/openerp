//! Centralized timestamp management for store operations.
//!
//! `stamp_create` and `stamp_update` inject `createdAt`/`updatedAt` into
//! serialized JSON objects. Used by both KvOps and SqlOps so the logic
//! lives in one place.

/// Stamp `createdAt` (if empty) and `updatedAt` on a JSON object.
pub(crate) fn stamp_create(val: &mut serde_json::Value) {
    if let Some(obj) = val.as_object_mut() {
        let now = chrono::Utc::now().to_rfc3339();
        let ca = obj.get("createdAt").and_then(|v| v.as_str()).unwrap_or("");
        if ca.is_empty() {
            obj.insert("createdAt".into(), serde_json::json!(now));
        }
        obj.insert("updatedAt".into(), serde_json::json!(now));
    }
}

/// Stamp a fresh `updatedAt` on a JSON object.
pub(crate) fn stamp_update(val: &mut serde_json::Value) {
    if let Some(obj) = val.as_object_mut() {
        obj.insert(
            "updatedAt".into(),
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        );
    }
}
