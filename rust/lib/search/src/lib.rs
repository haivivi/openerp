pub mod error;
pub mod tantivy;
pub mod traits;

pub use error::SearchError;
pub use self::tantivy::TantivyEngine;
pub use traits::{SearchEngine, SearchResult};
