//! Twitter model definitions.
//!
//! Each model is a single file with `#[model]` — the macro generates
//! serde, Field consts, IR metadata, and common fields.

pub mod user;
pub mod tweet;
pub mod like;
pub mod follow;

pub use user::User;
pub use tweet::Tweet;
pub use like::Like;
pub use follow::Follow;
