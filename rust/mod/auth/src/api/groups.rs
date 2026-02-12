use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};

use openerp_core::{ListParams, ServiceError};

use crate::model::{AddGroupMember, CreateGroup};
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/{id}", get(get_group).put(update_group).delete(delete_group))
        .route("/groups/{id}/members", get(list_members).post(add_member))
        .route("/groups/{id}/members/{member_ref}", delete(remove_member))
        .route("/groups/{id}/@sync", post(sync_group))
}

async fn list_groups(
    State(svc): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let result = svc.list_groups(&params).map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({
        "items": result.items,
        "total": result.total,
    })))
}

async fn create_group(
    State(svc): State<AppState>,
    Json(input): Json<CreateGroup>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ServiceError> {
    let group = svc.create_group(input).map_err(ServiceError::from)?;
    Ok((axum::http::StatusCode::CREATED, Json(serde_json::to_value(group).unwrap())))
}

async fn get_group(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let group = svc.get_group(&id).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(group).unwrap()))
}

async fn update_group(
    State(svc): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let group = svc.update_group(&id, patch).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(group).unwrap()))
}

async fn delete_group(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ServiceError> {
    svc.delete_group(&id).map_err(ServiceError::from)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn list_members(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let members = svc.list_group_members(&id).map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({"items": members})))
}

async fn add_member(
    State(svc): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<AddGroupMember>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ServiceError> {
    let member = svc.add_group_member(&id, input).map_err(ServiceError::from)?;
    Ok((axum::http::StatusCode::CREATED, Json(serde_json::to_value(member).unwrap())))
}

async fn remove_member(
    State(svc): State<AppState>,
    Path((id, member_ref)): Path<(String, String)>,
) -> Result<axum::http::StatusCode, ServiceError> {
    svc.remove_group_member(&id, &member_ref).map_err(ServiceError::from)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Trigger external source sync for a group.
/// This is a placeholder — actual sync logic depends on provider integrations.
async fn sync_group(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let group = svc.get_group(&id).map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "group_id": group.id,
        "external_source": group.external_source,
        "message": "sync triggered",
    })))
}
