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
pub mod facet;
pub mod format;
pub mod hierarchy;
pub mod kv;
pub mod sql;
pub mod search;
pub mod schema;
mod timestamp;
pub mod ui;
pub mod ui_macro;

pub use kv::{KvStore, KvOps};
pub use sql::{SqlStore, SqlOps};
pub use search::{SearchStore, SearchOps};
pub use admin::admin_kv_router;
pub use facet::FacetDef;
pub use format::{negotiate_format, FacetResponse, FacetListResponse};
pub use hierarchy::HierarchyNode;
pub use schema::{build_schema, EnumDef, ModuleDef, ResourceDef};
pub use ui::{WidgetOverride, apply_overrides};
