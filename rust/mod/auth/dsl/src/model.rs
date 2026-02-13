//! Auth model definitions — what the API layer sees.

use openerp_dsl_macro::model;
use openerp_types::{Avatar, DateTime, Email, Id, Url};

// ── User ──

#[model(module = "auth")]
#[key(id)]
pub struct User {
    pub id: Id,
    pub name: String,
    pub email: Option<Email>,
    pub avatar: Option<Avatar>,
    pub active: bool,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

// ── Role ──

#[model(module = "auth")]
#[key(id)]
pub struct Role {
    pub id: Id,
    pub description: Option<String>,
    #[ui(widget = "permission_picker")]
    pub permissions: Vec<String>,
    pub service: Option<String>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

// ── Group ──

#[model(module = "auth")]
#[key(id)]
pub struct Group {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Id>,
    pub external_source: Option<String>,
    pub external_id: Option<String>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

// ── Policy ──

#[model(module = "auth")]
#[key(id)]
pub struct Policy {
    pub id: Id,
    pub who: String,
    pub what: String,
    pub how: String,
    pub expires_at: Option<DateTime>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

// ── Session ──

#[model(module = "auth")]
#[key(id)]
pub struct Session {
    pub id: Id,
    pub user_id: Id,
    pub issued_at: DateTime,
    pub expires_at: DateTime,
    pub revoked: bool,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

// ── Provider ──

#[model(module = "auth")]
#[key(id)]
pub struct Provider {
    pub id: Id,
    pub name: String,
    pub provider_type: String,
    pub client_id: String,
    pub auth_url: Url,
    pub token_url: Url,
    pub userinfo_url: Option<Url>,
    pub scopes: Vec<String>,
    pub redirect_url: Url,
    pub enabled: bool,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}
