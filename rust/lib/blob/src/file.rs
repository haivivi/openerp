use std::fs;
use std::path::{Path, PathBuf};

use crate::error::BlobError;
use crate::traits::{BlobMeta, BlobStore};

/// FileStore is a BlobStore implementation backed by the local filesystem.
///
/// Keys are mapped to paths under `base_dir`:
///   key "firmware/h106/1.0.0.bin" → `{base_dir}/firmware/h106/1.0.0.bin`
///
/// Parent directories are created automatically on `put`.
pub struct FileStore {
    base_dir: PathBuf,
}

impl FileStore {
    /// Create a new FileStore rooted at `base_dir`.
    /// The directory is created if it doesn't exist.
    pub fn open(base_dir: &Path) -> Result<Self, BlobError> {
        fs::create_dir_all(base_dir).map_err(|e| BlobError::Io(e.to_string()))?;
        Ok(Self {
            base_dir: base_dir.to_path_buf(),
        })
    }

    /// Resolve a key to a filesystem path. Rejects keys that escape base_dir.
    fn resolve(&self, key: &str) -> Result<PathBuf, BlobError> {
        // Reject empty keys and absolute paths.
        if key.is_empty() || key.starts_with('/') || key.starts_with('\\') {
            return Err(BlobError::Io(format!("invalid blob key: {:?}", key)));
        }

        let path = self.base_dir.join(key);

        // Ensure the resolved path is still under base_dir (prevent traversal).
        let canonical_base = self
            .base_dir
            .canonicalize()
            .map_err(|e| BlobError::Io(e.to_string()))?;

        // For non-existent paths, check the parent.
        let check_path = if path.exists() {
            path.canonicalize()
                .map_err(|e| BlobError::Io(e.to_string()))?
        } else if let Some(parent) = path.parent() {
            if parent.exists() {
                let canonical_parent = parent
                    .canonicalize()
                    .map_err(|e| BlobError::Io(e.to_string()))?;
                canonical_parent.join(path.file_name().unwrap_or_default())
            } else {
                // Parent doesn't exist yet — will be created on put.
                // Do a basic component check instead.
                if key.contains("..") {
                    return Err(BlobError::Io(format!(
                        "path traversal detected in key: {:?}",
                        key
                    )));
                }
                return Ok(path);
            }
        } else {
            return Err(BlobError::Io(format!("invalid blob key: {:?}", key)));
        };

        if !check_path.starts_with(&canonical_base) {
            return Err(BlobError::Io(format!(
                "path traversal detected in key: {:?}",
                key
            )));
        }

        Ok(path)
    }
}

impl BlobStore for FileStore {
    fn put(&self, key: &str, data: &[u8]) -> Result<(), BlobError> {
        let path = self.resolve(key)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| BlobError::Io(e.to_string()))?;
        }
        fs::write(&path, data).map_err(|e| BlobError::Io(e.to_string()))?;
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, BlobError> {
        let path = self.resolve(key)?;
        if !path.is_file() {
            return Ok(None);
        }
        let data = fs::read(&path).map_err(|e| BlobError::Io(e.to_string()))?;
        Ok(Some(data))
    }

    fn delete(&self, key: &str) -> Result<(), BlobError> {
        let path = self.resolve(key)?;
        if path.is_file() {
            fs::remove_file(&path).map_err(|e| BlobError::Io(e.to_string()))?;
        }
        Ok(())
    }

    fn exists(&self, key: &str) -> Result<bool, BlobError> {
        let path = self.resolve(key)?;
        Ok(path.is_file())
    }

    fn list(&self, prefix: &str) -> Result<Vec<BlobMeta>, BlobError> {
        let mut results = Vec::new();
        self.walk_dir(&self.base_dir, prefix, &mut results)?;
        results.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(results)
    }
}

impl FileStore {
    /// Recursively walk directory, collecting blobs whose keys match prefix.
    fn walk_dir(
        &self,
        dir: &Path,
        prefix: &str,
        results: &mut Vec<BlobMeta>,
    ) -> Result<(), BlobError> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(dir).map_err(|e| BlobError::Io(e.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|e| BlobError::Io(e.to_string()))?;
            let path = entry.path();

            if path.is_dir() {
                self.walk_dir(&path, prefix, results)?;
            } else if path.is_file() {
                // Convert path back to key (relative to base_dir).
                if let Ok(rel) = path.strip_prefix(&self.base_dir) {
                    let key = rel.to_string_lossy().to_string();
                    if key.starts_with(prefix) {
                        let meta = entry
                            .metadata()
                            .map_err(|e| BlobError::Io(e.to_string()))?;
                        results.push(BlobMeta {
                            key,
                            size: meta.len(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
