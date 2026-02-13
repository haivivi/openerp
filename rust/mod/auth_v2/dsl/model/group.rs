use openerp_macro::model;
use openerp_types::*;

#[model(module = "auth")]
pub struct Group {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Id>,
    pub external_source: Option<String>,
    pub external_id: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
