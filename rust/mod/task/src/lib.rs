//! Task module v2 — built with the DSL framework.

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
    router = router.merge(admin_kv_router(KvOps::<Task>::new(kv.clone()), auth.clone(), "task", "tasks", "task"));
    router = router.merge(admin_kv_router(KvOps::<TaskType>::new(kv.clone()), auth.clone(), "task", "task-types", "task_type"));
    // Action routes
    router = router.merge(handlers::actions::routes(kv.clone()));
    router
}

/// Facet routers for Task. Empty for now.
pub fn facet_routers(_kv: Arc<dyn openerp_kv::KVStore>) -> Vec<openerp_store::FacetDef> {
    vec![]
}

pub fn schema_def() -> openerp_store::ModuleDef {
    use openerp_store::EnumDef;
    openerp_store::ModuleDef {
        id: "task",
        label: "Tasks",
        icon: "pulse",
        resources: vec![
            ResourceDef::from_ir("task", Task::__dsl_ir()).with_desc("Async task instances")
                .with_action("task", "claim")
                .with_action("task", "progress")
                .with_action("task", "complete")
                .with_action("task", "fail")
                .with_action("task", "cancel"),
            ResourceDef::from_ir("task", TaskType::__dsl_ir()).with_desc("Task type definitions"),
        ],
        enums: vec![
            EnumDef { name: "TaskStatus", variants: TaskStatus::variants() },
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
        std::mem::forget(dir);
        kv
    }

    // ── Task CRUD ──

    #[test]
    fn task_kv_crud() {
        let kv = test_kv("task");
        let ops = KvOps::<Task>::new(kv);

        let task = Task {
            id: openerp_types::Id::default(),
            task_type: "export".into(),
            total: 100, success: 0, failed: 0,
            status: TaskStatus::default(),
            message: None, error: None,
            claimed_by: None,
            last_active_at: None, created_by: Some("admin".into()),
            started_at: None, ended_at: None,
            timeout_secs: 300, retry_count: 0, max_retries: 3,
            display_name: Some("Export Job".into()),
            description: Some("Export all users".into()),
            metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };

        let created = ops.save_new(task).unwrap();
        assert!(!created.id.is_empty());
        assert_eq!(created.status, TaskStatus::Pending);
        assert!(!created.created_at.is_empty());

        let fetched = ops.get_or_err(created.id.as_str()).unwrap();
        assert_eq!(fetched.task_type, "export");
        assert_eq!(fetched.total, 100);

        // Update.
        let mut t = fetched;
        t.success = 50;
        ops.save(t).unwrap();
        let updated = ops.get_or_err(created.id.as_str()).unwrap();
        assert_eq!(updated.success, 50);

        // Delete.
        ops.delete(created.id.as_str()).unwrap();
        assert!(ops.get(created.id.as_str()).unwrap().is_none());
    }

    // ── TaskType CRUD ──

    #[test]
    fn task_type_kv_crud() {
        let kv = test_kv("tasktype");
        let ops = KvOps::<TaskType>::new(kv);

        let tt = TaskType {
            id: openerp_types::Id::new("firmware-upload"),
            service: "pms".into(),
            default_timeout: 600,
            max_concurrency: 2,
            display_name: Some("Firmware Upload".into()),
            description: Some("Upload firmware to device fleet".into()),
            metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };

        let created = ops.save_new(tt).unwrap();
        assert_eq!(created.id.as_str(), "firmware-upload");
        assert!(!created.created_at.is_empty());

        let all = ops.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].service, "pms");
        assert_eq!(all[0].max_concurrency, 2);
    }

    // ── Schema ──

    #[test]
    fn task_schema_has_all_resources() {
        let def = schema_def();
        assert_eq!(def.id, "task");
        assert_eq!(def.resources.len(), 2); // Task, TaskType
    }

    // ── Task lifecycle: claim → progress → complete ──

    #[tokio::test]
    async fn task_lifecycle_happy_path() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("life.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_router(kv, auth);

        // Create a task.
        let task_json = serde_json::json!({
            "taskType": "export", "total": 10,
            "status": "PENDING", "timeoutSecs": 60,
            "maxRetries": 0, "displayName": "Lifecycle Test",
        });
        let req = Request::builder()
            .method("POST").uri("/tasks")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&task_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let task: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let task_id = task["id"].as_str().unwrap();

        // Claim.
        let claim_json = serde_json::json!({"workerId": "worker-1"});
        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@claim", task_id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&claim_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let claimed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(claimed["status"], "RUNNING");

        // Progress.
        let prog_json = serde_json::json!({"success": 5, "message": "50%"});
        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@progress", task_id))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&prog_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Complete.
        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@complete", task_id))
            .header("content-type", "application/json")
            .body(Body::from("{}")).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let completed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(completed["status"], "COMPLETED");

        // Verify final state.
        let req = Request::builder()
            .uri(format!("/tasks/{}", task_id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let final_task: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(final_task["status"], "COMPLETED");
        assert_eq!(final_task["success"], 5);
    }

    // ── Task lifecycle: claim → fail → auto-retry ──

    #[tokio::test]
    async fn task_fail_with_retry() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("retry.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_router(kv, auth);

        // Create task with max_retries=2.
        let task_json = serde_json::json!({
            "taskType": "risky", "total": 1,
            "status": "PENDING", "timeoutSecs": 60,
            "maxRetries": 2, "displayName": "Retry Test",
        });
        let req = Request::builder()
            .method("POST").uri("/tasks")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&task_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let task: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let task_id = task["id"].as_str().unwrap();

        // Claim.
        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@claim", task_id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"workerId":"w1"}"#)).unwrap();
        router.clone().oneshot(req).await.unwrap();

        // Fail — should auto-retry (retry_count 0 < max_retries 2).
        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@fail", task_id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"error":"timeout"}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let failed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(failed["status"], "PENDING", "should be back to pending for retry");

        // Verify retry_count incremented.
        let req = Request::builder()
            .uri(format!("/tasks/{}", task_id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let t: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(t["retryCount"], 1);
    }

    // ── Task fail: retries exhausted → final failed ──

    #[tokio::test]
    async fn task_retries_exhausted_stays_failed() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("exhaust.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_router(kv, auth);

        // Create task with max_retries=1.
        let task_json = serde_json::json!({
            "taskType": "fragile", "total": 1,
            "status": "PENDING", "timeoutSecs": 60,
            "maxRetries": 1, "displayName": "Exhaust Test",
        });
        let req = Request::builder()
            .method("POST").uri("/tasks")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&task_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let task: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let task_id = task["id"].as_str().unwrap();

        // Claim + fail → retry_count=1, back to pending (retry_count < max_retries).
        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@claim", task_id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"workerId":"w1"}"#)).unwrap();
        router.clone().oneshot(req).await.unwrap();

        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@fail", task_id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"error":"attempt 1"}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let r: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(r["status"], "PENDING", "First fail: should retry");

        // Claim again + fail → retry_count=2 > max_retries=1, stays failed.
        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@claim", task_id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"workerId":"w2"}"#)).unwrap();
        router.clone().oneshot(req).await.unwrap();

        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@fail", task_id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"error":"attempt 2 final"}"#)).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let r: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(r["status"], "FAILED", "Second fail: retries exhausted, should stay failed");

        // Verify final state.
        let req = Request::builder()
            .uri(format!("/tasks/{}", task_id)).body(Body::empty()).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let t: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(t["status"], "FAILED");
        // retry_count=1: one successful retry happened (first fail), second fail exhausted retries.
        assert_eq!(t["retryCount"], 1);
        assert!(t["endedAt"].as_str().is_some(), "endedAt should be set on final failure");
    }

    // ── Task cancel ──

    #[tokio::test]
    async fn task_cancel() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("cancel.redb")).unwrap(),
        );
        let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);
        let router = admin_router(kv, auth);

        // Create task.
        let task_json = serde_json::json!({
            "taskType": "cancel-me", "total": 1,
            "status": "PENDING", "timeoutSecs": 60,
            "displayName": "Cancel Test",
        });
        let req = Request::builder()
            .method("POST").uri("/tasks")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&task_json).unwrap())).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let task: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let task_id = task["id"].as_str().unwrap();

        // Cancel pending task.
        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@cancel", task_id))
            .header("content-type", "application/json")
            .body(Body::from("{}")).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let cancelled: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(cancelled["status"], "CANCELLED");

        // Can't cancel again.
        let req = Request::builder()
            .method("POST").uri(format!("/tasks/{}/@cancel", task_id))
            .header("content-type", "application/json")
            .body(Body::from("{}")).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_ne!(resp.status(), StatusCode::OK, "double cancel should fail");
    }
}
