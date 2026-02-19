//! KvStore implementations for PMS models.
//! Timestamps (created_at, updated_at) are managed by the store layer.

use openerp_store::KvStore;
use openerp_types::*;
use crate::model::*;

macro_rules! impl_kv_basic {
    ($ty:ident, $prefix:expr, $key_field:ident, $gen_id:expr) => {
        impl KvStore for $ty {
            const KEY: Field = Self::$key_field;
            fn kv_prefix() -> &'static str { $prefix }
            fn key_value(&self) -> String { self.$key_field.to_string() }
            fn before_create(&mut self) {
                if $gen_id {
                    let id_str = self.$key_field.to_string();
                    if id_str.is_empty() {
                        self.$key_field = uuid::Uuid::new_v4().to_string().replace('-', "").into();
                    }
                }
            }
        }
    };
}

impl_kv_basic!(Batch, "pms:batch:", id, true);
impl_kv_basic!(Firmware, "pms:firmware:", id, true);
impl_kv_basic!(License, "pms:license:", id, true);
impl_kv_basic!(LicenseImport, "pms:license_import:", id, true);

impl KvStore for Model {
    const KEY: Field = Self::code;
    fn kv_prefix() -> &'static str { "pms:model:" }
    fn key_value(&self) -> String { self.code.to_string() }
}

impl KvStore for Device {
    const KEY: Field = Self::sn;
    fn kv_prefix() -> &'static str { "pms:device:" }
    fn key_value(&self) -> String { self.sn.clone() }
}

impl KvStore for Segment {
    const KEY: Field = Self::dimension;
    fn kv_prefix() -> &'static str { "pms:segment:" }
    fn key_value(&self) -> String { format!("{}:{}", self.dimension, self.code) }
}
