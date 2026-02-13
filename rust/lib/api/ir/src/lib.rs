//! OpenERP DSL Intermediate Representation (IR)
//!
//! Zero-dependency data structures shared between:
//! - proc macro (compile-time code generation)
//! - codegen binary (schema.json + TypeScript output)
//! - validator (compile-time checks)
//!
//! Five layers:
//! 1. Model    — data structures + method signatures
//! 2. DB       — persistent storage definitions
//! 3. Hierarchy — resource nesting (routes + parent-child)
//! 4. Facet    — REST API surfaces for different consumers
//! 5. Module   — aggregates all of the above

pub mod types;
pub mod model;
pub mod db;
pub mod hierarchy;
pub mod facet;
pub mod module;

pub use types::*;
pub use model::*;
pub use db::*;
pub use hierarchy::*;
pub use facet::*;
pub use module::*;
