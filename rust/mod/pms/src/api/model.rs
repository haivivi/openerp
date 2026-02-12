use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, delete},
    Json,
};
use serde::Deserialize;

use openerp_core::ListParams;
use crate::model::Model;
use super::{AppState, ApiError, ok_json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/models", post(create_model).get(list_models))
        .route("/models/{code}", get(get_model).patch(update_model).delete(delete_model))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateModelBody {
    code: u32,
    series_name: String,
    display_name: Option<String>,
    description: Option<String>,
    data: Option<String>,
}

async fn create_model(
    State(svc): State<AppState>,
    Json(body): Json<CreateModelBody>,
) -> Result<Json<Model>, ApiError> {
    ok_json(svc.create_model(
        body.code,
        body.series_name,
        body.display_name,
        body.description,
        body.data,
    ))
}

async fn get_model(
    State(svc): State<AppState>,
    Path(code): Path<u32>,
) -> Result<Json<Model>, ApiError> {
    ok_json(svc.get_model(code))
}

async fn list_models(
    State(svc): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<openerp_core::ListResult<Model>>, ApiError> {
    ok_json(svc.list_models(&params))
}

async fn update_model(
    State(svc): State<AppState>,
    Path(code): Path<u32>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<Model>, ApiError> {
    ok_json(svc.update_model(code, patch))
}

async fn delete_model(
    State(svc): State<AppState>,
    Path(code): Path<u32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    svc.delete_model(code).map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}
