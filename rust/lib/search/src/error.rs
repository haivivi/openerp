use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("index error: {0}")]
    Index(String),

    #[error("query error: {0}")]
    Query(String),

    #[error("schema error: {0}")]
    Schema(String),
}
