//! Bootstrap — first-start checks and auth:root role creation.
//!
//! When openerpd starts:
//! 1. Verify the config has a root password hash — if not, refuse to start.
//! 2. Ensure the `auth:root` role exists in the database.

use std::sync::Arc;

use oe_core::now_rfc3339;
use tracing::info;

use crate::config::ServerConfig;

/// The well-known role ID for the superadmin.
pub const ROOT_ROLE_ID: &str = "auth:root";

/// Verify server configuration is ready for production use.
pub fn verify_config(config: &ServerConfig) -> anyhow::Result<()> {
    if config.root.password_hash.is_empty() {
        anyhow::bail!(
            "No root password hash found in configuration.\n\
             Run `openerp context create <name>` to set up the server first."
        );
    }
    if config.jwt.secret.is_empty() {
        anyhow::bail!("JWT secret is empty in configuration.");
    }
    if config.storage.data_dir.is_empty() {
        anyhow::bail!("Storage data_dir is empty in configuration.");
    }
    Ok(())
}

/// Ensure the auth:root role exists. Creates it if missing.
pub fn ensure_root_role(
    kv: &Arc<dyn oe_kv::KVStore>,
) -> anyhow::Result<()> {
    let key = format!("auth/roles/{}", ROOT_ROLE_ID);

    match kv.get(&key) {
        Ok(Some(_)) => {
            info!("auth:root role already exists");
            Ok(())
        }
        Ok(None) | Err(_) => {
            let role = serde_json::json!({
                "id": ROOT_ROLE_ID,
                "description": "Superadmin — bypasses all permission checks",
                "permissions": [],
                "service": "auth",
                "created_at": now_rfc3339(),
                "updated_at": now_rfc3339(),
            });
            let data = serde_json::to_vec(&role)?;
            kv.set(&key, &data).map_err(|e| anyhow::anyhow!("failed to create auth:root role: {}", e))?;
            info!("Created auth:root role");
            Ok(())
        }
    }
}

/// Verify a root login attempt against the stored argon2id hash.
pub fn verify_root_password(password: &str, hash: &str) -> bool {
    use argon2::Argon2;
    use password_hash::PasswordHash;
    use password_hash::PasswordVerifier;

    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_config_empty_hash() {
        let config = ServerConfig {
            root: crate::config::RootConfig {
                password_hash: String::new(),
            },
            storage: crate::config::StorageConfig {
                data_dir: "/tmp".to_string(),
            },
            jwt: crate::config::JwtConfig {
                secret: "test".to_string(),
                expire_secs: 3600,
            },
        };
        assert!(verify_config(&config).is_err());
    }

    #[test]
    fn test_verify_root_password_invalid_hash() {
        assert!(!verify_root_password("test", "not-a-hash"));
    }
}
