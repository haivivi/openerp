use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, delete},
    Json,
};
use serde::Deserialize;

use openerp_core::ListParams;
use crate::model::{License, LicenseImport, LicenseSource};
use crate::service::license::LicenseFilters;
use super::{AppState, ApiError, ok_json};

pub fn routes() -> Router<AppState> {
    Router::new()
        // License CRUD
        .route("/licenses", get(list_licenses))
        .route("/licenses/{license_type}/{number}", get(get_license).patch(update_license).delete(delete_license))
        // LicenseImport CRUD
        .route("/license-imports", post(create_license_import).get(list_license_imports))
        .route("/license-imports/{id}", get(get_license_import).delete(delete_license_import))
        .route("/license-imports/{id}/import", post(execute_import))
        .route("/license-imports/{id}/generate", post(execute_generate))
}

// ── License ──

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LicenseQuery {
    #[serde(flatten)]
    params: ListParams,
    license_type: Option<String>,
    status: Option<String>,
    sn: Option<String>,
    import_id: Option<String>,
}

async fn get_license(
    State(svc): State<AppState>,
    Path((license_type, number)): Path<(String, String)>,
) -> Result<Json<License>, ApiError> {
    ok_json(svc.get_license(&license_type, &number))
}

async fn list_licenses(
    State(svc): State<AppState>,
    Query(q): Query<LicenseQuery>,
) -> Result<Json<openerp_core::ListResult<License>>, ApiError> {
    let filters = LicenseFilters {
        license_type: q.license_type,
        status: q.status,
        sn: q.sn,
        import_id: q.import_id,
    };
    ok_json(svc.list_licenses(&q.params, &filters))
}

async fn update_license(
    State(svc): State<AppState>,
    Path((license_type, number)): Path<(String, String)>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<License>, ApiError> {
    ok_json(svc.update_license(&license_type, &number, patch))
}

async fn delete_license(
    State(svc): State<AppState>,
    Path((license_type, number)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    svc.delete_license(&license_type, &number).map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ── LicenseImport ──

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateLicenseImportBody {
    #[serde(rename = "type")]
    license_type: String,
    source: LicenseSource,
    name: String,
}

async fn create_license_import(
    State(svc): State<AppState>,
    Json(body): Json<CreateLicenseImportBody>,
) -> Result<Json<LicenseImport>, ApiError> {
    ok_json(svc.create_license_import(
        body.license_type,
        body.source,
        body.name,
    ))
}

async fn get_license_import(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<LicenseImport>, ApiError> {
    ok_json(svc.get_license_import(&id))
}

async fn list_license_imports(
    State(svc): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<openerp_core::ListResult<LicenseImport>>, ApiError> {
    ok_json(svc.list_license_imports(&params))
}

async fn delete_license_import(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    svc.delete_license_import(&id).map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ── Import/Generate actions ──

#[derive(Deserialize)]
struct ImportEntry {
    number: String,
    data: Option<String>,
}

#[derive(Deserialize)]
struct ImportBody {
    entries: Vec<ImportEntry>,
}

async fn execute_import(
    State(svc): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ImportBody>,
) -> Result<Json<Vec<License>>, ApiError> {
    let entries: Vec<(String, Option<String>)> = body
        .entries
        .into_iter()
        .map(|e| (e.number, e.data))
        .collect();
    ok_json(svc.execute_import(&id, entries))
}

#[derive(Deserialize)]
struct GenerateBody {
    prefix: String,
    count: u64,
}

async fn execute_generate(
    State(svc): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<GenerateBody>,
) -> Result<Json<Vec<License>>, ApiError> {
    ok_json(svc.execute_generate(&id, &body.prefix, body.count))
}
