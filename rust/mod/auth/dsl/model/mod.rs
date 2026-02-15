//! Auth model definitions.
//!
//! Model = DB: all fields including hidden ones (password_hash, client_secret).
//! Admin API (/admin/) exposes the model directly.
//! Custom facets expose subsets.

pub mod user;
pub mod role;
pub mod group;
pub mod policy;
pub mod session;
pub mod provider;

pub use user::User;
pub use role::Role;
pub use group::Group;
pub use policy::Policy;
pub use session::Session;
pub use provider::Provider;
