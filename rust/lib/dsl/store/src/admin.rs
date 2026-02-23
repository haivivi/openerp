//! Admin router — auto-generates `/admin/{module}/{resource}` CRUD routes.
//!
//! The admin API uses the model directly (no facet struct projection).
//! Permissions follow `{module}:{resource}:{action}` format.
//! Authentication is delegated to the `Authenticator` trait.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use openerp_core::{Authenticator, CountResult, ListParams, ListResult, ServiceError};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::kv::{KvOps, KvStore};
use crate::sql::{SqlOps, SqlStore};
use openerp_types::DslModel;

/// Shared state for admin route handlers.
struct AdminState<T: KvStore + DslModel> {
    ops: KvOps<T>,
    auth: Arc<dyn Authenticator>,
    module: String,
    resource: String,
}

/// Build an Axum router for admin CRUD on a KvStore model.
///
/// Routes:
///   GET    /{resources}         — list (paginated)
///   POST   /{resources}         — create
///   GET    /{resources}/@count  — count (optional)
///   GET    /{resources}/{id}    — get by key
///   PUT    /{resources}/{id}    — full update (with updatedAt check)
///   PATCH  /{resources}/{id}    — partial update (RFC 7386 merge patch)
///   DELETE /{resources}/{id}    — delete
///
/// - `resource_path`: URL segment (e.g. "users", "roles")
/// - `resource_name`: permission resource name (e.g. "user", "role")
pub fn admin_kv_router<T: KvStore + DslModel + Serialize + DeserializeOwned>(
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
    let count_path = format!("/{}/@count", resource_path);
    let item_path = format!("/{}/{{id}}", resource_path);

    Router::new()
        .route(&list_path, get(list_handler::<T>).post(create_handler::<T>))
        .route(&count_path, get(count_handler::<T>))
        .route(
            &item_path,
            get(get_handler::<T>)
                .put(update_handler::<T>)
                .patch(patch_handler::<T>)
                .delete(delete_handler::<T>),
        )
        .with_state(state)
}

fn perm(module: &str, resource: &str, action: &str) -> String {
    format!("{}:{}:{}", module, resource, action)
}

async fn list_handler<T: KvStore + DslModel + Serialize>(
    State(state): State<Arc<AdminState<T>>>,
    headers: HeaderMap,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let p = perm(&state.module, &state.resource, "list");
    state.auth.check(&headers, &p)?;

    let result = state.ops.list_paginated(&params)?;
    // Get hidden fields from IR metadata
    let ir = T::__dsl_ir();
    let hidden_fields = get_hidden_fields(&ir);
    // Filter hidden fields from each record
    let filtered_items: Vec<serde_json::Value> = result
        .items
        .iter()
        .map(|item| secure_serialize(item, &hidden_fields))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ServiceError::Internal(format!("Serialization error: {}", e)))?;

    // Build response with correct camelCase naming for hasMore
    let filtered_result = serde_json::json!({
        "items": filtered_items,
        "hasMore": result.has_more,
    });
    Ok(Json(filtered_result))
}

async fn count_handler<T: KvStore + DslModel + Serialize>(
    State(state): State<Arc<AdminState<T>>>,
    headers: HeaderMap,
) -> Result<Json<CountResult>, ServiceError> {
    let p = perm(&state.module, &state.resource, "list");
    state.auth.check(&headers, &p)?;

    let count = state.ops.count()?;
    Ok(Json(CountResult { count }))
}

async fn get_handler<T: KvStore + DslModel + Serialize>(
    State(state): State<Arc<AdminState<T>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let p = perm(&state.module, &state.resource, "read");
    state.auth.check(&headers, &p)?;

    let record = state.ops.get_or_err(&id)?;
    let ir = T::__dsl_ir();
    let hidden_fields = get_hidden_fields(&ir);
    let filtered = secure_serialize(&record, &hidden_fields)
        .map_err(|e| ServiceError::Internal(format!("Serialization error: {}", e)))?;
    Ok(Json(filtered))
}

async fn create_handler<T: KvStore + DslModel + Serialize + DeserializeOwned>(
    State(state): State<Arc<AdminState<T>>>,
    headers: HeaderMap,
    Json(record): Json<T>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let p = perm(&state.module, &state.resource, "create");
    state.auth.check(&headers, &p)?;

    let created = state.ops.save_new(record)?;
    let ir = T::__dsl_ir();
    let hidden_fields = get_hidden_fields(&ir);
    let filtered = secure_serialize(&created, &hidden_fields)
        .map_err(|e| ServiceError::Internal(format!("Serialization error: {}", e)))?;
    Ok(Json(filtered))
}

