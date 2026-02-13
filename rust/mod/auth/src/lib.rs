//! Auth module — federation authentication + hierarchical groups + policy ACL.
//!
//! # Resources
//!
//! - **User** — identity with linked OAuth accounts
//! - **Group** — hierarchical organization unit (tree + external sync)
//! - **Provider** — OAuth provider configuration (Feishu/GitHub/Google/custom)
//! - **Role** — named permission set registered by services
//! - **Policy** — ACL quad (who, what?, how, time?)
//! - **Session** — JWT issuance record
//!
//! # Usage
//!
//! ```ignore
//! use auth::{AuthModule, service::AuthConfig};
//!
//! let module = AuthModule::new(sql, kv, AuthConfig::default())?;
//! let router = module.routes(); // Mount under /auth
//! ```

pub mod model;
pub mod service;
pub mod handlers;

use std::sync::Arc;

use axum::routing::{delete, get, post, patch};
use axum::Router;

use openerp_core::Module;

use crate::service::{AuthConfig, AuthService};

/// Auth module implementing the Module trait.
///
/// Holds the AuthService and provides HTTP routes for all auth endpoints.
pub struct AuthModule {
    service: Arc<AuthService>,
}

impl AuthModule {
    /// Create a new AuthModule.
    pub fn new(
        sql: Arc<dyn openerp_sql::SQLStore>,
        kv: Arc<dyn openerp_kv::KVStore>,
        config: AuthConfig,
    ) -> Result<Self, openerp_core::ServiceError> {
        let service = AuthService::new(sql, kv, config)
            .map_err(openerp_core::ServiceError::from)?;
        Ok(Self { service })
    }

    /// Get a reference to the underlying AuthService.
    pub fn service(&self) -> &Arc<AuthService> {
        &self.service
    }
}

impl Module for AuthModule {
    fn name(&self) -> &str {
        "auth"
    }

    fn routes(&self) -> Router {
        let svc = self.service.clone();

        Router::new()
            // User CRUD
            .route("/users", get(service::api::list_users).post(service::api::create_user))
            .route("/users/{id}", get(service::api::get_user).patch(service::api::update_user).delete(service::api::delete_user))
            // User custom: me
            .route("/me", get(handlers::user::me))
            // Session CRUD
            .route("/sessions", get(service::api::list_sessions))
            .route("/sessions/{id}", get(service::api::get_session).delete(service::api::delete_session))
            .route("/sessions/{id}/@revoke", post(handlers::session::revoke))
            // Role CRUD
            .route("/roles", get(service::api::list_roles).post(service::api::create_role))
            .route("/roles/{id}", get(service::api::get_role).patch(service::api::update_role).delete(service::api::delete_role))
            // Group CRUD + member management
            .route("/groups", get(service::api::list_groups).post(service::api::create_group))
            .route("/groups/{id}", get(service::api::get_group).patch(service::api::update_group).delete(service::api::delete_group))
            .route("/groups/{id}/@members", get(handlers::group::list_members).post(handlers::group::add_member))
            .route("/groups/{id}/@members/{member_ref}", delete(handlers::group::remove_member))
            // Policy CRUD + check
            .route("/policies", get(service::api::list_policies).post(service::api::create_policy))
            .route("/policies/{id}", get(service::api::get_policy).patch(service::api::update_policy).delete(service::api::delete_policy))
            .route("/check", post(handlers::policy::check))
            // Provider CRUD
            .route("/providers", get(service::api::list_providers).post(service::api::create_provider))
            .route("/providers/{id}", get(service::api::get_provider).patch(service::api::update_provider).delete(service::api::delete_provider))
            .with_state(svc)
    }
}
