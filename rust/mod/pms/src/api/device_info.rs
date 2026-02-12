use axum::{
    Router,
    extract::{Path, Query, State},
    routing::get,
    Json,
};
use serde::Deserialize;

use crate::service::device_info::DeviceInfo;
use super::{AppState, ApiError, ok_json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/device-info/{sn}", get(get_device_info_by_sn))
        .route("/device-info/by-secret", get(get_device_info_by_secret))
}

async fn get_device_info_by_sn(
    State(svc): State<AppState>,
    Path(sn): Path<String>,
) -> Result<Json<DeviceInfo>, ApiError> {
    ok_json(svc.get_device_info_by_sn(&sn))
}

#[derive(Deserialize)]
struct SecretQuery {
    secret: String,
}

async fn get_device_info_by_secret(
    State(svc): State<AppState>,
    Query(q): Query<SecretQuery>,
) -> Result<Json<DeviceInfo>, ApiError> {
    ok_json(svc.get_device_info_by_secret(&q.secret))
}
