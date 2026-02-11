pub mod api;
pub mod engine;
pub mod model;
pub mod store;
pub mod worker;

use std::sync::Arc;

use axum::Router;
use openerp_core::Module;
use openerp_sql::SQLStore;

use engine::TaskEngine;
use store::TaskStore;
use worker::WorkerConfig;

/// The Task module — async task execution engine.
///
/// Embed this in a business service to get handler registration, submission,
/// long-poll progress, cancellation, and timeout management.
pub struct TaskModule {
    engine: Arc<TaskEngine>,
    _worker_cancel: tokio_util::sync::CancellationToken,
}

impl TaskModule {
    /// Create the task module, initialise storage, and start background workers.
    pub fn new(db: Arc<dyn SQLStore>) -> Result<Self, openerp_core::ServiceError> {
        Self::with_config(db, WorkerConfig::default())
    }

    /// Create with explicit worker configuration.
    pub fn with_config(
        db: Arc<dyn SQLStore>,
        worker_config: WorkerConfig,
    ) -> Result<Self, openerp_core::ServiceError> {
        let store = Arc::new(TaskStore::new(db)?);
        let engine = Arc::new(TaskEngine::new(store));
        let cancel = worker::start(Arc::clone(&engine), worker_config);

        Ok(Self {
            engine,
            _worker_cancel: cancel,
        })
    }

    /// Get a reference to the TaskEngine for programmatic handler registration.
    pub fn engine(&self) -> &Arc<TaskEngine> {
        &self.engine
    }
}

impl Module for TaskModule {
    fn name(&self) -> &str {
        "task"
    }

    fn routes(&self) -> Router {
        api::router(Arc::clone(&self.engine))
    }
}
