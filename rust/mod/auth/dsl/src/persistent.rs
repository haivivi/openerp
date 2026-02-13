//! Auth persistent (DB) definitions — what the storage layer stores.
//!
//! Hidden fields (password_hash, client_secret, etc.) are in DB but not in model.

use openerp_dsl_macro::persistent;

// ── UserDB ──

#[persistent(User, store = "kv")]
#[key(id)]
#[unique(email)]
#[index(name)]
pub struct UserDB {
    #[auto(uuid)]
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub active: bool,
    /// Password hash (argon2id). Hidden from API.
    pub password_hash: Option<String>,
    /// Linked OAuth accounts: {"github": "12345", "feishu": "ou_xxx"}
    pub linked_accounts: Option<String>, // JSON-encoded HashMap
    #[auto(create_timestamp)]
    pub created_at: String,
    #[auto(update_timestamp)]
    pub updated_at: String,
}

// ── RoleDB ──

#[persistent(Role, store = "kv")]
#[key(id)]
#[index(service)]
pub struct RoleDB {
    pub id: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub service: Option<String>,
    #[auto(create_timestamp)]
    pub created_at: String,
    #[auto(update_timestamp)]
    pub updated_at: String,
}

// ── GroupDB ──

#[persistent(Group, store = "kv")]
#[key(id)]
#[index(parent_id)]
pub struct GroupDB {
    #[auto(uuid)]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub external_source: Option<String>,
    pub external_id: Option<String>,
    #[auto(create_timestamp)]
    pub created_at: String,
    #[auto(update_timestamp)]
    pub updated_at: String,
}

// ── PolicyDB ──

#[persistent(Policy, store = "kv")]
#[key(id)]
#[index(who)]
#[filter(how)]
pub struct PolicyDB {
    pub id: String,
    pub who: String,
    pub what: String,
    pub how: String,
    pub expires_at: Option<String>,
    #[auto(create_timestamp)]
    pub created_at: String,
    #[auto(update_timestamp)]
    pub updated_at: String,
}

// ── SessionDB ──

#[persistent(Session, store = "kv")]
#[key(id)]
#[index(user_id)]
pub struct SessionDB {
    #[auto(uuid)]
    pub id: String,
    pub user_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub revoked: bool,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

// ── ProviderDB ──

#[persistent(Provider, store = "kv")]
#[key(id)]
pub struct ProviderDB {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub client_id: String,
    /// Client secret — stored in DB, hidden from API.
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub userinfo_url: Option<String>,
    pub scopes: Vec<String>,
    pub redirect_url: String,
    pub enabled: bool,
    #[auto(create_timestamp)]
    pub created_at: String,
    #[auto(update_timestamp)]
    pub updated_at: String,
}
