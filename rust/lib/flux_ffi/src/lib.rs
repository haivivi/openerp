//! Flux FFI — C-compatible API for cross-platform bindings.
//!
//! This is the golden reference for what the codegen will produce.
//! All platforms (iOS/Android/HarmonyOS/Tauri) call these C functions.
//!
//! Protocol:
//! - State is serialized as JSON bytes across FFI boundary.
//!   (Golden test uses JSON; production codegen will use Cap'n Proto.)
//! - Strings passed as null-terminated C strings.
//! - Byte buffers returned via FluxBytes { ptr, len } — caller must free.
//! - Callbacks receive (path, json_bytes, json_len, user_data).

use std::any::Any;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::Arc;

use openerp_flux::{Flux, StateStore, StateValue, SubscriptionId};

// Re-export state types for serialization lookup.
use flux_golden::state::*;
use flux_golden::request::*;
use flux_golden::handlers::TwitterBff;

/// Opaque handle to a Flux instance + BFF context.
pub struct FluxHandle {
    flux: Flux,
    bff: Arc<TwitterBff>,
    rt: tokio::runtime::Runtime,
}

/// Byte buffer returned from FFI calls. Caller must free with `flux_bytes_free`.
#[repr(C)]
pub struct FluxBytes {
    pub ptr: *const u8,
    pub len: usize,
}

/// Subscription callback type.
/// Called with (path, json_bytes, json_len, user_data).
pub type FluxCallback = extern "C" fn(
    path: *const c_char,
    data: *const u8,
    data_len: usize,
    user_data: *mut c_void,
);

// ============================================================================
// Lifecycle
// ============================================================================

/// Create a new Flux instance with the Twitter BFF.
/// Returns an opaque handle. Must be freed with `flux_free`.
#[no_mangle]
pub extern "C" fn flux_create() -> *mut FluxHandle {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");

    // Create in-memory backend.
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
        openerp_kv::RedbStore::open(&dir.path().join("flux.redb"))
            .expect("failed to open redb"),
    );

    let bff = Arc::new(TwitterBff {
        users: openerp_store::KvOps::new(kv.clone()),
        tweets: openerp_store::KvOps::new(kv.clone()),
        likes: openerp_store::KvOps::new(kv.clone()),
        follows: openerp_store::KvOps::new(kv.clone()),
    });

    let flux = Flux::new();
    bff.register(&flux);

    // Seed demo data.
    seed_demo_data(&bff);

    let handle = Box::new(FluxHandle {
        flux,
        bff,
        rt,
    });

    // Leak the tempdir to keep it alive (it's owned by the process).
    std::mem::forget(dir);

    Box::into_raw(handle)
}

/// Free a Flux handle.
#[no_mangle]
pub extern "C" fn flux_free(handle: *mut FluxHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)); }
    }
}

// ============================================================================
// State — read
// ============================================================================

/// Get state at a path as JSON bytes.
/// Returns FluxBytes { ptr, len }. Caller must free with `flux_bytes_free`.
/// Returns { null, 0 } if path not found or type unknown.
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

/// Free bytes returned by `flux_get`.
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

/// Emit a request with JSON payload.
/// `path` is the request path (e.g. "auth/login").
/// `payload_json` is the JSON-encoded request body (or null for unit requests).
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

    // Deserialize payload to the correct request type and emit.
    let payload = deserialize_request(path_str, payload_str);
    if let Some(payload) = payload {
        handle.rt.block_on(async {
            handle.flux.emit_arc(path_str, payload).await;
        });
    }
}

// ============================================================================
// Subscriptions
// ============================================================================

