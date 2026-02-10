use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::TsDbError;

/// Metadata for a single compressed archive block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMeta {
    /// Block filename (relative to archive dir).
    pub filename: String,
    /// Minimum timestamp in this block (nanoseconds).
    pub min_ts: u64,
    /// Maximum timestamp in this block (nanoseconds).
    pub max_ts: u64,
    /// Number of entries in the block.
    pub entry_count: u64,
    /// Compressed size in bytes.
    pub compressed_size: u64,
    /// Uncompressed size in bytes.
    pub uncompressed_size: u64,
    /// Label keys → set of values present in this block.
    /// Used for filtering: if a query label value is not in this set,
    /// we can skip decompressing the block entirely.
    pub label_index: HashMap<String, Vec<String>>,
}

/// Manifest tracks all archive blocks for a stream.
///
/// Stored as `manifest.json` in the stream's archive directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Stream name.
    pub stream: String,
    /// Ordered list of blocks (sorted by min_ts ascending).
    pub blocks: Vec<BlockMeta>,
}

impl Manifest {
    /// Create a new empty manifest for a stream.
    pub fn new(stream: &str) -> Self {
        Self {
            stream: stream.to_string(),
            blocks: Vec::new(),
        }
    }

    /// Load manifest from disk. Returns empty manifest if file doesn't exist.
    pub fn load(path: &Path, stream: &str) -> Result<Self, TsDbError> {
        let manifest_path = Self::manifest_path(path);
        if !manifest_path.is_file() {
            return Ok(Self::new(stream));
        }
        let data = fs::read(&manifest_path).map_err(|e| TsDbError::Io(e.to_string()))?;
        let manifest: Manifest =
            serde_json::from_slice(&data).map_err(|e| TsDbError::Corrupt(e.to_string()))?;
        Ok(manifest)
    }

    /// Save manifest to disk.
    pub fn save(&self, path: &Path) -> Result<(), TsDbError> {
        fs::create_dir_all(path).map_err(|e| TsDbError::Io(e.to_string()))?;
        let manifest_path = Self::manifest_path(path);
        let data =
            serde_json::to_vec_pretty(self).map_err(|e| TsDbError::Io(e.to_string()))?;
        fs::write(&manifest_path, data).map_err(|e| TsDbError::Io(e.to_string()))?;
        Ok(())
    }

    /// Add a block and keep blocks sorted by min_ts.
    pub fn add_block(&mut self, block: BlockMeta) {
        self.blocks.push(block);
        self.blocks.sort_by_key(|b| b.min_ts);
    }

    /// Find blocks that might contain entries matching the given label filters
    /// and time range.
    pub fn find_blocks(
        &self,
        label_filters: &HashMap<String, String>,
        start: Option<u64>,
        end: Option<u64>,
    ) -> Vec<&BlockMeta> {
        self.blocks
            .iter()
            .filter(|block| {
                // Time range filter.
                if let Some(start_ts) = start {
                    if block.max_ts < start_ts {
                        return false;
                    }
                }
                if let Some(end_ts) = end {
                    if block.min_ts > end_ts {
                        return false;
                    }
                }

                // Label filter: all query labels must have matching values in block index.
                for (key, value) in label_filters {
                    if let Some(block_values) = block.label_index.get(key) {
                        if !block_values.contains(value) {
                            return false;
                        }
                    } else {
                        // Block doesn't have this label at all — skip it.
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Aggregate all label keys and values across all blocks.
    pub fn all_labels(&self) -> HashMap<String, Vec<String>> {
        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        for block in &self.blocks {
            for (key, values) in &block.label_index {
                let entry = result.entry(key.clone()).or_default();
                for v in values {
                    if !entry.contains(v) {
                        entry.push(v.clone());
                    }
                }
            }
        }
        // Sort values for determinism.
        for values in result.values_mut() {
            values.sort();
        }
        result
    }

    /// Total entry count across all blocks.
    pub fn total_entries(&self) -> u64 {
        self.blocks.iter().map(|b| b.entry_count).sum()
    }

    /// Total compressed size across all blocks.
    pub fn total_compressed_size(&self) -> u64 {
        self.blocks.iter().map(|b| b.compressed_size).sum()
    }

    fn manifest_path(dir: &Path) -> PathBuf {
        dir.join("manifest.json")
    }
}
