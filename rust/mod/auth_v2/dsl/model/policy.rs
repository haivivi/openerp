use openerp_macro::model;
use openerp_types::*;

#[model(module = "auth")]
pub struct Policy {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub who: String,
    pub what: String,
    pub how: String,
    pub expires_at: Option<DateTime>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
