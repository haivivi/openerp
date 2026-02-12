use axum::{
    Router,
    extract::{Path, Query, State},
    routing::get,
    Json,
};
use serde::Deserialize;

use openerp_core::ListParams;
use crate::model::Device;
use crate::service::device::DeviceFilters;
use super::{AppState, ApiError, ok_json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/devices", get(list_devices))
        .route("/devices/{sn}", get(get_device))
        .route("/devices/search", get(search_devices))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceQuery {
    #[serde(flatten)]
    params: ListParams,
    model: Option<u32>,
    batch_id: Option<String>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

fn default_search_limit() -> usize {
    20
}

async fn get_device(
    State(svc): State<AppState>,
    Path(sn): Path<String>,
) -> Result<Json<Device>, ApiError> {
    ok_json(svc.get_device(&sn))
}

async fn list_devices(
    State(svc): State<AppState>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<openerp_core::ListResult<Device>>, ApiError> {
    let filters = DeviceFilters {
        model: q.model,
        batch_id: q.batch_id,
        status: q.status,
    };
    ok_json(svc.list_devices(&q.params, &filters))
}

async fn search_devices(
    State(svc): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<Device>>, ApiError> {
    ok_json(svc.search_devices(&q.q, q.limit))
}
