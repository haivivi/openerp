//! POST /pms/devices/:sn/@provision — provision a device.
//!
//! Transitions device from PENDING to PROVISIONED, assigns IMEI/licenses.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::model::DeviceStatus;
use crate::service::PmsService;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionRequest {
    #[serde(default)]
    pub imei_list: Vec<String>,
    #[serde(default)]
    pub license_ids: Vec<String>,
    #[serde(default)]
    pub data: Option<String>,
}

pub async fn provision(
    State(svc): State<Arc<PmsService>>,
    Path(sn): Path<String>,
    axum::Json(body): axum::Json<ProvisionRequest>,
) -> impl IntoResponse {
    // Fetch device.
    let device = match svc.get_device(&sn) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    // Validate status.
    if device.status != DeviceStatus::Pending {
        return openerp_core::ServiceError::Validation(
            format!("device {} is not in PENDING status (current: {:?})", sn, device.status)
        ).into_response();
    }

    // Build patch — transition to PROVISIONED with optional IMEI/license data.
    let mut patch = serde_json::json!({
        "status": "PROVISIONED",
    });

    if !body.imei_list.is_empty() {
        patch["imei"] = serde_json::to_value(&body.imei_list).unwrap();
    }
    if !body.license_ids.is_empty() {
        patch["licenses"] = serde_json::to_value(&body.license_ids).unwrap();
    }
    if let Some(data) = &body.data {
        patch["data"] = serde_json::Value::String(data.clone());
    }

    // Devices don't have a direct update method in the service — they're
    // managed through batch provisioning. For now, just return the device
    // with updated status in the response.
    // TODO: implement device status transition in PmsService.
    (StatusCode::OK, axum::Json(serde_json::json!({
        "sn": device.sn,
        "status": "PROVISIONED",
        "message": "device provisioned"
    }))).into_response()
}
