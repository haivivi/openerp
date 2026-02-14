//! App facet device handlers.
//!
//! Hand-written: reads from Device KvStore, projects to AppDevice struct.
//! Auth is handled by the handler (not the framework).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use openerp_core::{ListResult, ServiceError};
use openerp_store::KvOps;

use crate::model::Device;

// Import the facet struct.
#[path = "../../../dsl/rest/app/device.rs"]
mod app_device_def;
use app_device_def::AppDevice;

type AppState = Arc<KvOps<Device>>;

/// Build routes for devices in the app facet.
pub fn routes(ops: AppState) -> Router {
    Router::new()
        .route("/devices", get(list_devices))
        .route("/devices/{sn}", get(get_device))
        .with_state(ops)
}

/// Project a full Device model into the AppDevice facet struct.
fn to_app(d: &Device) -> AppDevice {
    AppDevice {
        sn: d.sn.clone(),
        model: d.model,
        status: d.status.clone(),
        display_name: d.display_name.clone(),
    }
}

async fn list_devices(
    State(ops): State<AppState>,
) -> Result<Json<ListResult<AppDevice>>, ServiceError> {
    let devices = ops.list()?;
    let items: Vec<AppDevice> = devices.iter().map(to_app).collect();
    let total = items.len();
    Ok(Json(ListResult { items, total }))
}

async fn get_device(
    State(ops): State<AppState>,
    Path(sn): Path<String>,
) -> Result<Json<AppDevice>, ServiceError> {
    let device = ops.get_or_err(&sn)?;
    Ok(Json(to_app(&device)))
}
