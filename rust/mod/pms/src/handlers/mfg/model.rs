use std::sync::Arc;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use openerp_core::{ListParams, ListResult, ServiceError};
use openerp_store::KvOps;
use crate::model::Model;
use crate::mfg::MfgModel;

type S = Arc<KvOps<Model>>;

pub fn routes(ops: S) -> Router {
    Router::new()
        .route("/models", get(list))
        .with_state(ops)
}

fn project(m: &Model) -> MfgModel {
    MfgModel {
        code: m.code,
        series_name: m.series_name.clone(),
        display_name: m.display_name.clone(),
    }
}

async fn list(
    State(ops): State<S>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResult<MfgModel>>, ServiceError> {
    let result = ops.list_paginated(&params)?;
    let items = result.items.iter().map(project).collect();
    Ok(Json(ListResult { items, has_more: result.has_more }))
}
