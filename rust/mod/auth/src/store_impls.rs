//! KvStore implementations for Auth models.
//!
//! Defines KEY, kv_prefix, key_value, and hooks for each model.

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

// ── User ──

impl KvStore for User {
    const KEY: Field = Self::id;

    fn kv_prefix() -> &'static str {
        "auth:user:"
    }

    fn key_value(&self) -> String {
        self.id.to_string()
    }

    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(uuid::Uuid::new_v4().to_string().replace('-', ""));
        }
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() {
            self.created_at = DateTime::new(&now);
        }
        self.updated_at = DateTime::new(&now);
    }

    fn before_update(&mut self) {
        self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
    }
}

// ── Role ──

impl KvStore for Role {
    const KEY: Field = Self::id;

    fn kv_prefix() -> &'static str {
        "auth:role:"
    }

    fn key_value(&self) -> String {
        self.id.to_string()
    }

    fn before_create(&mut self) {
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() {
            self.created_at = DateTime::new(&now);
        }
        self.updated_at = DateTime::new(&now);
    }

    fn before_update(&mut self) {
        self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
    }
}

// ── Group ──

impl KvStore for Group {
    const KEY: Field = Self::id;

    fn kv_prefix() -> &'static str {
        "auth:group:"
    }

    fn key_value(&self) -> String {
        self.id.to_string()
    }

    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(uuid::Uuid::new_v4().to_string().replace('-', ""));
        }
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() {
            self.created_at = DateTime::new(&now);
        }
        self.updated_at = DateTime::new(&now);
    }

    fn before_update(&mut self) {
        self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
    }
}

// ── Policy ──

impl KvStore for Policy {
    const KEY: Field = Self::id;

    fn kv_prefix() -> &'static str {
        "auth:policy:"
    }

    fn key_value(&self) -> String {
        self.id.to_string()
    }

    fn before_create(&mut self) {
        if self.id.is_empty() {
            // Policy ID is deterministic: hash(who:what:how)
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.who.hash(&mut hasher);
            self.what.hash(&mut hasher);
            self.how.hash(&mut hasher);
            self.id = Id::new(format!("{:016x}", hasher.finish()));
        }
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() {
            self.created_at = DateTime::new(&now);
        }
        self.updated_at = DateTime::new(&now);
    }

    fn before_update(&mut self) {
        self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
    }
}

// ── Session ──

impl KvStore for Session {
    const KEY: Field = Self::id;

    fn kv_prefix() -> &'static str {
        "auth:session:"
    }

    fn key_value(&self) -> String {
        self.id.to_string()
    }

    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(uuid::Uuid::new_v4().to_string().replace('-', ""));
        }
    }
}

// ── Provider ──

impl KvStore for Provider {
    const KEY: Field = Self::id;

    fn kv_prefix() -> &'static str {
        "auth:provider:"
    }

    fn key_value(&self) -> String {
        self.id.to_string()
    }

    fn before_create(&mut self) {
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() {
            self.created_at = DateTime::new(&now);
        }
        self.updated_at = DateTime::new(&now);
    }

    fn before_update(&mut self) {
        self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
    }
}
