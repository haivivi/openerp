//! Firmware upload handler — creates a firmware record and marks it as uploaded.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use openerp_core::ServiceError;
use openerp_store::KvOps;
use openerp_types::*;

use crate::model::Firmware;

pub fn routes(kv: Arc<dyn openerp_kv::KVStore>) -> Router {
    let ops = Arc::new(KvOps::<Firmware>::new(kv));
    Router::new()
        .route("/firmwares/@upload", post(upload))
        .with_state(ops)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareUploadRequest {
    pub model: u32,
    pub semver: String,
    pub build: u64,
    pub release_notes: Option<String>,
}

#[derive(serde::Serialize)]
pub struct FirmwareUploadResponse {
    pub id: String,
    pub model: u32,
    pub semver: String,
    pub status: String,
}

async fn upload(
    State(ops): State<Arc<KvOps<Firmware>>>,
    Json(req): Json<FirmwareUploadRequest>,
) -> Result<Json<FirmwareUploadResponse>, ServiceError> {
    let firmware = Firmware {
        id: Id::default(),
        model: req.model,
        semver: SemVer::new(&req.semver),
        build: req.build,
        status: "uploaded".into(),
        release_notes: req.release_notes,
        display_name: Some(format!("v{}", req.semver)),
        description: None,
        metadata: None,
        created_at: DateTime::default(),
        updated_at: DateTime::default(),
        version: 0,
    };

    let created = ops.save_new(firmware)?;

    Ok(Json(FirmwareUploadResponse {
        id: created.id.to_string(),
        model: created.model,
        semver: created.semver.to_string(),
        status: created.status,
    }))
}
