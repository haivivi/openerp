use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, delete},
    Json,
};
use serde::Deserialize;

use openerp_core::ListParams;
use crate::model::Batch;
use crate::service::device::{BatchFilters, CreateBatchInput};
use super::{AppState, ApiError, ok_json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/batches", post(create_batch).get(list_batches))
        .route("/batches/{id}", get(get_batch).patch(update_batch).delete(delete_batch))
        .route("/batches/{id}/provision", post(provision_batch))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBatchBody {
    name: String,
    model: u32,
    quantity: u32,
    description: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchQuery {
    #[serde(flatten)]
    params: ListParams,
    model: Option<u32>,
    status: Option<String>,
}

async fn create_batch(
    State(svc): State<AppState>,
    Json(body): Json<CreateBatchBody>,
) -> Result<Json<Batch>, ApiError> {
    ok_json(svc.create_batch(CreateBatchInput {
        name: body.name,
        model: body.model,
        quantity: body.quantity,
        description: body.description,
    }))
}

async fn get_batch(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Batch>, ApiError> {
    ok_json(svc.get_batch(&id))
}

async fn list_batches(
    State(svc): State<AppState>,
    Query(q): Query<BatchQuery>,
) -> Result<Json<openerp_core::ListResult<Batch>>, ApiError> {
    let filters = BatchFilters {
        model: q.model,
        status: q.status,
    };
    ok_json(svc.list_batches(&q.params, &filters))
}

async fn update_batch(
    State(svc): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<Batch>, ApiError> {
    ok_json(svc.update_batch(&id, patch))
}

async fn delete_batch(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    svc.delete_batch(&id).map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

async fn provision_batch(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Batch>, ApiError> {
    ok_json(svc.provision_batch(&id))
}
