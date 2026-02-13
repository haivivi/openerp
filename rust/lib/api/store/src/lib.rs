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

pub mod kv;

pub use kv::{KvStore, KvOps};
