//! Task resource — asynchronous task tracked by the task module.
//!
//! db_resource + many custom APIs for executor interaction.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

// #[model]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Running => "RUNNING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "task", table = "tasks", display_name = "Task")]
// #[permission(create = "task:task:create")]
// #[permission(read = "task:task:read")]
// #[permission(list = "task:task:list")]
// Note: no update/delete — tasks are immutable once created, controlled via actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    // #[primary_key]
    pub id: String,

    #[serde(rename = "type")]
    pub task_type: String,

    // --- progress counters ---
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub success: i64,
    #[serde(default)]
    pub failed: i64,

    // --- execution state ---
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    // --- ownership ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    // --- timestamps ---
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,

    // --- config ---
    #[serde(default)]
    pub timeout_secs: i64,
    #[serde(default)]
    pub retry_count: i64,
    #[serde(default = "default_max_retries")]
    pub max_retries: i64,
}

fn default_max_retries() -> i64 {
    3
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    #[serde(rename = "type")]
    pub task_type: String,
    #[serde(default)]
    pub input: serde_json::Value,
    #[serde(default)]
    pub timeout_secs: Option<i64>,
    #[serde(default)]
    pub max_retries: Option<i64>,
    #[serde(default)]
    pub created_by: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListQuery {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(rename = "type", default)]
    pub task_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollQuery {
    #[serde(default = "default_poll_timeout")]
    pub timeout: u64,
}

fn default_poll_timeout() -> u64 {
    30
}

impl Default for PollQuery {
    fn default() -> Self {
        Self { timeout: default_poll_timeout() }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimRequest {
    pub claimed_by: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressReport {
    #[serde(default)]
    pub total: Option<i64>,
    #[serde(default)]
    pub success: Option<i64>,
    #[serde(default)]
    pub failed: Option<i64>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequest {
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailRequest {
    pub error: String,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRequest {
    #[serde(default = "default_log_level")]
    pub level: String,
    pub lines: Vec<String>,
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogQuery {
    #[serde(default = "default_log_limit")]
    pub limit: Option<usize>,
    #[serde(default = "default_true")]
    pub desc: bool,
    #[serde(default)]
    pub level: Option<String>,
}

fn default_log_limit() -> Option<usize> {
    Some(100)
}

fn default_true() -> bool {
    true
}

impl Default for LogQuery {
    fn default() -> Self {
        Self {
            limit: default_log_limit(),
            desc: true,
            level: None,
        }
    }
}

// #[model]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskLogEntry {
    pub ts: u64,
    pub level: String,
    pub data: String,
}

// ---------------------------------------------------------------------------
// Custom endpoints
// ---------------------------------------------------------------------------
//
// #[api(Task)]
// #[handlers_path = "crate::handlers::task"]
// impl TaskApi {
//     #[endpoint(POST "/task/tasks/:id/@claim")]
//     #[permission("task:task:claim")]
//     #[handler = "claim"]
//     async fn claim(id: String, body: ClaimRequest) -> Task;
//
//     #[endpoint(POST "/task/tasks/:id/@progress")]
//     #[permission("task:task:progress")]
//     #[handler = "progress"]
//     async fn progress(id: String, body: ProgressReport) -> Task;
//
//     #[endpoint(POST "/task/tasks/:id/@complete")]
//     #[permission("task:task:complete")]
//     #[handler = "complete"]
//     async fn complete(id: String, body: CompleteRequest) -> Task;
//
//     #[endpoint(POST "/task/tasks/:id/@fail")]
//     #[permission("task:task:fail")]
//     #[handler = "fail"]
//     async fn fail(id: String, body: FailRequest) -> Task;
//
//     #[endpoint(POST "/task/tasks/:id/@cancel")]
//     #[permission("task:task:cancel")]
//     #[handler = "cancel"]
//     async fn cancel(id: String) -> Task;
//
//     #[endpoint(GET "/task/tasks/:id/@poll")]
//     #[permission("task:task:poll")]
//     #[handler = "poll"]
//     async fn poll(id: String, query: PollQuery) -> Task;
//
//     #[endpoint(POST "/task/tasks/:id/@log")]
//     #[permission("task:task:log")]
//     #[handler = "log_write"]
//     async fn log_write(id: String, body: LogRequest) -> ();
//
//     #[endpoint(GET "/task/tasks/:id/@logs")]
//     #[permission("task:task:read")]
//     #[handler = "log_read"]
//     async fn log_read(id: String, query: LogQuery) -> Vec<TaskLogEntry>;
// }
