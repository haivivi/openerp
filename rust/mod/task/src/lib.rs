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
    router
}

/// Facet routers for Task. Empty for now.
pub fn facet_routers(_kv: Arc<dyn openerp_kv::KVStore>) -> Vec<openerp_store::FacetDef> {
    vec![]
}

pub fn schema_def() -> openerp_store::ModuleDef {
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
        hierarchy: hierarchy_def::hierarchy(),
    }
}
