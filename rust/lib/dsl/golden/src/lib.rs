//! Golden tests for the DSL framework.
//!
//! Verify that DSL definitions produce exactly the expected output.
//! If the macro or schema builder changes, these tests break and show the diff.

mod mini_erp;
mod edge_cases;
mod client_test;
mod facet;

#[cfg(test)]
mod tests {
    use openerp_macro::model;
    use openerp_store::{
        build_schema, HierarchyNode, KvOps, KvStore, ModuleDef, ResourceDef,
        apply_overrides,
    };
    use openerp_types::*;
    use std::sync::Arc;

    // =====================================================================
    // Sample DSL definitions
    // =====================================================================

    #[model(module = "test")]
    pub struct Widget {
        pub id: Id,
        pub count: u32,
        pub active: bool,
        pub email: Option<Email>,
        pub tags: Vec<String>,
        pub password_hash: Option<PasswordHash>,
    }

    impl KvStore for Widget {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "test:widget:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new("auto-id");
            }
            self.created_at = DateTime::new("2026-01-01T00:00:00Z");
            self.updated_at = DateTime::new("2026-01-01T00:00:00Z");
        }
    }

    #[model(module = "test")]
    pub struct Item {
        pub id: Id,
        pub widget_id: Id,
        pub quantity: u32,
        #[ui(widget = "select")]
        pub status: String,
    }

    impl KvStore for Item {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "test:item:" }
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

    // A model with complex field types.
    #[model(module = "infra")]
    pub struct Server {
        pub id: Id,
        pub hostname: String,
        pub ip_address: String,
        pub url: Url,
        pub secret_key: Option<Secret>,
        pub version: SemVer,
        pub active: bool,
        pub tags: Vec<String>,
        pub max_connections: u64,
    }

    impl KvStore for Server {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "infra:server:" }
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

    // =====================================================================
    // Golden: Model IR structure
    // =====================================================================

    #[test]
    fn golden_widget_ir_structure() {
        let ir = Widget::__dsl_ir();
        assert_eq!(ir["name"], "Widget");
        assert_eq!(ir["module"], "test");
        assert_eq!(ir["resource"], "widget");

        let fields = ir["fields"].as_array().unwrap();
        let names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();

        // User-defined fields.
        assert!(names.contains(&"id"));
        assert!(names.contains(&"count"));
        assert!(names.contains(&"active"));
        assert!(names.contains(&"email"));
        assert!(names.contains(&"tags"));
        assert!(names.contains(&"password_hash"));

        // Auto-injected common fields.
        assert!(names.contains(&"display_name"), "missing display_name in {:?}", names);
        assert!(names.contains(&"description"), "missing description in {:?}", names);
        assert!(names.contains(&"metadata"), "missing metadata in {:?}", names);
        assert!(names.contains(&"created_at"), "missing created_at in {:?}", names);
        assert!(names.contains(&"updated_at"), "missing updated_at in {:?}", names);

        assert!(names.contains(&"rev"), "missing rev in {:?}", names);

        // Total: 6 user + 6 common = 12.
        assert_eq!(fields.len(), 12, "expected 12 fields, got: {:?}", names);
    }

    #[test]
    fn golden_item_ir_with_ui_override() {
        let ir = Item::__dsl_ir();
        let fields = ir["fields"].as_array().unwrap();
        let status = fields.iter().find(|f| f["name"] == "status").unwrap();
        assert_eq!(status["widget"], "select", "explicit #[ui(widget)] override");
    }

    // =====================================================================
    // Golden: Widget inference by type
    // =====================================================================

    #[test]
    fn golden_widget_type_inference() {
        assert_eq!(Widget::id.widget, "readonly");        // Id -> readonly
        assert_eq!(Widget::count.widget, "text");            // u32 -> text (not a known newtype)
        assert_eq!(Widget::active.widget, "switch");       // bool -> switch
        assert_eq!(Widget::email.widget, "email");         // Email -> email
        assert_eq!(Widget::tags.widget, "tags");            // Vec<String> -> tags
        assert_eq!(Widget::password_hash.widget, "hidden"); // PasswordHash -> hidden
        assert_eq!(Widget::display_name.widget, "text");   // String (common)
        assert_eq!(Widget::description.widget, "textarea"); // "description" heuristic
        assert_eq!(Widget::created_at.widget, "datetime"); // DateTime (common)
        assert_eq!(Widget::updated_at.widget, "datetime"); // DateTime (common)
    }

    // =====================================================================
    // Golden: ResourceDef auto-derivation
    // =====================================================================

    #[test]
    fn golden_resource_def_from_ir() {
        let def = ResourceDef::from_ir("test", Widget::__dsl_ir());
        assert_eq!(def.name, "widget");
        assert_eq!(def.label, "Widgets");
        assert_eq!(def.path, "widgets");
        assert_eq!(def.icon, "file"); // default icon for unknown resource
        assert_eq!(def.permissions.len(), 5);
        assert!(def.permissions.contains(&"test:widget:create".to_string()));
        assert!(def.permissions.contains(&"test:widget:read".to_string()));
        assert!(def.permissions.contains(&"test:widget:update".to_string()));
        assert!(def.permissions.contains(&"test:widget:delete".to_string()));
        assert!(def.permissions.contains(&"test:widget:list".to_string()));
    }

    #[test]
    fn golden_resource_def_with_action() {
        let def = ResourceDef::from_ir("test", Item::__dsl_ir())
            .with_action("test", "approve")
            .with_action("test", "reject");
        assert_eq!(def.permissions.len(), 7); // 5 CRUD + approve + reject
        assert!(def.permissions.contains(&"test:item:approve".to_string()));
        assert!(def.permissions.contains(&"test:item:reject".to_string()));
    }

    // =====================================================================
    // Golden: Schema builder output
    // =====================================================================

    #[test]
    fn golden_schema_structure() {
        let schema = build_schema(
            "TestApp",
            vec![ModuleDef {
                id: "test",
                label: "Test Module",
                icon: "flask",
                resources: vec![
                    ResourceDef::from_ir("test", Widget::__dsl_ir()).with_desc("Test widgets"),
                    ResourceDef::from_ir("test", Item::__dsl_ir()).with_desc("Widget items"),
                ],
                enums: vec![],
                hierarchy: vec![
                    HierarchyNode {
                        resource: "widget", label: "Widgets", icon: "cube",
                        description: "Test widgets",
                        children: vec![
                            HierarchyNode::leaf("item", "Items", "file-text", "Widget items"),
                        ],
                    },
                ],
            }],
        );

        assert_eq!(schema["name"], "TestApp");
        assert_eq!(schema["modules"].as_array().unwrap().len(), 1);

        let m = &schema["modules"][0];
        assert_eq!(m["id"], "test");
        assert_eq!(m["resources"].as_array().unwrap().len(), 2);

        // Hierarchy tree.
        let nav = m["hierarchy"]["nav"].as_array().unwrap();
        assert_eq!(nav.len(), 1); // Only Widget at top level.
        assert_eq!(nav[0]["resource"], "widget");
        assert_eq!(nav[0]["children"].as_array().unwrap().len(), 1);
        assert_eq!(nav[0]["children"][0]["resource"], "item");

        // Permissions.
        let perms = &schema["permissions"]["test"];
        assert!(perms["widget"]["actions"].as_array().unwrap().len() >= 5);
        assert!(perms["item"]["actions"].as_array().unwrap().len() >= 5);

        // Permission entry structure.
        let first_perm = &perms["widget"]["actions"][0];
        assert!(first_perm["perm"].is_string());
        assert!(first_perm["action"].is_string());
        assert!(first_perm["desc"].is_string());
    }

    #[test]
    fn golden_schema_ui_overrides() {
        let mut schema = build_schema(
            "TestApp",
            vec![ModuleDef {
                id: "test",
                label: "Test",
                icon: "flask",
                resources: vec![
                    ResourceDef::from_ir("test", Item::__dsl_ir()),
                ],
                enums: vec![],
                hierarchy: vec![HierarchyNode::leaf("item", "Items", "file-text", "Items")],
            }],
        );

        let overrides = vec![
            openerp_store::widget!(select {
                source: "/admin/test/widgets",
                display: "display_name",
                value: "id"
            } => [Item.widget_id]),
        ];
        apply_overrides(&mut schema, &overrides);

        // Check the override was applied.
        let fields = schema["modules"][0]["resources"][0]["fields"].as_array().unwrap();
        let wid_field = fields.iter().find(|f| f["name"] == "widget_id").unwrap();
        assert_eq!(wid_field["widget"], "select");
        assert_eq!(wid_field["source"], "/admin/test/widgets");
        assert_eq!(wid_field["display"], "display_name");
        assert_eq!(wid_field["value"], "id");
    }

    // =====================================================================
    // Golden: Hierarchy to_json
    // =====================================================================

    #[test]
    fn golden_hierarchy_json() {
        let tree = HierarchyNode {
            resource: "model", label: "Models", icon: "cube",
            description: "Product models",
            children: vec![
                HierarchyNode::leaf("device", "Devices", "desktop", "Produced devices"),
                HierarchyNode::leaf("batch", "Batches", "package", "Production batches"),
            ],
        };

        let json = tree.to_json();
        assert_eq!(json["resource"], "model");
        assert_eq!(json["label"], "Models");
        assert_eq!(json["children"].as_array().unwrap().len(), 2);
        assert_eq!(json["children"][0]["resource"], "device");
        assert_eq!(json["children"][1]["resource"], "batch");
        // Leaf children have empty children array.
        assert_eq!(json["children"][0]["children"].as_array().unwrap().len(), 0);
    }

    // =====================================================================
    // Golden: KvStore CRUD lifecycle
    // =====================================================================

    #[test]
    fn golden_kvstore_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> =
            Arc::new(openerp_kv::RedbStore::open(&dir.path().join("golden.redb")).unwrap());
        let ops = KvOps::<Widget>::new(kv);

        // Create with hook.
        let w = Widget {
            id: Id::default(),
            count: 42,
            active: true,
            email: Some(Email::new("test@test.com")),
            tags: vec!["a".into(), "b".into()],
            password_hash: None,
            display_name: Some("Test".into()),
            description: None,
            metadata: None,
            created_at: DateTime::default(),
            updated_at: DateTime::default(),
            rev: 0,
        };
        let created = ops.save_new(w).unwrap();
        assert_eq!(created.id.as_str(), "auto-id");
        assert_eq!(created.created_at.as_str(), "2026-01-01T00:00:00Z");

        // Get.
        let fetched = ops.get_or_err("auto-id").unwrap();
        assert_eq!(fetched.count, 42);

        // List.
        assert_eq!(ops.list().unwrap().len(), 1);

        // Update.
        let mut u = fetched;
        u.count = 100;
        ops.save(u).unwrap();
        assert_eq!(ops.get_or_err("auto-id").unwrap().count, 100);

        // Delete.
        ops.delete("auto-id").unwrap();
        assert!(ops.get("auto-id").unwrap().is_none());
        assert_eq!(ops.list().unwrap().len(), 0);
    }

    // =====================================================================
    // Golden: Serde camelCase + defaults
    // =====================================================================

    #[test]
    fn golden_serde_camel_case() {
        let w = Widget {
            id: Id::new("x"),
            count: 1,
            active: true,
            email: None,
            tags: vec![],
            password_hash: None,
            display_name: None,
            description: None,
            metadata: None,
            created_at: DateTime::default(),
            updated_at: DateTime::default(),
            rev: 0,
        };
        let json = serde_json::to_value(&w).unwrap();
        // camelCase keys.
        assert!(json.get("displayName").is_some(), "expected camelCase displayName");
        assert!(json.get("passwordHash").is_some(), "expected camelCase passwordHash");
        assert!(json.get("createdAt").is_some(), "expected camelCase createdAt");
    }

    #[test]
    fn golden_serde_defaults() {
        // Deserialize with missing fields — all should default.
        let json = r#"{"count": 5}"#;
        let w: Widget = serde_json::from_str(json).unwrap();
        assert_eq!(w.count, 5);
        assert!(w.id.is_empty()); // default Id
        assert!(!w.active); // default bool = false
        assert!(w.email.is_none()); // default Option
        assert!(w.tags.is_empty()); // default Vec
        assert!(w.display_name.is_none()); // common field default
    }

    // =====================================================================
    // Golden: Complex type widget inference (Server model)
    // =====================================================================

    #[test]
    fn golden_complex_type_inference() {
        assert_eq!(Server::id.widget, "readonly");
        assert_eq!(Server::url.widget, "url");
        assert_eq!(Server::secret_key.widget, "hidden"); // Secret → hidden
        assert_eq!(Server::version.widget, "text");      // SemVer → text
        assert_eq!(Server::active.widget, "switch");
        assert_eq!(Server::tags.widget, "tags");
        assert_eq!(Server::max_connections.widget, "text"); // u64 → text
    }

    #[test]
    fn golden_complex_type_ir() {
        let ir = Server::__dsl_ir();
        assert_eq!(ir["module"], "infra");
        assert_eq!(ir["name"], "Server");
        assert_eq!(ir["resource"], "server");

        let fields = ir["fields"].as_array().unwrap();
        // 9 user + 6 common = 15.
        assert_eq!(fields.len(), 15, "Server should have 15 fields");

        // Verify type names in IR.
        let url_f = fields.iter().find(|f| f["name"] == "url").unwrap();
        assert_eq!(url_f["ty"], "Url");

        let secret_f = fields.iter().find(|f| f["name"] == "secret_key").unwrap();
        assert!(secret_f["ty"].as_str().unwrap().contains("Secret"));
        assert_eq!(secret_f["widget"], "hidden");
    }

    // =====================================================================
    // Golden: Multi-module schema
    // =====================================================================

    #[test]
    fn golden_multi_module_schema() {
        let schema = build_schema(
            "MultiApp",
            vec![
                ModuleDef {
                    id: "test",
                    label: "Test Module",
                    icon: "flask",
                    resources: vec![
                        ResourceDef::from_ir("test", Widget::__dsl_ir()),
                        ResourceDef::from_ir("test", Item::__dsl_ir()),
                    ],
                    enums: vec![],
                    hierarchy: vec![
                        HierarchyNode {
                            resource: "widget", label: "Widgets", icon: "cube",
                            description: "Widgets", children: vec![
                                HierarchyNode::leaf("item", "Items", "file", "Items"),
                            ],
                        },
                    ],
                },
                ModuleDef {
                    id: "infra",
                    label: "Infrastructure",
                    icon: "server",
                    resources: vec![
                        ResourceDef::from_ir("infra", Server::__dsl_ir()),
                    ],
                    enums: vec![],
                    hierarchy: vec![
                        HierarchyNode::leaf("server", "Servers", "desktop", "Servers"),
                    ],
                },
            ],
        );

        assert_eq!(schema["name"], "MultiApp");
        let modules = schema["modules"].as_array().unwrap();
        assert_eq!(modules.len(), 2);

        // Module isolation: each has its own resources.
        assert_eq!(modules[0]["id"], "test");
        assert_eq!(modules[0]["resources"].as_array().unwrap().len(), 2);
        assert_eq!(modules[1]["id"], "infra");
        assert_eq!(modules[1]["resources"].as_array().unwrap().len(), 1);

        // Permissions are per-module.
        assert!(schema["permissions"]["test"]["widget"].is_object());
        assert!(schema["permissions"]["test"]["item"].is_object());
        assert!(schema["permissions"]["infra"]["server"].is_object());
        // No cross-contamination.
        assert!(schema["permissions"]["test"]["server"].is_null());
        assert!(schema["permissions"]["infra"]["widget"].is_null());
    }

    // =====================================================================
    // Golden: Admin router — Authenticator hook (AllowAll vs DenyAll)
    // =====================================================================

    #[tokio::test]
    async fn golden_admin_router_allow_all() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;
        use openerp_store::admin_kv_router;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("allow.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(KvOps::<Widget>::new(kv), auth, "test", "widgets", "widget");

        // AllowAll: any request should succeed.
        let req = Request::builder().uri("/widgets").body(Body::empty()).unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn golden_admin_router_deny_all() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;
        use openerp_store::admin_kv_router;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("deny.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::DenyAll);
        let router = admin_kv_router(KvOps::<Widget>::new(kv), auth, "test", "widgets", "widget");

        // DenyAll: every request should be rejected.
        let req = Request::builder().uri("/widgets").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "DenyAll should reject list");

        let req = Request::builder()
            .method("POST").uri("/widgets")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"count":1}"#)).unwrap();
        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "DenyAll should reject create");
    }

    // =====================================================================
    // Golden: Admin router full CRUD with hooks verified
    // =====================================================================

    #[tokio::test]
    async fn golden_admin_crud_with_hooks() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;
        use openerp_store::admin_kv_router;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("crud.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(KvOps::<Item>::new(kv), auth, "test", "items", "item");

        // 1. POST: create — before_create fills id + timestamps.
        let req = Request::builder()
            .method("POST").uri("/items")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"widgetId":"w1","quantity":10,"status":"draft"}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let id = created["id"].as_str().unwrap();
        assert!(!id.is_empty(), "before_create should auto-generate id");
        assert!(created["createdAt"].as_str().unwrap().contains("T"), "created_at should be ISO datetime");
        assert!(created["updatedAt"].as_str().unwrap().contains("T"), "updated_at should be set");
        assert_eq!(created["widgetId"], "w1");
        assert_eq!(created["quantity"], 10);
        assert_eq!(created["status"], "draft");

        // 2. GET by id.
        let req = Request::builder()
            .uri(format!("/items/{}", id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let fetched: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(fetched["id"], id);
        assert_eq!(fetched["quantity"], 10);

        // 3. PUT: update — before_update changes updated_at.
        let old_updated_at = created["updatedAt"].as_str().unwrap().to_string();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut update = created.clone();
        update["quantity"] = serde_json::json!(20);
        update["status"] = serde_json::json!("approved");
        let req = Request::builder()
            .method("PUT").uri(format!("/items/{}", id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&update).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let updated: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(updated["id"], id, "id must not change");
        assert_eq!(updated["quantity"], 20);
        assert_eq!(updated["status"], "approved");
        assert_eq!(updated["createdAt"], created["createdAt"], "createdAt must be preserved");
        assert_ne!(updated["updatedAt"].as_str().unwrap(), old_updated_at, "updatedAt should change");

        // 4. GET list — should have exactly 1 item.
        let req = Request::builder().uri("/items").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["items"].as_array().unwrap().len(), 1);
        assert_eq!(list["hasMore"], false);
        assert_eq!(list["items"][0]["quantity"], 20);

        // 5. DELETE.
        let req = Request::builder()
            .method("DELETE").uri(format!("/items/{}", id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 6. GET after delete → 404.
        let req = Request::builder()
            .uri(format!("/items/{}", id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        // 7. List is empty.
        let req = Request::builder().uri("/items").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["items"].as_array().unwrap().len(), 0);
        assert_eq!(list["hasMore"], false);
    }

    // =====================================================================
    // Golden: Admin router error responses
    // =====================================================================

    #[tokio::test]
    async fn golden_admin_error_responses() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;
        use openerp_store::admin_kv_router;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("err.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(KvOps::<Widget>::new(kv), auth, "test", "widgets", "widget");

        // GET non-existent → 404 with error message.
        let req = Request::builder()
            .uri("/widgets/nonexistent").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let err: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(err["code"], "NOT_FOUND");
        assert!(err["message"].as_str().unwrap().contains("not found"));

        // PUT non-existent → 404.
        let req = Request::builder()
            .method("PUT").uri("/widgets/ghost")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"id":"ghost","count":1}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        // DELETE non-existent → 404.
        let req = Request::builder()
            .method("DELETE").uri("/widgets/ghost").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        // POST malformed JSON → 4xx (axum returns 400 or 422 depending on version).
        let req = Request::builder()
            .method("POST").uri("/widgets")
            .header("content-type", "application/json")
            .body(Body::from("not json")).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert!(resp.status().is_client_error(), "Malformed JSON should return 4xx, got {}", resp.status());

        // POST duplicate → validation error.
        let req = Request::builder()
            .method("POST").uri("/widgets")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"count":1}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Second POST with same auto-id "auto-id" → duplicate.
        let req = Request::builder()
            .method("POST").uri("/widgets")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"count":2}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT, "Duplicate should return 409");
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let err: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(err["code"], "ALREADY_EXISTS");
        assert!(err["message"].as_str().unwrap().contains("already exists"));
    }

    // =====================================================================
    // Golden: Serde roundtrip — JSON → KV → JSON preserves all fields
    // =====================================================================

    #[tokio::test]
    async fn golden_serde_roundtrip_through_admin() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;
        use openerp_store::admin_kv_router;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("rt.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(KvOps::<Server>::new(kv), auth, "infra", "servers", "server");

        // Create with all field types.
        let server_json = serde_json::json!({
            "hostname": "web-01",
            "ipAddress": "10.0.0.1",
            "url": "https://web-01.internal:8443",
            "secretKey": "s3cr3t-k3y",
            "version": "2.5.1",
            "active": true,
            "tags": ["prod", "us-east", "critical"],
            "maxConnections": 10000,
            "displayName": "Web Server 01",
            "description": "Primary web server",
        });

        let req = Request::builder()
            .method("POST").uri("/servers")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&server_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = created["id"].as_str().unwrap();

        // Fetch back and verify every field.
        let req = Request::builder()
            .uri(format!("/servers/{}", id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let fetched: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(fetched["hostname"], "web-01");
        assert_eq!(fetched["ipAddress"], "10.0.0.1");
        assert_eq!(fetched["url"], "https://web-01.internal:8443");
        assert_eq!(fetched["secretKey"], "s3cr3t-k3y");
        assert_eq!(fetched["version"], "2.5.1");
        assert_eq!(fetched["active"], true);
        assert_eq!(fetched["tags"].as_array().unwrap().len(), 3);
        assert_eq!(fetched["tags"][0], "prod");
        assert_eq!(fetched["tags"][2], "critical");
        assert_eq!(fetched["maxConnections"], 10000);
        assert_eq!(fetched["displayName"], "Web Server 01");
        assert_eq!(fetched["description"], "Primary web server");
        // Auto-filled fields.
        assert!(!fetched["id"].as_str().unwrap().is_empty());
        assert!(fetched["createdAt"].as_str().unwrap().contains("T"));
        assert!(fetched["updatedAt"].as_str().unwrap().contains("T"));
    }

    // =====================================================================
    // Golden: Permission string format
    // =====================================================================

    #[test]
    fn golden_permission_format() {
        // Verify the exact permission strings generated.
        let schema = build_schema(
            "PermApp",
            vec![
                ModuleDef {
                    id: "crm",
                    label: "CRM",
                    icon: "users",
                    resources: vec![
                        ResourceDef::from_ir("crm", Widget::__dsl_ir())
                            .with_action("crm", "export")
                            .with_action("crm", "archive"),
                    ],
                    enums: vec![],
                    hierarchy: vec![HierarchyNode::leaf("widget", "Widgets", "cube", "")],
                },
            ],
        );

        let perms = &schema["permissions"]["crm"]["widget"]["actions"];
        let perm_strings: Vec<&str> = perms.as_array().unwrap().iter()
            .map(|p| p["perm"].as_str().unwrap())
            .collect();

        // Standard CRUD.
        assert!(perm_strings.contains(&"crm:widget:create"));
        assert!(perm_strings.contains(&"crm:widget:read"));
        assert!(perm_strings.contains(&"crm:widget:update"));
        assert!(perm_strings.contains(&"crm:widget:delete"));
        assert!(perm_strings.contains(&"crm:widget:list"));
        // Custom actions.
        assert!(perm_strings.contains(&"crm:widget:export"));
        assert!(perm_strings.contains(&"crm:widget:archive"));
        // Total 7.
        assert_eq!(perm_strings.len(), 7);

        // Each entry has action + desc.
        let export = perms.as_array().unwrap().iter()
            .find(|p| p["perm"] == "crm:widget:export").unwrap();
        assert_eq!(export["action"], "export");
        assert!(export["desc"].is_string());
    }

    // =====================================================================
    // Golden: Field constants are compile-time accessible
    // =====================================================================

    #[test]
    fn golden_field_const_properties() {
        // Field consts carry name + type + widget at compile time.
        let f = Widget::email;
        assert_eq!(f.name, "email");
        assert!(f.ty.contains("Email"), "type should contain Email, got: {}", f.ty);
        assert_eq!(f.widget, "email");

        let f = Server::secret_key;
        assert_eq!(f.name, "secret_key");
        assert!(f.ty.contains("Secret"));
        assert_eq!(f.widget, "hidden");

        // Common fields are also consts.
        let f = Widget::created_at;
        assert_eq!(f.name, "created_at");
        assert_eq!(f.widget, "datetime");
    }

    // =====================================================================
    // Golden: widget! macro with multiple fields
    // =====================================================================

    #[test]
    fn golden_widget_macro_multi_field() {
        let overrides = vec![
            openerp_store::widget!(textarea { rows: 5, placeholder: "Enter notes..." } => [
                Widget.description,
                Item.status
            ]),
        ];

        assert_eq!(overrides.len(), 1);
        assert_eq!(overrides[0].widget, "textarea");
        assert_eq!(overrides[0].apply_to.len(), 2);
        assert_eq!(overrides[0].apply_to[0], "Widget.description");
        assert_eq!(overrides[0].apply_to[1], "Item.status");
        assert_eq!(overrides[0].params["rows"], 5);
        assert_eq!(overrides[0].params["placeholder"], "Enter notes...");
    }

    // =====================================================================
    // Golden: Pagination via admin router
    // =====================================================================

    #[tokio::test]
    async fn golden_admin_pagination() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;
        use openerp_store::admin_kv_router;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("page.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(KvOps::<Item>::new(kv), auth, "test", "items", "item");

        // Create 5 items.
        for i in 0..5 {
            let body = serde_json::json!({"widgetId": "w1", "quantity": i, "status": "ok"});
            let req = Request::builder()
                .method("POST").uri("/items")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
            let resp = router.clone().oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }

        // Page 1: limit=2.
        let req = Request::builder().uri("/items?limit=2&offset=0").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["items"].as_array().unwrap().len(), 2);
        assert_eq!(list["hasMore"], true);

        // Page 3: offset=4 → 1 item left.
        let req = Request::builder().uri("/items?limit=2&offset=4").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["items"].as_array().unwrap().len(), 1);
        assert_eq!(list["hasMore"], false);

        // Default (no params): all 5.
        let req = Request::builder().uri("/items").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["items"].as_array().unwrap().len(), 5);
        assert_eq!(list["hasMore"], false);
    }

    // =====================================================================
    // Golden: @count endpoint via admin router
    // =====================================================================

    #[tokio::test]
    async fn golden_admin_count() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;
        use openerp_store::admin_kv_router;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("count.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(KvOps::<Item>::new(kv), auth, "test", "items", "item");

        // Empty: count=0.
        let req = Request::builder().uri("/items/@count").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["count"], 0);

        // Create 3 items.
        for _ in 0..3 {
            let body = serde_json::json!({"widgetId": "w1", "quantity": 1, "status": "ok"});
            let req = Request::builder()
                .method("POST").uri("/items")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
            router.clone().oneshot(req).await.unwrap();
        }

        // count=3.
        let req = Request::builder().uri("/items/@count").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["count"], 3);
    }

    // =====================================================================
    // Golden: Optimistic locking (rev) via admin PUT → 409
    // =====================================================================

    #[tokio::test]
    async fn golden_admin_optimistic_lock_409() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;
        use openerp_store::admin_kv_router;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("lock.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(KvOps::<Item>::new(kv), auth, "test", "items", "item");

        // Create item → rev=1.
        let req = Request::builder()
            .method("POST").uri("/items")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"widgetId":"w1","quantity":10,"status":"new"}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = created["id"].as_str().unwrap();
        assert_eq!(created["rev"], 1);

        // Simulate two concurrent GETs.
        let client_a = created.clone();
        let client_b = created.clone();

        // Client A PUTs first → succeeds, rev becomes 2.
        let mut update_a = client_a.clone();
        update_a["quantity"] = serde_json::json!(20);
        let req = Request::builder()
            .method("PUT").uri(format!("/items/{}", id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&update_a).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Client A should succeed");
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let updated_a: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(updated_a["rev"], 2);
        assert_eq!(updated_a["quantity"], 20);

        // Client B PUTs with stale rev=1 → 409 Conflict.
        let mut update_b = client_b.clone();
        update_b["status"] = serde_json::json!("approved");
        let req = Request::builder()
            .method("PUT").uri(format!("/items/{}", id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&update_b).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT, "Client B should get 409");

        // Verify data: Client A's changes persist.
        let req = Request::builder().uri(format!("/items/{}", id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let final_state: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(final_state["quantity"], 20, "Client A's update persists");
        assert_eq!(final_state["status"], "new", "Client B's update was rejected");
        assert_eq!(final_state["rev"], 2);
    }

    // =====================================================================
    // Golden: PATCH partial update via admin router
    // =====================================================================

    #[tokio::test]
    async fn golden_admin_patch() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;
        use openerp_store::admin_kv_router;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("patch.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(KvOps::<Item>::new(kv), auth, "test", "items", "item");

        // Create item.
        let req = Request::builder()
            .method("POST").uri("/items")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"widgetId":"w1","quantity":10,"status":"draft"}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = created["id"].as_str().unwrap();
        assert_eq!(created["rev"], 1);

        // PATCH: only change status, include rev for locking.
        let patch = serde_json::json!({"status": "approved", "rev": 1});
        let req = Request::builder()
            .method("PATCH").uri(format!("/items/{}", id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&patch).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "PATCH should succeed");
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let patched: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(patched["status"], "approved", "status should be updated");
        assert_eq!(patched["quantity"], 10, "quantity should be unchanged");
        assert_eq!(patched["widgetId"], "w1", "widgetId should be unchanged");
        assert_eq!(patched["rev"], 2, "rev should be bumped");

        // PATCH with stale rev → 409.
        let stale_patch = serde_json::json!({"status": "rejected", "rev": 1});
        let req = Request::builder()
            .method("PATCH").uri(format!("/items/{}", id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&stale_patch).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT, "Stale PATCH should get 409");

        // PATCH without rev → no conflict check, just updates.
        let no_rev_patch = serde_json::json!({"quantity": 99});
        let req = Request::builder()
            .method("PATCH").uri(format!("/items/{}", id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&no_rev_patch).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "PATCH without rev should succeed");
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let patched2: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(patched2["quantity"], 99);
        assert_eq!(patched2["status"], "approved", "status still from first patch");
        assert_eq!(patched2["rev"], 3, "rev bumped again");

        // PATCH on nonexistent → 404.
        let req = Request::builder()
            .method("PATCH").uri("/items/nonexistent")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"status":"x"}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
