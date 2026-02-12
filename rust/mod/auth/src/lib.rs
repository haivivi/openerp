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
pub mod api;

use std::sync::Arc;

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
        sql: Box<dyn openerp_sql::SQLStore>,
        kv: Box<dyn openerp_kv::KVStore>,
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
        api::build_router(self.service.clone())
    }
}
