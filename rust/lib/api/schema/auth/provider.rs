//! Provider resource — OAuth provider configuration.
//!
//! Pure db_resource. The OAuth flow endpoints (authorize, callback)
//! are handled by the session handler, not the provider resource.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "auth", table = "providers", display_name = "Provider")]
// #[permission(create = "auth:provider:create")]
// #[permission(read = "auth:provider:read")]
// #[permission(update = "auth:provider:update")]
// #[permission(delete = "auth:provider:delete")]
// #[permission(list = "auth:provider:list")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// Unique identifier (e.g. "github", "feishu", "google").
    // #[primary_key]
    pub id: String,

    pub name: String,

    /// Provider type: "oauth2", "oidc", "custom".
    pub provider_type: String,

    pub client_id: String,

    /// OAuth client secret (stored, never returned in API responses).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,

    pub auth_url: String,
    pub token_url: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userinfo_url: Option<String>,

    #[serde(default)]
    pub scopes: Vec<String>,

    pub redirect_url: String,

    #[serde(default = "default_true")]
    pub enabled: bool,

    // #[auto_timestamp(on_create)]
    pub created_at: String,

    // #[auto_timestamp(on_update)]
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProvider {
    pub id: String,
    pub name: String,
    #[serde(default = "default_oauth2")]
    pub provider_type: String,
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    #[serde(default)]
    pub userinfo_url: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub redirect_url: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Public view (client_secret redacted)
// ---------------------------------------------------------------------------

// #[model]
#[derive(Debug, Clone, Serialize)]
pub struct ProviderPublic {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub client_id: String,
    pub auth_url: String,
    pub token_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub userinfo_url: Option<String>,
    pub scopes: Vec<String>,
    pub redirect_url: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Provider> for ProviderPublic {
    fn from(p: Provider) -> Self {
        Self {
            id: p.id,
            name: p.name,
            provider_type: p.provider_type,
            client_id: p.client_id,
            auth_url: p.auth_url,
            token_url: p.token_url,
            userinfo_url: p.userinfo_url,
            scopes: p.scopes,
            redirect_url: p.redirect_url,
            enabled: p.enabled,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_oauth2() -> String {
    "oauth2".to_string()
}
