//! Flux FFI — C-compatible API for cross-platform bindings.
//!
//! Architecture:
//! 1. flux_create() starts an embedded HTTP server (twitterd) on a random port
//! 2. BFF handlers use generated HTTP client to call the server
//! 3. Admin dashboard accessible at http://<lan-ip>:<port>/dashboard
//! 4. iOS/Android/Desktop all share the same backend data

use std::any::Any;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::Arc;

use openerp_flux::{Flux, StateStore, StateValue, SubscriptionId};

use flux_golden::state::*;
use flux_golden::request::*;
use flux_golden::handlers::TwitterBff;

/// Opaque handle to a Flux instance + embedded server.
pub struct FluxHandle {
    flux: Flux,
    _bff: Arc<TwitterBff>,
    rt: tokio::runtime::Runtime,
    /// The server URL (e.g. "http://192.168.1.100:3000").
    server_url: CString,
}

/// Byte buffer returned from FFI calls.
#[repr(C)]
pub struct FluxBytes {
    pub ptr: *const u8,
    pub len: usize,
}

// ============================================================================
// Lifecycle
// ============================================================================

/// Create a new Flux instance.
/// Starts an embedded HTTP server with admin dashboard + REST API.
/// Returns an opaque handle. Must be freed with `flux_free`.
#[no_mangle]
pub extern "C" fn flux_create() -> *mut FluxHandle {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .expect("failed to create tokio runtime");

    // Start embedded HTTP server.
    let (server_url, bff) = rt.block_on(async {
        start_embedded_server().await
    });

    let bff = Arc::new(bff);
    let flux = Flux::new();
    bff.register(&flux);

    let handle = Box::new(FluxHandle {
        flux,
        _bff: bff,
        rt,
        server_url: CString::new(server_url).unwrap(),
    });

    Box::into_raw(handle)
}

/// Free a Flux handle.
#[no_mangle]
pub extern "C" fn flux_free(handle: *mut FluxHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)); }
    }
}

/// Get the server URL (e.g. "http://192.168.1.100:3000").
/// Returns a null-terminated C string. Do NOT free it.
#[no_mangle]
pub extern "C" fn flux_server_url(handle: *const FluxHandle) -> *const c_char {
    let handle = unsafe { &*handle };
    handle.server_url.as_ptr()
}

// ============================================================================
// State — read
// ============================================================================

#[no_mangle]
pub extern "C" fn flux_get(handle: *const FluxHandle, path: *const c_char) -> FluxBytes {
    let handle = unsafe { &*handle };
    let path = unsafe { CStr::from_ptr(path) }.to_str().unwrap_or("");

    match handle.flux.get(path) {
        Some(value) => match serialize_state(path, &value) {
            Some(json) => bytes_to_ffi(json),
            None => FluxBytes { ptr: std::ptr::null(), len: 0 },
        },
        None => FluxBytes { ptr: std::ptr::null(), len: 0 },
    }
}

#[no_mangle]
pub extern "C" fn flux_bytes_free(bytes: FluxBytes) {
    if !bytes.ptr.is_null() && bytes.len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(bytes.ptr as *mut u8, bytes.len, bytes.len);
        }
    }
}

// ============================================================================
// Requests — emit
// ============================================================================

#[no_mangle]
pub extern "C" fn flux_emit(
    handle: *mut FluxHandle,
    path: *const c_char,
    payload_json: *const c_char,
) {
    let handle = unsafe { &*handle };
    let path_str = unsafe { CStr::from_ptr(path) }.to_str().unwrap_or("");
    let payload_str = if payload_json.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(payload_json) }.to_str().unwrap_or("")
    };

    let payload = deserialize_request(path_str, payload_str);
    if let Some(payload) = payload {
        handle.rt.block_on(async {
            handle.flux.emit_arc(path_str, payload).await;
        });
    }
}

// ============================================================================
// Server startup
// ============================================================================

