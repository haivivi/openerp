//! Client golden tests — verify the generated HTTP client against a real server.
//!
//! Starts an axum HTTP server with JWT authentication, then exercises
//! every `ResourceClient` method through actual HTTP requests.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::http::HeaderMap;
    use axum::routing::post;
    use axum::Router;
    use jsonwebtoken::{encode, decode, EncodingKey, DecodingKey, Header, Validation};
    use serde::{Deserialize, Serialize};

    use openerp_client::{ApiError, PasswordLogin, ResourceClient, StaticToken, TokenSource};
    use openerp_core::{Authenticator, ServiceError};
    use openerp_macro::model;
    use openerp_store::{admin_kv_router, KvOps, KvStore};
    use openerp_types::*;

    // =====================================================================
    // Test models
    // =====================================================================

    #[model(module = "hr")]
    pub struct Employee {
        pub id: Id,
        pub email: Email,
        pub active: bool,
        pub salary: Option<u64>,
    }

    impl KvStore for Employee {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "hr:employee:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
            self.email = Email::new(&self.email.as_str().to_lowercase());
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
        fn before_update(&mut self) {
            self.email = Email::new(&self.email.as_str().to_lowercase());
            self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
        }
    }

    #[model(module = "pm")]
    pub struct Project {
        pub id: Id,
        pub owner_id: Id,
        pub status: String,
        pub budget: u64,
        pub tags: Vec<String>,
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
    // JWT auth infrastructure (self-contained for golden tests)
    // =====================================================================

    const JWT_SECRET: &str = "golden-test-secret-key-for-jwt";
    const ROOT_PASSWORD: &str = "golden-root-pw";

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        roles: Vec<String>,
        exp: i64,
    }

    fn sign_jwt(sub: &str, roles: Vec<String>, expire_secs: i64) -> String {
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: sub.to_string(),
            roles,
            exp: now + expire_secs,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
        ).unwrap()
    }

    /// JWT-based Authenticator: extracts Bearer token, verifies JWT,
    /// and checks if the "root" role is present (root bypasses all).
    struct JwtAuth;

    impl Authenticator for JwtAuth {
        fn check(&self, headers: &HeaderMap, _permission: &str) -> Result<(), ServiceError> {
            let token = headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .ok_or_else(|| ServiceError::Validation("missing Bearer token".into()))?;

            let token_data = decode::<Claims>(
                token,
                &DecodingKey::from_secret(JWT_SECRET.as_bytes()),
                &Validation::default(),
            ).map_err(|e| ServiceError::Validation(format!("invalid token: {}", e)))?;

            // Root role bypasses permission checks.
            if token_data.claims.roles.contains(&"root".to_string()) {
                return Ok(());
            }

            Err(ServiceError::Validation("insufficient permissions".into()))
        }
    }

    /// Login endpoint handler for the test server.
    #[derive(Deserialize)]
    struct LoginRequest {
        username: String,
        password: String,
    }

    async fn login_handler(
        axum::Json(body): axum::Json<LoginRequest>,
    ) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::response::IntoResponse;

        if body.username == "root" && body.password == ROOT_PASSWORD {
            let token = sign_jwt("root", vec!["root".into()], 3600);
            (StatusCode::OK, axum::Json(serde_json::json!({
                "access_token": token,
                "token_type": "Bearer",
                "expires_in": 3600u64,
            }))).into_response()
        } else {
            (StatusCode::UNAUTHORIZED, axum::Json(serde_json::json!({
                "error": "invalid credentials",
            }))).into_response()
        }
    }

    // =====================================================================
    // Test server setup
    // =====================================================================

    struct TestServer {
        base_url: String,
        _dir: tempfile::TempDir,
    }

    async fn start_test_server() -> TestServer {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("client-test.redb")).unwrap(),
        );
        let auth: Arc<dyn Authenticator> = Arc::new(JwtAuth);

        let mut app = Router::new();

        // Login endpoint.
        app = app.route("/auth/login", post(login_handler));

        // Admin CRUD routes.
        app = app.nest("/admin/hr", admin_kv_router(
            KvOps::<Employee>::new(kv.clone()), auth.clone(), "hr", "employees", "employee",
        ));
        app = app.nest("/admin/pm", admin_kv_router(
            KvOps::<Project>::new(kv.clone()), auth.clone(), "pm", "projects", "project",
        ));

        // Bind to random port.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Wait for server to be ready.
        let client = reqwest::Client::new();
        for _ in 0..50 {
            if client.get(&format!("{}/auth/login", base_url)).send().await.is_ok() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        TestServer { base_url, _dir: dir }
    }

    // =====================================================================
    // DslModel trait verification
    // =====================================================================

    #[test]
    fn dsl_model_metadata() {
        assert_eq!(Employee::module(), "hr");
        assert_eq!(Employee::resource(), "employee");
        assert_eq!(Employee::resource_path(), "employees");

        assert_eq!(Project::module(), "pm");
        assert_eq!(Project::resource(), "project");
        assert_eq!(Project::resource_path(), "projects");
    }

    #[test]
    fn dsl_model_path_const() {
        assert_eq!(Employee::__DSL_PATH, "employees");
        assert_eq!(Project::__DSL_PATH, "projects");
    }

    // =====================================================================
    // PasswordLogin token source
    // =====================================================================

    #[tokio::test]
    async fn password_login_success() {
        let server = start_test_server().await;
        let ts = PasswordLogin::new(&server.base_url, "root", ROOT_PASSWORD);

        let token = ts.token().await.unwrap();
        assert!(token.is_some(), "should get a JWT");

        let jwt = token.unwrap();
        assert!(!jwt.is_empty());
        assert!(jwt.contains('.'), "JWT should have dot-separated parts");

        // Second call should return cached token (same value).
        let token2 = ts.token().await.unwrap().unwrap();
        assert_eq!(jwt, token2, "should return cached token");
    }

    #[tokio::test]
    async fn password_login_bad_credentials() {
        let server = start_test_server().await;
        let ts = PasswordLogin::new(&server.base_url, "root", "wrong-password");

        let err = ts.token().await.unwrap_err();
        match err {
            ApiError::Auth(msg) => assert!(msg.contains("login failed"), "got: {}", msg),
            other => panic!("expected Auth error, got: {:?}", other),
        }
    }

    // =====================================================================
    // ResourceClient: full CRUD lifecycle
    // =====================================================================

    #[tokio::test]
    async fn client_employee_crud_lifecycle() {
        let server = start_test_server().await;
        let ts = Arc::new(PasswordLogin::new(&server.base_url, "root", ROOT_PASSWORD));
        let client = ResourceClient::<Employee>::new(&server.base_url, ts);

        // 1. List: empty.
        let list = client.list().await.unwrap();
        assert_eq!(list.total, 0);
        assert!(list.items.is_empty());

        // 2. Create.
        let emp = Employee {
            id: Id::default(),
            email: Email::new("Alice@Example.COM"),
            active: true,
            salary: Some(80000),
            display_name: Some("Alice".into()),
            description: None,
            metadata: None,
            created_at: DateTime::default(),
            updated_at: DateTime::default(),
        };
        let created = client.create(&emp).await.unwrap();
        assert!(!created.id.is_empty(), "server should auto-generate id");
        assert_eq!(created.email.as_str(), "alice@example.com", "before_create normalizes email");
        assert_eq!(created.salary, Some(80000));
        assert!(created.created_at.as_str().contains("T"), "should have ISO timestamp");

        let id = created.id.to_string();

        // 3. Get.
        let fetched = client.get(&id).await.unwrap();
        assert_eq!(fetched.id.as_str(), id);
        assert_eq!(fetched.email.as_str(), "alice@example.com");
        assert_eq!(fetched.display_name, Some("Alice".into()));

        // 4. Update.
        let mut updated = fetched.clone();
        updated.display_name = Some("Alice Updated".into());
        updated.salary = Some(90000);
        let result = client.update(&id, &updated).await.unwrap();
        assert_eq!(result.display_name, Some("Alice Updated".into()));
        assert_eq!(result.salary, Some(90000));
        assert_eq!(result.created_at, created.created_at, "created_at must not change");

        // 5. List: one item.
        let list = client.list().await.unwrap();
        assert_eq!(list.total, 1);
        assert_eq!(list.items[0].id.as_str(), id);

        // 6. Delete.
        client.delete(&id).await.unwrap();

        // 7. List: empty again.
        let list = client.list().await.unwrap();
        assert_eq!(list.total, 0);

        // 8. Get deleted: should error.
        let err = client.get(&id).await.unwrap_err();
        match err {
            ApiError::Server { status, .. } => assert_eq!(status, 404),
            other => panic!("expected 404, got: {:?}", other),
        }
    }

    // =====================================================================
    // Cross-module isolation via client
    // =====================================================================

    #[tokio::test]
    async fn client_cross_module_isolation() {
        let server = start_test_server().await;
        let ts = Arc::new(PasswordLogin::new(&server.base_url, "root", ROOT_PASSWORD));
        let emp_client = ResourceClient::<Employee>::new(&server.base_url, ts.clone());
        let proj_client = ResourceClient::<Project>::new(&server.base_url, ts);

        // Create employee.
        let emp = Employee {
            id: Id::default(), email: Email::new("iso@co.com"), active: true,
            salary: None, display_name: Some("Iso".into()),
            description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        emp_client.create(&emp).await.unwrap();

        // Create project.
        let proj = Project {
            id: Id::default(), owner_id: Id::new("emp1"), status: String::new(),
            budget: 50000, tags: vec!["rust".into()],
            display_name: Some("Client Test".into()),
            description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        proj_client.create(&proj).await.unwrap();

        // Each module sees only its own records.
        let emps = emp_client.list().await.unwrap();
        assert_eq!(emps.total, 1);
        assert_eq!(emps.items[0].email.as_str(), "iso@co.com");

        let projs = proj_client.list().await.unwrap();
        assert_eq!(projs.total, 1);
        assert_eq!(projs.items[0].budget, 50000);
    }

    // =====================================================================
    // Multiple records + before_create hooks
    // =====================================================================

    #[tokio::test]
    async fn client_multiple_records() {
        let server = start_test_server().await;
        let ts = Arc::new(PasswordLogin::new(&server.base_url, "root", ROOT_PASSWORD));
        let client = ResourceClient::<Employee>::new(&server.base_url, ts);

        let names = ["Alice", "Bob", "Carol", "Dave", "Eve"];
        let mut ids = Vec::new();

        for name in &names {
            let emp = Employee {
                id: Id::default(),
                email: Email::new(&format!("{}@co.com", name.to_lowercase())),
                active: true, salary: None,
                display_name: Some(name.to_string()),
                description: None, metadata: None,
                created_at: DateTime::default(), updated_at: DateTime::default(),
            };
            let created = client.create(&emp).await.unwrap();
            ids.push(created.id.to_string());
        }

        // All unique IDs.
        let unique: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
        assert_eq!(unique.len(), 5, "all IDs should be unique");

        // List returns all 5.
        let list = client.list().await.unwrap();
        assert_eq!(list.total, 5);

        // Get each by ID.
        for (i, id) in ids.iter().enumerate() {
            let emp = client.get(id).await.unwrap();
            assert_eq!(emp.display_name, Some(names[i].to_string()));
            // Email lowercased by hook.
            assert_eq!(emp.email.as_str(), format!("{}@co.com", names[i].to_lowercase()));
        }
    }

    // =====================================================================
    // Edit roundtrip: all fields preserved
    // =====================================================================

    #[tokio::test]
    async fn client_edit_roundtrip() {
        let server = start_test_server().await;
        let ts = Arc::new(PasswordLogin::new(&server.base_url, "root", ROOT_PASSWORD));
        let client = ResourceClient::<Project>::new(&server.base_url, ts);

        let proj = Project {
            id: Id::default(), owner_id: Id::new("emp1"), status: String::new(),
            budget: 100000, tags: vec!["rust".into(), "erp".into(), "production".into()],
            display_name: Some("Full Roundtrip".into()),
            description: Some("All fields survive edit".into()),
            metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        let created = client.create(&proj).await.unwrap();
        let id = created.id.to_string();
        let created_at = created.created_at.clone();

        // Edit only display_name.
        let mut edit = created.clone();
        edit.display_name = Some("Roundtrip Edited".into());
        let updated = client.update(&id, &edit).await.unwrap();

        // All fields preserved.
        assert_eq!(updated.display_name, Some("Roundtrip Edited".into()));
        assert_eq!(updated.owner_id.as_str(), "emp1");
        assert_eq!(updated.budget, 100000);
        assert_eq!(updated.tags.len(), 3);
        assert_eq!(updated.description, Some("All fields survive edit".into()));
        assert_eq!(updated.status, "draft");
        assert_eq!(updated.created_at, created_at, "created_at must not change");

        // Re-fetch.
        let fetched = client.get(&id).await.unwrap();
        assert_eq!(fetched.display_name, Some("Roundtrip Edited".into()));
        assert_eq!(fetched.tags, vec!["rust", "erp", "production"]);
    }

    // =====================================================================
    // Auth: unauthenticated request fails
    // =====================================================================

    #[tokio::test]
    async fn client_no_auth_rejected() {
        let server = start_test_server().await;
        let client = ResourceClient::<Employee>::new(
            &server.base_url,
            Arc::new(openerp_client::NoAuth),
        );

        let err = client.list().await.unwrap_err();
        match err {
            ApiError::Server { status, .. } => assert_eq!(status, 400),
            other => panic!("expected 400 (missing token), got: {:?}", other),
        }
    }

    // =====================================================================
    // Auth: invalid token fails
    // =====================================================================

    #[tokio::test]
    async fn client_bad_token_rejected() {
        let server = start_test_server().await;
        let client = ResourceClient::<Employee>::new(
            &server.base_url,
            Arc::new(StaticToken::new("not-a-valid-jwt")),
        );

        let err = client.list().await.unwrap_err();
        match err {
            ApiError::Server { status, .. } => assert_eq!(status, 400),
            other => panic!("expected 400 (invalid token), got: {:?}", other),
        }
    }

    // =====================================================================
    // Token source shared across two ResourceClients
    // =====================================================================

    #[tokio::test]
    async fn client_shared_token_source() {
        let server = start_test_server().await;
        let ts: Arc<dyn TokenSource> = Arc::new(
            PasswordLogin::new(&server.base_url, "root", ROOT_PASSWORD),
        );

        // Same token source, two different resource clients.
        let emp_client = ResourceClient::<Employee>::new(&server.base_url, ts.clone());
        let proj_client = ResourceClient::<Project>::new(&server.base_url, ts);

        // Both should work (token cached after first login).
        let emp = Employee {
            id: Id::default(), email: Email::new("shared@co.com"), active: true,
            salary: None, display_name: Some("Shared".into()),
            description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        emp_client.create(&emp).await.unwrap();

        let proj = Project {
            id: Id::default(), owner_id: Id::new("e1"), status: String::new(),
            budget: 1000, tags: vec![],
            display_name: Some("Shared".into()),
            description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        proj_client.create(&proj).await.unwrap();

        assert_eq!(emp_client.list().await.unwrap().total, 1);
        assert_eq!(proj_client.list().await.unwrap().total, 1);
    }

    // =====================================================================
    // Error handling: delete nonexistent -> 404
    // =====================================================================

    #[tokio::test]
    async fn client_delete_nonexistent() {
        let server = start_test_server().await;
        let ts = Arc::new(PasswordLogin::new(&server.base_url, "root", ROOT_PASSWORD));
        let client = ResourceClient::<Employee>::new(&server.base_url, ts);

        let err = client.delete("nonexistent-id").await.unwrap_err();
        match err {
            ApiError::Server { status, .. } => assert_eq!(status, 404),
            other => panic!("expected 404, got: {:?}", other),
        }
    }

    // =====================================================================
    // Error handling: update nonexistent -> 404
    // =====================================================================

    #[tokio::test]
    async fn client_update_nonexistent() {
        let server = start_test_server().await;
        let ts = Arc::new(PasswordLogin::new(&server.base_url, "root", ROOT_PASSWORD));
        let client = ResourceClient::<Employee>::new(&server.base_url, ts);

        let emp = Employee {
            id: Id::new("ghost"), email: Email::new("g@g.com"), active: true,
            salary: None, display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        let err = client.update("ghost", &emp).await.unwrap_err();
        match err {
            ApiError::Server { status, .. } => assert_eq!(status, 404),
            other => panic!("expected 404, got: {:?}", other),
        }
    }
}
