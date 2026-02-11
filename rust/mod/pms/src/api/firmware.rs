use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, delete},
    Json,
};
use serde::Deserialize;

use openerp_core::ListParams;
use crate::model::{Firmware, FirmwareFile};
use crate::service::firmware::FirmwareFilters;
use super::{AppState, ApiError, ok_json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/firmwares", post(create_firmware).get(list_firmwares))
        .route("/firmwares/{model}/{semver}", get(get_firmware).patch(update_firmware).delete(delete_firmware))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateFirmwareBody {
    model: u32,
    semver: String,
    build: u64,
    #[serde(default)]
    files: Vec<FirmwareFile>,
    release_notes: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareQuery {
    #[serde(flatten)]
    params: ListParams,
    model: Option<u32>,
    status: Option<String>,
}

async fn create_firmware(
    State(svc): State<AppState>,
    Json(body): Json<CreateFirmwareBody>,
) -> Result<Json<Firmware>, ApiError> {
    use crate::service::firmware::CreateFirmwareInput;
    ok_json(svc.create_firmware(CreateFirmwareInput {
        model: body.model,
        semver: body.semver,
        build: body.build,
        files: body.files,
        release_notes: body.release_notes,
    }))
}

async fn get_firmware(
    State(svc): State<AppState>,
    Path((model, semver)): Path<(u32, String)>,
) -> Result<Json<Firmware>, ApiError> {
    ok_json(svc.get_firmware(model, &semver))
}

async fn list_firmwares(
    State(svc): State<AppState>,
    Query(q): Query<FirmwareQuery>,
) -> Result<Json<openerp_core::ListResult<Firmware>>, ApiError> {
    let filters = FirmwareFilters {
        model: q.model,
        status: q.status,
    };
    ok_json(svc.list_firmwares(&q.params, &filters))
}

async fn update_firmware(
    State(svc): State<AppState>,
    Path((model, semver)): Path<(u32, String)>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<Firmware>, ApiError> {
    ok_json(svc.update_firmware(model, &semver, patch))
}

async fn delete_firmware(
    State(svc): State<AppState>,
    Path((model, semver)): Path<(u32, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    svc.delete_firmware(model, &semver).map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}
