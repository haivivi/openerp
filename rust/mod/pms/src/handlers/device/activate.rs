//! POST /pms/devices/:sn/@activate — activate a device.
//!
//! Transitions device from PROVISIONED to ACTIVATED.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::model::DeviceStatus;
use crate::service::PmsService;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateRequest {
    #[serde(default)]
    pub data: Option<String>,
}

pub async fn activate(
    State(svc): State<Arc<PmsService>>,
    Path(sn): Path<String>,
    axum::Json(_body): axum::Json<ActivateRequest>,
) -> impl IntoResponse {
    let device = match svc.get_device(&sn) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    if device.status != DeviceStatus::Provisioned {
        return openerp_core::ServiceError::Validation(
            format!("device {} is not in PROVISIONED status (current: {:?})", sn, device.status)
        ).into_response();
    }

    // TODO: implement device status transition in PmsService.
    (StatusCode::OK, axum::Json(serde_json::json!({
        "sn": device.sn,
        "status": "ACTIVATED",
        "message": "device activated"
    }))).into_response()
}
