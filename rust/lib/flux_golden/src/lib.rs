//! Flux Golden Test — Twitter-like app.
//!
//! Structure:
//! - `server/` — DSL-defined backend (models + KvStore + auto-CRUD)
//! - `bff/dsl/state/` — BFF state definitions (golden ref for #[state] macro)
//! - `bff/dsl/request/` — BFF request definitions (golden ref for #[request] macro)
//! - `bff/src/` — handler implementations (golden ref for #[flux_handlers] macro)

// Backend.
#[path = "../server/src/mod.rs"]
pub mod server;

// BFF state types — flat access as `crate::state::*`.
#[path = "../bff/dsl/state/global/mod.rs"]
pub mod state;

// BFF request types — flat access as `crate::request::*`.
#[path = "../bff/dsl/request/global/mod.rs"]
pub mod request;

// BFF handler implementations + Flux wiring.
#[path = "../bff/src/mod.rs"]
pub mod handlers;
