use std::sync::Arc;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use openerp_core::{ListResult, ServiceError};
use openerp_store::KvOps;
use crate::model::Batch;

#[path = "../../../dsl/rest/mfg/batch.rs"]
mod mfg_batch_def;
use mfg_batch_def::MfgBatch;

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
        status: b.status.clone(),
        display_name: b.display_name.clone(),
    }
}

async fn list(State(ops): State<S>) -> Result<Json<ListResult<MfgBatch>>, ServiceError> {
    let all = ops.list()?;
    let items: Vec<MfgBatch> = all.iter().map(project).collect();
    let total = items.len();
    Ok(Json(ListResult { items, total }))
}

async fn get_one(State(ops): State<S>, Path(id): Path<String>) -> Result<Json<MfgBatch>, ServiceError> {
    let b = ops.get_or_err(&id)?;
    Ok(Json(project(&b)))
}
