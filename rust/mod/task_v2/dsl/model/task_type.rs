use oe_macro::model;
use oe_types::*;

/// Task type definition — registered by services at startup.
#[model(module = "task")]
pub struct TaskType {
    pub id: Id,
    pub service: String,
    pub default_timeout: i64,
    pub max_concurrency: i64,
    // display_name, description, metadata, created_at, updated_at → auto
}
