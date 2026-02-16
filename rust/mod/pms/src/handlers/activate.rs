//! Device activation handler — transitions device from provisioned → active.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use openerp_core::ServiceError;
use openerp_store::KvOps;

use crate::model::Device;
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

    match device.status.as_str() {
        "provisioned" | "inactive" => {}
        "active" => {
            return Err(ServiceError::Validation("device is already active".into()));
        }
        other => {
            return Err(ServiceError::Validation(format!(
                "cannot activate device in '{}' status",
                other
            )));
        }
    }

    device.status = "active".into();
    ops.save(device)?;

    Ok(Json(ActivateResponse {
        sn,
        status: "active".into(),
    }))
}
