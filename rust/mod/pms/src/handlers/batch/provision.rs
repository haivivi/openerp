//! POST /pms/batches/:id/@provision — execute batch device provisioning.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::model::BatchStatus;
use crate::service::PmsService;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionBatchRequest {
    #[serde(default)]
    pub count: Option<u32>,
}

pub async fn provision(
    State(svc): State<Arc<PmsService>>,
    Path(id): Path<String>,
    axum::Json(_body): axum::Json<ProvisionBatchRequest>,
) -> impl IntoResponse {
    let batch = match svc.get_batch(&id) {
        Ok(b) => b,
        Err(e) => return e.into_response(),
    };

    // Only Draft batches can be provisioned.
    if batch.status != BatchStatus::Draft {
        return openerp_core::ServiceError::Validation(
            format!("batch {} is not in DRAFT status (current: {:?})", id, batch.status)
        ).into_response();
    }

    // Delegate to PmsService.provision_batch which handles the
    // status transition and device generation.
    match svc.provision_batch(&id) {
        Ok(updated) => {
            (StatusCode::ACCEPTED, axum::Json(serde_json::to_value(&updated).unwrap())).into_response()
        }
        Err(e) => e.into_response(),
    }
}
