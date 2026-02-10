use thiserror::Error;

#[derive(Error, Debug)]
pub enum KVError {
    #[error("key is read-only: {0}")]
    ReadOnly(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}
