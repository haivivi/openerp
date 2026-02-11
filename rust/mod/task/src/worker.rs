use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::engine::TaskEngine;

/// Configuration for the background worker.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// How often to scan for pending tasks to dispatch (seconds).
    pub dispatch_interval: u64,
    /// How often to check for timed-out tasks (seconds).
    pub watchdog_interval: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            dispatch_interval: 2,
            watchdog_interval: 10,
        }
    }
}

/// Start background loops: dispatch scanner + timeout watchdog.
///
/// Returns a CancellationToken that can be used to stop the worker.
pub fn start(engine: Arc<TaskEngine>, config: WorkerConfig) -> CancellationToken {
    let cancel = CancellationToken::new();

    // --- Dispatch loop ---
    {
        let engine = Arc::clone(&engine);
        let cancel = cancel.clone();
        let interval = Duration::from_secs(config.dispatch_interval);

        tokio::spawn(async move {
            info!("task dispatch worker started (interval={interval:?})");
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        info!("task dispatch worker stopped");
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        debug!("dispatch scan");
                        if let Err(e) = engine.dispatch_all().await {
                            error!("dispatch error: {e}");
                        }
                    }
                }
            }
        });
    }

    // --- Watchdog loop ---
    {
        let engine = Arc::clone(&engine);
        let cancel = cancel.clone();
        let interval = Duration::from_secs(config.watchdog_interval);

        tokio::spawn(async move {
            info!("task watchdog started (interval={interval:?})");
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        info!("task watchdog stopped");
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        debug!("watchdog scan");
                        match engine.check_timeouts().await {
                            Ok(0) => {}
                            Ok(n) => info!("watchdog: timed out {n} tasks"),
                            Err(e) => error!("watchdog error: {e}"),
                        }
                    }
                }
            }
        });
    }

    cancel
}
