//! Flux — cross-platform state engine.
//!
//! A path-based state machine with pub/sub for driving multi-platform apps.
//! Rust owns all state and logic; each platform (iOS/Android/Web/Desktop)
//! only renders the UI.
//!
//! # Three Primitives
//!
//! - `get(path)` — read state at a path, Arc zero-copy
//! - `emit(path, payload)` — send a request, Trie-routed to handler(s)
//! - `subscribe(pattern)` — observe state changes, Trie-matched notifications
//!
//! # Path Addressing
//!
//! All state and requests live in a flat path namespace with `/` as separator:
//! - Global: `auth/state`, `app/route`
//! - Page: `home/devices`, `chat/messages`
//! - Items: `home/devices/items/{id}`
//! - Nested: `chat/conversations/items/{conv_id}/messages/items/{msg_id}`
//!
//! # Trie Pattern Matching
//!
//! Both subscriptions and request handlers use MQTT-style wildcards:
//! - Exact: `auth/state`
//! - Single-level: `auth/+` matches `auth/state`, `auth/terms`
//! - Multi-level: `home/#` matches everything under `home/`
//! - All: `#` matches everything
//!
//! # Example
//!
//! ```ignore
//! use flux::Flux;
//!
//! let app = Flux::new();
//!
//! // Register handlers.
//! app.on("app/initialize", |_, _, store| async move {
//!     store.set("auth/state", AuthState::unauthenticated());
//!     store.set("app/route", "/onboarding".to_string());
//! });
//!
//! // Subscribe to changes.
//! app.subscribe("#", |path, value| {
//!     println!("state changed: {}", path);
//! });
//!
//! // Emit requests.
//! app.emit("app/initialize", ()).await;
//! ```

pub mod app;
pub mod i18n;
pub mod router;
pub mod store;
pub mod trie;
pub mod value;

// Re-export primary types at crate root.
pub use app::Flux;
pub use i18n::{I18nHandler, I18nStore, QueryParams};
pub use router::{BoxFuture, Router};
pub use store::{ChangeHandler, StateStore};
pub use value::{StateValue, SubscriptionId};
