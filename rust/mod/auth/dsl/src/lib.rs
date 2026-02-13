//! Auth module DSL definitions.
//!
//! This crate defines the Auth module using the OpenERP DSL:
//! - `model/` — data structures (what the API sees)
//! - `persistent/` — storage definitions (what the DB stores)
//! - `rest/` — API facets (which fields are exposed, to whom)

pub mod model;
pub mod persistent;
pub mod facet;

// Re-export stores for use by the module's lib.rs.
pub use persistent::{
    UserStore, RoleStore, GroupStore, PolicyStore, SessionStore, ProviderStore,
};

// Re-export facet routers.
pub use facet::{
    data_user_router, data_role_router, data_group_router,
    data_policy_router, data_session_router, data_provider_router,
};
