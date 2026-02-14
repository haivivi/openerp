use oe_macro::model;
use oe_types::*;

#[model(module = "auth")]
pub struct Session {
    pub id: Id,
    pub user_id: Id,
    pub issued_at: DateTime,
    pub expires_at: DateTime,
    pub revoked: bool,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
