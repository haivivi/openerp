use thiserror::Error;

#[derive(Error, Debug)]
pub enum TsDbError {
    #[error("I/O error: {0}")]
    Io(String),

    #[error("compression error: {0}")]
    Compression(String),

    #[error("corrupt data: {0}")]
    Corrupt(String),

    #[error("query error: {0}")]
    Query(String),
}
