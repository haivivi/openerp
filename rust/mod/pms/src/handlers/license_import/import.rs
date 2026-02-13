//! POST /pms/license-imports/:id/@import — execute bulk license import.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::service::PmsService;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportLicensesRequest {
    /// License numbers to import.
    pub numbers: Vec<String>,
}

pub async fn import_licenses(
    State(svc): State<Arc<PmsService>>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<ImportLicensesRequest>,
) -> impl IntoResponse {
    // Verify import record exists.
    let _import_record = match svc.get_license_import(&id) {
        Ok(r) => r,
        Err(e) => return e.into_response(),
    };

    if body.numbers.is_empty() {
        return openerp_core::ServiceError::Validation("no license numbers provided".into()).into_response();
    }

    // Convert numbers to (number, metadata) tuples.
    let entries: Vec<(String, Option<String>)> = body.numbers
        .into_iter()
        .map(|n| (n, None))
        .collect();

    // Execute the import through the service method which handles
    // license creation and count tracking.
    match svc.execute_import(&id, entries) {
        Ok(licenses) => {
            (StatusCode::OK, axum::Json(serde_json::json!({
                "imported": licenses.len(),
                "import_id": id,
            }))).into_response()
        }
        Err(e) => e.into_response(),
    }
}
