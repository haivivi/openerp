use serde::{Deserialize, Serialize};

/// Parameters for list/query operations.
#[derive(Debug, Clone, Deserialize)]
pub struct ListParams {
    /// Maximum number of results to return.
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Offset for pagination.
    #[serde(default)]
    pub offset: usize,

    /// Sort field.
    #[serde(default)]
    pub sort: Option<String>,

    /// Search query string (for full-text search).
    #[serde(default)]
    pub q: Option<String>,
}

fn default_limit() -> usize {
    50
}

impl Default for ListParams {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            offset: 0,
            sort: None,
            q: None,
        }
    }
}

/// Result wrapper for list operations.
#[derive(Debug, Clone, Serialize)]
pub struct ListResult<T: Serialize> {
    pub items: Vec<T>,
    pub total: usize,
}

/// Generate a new random ID (UUIDv4, no dashes).
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}

/// Get the current time as an RFC 3339 string.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Merge a JSON patch into a base value.
///
/// For each key in `patch`:
/// - If the value is `null`, the key is removed from `base`.
/// - Otherwise, the key is set to the patch value.
///
/// This follows RFC 7386 (JSON Merge Patch) semantics.
pub fn merge_patch(
    base: &mut serde_json::Value,
    patch: &serde_json::Value,
) {
    if let (Some(base_obj), Some(patch_obj)) = (base.as_object_mut(), patch.as_object()) {
        for (key, value) in patch_obj {
            if value.is_null() {
                base_obj.remove(key);
            } else if value.is_object() {
                // Recursively merge nested objects.
                let entry = base_obj
                    .entry(key.clone())
                    .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                merge_patch(entry, value);
            } else {
                base_obj.insert(key.clone(), value.clone());
            }
        }
    } else {
        *base = patch.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_id() {
        let id = new_id();
        assert_eq!(id.len(), 32);
        assert!(!id.contains('-'));
    }

    #[test]
    fn test_now_rfc3339() {
        let ts = now_rfc3339();
        assert!(ts.contains('T'));
    }

    #[test]
    fn test_merge_patch() {
        let mut base = serde_json::json!({"a": 1, "b": 2, "c": {"d": 3}});
        let patch = serde_json::json!({"b": null, "c": {"e": 4}, "f": 5});
        merge_patch(&mut base, &patch);
        assert_eq!(
            base,
            serde_json::json!({"a": 1, "c": {"d": 3, "e": 4}, "f": 5})
        );
    }
}
