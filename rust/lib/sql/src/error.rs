use thiserror::Error;

#[derive(Error, Debug)]
pub enum SQLError {
    #[error("query error: {0}")]
    Query(String),

    #[error("execution error: {0}")]
    Execution(String),

    #[error("connection error: {0}")]
    Connection(String),
}
