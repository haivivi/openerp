//! PMS module v2 — built with the DSL framework.

#[path = "../dsl/model/mod.rs"]
pub mod model;

pub mod handlers;
mod store_impls;

use std::sync::Arc;
use axum::Router;
use openerp_store::{admin_kv_router, KvOps, ResourceDef};
use model::*;

pub fn admin_router(
    kv: Arc<dyn openerp_kv::KVStore>,
    auth: Arc<dyn openerp_core::Authenticator>,
) -> Router {
    let mut router = Router::new();
    router = router.merge(admin_kv_router(KvOps::<Model>::new(kv.clone()), auth.clone(), "pms", "models", "model"));
    router = router.merge(admin_kv_router(KvOps::<Device>::new(kv.clone()), auth.clone(), "pms", "devices", "device"));
    router = router.merge(admin_kv_router(KvOps::<Batch>::new(kv.clone()), auth.clone(), "pms", "batches", "batch"));
    router = router.merge(admin_kv_router(KvOps::<Firmware>::new(kv.clone()), auth.clone(), "pms", "firmwares", "firmware"));
    router = router.merge(admin_kv_router(KvOps::<License>::new(kv.clone()), auth.clone(), "pms", "licenses", "license"));
    router = router.merge(admin_kv_router(KvOps::<LicenseImport>::new(kv.clone()), auth.clone(), "pms", "license-imports", "license_import"));
    router = router.merge(admin_kv_router(KvOps::<Segment>::new(kv.clone()), auth.clone(), "pms", "segments", "segment"));
    router
}

/// Facet routers for PMS.
pub fn facet_routers(kv: Arc<dyn openerp_kv::KVStore>) -> Vec<openerp_store::FacetDef> {
    vec![
        openerp_store::FacetDef {
            name: "mfg",
            module: "pms",
            router: handlers::mfg::router(kv.clone()),
        },
    ]
}

pub fn schema_def() -> openerp_store::ModuleDef {
    openerp_store::ModuleDef {
        id: "pms",
        label: "Product Management",
        icon: "cube",
        resources: vec![
            ResourceDef::from_ir("pms", Model::__dsl_ir()).with_desc("Device model/series definitions"),
            ResourceDef::from_ir("pms", Device::__dsl_ir()).with_desc("Produced devices with SN"),
            ResourceDef::from_ir("pms", Batch::__dsl_ir()).with_desc("Production batches")
                .with_action("pms", "provision"),
            ResourceDef::from_ir("pms", Firmware::__dsl_ir()).with_desc("Firmware versions")
                .with_action("pms", "upload"),
            ResourceDef::from_ir("pms", License::__dsl_ir()).with_desc("Licenses (MIIT, WiFi, etc.)"),
            ResourceDef::from_ir("pms", LicenseImport::__dsl_ir()).with_desc("License import batches")
                .with_action("pms", "import"),
            ResourceDef::from_ir("pms", Segment::__dsl_ir()).with_desc("SN encoding segments"),
        ],
    }
}
