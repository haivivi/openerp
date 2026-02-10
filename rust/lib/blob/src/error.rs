use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlobError {
    #[error("I/O error: {0}")]
    Io(String),

    #[error("blob not found: {0}")]
    NotFound(String),
}
