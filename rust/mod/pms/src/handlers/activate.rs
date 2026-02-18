//! Device activation handler — transitions device from provisioned → active.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use openerp_core::ServiceError;
use openerp_store::KvOps;

use crate::model::{Device, DeviceStatus};
use crate::mfg::ActivateResponse;

pub fn routes(kv: Arc<dyn openerp_kv::KVStore>) -> Router {
    let ops = Arc::new(KvOps::<Device>::new(kv));
    Router::new()
        .route("/devices/{sn}/@activate", post(activate))
        .with_state(ops)
}

async fn activate(
    State(ops): State<Arc<KvOps<Device>>>,
    Path(sn): Path<String>,
) -> Result<Json<ActivateResponse>, ServiceError> {
    let mut device = ops.get_or_err(&sn)?;

    match device.status {
        DeviceStatus::Provisioned | DeviceStatus::Inactive => {}
        DeviceStatus::Active => {
            return Err(ServiceError::Validation("device is already active".into()));
        }
    }

    device.status = DeviceStatus::Active;
    ops.save(device)?;

    Ok(Json(ActivateResponse {
        sn,
        status: DeviceStatus::Active.to_string(),
    }))
}
