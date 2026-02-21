//! Golden tests for admin_sql_router — SQL-backed CRUD handlers.
//!
//! Covers:
//!   - Single PK: full CRUD lifecycle, pagination, count, optimistic locking, PATCH
//!   - Compound PK: CRUD with multi-segment URL paths
//!   - ensure_table: first call auto-creates table
//!   - Error responses: 404, 409, 400 (symmetric with KV router)
//!   - Auth: AllowAll vs DenyAll
//!   - Symmetry with admin_kv_router behavior

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use openerp_macro::model;
    use openerp_store::{admin_sql_router, SqlOps, SqlStore};
    use openerp_types::*;

    // =====================================================================
    // Test models
    // =====================================================================

    #[model(module = "test")]
    pub struct SqlWidget {
        pub id: Id,
        pub label: String,
        pub count: u32,
        pub active: bool,
    }

    impl SqlStore for SqlWidget {
        const PK: &[Field] = &[Field::new("id", "Id", "readonly")];

        fn table_name() -> &'static str {
            "sql_widgets"
        }

        fn pk_values(&self) -> Vec<String> {
            vec![self.id.to_string()]
        }

        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
        }
    }

    #[model(module = "test")]
    pub struct CompoundItem {
        pub region: String,
        pub sku: String,
        pub quantity: u32,
    }

    impl SqlStore for CompoundItem {
        const PK: &[Field] = &[
            Field::new("region", "String", "text"),
            Field::new("sku", "String", "text"),
        ];

        fn table_name() -> &'static str {
            "compound_items"
        }

        fn pk_values(&self) -> Vec<String> {
            vec![self.region.clone(), self.sku.clone()]
        }
    }

    // =====================================================================
    // Helpers
    // =====================================================================

    fn make_sql_ops<T: SqlStore>() -> (SqlOps<T>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let sql: Arc<dyn openerp_sql::SQLStore> =
            Arc::new(openerp_sql::SqliteStore::open(&dir.path().join("test.db")).unwrap());
        let ops = SqlOps::new(sql);
        ops.ensure_table().unwrap();
        (ops, dir)
    }

    fn make_sql_ops_no_ensure<T: SqlStore>() -> (SqlOps<T>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let sql: Arc<dyn openerp_sql::SQLStore> =
            Arc::new(openerp_sql::SqliteStore::open(&dir.path().join("test.db")).unwrap());
        let ops = SqlOps::new(sql);
        (ops, dir)
    }

    async fn api(
        router: &axum::Router,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
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
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json = if bytes.is_empty() {
            serde_json::json!(null)
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::json!(null))
        };
        (status, json)
    }

    fn widget_router(ops: SqlOps<SqlWidget>) -> axum::Router {
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        admin_sql_router(ops, auth, "test", "widgets", "widget")
    }

    fn compound_router(ops: SqlOps<CompoundItem>) -> axum::Router {
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        admin_sql_router(ops, auth, "test", "items", "item")
    }

    // =====================================================================
    // 1. Empty list
    // =====================================================================

    #[tokio::test]
    async fn sql_list_empty() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (s, json) = api(&r, "GET", "/widgets", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(json["items"].as_array().unwrap().len(), 0);
        assert_eq!(json["hasMore"], false);
    }

    // =====================================================================
    // 2. Create + auto-generated fields
    // =====================================================================

    #[tokio::test]
    async fn sql_create_success() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (s, json) = api(
            &r,
            "POST",
            "/widgets",
            Some(serde_json::json!({"label": "Alpha", "count": 42, "active": true})),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert!(!json["id"].as_str().unwrap().is_empty(), "id should be auto-generated");
        assert!(json["createdAt"].as_str().unwrap().contains("T"));
        assert!(json["updatedAt"].as_str().unwrap().contains("T"));
        assert_eq!(json["label"], "Alpha");
        assert_eq!(json["count"], 42);
    }

    // =====================================================================
    // 3. Duplicate PK → 409
    // =====================================================================

    #[tokio::test]
    async fn sql_create_duplicate_pk() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let body = serde_json::json!({"id": "dup1", "label": "A", "count": 1});
        let (s, _) = api(&r, "POST", "/widgets", Some(body.clone())).await;
        assert_eq!(s, StatusCode::OK);

        let (s, err) = api(&r, "POST", "/widgets", Some(body)).await;
        assert!(
            s == StatusCode::CONFLICT || s.is_client_error(),
            "duplicate PK should fail, got {}",
            s
        );
        if s == StatusCode::CONFLICT {
            assert!(err["message"].as_str().unwrap_or("").contains("UNIQUE")
                || err["code"].as_str().unwrap_or("") == "ALREADY_EXISTS"
                || err["code"].as_str().unwrap_or("") == "STORAGE_ERROR");
        }
    }

    // =====================================================================
    // 4. GET by id
    // =====================================================================

    #[tokio::test]
    async fn sql_get_by_id() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (_, created) = api(
            &r,
            "POST",
            "/widgets",
            Some(serde_json::json!({"label": "Bravo", "count": 7})),
        )
        .await;
        let id = created["id"].as_str().unwrap();

        let (s, fetched) = api(&r, "GET", &format!("/widgets/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["label"], "Bravo");
        assert_eq!(fetched["count"], 7);
    }

    // =====================================================================
    // 5. GET non-existent → 404
    // =====================================================================

    #[tokio::test]
    async fn sql_get_not_found() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (s, err) = api(&r, "GET", "/widgets/ghost", None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        assert_eq!(err["code"], "NOT_FOUND");
    }

    // =====================================================================
    // 6. PUT update
    // =====================================================================

    #[tokio::test]
    async fn sql_put_update() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (_, created) = api(
            &r,
            "POST",
            "/widgets",
            Some(serde_json::json!({"label": "Charlie", "count": 1})),
        )
        .await;
        let id = created["id"].as_str().unwrap();
        let old_ts = created["updatedAt"].as_str().unwrap().to_string();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut edit = created.clone();
        edit["label"] = serde_json::json!("Charlie Updated");
        edit["count"] = serde_json::json!(99);
        let (s, updated) = api(&r, "PUT", &format!("/widgets/{}", id), Some(edit)).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(updated["label"], "Charlie Updated");
        assert_eq!(updated["count"], 99);
        assert_ne!(updated["updatedAt"].as_str().unwrap(), &old_ts);
    }

    // =====================================================================
    // 7. PUT non-existent → 404
    // =====================================================================

    #[tokio::test]
    async fn sql_put_not_found() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let body = serde_json::json!({"id": "ghost", "label": "X", "count": 0});
        let (s, _) = api(&r, "PUT", "/widgets/ghost", Some(body)).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }

    // =====================================================================
    // 8. PUT optimistic lock → 409
    // =====================================================================

    #[tokio::test]
    async fn sql_put_optimistic_lock() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (_, created) = api(
            &r,
            "POST",
            "/widgets",
            Some(serde_json::json!({"label": "Lock", "count": 1})),
        )
        .await;
        let id = created["id"].as_str().unwrap();

        let client_a = created.clone();
        let client_b = created.clone();

        let mut update_a = client_a;
        update_a["label"] = serde_json::json!("A wins");
        let (s, _) = api(&r, "PUT", &format!("/widgets/{}", id), Some(update_a)).await;
        assert_eq!(s, StatusCode::OK);

        let mut update_b = client_b;
        update_b["label"] = serde_json::json!("B loses");
        let (s, _) = api(&r, "PUT", &format!("/widgets/{}", id), Some(update_b)).await;
        assert_eq!(s, StatusCode::CONFLICT);

        let (_, final_state) = api(&r, "GET", &format!("/widgets/{}", id), None).await;
        assert_eq!(final_state["label"], "A wins");
    }

    // =====================================================================
    // 9. PATCH partial update
    // =====================================================================

    #[tokio::test]
    async fn sql_patch_partial() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (_, created) = api(
            &r,
            "POST",
            "/widgets",
            Some(serde_json::json!({"label": "Patch Me", "count": 10, "active": true})),
        )
        .await;
        let id = created["id"].as_str().unwrap();
        let ts = created["updatedAt"].as_str().unwrap();

        let patch = serde_json::json!({"label": "Patched", "updatedAt": ts});
        let (s, patched) = api(&r, "PATCH", &format!("/widgets/{}", id), Some(patch)).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(patched["label"], "Patched");
        assert_eq!(patched["count"], 10, "unchanged field preserved");
        assert_eq!(patched["active"], true, "unchanged field preserved");
        assert_ne!(patched["updatedAt"].as_str().unwrap(), ts);
    }

    // =====================================================================
    // 10. PATCH stale updatedAt → 409
    // =====================================================================

    #[tokio::test]
    async fn sql_patch_stale_conflict() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (_, created) = api(
            &r,
            "POST",
            "/widgets",
            Some(serde_json::json!({"label": "Stale", "count": 1})),
        )
        .await;
        let id = created["id"].as_str().unwrap();
        let old_ts = created["updatedAt"].as_str().unwrap().to_string();

        let patch1 = serde_json::json!({"label": "Fresh", "updatedAt": &old_ts});
        let (s, _) = api(&r, "PATCH", &format!("/widgets/{}", id), Some(patch1)).await;
        assert_eq!(s, StatusCode::OK);

        let patch2 = serde_json::json!({"label": "Stale", "updatedAt": &old_ts});
        let (s, _) = api(&r, "PATCH", &format!("/widgets/{}", id), Some(patch2)).await;
        assert_eq!(s, StatusCode::CONFLICT);
    }

    // =====================================================================
    // 11. PATCH without updatedAt → no conflict
    // =====================================================================

    #[tokio::test]
    async fn sql_patch_no_ts_no_conflict() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (_, created) = api(
            &r,
            "POST",
            "/widgets",
            Some(serde_json::json!({"label": "NoTs", "count": 1})),
        )
        .await;
        let id = created["id"].as_str().unwrap();

        let (s, patched) =
            api(&r, "PATCH", &format!("/widgets/{}", id), Some(serde_json::json!({"count": 99}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(patched["count"], 99);
    }

    // =====================================================================
    // 12. PATCH non-existent → 404
    // =====================================================================

    #[tokio::test]
    async fn sql_patch_not_found() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (s, _) = api(
            &r,
            "PATCH",
            "/widgets/ghost",
            Some(serde_json::json!({"label": "X"})),
        )
        .await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }

    // =====================================================================
    // 13. DELETE
    // =====================================================================

    #[tokio::test]
    async fn sql_delete_success() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (_, created) = api(
            &r,
            "POST",
            "/widgets",
            Some(serde_json::json!({"label": "Del", "count": 1})),
        )
        .await;
        let id = created["id"].as_str().unwrap();

        let (s, _) = api(&r, "DELETE", &format!("/widgets/{}", id), None).await;
        assert_eq!(s, StatusCode::OK);

        let (s, _) = api(&r, "GET", &format!("/widgets/{}", id), None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }

    // =====================================================================
    // 14. DELETE non-existent → 404
    // =====================================================================

    #[tokio::test]
    async fn sql_delete_not_found() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);
        let (s, _) = api(&r, "DELETE", "/widgets/ghost", None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }

    // =====================================================================
    // 15. Count
    // =====================================================================

    #[tokio::test]
    async fn sql_count() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);

        let (s, json) = api(&r, "GET", "/widgets/@count", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(json["count"], 0);

        for i in 0..3 {
            api(
                &r,
                "POST",
                "/widgets",
                Some(serde_json::json!({"id": format!("c{}", i), "label": "X", "count": i})),
            )
            .await;
        }

        let (s, json) = api(&r, "GET", "/widgets/@count", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(json["count"], 3);
    }

    // =====================================================================
    // 16-17. Pagination
    // =====================================================================

    #[tokio::test]
    async fn sql_pagination() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);

        for i in 0..5 {
            api(
                &r,
                "POST",
                "/widgets",
                Some(serde_json::json!({"id": format!("pg{}", i), "label": format!("W{}", i), "count": i})),
            )
            .await;
        }

        let (s, json) = api(&r, "GET", "/widgets?limit=2&offset=0", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(json["items"].as_array().unwrap().len(), 2);
        assert_eq!(json["hasMore"], true);

        let (_, json) = api(&r, "GET", "/widgets?limit=2&offset=4", None).await;
        assert_eq!(json["items"].as_array().unwrap().len(), 1);
        assert_eq!(json["hasMore"], false);
    }

    // =====================================================================
    // 18. Compound PK: CRUD
    // =====================================================================

    #[tokio::test]
    async fn sql_compound_pk_crud() {
        let (ops, _dir) = make_sql_ops::<CompoundItem>();
        let r = compound_router(ops);

        let (s, created) = api(
            &r,
            "POST",
            "/items",
            Some(serde_json::json!({"region": "us-east", "sku": "ABC-001", "quantity": 100})),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(created["region"], "us-east");
        assert_eq!(created["sku"], "ABC-001");

        let (s, fetched) = api(&r, "GET", "/items/us-east/ABC-001", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["quantity"], 100);

        let (s, _) = api(&r, "DELETE", "/items/us-east/ABC-001", None).await;
        assert_eq!(s, StatusCode::OK);

        let (s, _) = api(&r, "GET", "/items/us-east/ABC-001", None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }

    // =====================================================================
    // 19. Compound PK: PATCH
    // =====================================================================

    #[tokio::test]
    async fn sql_compound_pk_patch() {
        let (ops, _dir) = make_sql_ops::<CompoundItem>();
        let r = compound_router(ops);

        let (_, created) = api(
            &r,
            "POST",
            "/items",
            Some(serde_json::json!({"region": "eu-west", "sku": "XYZ-999", "quantity": 50})),
        )
        .await;
        let ts = created["updatedAt"].as_str().unwrap();

        let (s, patched) = api(
            &r,
            "PATCH",
            "/items/eu-west/XYZ-999",
            Some(serde_json::json!({"quantity": 75, "updatedAt": ts})),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(patched["quantity"], 75);
        assert_eq!(patched["region"], "eu-west");
    }

    // =====================================================================
    // 20. ensure_table auto-creates table
    // =====================================================================

    #[tokio::test]
    async fn sql_ensure_table_auto_create() {
        let (ops, _dir) = make_sql_ops_no_ensure::<SqlWidget>();
        ops.ensure_table().unwrap();
        let r = widget_router(ops);

        let (s, _) = api(
            &r,
            "POST",
            "/widgets",
            Some(serde_json::json!({"label": "Auto", "count": 1})),
        )
        .await;
        assert_eq!(s, StatusCode::OK);

        let (s, json) = api(&r, "GET", "/widgets/@count", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(json["count"], 1);
    }

    // =====================================================================
    // 21. UNIQUE index violation → error
    // =====================================================================

    #[model(module = "test")]
    pub struct UniqueWidget {
        pub id: Id,
        pub code: String,
    }

    impl SqlStore for UniqueWidget {
        const PK: &[Field] = &[Field::new("id", "Id", "readonly")];
        const UNIQUE: &[&[Field]] = &[&[Field::new("code", "String", "text")]];

        fn table_name() -> &'static str {
            "unique_widgets"
        }

        fn pk_values(&self) -> Vec<String> {
            vec![self.id.to_string()]
        }

        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
        }
    }

    #[tokio::test]
    async fn sql_unique_violation() {
        let (ops, _dir) = make_sql_ops::<UniqueWidget>();
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let r = admin_sql_router(ops, auth, "test", "unique_widgets", "unique_widget");

        let (s, _) = api(
            &r,
            "POST",
            "/unique_widgets",
            Some(serde_json::json!({"code": "UNIQ-001"})),
        )
        .await;
        assert_eq!(s, StatusCode::OK);

        let (s, _) = api(
            &r,
            "POST",
            "/unique_widgets",
            Some(serde_json::json!({"code": "UNIQ-001"})),
        )
        .await;
        assert!(s.is_client_error(), "duplicate UNIQUE should fail, got {}", s);
    }

    // =====================================================================
    // 22-23. Auth: AllowAll vs DenyAll
    // =====================================================================

    #[tokio::test]
    async fn sql_auth_allow_all() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let r = admin_sql_router(ops, auth, "test", "widgets", "widget");

        let (s, _) = api(&r, "GET", "/widgets", None).await;
        assert_eq!(s, StatusCode::OK);
    }

    #[tokio::test]
    async fn sql_auth_deny_all() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::DenyAll);
        let r = admin_sql_router(ops, auth, "test", "widgets", "widget");

        let (s, _) = api(&r, "GET", "/widgets", None).await;
        assert_eq!(s, StatusCode::FORBIDDEN);

        let (s, _) = api(
            &r,
            "POST",
            "/widgets",
            Some(serde_json::json!({"label": "X", "count": 1})),
        )
        .await;
        assert_eq!(s, StatusCode::FORBIDDEN);
    }

    // =====================================================================
    // 24. Malformed JSON → 4xx
    // =====================================================================

    #[tokio::test]
    async fn sql_malformed_json() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);

        let req = Request::builder()
            .method("POST")
            .uri("/widgets")
            .header("content-type", "application/json")
            .body(Body::from("not json"))
            .unwrap();
        let resp = r.oneshot(req).await.unwrap();
        assert!(resp.status().is_client_error());
    }

    // =====================================================================
    // 25. Symmetry: SQL and KV router produce same response format
    // =====================================================================

    #[tokio::test]
    async fn sql_kv_symmetry_error_format() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);

        let (s, err) = api(&r, "GET", "/widgets/nonexistent", None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        assert!(err["code"].is_string(), "error should have code field");
        assert!(err["message"].is_string(), "error should have message field");
    }

    #[tokio::test]
    async fn sql_kv_symmetry_list_format() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);

        let (s, json) = api(&r, "GET", "/widgets", None).await;
        assert_eq!(s, StatusCode::OK);
        assert!(json["items"].is_array(), "list should have items array");
        assert!(json["hasMore"].is_boolean(), "list should have hasMore boolean");
    }

    #[tokio::test]
    async fn sql_kv_symmetry_count_format() {
        let (ops, _dir) = make_sql_ops::<SqlWidget>();
        let r = widget_router(ops);

        let (s, json) = api(&r, "GET", "/widgets/@count", None).await;
        assert_eq!(s, StatusCode::OK);
        assert!(json["count"].is_number(), "count should have count number");
    }
}
