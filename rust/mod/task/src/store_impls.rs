//! KvStore implementations for Task models.
//! Timestamps (created_at, updated_at) are managed by the store layer.

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
    }
}

impl KvStore for TaskType {
    const KEY: Field = Self::id;
    fn kv_prefix() -> &'static str { "task:task_type:" }
    fn key_value(&self) -> String { self.id.to_string() }
}
