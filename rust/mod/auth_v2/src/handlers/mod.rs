//! Hand-written Auth handlers — business logic beyond CRUD.
//!
//! These are called by the server binary (openerpd) for operations
//! that require custom logic:
//!
//! - **Login**: password verification, JWT issuance (in openerpd/login.rs)
//! - **Bootstrap**: root role creation (in openerpd/bootstrap.rs)
//! - **Policy check**: permission resolution (future)
//! - **OAuth**: provider callback flow (future)
//!
//! The DSL module (auth_v2) provides:
//! - Model definitions (User, Role, Group, Policy, Session, Provider)
//! - KvStore CRUD operations via KvOps<T>
//! - Admin router (/admin/auth/*)
//! - Schema definition for /meta/schema
//!
//! Custom handlers that need direct model access can use KvOps:
//!
//! ```ignore
//! use auth_v2::model::User;
//! use oe_store::KvOps;
//!
//! let ops = KvOps::<User>::new(kv.clone());
//! let user = ops.get_or_err("user-id")?;
//! ```

pub mod policy_check;
