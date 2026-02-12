// TODO: API layer will be generated from lib/api/schema/pms.api
pub mod model;
pub mod sn;
pub mod service;

use std::sync::Arc;

use axum::Router;
use openerp_core::Module;

use service::PmsService;

/// PMS Module — product manufacturing management.
pub struct PmsModule {
    service: Arc<PmsService>,
}

impl PmsModule {
    pub fn new(service: PmsService) -> Self {
        Self {
            service: Arc::new(service),
        }
    }
}

impl Module for PmsModule {
    fn name(&self) -> &str {
        "pms"
    }

    fn routes(&self) -> Router {
        // TODO: Will be replaced with generated API from lib/api/server/pms
        Router::new()
    }
}
