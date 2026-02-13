use openerp_macro::model;
use openerp_types::*;

#[model(module = "auth")]
pub struct Session {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub user_id: Id,
    pub issued_at: DateTime,
    pub expires_at: DateTime,
    pub revoked: bool,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}
