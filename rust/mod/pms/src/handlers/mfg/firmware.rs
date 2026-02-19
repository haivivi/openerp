use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use openerp_core::{ListParams, ListResult, ServiceError};
use openerp_store::KvOps;
use crate::model::Firmware;
use crate::mfg::MfgFirmware;

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
        status: f.status.to_string(),
        display_name: f.display_name.clone(),
    }
}

async fn list(
    State(ops): State<S>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResult<MfgFirmware>>, ServiceError> {
    let result = ops.list_paginated(&params)?;
    let items = result.items.iter().map(project).collect();
    Ok(Json(ListResult { items, has_more: result.has_more }))
}

async fn get_one(State(ops): State<S>, Path(id): Path<String>) -> Result<Json<MfgFirmware>, ServiceError> {
    let f = ops.get_or_err(&id)?;
    Ok(Json(project(&f)))
}
