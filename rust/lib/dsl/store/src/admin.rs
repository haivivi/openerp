//! Admin router — auto-generates `/admin/{module}/{resource}` CRUD routes.
//!
//! The admin API uses the model directly (no facet struct projection).
//! Permissions follow `{module}:{resource}:{action}` format.
//! Authentication is delegated to the `Authenticator` trait.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use oe_core::{Authenticator, ListResult, ServiceError};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::kv::{KvOps, KvStore};

/// Shared state for admin route handlers.
struct AdminState<T: KvStore> {
    ops: KvOps<T>,
    auth: Arc<dyn Authenticator>,
    module: String,
    resource: String,
}

/// Build an Axum router for admin CRUD on a KvStore model.
///
/// Routes:
///   GET  /{resources}       — list all
///   POST /{resources}       — create
///   GET  /{resources}/{id}  — get by key
///   PUT  /{resources}/{id}  — update
///   DELETE /{resources}/{id} — delete
///
/// - `resource_path`: URL segment (e.g. "users", "roles")
/// - `resource_name`: permission resource name (e.g. "user", "role")
pub fn admin_kv_router<T: KvStore + Serialize + DeserializeOwned>(
    ops: KvOps<T>,
    auth: Arc<dyn Authenticator>,
    module: &str,
    resource_path: &str,
    resource_name: &str,
) -> Router {
    let state = Arc::new(AdminState {
        ops,
        auth,
        module: module.to_string(),
        resource: resource_name.to_string(),
    });

    let list_path = format!("/{}", resource_path);
    let item_path = format!("/{}/{{id}}", resource_path);

    Router::new()
        .route(&list_path, get(list_handler::<T>).post(create_handler::<T>))
        .route(
            &item_path,
            get(get_handler::<T>)
                .put(update_handler::<T>)
                .delete(delete_handler::<T>),
        )
        .with_state(state)
}

fn perm(module: &str, resource: &str, action: &str) -> String {
    format!("{}:{}:{}", module, resource, action)
}

async fn list_handler<T: KvStore + Serialize>(
    State(state): State<Arc<AdminState<T>>>,
    headers: HeaderMap,
) -> Result<Json<ListResult<T>>, ServiceError> {
    let p = perm(&state.module, &state.resource, "list");
    state.auth.check(&headers, &p)?;

    let items = state.ops.list()?;
    let total = items.len();
    Ok(Json(ListResult { items, total }))
}

async fn get_handler<T: KvStore + Serialize>(
    State(state): State<Arc<AdminState<T>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<T>, ServiceError> {
    let p = perm(&state.module, &state.resource, "read");
    state.auth.check(&headers, &p)?;

    let record = state.ops.get_or_err(&id)?;
    Ok(Json(record))
}

async fn create_handler<T: KvStore + Serialize + DeserializeOwned>(
    State(state): State<Arc<AdminState<T>>>,
    headers: HeaderMap,
    Json(record): Json<T>,
) -> Result<Json<T>, ServiceError> {
    let p = perm(&state.module, &state.resource, "create");
    state.auth.check(&headers, &p)?;

    let created = state.ops.save_new(record)?;
    Ok(Json(created))
}

async fn update_handler<T: KvStore + Serialize + DeserializeOwned>(
    State(state): State<Arc<AdminState<T>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(record): Json<T>,
) -> Result<Json<T>, ServiceError> {
    let p = perm(&state.module, &state.resource, "update");
    state.auth.check(&headers, &p)?;

    // Ensure it exists first.
    let _existing = state.ops.get_or_err(&id)?;
    let updated = state.ops.save(record)?;
    Ok(Json(updated))
}

async fn delete_handler<T: KvStore + Serialize>(
    State(state): State<Arc<AdminState<T>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<(), ServiceError> {
    let p = perm(&state.module, &state.resource, "delete");
    state.auth.check(&headers, &p)?;

    state.ops.delete(&id)?;
    Ok(())
}
