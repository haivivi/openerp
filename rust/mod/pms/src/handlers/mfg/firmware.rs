use std::sync::Arc;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use openerp_core::{ListResult, ServiceError};
use openerp_store::KvOps;
use crate::model::Firmware;

#[path = "../../../dsl/rest/mfg/firmware.rs"]
mod mfg_firmware_def;
use mfg_firmware_def::MfgFirmware;

type S = Arc<KvOps<Firmware>>;

pub fn routes(ops: S) -> Router {
    Router::new()
        .route("/firmwares", get(list))
        .route("/firmwares/{id}", get(get_one))
        .with_state(ops)
}

fn project(f: &Firmware) -> MfgFirmware {
    MfgFirmware {
        id: f.id.to_string(),
        model: f.model,
        semver: f.semver.to_string(),
        build: f.build,
        status: f.status.clone(),
        display_name: f.display_name.clone(),
    }
}

async fn list(State(ops): State<S>) -> Result<Json<ListResult<MfgFirmware>>, ServiceError> {
    let all = ops.list()?;
    let items: Vec<MfgFirmware> = all.iter().map(project).collect();
    Ok(Json(ListResult { items, has_more: false }))
}

async fn get_one(State(ops): State<S>, Path(id): Path<String>) -> Result<Json<MfgFirmware>, ServiceError> {
    let f = ops.get_or_err(&id)?;
    Ok(Json(project(&f)))
}
