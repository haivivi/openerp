//! Auth model definitions — what the API layer sees.

use openerp_dsl_macro::model;

// ── User ──

#[model(module = "auth")]
#[key(id)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ── Role ──

#[model(module = "auth")]
#[key(id)]
pub struct Role {
    pub id: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub service: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ── Group ──

#[model(module = "auth")]
#[key(id)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub external_source: Option<String>,
    pub external_id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ── Policy ──

#[model(module = "auth")]
#[key(id)]
pub struct Policy {
    pub id: String,
    pub who: String,
    pub what: String,
    pub how: String,
    pub expires_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ── Session ──

#[model(module = "auth")]
#[key(id)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub revoked: bool,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

// ── Provider ──

#[model(module = "auth")]
#[key(id)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub client_id: String,
    pub auth_url: String,
    pub token_url: String,
    pub userinfo_url: Option<String>,
    pub scopes: Vec<String>,
    pub redirect_url: String,
    pub enabled: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
