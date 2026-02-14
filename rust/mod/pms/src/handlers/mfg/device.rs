use std::sync::Arc;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use openerp_core::{ListResult, ServiceError};
use openerp_store::KvOps;
use crate::model::Device;

#[path = "../../../dsl/rest/mfg/device.rs"]
mod mfg_device_def;
use mfg_device_def::MfgDevice;

type S = Arc<KvOps<Device>>;

pub fn routes(ops: S) -> Router {
    Router::new()
        .route("/devices", get(list))
        .route("/devices/{sn}", get(get_one))
        .with_state(ops)
}

fn project(d: &Device) -> MfgDevice {
    MfgDevice {
        sn: d.sn.clone(),
        model: d.model,
        status: d.status.clone(),
        sku: d.sku.clone(),
        imei: d.imei.clone(),
        licenses: d.licenses.clone(),
        display_name: d.display_name.clone(),
    }
}

async fn list(State(ops): State<S>) -> Result<Json<ListResult<MfgDevice>>, ServiceError> {
    let all = ops.list()?;
    let items: Vec<MfgDevice> = all.iter().map(project).collect();
    let total = items.len();
    Ok(Json(ListResult { items, total }))
}

async fn get_one(State(ops): State<S>, Path(sn): Path<String>) -> Result<Json<MfgDevice>, ServiceError> {
    let d = ops.get_or_err(&sn)?;
    Ok(Json(project(&d)))
}
