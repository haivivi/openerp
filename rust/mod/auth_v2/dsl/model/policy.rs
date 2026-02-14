use oe_macro::model;
use oe_types::*;

#[model(module = "auth")]
pub struct Policy {
    pub id: Id,
    pub who: String,
    pub what: String,
    pub how: String,
    pub expires_at: Option<DateTime>,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
