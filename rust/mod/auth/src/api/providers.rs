use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};

use openerp_core::{ListParams, ServiceError};

use crate::model::CreateProvider;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/providers", get(list_providers).post(create_provider))
        .route("/providers/{id}", get(get_provider).put(update_provider).delete(delete_provider))
}

async fn list_providers(
    State(svc): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let result = svc.list_providers(&params).map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({
        "items": result.items,
        "total": result.total,
    })))
}

async fn create_provider(
    State(svc): State<AppState>,
    Json(input): Json<CreateProvider>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ServiceError> {
    let provider = svc.create_provider(input).map_err(ServiceError::from)?;
    // Return public view (no secret)
    let public: crate::model::ProviderPublic = provider.into();
    Ok((axum::http::StatusCode::CREATED, Json(serde_json::to_value(public).unwrap())))
}

async fn get_provider(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let public = svc.get_provider_public(&id).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(public).unwrap()))
}

async fn update_provider(
    State(svc): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let public = svc.update_provider(&id, patch).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(public).unwrap()))
}

async fn delete_provider(
    State(svc): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ServiceError> {
    svc.delete_provider(&id).map_err(ServiceError::from)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
