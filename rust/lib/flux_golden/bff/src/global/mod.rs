//! Handler implementations for global requests.
//!
//! Each handler is an `impl` method on a context struct.
//! In Phase 2, `#[flux_handlers]` macro generates the router
//! registration (path → handler) from the `#[request]` annotations.

pub mod auth_handlers;
pub mod tweet_handlers;
pub mod user_handlers;
pub mod app_handlers;
pub mod helpers;
