use oe_macro::model;
use oe_types::*;

#[model(module = "auth")]
pub struct User {
    pub id: Id,
    pub email: Option<Email>,
    pub avatar: Option<Avatar>,
    pub active: bool,
    pub password_hash: Option<PasswordHash>,
    pub linked_accounts: Option<String>,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
