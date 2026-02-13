pub mod engine;
pub mod model;
pub mod store;
pub mod worker;
pub mod handlers;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use openerp_core::Module;
use openerp_kv::KVStore;
use openerp_sql::SQLStore;
use openerp_tsdb::TsDb;

use engine::TaskEngine;
use store::TaskStore;
use worker::WorkerConfig;

/// The Task module — state machine + notification center for async tasks.
pub struct TaskModule {
    engine: Arc<TaskEngine>,
    /// Dropping this guard cancels the background watchdog loops.
    _worker_cancel: tokio_util::sync::DropGuard,
}

impl TaskModule {
    /// Create the task module, initialise storage, and start background watchdogs.
    pub fn new(
        db: Arc<dyn SQLStore>,
        kv: Arc<dyn KVStore>,
        ts: Arc<dyn TsDb>,
    ) -> Result<Self, openerp_core::ServiceError> {
        Self::with_config(db, kv, ts, WorkerConfig::default())
    }

    /// Create with explicit worker configuration.
    pub fn with_config(
        db: Arc<dyn SQLStore>,
        kv: Arc<dyn KVStore>,
        ts: Arc<dyn TsDb>,
        worker_config: WorkerConfig,
    ) -> Result<Self, openerp_core::ServiceError> {
        let store = Arc::new(TaskStore::new(db, kv, ts)?);
        let engine = Arc::new(TaskEngine::new(store));
        let cancel = worker::start(Arc::clone(&engine), worker_config);

        Ok(Self {
            engine,
            _worker_cancel: cancel.drop_guard(),
        })
    }

    /// Get a reference to the TaskEngine for programmatic use.
    pub fn engine(&self) -> &Arc<TaskEngine> {
        &self.engine
    }
}

impl Module for TaskModule {
    fn name(&self) -> &str {
        "task"
    }

    fn routes(&self) -> Router {
        let engine = self.engine.clone();

        Router::new()
            // Task CRUD (create + list + get)
            .route("/tasks", post(task_api::create_task).get(task_api::list_tasks))
            .route("/tasks/{id}", get(task_api::get_task))
            // Task custom actions
            .route("/tasks/{id}/@claim", post(handlers::task::claim))
            .route("/tasks/{id}/@progress", post(handlers::task::progress))
            .route("/tasks/{id}/@complete", post(handlers::task::complete))
            .route("/tasks/{id}/@fail", post(handlers::task::fail))
            .route("/tasks/{id}/@cancel", post(handlers::task::cancel))
            .route("/tasks/{id}/@poll", get(handlers::task::poll))
            .route("/tasks/{id}/@log", post(handlers::task::log_write))
            .route("/tasks/{id}/@logs", get(handlers::task::log_read))
            // TaskType CRUD
            .route("/task-types", post(task_api::create_task_type).get(task_api::list_task_types))
            .route("/task-types/{id}", get(task_api::get_task_type).delete(task_api::delete_task_type))
            .with_state(engine)
    }
}

/// Inline API handlers for Task CRUD.
mod task_api {
    use std::sync::Arc;

    use axum::extract::{Path, Query, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use crate::engine::TaskEngine;
    use crate::model::{CreateTaskRequest, TaskListQuery, RegisterTaskTypeRequest};

    pub async fn create_task(
        State(engine): State<Arc<TaskEngine>>,
        axum::Json(body): axum::Json<CreateTaskRequest>,
    ) -> impl IntoResponse {
        match engine.create_task(
            &body.task_type,
            &body.input,
            body.timeout_secs,
            body.max_retries,
            body.created_by,
        ).await {
            Ok(task) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&task).unwrap())).into_response(),
            Err(e) => e.into_response(),
        }
    }

    pub async fn get_task(
        State(engine): State<Arc<TaskEngine>>,
        Path(id): Path<String>,
    ) -> impl IntoResponse {
        match engine.get(&id) {
            Ok(task) => (StatusCode::OK, axum::Json(serde_json::to_value(&task).unwrap())).into_response(),
            Err(e) => e.into_response(),
        }
    }

    pub async fn list_tasks(
        State(engine): State<Arc<TaskEngine>>,
        Query(query): Query<TaskListQuery>,
    ) -> impl IntoResponse {
        match engine.list(query) {
            Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
            Err(e) => e.into_response(),
        }
    }

    pub async fn create_task_type(
        State(engine): State<Arc<TaskEngine>>,
        axum::Json(body): axum::Json<RegisterTaskTypeRequest>,
    ) -> impl IntoResponse {
        match engine.register_task_type(body).await {
            Ok(tt) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&tt).unwrap())).into_response(),
            Err(e) => e.into_response(),
        }
    }

    pub async fn get_task_type(
        State(engine): State<Arc<TaskEngine>>,
        Path(id): Path<String>,
    ) -> impl IntoResponse {
        match engine.get_task_type(&id).await {
            Ok(tt) => (StatusCode::OK, axum::Json(serde_json::to_value(&tt).unwrap())).into_response(),
            Err(e) => e.into_response(),
        }
    }

    pub async fn list_task_types(
        State(engine): State<Arc<TaskEngine>>,
    ) -> impl IntoResponse {
        match engine.list_task_types().await {
            Ok(types) => (StatusCode::OK, axum::Json(serde_json::to_value(&types).unwrap())).into_response(),
            Err(e) => e.into_response(),
        }
    }

    pub async fn delete_task_type(
        State(engine): State<Arc<TaskEngine>>,
        Path(id): Path<String>,
    ) -> impl IntoResponse {
        match engine.delete_task_type(&id).await {
            Ok(()) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => e.into_response(),
        }
    }
}
