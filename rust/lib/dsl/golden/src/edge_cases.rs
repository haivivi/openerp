//! Edge case golden tests — validates framework robustness at boundaries.
//!
//! Tests that the DSL framework handles unusual inputs, edge conditions,
//! and corner cases correctly. These are the tests that catch bugs before
//! real users do.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use openerp_core::{Authenticator, ServiceError};
    use openerp_macro::model;
    use openerp_store::{
        admin_kv_router, apply_overrides, build_schema, HierarchyNode, KvOps, KvStore,
        ModuleDef, ResourceDef, WidgetOverride,
    };
    use openerp_types::*;

    // =====================================================================
    // Test model: deliberately minimal (only common fields + 1 user field)
    // =====================================================================

    #[model(module = "edge")]
    pub struct MinimalRecord {
        pub id: Id,
        // Only common fields auto-injected. No other user fields.
    }

    impl KvStore for MinimalRecord {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "edge:minimal:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
        fn before_update(&mut self) {
            self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
        }
    }

    // Model with all Option fields to test null handling.
    #[model(module = "edge")]
    pub struct AllOptional {
        pub id: Id,
        pub name: Option<String>,
        pub email: Option<Email>,
        pub url: Option<Url>,
        pub avatar: Option<Avatar>,
        pub secret: Option<Secret>,
        pub count: Option<u64>,
        pub active: Option<bool>,
        pub tags: Vec<String>,
    }

    impl KvStore for AllOptional {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "edge:allopt:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
        fn before_update(&mut self) {
            self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
        }
    }

    // Model with a validating hook.
    #[model(module = "edge")]
    pub struct ValidatedRecord {
        pub id: Id,
        pub status: String,
        pub priority: u32,
    }

    impl KvStore for ValidatedRecord {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "edge:validated:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
            if self.status.is_empty() { self.status = "open".into(); }
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
        fn before_update(&mut self) {
            self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
        }
    }

    fn make_router<T: KvStore + serde::Serialize + serde::de::DeserializeOwned>(
        path: &str, resource: &str,
    ) -> (Router<()>, Arc<dyn openerp_kv::KVStore>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("edge.redb")).unwrap(),
        );
        let auth: Arc<dyn Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(KvOps::<T>::new(kv.clone()), auth, "edge", path, resource);
        (router, kv, dir)
    }

    use axum::Router;

    async fn call(
        router: &Router, method: &str, uri: &str, body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if body.is_some() {
            builder = builder.header("content-type", "application/json");
        }
        let body = match body {
            Some(v) => Body::from(serde_json::to_string(&v).unwrap()),
            None => Body::empty(),
        };
        let req = builder.body(body).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let json = if bytes.is_empty() {
            serde_json::json!(null)
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::json!(null))
        };
        (status, json)
    }

    // =====================================================================
    // 1. Serde: Unicode in field values
    // =====================================================================

    #[tokio::test]
    async fn serde_unicode_fields() {
        let (router, _, _dir) = make_router::<AllOptional>("records", "record");

        let (s, created) = call(&router, "POST", "/records",
            Some(serde_json::json!({
                "name": "中文测试 — 企业名称",
                "email": "用户@例子.中国",
                "tags": ["标签一", "🚀 rocket", "café résumé"],
                "displayName": "日本語テスト",
                "description": "Ελληνικά العربية हिन्दी",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);
        let id = created["id"].as_str().unwrap();

        // Read back — all Unicode preserved.
        let (s, fetched) = call(&router, "GET", &format!("/records/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["name"], "中文测试 — 企业名称");
        assert_eq!(fetched["email"], "用户@例子.中国");
        assert_eq!(fetched["tags"][0], "标签一");
        assert_eq!(fetched["tags"][1], "🚀 rocket");
        assert_eq!(fetched["tags"][2], "café résumé");
        assert_eq!(fetched["displayName"], "日本語テスト");
        assert_eq!(fetched["description"], "Ελληνικά العربية हिन्दी");
    }

    // =====================================================================
    // 2. Serde: all-null optional fields
    // =====================================================================

    #[tokio::test]
    async fn serde_all_null_optionals() {
        let (router, _, _dir) = make_router::<AllOptional>("records", "record");

        // Create with only required field (nothing required except id which is auto).
        let (s, created) = call(&router, "POST", "/records", Some(serde_json::json!({}))).await;
        assert_eq!(s, StatusCode::OK);
        let id = created["id"].as_str().unwrap();
        assert!(!id.is_empty());

        // All optional fields should be null/default.
        assert!(created["name"].is_null());
        assert!(created["email"].is_null());
        assert!(created["url"].is_null());
        assert!(created["count"].is_null());
        // Vec defaults to empty.
        assert_eq!(created["tags"].as_array().unwrap().len(), 0);

        let (s, fetched) = call(&router, "GET", &format!("/records/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);
        assert!(fetched["name"].is_null());
    }

    // =====================================================================
    // 3. Serde: set field to null via PUT (clear optional value)
    // =====================================================================

    #[tokio::test]
    async fn serde_clear_optional_on_update() {
        let (router, _, _dir) = make_router::<AllOptional>("records", "record");

        // Create with values.
        let (s, created) = call(&router, "POST", "/records",
            Some(serde_json::json!({
                "name": "Has Value",
                "email": "test@test.com",
                "count": 42,
                "tags": ["a", "b"],
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);
        let id = created["id"].as_str().unwrap();
        assert_eq!(created["name"], "Has Value");

        // Update: set name to null, clear tags.
        let mut edit = created.clone();
        edit["name"] = serde_json::json!(null);
        edit["tags"] = serde_json::json!([]);
        edit["count"] = serde_json::json!(null);
        let (s, updated) = call(&router, "PUT", &format!("/records/{}", id), Some(edit)).await;
        assert_eq!(s, StatusCode::OK);
        assert!(updated["name"].is_null(), "name should be cleared to null");
        assert_eq!(updated["tags"].as_array().unwrap().len(), 0, "tags should be empty");
        assert!(updated["count"].is_null(), "count should be null");
        // Email should still be there.
        assert_eq!(updated["email"], "test@test.com");
    }

    // =====================================================================
    // 4. Special characters in ID
    // =====================================================================

    #[tokio::test]
    async fn special_chars_in_id() {
        let (router, _, _dir) = make_router::<ValidatedRecord>("items", "item");

        // Create with explicit ID containing special characters.
        let (s, created) = call(&router, "POST", "/items",
            Some(serde_json::json!({
                "id": "item:special/2026.01",
                "priority": 1,
                "displayName": "Special ID",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(created["id"], "item:special/2026.01");

        // URL-encoded GET.
        let (s, fetched) = call(&router, "GET", "/items/item:special%2F2026.01", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["id"], "item:special/2026.01");
    }

    // =====================================================================
    // 5. Minimal model: only id + common fields
    // =====================================================================

    #[tokio::test]
    async fn minimal_model_crud() {
        let (router, _, _dir) = make_router::<MinimalRecord>("records", "record");

        // Create with just displayName.
        let (s, created) = call(&router, "POST", "/records",
            Some(serde_json::json!({"displayName": "Minimal"})),
        ).await;
        assert_eq!(s, StatusCode::OK);
        let id = created["id"].as_str().unwrap();
        assert!(!id.is_empty());
        assert!(created["createdAt"].as_str().unwrap().contains("T"));

        // Has all common fields.
        assert_eq!(created["displayName"], "Minimal");
        assert!(created.get("description").is_some());
        assert!(created.get("metadata").is_some());
        assert!(created.get("createdAt").is_some());
        assert!(created.get("updatedAt").is_some());

        // Update displayName.
        let mut edit = created.clone();
        edit["displayName"] = serde_json::json!("Updated Minimal");
        let (s, updated) = call(&router, "PUT", &format!("/records/{}", id), Some(edit)).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(updated["displayName"], "Updated Minimal");

        // Delete.
        let (s, _) = call(&router, "DELETE", &format!("/records/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);
        let (s, _) = call(&router, "GET", &format!("/records/{}", id), None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }

    // =====================================================================
    // 6. Number edge cases: 0, max u64
    // =====================================================================

    #[tokio::test]
    async fn number_edge_cases() {
        let (router, _, _dir) = make_router::<AllOptional>("records", "record");

        // Zero.
        let (s, created) = call(&router, "POST", "/records",
            Some(serde_json::json!({"count": 0, "displayName": "Zero"})),
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(created["count"], 0);

        // Large number.
        let (s, created) = call(&router, "POST", "/records",
            Some(serde_json::json!({"count": 9_007_199_254_740_992u64, "displayName": "Big"})),
        ).await;
        assert_eq!(s, StatusCode::OK);
        let id = created["id"].as_str().unwrap();
        let (s, fetched) = call(&router, "GET", &format!("/records/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["count"], 9_007_199_254_740_992u64);
    }

    // =====================================================================
    // 7. Empty string vs null vs missing
    // =====================================================================

    #[tokio::test]
    async fn empty_vs_null_vs_missing() {
        let (router, _, _dir) = make_router::<AllOptional>("records", "record");

        // Explicit null.
        let (s, r1) = call(&router, "POST", "/records",
            Some(serde_json::json!({"name": null, "displayName": "Null name"})),
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert!(r1["name"].is_null());

        // Empty string.
        let (s, r2) = call(&router, "POST", "/records",
            Some(serde_json::json!({"name": "", "displayName": "Empty name"})),
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(r2["name"], "");

        // Missing field entirely.
        let (s, r3) = call(&router, "POST", "/records",
            Some(serde_json::json!({"displayName": "Missing name"})),
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert!(r3["name"].is_null());

        // Verify they're different records with different values.
        let id2 = r2["id"].as_str().unwrap();
        let (_, fetched) = call(&router, "GET", &format!("/records/{}", id2), None).await;
        assert_eq!(fetched["name"], "", "empty string preserved, not converted to null");
    }

    // =====================================================================
    // 8. PUT with mismatched ID in URL vs body
    // =====================================================================

    #[tokio::test]
    async fn put_id_mismatch() {
        let (router, _, _dir) = make_router::<ValidatedRecord>("items", "item");

        // Create two records.
        let (s, r1) = call(&router, "POST", "/items",
            Some(serde_json::json!({"id": "rec-1", "priority": 1, "displayName": "R1"})),
        ).await;
        assert_eq!(s, StatusCode::OK);
        let (s, _) = call(&router, "POST", "/items",
            Some(serde_json::json!({"id": "rec-2", "priority": 2, "displayName": "R2"})),
        ).await;
        assert_eq!(s, StatusCode::OK);

        // PUT to /items/rec-1 but body has id=rec-2.
        // Framework uses body's key_value() for storage, URL id only for existence check.
        let mut edit = r1.clone();
        edit["id"] = serde_json::json!("rec-2"); // mismatch!
        edit["displayName"] = serde_json::json!("Overwritten");
        let (s, _) = call(&router, "PUT", "/items/rec-1", Some(edit)).await;
        assert_eq!(s, StatusCode::OK);

        // Verify rec-1 still exists unchanged (URL id was checked for existence).
        let (s, check1) = call(&router, "GET", "/items/rec-1", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(check1["displayName"], "R1", "rec-1 should be unchanged");

        // rec-2 was overwritten by the body's id.
        let (s, check2) = call(&router, "GET", "/items/rec-2", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(check2["displayName"], "Overwritten");
    }

    // =====================================================================
    // 9. after_delete hook fires
    // =====================================================================

    use std::sync::atomic::{AtomicU32, Ordering};

    static DELETE_COUNTER: AtomicU32 = AtomicU32::new(0);

    #[model(module = "edge")]
    pub struct Tracked {
        pub id: Id,
        pub value: String,
    }

    impl KvStore for Tracked {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "edge:tracked:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
        fn after_delete(&self) {
            DELETE_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn after_delete_hook_fires() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("del.redb")).unwrap(),
        );
        let ops = KvOps::<Tracked>::new(kv);

        let before = DELETE_COUNTER.load(Ordering::SeqCst);

        let t = Tracked {
            id: Id::default(), value: "will be deleted".into(),
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        let created = ops.save_new(t).unwrap();
        ops.delete(created.id.as_str()).unwrap();

        let after = DELETE_COUNTER.load(Ordering::SeqCst);
        assert_eq!(after, before + 1, "after_delete should have been called once");
    }

    // =====================================================================
    // 10. Batch create 50 + delete all + list = 0
    // =====================================================================

    #[tokio::test]
    async fn batch_create_and_delete_all() {
        let (router, _, _dir) = make_router::<ValidatedRecord>("items", "item");

        let mut ids = Vec::new();
        for i in 0..50 {
            let (s, r) = call(&router, "POST", "/items",
                Some(serde_json::json!({
                    "priority": i,
                    "displayName": format!("Item {}", i),
                })),
            ).await;
            assert_eq!(s, StatusCode::OK);
            ids.push(r["id"].as_str().unwrap().to_string());
        }

        // List should have 50.
        let (s, list) = call(&router, "GET", "/items", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(list["total"], 50);

        // Delete all.
        for id in &ids {
            let (s, _) = call(&router, "DELETE", &format!("/items/{}", id), None).await;
            assert_eq!(s, StatusCode::OK);
        }

        // List should be empty.
        let (s, list) = call(&router, "GET", "/items", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(list["total"], 0);
        assert_eq!(list["items"].as_array().unwrap().len(), 0);
    }

    // =====================================================================
    // 11. apply_overrides on non-existent model/field (no-op)
    // =====================================================================

    #[test]
    fn apply_overrides_nonexistent_noop() {
        let mut schema = build_schema(
            "Test",
            vec![ModuleDef {
                id: "edge",
                label: "Edge",
                icon: "alert",
                resources: vec![
                    ResourceDef::from_ir("edge", MinimalRecord::__dsl_ir()),
                ],
                hierarchy: vec![HierarchyNode::leaf("minimal_record", "Records", "file", "")],
            }],
        );

        let before = schema.to_string();

        // Override targeting non-existent model.
        let overrides = vec![WidgetOverride {
            widget: "textarea".into(),
            apply_to: vec!["NonExistent.field".into()],
            params: serde_json::json!({"rows": 5}),
        }];
        apply_overrides(&mut schema, &overrides);

        // Schema unchanged.
        assert_eq!(schema.to_string(), before, "Non-existent model override should be no-op");

        // Override targeting non-existent field on existing model.
        let overrides = vec![WidgetOverride {
            widget: "textarea".into(),
            apply_to: vec!["MinimalRecord.nonexistent_field".into()],
            params: serde_json::json!({"rows": 5}),
        }];
        apply_overrides(&mut schema, &overrides);
        assert_eq!(schema.to_string(), before, "Non-existent field override should be no-op");
    }

    // =====================================================================
    // 12. Schema with module that has 0 resources
    // =====================================================================

    #[test]
    fn schema_empty_module() {
        let schema = build_schema(
            "EmptyTest",
            vec![
                ModuleDef {
                    id: "empty",
                    label: "Empty Module",
                    icon: "box",
                    resources: vec![],
                    hierarchy: vec![],
                },
                ModuleDef {
                    id: "full",
                    label: "Full",
                    icon: "cube",
                    resources: vec![ResourceDef::from_ir("full", MinimalRecord::__dsl_ir())],
                    hierarchy: vec![HierarchyNode::leaf("minimal_record", "Records", "file", "")],
                },
            ],
        );

        let modules = schema["modules"].as_array().unwrap();
        assert_eq!(modules.len(), 2);
        assert_eq!(modules[0]["resources"].as_array().unwrap().len(), 0);
        assert_eq!(modules[1]["resources"].as_array().unwrap().len(), 1);

        // Empty module has no permissions.
        assert_eq!(schema["permissions"]["empty"].as_object().unwrap().len(), 0);
        assert!(schema["permissions"]["full"].as_object().unwrap().len() > 0);
    }

    // =====================================================================
    // 13. Very long string values
    // =====================================================================

    #[tokio::test]
    async fn very_long_string_values() {
        let (router, _, _dir) = make_router::<AllOptional>("records", "record");

        let long_name = "A".repeat(10_000);
        let long_tags: Vec<String> = (0..100).map(|i| format!("tag-{:04}", i)).collect();

        let (s, created) = call(&router, "POST", "/records",
            Some(serde_json::json!({
                "name": long_name,
                "tags": long_tags,
                "displayName": "Long Test",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(created["name"].as_str().unwrap().len(), 10_000);
        assert_eq!(created["tags"].as_array().unwrap().len(), 100);

        // Roundtrip.
        let id = created["id"].as_str().unwrap();
        let (s, fetched) = call(&router, "GET", &format!("/records/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["name"].as_str().unwrap().len(), 10_000);
        assert_eq!(fetched["tags"].as_array().unwrap().len(), 100);
    }

    // =====================================================================
    // 14. POST with explicit ID (no auto-generate)
    // =====================================================================

    #[tokio::test]
    async fn explicit_id_no_auto_generate() {
        let (router, _, _dir) = make_router::<ValidatedRecord>("items", "item");

        let (s, created) = call(&router, "POST", "/items",
            Some(serde_json::json!({
                "id": "my-custom-id-123",
                "priority": 5,
                "displayName": "Custom ID",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(created["id"], "my-custom-id-123", "explicit ID should be preserved");

        // Can fetch by that ID.
        let (s, fetched) = call(&router, "GET", "/items/my-custom-id-123", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["displayName"], "Custom ID");
    }

    // =====================================================================
    // 15. Create → immediately read → all fields match
    // =====================================================================

    #[tokio::test]
    async fn create_read_consistency() {
        let (router, _, _dir) = make_router::<AllOptional>("records", "record");

        let input = serde_json::json!({
            "name": "Consistency Check",
            "email": "check@test.com",
            "url": "https://example.com/resource",
            "count": 999,
            "active": true,
            "tags": ["alpha", "beta", "gamma"],
            "displayName": "Check Record",
            "description": "Testing read-after-write consistency",
        });

        let (s, created) = call(&router, "POST", "/records", Some(input.clone())).await;
        assert_eq!(s, StatusCode::OK);
        let id = created["id"].as_str().unwrap();

        let (s, fetched) = call(&router, "GET", &format!("/records/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);

        // Every input field matches.
        assert_eq!(fetched["name"], "Consistency Check");
        assert_eq!(fetched["email"], "check@test.com");
        assert_eq!(fetched["url"], "https://example.com/resource");
        assert_eq!(fetched["count"], 999);
        assert_eq!(fetched["active"], true);
        assert_eq!(fetched["tags"][0], "alpha");
        assert_eq!(fetched["tags"][1], "beta");
        assert_eq!(fetched["tags"][2], "gamma");
        assert_eq!(fetched["displayName"], "Check Record");
        assert_eq!(fetched["description"], "Testing read-after-write consistency");
        // Auto-filled.
        assert_eq!(fetched["id"], id);
        assert!(fetched["createdAt"].as_str().unwrap().len() > 10);
        assert!(fetched["updatedAt"].as_str().unwrap().len() > 10);
    }

    // =====================================================================
    // 16. Role permissions updated → takes effect immediately
    // =====================================================================

    // Local Role model + MiniAuth for this test (avoids cross-module dependency).
    #[model(module = "edge")]
    pub struct LocalRole {
        pub id: Id,
        pub permissions: Vec<String>,
    }

    impl KvStore for LocalRole {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "edge:localrole:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
    }

    struct LocalAuth { kv: Arc<dyn openerp_kv::KVStore> }

    impl Authenticator for LocalAuth {
        fn check(&self, headers: &axum::http::HeaderMap, permission: &str) -> Result<(), ServiceError> {
            let roles = headers.get("x-roles").and_then(|v| v.to_str().ok())
                .ok_or_else(|| ServiceError::Unauthorized("missing x-roles".into()))?;
            if roles == "root" { return Ok(()); }
            let ops = KvOps::<LocalRole>::new(self.kv.clone());
            for rid in roles.split(',').map(|s| s.trim()) {
                if let Ok(Some(r)) = ops.get(rid) {
                    if r.permissions.iter().any(|p| p == permission) { return Ok(()); }
                }
            }
            Err(ServiceError::PermissionDenied(format!("denied: {}", permission)))
        }
    }

    #[tokio::test]
    async fn role_permission_change_takes_effect() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("dynrole.redb")).unwrap(),
        );

        let auth: Arc<dyn Authenticator> = Arc::new(LocalAuth { kv: kv.clone() });
        let router = admin_kv_router(
            KvOps::<ValidatedRecord>::new(kv.clone()), auth, "edge", "items", "item",
        );
        let role_ops = KvOps::<LocalRole>::new(kv.clone());

        // Create role with only list permission.
        role_ops.save_new(LocalRole {
            id: Id::new("dynamic"),
            permissions: vec!["edge:item:list".into()],
            display_name: Some("Dynamic".into()),
            description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();

        // User with "dynamic" role can list but not create.
        let req = Request::builder().uri("/items").header("x-roles", "dynamic").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let req = Request::builder().method("POST").uri("/items")
            .header("x-roles", "dynamic").header("content-type", "application/json")
            .body(Body::from(r#"{"priority":1,"displayName":"X"}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "No create permission yet");

        // Update role: add create permission.
        let mut role = role_ops.get_or_err("dynamic").unwrap();
        role.permissions.push("edge:item:create".into());
        role_ops.save(role).unwrap();

        // Now the same user can create (permission change takes effect immediately).
        let req = Request::builder().method("POST").uri("/items")
            .header("x-roles", "dynamic").header("content-type", "application/json")
            .body(Body::from(r#"{"priority":1,"displayName":"Now works"}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Create should work after role update");
    }

    // =====================================================================
    // 18. Metadata JSON object roundtrip
    // =====================================================================

    #[tokio::test]
    async fn metadata_json_string_roundtrip() {
        let (router, _, _dir) = make_router::<AllOptional>("records", "record");

        // metadata is Option<String>, so we store JSON as a serialized string.
        let meta_obj = serde_json::json!({
            "source": "import", "version": 3,
            "nested": {"key": "value", "array": [1, 2, 3]},
        });
        let meta_str = serde_json::to_string(&meta_obj).unwrap();

        let (s, created) = call(&router, "POST", "/records",
            Some(serde_json::json!({
                "name": "With Metadata",
                "metadata": meta_str,
                "displayName": "Meta Test",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);
        let id = created["id"].as_str().unwrap();
        assert_eq!(created["metadata"].as_str().unwrap(), meta_str);

        // Read back — metadata string preserved exactly.
        let (s, fetched) = call(&router, "GET", &format!("/records/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);
        let fetched_meta: serde_json::Value = serde_json::from_str(fetched["metadata"].as_str().unwrap()).unwrap();
        assert_eq!(fetched_meta["source"], "import");
        assert_eq!(fetched_meta["version"], 3);
        assert_eq!(fetched_meta["nested"]["array"][2], 3);

        // Update metadata.
        let mut edit = fetched.clone();
        let new_meta = serde_json::json!({"source": "updated", "version": 4});
        edit["metadata"] = serde_json::json!(serde_json::to_string(&new_meta).unwrap());
        let (s, updated) = call(&router, "PUT", &format!("/records/{}", id), Some(edit)).await;
        assert_eq!(s, StatusCode::OK);
        let upd_meta: serde_json::Value = serde_json::from_str(updated["metadata"].as_str().unwrap()).unwrap();
        assert_eq!(upd_meta["version"], 4);
    }

    // =====================================================================
    // 19. Unknown fields in POST body silently dropped
    // =====================================================================

    #[tokio::test]
    async fn unknown_fields_dropped() {
        let (router, _, _dir) = make_router::<ValidatedRecord>("items", "item");

        let (s, created) = call(&router, "POST", "/items",
            Some(serde_json::json!({
                "priority": 5,
                "displayName": "Known",
                "unknownField1": "should be dropped",
                "nonExistent": 12345,
                "extra": {"nested": true},
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(created["priority"], 5);
        assert_eq!(created["displayName"], "Known");
        // Unknown fields should not appear in response.
        assert!(created.get("unknownField1").is_none() || created["unknownField1"].is_null(),
            "unknownField1 should be dropped");
        assert!(created.get("nonExistent").is_none() || created["nonExistent"].is_null(),
            "nonExistent should be dropped");
    }

    // =====================================================================
    // 20. Delete then recreate with same ID
    // =====================================================================

    #[tokio::test]
    async fn delete_and_recreate_same_id() {
        let (router, _, _dir) = make_router::<ValidatedRecord>("items", "item");

        // Create with explicit ID.
        let (s, _) = call(&router, "POST", "/items",
            Some(serde_json::json!({"id": "reuse-me", "priority": 1, "displayName": "V1"})),
        ).await;
        assert_eq!(s, StatusCode::OK);

        // Delete.
        let (s, _) = call(&router, "DELETE", "/items/reuse-me", None).await;
        assert_eq!(s, StatusCode::OK);

        // Verify deleted.
        let (s, _) = call(&router, "GET", "/items/reuse-me", None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);

        // Recreate with same ID but different data.
        let (s, v2) = call(&router, "POST", "/items",
            Some(serde_json::json!({"id": "reuse-me", "priority": 99, "displayName": "V2"})),
        ).await;
        assert_eq!(s, StatusCode::OK, "Should allow recreate after delete");
        assert_eq!(v2["id"], "reuse-me");
        assert_eq!(v2["priority"], 99);
        assert_eq!(v2["displayName"], "V2");

        // Verify only V2 data exists.
        let (s, fetched) = call(&router, "GET", "/items/reuse-me", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["priority"], 99, "New data, not old data");
        assert_eq!(fetched["displayName"], "V2");

        // List should have exactly 1.
        let (s, list) = call(&router, "GET", "/items", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(list["total"], 1);
    }

    // =====================================================================
    // 21. Schema field order matches struct definition
    // =====================================================================

    #[test]
    fn schema_field_order_matches_struct() {
        let ir = AllOptional::__dsl_ir();
        let fields = ir["fields"].as_array().unwrap();
        let names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();

        // User fields should come first, in definition order.
        assert_eq!(names[0], "id");
        assert_eq!(names[1], "name");
        assert_eq!(names[2], "email");
        assert_eq!(names[3], "url");
        assert_eq!(names[4], "avatar");
        assert_eq!(names[5], "secret");
        assert_eq!(names[6], "count");
        assert_eq!(names[7], "active");
        assert_eq!(names[8], "tags");
        // Then common fields (injected by #[model]).
        assert_eq!(names[9], "display_name");
        assert_eq!(names[10], "description");
        assert_eq!(names[11], "metadata");
        assert_eq!(names[12], "created_at");
        assert_eq!(names[13], "updated_at");
    }

    // =====================================================================
    // 22. PUT change every single field at once
    // =====================================================================

    #[tokio::test]
    async fn put_change_every_field() {
        let (router, _, _dir) = make_router::<AllOptional>("records", "record");

        let (s, created) = call(&router, "POST", "/records",
            Some(serde_json::json!({
                "name": "Original",
                "email": "old@test.com",
                "url": "https://old.com",
                "count": 1,
                "active": false,
                "tags": ["old"],
                "displayName": "Old Name",
                "description": "Old desc",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);
        let id = created["id"].as_str().unwrap();

        // Change EVERY field.
        let mut edit = created.clone();
        edit["name"] = serde_json::json!("Changed");
        edit["email"] = serde_json::json!("new@test.com");
        edit["url"] = serde_json::json!("https://new.com");
        edit["count"] = serde_json::json!(999);
        edit["active"] = serde_json::json!(true);
        edit["tags"] = serde_json::json!(["new", "changed"]);
        edit["displayName"] = serde_json::json!("New Name");
        edit["description"] = serde_json::json!("New desc");

        let (s, updated) = call(&router, "PUT", &format!("/records/{}", id), Some(edit)).await;
        assert_eq!(s, StatusCode::OK);

        // Verify ALL fields changed.
        assert_eq!(updated["name"], "Changed");
        assert_eq!(updated["email"], "new@test.com");
        assert_eq!(updated["url"], "https://new.com");
        assert_eq!(updated["count"], 999);
        assert_eq!(updated["active"], true);
        assert_eq!(updated["tags"][0], "new");
        assert_eq!(updated["tags"][1], "changed");
        assert_eq!(updated["displayName"], "New Name");
        assert_eq!(updated["description"], "New desc");
        // id unchanged.
        assert_eq!(updated["id"], id);

        // Re-fetch to confirm persistence.
        let (s, fetched) = call(&router, "GET", &format!("/records/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["name"], "Changed");
        assert_eq!(fetched["active"], true);
        assert_eq!(fetched["count"], 999);
    }

    // =====================================================================
    // 23. Bool field: default false, set true, toggle back to false
    // =====================================================================

    #[tokio::test]
    async fn bool_field_toggle() {
        let (router, _, _dir) = make_router::<AllOptional>("records", "record");

        // Create without active → defaults to null (Option<bool>).
        let (s, created) = call(&router, "POST", "/records",
            Some(serde_json::json!({"displayName": "Bool Test"})),
        ).await;
        assert_eq!(s, StatusCode::OK);
        let id = created["id"].as_str().unwrap();
        assert!(created["active"].is_null(), "Option<bool> default is null");

        // Set to true.
        let mut edit = created.clone();
        edit["active"] = serde_json::json!(true);
        let (s, u1) = call(&router, "PUT", &format!("/records/{}", id), Some(edit)).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(u1["active"], true);

        // Toggle to false.
        let mut edit = u1.clone();
        edit["active"] = serde_json::json!(false);
        let (s, u2) = call(&router, "PUT", &format!("/records/{}", id), Some(edit)).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(u2["active"], false, "false should be stored, not treated as null");

        // Verify false persisted.
        let (s, fetched) = call(&router, "GET", &format!("/records/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["active"], false);
    }

    // =====================================================================
    // 17. Deeply nested hierarchy
    // =====================================================================

    #[test]
    fn deeply_nested_hierarchy() {
        let tree = HierarchyNode {
            resource: "root", label: "Root", icon: "folder", description: "",
            children: vec![
                HierarchyNode {
                    resource: "level1", label: "Level 1", icon: "folder", description: "",
                    children: vec![
                        HierarchyNode {
                            resource: "level2", label: "Level 2", icon: "folder", description: "",
                            children: vec![
                                HierarchyNode::leaf("level3", "Level 3", "file", "Deep leaf"),
                            ],
                        },
                    ],
                },
            ],
        };

        let json = tree.to_json();
        assert_eq!(json["resource"], "root");
        assert_eq!(json["children"][0]["resource"], "level1");
        assert_eq!(json["children"][0]["children"][0]["resource"], "level2");
        assert_eq!(json["children"][0]["children"][0]["children"][0]["resource"], "level3");
        assert_eq!(json["children"][0]["children"][0]["children"][0]["description"], "Deep leaf");
    }
}
