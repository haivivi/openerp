//! User resource — identity with linked OAuth accounts.
//!
//! db_resource: automatic CRUD (create, get, list, update, delete).
//! Custom APIs: login, logout, me.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "auth", table = "users", display_name = "User")]
// #[permission(create = "auth:user:create")]
// #[permission(read = "auth:user:read")]
// #[permission(update = "auth:user:update")]
// #[permission(delete = "auth:user:delete")]
// #[permission(list = "auth:user:list")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    // #[primary_key]
    pub id: String,

    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,

    #[serde(default = "default_true")]
    pub active: bool,

    /// Linked accounts: provider_id -> external user id.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub linked_accounts: HashMap<String, String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    // #[auto_timestamp(on_create)]
    pub created_at: String,

    // #[auto_timestamp(on_update)]
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUser {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub linked_accounts: HashMap<String, String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

// ---------------------------------------------------------------------------
// Custom endpoints
// ---------------------------------------------------------------------------
//
// #[api(User)]
// #[handlers_path = "crate::handlers::user"]
// impl UserApi {
//     #[endpoint(POST "/auth/login")]
//     #[public]
//     #[handler = "login"]
//     async fn login(body: LoginRequest) -> LoginResponse;
//
//     #[endpoint(POST "/auth/logout")]
//     #[permission("auth:session:revoke")]
//     #[handler = "logout"]
//     async fn logout() -> ();
//
//     #[endpoint(GET "/auth/me")]
//     #[permission("auth:user:read")]
//     #[handler = "me"]
//     async fn me() -> User;
// }

fn default_true() -> bool {
    true
}
