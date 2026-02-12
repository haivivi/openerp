use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};

use openerp_core::{ListParams, ServiceError};

use crate::model::{CreatePolicy, PolicyQuery};
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/policies", get(list_policies).post(create_policy).delete(delete_policy_by_query))
        .route("/policies/{id}", get(get_policy).delete(delete_policy))
        .route("/policies/@query", get(query_policies))
}

async fn list_policies(
    State(svc): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let result = svc.list_policies(&params).map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({
        "items": result.items,
        "total": result.total,
    })))
}

async fn create_policy(
    State(svc): State<AppState>,
    Json(input): Json<CreatePolicy>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ServiceError> {
    let policy = svc.create_policy(input).map_err(ServiceError::from)?;
    Ok((axum::http::StatusCode::CREATED, Json(serde_json::to_value(policy).unwrap())))
}

async fn get_policy(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let policy = svc.get_policy(&id).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(policy).unwrap()))
}

async fn delete_policy(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ServiceError> {
    svc.delete_policy(&id).map_err(ServiceError::from)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Query policies with optional who/what/how filters.
async fn query_policies(
    State(svc): State<AppState>,
    Query(query): Query<PolicyQuery>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let policies = svc.query_policies(&query).map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({"items": policies})))
}

/// Delete a policy by (who, what, how) query params.
/// DELETE /auth/policies?who=...&what=...&how=...
async fn delete_policy_by_query(
    State(svc): State<AppState>,
    Query(query): Query<PolicyDeleteQuery>,
) -> Result<axum::http::StatusCode, ServiceError> {
    svc.delete_policy_by_triple(&query.who, &query.what, &query.how)
        .map_err(ServiceError::from)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct PolicyDeleteQuery {
    who: String,
    #[serde(default)]
    what: String,
    how: String,
}
