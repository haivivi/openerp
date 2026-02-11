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
// Task
// ---------------------------------------------------------------------------

/// A single asynchronous task tracked by the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,

    // --- definition ---
    #[serde(rename = "type")]
    pub task_type: String,
    #[serde(default)]
    pub params: serde_json::Value,

    // --- execution state ---
    pub status: TaskStatus,
    #[serde(default)]
    pub progress: u64,
    #[serde(default)]
    pub total: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    // --- result ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    // --- timestamps ---
    pub create_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_at: Option<String>,

    // --- config ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
}

// ---------------------------------------------------------------------------
// TaskType
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
    pub default_timeout: u64,

    /// Max concurrent running tasks of this type (0 = unlimited).
    #[serde(default)]
    pub max_concurrency: u32,
}

// ---------------------------------------------------------------------------
// API request types
// ---------------------------------------------------------------------------

/// Body for `POST /tasks`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    #[serde(rename = "type")]
    pub task_type: String,

    #[serde(default)]
    pub params: serde_json::Value,

    #[serde(default)]
    pub timeout: Option<u64>,

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
    /// Max seconds to block (default 30).
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
    pub default_timeout: u64,

    #[serde(default)]
    pub max_concurrency: u32,
}

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
            params: serde_json::json!({"batchId": "b1"}),
            status: TaskStatus::Pending,
            progress: 0,
            total: 0,
            message: None,
            result: None,
            error: None,
            create_at: "2026-01-01T00:00:00Z".into(),
            start_at: None,
            end_at: None,
            timeout: Some(300),
            created_by: Some("user1".into()),
        };
        let json = serde_json::to_string(&task).unwrap();
        let back: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "abc123");
        assert_eq!(back.status, TaskStatus::Pending);
        // Optional None fields should not appear in JSON
        assert!(!json.contains("\"message\""));
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn create_request_deserialize() {
        let json = r#"{"type":"pms.export","params":{"format":"csv"}}"#;
        let req: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task_type, "pms.export");
        assert_eq!(req.params["format"], "csv");
        assert!(req.timeout.is_none());
    }
}
