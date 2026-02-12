use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::engine::TaskEngine;

/// Configuration for the background watchdog.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// How often to check for timed-out tasks (seconds).
    pub timeout_check_interval: u64,
    /// How often to check for stale (no heartbeat) tasks (seconds).
    pub stale_check_interval: u64,
    /// A RUNNING task with no heartbeat for this many seconds is considered stale.
    pub stale_threshold: i64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            timeout_check_interval: 10,
            stale_check_interval: 30,
            stale_threshold: 120,
        }
    }
}

/// Start background watchdog loops.
///
/// - **Timeout watchdog**: marks RUNNING tasks as FAILED if they exceed `timeout_secs`.
/// - **Stale watchdog**: resets RUNNING tasks to PENDING if `last_active_at` is too old.
///
/// Returns a CancellationToken that stops the workers when cancelled.
pub fn start(engine: Arc<TaskEngine>, config: WorkerConfig) -> CancellationToken {
    let cancel = CancellationToken::new();

    // --- Timeout watchdog ---
    {
        let engine = Arc::clone(&engine);
        let cancel = cancel.clone();
        let interval = Duration::from_secs(config.timeout_check_interval);

        tokio::spawn(async move {
            info!("task timeout watchdog started (interval={interval:?})");
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        info!("task timeout watchdog stopped");
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        debug!("timeout watchdog scan");
                        match engine.check_timeouts() {
                            Ok(0) => {}
                            Ok(n) => info!("timeout watchdog: timed out {n} tasks"),
                            Err(e) => error!("timeout watchdog error: {e}"),
                        }
                    }
                }
            }
        });
    }

    // --- Stale watchdog ---
    {
        let engine = Arc::clone(&engine);
        let cancel = cancel.clone();
        let interval = Duration::from_secs(config.stale_check_interval);
        let threshold = config.stale_threshold;

        tokio::spawn(async move {
            info!(
                "task stale watchdog started (interval={interval:?}, threshold={threshold}s)"
            );
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        info!("task stale watchdog stopped");
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        debug!("stale watchdog scan");
                        match engine.check_stale(threshold) {
                            Ok(0) => {}
                            Ok(n) => info!("stale watchdog: reset {n} stale tasks"),
                            Err(e) => error!("stale watchdog error: {e}"),
                        }
                    }
                }
            }
        });
    }

    cancel
}
