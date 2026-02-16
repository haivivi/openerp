use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use openerp_core::{ListParams, ListResult, ServiceError};
use openerp_store::KvOps;
use crate::model::Device;
use crate::mfg::MfgDevice;

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

async fn list(
    State(ops): State<S>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResult<MfgDevice>>, ServiceError> {
    let result = ops.list_paginated(&params)?;
    let items = result.items.iter().map(project).collect();
    Ok(Json(ListResult { items, has_more: result.has_more }))
}

async fn get_one(State(ops): State<S>, Path(sn): Path<String>) -> Result<Json<MfgDevice>, ServiceError> {
    let d = ops.get_or_err(&sn)?;
    Ok(Json(project(&d)))
}
