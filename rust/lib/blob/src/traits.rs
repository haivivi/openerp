use std::io::{Read, Write};

use crate::error::BlobError;

/// Metadata for a stored blob.
#[derive(Debug, Clone)]
pub struct BlobMeta {
    pub key: String,
    pub size: u64,
}

/// BlobStore provides storage for binary large objects (firmware images, CSV
/// imports, uploaded files, etc.).
///
/// Keys are path-like strings: `firmware/h106/1.0.0.bin`, `imports/batch-42.csv`.
/// The default implementation (`FileStore`) maps keys to local filesystem paths.
/// Can be swapped for S3/OSS backends by implementing this trait.
pub trait BlobStore: Send + Sync {
    /// Store a blob. Overwrites if the key already exists.
    fn put(&self, key: &str, data: &[u8]) -> Result<(), BlobError>;

    /// Retrieve a blob. Returns None if the key does not exist.
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, BlobError>;

    /// Delete a blob. No-op if the key does not exist.
    fn delete(&self, key: &str) -> Result<(), BlobError>;

    /// Check whether a blob exists.
    fn exists(&self, key: &str) -> Result<bool, BlobError>;

    /// List blobs matching a key prefix. Returns metadata sorted by key.
    fn list(&self, prefix: &str) -> Result<Vec<BlobMeta>, BlobError>;

    /// Open a blob for streaming read. Returns a reader.
    /// Returns `BlobError::NotFound` if the key does not exist.
    fn read_stream(&self, key: &str) -> Result<Box<dyn Read + Send>, BlobError>;

    /// Open a blob for streaming write. Returns a writer.
    /// Parent directories are created automatically.
    /// The blob is committed when the writer is dropped/closed.
    fn write_stream(&self, key: &str) -> Result<Box<dyn Write + Send>, BlobError>;
}
