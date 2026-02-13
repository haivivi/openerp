//! POST /pms/firmwares/:model/:semver/@upload — upload firmware file.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::model::FirmwareFile;
use crate::service::PmsService;

pub async fn upload(
    State(svc): State<Arc<PmsService>>,
    Path((model, semver)): Path<(u32, String)>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    // Verify firmware exists.
    let firmware = match svc.get_firmware(model, &semver) {
        Ok(fw) => fw,
        Err(e) => return e.into_response(),
    };

    let file_name = body["name"].as_str().unwrap_or("firmware.bin");
    let file_url = match body["url"].as_str() {
        Some(u) if !u.is_empty() => u,
        _ => return openerp_core::ServiceError::Validation("missing 'url' field".into()).into_response(),
    };
    let file_md5 = match body["md5"].as_str() {
        Some(m) if !m.is_empty() => m,
        _ => return openerp_core::ServiceError::Validation("missing 'md5' field".into()).into_response(),
    };

    // Add file entry to firmware.
    let mut files = firmware.files.clone();
    files.push(FirmwareFile {
        name: file_name.to_string(),
        url: file_url.to_string(),
        md5: file_md5.to_string(),
        size: body["size"].as_u64(),
    });

    let patch = serde_json::json!({
        "files": files,
    });

    match svc.update_firmware(model, &semver, patch) {
        Ok(updated) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&updated).unwrap())).into_response()
        }
        Err(e) => e.into_response(),
    }
}
