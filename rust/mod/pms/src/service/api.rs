//! Axum HTTP handlers for PMS CRUD operations.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use openerp_core::ListParams;

use super::PmsService;
use super::device::{CreateBatchInput, BatchFilters, DeviceFilters};
use super::firmware::{CreateFirmwareInput, FirmwareFilters};
use super::license::LicenseFilters;

// ── Device (read-only) ───────────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
pub struct DeviceQuery {
    #[serde(flatten)]
    pub params: ListParams,
    pub model: Option<u32>,
    pub batch_id: Option<String>,
    pub status: Option<String>,
}

pub async fn get_device(
    State(svc): State<Arc<PmsService>>,
    Path(sn): Path<String>,
) -> impl IntoResponse {
    match svc.get_device(&sn) {
        Ok(device) => (StatusCode::OK, axum::Json(serde_json::to_value(&device).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn list_devices(
    State(svc): State<Arc<PmsService>>,
    Query(query): Query<DeviceQuery>,
) -> impl IntoResponse {
    let filters = DeviceFilters {
        model: query.model,
        batch_id: query.batch_id,
        status: query.status,
    };
    match svc.list_devices(&query.params, &filters) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

// ── Batch ────────────────────────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
pub struct BatchQuery {
    #[serde(flatten)]
    pub params: ListParams,
    pub model: Option<u32>,
    pub status: Option<String>,
}

pub async fn create_batch(
    State(svc): State<Arc<PmsService>>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let name = match body["name"].as_str() {
        Some(s) => s.to_string(),
        None => return openerp_core::ServiceError::Validation("missing 'name'".into()).into_response(),
    };
    let model = match body["model"].as_u64() {
        Some(c) => c as u32,
        None => return openerp_core::ServiceError::Validation("missing 'model'".into()).into_response(),
    };
    let quantity = match body["quantity"].as_u64() {
        Some(q) => q as u32,
        None => return openerp_core::ServiceError::Validation("missing 'quantity'".into()).into_response(),
    };

    match svc.create_batch(CreateBatchInput {
        name,
        model,
        quantity,
        description: body["description"].as_str().map(String::from),
    }) {
        Ok(batch) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&batch).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn get_batch(
    State(svc): State<Arc<PmsService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.get_batch(&id) {
        Ok(batch) => (StatusCode::OK, axum::Json(serde_json::to_value(&batch).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn list_batches(
    State(svc): State<Arc<PmsService>>,
    Query(query): Query<BatchQuery>,
) -> impl IntoResponse {
    let filters = BatchFilters {
        model: query.model,
        status: query.status,
    };
    match svc.list_batches(&query.params, &filters) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn delete_batch(
    State(svc): State<Arc<PmsService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.delete_batch(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

// ── License ──────────────────────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
pub struct LicenseQuery {
    #[serde(flatten)]
    pub params: ListParams,
    pub license_type: Option<String>,
    pub status: Option<String>,
    pub sn: Option<String>,
    pub import_id: Option<String>,
}

pub async fn get_license(
    State(svc): State<Arc<PmsService>>,
    Path((license_type, number)): Path<(String, String)>,
) -> impl IntoResponse {
    match svc.get_license(&license_type, &number) {
        Ok(license) => (StatusCode::OK, axum::Json(serde_json::to_value(&license).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn list_licenses(
    State(svc): State<Arc<PmsService>>,
    Query(query): Query<LicenseQuery>,
) -> impl IntoResponse {
    let filters = LicenseFilters {
        license_type: query.license_type,
        status: query.status,
        sn: query.sn,
        import_id: query.import_id,
    };
    match svc.list_licenses(&query.params, &filters) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn delete_license(
    State(svc): State<Arc<PmsService>>,
    Path((license_type, number)): Path<(String, String)>,
) -> impl IntoResponse {
    match svc.delete_license(&license_type, &number) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

// ── Firmware ─────────────────────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
pub struct FirmwareQuery {
    #[serde(flatten)]
    pub params: ListParams,
    pub model: Option<u32>,
    pub status: Option<String>,
}

pub async fn create_firmware(
    State(svc): State<Arc<PmsService>>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let model = match body["model"].as_u64() {
        Some(c) => c as u32,
        None => return openerp_core::ServiceError::Validation("missing 'model'".into()).into_response(),
    };
    let semver = match body["semver"].as_str() {
        Some(s) => s.to_string(),
        None => return openerp_core::ServiceError::Validation("missing 'semver'".into()).into_response(),
    };
    let build = body["build"].as_u64().unwrap_or(0);
    let files: Vec<crate::model::FirmwareFile> = body["files"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|f| serde_json::from_value(f.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    match svc.create_firmware(CreateFirmwareInput {
        model,
        semver,
        build,
        files,
        release_notes: body["release_notes"].as_str().map(String::from),
    }) {
        Ok(fw) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&fw).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn get_firmware(
    State(svc): State<Arc<PmsService>>,
    Path((model, semver)): Path<(u32, String)>,
) -> impl IntoResponse {
    match svc.get_firmware(model, &semver) {
        Ok(fw) => (StatusCode::OK, axum::Json(serde_json::to_value(&fw).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn list_firmwares(
    State(svc): State<Arc<PmsService>>,
    Query(query): Query<FirmwareQuery>,
) -> impl IntoResponse {
    let filters = FirmwareFilters {
        model: query.model,
        status: query.status,
    };
    match svc.list_firmwares(&query.params, &filters) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn delete_firmware(
    State(svc): State<Arc<PmsService>>,
    Path((model, semver)): Path<(u32, String)>,
) -> impl IntoResponse {
    match svc.delete_firmware(model, &semver) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

// ── Model ────────────────────────────────────────────────────

pub async fn create_model(
    State(svc): State<Arc<PmsService>>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let code = match body["code"].as_u64() {
        Some(c) => c as u32,
        None => return openerp_core::ServiceError::Validation("missing 'code'".into()).into_response(),
    };
    let series_name = match body["series_name"].as_str() {
        Some(s) => s.to_string(),
        None => return openerp_core::ServiceError::Validation("missing 'series_name'".into()).into_response(),
    };

    match svc.create_model(
        code,
        series_name,
        body["display_name"].as_str().map(String::from),
        body["description"].as_str().map(String::from),
        body["data"].as_str().map(String::from),
    ) {
        Ok(model) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&model).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn get_model(
    State(svc): State<Arc<PmsService>>,
    Path(code): Path<u32>,
) -> impl IntoResponse {
    match svc.get_model(code) {
        Ok(model) => (StatusCode::OK, axum::Json(serde_json::to_value(&model).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn list_models(
    State(svc): State<Arc<PmsService>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    match svc.list_models(&params) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn delete_model(
    State(svc): State<Arc<PmsService>>,
    Path(code): Path<u32>,
) -> impl IntoResponse {
    match svc.delete_model(code) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

// ── SN Segment ───────────────────────────────────────────────

#[derive(Debug, Default, serde::Deserialize)]
pub struct SegmentQuery {
    pub dimension: Option<String>,
}

pub async fn list_sn_segments(
    State(svc): State<Arc<PmsService>>,
    Query(query): Query<SegmentQuery>,
) -> impl IntoResponse {
    match svc.list_sn_segments(query.dimension.as_deref()) {
        Ok(segments) => (StatusCode::OK, axum::Json(serde_json::to_value(&segments).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn upsert_sn_segment(
    State(svc): State<Arc<PmsService>>,
    axum::Json(body): axum::Json<crate::model::SNSegment>,
) -> impl IntoResponse {
    match svc.upsert_sn_segment(&body) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn delete_sn_segment(
    State(svc): State<Arc<PmsService>>,
    Path((dimension, name)): Path<(String, String)>,
) -> impl IntoResponse {
    match svc.delete_sn_segment(&dimension, &name) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

// ── License Import ───────────────────────────────────────────

pub async fn create_license_import(
    State(svc): State<Arc<PmsService>>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let license_type = match body["license_type"].as_str() {
        Some(s) => s.to_string(),
        None => return openerp_core::ServiceError::Validation("missing 'license_type'".into()).into_response(),
    };
    let source: crate::model::LicenseSource = match serde_json::from_value(body["source"].clone()) {
        Ok(s) => s,
        Err(_) => return openerp_core::ServiceError::Validation("invalid 'source'".into()).into_response(),
    };
    let name = match body["name"].as_str() {
        Some(s) => s.to_string(),
        None => return openerp_core::ServiceError::Validation("missing 'name'".into()).into_response(),
    };

    match svc.create_license_import(license_type, source, name) {
        Ok(import) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&import).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn get_license_import(
    State(svc): State<Arc<PmsService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.get_license_import(&id) {
        Ok(import) => (StatusCode::OK, axum::Json(serde_json::to_value(&import).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn list_license_imports(
    State(svc): State<Arc<PmsService>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    match svc.list_license_imports(&params) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn delete_license_import(
    State(svc): State<Arc<PmsService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.delete_license_import(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}
