use openerp_macro::model;
use openerp_types::*;

/// A license entry (MIIT, WiFi, etc.).
#[model(module = "pms")]
pub struct License {
    pub id: Id,
    pub license_type: String,
    pub number: String,
    pub source: String,
    pub sn: Option<String>,
    pub import_id: Option<Id>,
    pub status: String,
    // display_name, description, metadata, created_at, updated_at → auto
}
