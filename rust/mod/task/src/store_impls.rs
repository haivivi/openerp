//! KvStore implementations for Task models.

use openerp_store::KvStore;
use openerp_types::*;
use crate::model::*;

impl KvStore for Task {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "task:task:" }
    fn key_value(&self) -> String { self.id.to_string() }
    fn before_create(&mut self) {
        if self.id.is_empty() {
            self.id = Id::new(uuid::Uuid::new_v4().to_string().replace('-', ""));
        }
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
        self.updated_at = DateTime::new(&now);
        // status defaults to Pending via DslEnum Default
    }
    fn before_update(&mut self) {
        self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
    }
}

impl KvStore for TaskType {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "task:task_type:" }
    fn key_value(&self) -> String { self.id.to_string() }
    fn before_create(&mut self) {
        let now = chrono::Utc::now().to_rfc3339();
        if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
        self.updated_at = DateTime::new(&now);
    }
    fn before_update(&mut self) {
        self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
    }
}