async fn update_handler<T: KvStore + DslModel + Serialize + DeserializeOwned>(
    State(state): State<Arc<AdminState<T>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(record): Json<T>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let p = perm(&state.module, &state.resource, "update");
    state.auth.check(&headers, &p)?;

    let body_key = record.key_value();
    if body_key != id {
        return Err(ServiceError::Validation(format!(
            "URL key '{}' does not match body key '{}'", id, body_key
        )));
    }
    let existing = state.ops.get_or_err(&id)?;
    let ir = T::__dsl_ir();
    let hidden_fields = get_hidden_fields(&ir);
    // Merge hidden fields: preserve existing values if client sent null
    let merged = merge_hidden_fields(&record, &existing, &hidden_fields)
        .map_err(|e| ServiceError::Internal(format!("Merge error: {}", e)))?;
    let updated = state.ops.save(merged)?;
    let filtered = secure_serialize(&updated, &hidden_fields)
        .map_err(|e| ServiceError::Internal(format!("Serialization error: {}", e)))?;
    Ok(Json(filtered))
}

async fn patch_handler<T: KvStore + DslModel + Serialize + DeserializeOwned>(
    State(state): State<Arc<AdminState<T>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let p = perm(&state.module, &state.resource, "update");
    state.auth.check(&headers, &p)?;

    let patched = state.ops.patch(&id, &patch)?;
    let ir = T::__dsl_ir();
    let hidden_fields = get_hidden_fields(&ir);
    let filtered = secure_serialize(&patched, &hidden_fields)
        .map_err(|e| ServiceError::Internal(format!("Serialization error: {}", e)))?;
    Ok(Json(filtered))
}

async fn delete_handler<T: KvStore + DslModel + Serialize>(
    State(state): State<Arc<AdminState<T>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<(), ServiceError> {
    let p = perm(&state.module, &state.resource, "delete");
    state.auth.check(&headers, &p)?;

    state.ops.delete(&id)?;
    Ok(())
}

// ── SQL admin router ──

struct SqlAdminState<T: SqlStore + DslModel> {
    ops: SqlOps<T>,
    auth: Arc<dyn Authenticator>,
    module: String,
    resource: String,
}

/// Build an Axum router for admin CRUD on a SqlStore model.
///
/// Symmetric with `admin_kv_router`. Key differences:
/// - Supports compound primary keys via `SqlStore::PK`
/// - Single PK: `/{resources}/{id}` (same as KV)
/// - Compound PK: `/{resources}/*pk` (segments split by `/`)
///
/// Routes:
///   GET    /{resources}              — list (paginated)
///   POST   /{resources}              — create
///   GET    /{resources}/@count       — count
///   GET    /{resources}/{pk...}      — get by PK
///   PUT    /{resources}/{pk...}      — full update (with updatedAt check)
///   PATCH  /{resources}/{pk...}      — partial update (RFC 7386 merge patch)
///   DELETE /{resources}/{pk...}      — delete
pub fn admin_sql_router<T: SqlStore + DslModel + Serialize + DeserializeOwned>(
    ops: SqlOps<T>,
    auth: Arc<dyn Authenticator>,
    module: &str,
    resource_path: &str,
    resource_name: &str,
) -> Router {
    let state = Arc::new(SqlAdminState {
        ops,
        auth,
        module: module.to_string(),
        resource: resource_name.to_string(),
    });

    let list_path = format!("/{}", resource_path);
    let count_path = format!("/{}/@count", resource_path);

    let item_path = if T::PK.len() <= 1 {
        format!("/{}/{{id}}", resource_path)
    } else {
        format!("/{}/{{*pk}}", resource_path)
    };

    Router::new()
        .route(
            &list_path,
            get(sql_list_handler::<T>).post(sql_create_handler::<T>),
        )
        .route(&count_path, get(sql_count_handler::<T>))
        .route(
            &item_path,
            get(sql_get_handler::<T>)
                .put(sql_update_handler::<T>)
                .patch(sql_patch_handler::<T>)
                .delete(sql_delete_handler::<T>),
        )
        .with_state(state)
}

fn parse_pk_path(pk_path: &str, expected: usize) -> Result<Vec<String>, ServiceError> {
    let parts: Vec<String> = pk_path.split('/').map(|s| s.to_string()).collect();
    if parts.len() != expected {
        return Err(ServiceError::Validation(format!(
            "expected {} PK segments, got {}",
            expected,
            parts.len()
        )));
    }
    Ok(parts)
}

async fn sql_list_handler<T: SqlStore + DslModel + Serialize>(
    State(state): State<Arc<SqlAdminState<T>>>,
    headers: HeaderMap,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let p = perm(&state.module, &state.resource, "list");
    state.auth.check(&headers, &p)?;

    let result = state.ops.list_paginated(&params)?;
    // Get hidden fields from IR metadata
    let ir = T::__dsl_ir();
    let hidden_fields = get_hidden_fields(&ir);
    // Filter hidden fields from each record
    let filtered_items: Vec<serde_json::Value> = result
        .items
        .iter()
        .map(|item| secure_serialize(item, &hidden_fields))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ServiceError::Internal(format!("Serialization error: {}", e)))?;

    // Build response with correct camelCase naming for hasMore
    let filtered_result = serde_json::json!({
        "items": filtered_items,
        "hasMore": result.has_more,
    });
    Ok(Json(filtered_result))
}