async fn start_embedded_server() -> (String, TwitterBff) {
    use std::sync::Arc;

    // Create in-memory storage.
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
        openerp_kv::RedbStore::open(&dir.path().join("flux.redb"))
            .expect("failed to open redb"),
    );
    std::mem::forget(dir);

    let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);

    // Seed demo data.
    seed_demo_data(&kv);

    // Build admin router (for dashboard).
    let twitter_admin = flux_golden::server::admin_router(kv.clone(), auth);

    // Build facet router (for app).
    let facet_state = Arc::new(flux_golden::server::facet_handlers::FacetStateInner {
        users: openerp_store::KvOps::new(kv.clone()),
        tweets: openerp_store::KvOps::new(kv.clone()),
        likes: openerp_store::KvOps::new(kv.clone()),
        follows: openerp_store::KvOps::new(kv.clone()),
        jwt: flux_golden::server::jwt::JwtService::golden_test(),
    });
    let facet_router = flux_golden::server::facet_handlers::facet_router(facet_state);

    // Build schema.
    let schema_json = openerp_store::build_schema("Twitter", vec![
        flux_golden::server::schema_def(),
    ]);

    // Detect LAN IP.
    let lan_ip = get_lan_ip().unwrap_or_else(|| "127.0.0.1".to_string());

    // Bind to a random port.
    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await
        .expect("failed to bind");
    let port = listener.local_addr().unwrap().port();
    let server_url = format!("http://{}:{}", lan_ip, port);

    tracing::info!("Embedded server: {}", server_url);
    tracing::info!("Dashboard: {}/dashboard", server_url);

    // Build the router.
    let schema = schema_json.clone();

    // Simple login handler (any password).
    let login_handler = axum::routing::post(|| async {
        let now = chrono::Utc::now().timestamp();
        let header = base64_url("{}");
        let payload = base64_url(&serde_json::json!({
            "sub": "app", "roles": ["admin"],
            "iat": now, "exp": now + 86400,
        }).to_string());
        let sig = base64_url("sig");
        let token = format!("{}.{}.{}", header, payload, sig);
        axum::Json(serde_json::json!({
            "access_token": token, "token_type": "Bearer", "expires_in": 86400,
        }))
    });

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async {
            axum::response::Html(openerp_web::login_html())
        }))
        .route("/dashboard", axum::routing::get(|| async {
            axum::response::Html(openerp_web::dashboard_html())
        }))
        .route("/meta/schema", axum::routing::get(move || {
            let s = schema.clone();
            async move { axum::Json(s) }
        }))
        .route("/health", axum::routing::get(|| async {
            axum::Json(serde_json::json!({"status": "ok"}))
        }))
        .route("/auth/login", login_handler)
        .nest("/app/twitter", facet_router)
        .nest("/admin/twitter", twitter_admin);

    // Spawn server in background.
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    // Wait for server to be ready.
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create BFF — login handler saves JWT, subsequent calls use it.
    let bff = TwitterBff::new(&server_url);

    (server_url, bff)
}

fn get_lan_ip() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip().to_string())
}

fn base64_url(input: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input.as_bytes())
}

