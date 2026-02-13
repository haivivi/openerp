//! OpenERP Store traits.
//!
//! Store traits define how models are persisted. A model implements
//! the trait to declare storage config (KEY, UNIQUE, INDEX) and hooks.
//! CRUD operations are provided by the framework.
//!
//! ```ignore
//! impl KvStore for User {
//!     const KEY: Field = Self::id;
//!     fn before_create(&mut self) { self.id = Id::new_uuid(); }
//! }
//! ```

pub mod admin;
pub mod kv;
pub mod sql;
pub mod search;
pub mod schema;

pub use kv::{KvStore, KvOps};
pub use sql::{SqlStore, SqlOps};
pub use search::{SearchStore, SearchOps};
pub use admin::admin_kv_router;
pub use schema::{build_schema, ModuleDef, ResourceDef};
