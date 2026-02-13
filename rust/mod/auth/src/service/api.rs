//! Axum HTTP handlers for Auth CRUD operations.
//!
//! These are thin wrappers around AuthService methods, translating
//! axum types (Path, Query, Json) to service method calls.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use openerp_core::{ListParams, ServiceError};

use crate::model::*;
use crate::service::AuthService;

// ── Users ────────────────────────────────────────────────────

pub async fn create_user(
    State(svc): State<Arc<AuthService>>,
    axum::Json(body): axum::Json<CreateUser>,
) -> impl IntoResponse {
    match svc.create_user(body) {
        Ok(user) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&user).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn get_user(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.get_user(&id) {
        Ok(user) => (StatusCode::OK, axum::Json(serde_json::to_value(&user).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn list_users(
    State(svc): State<Arc<AuthService>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    match svc.list_users(&params) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn update_user(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
    axum::Json(patch): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    match svc.update_user(&id, patch) {
        Ok(user) => (StatusCode::OK, axum::Json(serde_json::to_value(&user).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn delete_user(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.delete_user(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

// ── Sessions ─────────────────────────────────────────────────

pub async fn get_session(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.get_session(&id) {
        Ok(session) => (StatusCode::OK, axum::Json(serde_json::to_value(&session).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn list_sessions(
    State(svc): State<Arc<AuthService>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    // List sessions for a specific user (from q param) or return empty.
    // TODO: Enhance with proper admin list-all-sessions capability.
    let user_id = params.q.as_deref().unwrap_or("");
    if user_id.is_empty() {
        return (StatusCode::OK, axum::Json(serde_json::json!({ "items": [], "total": 0 }))).into_response();
    }
    match svc.list_user_sessions(user_id) {
        Ok(sessions) => (StatusCode::OK, axum::Json(serde_json::to_value(&sessions).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn delete_session(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.revoke_session(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

// ── Roles ────────────────────────────────────────────────────

pub async fn create_role(
    State(svc): State<Arc<AuthService>>,
    axum::Json(body): axum::Json<CreateRole>,
) -> impl IntoResponse {
    match svc.create_role(body) {
        Ok(role) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&role).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn get_role(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.get_role(&id) {
        Ok(role) => (StatusCode::OK, axum::Json(serde_json::to_value(&role).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn list_roles(
    State(svc): State<Arc<AuthService>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    match svc.list_roles(&params) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn update_role(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
    axum::Json(patch): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    match svc.update_role(&id, patch) {
        Ok(role) => (StatusCode::OK, axum::Json(serde_json::to_value(&role).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn delete_role(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.delete_role(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

// ── Groups ───────────────────────────────────────────────────

pub async fn create_group(
    State(svc): State<Arc<AuthService>>,
    axum::Json(body): axum::Json<CreateGroup>,
) -> impl IntoResponse {
    match svc.create_group(body) {
        Ok(group) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&group).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn get_group(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.get_group(&id) {
        Ok(group) => (StatusCode::OK, axum::Json(serde_json::to_value(&group).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn list_groups(
    State(svc): State<Arc<AuthService>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    match svc.list_groups(&params) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn update_group(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
    axum::Json(patch): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    match svc.update_group(&id, patch) {
        Ok(group) => (StatusCode::OK, axum::Json(serde_json::to_value(&group).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn delete_group(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.delete_group(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

// ── Policies ─────────────────────────────────────────────────

pub async fn create_policy(
    State(svc): State<Arc<AuthService>>,
    axum::Json(body): axum::Json<CreatePolicy>,
) -> impl IntoResponse {
    match svc.create_policy(body) {
        Ok(policy) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&policy).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn get_policy(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.get_policy(&id) {
        Ok(policy) => (StatusCode::OK, axum::Json(serde_json::to_value(&policy).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn list_policies(
    State(svc): State<Arc<AuthService>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    match svc.list_policies(&params) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn update_policy(
    State(_svc): State<Arc<AuthService>>,
    Path(_id): Path<String>,
    axum::Json(_patch): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    // Policies are identified by (who, what, how) triple and upserted via create_policy.
    // Direct PATCH by ID is not supported — delete and recreate instead.
    ServiceError::Validation("policy update by ID is not supported; delete and recreate".into()).into_response()
}

pub async fn delete_policy(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.delete_policy(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

// ── Providers ────────────────────────────────────────────────

pub async fn create_provider(
    State(svc): State<Arc<AuthService>>,
    axum::Json(body): axum::Json<CreateProvider>,
) -> impl IntoResponse {
    match svc.create_provider(body) {
        Ok(provider) => (StatusCode::CREATED, axum::Json(serde_json::to_value(&provider).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn get_provider(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.get_provider(&id) {
        Ok(provider) => (StatusCode::OK, axum::Json(serde_json::to_value(&provider).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn list_providers(
    State(svc): State<Arc<AuthService>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    match svc.list_providers(&params) {
        Ok(result) => (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn update_provider(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
    axum::Json(patch): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    match svc.update_provider(&id, patch) {
        Ok(provider) => (StatusCode::OK, axum::Json(serde_json::to_value(&provider).unwrap())).into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}

pub async fn delete_provider(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.delete_provider(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ServiceError::from(e).into_response(),
    }
}
