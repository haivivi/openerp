//! Batch provision handler — generates Device records for a batch.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use openerp_core::ServiceError;
use openerp_store::KvOps;
use openerp_types::*;

use crate::model::{Batch, Device};

pub struct ProvisionState {
    pub batch_ops: KvOps<Batch>,
    pub device_ops: KvOps<Device>,
}

pub fn routes(kv: Arc<dyn openerp_kv::KVStore>) -> Router {
    let state = Arc::new(ProvisionState {
        batch_ops: KvOps::<Batch>::new(kv.clone()),
        device_ops: KvOps::<Device>::new(kv),
    });
    Router::new()
        .route("/batches/{id}/@provision", post(provision))
        .with_state(state)
}

#[derive(serde::Deserialize)]
pub struct ProvisionRequest {
    /// Number of devices to provision (defaults to batch.quantity - batch.provisioned_count).
    pub count: Option<u32>,
}

#[derive(serde::Serialize)]
pub struct ProvisionResponse {
    pub batch_id: String,
    pub provisioned: u32,
    pub devices: Vec<String>,
}

async fn provision(
    State(state): State<Arc<ProvisionState>>,
    Path(batch_id): Path<String>,
    Json(req): Json<ProvisionRequest>,
) -> Result<Json<ProvisionResponse>, ServiceError> {
    let mut batch = state.batch_ops.get_or_err(&batch_id)?;

    let remaining = batch.quantity.saturating_sub(batch.provisioned_count);
    let count = req.count.unwrap_or(remaining).min(remaining);

    if count == 0 {
        return Err(ServiceError::Validation(
            "batch is fully provisioned".into(),
        ));
    }

    let mut device_sns = Vec::with_capacity(count as usize);

    for _ in 0..count {
        let sn = format!(
            "{:04X}{:08X}",
            batch.model,
            batch.provisioned_count + 1
        );
        let secret_val = uuid::Uuid::new_v4().to_string().replace('-', "");

        let device = Device {
            sn: sn.clone(),
            secret: Secret::new(&secret_val),
            model: batch.model,
            status: "provisioned".into(),
            sku: None,
            imei: vec![],
            licenses: vec![],
            display_name: Some(format!("Device {}", sn)),
            description: None,
            metadata: None,
            created_at: DateTime::default(),
            updated_at: DateTime::default(),
        };

        state.device_ops.save_new(device)?;
        device_sns.push(sn);
        batch.provisioned_count += 1;
    }

    // Update batch status.
    if batch.provisioned_count >= batch.quantity {
        batch.status = "completed".into();
    } else {
        batch.status = "in_progress".into();
    }
    state.batch_ops.save(batch)?;

    Ok(Json(ProvisionResponse {
        batch_id,
        provisioned: count,
        devices: device_sns,
    }))
}
