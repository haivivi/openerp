//! Golden tests for Name<T> — typed resource references.
//!
//! Covers:
//!   1. #[model(name = "...")] generates NameTemplate impl
//!   2. Name<T> single-type: construct, validate, resource_id, serde
//!   3. Name<(A, B)> multi-type: validate both prefixes
//!   4. Name<()> any-type: basic format check
//!   5. Widget inference: Name<T> → "select"
//!   6. IR includes "ref" targets for Name fields
//!   7. validate_names hook rejects invalid names on save
//!   8. Admin router roundtrip with Name fields
//!   9. Schema includes ref info

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use openerp_macro::model;
    use openerp_store::{
        admin_kv_router, build_schema, HierarchyNode, KvOps, KvStore, ModuleDef, ResourceDef,
    };
    use openerp_types::*;

    // =====================================================================
    // Models with name templates
    // =====================================================================

    #[model(module = "auth", name = "auth/users/{id}")]
    pub struct TestUser {
        pub id: Id,
        pub email: Email,
        pub active: bool,
    }

    #[model(module = "pms", name = "pms/devices/{sn}")]
    pub struct TestDevice {
        pub sn: Id,
        pub model_code: u32,
        /// References a TestUser — single-type Name.
        pub owner: Name<TestUser>,
    }

    #[model(module = "pms", name = "pms/batches/{id}")]
    pub struct TestBatch {
        pub id: Id,
        pub quantity: u32,
    }

    /// A model that uses multi-type Name and any-type Name.
    #[model(module = "audit")]
    pub struct AuditEntry {
        pub id: Id,
        /// Can reference either a TestUser or a TestDevice.
        pub subject: Name<(TestUser, TestDevice)>,
        /// Can reference any resource.
        pub target: Name<()>,
        pub action: String,
    }

    // =====================================================================
    // KvStore impls
    // =====================================================================

    impl KvStore for TestUser {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "auth:user:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
        }
    }

    impl KvStore for TestDevice {
        const KEY: Field = Self::sn;
        fn kv_prefix() -> &'static str { "pms:device:" }
        fn key_value(&self) -> String { self.sn.to_string() }
        fn validate_names(&self) -> Vec<(&'static str, String)> {
            let mut invalid = Vec::new();
            if !self.owner.is_empty() && !self.owner.validate() {
                invalid.push(("owner", self.owner.to_string()));
            }
            invalid
        }
    }

    impl KvStore for TestBatch {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "pms:batch:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
        }
    }

    impl KvStore for AuditEntry {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "audit:entry:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
        }
        fn validate_names(&self) -> Vec<(&'static str, String)> {
            let mut invalid = Vec::new();
            if !self.subject.is_empty() && !self.subject.validate() {
                invalid.push(("subject", self.subject.to_string()));
            }
            if !self.target.is_empty() && !self.target.validate() {
                invalid.push(("target", self.target.to_string()));
            }
            invalid
        }
    }

    // =====================================================================
    // 1. NameTemplate generation
    // =====================================================================

    #[test]
    fn name_template_generated_for_user() {
        assert_eq!(TestUser::name_prefix(), "auth/users/");
        assert_eq!(TestUser::name_template(), "auth/users/{id}");
    }

    #[test]
    fn name_template_generated_for_device() {
        assert_eq!(TestDevice::name_prefix(), "pms/devices/");
        assert_eq!(TestDevice::name_template(), "pms/devices/{sn}");
    }

    #[test]
    fn name_template_generated_for_batch() {
        assert_eq!(TestBatch::name_prefix(), "pms/batches/");
        assert_eq!(TestBatch::name_template(), "pms/batches/{id}");
    }

    #[test]
    fn name_of_builds_from_instance() {
        let user = TestUser {
            id: Id::new("abc123"),
            email: Email::new("a@b.com"),
            active: true,
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        assert_eq!(user.name_of(), "auth/users/abc123");

        let device = TestDevice {
            sn: Id::new("SN001"),
            model_code: 42,
            owner: Name::new("auth/users/abc123"),
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        assert_eq!(device.name_of(), "pms/devices/SN001");
    }

    // =====================================================================
    // 2. Name<T> single-type: construct, validate, resource_id
    // =====================================================================

    #[test]
    fn name_single_type_from_resource() {
        let user = TestUser {
            id: Id::new("u1"),
            email: Email::new("a@b.com"),
            active: true,
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        let name = Name::<TestUser>::from_resource(&user);
        assert_eq!(name.as_str(), "auth/users/u1");
        assert!(name.validate());
        assert_eq!(name.resource_id(), "u1");
    }

    #[test]
    fn name_single_type_validate() {
        let valid: Name<TestUser> = Name::new("auth/users/xyz");
        assert!(valid.validate());
        assert_eq!(valid.resource_id(), "xyz");

        let invalid: Name<TestUser> = Name::new("pms/devices/SN001");
        assert!(!invalid.validate(), "wrong prefix should fail");

        let empty: Name<TestUser> = Name::default();
        assert!(!empty.validate(), "empty name should fail");

        let prefix_only: Name<TestUser> = Name::new("auth/users/");
        assert!(!prefix_only.validate(), "prefix-only with empty resource id should fail");
    }

    #[test]
    fn name_single_type_serde_roundtrip() {
        let name: Name<TestUser> = Name::new("auth/users/abc");
        let json = serde_json::to_string(&name).unwrap();
        assert_eq!(json, "\"auth/users/abc\"");

        let back: Name<TestUser> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "auth/users/abc");
        assert!(back.validate());
    }

    #[test]
    fn name_single_type_resource_type() {
        let name: Name<TestUser> = Name::new("auth/users/abc");
        assert_eq!(name.resource_type(), "auth/users");
    }

    // =====================================================================
    // 3. Name<(A, B)> multi-type: validate both prefixes
    // =====================================================================

    #[test]
    fn name_tuple_validate_user() {
        let name: Name<(TestUser, TestDevice)> = Name::new("auth/users/u1");
        assert!(name.validate());
    }

    #[test]
    fn name_tuple_validate_device() {
        let name: Name<(TestUser, TestDevice)> = Name::new("pms/devices/SN001");
        assert!(name.validate());
    }

    #[test]
    fn name_tuple_reject_wrong_prefix() {
        let name: Name<(TestUser, TestDevice)> = Name::new("pms/batches/b1");
        assert!(!name.validate(), "batch prefix should not match (User, Device)");
    }

    #[test]
    fn name_tuple_reject_prefix_only() {
        let user_prefix: Name<(TestUser, TestDevice)> = Name::new("auth/users/");
        assert!(!user_prefix.validate(), "user prefix-only should fail");

        let device_prefix: Name<(TestUser, TestDevice)> = Name::new("pms/devices/");
        assert!(!device_prefix.validate(), "device prefix-only should fail");
    }

    #[test]
    fn name_tuple_resource_type() {
        let u: Name<(TestUser, TestDevice)> = Name::new("auth/users/u1");
        assert_eq!(u.resource_type(), "auth/users");

        let d: Name<(TestUser, TestDevice)> = Name::new("pms/devices/SN001");
        assert_eq!(d.resource_type(), "pms/devices");
    }

    // =====================================================================
    // 4. Name<()> any-type: basic format check
    // =====================================================================

    #[test]
    fn name_any_validate() {
        let valid: Name<()> = Name::new("auth/users/abc");
        assert!(valid.validate());

        let also_valid: Name<()> = Name::new("whatever/anything/here");
        assert!(also_valid.validate());

        let invalid: Name<()> = Name::new("no-slash");
        assert!(!invalid.validate());
    }

    // =====================================================================
    // 5. Widget inference: Name<T> → "select"
    // =====================================================================

    #[test]
    fn name_field_widget_is_select() {
        assert_eq!(TestDevice::owner.widget, "select");
        assert_eq!(AuditEntry::subject.widget, "select");
        assert_eq!(AuditEntry::target.widget, "select");
    }

    // =====================================================================
    // 6. IR includes "ref" targets for Name fields
    // =====================================================================

    #[test]
    fn ir_ref_single_type() {
        let ir = TestDevice::__dsl_ir();
        let fields = ir["fields"].as_array().unwrap();
        let owner = fields.iter().find(|f| f["name"] == "owner").unwrap();
        assert_eq!(owner["widget"], "select");

        let refs = owner["ref"].as_array().unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0]["type"], "TestUser");
        assert_eq!(refs[0]["resource"], "test_user");
    }

    #[test]
    fn ir_ref_tuple_type() {
        let ir = AuditEntry::__dsl_ir();
        let fields = ir["fields"].as_array().unwrap();
        let subject = fields.iter().find(|f| f["name"] == "subject").unwrap();

        let refs = subject["ref"].as_array().unwrap();
        assert_eq!(refs.len(), 2);
        let types: Vec<&str> = refs.iter().map(|r| r["type"].as_str().unwrap()).collect();
        assert!(types.contains(&"TestUser"));
        assert!(types.contains(&"TestDevice"));
    }

    #[test]
    fn ir_ref_any_type() {
        let ir = AuditEntry::__dsl_ir();
        let fields = ir["fields"].as_array().unwrap();
        let target = fields.iter().find(|f| f["name"] == "target").unwrap();

        let refs = target["ref"].as_array().unwrap();
        assert_eq!(refs.len(), 0, "Name<()> should have empty ref array");
    }

    // =====================================================================
    // 7. validate_names rejects invalid names on save
    // =====================================================================

    #[test]
    fn validate_names_rejects_bad_owner() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("vn.redb")).unwrap(),
        );
        let ops = KvOps::<TestDevice>::new(kv);

        let device = TestDevice {
            sn: Id::new("SN001"),
            model_code: 42,
            owner: Name::new("wrong/prefix/u1"),
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        let err = ops.save_new(device).unwrap_err();
        assert!(err.to_string().contains("invalid resource name"),
            "Expected validation error, got: {}", err);
    }

    #[test]
    fn validate_names_accepts_valid_owner() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("vn2.redb")).unwrap(),
        );
        let ops = KvOps::<TestDevice>::new(kv);

        let device = TestDevice {
            sn: Id::new("SN002"),
            model_code: 42,
            owner: Name::new("auth/users/u1"),
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        let created = ops.save_new(device).unwrap();
        assert_eq!(created.owner.as_str(), "auth/users/u1");
    }

    #[test]
    fn validate_names_allows_empty_owner() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("vn3.redb")).unwrap(),
        );
        let ops = KvOps::<TestDevice>::new(kv);

        let device = TestDevice {
            sn: Id::new("SN003"),
            model_code: 42,
            owner: Name::default(),
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        ops.save_new(device).unwrap();
    }

    #[test]
    fn validate_names_audit_tuple_type() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("vn4.redb")).unwrap(),
        );
        let ops = KvOps::<AuditEntry>::new(kv);

        // Valid: subject is a user name.
        let entry = AuditEntry {
            id: Id::default(),
            subject: Name::new("auth/users/u1"),
            target: Name::new("pms/batches/b1"),
            action: "create".into(),
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        ops.save_new(entry).unwrap();

        // Valid: subject is a device name.
        let entry2 = AuditEntry {
            id: Id::default(),
            subject: Name::new("pms/devices/SN001"),
            target: Name::new("whatever/resource/x"),
            action: "update".into(),
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        ops.save_new(entry2).unwrap();

        // Invalid: subject has wrong prefix.
        let entry3 = AuditEntry {
            id: Id::default(),
            subject: Name::new("pms/batches/b1"),
            target: Name::new("ok/fine/x"),
            action: "delete".into(),
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        let err = ops.save_new(entry3).unwrap_err();
        assert!(err.to_string().contains("invalid resource name"),
            "Expected validation error for subject, got: {}", err);
    }

    // =====================================================================
    // 8. Admin router roundtrip with Name fields
    // =====================================================================

    async fn api_call(
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
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let json = if bytes.is_empty() {
            serde_json::json!(null)
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::json!(null))
        };
        (status, json)
    }

    #[tokio::test]
    async fn admin_router_name_field_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("rt.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(
            KvOps::<TestDevice>::new(kv), auth, "pms", "devices", "device",
        );

        // Create with valid owner Name.
        let (s, dev) = api_call(&router, "POST", "/devices",
            Some(serde_json::json!({
                "sn": "SN100",
                "modelCode": 42,
                "owner": "auth/users/alice",
                "displayName": "Test Device",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(dev["owner"], "auth/users/alice");
        assert_eq!(dev["sn"], "SN100");

        // Fetch back.
        let (s, fetched) = api_call(&router, "GET", "/devices/SN100", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["owner"], "auth/users/alice");

        // Update: change owner.
        let mut edit = fetched.clone();
        edit["owner"] = serde_json::json!("auth/users/bob");
        let (s, updated) = api_call(&router, "PUT", "/devices/SN100", Some(edit)).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(updated["owner"], "auth/users/bob");

        // Verify via GET.
        let (s, re_read) = api_call(&router, "GET", "/devices/SN100", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(re_read["owner"], "auth/users/bob");
    }

    #[tokio::test]
    async fn admin_router_rejects_invalid_name() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("rj.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(
            KvOps::<TestDevice>::new(kv), auth, "pms", "devices", "device",
        );

        let (s, err) = api_call(&router, "POST", "/devices",
            Some(serde_json::json!({
                "sn": "SN200",
                "modelCode": 42,
                "owner": "wrong/prefix/u1",
                "displayName": "Bad Device",
            })),
        ).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "invalid name should return 400");
        assert_eq!(err["code"], "VALIDATION_FAILED");
    }

    // ── 8b. PUT update with invalid Name → 400 ──

    #[tokio::test]
    async fn admin_router_put_rejects_invalid_name() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("put_rj.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(
            KvOps::<TestDevice>::new(kv), auth, "pms", "devices", "device",
        );

        // Create with valid owner first.
        let (s, dev) = api_call(&router, "POST", "/devices",
            Some(serde_json::json!({
                "sn": "SN300",
                "modelCode": 42,
                "owner": "auth/users/alice",
                "displayName": "Device 300",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);

        // PUT: change owner to invalid prefix → 400.
        let mut edit = dev.clone();
        edit["owner"] = serde_json::json!("pms/batches/b1");
        let (s, err) = api_call(&router, "PUT", "/devices/SN300", Some(edit)).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "PUT with invalid name should return 400");
        assert_eq!(err["code"], "VALIDATION_FAILED");

        // Verify original value is preserved.
        let (s, fetched) = api_call(&router, "GET", "/devices/SN300", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["owner"], "auth/users/alice", "original owner should be preserved");
    }

    // ── 8c. PATCH with invalid Name → 400 ──

    #[tokio::test]
    async fn admin_router_patch_rejects_invalid_name() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("patch_rj.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(
            KvOps::<TestDevice>::new(kv), auth, "pms", "devices", "device",
        );

        // Create valid device.
        let (s, _) = api_call(&router, "POST", "/devices",
            Some(serde_json::json!({
                "sn": "SN400",
                "modelCode": 42,
                "owner": "auth/users/carol",
                "displayName": "Device 400",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK);

        // PATCH: change owner to invalid prefix → 400.
        let (s, err) = api_call(&router, "PATCH", "/devices/SN400",
            Some(serde_json::json!({ "owner": "not-a-valid/name" })),
        ).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "PATCH with invalid name should return 400");
        assert_eq!(err["code"], "VALIDATION_FAILED");

        // Verify original value is preserved.
        let (s, fetched) = api_call(&router, "GET", "/devices/SN400", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["owner"], "auth/users/carol", "original owner should survive bad PATCH");
    }

    // ── 8d. Name<()> rejects no-slash via API ──

    #[tokio::test]
    async fn admin_router_name_any_rejects_no_slash() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("any_rj.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(
            KvOps::<AuditEntry>::new(kv), auth, "audit", "entries", "audit_entry",
        );

        // subject is valid (user name), but target has no slash → 400.
        let (s, err) = api_call(&router, "POST", "/entries",
            Some(serde_json::json!({
                "subject": "auth/users/u1",
                "target": "no-slash-here",
                "action": "create",
                "displayName": "Bad Target",
            })),
        ).await;
        assert_eq!(s, StatusCode::BAD_REQUEST, "Name<()> with no slash should return 400");
        assert_eq!(err["code"], "VALIDATION_FAILED");
    }

    // ── 8e. Edge cases: various invalid Name values via API ──

    #[tokio::test]
    async fn admin_router_name_edge_cases() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("edge.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(
            KvOps::<TestDevice>::new(kv), auth, "pms", "devices", "device",
        );

        let bad_owners = vec![
            ("bare-string", "bare string with no structure"),
            ("auth/groups/g1", "valid path but wrong resource type"),
            ("AUTH/USERS/u1", "case-sensitive prefix mismatch"),
            ("auth/users/", "prefix-only with empty resource id"),
        ];

        for (i, (bad_owner, desc)) in bad_owners.iter().enumerate() {
            let (s, err) = api_call(&router, "POST", "/devices",
                Some(serde_json::json!({
                    "sn": format!("EDGE{}", i),
                    "modelCode": 42,
                    "owner": bad_owner,
                    "displayName": "Edge Case",
                })),
            ).await;
            assert_eq!(s, StatusCode::BAD_REQUEST, "should reject: {}", desc);
            assert_eq!(err["code"], "VALIDATION_FAILED", "should be VALIDATION_FAILED for: {}", desc);
        }
    }

    // ── 8f. Tuple Name field rejects wrong type via API ──

    #[tokio::test]
    async fn admin_router_tuple_name_rejects_wrong_type() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("tuple_rj.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_kv_router(
            KvOps::<AuditEntry>::new(kv), auth, "audit", "entries", "audit_entry",
        );

        // subject is Name<(TestUser, TestDevice)> — batch prefix should fail.
        let (s, err) = api_call(&router, "POST", "/entries",
            Some(serde_json::json!({
                "subject": "pms/batches/b1",
                "target": "auth/users/u1",
                "action": "create",
                "displayName": "Wrong Subject Type",
            })),
        ).await;
        assert_eq!(s, StatusCode::BAD_REQUEST,
            "Name<(User, Device)> should reject batch prefix");
        assert_eq!(err["code"], "VALIDATION_FAILED");

        // Valid: subject is user, target is anything with slash.
        let (s, _) = api_call(&router, "POST", "/entries",
            Some(serde_json::json!({
                "subject": "auth/users/u1",
                "target": "pms/batches/b1",
                "action": "create",
                "displayName": "Good Entry",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK, "valid subject and target should succeed");

        // Valid: subject is device.
        let (s, _) = api_call(&router, "POST", "/entries",
            Some(serde_json::json!({
                "subject": "pms/devices/SN001",
                "target": "whatever/resource/x",
                "action": "update",
                "displayName": "Device Subject",
            })),
        ).await;
        assert_eq!(s, StatusCode::OK, "device subject should succeed for tuple Name");
    }

    // =====================================================================
    // 9. Schema includes ref info
    // =====================================================================

    #[test]
    fn schema_ref_info() {
        let schema = build_schema(
            "RefTestApp",
            vec![
                ModuleDef {
                    id: "auth", label: "Auth", icon: "shield",
                    resources: vec![
                        ResourceDef::from_ir("auth", TestUser::__dsl_ir()),
                    ],
                    enums: vec![],
                    hierarchy: vec![
                        HierarchyNode::leaf("test_user", "Users", "users", ""),
                    ],
                },
                ModuleDef {
                    id: "pms", label: "PMS", icon: "box",
                    resources: vec![
                        ResourceDef::from_ir("pms", TestDevice::__dsl_ir()),
                        ResourceDef::from_ir("pms", TestBatch::__dsl_ir()),
                    ],
                    enums: vec![],
                    hierarchy: vec![
                        HierarchyNode::leaf("test_device", "Devices", "monitor", ""),
                        HierarchyNode::leaf("test_batch", "Batches", "package", ""),
                    ],
                },
                ModuleDef {
                    id: "audit", label: "Audit", icon: "clipboard",
                    resources: vec![
                        ResourceDef::from_ir("audit", AuditEntry::__dsl_ir()),
                    ],
                    enums: vec![],
                    hierarchy: vec![
                        HierarchyNode::leaf("audit_entry", "Audit Log", "list", ""),
                    ],
                },
            ],
        );

        let modules = schema["modules"].as_array().unwrap();
        assert_eq!(modules.len(), 3);

        // Device.owner has ref to TestUser.
        let pms_resources = modules[1]["resources"].as_array().unwrap();
        let device_ir = pms_resources.iter().find(|r| r["name"] == "TestDevice").unwrap();
        let owner_field = device_ir["fields"].as_array().unwrap()
            .iter().find(|f| f["name"] == "owner").unwrap();
        assert_eq!(owner_field["widget"], "select");
        assert_eq!(owner_field["ref"][0]["type"], "TestUser");

        // AuditEntry.subject has ref to [TestUser, TestDevice].
        let audit_resources = modules[2]["resources"].as_array().unwrap();
        let audit_ir = audit_resources.iter().find(|r| r["name"] == "AuditEntry").unwrap();
        let subject_field = audit_ir["fields"].as_array().unwrap()
            .iter().find(|f| f["name"] == "subject").unwrap();
        assert_eq!(subject_field["ref"].as_array().unwrap().len(), 2);

        // AuditEntry.target has ref = [] (any resource).
        let target_field = audit_ir["fields"].as_array().unwrap()
            .iter().find(|f| f["name"] == "target").unwrap();
        assert_eq!(target_field["ref"].as_array().unwrap().len(), 0);
    }
}
