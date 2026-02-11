pub mod model;
pub mod firmware;
pub mod batch;
pub mod device;
pub mod license;
pub mod sn;
pub mod device_info;

use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use openerp_core::ServiceError;

use crate::service::PmsService;

/// Shared application state.
pub type AppState = Arc<PmsService>;

/// Build the PMS API router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/pms/v1", api_routes())
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(model::routes())
        .merge(firmware::routes())
        .merge(batch::routes())
        .merge(device::routes())
        .merge(license::routes())
        .merge(sn::routes())
        .merge(device_info::routes())
}

/// Standard API error response body.
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: u16,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.code)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = Json(serde_json::json!({
            "error": {
                "code": self.code,
                "message": self.message,
            }
        }));
        (status, body).into_response()
    }
}

impl From<ServiceError> for ApiError {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::NotFound(msg) => ApiError {
                code: 404,
                message: msg,
            },
            ServiceError::Validation(msg) => ApiError {
                code: 400,
                message: msg,
            },
            ServiceError::Conflict(msg) => ApiError {
                code: 409,
                message: msg,
            },
            ServiceError::ReadOnly(msg) => ApiError {
                code: 403,
                message: msg,
            },
            ServiceError::Storage(msg) => ApiError {
                code: 500,
                message: msg,
            },
            ServiceError::Internal(msg) => ApiError {
                code: 500,
                message: msg,
            },
        }
    }
}

/// Wrap a Result<T, ServiceError> into an API response.
pub(crate) fn ok_json<T: Serialize>(result: Result<T, ServiceError>) -> Result<Json<T>, ApiError> {
    result.map(Json).map_err(ApiError::from)
}
