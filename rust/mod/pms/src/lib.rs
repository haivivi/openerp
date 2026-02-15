//! PMS module v2 — built with the DSL framework.

#[path = "../dsl/model/mod.rs"]
pub mod model;
#[path = "../dsl/hierarchy/mod.rs"]
pub mod hierarchy_def;

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
    // Action routes
    router = router.merge(handlers::provision::routes(kv.clone()));
    router = router.merge(handlers::activate::routes(kv.clone()));
    router = router.merge(handlers::firmware_upload::routes(kv.clone()));
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
        hierarchy: hierarchy_def::hierarchy(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_kv(name: &str) -> Arc<dyn openerp_kv::KVStore> {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join(format!("{}.redb", name))).unwrap(),
        );
        // Leak tempdir to keep it alive.
        std::mem::forget(dir);
        kv
    }

    // ── Model CRUD ──

    #[test]
    fn model_kv_crud() {
        let kv = test_kv("model");
        let ops = KvOps::<Model>::new(kv);

        let m = Model {
            code: 1001,
            series_name: "H106".into(),
            display_name: Some("H106 Speaker".into()),
            description: None, metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };

        let created = ops.save_new(m).unwrap();
        assert_eq!(created.code, 1001);
        assert!(!created.created_at.is_empty());

        let fetched = ops.get_or_err("1001").unwrap();
        assert_eq!(fetched.series_name, "H106");

        ops.delete("1001").unwrap();
        assert!(ops.get("1001").unwrap().is_none());
    }

    // ── Device CRUD ──

    #[test]
    fn device_kv_crud() {
        let kv = test_kv("device");
        let ops = KvOps::<Device>::new(kv);

        let d = Device {
            sn: "SN001".into(),
            secret: openerp_types::Secret::new("sec123"),
            model: 42,
            status: "provisioned".into(),
            sku: Some("SKU-A".into()),
            imei: vec!["860000001".into()],
            licenses: vec![],
            display_name: Some("Test Device".into()),
            description: None, metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };

        let created = ops.save_new(d).unwrap();
        assert_eq!(created.sn, "SN001");

        let fetched = ops.get_or_err("SN001").unwrap();
        assert_eq!(fetched.model, 42);
        assert_eq!(fetched.secret.as_str(), "sec123");

        assert_eq!(ops.list().unwrap().len(), 1);
    }

    // ── Batch CRUD ──

    #[test]
    fn batch_kv_crud() {
        let kv = test_kv("batch");
        let ops = KvOps::<Batch>::new(kv);

        let b = Batch {
            id: openerp_types::Id::default(),
            model: 42, quantity: 100, provisioned_count: 0,
            status: "pending".into(),
            display_name: Some("Batch A".into()),
            description: None, metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };

        let created = ops.save_new(b).unwrap();
        assert!(!created.id.is_empty());
        assert_eq!(created.quantity, 100);
    }

    // ── Firmware CRUD ──

    #[test]
    fn firmware_kv_crud() {
        let kv = test_kv("firmware");
        let ops = KvOps::<Firmware>::new(kv);

        let f = Firmware {
            id: openerp_types::Id::default(),
            model: 42,
            semver: openerp_types::SemVer::new("1.2.3"),
            build: 456,
            status: "uploaded".into(),
            release_notes: Some("Bug fixes".into()),
            display_name: Some("v1.2.3".into()),
            description: None, metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };

        let created = ops.save_new(f).unwrap();
        assert!(!created.id.is_empty());
        assert_eq!(created.semver.as_str(), "1.2.3");
        assert_eq!(created.build, 456);
    }

    // ── License CRUD ──

    #[test]
    fn license_kv_crud() {
        let kv = test_kv("license");
        let ops = KvOps::<License>::new(kv);

        let l = License {
            id: openerp_types::Id::default(),
            license_type: "MIIT".into(),
            number: "MIIT-2026-001".into(),
            source: "manual".into(),
            sn: Some("SN001".into()),
            import_id: None,
            status: "active".into(),
            display_name: Some("MIIT License".into()),
            description: None, metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };

        let created = ops.save_new(l).unwrap();
        assert!(!created.id.is_empty());
        assert_eq!(created.license_type, "MIIT");
    }

    // ── LicenseImport CRUD ──

    #[test]
    fn license_import_kv_crud() {
        let kv = test_kv("licimport");
        let ops = KvOps::<LicenseImport>::new(kv);

        let li = LicenseImport {
            id: openerp_types::Id::default(),
            license_type: "WiFi".into(),
            source: "batch-import".into(),
            count: 500,
            allocated_count: 0,
            display_name: Some("WiFi Batch Import".into()),
            description: None, metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };

        let created = ops.save_new(li).unwrap();
        assert!(!created.id.is_empty());
        assert_eq!(created.count, 500);
    }

    // ── Segment CRUD ──

    #[test]
    fn segment_kv_crud() {
        let kv = test_kv("segment");
        let ops = KvOps::<Segment>::new(kv);

        let s = Segment {
            dimension: "manufacturer".into(),
            code: 1,
            display_name: Some("Haivivi".into()),
            description: None, metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };

        let created = ops.save_new(s).unwrap();
        assert_eq!(created.dimension, "manufacturer");
        assert_eq!(created.code, 1);

        // Compound key: manufacturer:1
        let fetched = ops.get_or_err("manufacturer:1").unwrap();
        assert_eq!(fetched.display_name, Some("Haivivi".into()));
    }

    // ── Schema ──

    #[test]
    fn pms_schema_has_all_resources() {
        let def = schema_def();
        assert_eq!(def.id, "pms");
        assert_eq!(def.resources.len(), 7); // Model, Device, Batch, Firmware, License, LicenseImport, Segment
    }

    // ── Provision action ──

    #[tokio::test]
    async fn provision_creates_devices() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("prov.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_router(kv.clone(), auth);

        // Create a batch.
        let batch_json = serde_json::json!({
            "model": 42, "quantity": 3, "provisionedCount": 0,
            "status": "pending", "displayName": "Test Batch",
        });
        let req = Request::builder()
            .method("POST").uri("/batches")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&batch_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let batch: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let batch_id = batch["id"].as_str().unwrap();

        // Provision.
        let prov_json = serde_json::json!({});
        let req = Request::builder()
            .method("POST").uri(format!("/batches/{}/@provision", batch_id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&prov_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let prov: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(prov["provisioned"], 3);
        assert_eq!(prov["devices"].as_array().unwrap().len(), 3);

        // Verify devices exist.
        let req = Request::builder().uri("/devices").body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let devices: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(devices["total"], 3);

        // Batch should be completed.
        let req = Request::builder().uri(format!("/batches/{}", batch_id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let updated_batch: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(updated_batch["status"], "completed");
        assert_eq!(updated_batch["provisionedCount"], 3);
    }

    // ── Partial provision ──

    #[tokio::test]
    async fn provision_partial_count() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("partial.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_router(kv, auth);

        // Create batch with quantity=5.
        let batch_json = serde_json::json!({
            "model": 77, "quantity": 5, "provisionedCount": 0,
            "status": "pending", "displayName": "Partial Batch",
        });
        let req = Request::builder()
            .method("POST").uri("/batches")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&batch_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let batch: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let batch_id = batch["id"].as_str().unwrap();

        // Provision only 2 of 5.
        let prov_json = serde_json::json!({"count": 2});
        let req = Request::builder()
            .method("POST").uri(format!("/batches/{}/@provision", batch_id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&prov_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let prov: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(prov["provisioned"], 2);
        assert_eq!(prov["devices"].as_array().unwrap().len(), 2);

        // Batch should be in_progress (not completed).
        let req = Request::builder().uri(format!("/batches/{}", batch_id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let b: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(b["status"], "in_progress");
        assert_eq!(b["provisionedCount"], 2);

        // Provision remaining 3.
        let prov_json = serde_json::json!({});
        let req = Request::builder()
            .method("POST").uri(format!("/batches/{}/@provision", batch_id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&prov_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let prov: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(prov["provisioned"], 3);

        // Batch should be completed now.
        let req = Request::builder().uri(format!("/batches/{}", batch_id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let b: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(b["status"], "completed");
        assert_eq!(b["provisionedCount"], 5);
    }

    // ── Activate action ──

    #[tokio::test]
    async fn activate_device() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("act.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_router(kv, auth);

        // Create a device.
        let dev_json = serde_json::json!({
            "sn": "ACT-001", "model": 42, "status": "provisioned",
            "displayName": "Activate Test",
        });
        let req = Request::builder()
            .method("POST").uri("/devices")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&dev_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Activate.
        let req = Request::builder()
            .method("POST").uri("/devices/ACT-001/@activate")
            .header("content-type", "application/json")
            .body(Body::from("{}")).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let act: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(act["status"], "active");

        // Double activate → error.
        let req = Request::builder()
            .method("POST").uri("/devices/ACT-001/@activate")
            .header("content-type", "application/json")
            .body(Body::from("{}")).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_ne!(resp.status(), StatusCode::OK, "double activate should fail");
    }

    // ── Firmware upload action ──

    #[tokio::test]
    async fn firmware_upload() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("fwup.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_router(kv, auth);

        let fw_json = serde_json::json!({
            "model": 42, "semver": "2.0.0", "build": 100,
            "releaseNotes": "Major update",
        });
        let req = Request::builder()
            .method("POST").uri("/firmwares/@upload")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&fw_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let fw: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(fw["status"], "uploaded");
        assert_eq!(fw["semver"], "2.0.0");
        assert!(!fw["id"].as_str().unwrap().is_empty());
    }
}
