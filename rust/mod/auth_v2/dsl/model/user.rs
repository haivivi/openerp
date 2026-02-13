use openerp_macro::model;
use openerp_types::*;

#[model(module = "auth")]
pub struct User {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub email: Option<Email>,
    pub avatar: Option<Avatar>,
    pub active: bool,
    pub password_hash: Option<PasswordHash>,
    pub linked_accounts: Option<String>,
    pub metadata: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