fn seed_demo_data(kv: &Arc<dyn openerp_kv::KVStore>) {
    use openerp_store::KvOps;
    use openerp_types::*;
    use flux_golden::server::model::*;

    let users_ops = KvOps::<User>::new(kv.clone());
    let tweets_ops = KvOps::<Tweet>::new(kv.clone());

    for &(username, display, bio) in &[
        ("alice", "Alice Wang", "Rust developer & open source enthusiast"),
        ("bob", "Bob Li", "Product designer at Haivivi"),
        ("carol", "Carol Zhang", "Full-stack engineer"),
    ] {
        users_ops.save_new(User {
            id: Id::default(), username: username.into(),
            bio: Some(bio.into()),
            avatar: Some(Avatar::new(&format!("https://api.dicebear.com/7.x/initials/svg?seed={}", username))),
            follower_count: 0, following_count: 0, tweet_count: 0,
            display_name: Some(display.into()),
            description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(), rev: 0,
        }).unwrap();
    }

    for &(author, content) in &[
        ("alice", "Just shipped Flux — a cross-platform state engine in Rust!"),
        ("bob", "Dark mode design system is ready. Ship it!"),
        ("carol", "Hot take: Bazel > Cargo for monorepos."),
    ] {
        tweets_ops.save_new(Tweet {
            id: Id::default(), author_id: Id::new(author),
            content: content.into(),
            like_count: 0, reply_count: 0, reply_to_id: None,
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(), rev: 0,
        }).unwrap();
        if let Ok(Some(mut u)) = users_ops.get(author) {
            u.tweet_count += 1;
            let _ = users_ops.save(u);
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

// ============================================================================
// Serialization — type registry
// ============================================================================

fn serialize_state(path: &str, value: &StateValue) -> Option<Vec<u8>> {
    if path == AuthState::PATH {
        return value.downcast_ref::<AuthState>()
            .and_then(|v| serde_json::to_vec(v).ok());
    }
    if path == TimelineFeed::PATH {
        return value.downcast_ref::<TimelineFeed>()
            .and_then(|v| serde_json::to_vec(v).ok());
    }
    if path == ComposeState::PATH {
        return value.downcast_ref::<ComposeState>()
            .and_then(|v| serde_json::to_vec(v).ok());
    }
    if path == AppRoute::PATH {
        return value.downcast_ref::<AppRoute>()
            .and_then(|v| serde_json::to_vec(v).ok());
    }
    if path == SearchState::PATH {
        return value.downcast_ref::<SearchState>()
            .and_then(|v| serde_json::to_vec(v).ok());
    }
    if path == SettingsState::PATH {
        return value.downcast_ref::<SettingsState>()
            .and_then(|v| serde_json::to_vec(v).ok());
    }
    if path == PasswordState::PATH {
        return value.downcast_ref::<PasswordState>()
            .and_then(|v| serde_json::to_vec(v).ok());
    }
    if path.starts_with("profile/") {
        return value.downcast_ref::<ProfilePage>()
            .and_then(|v| serde_json::to_vec(v).ok());
    }
    if path.starts_with("tweet/") {
        return value.downcast_ref::<TweetDetail>()
            .and_then(|v| serde_json::to_vec(v).ok());
    }
    None
}

fn deserialize_request(path: &str, json: &str) -> Option<Arc<dyn Any + Send + Sync>> {
    match path {
        "app/initialize" => Some(Arc::new(InitializeReq)),
        "auth/login" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(LoginReq {
                    username: v["username"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "auth/logout" => Some(Arc::new(LogoutReq)),
        "timeline/load" => Some(Arc::new(TimelineLoadReq)),
        "tweet/create" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(CreateTweetReq {
                    content: v["content"].as_str().unwrap_or("").to_string(),
                    reply_to_id: v["replyToId"].as_str().map(|s| s.to_string()),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "tweet/like" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(LikeTweetReq {
                    tweet_id: v["tweetId"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "tweet/unlike" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(UnlikeTweetReq {
                    tweet_id: v["tweetId"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "tweet/load" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(LoadTweetReq {
                    tweet_id: v["tweetId"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "user/follow" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(FollowUserReq {
                    user_id: v["userId"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "user/unfollow" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(UnfollowUserReq {
                    user_id: v["userId"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "profile/load" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(LoadProfileReq {
                    user_id: v["userId"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "search/query" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(SearchReq {
                    query: v["query"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "search/clear" => Some(Arc::new(SearchClearReq)),
        "settings/load" => Some(Arc::new(SettingsLoadReq)),
        "settings/save" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(SettingsSaveReq {
                    display_name: v["displayName"].as_str().unwrap_or("").to_string(),
                    bio: v["bio"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "settings/change-password" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(ChangePasswordReq {
                    old_password: v["oldPassword"].as_str().unwrap_or("").to_string(),
                    new_password: v["newPassword"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        "compose/update-field" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                Arc::new(ComposeUpdateReq {
                    field: v["field"].as_str().unwrap_or("").to_string(),
                    value: v["value"].as_str().unwrap_or("").to_string(),
                }) as Arc<dyn Any + Send + Sync>
            })
        }
        _ => None,
    }
}

fn bytes_to_ffi(bytes: Vec<u8>) -> FluxBytes {
    let len = bytes.len();
    let ptr = bytes.as_ptr();
    std::mem::forget(bytes);
    FluxBytes { ptr, len }
}
