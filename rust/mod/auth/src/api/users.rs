use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};

use openerp_core::{ListParams, ServiceError};

use crate::model::CreateUser;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", get(get_user).put(update_user).delete(delete_user))
        .route("/users/{id}/groups", get(get_user_groups))
        .route("/users/{id}/policies", get(get_user_policies))
}

async fn list_users(
    State(svc): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let result = svc.list_users(&params).map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({
        "items": result.items,
        "total": result.total,
    })))
}

async fn create_user(
    State(svc): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ServiceError> {
    let user = svc.create_user(input).map_err(ServiceError::from)?;
    Ok((axum::http::StatusCode::CREATED, Json(serde_json::to_value(user).unwrap())))
}

async fn get_user(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let user = svc.get_user(&id).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}

async fn update_user(
    State(svc): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let user = svc.update_user(&id, patch).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}

async fn delete_user(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ServiceError> {
    svc.delete_user(&id).map_err(ServiceError::from)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn get_user_groups(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let groups = svc.get_user_direct_groups(&id).map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({"items": groups})))
}

async fn get_user_policies(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    use crate::model::PolicyQuery;
    let policies = svc.query_policies(&PolicyQuery {
        who: Some(format!("user:{}", id)),
        what: None,
        how: None,
    }).map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({"items": policies})))
}
