pub mod model;
pub mod sn;
pub mod service;
pub mod handlers;

use std::sync::Arc;

use axum::routing::{get, post, delete};
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
        let svc = self.service.clone();

        Router::new()
            // Device (read-only — devices created through batch provisioning)
            .route("/devices", get(service::api::list_devices))
            .route("/devices/{sn}", get(service::api::get_device))
            .route("/devices/{sn}/@provision", post(handlers::device::provision))
            .route("/devices/{sn}/@activate", post(handlers::device::activate))
            // Batch CRUD + custom
            .route("/batches", get(service::api::list_batches).post(service::api::create_batch))
            .route("/batches/{id}", get(service::api::get_batch).delete(service::api::delete_batch))
            .route("/batches/{id}/@provision", post(handlers::batch::provision))
            // License (composite key: type + number)
            .route("/licenses", get(service::api::list_licenses))
            .route("/licenses/{license_type}/{number}", get(service::api::get_license).delete(service::api::delete_license))
            // Firmware (composite key: model + semver)
            .route("/firmwares", get(service::api::list_firmwares).post(service::api::create_firmware))
            .route("/firmwares/{model}/{semver}", get(service::api::get_firmware).delete(service::api::delete_firmware))
            .route("/firmwares/{model}/{semver}/@upload", post(handlers::firmware::upload))
            // Model CRUD
            .route("/models", get(service::api::list_models).post(service::api::create_model))
            .route("/models/{code}", get(service::api::get_model).delete(service::api::delete_model))
            // SN Segment
            .route("/segments", get(service::api::list_sn_segments).post(service::api::upsert_sn_segment))
            .route("/segments/{dimension}/{name}", delete(service::api::delete_sn_segment))
            // LicenseImport CRUD + custom
            .route("/license-imports", get(service::api::list_license_imports).post(service::api::create_license_import))
            .route("/license-imports/{id}", get(service::api::get_license_import).delete(service::api::delete_license_import))
            .route("/license-imports/{id}/@import", post(handlers::license_import::import_licenses))
            .with_state(svc)
    }
}
