use serde::{Deserialize, Serialize};

/// An OAuth login provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// Unique identifier (e.g. "github", "feishu", "google").
    pub id: String,

    /// Display name.
    pub name: String,

    /// Provider type: "oauth2", "oidc", "custom".
    pub provider_type: String,

    /// OAuth client id.
    pub client_id: String,

    /// OAuth client secret (stored, never returned in API responses).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,

    /// Authorization URL.
    pub auth_url: String,

    /// Token exchange URL.
    pub token_url: String,

    /// User info URL (to fetch profile after token exchange).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userinfo_url: Option<String>,

    /// OAuth scopes.
    #[serde(default)]
    pub scopes: Vec<String>,

    /// Redirect URL after OAuth callback.
    pub redirect_url: String,

    /// Whether this provider is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// RFC 3339 creation timestamp.
    pub created_at: String,

    /// RFC 3339 last update timestamp.
    pub updated_at: String,
}

/// Input for creating a new provider.
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

/// Provider data returned in API responses (client_secret redacted).
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
