//! Mini-ERP golden test — simulates a realistic ERP with the DSL framework.
//!
//! Models: Employee, Department, Role, Project
//! Tests the full framework lifecycle:
//!   1. Define models with #[model] (various field types)
//!   2. KvStore hooks (password hashing, auto-id, timestamps, normalization)
//!   3. Admin CRUD for multiple resources
//!   4. Custom Authenticator with real permission logic
//!   5. Permission enforcement: allowed/denied per resource
//!   6. Schema generation with hierarchy + UI overrides
//!   7. Facet API (external-facing subset)
//!   8. Cross-resource isolation (KV prefix doesn't leak)
//!   9. List with multiple records
//!  10. Edit preserves all fields (serde roundtrip)

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::extract::{Path, State};
    use axum::http::{HeaderMap, Request, StatusCode};
    use axum::routing::get;
    use axum::{Json, Router};
    use tower::ServiceExt;

    use openerp_core::{Authenticator, ListResult, ServiceError};
    use openerp_macro::model;
    use openerp_store::{
        admin_kv_router, apply_overrides, build_schema, FacetDef, HierarchyNode, KvOps, KvStore,
        ModuleDef, ResourceDef,
    };
    use openerp_types::*;

    // =====================================================================
    // Mini-ERP Models
    // =====================================================================

    #[model(module = "hr")]
    pub struct Employee {
        pub id: Id,
        pub email: Email,
        pub password_hash: Option<PasswordHash>,
        pub department_id: Option<Id>,
        pub active: bool,
        pub avatar: Option<Avatar>,
        pub phone: Option<String>,
        pub hire_date: Option<DateTime>,
        pub salary: Option<u64>,
    }

    #[model(module = "hr")]
    pub struct Department {
        pub id: Id,
        pub parent_id: Option<Id>,
        pub head_employee_id: Option<Id>,
        pub budget: u64,
    }

    #[model(module = "hr")]
    pub struct Role {
        pub id: Id,
        pub permissions: Vec<String>,
    }

    #[model(module = "pm")]
    pub struct Project {
        pub id: Id,
        pub owner_id: Id,
        pub status: String,
        pub budget: u64,
        pub url: Option<Url>,
        pub tags: Vec<String>,
        pub secret_token: Option<Secret>,
    }

    // =====================================================================
    // KvStore Implementations with hooks
    // =====================================================================

    impl KvStore for Employee {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "hr:employee:" }
        fn key_value(&self) -> String { self.id.to_string() }

        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
            // Normalize email to lowercase.
            self.email = Email::new(&self.email.as_str().to_lowercase());
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }

        fn before_update(&mut self) {
            // Normalize email on update too.
            self.email = Email::new(&self.email.as_str().to_lowercase());
            self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
        }
    }

    impl KvStore for Department {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "hr:department:" }
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

    impl KvStore for Role {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "hr:role:" }
        fn key_value(&self) -> String { self.id.to_string() }

        fn before_create(&mut self) {
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }

        fn before_update(&mut self) {
            self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
        }
    }

    impl KvStore for Project {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "pm:project:" }
        fn key_value(&self) -> String { self.id.to_string() }

        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
            if self.status.is_empty() { self.status = "draft".into(); }
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }

        fn before_update(&mut self) {
            self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
        }
    }

    // =====================================================================
    // Custom Authenticator (simulates real auth)
    // =====================================================================

    /// Mini auth: checks a static permission map from JWT-like headers.
    /// Header "x-roles" is a comma-separated role list.
    /// Roles are looked up in KV to find permissions.
    struct MiniAuth {
        kv: Arc<dyn openerp_kv::KVStore>,
    }

    impl Authenticator for MiniAuth {
        fn check(&self, headers: &HeaderMap, permission: &str) -> Result<(), ServiceError> {
            let roles_header = headers
                .get("x-roles")
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| ServiceError::Unauthorized("missing x-roles header".into()))?;

            // "root" bypasses everything.
            if roles_header == "root" {
                return Ok(());
            }

            let role_ids: Vec<&str> = roles_header.split(',').map(|s| s.trim()).collect();
            let role_ops = KvOps::<Role>::new(self.kv.clone());

            for role_id in &role_ids {
                if let Ok(Some(role)) = role_ops.get(role_id) {
                    if role.permissions.iter().any(|p| p == permission) {
                        return Ok(());
                    }
                }
            }

            Err(ServiceError::PermissionDenied(format!(
                "none of roles {:?} have permission '{}'",
                role_ids, permission
            )))
        }
    }

    // =====================================================================
    // Helper: build mini-ERP
    // =====================================================================

    struct MiniErp {
        kv: Arc<dyn openerp_kv::KVStore>,
        hr_router: Router,
        pm_router: Router,
        _dir: tempfile::TempDir,
    }

    fn setup_mini_erp() -> MiniErp {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("erp.redb")).unwrap(),
        );
        let auth: Arc<dyn Authenticator> = Arc::new(MiniAuth { kv: kv.clone() });

        let mut hr_router = Router::new();
        hr_router = hr_router.merge(admin_kv_router(
            KvOps::<Employee>::new(kv.clone()), auth.clone(), "hr", "employees", "employee",
        ));
        hr_router = hr_router.merge(admin_kv_router(
            KvOps::<Department>::new(kv.clone()), auth.clone(), "hr", "departments", "department",
        ));
        hr_router = hr_router.merge(admin_kv_router(
            KvOps::<Role>::new(kv.clone()), auth.clone(), "hr", "roles", "role",
        ));

        let pm_router = admin_kv_router(
            KvOps::<Project>::new(kv.clone()), auth.clone(), "pm", "projects", "project",
        );

        MiniErp { kv, hr_router, pm_router, _dir: dir }
    }

    async fn api_call(
        router: &Router,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
        roles: &str,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        builder = builder.header("x-roles", roles);
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
    // Tests
    // =====================================================================

    // ── 1. Model IR for all 4 models ──

    #[test]
    fn mini_erp_model_ir() {
        let emp_ir = Employee::__dsl_ir();
        assert_eq!(emp_ir["module"], "hr");
        assert_eq!(emp_ir["name"], "Employee");
        // 9 user fields + 5 common = 14
        assert_eq!(emp_ir["fields"].as_array().unwrap().len(), 14);

        let dept_ir = Department::__dsl_ir();
        assert_eq!(dept_ir["module"], "hr");
        // 4 user (id, parent_id, head_employee_id, budget) + 5 common = 9
        assert_eq!(dept_ir["fields"].as_array().unwrap().len(), 9);

        let proj_ir = Project::__dsl_ir();
        assert_eq!(proj_ir["module"], "pm");
        // 7 user + 5 common = 12
        assert_eq!(proj_ir["fields"].as_array().unwrap().len(), 12);
    }

    // ── 2. Widget inference for all types ──

    #[test]
    fn mini_erp_widget_inference() {
        // Employee
        assert_eq!(Employee::id.widget, "readonly");
        assert_eq!(Employee::email.widget, "email");
        assert_eq!(Employee::password_hash.widget, "hidden");
        assert_eq!(Employee::active.widget, "switch");
        assert_eq!(Employee::avatar.widget, "image");
        assert_eq!(Employee::hire_date.widget, "datetime");
        assert_eq!(Employee::salary.widget, "text"); // u64 → text
        // Project
        assert_eq!(Project::url.widget, "url");
        assert_eq!(Project::secret_token.widget, "hidden");
        assert_eq!(Project::tags.widget, "tags");
    }

    // ── 3. Schema with 2 modules + hierarchy + overrides ──

    #[test]
    fn mini_erp_schema() {
        let mut schema = build_schema(
            "MiniERP",
            vec![
                ModuleDef {
                    id: "hr", label: "Human Resources", icon: "users",
                    resources: vec![
                        ResourceDef::from_ir("hr", Employee::__dsl_ir()).with_desc("Company employees"),
                        ResourceDef::from_ir("hr", Department::__dsl_ir()).with_desc("Organizational units"),
                        ResourceDef::from_ir("hr", Role::__dsl_ir()).with_desc("Permission roles"),
                    ],
                    hierarchy: vec![
                        HierarchyNode {
                            resource: "employee", label: "Employees", icon: "users",
                            description: "Employees",
                            children: vec![],
                        },
                        HierarchyNode {
                            resource: "department", label: "Departments", icon: "building",
                            description: "Departments",
                            children: vec![],
                        },
                        HierarchyNode::leaf("role", "Roles", "shield", "Roles"),
                    ],
                },
                ModuleDef {
                    id: "pm", label: "Project Management", icon: "folder",
                    resources: vec![
                        ResourceDef::from_ir("pm", Project::__dsl_ir())
                            .with_desc("Projects")
                            .with_action("pm", "archive")
                            .with_action("pm", "activate"),
                    ],
                    hierarchy: vec![
                        HierarchyNode::leaf("project", "Projects", "folder", "Projects"),
                    ],
                },
            ],
        );

        // Apply UI overrides.
        let overrides = vec![
            openerp_store::widget!(permission_picker { source: "schema.permissions" }
                => [Role.permissions]),
            openerp_store::widget!(textarea { rows: 3 }
                => [Employee.description, Department.description, Project.description]),
        ];
        apply_overrides(&mut schema, &overrides);

        // Verify structure.
        assert_eq!(schema["name"], "MiniERP");
        let modules = schema["modules"].as_array().unwrap();
        assert_eq!(modules.len(), 2);
        assert_eq!(modules[0]["id"], "hr");
        assert_eq!(modules[1]["id"], "pm");

        // HR has 3 resources, PM has 1.
        assert_eq!(modules[0]["resources"].as_array().unwrap().len(), 3);
        assert_eq!(modules[1]["resources"].as_array().unwrap().len(), 1);

        // HR hierarchy: 3 top-level items.
        let hr_nav = modules[0]["hierarchy"]["nav"].as_array().unwrap();
        assert_eq!(hr_nav.len(), 3);

        // PM project has 5 CRUD + 2 custom = 7 permissions.
        let pm_perms = &schema["permissions"]["pm"]["project"]["actions"];
        assert_eq!(pm_perms.as_array().unwrap().len(), 7);

        // UI override applied: Role.permissions → permission_picker.
        // Note: schema embeds raw IR where "name" is PascalCase (struct name).
        let role_res = modules[0]["resources"].as_array().unwrap()
            .iter().find(|r| r["name"] == "Role").unwrap();
        let role_fields = role_res["fields"].as_array().unwrap();
        let perm_field = role_fields.iter().find(|f| f["name"] == "permissions").unwrap();
        assert_eq!(perm_field["widget"], "permission_picker");
        assert_eq!(perm_field["source"], "schema.permissions");

        // UI override: description fields → textarea.
        let emp_fields = modules[0]["resources"].as_array().unwrap()
            .iter().find(|r| r["name"] == "Employee").unwrap()["fields"].as_array().unwrap();
        let desc_field = emp_fields.iter().find(|f| f["name"] == "description").unwrap();
        assert_eq!(desc_field["widget"], "textarea");
        assert_eq!(desc_field["rows"], 3);
    }

    // ── 4. before_create hook: email normalization ──

    #[tokio::test]
    async fn mini_erp_hook_email_normalize() {
        let erp = setup_mini_erp();

        let (status, emp) = api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({
                "email": "Alice@Example.COM",
                "active": true,
                "displayName": "Alice",
            })),
            "root",
        ).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(emp["email"], "alice@example.com", "email should be lowercased by before_create");
        assert!(!emp["id"].as_str().unwrap().is_empty(), "id auto-generated");
        assert!(emp["createdAt"].as_str().unwrap().contains("T"));
    }

    // ── 5. before_create hook: default status ──

    #[tokio::test]
    async fn mini_erp_hook_default_status() {
        let erp = setup_mini_erp();

        let (status, proj) = api_call(&erp.pm_router, "POST", "/projects",
            Some(serde_json::json!({
                "ownerId": "emp1",
                "budget": 50000,
                "tags": ["web", "frontend"],
                "displayName": "Website Redesign",
            })),
            "root",
        ).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(proj["status"], "draft", "status defaults to 'draft' via before_create");
        assert_eq!(proj["budget"], 50000);
        assert_eq!(proj["tags"].as_array().unwrap().len(), 2);
    }

    // ── 6. Custom Authenticator: role-based permission ──

    #[tokio::test]
    async fn mini_erp_auth_role_permission() {
        let erp = setup_mini_erp();

        // Setup: create roles via root.
        let (s, _) = api_call(&erp.hr_router, "POST", "/roles",
            Some(serde_json::json!({
                "id": "hr-viewer",
                "permissions": ["hr:employee:list", "hr:employee:read", "hr:department:list"],
                "displayName": "HR Viewer",
            })),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);

        let (s, _) = api_call(&erp.hr_router, "POST", "/roles",
            Some(serde_json::json!({
                "id": "hr-admin",
                "permissions": [
                    "hr:employee:list", "hr:employee:read", "hr:employee:create",
                    "hr:employee:update", "hr:employee:delete",
                    "hr:department:list", "hr:department:read",
                    "hr:department:create", "hr:department:update",
                ],
                "displayName": "HR Admin",
            })),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);

        // Create an employee as root.
        let (s, emp) = api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({
                "email": "bob@company.com", "active": true, "displayName": "Bob",
            })),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);
        let emp_id = emp["id"].as_str().unwrap();

        // hr-viewer: can list employees.
        let (s, list) = api_call(&erp.hr_router, "GET", "/employees", None, "hr-viewer").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(list["items"].as_array().unwrap().len(), 1);

        // hr-viewer: can GET employee.
        let (s, _) = api_call(&erp.hr_router, "GET", &format!("/employees/{}", emp_id), None, "hr-viewer").await;
        assert_eq!(s, StatusCode::OK);

        // hr-viewer: CANNOT create.
        let (s, err) = api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({"email": "x@y.com", "active": true, "displayName": "X"})),
            "hr-viewer",
        ).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "viewer cannot create");
        assert_eq!(err["code"], "PERMISSION_DENIED");

        // hr-viewer: CANNOT delete.
        let (s, _) = api_call(&erp.hr_router, "DELETE", &format!("/employees/{}", emp_id), None, "hr-viewer").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "viewer cannot delete");

        // hr-admin: CAN create.
        let (s, _) = api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({"email": "carol@co.com", "active": true, "displayName": "Carol"})),
            "hr-admin",
        ).await;
        assert_eq!(s, StatusCode::OK);

        // hr-admin: CAN update.
        let (s, emp2) = api_call(&erp.hr_router, "GET", &format!("/employees/{}", emp_id), None, "hr-admin").await;
        assert_eq!(s, StatusCode::OK);
        let mut updated = emp2.clone();
        updated["displayName"] = serde_json::json!("Bob Updated");
        let (s, _) = api_call(&erp.hr_router, "PUT", &format!("/employees/{}", emp_id),
            Some(updated), "hr-admin",
        ).await;
        assert_eq!(s, StatusCode::OK);

        // No token at all → rejected.
        let req = Request::builder().uri("/employees").body(Body::empty()).unwrap();
        let resp = erp.hr_router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "Missing x-roles → rejected");
    }

    // ── 7. Cross-module isolation ──

    #[tokio::test]
    async fn mini_erp_cross_module_isolation() {
        let erp = setup_mini_erp();

        // Create employee in HR module.
        let (s, _) = api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({"email": "iso@co.com", "active": true, "displayName": "Iso"})),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);

        // Create project in PM module.
        let (s, _) = api_call(&erp.pm_router, "POST", "/projects",
            Some(serde_json::json!({"ownerId": "emp1", "budget": 1000, "displayName": "Proj"})),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);

        // HR list: only employees, no projects.
        let (_, list) = api_call(&erp.hr_router, "GET", "/employees", None, "root").await;
        assert_eq!(list["items"].as_array().unwrap().len(), 1);
        assert_eq!(list["items"][0]["email"], "iso@co.com");

        // PM list: only projects, no employees.
        let (_, list) = api_call(&erp.pm_router, "GET", "/projects", None, "root").await;
        assert_eq!(list["items"].as_array().unwrap().len(), 1);
        assert_eq!(list["items"][0]["ownerId"], "emp1");

        // KV prefix isolation: scan hr: and pm: prefixes.
        let hr_entries = erp.kv.scan("hr:employee:").unwrap();
        let pm_entries = erp.kv.scan("pm:project:").unwrap();
        assert_eq!(hr_entries.len(), 1, "HR KV has 1 employee");
        assert_eq!(pm_entries.len(), 1, "PM KV has 1 project");
    }

    // ── 8. List multiple records ──

    #[tokio::test]
    async fn mini_erp_list_multiple() {
        let erp = setup_mini_erp();
        let names = ["Alice", "Bob", "Carol", "Dave", "Eve"];

        for name in &names {
            let (s, _) = api_call(&erp.hr_router, "POST", "/employees",
                Some(serde_json::json!({
                    "email": format!("{}@co.com", name.to_lowercase()),
                    "active": true,
                    "displayName": name,
                })),
                "root",
            ).await;
            assert_eq!(s, StatusCode::OK);
        }

        let (s, list) = api_call(&erp.hr_router, "GET", "/employees", None, "root").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(list["items"].as_array().unwrap().len(), 5);
        assert_eq!(list["hasMore"], false);

        // All emails should be lowercase.
        for item in list["items"].as_array().unwrap() {
            let email = item["email"].as_str().unwrap();
            assert_eq!(email, email.to_lowercase(), "email should be normalized");
        }
    }

    // ── 9. Full edit roundtrip: all fields preserved ──

    #[tokio::test]
    async fn mini_erp_edit_roundtrip() {
        let erp = setup_mini_erp();

        // Create a project with all optional fields.
        let (s, created) = api_call(&erp.pm_router, "POST", "/projects",
            Some(serde_json::json!({
                "ownerId": "emp1",
                "budget": 100000,
                "url": "https://github.com/myproject",
                "tags": ["rust", "erp", "production"],
                "secretToken": "ghp_abc123secret",
                "displayName": "Full Roundtrip",
                "description": "Test all fields survive edit",
            })),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);
        let id = created["id"].as_str().unwrap();
        let created_at = created["createdAt"].as_str().unwrap().to_string();

        std::thread::sleep(std::time::Duration::from_millis(10));

        // Edit: change displayName only, send full record back.
        let mut edit = created.clone();
        edit["displayName"] = serde_json::json!("Roundtrip Edited");

        let (s, updated) = api_call(&erp.pm_router, "PUT", &format!("/projects/{}", id),
            Some(edit), "root",
        ).await;
        assert_eq!(s, StatusCode::OK);

        // All fields preserved.
        assert_eq!(updated["id"], id);
        assert_eq!(updated["displayName"], "Roundtrip Edited");
        assert_eq!(updated["ownerId"], "emp1");
        assert_eq!(updated["budget"], 100000);
        assert_eq!(updated["url"], "https://github.com/myproject");
        assert_eq!(updated["secretToken"], "ghp_abc123secret");
        assert_eq!(updated["tags"].as_array().unwrap().len(), 3);
        assert_eq!(updated["description"], "Test all fields survive edit");
        assert_eq!(updated["status"], "draft");
        // Timestamps.
        assert_eq!(updated["createdAt"], created_at, "createdAt must not change");
        assert_ne!(updated["updatedAt"], created["updatedAt"], "updatedAt must change");

        // Re-fetch to verify persistence.
        let (s, fetched) = api_call(&erp.pm_router, "GET", &format!("/projects/{}", id), None, "root").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["displayName"], "Roundtrip Edited");
        assert_eq!(fetched["secretToken"], "ghp_abc123secret");
        assert_eq!(fetched["createdAt"], created_at);
    }

    // ── 10. before_update hook: email re-normalized on edit ──

    #[tokio::test]
    async fn mini_erp_edit_renormalize() {
        let erp = setup_mini_erp();

        let (s, emp) = api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({
                "email": "test@CO.COM", "active": true, "displayName": "Norm",
            })),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(emp["email"], "test@co.com");
        let id = emp["id"].as_str().unwrap();

        // Edit with uppercase email.
        let mut edit = emp.clone();
        edit["email"] = serde_json::json!("NEW@UPPER.COM");
        let (s, updated) = api_call(&erp.hr_router, "PUT", &format!("/employees/{}", id),
            Some(edit), "root",
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(updated["email"], "new@upper.com", "before_update normalizes email");
    }

    // ── 11. Facet API: external subset ──

    #[tokio::test]
    async fn mini_erp_facet_api() {
        let erp = setup_mini_erp();

        // Create departments.
        for name in &["Engineering", "Marketing", "Sales"] {
            api_call(&erp.hr_router, "POST", "/departments",
                Some(serde_json::json!({
                    "budget": 100000, "displayName": name,
                })),
                "root",
            ).await;
        }

        // Build a facet: "public" API that only exposes department names.
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct PublicDept {
            id: String,
            display_name: Option<String>,
        }

        let facet_kv = erp.kv.clone();
        let facet_router = Router::new()
            .route("/departments", get(move |State(kv): State<Arc<dyn openerp_kv::KVStore>>| async move {
                let ops = KvOps::<Department>::new(kv);
                let all = ops.list().unwrap();
                let items: Vec<PublicDept> = all.iter().map(|d| PublicDept {
                    id: d.id.to_string(),
                    display_name: d.display_name.clone(),
                }).collect();
                Json(ListResult { items, has_more: false })
            }))
            .with_state(facet_kv);

        // Facet returns limited fields.
        let req = Request::builder().uri("/departments").body(Body::empty()).unwrap();
        let resp = facet_router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(list["hasMore"], false);
        assert_eq!(list["items"].as_array().unwrap().len(), 3);

        // Facet items should NOT have budget, parent_id, etc.
        let item = &list["items"][0];
        assert!(item.get("displayName").is_some());
        assert!(item.get("id").is_some());
        assert!(item.get("budget").is_none(), "Facet should not expose budget");
        assert!(item.get("parentId").is_none(), "Facet should not expose parentId");
    }

    // ── 12. Multiple roles combined (union of permissions) ──

    #[tokio::test]
    async fn mini_erp_multi_role_union() {
        let erp = setup_mini_erp();

        // Create two roles.
        api_call(&erp.hr_router, "POST", "/roles",
            Some(serde_json::json!({
                "id": "emp-reader", "permissions": ["hr:employee:list", "hr:employee:read"],
                "displayName": "Reader",
            })),
            "root",
        ).await;
        api_call(&erp.hr_router, "POST", "/roles",
            Some(serde_json::json!({
                "id": "dept-reader", "permissions": ["hr:department:list", "hr:department:read"],
                "displayName": "Dept Reader",
            })),
            "root",
        ).await;

        // Create data.
        api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({"email": "a@b.com", "active": true, "displayName": "A"})),
            "root",
        ).await;
        api_call(&erp.hr_router, "POST", "/departments",
            Some(serde_json::json!({"budget": 1000, "displayName": "Eng"})),
            "root",
        ).await;

        // User with both roles (comma-separated): can list both.
        let (s, _) = api_call(&erp.hr_router, "GET", "/employees", None, "emp-reader,dept-reader").await;
        assert_eq!(s, StatusCode::OK);
        let (s, _) = api_call(&erp.hr_router, "GET", "/departments", None, "emp-reader,dept-reader").await;
        assert_eq!(s, StatusCode::OK);

        // User with only emp-reader: cannot list departments.
        let (s, _) = api_call(&erp.hr_router, "GET", "/departments", None, "emp-reader").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "emp-reader cannot list departments");
    }

    // ── 13. Concurrent creates don't collide (unique auto-ids) ──

    #[tokio::test]
    async fn mini_erp_unique_auto_ids() {
        let erp = setup_mini_erp();
        let mut ids = std::collections::HashSet::new();

        for i in 0..20 {
            let (s, emp) = api_call(&erp.hr_router, "POST", "/employees",
                Some(serde_json::json!({
                    "email": format!("user{}@co.com", i),
                    "active": true,
                    "displayName": format!("User {}", i),
                })),
                "root",
            ).await;
            assert_eq!(s, StatusCode::OK);
            let id = emp["id"].as_str().unwrap().to_string();
            assert!(ids.insert(id.clone()), "ID {} should be unique", id);
        }

        assert_eq!(ids.len(), 20, "All 20 IDs should be unique");

        let (_, list) = api_call(&erp.hr_router, "GET", "/employees", None, "root").await;
        assert_eq!(list["items"].as_array().unwrap().len(), 20);
    }

    // =====================================================================
    // Extended models: Document, CompanyProfile — enterprise data
    // =====================================================================

    #[model(module = "km")]
    pub struct Document {
        pub id: Id,
        pub title: String,
        pub content: Option<String>,
        pub author_id: Id,
        pub visibility: String,
        pub tags: Vec<String>,
        pub attachment_url: Option<Url>,
        pub published: bool,
        pub version: u32,
    }

    impl KvStore for Document {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "km:document:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
            if self.visibility.is_empty() { self.visibility = "private".into(); }
            if self.version == 0 { self.version = 1; }
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
        fn before_update(&mut self) {
            self.version += 1;
            self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
        }
    }

    #[model(module = "org")]
    pub struct CompanyProfile {
        pub id: Id,
        pub legal_name: String,
        pub trade_name: Option<String>,
        pub tax_id: Option<String>,
        pub address: Option<String>,
        pub phone: Option<String>,
        pub website: Option<Url>,
        pub logo: Option<Avatar>,
        pub founded_date: Option<DateTime>,
        pub employee_count: u32,
        pub industry: Option<String>,
        pub annual_revenue: Option<u64>,
    }

    impl KvStore for CompanyProfile {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "org:company:" }
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
    // Extended MiniErp with all modules
    // =====================================================================

    struct FullErp {
        kv: Arc<dyn openerp_kv::KVStore>,
        hr_router: Router,
        pm_router: Router,
        km_router: Router,
        org_router: Router,
        _dir: tempfile::TempDir,
    }

    fn setup_full_erp() -> FullErp {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("full.redb")).unwrap(),
        );
        let auth: Arc<dyn Authenticator> = Arc::new(MiniAuth { kv: kv.clone() });

        let mut hr_router = Router::new();
        hr_router = hr_router.merge(admin_kv_router(KvOps::<Employee>::new(kv.clone()), auth.clone(), "hr", "employees", "employee"));
        hr_router = hr_router.merge(admin_kv_router(KvOps::<Department>::new(kv.clone()), auth.clone(), "hr", "departments", "department"));
        hr_router = hr_router.merge(admin_kv_router(KvOps::<Role>::new(kv.clone()), auth.clone(), "hr", "roles", "role"));

        let pm_router = admin_kv_router(KvOps::<Project>::new(kv.clone()), auth.clone(), "pm", "projects", "project");
        let km_router = admin_kv_router(KvOps::<Document>::new(kv.clone()), auth.clone(), "km", "documents", "document");
        let org_router = admin_kv_router(KvOps::<CompanyProfile>::new(kv.clone()), auth.clone(), "org", "companies", "company");

        FullErp { kv, hr_router, pm_router, km_router, org_router, _dir: dir }
    }

    /// Helper to seed roles into KV.
    fn seed_role(kv: &Arc<dyn openerp_kv::KVStore>, id: &str, perms: &[&str]) {
        let ops = KvOps::<Role>::new(kv.clone());
        ops.save_new(Role {
            id: Id::new(id),
            permissions: perms.iter().map(|s| s.to_string()).collect(),
            display_name: Some(id.into()),
            description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();
    }

    // ── 14. Per-CRUD permission granularity ──

    #[tokio::test]
    async fn auth_per_crud_granularity() {
        let erp = setup_full_erp();

        // Roles with single CRUD permissions.
        seed_role(&erp.kv, "only-create", &["km:document:create"]);
        seed_role(&erp.kv, "only-read", &["km:document:read"]);
        seed_role(&erp.kv, "only-list", &["km:document:list"]);
        seed_role(&erp.kv, "only-update", &["km:document:update", "km:document:read"]);
        seed_role(&erp.kv, "only-delete", &["km:document:delete", "km:document:read"]);

        // Create a document as root.
        let (s, doc) = api_call(&erp.km_router, "POST", "/documents",
            Some(serde_json::json!({
                "title": "Architecture Guide",
                "content": "# Chapter 1\nIntroduction...",
                "authorId": "emp1",
                "tags": ["architecture", "guide"],
                "displayName": "Arch Guide",
            })),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);
        let doc_id = doc["id"].as_str().unwrap();

        // only-create: can create but NOT read/list/update/delete.
        let (s, _) = api_call(&erp.km_router, "POST", "/documents",
            Some(serde_json::json!({"title": "Draft", "authorId": "emp1", "displayName": "D"})),
            "only-create",
        ).await;
        assert_eq!(s, StatusCode::OK, "only-create can create");
        let (s, _) = api_call(&erp.km_router, "GET", "/documents", None, "only-create").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "only-create cannot list");
        let (s, _) = api_call(&erp.km_router, "GET", &format!("/documents/{}", doc_id), None, "only-create").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "only-create cannot read");

        // only-read: can GET single but NOT list/create/update/delete.
        let (s, _) = api_call(&erp.km_router, "GET", &format!("/documents/{}", doc_id), None, "only-read").await;
        assert_eq!(s, StatusCode::OK, "only-read can read");
        let (s, _) = api_call(&erp.km_router, "GET", "/documents", None, "only-read").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "only-read cannot list");
        let (s, _) = api_call(&erp.km_router, "DELETE", &format!("/documents/{}", doc_id), None, "only-read").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "only-read cannot delete");

        // only-list: can list but NOT read single.
        let (s, list) = api_call(&erp.km_router, "GET", "/documents", None, "only-list").await;
        assert_eq!(s, StatusCode::OK, "only-list can list");
        assert!(list["items"].as_array().unwrap().len() >= 2);
        let (s, _) = api_call(&erp.km_router, "GET", &format!("/documents/{}", doc_id), None, "only-list").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "only-list cannot read single");

        // only-update: can read + update but NOT create/delete.
        let (s, d) = api_call(&erp.km_router, "GET", &format!("/documents/{}", doc_id), None, "only-update").await;
        assert_eq!(s, StatusCode::OK);
        let mut edit = d.clone();
        edit["title"] = serde_json::json!("Updated Guide");
        let (s, _) = api_call(&erp.km_router, "PUT", &format!("/documents/{}", doc_id), Some(edit), "only-update").await;
        assert_eq!(s, StatusCode::OK, "only-update can update");
        let (s, _) = api_call(&erp.km_router, "POST", "/documents",
            Some(serde_json::json!({"title": "X", "authorId": "e", "displayName": "X"})),
            "only-update",
        ).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "only-update cannot create");

        // only-delete: can read + delete but NOT create/update.
        let (s, _) = api_call(&erp.km_router, "DELETE", &format!("/documents/{}", doc_id), None, "only-delete").await;
        assert_eq!(s, StatusCode::OK, "only-delete can delete");
    }

    // ── 15. Cross-module permission isolation ──

    #[tokio::test]
    async fn auth_cross_module_isolation() {
        let erp = setup_full_erp();

        // Role that only has HR permissions, nothing else.
        seed_role(&erp.kv, "hr-only", &[
            "hr:employee:list", "hr:employee:read", "hr:employee:create",
        ]);
        // Role that only has KM permissions.
        seed_role(&erp.kv, "km-only", &[
            "km:document:list", "km:document:read", "km:document:create",
        ]);

        // Create data in each module as root.
        api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({"email": "a@b.com", "active": true, "displayName": "A"})),
            "root",
        ).await;
        api_call(&erp.km_router, "POST", "/documents",
            Some(serde_json::json!({"title": "Doc1", "authorId": "e1", "displayName": "D1"})),
            "root",
        ).await;
        api_call(&erp.org_router, "POST", "/companies",
            Some(serde_json::json!({"legalName": "Acme Inc", "employeeCount": 50, "displayName": "Acme"})),
            "root",
        ).await;

        // hr-only: can access HR, blocked from KM and Org.
        let (s, _) = api_call(&erp.hr_router, "GET", "/employees", None, "hr-only").await;
        assert_eq!(s, StatusCode::OK, "hr-only can list employees");
        let (s, _) = api_call(&erp.km_router, "GET", "/documents", None, "hr-only").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "hr-only blocked from KM documents");
        let (s, _) = api_call(&erp.org_router, "GET", "/companies", None, "hr-only").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "hr-only blocked from Org companies");

        // km-only: can access KM, blocked from HR and Org.
        let (s, _) = api_call(&erp.km_router, "GET", "/documents", None, "km-only").await;
        assert_eq!(s, StatusCode::OK, "km-only can list documents");
        let (s, _) = api_call(&erp.hr_router, "GET", "/employees", None, "km-only").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "km-only blocked from HR employees");
        let (s, _) = api_call(&erp.pm_router, "GET", "/projects", None, "km-only").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "km-only blocked from PM projects");
    }

    // ── 16. Document version auto-increment on update ──

    #[tokio::test]
    async fn document_version_increment() {
        let erp = setup_full_erp();

        let (s, doc) = api_call(&erp.km_router, "POST", "/documents",
            Some(serde_json::json!({
                "title": "Versioned Doc",
                "authorId": "e1",
                "displayName": "V1",
            })),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(doc["version"], 1, "Initial version should be 1");
        let id = doc["id"].as_str().unwrap();

        // First update → version 2.
        let mut edit = doc.clone();
        edit["title"] = serde_json::json!("Versioned Doc v2");
        let (s, v2) = api_call(&erp.km_router, "PUT", &format!("/documents/{}", id), Some(edit), "root").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v2["version"], 2, "First update → version 2");

        // Second update → version 3.
        let mut edit = v2.clone();
        edit["content"] = serde_json::json!("Updated content");
        let (s, v3) = api_call(&erp.km_router, "PUT", &format!("/documents/{}", id), Some(edit), "root").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v3["version"], 3, "Second update → version 3");

        // Verify via GET.
        let (s, fetched) = api_call(&erp.km_router, "GET", &format!("/documents/{}", id), None, "root").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["version"], 3);
        assert_eq!(fetched["title"], "Versioned Doc v2");
        assert_eq!(fetched["content"], "Updated content");
    }

    // ── 17. CompanyProfile full CRUD ──

    #[tokio::test]
    async fn company_profile_crud() {
        let erp = setup_full_erp();

        // Create.
        let (s, co) = api_call(&erp.org_router, "POST", "/companies",
            Some(serde_json::json!({
                "legalName": "Haivivi Technology Co., Ltd.",
                "tradeName": "Haivivi",
                "taxId": "91440300MA5G5XKT8J",
                "address": "Shenzhen, Guangdong, China",
                "phone": "+86-755-12345678",
                "website": "https://haivivi.com",
                "employeeCount": 120,
                "industry": "IoT & Consumer Electronics",
                "annualRevenue": 50000000,
                "displayName": "Haivivi",
                "description": "Smart home device company",
            })),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);
        let id = co["id"].as_str().unwrap();
        assert_eq!(co["legalName"], "Haivivi Technology Co., Ltd.");
        assert_eq!(co["website"], "https://haivivi.com");
        assert_eq!(co["employeeCount"], 120);
        assert_eq!(co["annualRevenue"], 50000000u64);

        // Read.
        let (s, fetched) = api_call(&erp.org_router, "GET", &format!("/companies/{}", id), None, "root").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(fetched["taxId"], "91440300MA5G5XKT8J");
        assert_eq!(fetched["industry"], "IoT & Consumer Electronics");

        // Update.
        let mut edit = fetched.clone();
        edit["employeeCount"] = serde_json::json!(150);
        edit["annualRevenue"] = serde_json::json!(80000000u64);
        let (s, updated) = api_call(&erp.org_router, "PUT", &format!("/companies/{}", id), Some(edit), "root").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(updated["employeeCount"], 150);
        assert_eq!(updated["annualRevenue"], 80000000u64);
        assert_eq!(updated["legalName"], "Haivivi Technology Co., Ltd.", "other fields preserved");

        // Delete.
        let (s, _) = api_call(&erp.org_router, "DELETE", &format!("/companies/{}", id), None, "root").await;
        assert_eq!(s, StatusCode::OK);
        let (s, _) = api_call(&erp.org_router, "GET", &format!("/companies/{}", id), None, "root").await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }

    // ── 18. Document with tags and visibility ──

    #[tokio::test]
    async fn document_tags_and_visibility() {
        let erp = setup_full_erp();

        // Create document with all fields.
        let (s, doc) = api_call(&erp.km_router, "POST", "/documents",
            Some(serde_json::json!({
                "title": "API Design Principles",
                "content": "# REST API Design\n\n## Principles\n1. Resources\n2. Verbs\n3. Status codes",
                "authorId": "emp42",
                "tags": ["api", "design", "rest", "best-practices"],
                "attachmentUrl": "https://cdn.example.com/docs/api-design.pdf",
                "published": true,
                "displayName": "API Design",
                "description": "Company API design guidelines",
            })),
            "root",
        ).await;
        assert_eq!(s, StatusCode::OK);
        let id = doc["id"].as_str().unwrap();
        assert_eq!(doc["visibility"], "private", "default visibility from before_create");
        assert_eq!(doc["version"], 1);
        assert_eq!(doc["published"], true);
        assert_eq!(doc["tags"].as_array().unwrap().len(), 4);
        assert_eq!(doc["attachmentUrl"], "https://cdn.example.com/docs/api-design.pdf");

        // Update: change visibility and add tag.
        let mut edit = doc.clone();
        edit["visibility"] = serde_json::json!("public");
        edit["tags"] = serde_json::json!(["api", "design", "rest", "best-practices", "v2"]);
        let (s, updated) = api_call(&erp.km_router, "PUT", &format!("/documents/{}", id), Some(edit), "root").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(updated["visibility"], "public");
        assert_eq!(updated["tags"].as_array().unwrap().len(), 5);
        assert_eq!(updated["version"], 2, "version incremented by before_update");
    }

    // ── 19. Multiple roles with overlapping permissions ──

    #[tokio::test]
    async fn auth_overlapping_role_permissions() {
        let erp = setup_full_erp();

        seed_role(&erp.kv, "role-a", &["hr:employee:list", "hr:employee:read", "km:document:list"]);
        seed_role(&erp.kv, "role-b", &["hr:employee:list", "hr:department:list", "km:document:read"]);
        // Union: employee:list+read, department:list, document:list+read

        api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({"email": "x@y.com", "active": true, "displayName": "X"})),
            "root").await;
        api_call(&erp.hr_router, "POST", "/departments",
            Some(serde_json::json!({"budget": 1000, "displayName": "Eng"})),
            "root").await;
        api_call(&erp.km_router, "POST", "/documents",
            Some(serde_json::json!({"title": "D", "authorId": "e", "displayName": "D"})),
            "root").await;

        // User with both roles can access all unions.
        let (s, _) = api_call(&erp.hr_router, "GET", "/employees", None, "role-a,role-b").await;
        assert_eq!(s, StatusCode::OK);
        let (s, _) = api_call(&erp.hr_router, "GET", "/departments", None, "role-a,role-b").await;
        assert_eq!(s, StatusCode::OK, "department:list from role-b");
        let (s, _) = api_call(&erp.km_router, "GET", "/documents", None, "role-a,role-b").await;
        assert_eq!(s, StatusCode::OK, "document:list from role-a");

        // But neither role has create permission.
        let (s, _) = api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({"email":"z@z.com","active":true,"displayName":"Z"})),
            "role-a,role-b").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "no create permission in union");
    }

    // ── 20. Empty/invalid role → no permissions ──

    #[tokio::test]
    async fn auth_empty_role() {
        let erp = setup_full_erp();

        // Role with empty permissions.
        seed_role(&erp.kv, "empty-role", &[]);

        api_call(&erp.hr_router, "POST", "/employees",
            Some(serde_json::json!({"email": "a@b.com", "active": true, "displayName": "A"})),
            "root").await;

        // Empty role can't do anything.
        let (s, _) = api_call(&erp.hr_router, "GET", "/employees", None, "empty-role").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "empty role has no permissions");

        // Non-existent role in header.
        let (s, _) = api_call(&erp.hr_router, "GET", "/employees", None, "nonexistent-role").await;
        assert_eq!(s, StatusCode::FORBIDDEN, "non-existent role has no permissions");
    }

    // ── 21. 4-module schema generation ──

    #[test]
    fn full_erp_schema_4_modules() {
        let schema = build_schema(
            "FullERP",
            vec![
                ModuleDef {
                    id: "hr", label: "Human Resources", icon: "users",
                    resources: vec![
                        ResourceDef::from_ir("hr", Employee::__dsl_ir()),
                        ResourceDef::from_ir("hr", Department::__dsl_ir()),
                        ResourceDef::from_ir("hr", Role::__dsl_ir()),
                    ],
                    hierarchy: vec![
                        HierarchyNode { resource: "employee", label: "Employees", icon: "users",
                            description: "", children: vec![HierarchyNode::leaf("role", "Roles", "shield", "")] },
                        HierarchyNode::leaf("department", "Departments", "building", ""),
                    ],
                },
                ModuleDef {
                    id: "pm", label: "Projects", icon: "folder",
                    resources: vec![
                        ResourceDef::from_ir("pm", Project::__dsl_ir())
                            .with_action("pm", "archive").with_action("pm", "publish"),
                    ],
                    hierarchy: vec![HierarchyNode::leaf("project", "Projects", "folder", "")],
                },
                ModuleDef {
                    id: "km", label: "Knowledge", icon: "book",
                    resources: vec![
                        ResourceDef::from_ir("km", Document::__dsl_ir())
                            .with_action("km", "publish").with_action("km", "archive"),
                    ],
                    hierarchy: vec![HierarchyNode::leaf("document", "Documents", "file-text", "")],
                },
                ModuleDef {
                    id: "org", label: "Organization", icon: "building",
                    resources: vec![
                        ResourceDef::from_ir("org", CompanyProfile::__dsl_ir()),
                    ],
                    hierarchy: vec![HierarchyNode::leaf("company_profile", "Companies", "building", "")],
                },
            ],
        );

        let modules = schema["modules"].as_array().unwrap();
        assert_eq!(modules.len(), 4);

        // Verify module IDs.
        let ids: Vec<&str> = modules.iter().map(|m| m["id"].as_str().unwrap()).collect();
        assert_eq!(ids, vec!["hr", "pm", "km", "org"]);

        // Verify resource counts.
        assert_eq!(modules[0]["resources"].as_array().unwrap().len(), 3); // HR: 3
        assert_eq!(modules[1]["resources"].as_array().unwrap().len(), 1); // PM: 1
        assert_eq!(modules[2]["resources"].as_array().unwrap().len(), 1); // KM: 1
        assert_eq!(modules[3]["resources"].as_array().unwrap().len(), 1); // Org: 1

        // PM project: 5 CRUD + archive + publish = 7 permissions.
        let pm_perms = &schema["permissions"]["pm"]["project"]["actions"];
        assert_eq!(pm_perms.as_array().unwrap().len(), 7);

        // KM document: 5 CRUD + publish + archive = 7 permissions.
        let km_perms = &schema["permissions"]["km"]["document"]["actions"];
        assert_eq!(km_perms.as_array().unwrap().len(), 7);

        // Org company_profile: 5 CRUD only.
        let org_perms = &schema["permissions"]["org"]["company_profile"]["actions"];
        assert_eq!(org_perms.as_array().unwrap().len(), 5);

        // HR hierarchy: employee has child "role".
        let hr_nav = modules[0]["hierarchy"]["nav"].as_array().unwrap();
        assert_eq!(hr_nav[0]["resource"], "employee");
        assert_eq!(hr_nav[0]["children"][0]["resource"], "role");
    }
}
