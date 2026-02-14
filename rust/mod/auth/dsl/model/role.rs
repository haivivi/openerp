use openerp_macro::model;
use openerp_types::*;

#[model(module = "auth")]
pub struct Role {
    pub id: Id,
    #[ui(widget = "permission_picker")]
    pub permissions: Vec<String>,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