async fn sql_count_handler<T: SqlStore + DslModel + Serialize>(
    State(state): State<Arc<SqlAdminState<T>>>,
    headers: HeaderMap,
) -> Result<Json<CountResult>, ServiceError> {
    let p = perm(&state.module, &state.resource, "list");
    state.auth.check(&headers, &p)?;

    let count = state.ops.count()?;
    Ok(Json(CountResult { count }))
}

async fn sql_get_handler<T: SqlStore + DslModel + Serialize>(
    State(state): State<Arc<SqlAdminState<T>>>,
    Path(pk_path): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let p = perm(&state.module, &state.resource, "read");
    state.auth.check(&headers, &p)?;

    let pks = parse_pk_path(&pk_path, T::PK.len())?;
    let pk_refs: Vec<&str> = pks.iter().map(|s| s.as_str()).collect();
    let record = state.ops.get_or_err(&pk_refs)?;
    let ir = T::__dsl_ir();
    let hidden_fields = get_hidden_fields(&ir);
    let filtered = secure_serialize(&record, &hidden_fields)
        .map_err(|e| ServiceError::Internal(format!("Serialization error: {}", e)))?;
    Ok(Json(filtered))
}

async fn sql_create_handler<T: SqlStore + DslModel + Serialize + DeserializeOwned>(
    State(state): State<Arc<SqlAdminState<T>>>,
    headers: HeaderMap,
    Json(record): Json<T>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let p = perm(&state.module, &state.resource, "create");
    state.auth.check(&headers, &p)?;

    let created = state.ops.save_new(record)?;
    let ir = T::__dsl_ir();
    let hidden_fields = get_hidden_fields(&ir);
    let filtered = secure_serialize(&created, &hidden_fields)
        .map_err(|e| ServiceError::Internal(format!("Serialization error: {}", e)))?;
    Ok(Json(filtered))
}

async fn sql_update_handler<T: SqlStore + DslModel + Serialize + DeserializeOwned>(
    State(state): State<Arc<SqlAdminState<T>>>,
    Path(pk_path): Path<String>,
    headers: HeaderMap,
    Json(record): Json<T>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let p = perm(&state.module, &state.resource, "update");
    state.auth.check(&headers, &p)?;

    let pks = parse_pk_path(&pk_path, T::PK.len())?;
    let body_pks = record.pk_values();
    if pks != body_pks {
        return Err(ServiceError::Validation(format!(
            "URL PK {:?} does not match body PK {:?}", pks, body_pks
        )));
    }
    let pk_refs: Vec<&str> = pks.iter().map(|s| s.as_str()).collect();
    let existing = state.ops.get_or_err(&pk_refs)?;
    let ir = T::__dsl_ir();
    let hidden_fields = get_hidden_fields(&ir);
    // Merge hidden fields: preserve existing values if client sent null
    let merged = merge_hidden_fields(&record, &existing, &hidden_fields)
        .map_err(|e| ServiceError::Internal(format!("Merge error: {}", e)))?;
    let updated = state.ops.save(merged)?;
    let filtered = secure_serialize(&updated, &hidden_fields)
        .map_err(|e| ServiceError::Internal(format!("Serialization error: {}", e)))?;
    Ok(Json(filtered))
}

