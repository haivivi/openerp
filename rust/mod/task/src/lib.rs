// TODO: API layer will be generated from lib/api/schema/task.api
pub mod engine;
pub mod model;
pub mod store;
pub mod worker;

use std::sync::Arc;

use axum::Router;
use openerp_core::Module;
use openerp_kv::KVStore;
use openerp_sql::SQLStore;
use openerp_tsdb::TsDb;

use engine::TaskEngine;
use store::TaskStore;
use worker::WorkerConfig;

/// The Task module — state machine + notification center for async tasks.
///
/// Does **not** execute tasks itself. Instead it:
/// - Records task state in SQL (proper columns, lightweight progress updates).
/// - Stores executor runtime data in KV (input params, checkpoint state).
/// - Stores executor logs in TSDB.
/// - Notifies executors when tasks are created (trigger callbacks).
/// - Provides HTTP APIs for executors to claim, report progress, complete/fail.
/// - Runs watchdog loops to detect stale/timed-out tasks.
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
        // TODO: Will be replaced with generated API from lib/api/server/task
        Router::new()
    }
}
