use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use openerp_core::{ListParams, ListResult, ServiceError};
use openerp_store::KvOps;
use crate::model::Batch;
use crate::mfg::MfgBatch;

type S = Arc<KvOps<Batch>>;

pub fn routes(ops: S) -> Router {
    Router::new()
        .route("/batches", get(list))
        .route("/batches/{id}", get(get_one))
        .with_state(ops)
}

fn project(b: &Batch) -> MfgBatch {
    MfgBatch {
        id: b.id.to_string(),
        model: b.model,
        quantity: b.quantity,
        provisioned_count: b.provisioned_count,
        status: b.status.to_string(),
        display_name: b.display_name.clone(),
    }
}

async fn list(
    State(ops): State<S>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResult<MfgBatch>>, ServiceError> {
    let result = ops.list_paginated(&params)?;
    let items = result.items.iter().map(project).collect();
    Ok(Json(ListResult { items, has_more: result.has_more }))
}

async fn get_one(State(ops): State<S>, Path(id): Path<String>) -> Result<Json<MfgBatch>, ServiceError> {
    let b = ops.get_or_err(&id)?;
    Ok(Json(project(&b)))
}
