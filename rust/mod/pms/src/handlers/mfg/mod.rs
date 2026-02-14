//! "mfg" facet handlers — Manufacturing App API.

mod device;
mod batch;
mod firmware;
mod model;

use std::sync::Arc;
use axum::Router;
use openerp_store::KvOps;
use crate::model::{Device, Batch, Firmware, Model};

pub fn router(kv: Arc<dyn openerp_kv::KVStore>) -> Router {
    Router::new()
        .merge(model::routes(Arc::new(KvOps::<Model>::new(kv.clone()))))
        .merge(device::routes(Arc::new(KvOps::<Device>::new(kv.clone()))))
        .merge(batch::routes(Arc::new(KvOps::<Batch>::new(kv.clone()))))
        .merge(firmware::routes(Arc::new(KvOps::<Firmware>::new(kv))))
}
