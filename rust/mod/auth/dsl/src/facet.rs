//! Auth REST facets — API surfaces for different consumers.
//!
//! `data/` facet exposes all resources with full access, controlled by IAM.

use openerp_dsl_macro::facet;
use openerp_types::{Avatar, DateTime, Email, Id, Url};

// The facet macro generates router functions that reference *Store types.
use crate::persistent::{
    UserStore, RoleStore, GroupStore, PolicyStore, SessionStore, ProviderStore,
};

// ── Data facet: full access ──

#[facet(path = "/data", auth = "jwt", model = "User")]
pub struct DataUser {
    #[readonly]
    pub id: Id,
    pub name: String,
    pub email: Option<Email>,
    pub avatar: Option<Avatar>,
    pub active: bool,
    #[readonly]
    pub created_at: Option<DateTime>,
    #[readonly]
    pub updated_at: Option<DateTime>,
}

#[facet(path = "/data", auth = "jwt", model = "Role")]
pub struct DataRole {
    pub id: Id,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub service: Option<String>,
    #[readonly]
    pub created_at: Option<DateTime>,
    #[readonly]
    pub updated_at: Option<DateTime>,
}

#[facet(path = "/data", auth = "jwt", model = "Group")]
pub struct DataGroup {
    #[readonly]
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Id>,
    pub external_source: Option<String>,
    pub external_id: Option<String>,
    #[readonly]
    pub created_at: Option<DateTime>,
    #[readonly]
    pub updated_at: Option<DateTime>,
}

#[facet(path = "/data", auth = "jwt", model = "Policy")]
pub struct DataPolicy {
    #[readonly]
    pub id: Id,
    pub who: String,
    pub what: String,
    pub how: String,
    pub expires_at: Option<DateTime>,
    #[readonly]
    pub created_at: Option<DateTime>,
    #[readonly]
    pub updated_at: Option<DateTime>,
}

#[facet(path = "/data", auth = "jwt", model = "Session")]
pub struct DataSession {
    #[readonly]
    pub id: Id,
    pub user_id: Id,
    #[readonly]
    pub issued_at: DateTime,
    #[readonly]
    pub expires_at: DateTime,
    pub revoked: bool,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

#[facet(path = "/data", auth = "jwt", model = "Provider")]
pub struct DataProvider {
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
    #[readonly]
    pub created_at: Option<DateTime>,
    #[readonly]
    pub updated_at: Option<DateTime>,
}
