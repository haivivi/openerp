//! "app" facet handlers — hand-written API for mobile app.

mod device;

use std::sync::Arc;
use axum::Router;
use openerp_store::KvOps;
use crate::model::Device;

pub fn router(kv: Arc<dyn openerp_kv::KVStore>) -> Router {
    let ops = Arc::new(KvOps::<Device>::new(kv));
    device::routes(ops)
}
