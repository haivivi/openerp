use std::sync::Arc;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use openerp_core::{ListResult, ServiceError};
use openerp_store::KvOps;
use crate::model::Model;

#[path = "../../../dsl/rest/mfg/model.rs"]
mod mfg_model_def;
use mfg_model_def::MfgModel;

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

async fn list(State(ops): State<S>) -> Result<Json<ListResult<MfgModel>>, ServiceError> {
    let all = ops.list()?;
    let items: Vec<MfgModel> = all.iter().map(project).collect();
    let total = items.len();
    Ok(Json(ListResult { items, total }))
}
