//! OpenERP DSL Parser
//!
//! Parses annotated Rust source (syn TokenStream) into IR data structures.
//! Used by:
//! - proc macro (at compile time)
//! - codegen binary (reads source files)

pub mod model_parser;
pub mod db_parser;
pub mod util;

pub use model_parser::parse_model;
pub use db_parser::parse_persistent;
