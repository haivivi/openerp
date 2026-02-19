//! KvStore implementations for Auth models.
//!
//! Defines KEY, kv_prefix, key_value, and hooks for each model.
//! Timestamps (created_at, updated_at) are managed by the store layer.

use openerp_store::KvStore;
use openerp_types::*;

use crate::model::*;

// ── Password helpers ──

/// Hash a plain password with argon2id.
pub fn hash_password(password: &str) -> Result<String, String> {
    use argon2::Argon2;
    use password_hash::{PasswordHasher, SaltString};
    use password_hash::rand_core::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

/// Verify a password against an argon2id hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::Argon2;
    use password_hash::{PasswordHash, PasswordVerifier};

    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

/// Find a user by email (scans all users).
pub fn find_user_by_email(
    kv: &std::sync::Arc<dyn openerp_kv::KVStore>,
    email: &str,
) -> Result<Option<User>, openerp_core::ServiceError> {
    use openerp_store::KvOps;
    let ops = KvOps::<User>::new(kv.clone());
    let all = ops.list()?;
    Ok(all.into_iter().find(|u| {
        u.email.as_ref().map(|e| e.as_str()) == Some(email)
    }))
}

/// Look up role IDs for a given user by scanning policies.
///
/// A Policy with `who` = user_id and `what` = "role" yields `how` as the role ID.
pub fn find_roles_for_user(
    kv: &std::sync::Arc<dyn openerp_kv::KVStore>,
    user_id: &str,
) -> Result<Vec<String>, openerp_core::ServiceError> {
    use openerp_store::KvOps;
    let ops = KvOps::<Policy>::new(kv.clone());
    let all = ops.list()?;
    let now = chrono::Utc::now();
    let roles = all
        .into_iter()
        .filter(|p| {
            p.who == user_id && p.what == "role" && {
                match &p.expires_at {
                    Some(exp) if !exp.is_empty() => {
                        chrono::DateTime::parse_from_rfc3339(exp.as_str())
                            .map(|d| d > now)
                            .unwrap_or(true)
                    }
                    _ => true,
                }
            }
        })
        .map(|p| p.how.clone())
        .collect();
    Ok(roles)
}

// ── User ──

impl KvStore for User {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "auth:user:" }
    fn key_value(&self) -> String { self.id.to_string() }
    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(uuid::Uuid::new_v4().to_string().replace('-', ""));
        }
    }
}

// ── Role ──

impl KvStore for Role {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "auth:role:" }
    fn key_value(&self) -> String { self.id.to_string() }
}

// ── Group ──

impl KvStore for Group {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "auth:group:" }
    fn key_value(&self) -> String { self.id.to_string() }
    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(uuid::Uuid::new_v4().to_string().replace('-', ""));
        }
    }
}

// ── Policy ──

impl KvStore for Policy {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "auth:policy:" }
    fn key_value(&self) -> String { self.id.to_string() }
    fn before_create(&mut self) {
        if self.id.is_empty() {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.who.hash(&mut hasher);
            self.what.hash(&mut hasher);
            self.how.hash(&mut hasher);
            self.id = Id::new(format!("{:016x}", hasher.finish()));
        }
    }
}

// ── Session ──

impl KvStore for Session {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "auth:session:" }
    fn key_value(&self) -> String { self.id.to_string() }
    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(uuid::Uuid::new_v4().to_string().replace('-', ""));
        }
    }
}

// ── Provider ──

impl KvStore for Provider {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "auth:provider:" }
    fn key_value(&self) -> String { self.id.to_string() }
}