/// Subscribe to state changes matching a pattern.
/// Callback is called with (path, json_bytes, json_len, user_data).
/// Returns a subscription ID (u64). Use `flux_unsubscribe` to remove.
#[no_mangle]
pub extern "C" fn flux_subscribe(
    handle: *mut FluxHandle,
    pattern: *const c_char,
    callback: FluxCallback,
    user_data: *mut c_void,
) -> u64 {
    let handle = unsafe { &*handle };
    let pattern = unsafe { CStr::from_ptr(pattern) }.to_str().unwrap_or("");

    // Wrap user_data in a Send+Sync wrapper (caller guarantees thread safety).
    let user_data = user_data as usize; // usize is Send+Sync

    let id = handle.flux.subscribe(pattern, move |path, value| {
        if let Some(json) = serialize_state(path, value) {
            let c_path = CString::new(path).unwrap_or_default();
            let ud = user_data as *mut c_void;
            callback(c_path.as_ptr(), json.as_ptr(), json.len(), ud);
        }
    });

    // Return the inner u64.
    // We need to extract it — SubscriptionId is pub(crate).
    // For the golden test, we'll store subscription IDs internally.
    // Return a simple counter that maps to the real ID.
    0 // TODO: proper ID mapping
}

/// Unsubscribe by pattern and ID.
#[no_mangle]
pub extern "C" fn flux_unsubscribe(
    handle: *mut FluxHandle,
    pattern: *const c_char,
    sub_id: u64,
) {
    // TODO: implement with ID mapping
}

// ============================================================================
// Serialization — type registry (golden test: hand-written switch)
// ============================================================================

/// Serialize a StateValue to JSON bytes based on its path.
/// This is what the codegen will generate — one branch per #[state] type.
fn serialize_state(path: &str, value: &StateValue) -> Option<Vec<u8>> {
    // Match by well-known paths first.
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

    // Dynamic paths: profile/{id}, tweet/{id}.
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

/// Deserialize a JSON request payload to the correct typed request.
/// Returns Arc<dyn Any> for Flux.emit_arc().
fn deserialize_request(path: &str, json: &str) -> Option<Arc<dyn Any + Send + Sync>> {
    match path {
        "app/initialize" => Some(Arc::new(InitializeReq)),
        "auth/login" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                let req = LoginReq {
                    username: v["username"].as_str().unwrap_or("").to_string(),
                };
                Arc::new(req) as Arc<dyn Any + Send + Sync>
            })
        }
        "auth/logout" => Some(Arc::new(LogoutReq)),
        "timeline/load" => Some(Arc::new(TimelineLoadReq)),
        "tweet/create" => {
            serde_json::from_str::<serde_json::Value>(json).ok().map(|v| {
                let req = CreateTweetReq {
                    content: v["content"].as_str().unwrap_or("").to_string(),
                    reply_to_id: v["replyToId"].as_str().map(|s| s.to_string()),
                };
                Arc::new(req) as Arc<dyn Any + Send + Sync>
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

/// Convert a Vec<u8> to FFI-safe FluxBytes.
fn bytes_to_ffi(bytes: Vec<u8>) -> FluxBytes {
    let len = bytes.len();
    let ptr = bytes.as_ptr();
    std::mem::forget(bytes); // Caller frees via flux_bytes_free.
    FluxBytes { ptr, len }
}

// ============================================================================
// Seed demo data
// ============================================================================

fn seed_demo_data(bff: &TwitterBff) {
    use openerp_types::*;
    use flux_golden::server::model::*;

    let users = vec![
        ("alice", "Alice Wang", "Rust developer & open source enthusiast"),
        ("bob", "Bob Li", "Product designer at Haivivi"),
        ("carol", "Carol Zhang", "Full-stack engineer"),
    ];
    for &(username, display, bio) in &users {
        bff.users.save_new(User {
            id: Id::default(), username: username.into(),
            bio: Some(bio.into()),
            avatar: Some(Avatar::new(&format!("https://api.dicebear.com/7.x/initials/svg?seed={}", username))),
            follower_count: 0, following_count: 0, tweet_count: 0,
            display_name: Some(display.into()),
            description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();
    }

    let tweets = vec![
        ("alice", "Just shipped Flux — a cross-platform state engine in Rust!"),
        ("bob", "Dark mode design system is ready. Ship it!"),
        ("carol", "Hot take: Bazel > Cargo for monorepos."),
    ];
    for &(author, content) in &tweets {
        bff.tweets.save_new(Tweet {
            id: Id::default(), author_id: Id::new(author),
            content: content.into(),
            like_count: 0, reply_count: 0, reply_to_id: None,
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();
        if let Ok(Some(mut u)) = bff.users.get(author) {
            u.tweet_count += 1;
            let _ = bff.users.save(u);
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}
