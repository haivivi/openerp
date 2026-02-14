use oe_macro::model;
use oe_types::*;

/// An async task instance.
#[model(module = "task")]
pub struct Task {
    pub id: Id,
    pub task_type: String,
    pub total: i64,
    pub success: i64,
    pub failed: i64,
    pub status: String,
    pub message: Option<String>,
    pub error: Option<String>,
    pub claimed_by: Option<String>,
    pub last_active_at: Option<DateTime>,
    pub created_by: Option<String>,
    pub started_at: Option<DateTime>,
    pub ended_at: Option<DateTime>,
    pub timeout_secs: i64,
    pub retry_count: i64,
    pub max_retries: i64,
    // display_name, description, metadata, created_at, updated_at → auto
}
