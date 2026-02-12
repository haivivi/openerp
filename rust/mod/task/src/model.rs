use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// TaskStatus
// ---------------------------------------------------------------------------

/// Lifecycle state of a task.
///
/// ```text
/// PENDING → RUNNING → COMPLETED
///                   → FAILED
///          → CANCELLED
/// ```
///
/// Stale detection: RUNNING tasks with no heartbeat are reset to PENDING
/// by the watchdog (up to `max_retries` times, then marked FAILED).
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "PENDING" => Some(Self::Pending),
            "RUNNING" => Some(Self::Running),
            "COMPLETED" => Some(Self::Completed),
            "FAILED" => Some(Self::Failed),
            "CANCELLED" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Whether the task has reached a terminal state.
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
// Task — the core data model, maps 1:1 to SQL columns
// ---------------------------------------------------------------------------

/// A single asynchronous task tracked by the task module.
///
/// All fields map directly to SQL columns — no JSON blob.
/// Executor runtime data (input params, checkpoint state) lives in KV.
/// Logs live in TSDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,

    // --- definition ---
    #[serde(rename = "type")]
    pub task_type: String,

    // --- progress counters ---
    /// Total number of work items (set by executor).
    #[serde(default)]
    pub total: i64,
    /// Successfully processed items.
    #[serde(default)]
    pub success: i64,
    /// Failed items.
    #[serde(default)]
    pub failed: i64,

    // --- execution state ---
    pub status: TaskStatus,
    /// Human-readable progress message from the executor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Error description (set on FAILED).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    // --- ownership ---
    /// Who claimed this task (executor identity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
    /// Last heartbeat / activity timestamp (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<String>,
    /// Who created this task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    // --- timestamps ---
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,

    // --- config ---
    /// Timeout in seconds (0 = no limit).
    #[serde(default)]
    pub timeout_secs: i64,
    /// How many times this task has been re-queued after stale detection.
    #[serde(default)]
    pub retry_count: i64,
    /// Maximum number of retries before marking FAILED.
    #[serde(default = "default_max_retries")]
    pub max_retries: i64,
}

fn default_max_retries() -> i64 {
    3
}

// ---------------------------------------------------------------------------
// TaskType — runtime-registered task type definition
// ---------------------------------------------------------------------------

/// Runtime-registered task type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskType {
    /// Unique type key, e.g. `"pms.batch.provision"`.
    #[serde(rename = "type")]
    pub task_type: String,

    /// Owning service name, e.g. `"pms"`.
    pub service: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: String,

    /// Default timeout in seconds (0 = no limit).
    #[serde(default)]
    pub default_timeout: i64,

    /// Max concurrent running tasks of this type (0 = unlimited).
    #[serde(default)]
    pub max_concurrency: i64,
}

// ---------------------------------------------------------------------------
// API request / response types — Caller-facing
// ---------------------------------------------------------------------------

/// Body for `POST /tasks` — create a new task.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    #[serde(rename = "type")]
    pub task_type: String,

    /// Input parameters for the executor (stored in KV, opaque to task module).
    #[serde(default)]
    pub input: serde_json::Value,

    #[serde(default)]
    pub timeout_secs: Option<i64>,

    #[serde(default)]
    pub max_retries: Option<i64>,

    #[serde(default)]
    pub created_by: Option<String>,
}

/// Query parameters for `GET /tasks`.
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

/// Query parameters for `GET /tasks/{id}/@poll`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollQuery {
    /// Max seconds to block (default 30, max 120).
    #[serde(default = "default_poll_timeout")]
    pub timeout: u64,
}

fn default_poll_timeout() -> u64 {
    30
}

impl Default for PollQuery {
    fn default() -> Self {
        Self {
            timeout: default_poll_timeout(),
        }
    }
}

// ---------------------------------------------------------------------------
// API request types — Executor-facing
// ---------------------------------------------------------------------------

