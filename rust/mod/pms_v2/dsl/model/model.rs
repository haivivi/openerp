use oe_macro::model;
use oe_types::*;

/// Product model/series definition. Primary key is numeric code.
#[model(module = "pms")]
pub struct Model {
    pub code: u32,
    pub series_name: String,
    // display_name, description, metadata, created_at, updated_at → auto
}
