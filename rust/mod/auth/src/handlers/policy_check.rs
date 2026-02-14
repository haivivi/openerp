//! Policy-based permission checking — implements Authenticator trait.

use std::sync::Arc;

use axum::http::HeaderMap;
use openerp_core::{Authenticator, ServiceError};

/// JWT-based Authenticator.
///
/// Verifies JWT from Authorization header, extracts roles,
/// checks if any role grants the required permission.
pub struct AuthChecker {
    jwt_secret: String,
    root_role: String,
    kv: Arc<dyn openerp_kv::KVStore>,
}

impl AuthChecker {
    pub fn new(
        kv: Arc<dyn openerp_kv::KVStore>,
        jwt_secret: &str,
        root_role: &str,
    ) -> Self {
        Self {
            jwt_secret: jwt_secret.to_string(),
            root_role: root_role.to_string(),
            kv,
        }
    }

    fn extract_roles(&self, headers: &HeaderMap) -> Result<Vec<String>, ServiceError> {
        // Get JWT from Authorization: Bearer <token>
        let token = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| ServiceError::Validation("missing authorization token".into()))?;

        // Decode JWT.
        let key = jsonwebtoken::DecodingKey::from_secret(self.jwt_secret.as_bytes());
        let validation = jsonwebtoken::Validation::default();
        let data = jsonwebtoken::decode::<Claims>(token, &key, &validation)
            .map_err(|e| ServiceError::Validation(format!("invalid token: {}", e)))?;

        Ok(data.claims.roles)
    }
}

#[derive(Debug, serde::Deserialize)]
struct Claims {
    #[serde(default)]
    roles: Vec<String>,
}

impl Authenticator for AuthChecker {
    fn check(&self, headers: &HeaderMap, permission: &str) -> Result<(), ServiceError> {
        let roles = self.extract_roles(headers)?;

        // Root role bypasses all checks.
        if roles.iter().any(|r| r == &self.root_role) {
            return Ok(());
        }

        // Check if any role contains the required permission.
        use openerp_store::KvOps;
        use crate::model::Role;
        let role_ops = KvOps::<Role>::new(self.kv.clone());

        for role_id in &roles {
            if let Ok(Some(role)) = role_ops.get(role_id) {
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
