use oe_macro::model;
use oe_types::*;

#[model(module = "auth")]
pub struct Group {
    pub id: Id,
    pub parent_id: Option<Id>,
    pub external_source: Option<String>,
    pub external_id: Option<String>,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
