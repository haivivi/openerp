use openerp_macro::model;
use openerp_types::*;

#[model(module = "auth")]
pub struct Role {
    pub id: Id,
    pub description: Option<String>,
    #[ui(widget = "permission_picker")]
    pub permissions: Vec<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
