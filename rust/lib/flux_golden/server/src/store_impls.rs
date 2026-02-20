//! KvStore implementations for Twitter models.
//!
//! Defines KEY, kv_prefix, key_value, and hooks for each model.

use openerp_store::KvStore;
use openerp_types::*;

use crate::server::model::*;

// ── User ──

impl KvStore for User {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "twitter:user:" }
    fn key_value(&self) -> String { self.id.to_string() }

    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(&self.username);
        }
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
        self.updated_at = DateTime::new(&now);
    }

    fn before_update(&mut self) {
        self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
    }
}

// ── Tweet ──

impl KvStore for Tweet {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "twitter:tweet:" }
    fn key_value(&self) -> String { self.id.to_string() }

    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
        }
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
        self.updated_at = DateTime::new(&now);
    }

    fn before_update(&mut self) {
        self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
    }
}

// ── Like ──

impl KvStore for Like {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "twitter:like:" }
    fn key_value(&self) -> String { self.id.to_string() }

    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(&format!("{}:{}", self.user.resource_id(), self.tweet.resource_id()));
        }
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
        self.updated_at = DateTime::new(&now);
    }
}

// ── Message ──

impl KvStore for Message {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "twitter:message:" }
    fn key_value(&self) -> String { self.id.to_string() }

    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
        }
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
        self.updated_at = DateTime::new(&now);
    }

    fn before_update(&mut self) {
        self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
    }
}

// ── Follow ──

impl KvStore for Follow {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "twitter:follow:" }
    fn key_value(&self) -> String { self.id.to_string() }

    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(&format!("{}:{}", self.follower.resource_id(), self.followee.resource_id()));
        }
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
        self.updated_at = DateTime::new(&now);
    }
}