async fn sql_patch_handler<T: SqlStore + DslModel + Serialize + DeserializeOwned>(
    State(state): State<Arc<SqlAdminState<T>>>,
    Path(pk_path): Path<String>,
    headers: HeaderMap,
    Json(patch): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let p = perm(&state.module, &state.resource, "update");
    state.auth.check(&headers, &p)?;

    let pks = parse_pk_path(&pk_path, T::PK.len())?;
    let pk_refs: Vec<&str> = pks.iter().map(|s| s.as_str()).collect();
    let patched = state.ops.patch(&pk_refs, &patch)?;
    let ir = T::__dsl_ir();
    let hidden_fields = get_hidden_fields(&ir);
    let filtered = secure_serialize(&patched, &hidden_fields)
        .map_err(|e| ServiceError::Internal(format!("Serialization error: {}", e)))?;
    Ok(Json(filtered))
}

async fn sql_delete_handler<T: SqlStore + DslModel + Serialize>(
    State(state): State<Arc<SqlAdminState<T>>>,
    Path(pk_path): Path<String>,
    headers: HeaderMap,
) -> Result<(), ServiceError> {
    let p = perm(&state.module, &state.resource, "delete");
    state.auth.check(&headers, &p)?;

    let pks = parse_pk_path(&pk_path, T::PK.len())?;
    let pk_refs: Vec<&str> = pks.iter().map(|s| s.as_str()).collect();
    state.ops.delete(&pk_refs)?;
    Ok(())
}

/// Securely serialize a record, masking hidden field values with `null`.
///
/// Uses the model's IR metadata to determine which fields should be hidden.
/// Hidden fields are preserved in the response (so clients know they exist)
/// but their values are masked as `null` for security.
fn secure_serialize<T: Serialize>(
    record: &T,
    hidden_fields: &[String],
) -> Result<serde_json::Value, serde_json::Error> {
    let value = serde_json::to_value(record)?;

    if let Some(obj) = value.as_object() {
        let masked: serde_json::Map<String, serde_json::Value> = obj
            .iter()
            .map(|(k, v)| {
                if hidden_fields.contains(k) {
                    (k.clone(), serde_json::Value::Null)
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect();
        Ok(serde_json::Value::Object(masked))
    } else {
        Ok(value)
    }
}

/// Extract field names with widget="hidden" from model IR.
/// Returns camelCase field names to match serde serialization.
fn get_hidden_fields(ir: &serde_json::Value) -> Vec<String> {
    ir["fields"]
        .as_array()
        .map(|fields| {
            fields
                .iter()
                .filter(|f| f["widget"].as_str() == Some("hidden"))
                .filter_map(|f| f["name"].as_str().map(|s| to_camel_case(s)))
                .collect()
        })
        .unwrap_or_default()
}

/// Convert snake_case field name to camelCase.
fn to_camel_case(s: &str) -> String {
    let parts: Vec<&str> = s.split('_').collect();
    if parts.is_empty() {
        return s.to_string();
    }
    let mut result = parts[0].to_string();
    for part in &parts[1..] {
        if !part.is_empty() {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_ascii_uppercase());
                result.extend(chars);
            }
        }
    }
    result
}

/// Merge hidden fields from existing record into new record.
/// For PUT updates: preserves hidden field values that client didn't provide (sent as null).
fn merge_hidden_fields<T: Serialize + DeserializeOwned>(
    new_record: &T,
    existing_record: &T,
    hidden_fields: &[String],
) -> Result<T, serde_json::Error> {
    let mut new_json = serde_json::to_value(new_record)?;
    let existing_json = serde_json::to_value(existing_record)?;

    if let (Some(new_obj), Some(existing_obj)) = (new_json.as_object_mut(), existing_json.as_object()) {
        for field in hidden_fields {
            // If new record has null for hidden field, preserve existing value
            if new_obj.get(field).map(|v| v.is_null()).unwrap_or(true) {
                if let Some(existing_value) = existing_obj.get(field) {
                    new_obj.insert(field.clone(), existing_value.clone());
                }
            }
        }
    }

    serde_json::from_value(new_json)
}
