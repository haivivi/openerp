use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};

use openerp_core::{ListParams, ServiceError};

use crate::model::CreateRole;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/roles", get(list_roles).post(create_role))
        .route("/roles/{id}", get(get_role).put(update_role).delete(delete_role))
}

async fn list_roles(
    State(svc): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let result = svc.list_roles(&params).map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({
        "items": result.items,
        "total": result.total,
    })))
}

async fn create_role(
    State(svc): State<AppState>,
    Json(input): Json<CreateRole>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ServiceError> {
    let role = svc.create_role(input).map_err(ServiceError::from)?;
    Ok((axum::http::StatusCode::CREATED, Json(serde_json::to_value(role).unwrap())))
}

async fn get_role(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let role = svc.get_role(&id).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(role).unwrap()))
}

async fn update_role(
    State(svc): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let role = svc.update_role(&id, patch).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(role).unwrap()))
}

async fn delete_role(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ServiceError> {
    svc.delete_role(&id).map_err(ServiceError::from)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
