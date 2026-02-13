//! Policy-based permission checking.
//!
//! Implements the Authenticator trait using the Policy + Role models.
//! This is the bridge between the DSL framework's Authenticator trait
//! and the Auth module's policy storage.

use std::sync::Arc;

use axum::http::HeaderMap;
use openerp_core::{Authenticator, ServiceError};
use openerp_store::KvOps;

use crate::model::{Policy, Role};

/// Auth-based Authenticator implementation.
///
/// Checks permissions by:
/// 1. Extracting user identity from JWT in headers (done by middleware)
/// 2. Looking up policies for the user
/// 3. Checking if any policy grants a role with the required permission
pub struct AuthChecker {
    pub role_ops: KvOps<Role>,
    pub policy_ops: KvOps<Policy>,
    pub root_role: String,
}

impl AuthChecker {
    pub fn new(
        kv: Arc<dyn openerp_kv::KVStore>,
        root_role: &str,
    ) -> Self {
        Self {
            role_ops: KvOps::new(kv.clone()),
            policy_ops: KvOps::new(kv),
            root_role: root_role.to_string(),
        }
    }
}

impl Authenticator for AuthChecker {
    fn check(&self, headers: &HeaderMap, permission: &str) -> Result<(), ServiceError> {
        // Extract roles from the request.
        // The JWT middleware should have validated the token and placed
        // claims info in a header or extension. For now we check a custom header.
        let roles_header = headers
            .get("x-openerp-roles")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if roles_header.is_empty() {
            return Err(ServiceError::Validation("authentication required".into()));
        }

        let user_roles: Vec<&str> = roles_header.split(',').map(|s| s.trim()).collect();

        // Root role bypasses all checks.
        if user_roles.contains(&self.root_role.as_str()) {
            return Ok(());
        }

        // Check if any role contains the required permission.
        for role_id in &user_roles {
            if let Ok(Some(role)) = self.role_ops.get(role_id) {
                if role.permissions.iter().any(|p| p == permission) {
                    return Ok(());
                }
            }
        }

        Err(ServiceError::Validation(format!(
            "permission denied: requires '{}'",
            permission
        )))
    }
}