/// Body for `POST /tasks/{id}/@claim`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimRequest {
    /// Identity of the executor claiming this task.
    pub claimed_by: String,
}

/// Body for `POST /tasks/{id}/@progress`.
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

/// Body for `POST /tasks/{id}/@complete`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequest {
    /// Optional final message.
    #[serde(default)]
    pub message: Option<String>,
}

/// Body for `POST /tasks/{id}/@fail`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailRequest {
    /// Error description.
    pub error: String,
    /// Optional message.
    #[serde(default)]
    pub message: Option<String>,
}

/// Body for `POST /tasks/{id}/@log`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRequest {
    /// Log level (e.g. "info", "error", "warn", "debug").
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log lines.
    pub lines: Vec<String>,
}

fn default_log_level() -> String {
    "info".to_string()
}

/// Query parameters for `GET /tasks/{id}/@logs`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogQuery {
    /// Max entries to return (default 100).
    #[serde(default = "default_log_limit")]
    pub limit: Option<usize>,
    /// If true, return newest first (default true).
    #[serde(default = "default_true")]
    pub desc: bool,
    /// Filter by log level.
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

/// Body for `POST /task-types`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterTaskTypeRequest {
    #[serde(rename = "type")]
    pub task_type: String,

    pub service: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub default_timeout: i64,

    #[serde(default)]
    pub max_concurrency: i64,
}

// ---------------------------------------------------------------------------
// Log entry for API responses
// ---------------------------------------------------------------------------

/// A single log entry returned by the logs endpoint.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskLogEntry {
    /// Nanosecond Unix timestamp.
    pub ts: u64,
    /// Log level.
    pub level: String,
    /// Log content.
    pub data: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_roundtrip() {
        for s in &[
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            let json = serde_json::to_string(s).unwrap();
            let back: TaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
            assert_eq!(TaskStatus::from_str(s.as_str()), Some(*s));
        }
    }

    #[test]
    fn status_terminal() {
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
    }

    #[test]
    fn task_json_roundtrip() {
        let task = Task {
            id: "abc123".into(),
            task_type: "pms.batch.provision".into(),
            total: 100,
            success: 50,
            failed: 2,
            status: TaskStatus::Running,
            message: Some("processing".into()),
            error: None,
            claimed_by: Some("executor-1".into()),
            last_active_at: Some("2026-01-01T00:01:00Z".into()),
            created_by: Some("user1".into()),
            created_at: "2026-01-01T00:00:00Z".into(),
            started_at: Some("2026-01-01T00:00:01Z".into()),
            ended_at: None,
            timeout_secs: 3600,
            retry_count: 0,
            max_retries: 3,
        };
        let json = serde_json::to_string(&task).unwrap();
        let back: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "abc123");
        assert_eq!(back.status, TaskStatus::Running);
        assert_eq!(back.total, 100);
        assert_eq!(back.success, 50);
        assert_eq!(back.failed, 2);
        assert_eq!(back.claimed_by.as_deref(), Some("executor-1"));
        // Optional None fields should not appear in JSON
        assert!(!json.contains("\"error\""));
        assert!(!json.contains("\"endedAt\""));
    }

    #[test]
    fn create_request_deserialize() {
        let json = r#"{"type":"pms.export","input":{"format":"csv"}}"#;
        let req: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task_type, "pms.export");
        assert_eq!(req.input["format"], "csv");
        assert!(req.timeout_secs.is_none());
    }

    #[test]
    fn claim_request_deserialize() {
        let json = r#"{"claimedBy":"worker-42"}"#;
        let req: ClaimRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.claimed_by, "worker-42");
    }

    #[test]
    fn progress_report_partial() {
        let json = r#"{"success":10}"#;
        let req: ProgressReport = serde_json::from_str(json).unwrap();
        assert_eq!(req.success, Some(10));
        assert!(req.total.is_none());
        assert!(req.failed.is_none());
        assert!(req.message.is_none());
    }
}
