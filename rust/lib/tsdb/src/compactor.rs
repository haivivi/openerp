use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::TsDbError;
use crate::manifest::{BlockMeta, Manifest};
use crate::traits::LogEntry;

/// Compactor takes WAL entries and compresses them into archive blocks.
///
/// Flow:
/// 1. Read all entries from WAL segments
/// 2. Group by stream (already done — compactor works per-stream)
/// 3. Compress entries with zstd
/// 4. Write block file: `blk-{min_ts}-{max_ts}.zst`
/// 5. Update manifest with block metadata + label index
/// 6. Delete compacted WAL segments
pub struct Compactor;

impl Compactor {
    /// Compact a batch of entries into a single compressed block.
    ///
    /// Returns the BlockMeta if entries were written, or None if no entries.
    pub fn compact(
        archive_dir: &Path,
        entries: &[LogEntry],
        manifest: &mut Manifest,
    ) -> Result<Option<BlockMeta>, TsDbError> {
        if entries.is_empty() {
            return Ok(None);
        }

        fs::create_dir_all(archive_dir).map_err(|e| TsDbError::Io(e.to_string()))?;

        // Find time range.
        let min_ts = entries.iter().map(|e| e.ts).min().unwrap();
        let max_ts = entries.iter().map(|e| e.ts).max().unwrap();

        // Serialize entries: same WAL binary format but without per-record CRC.
        // Format per entry: [ts:8] [labels_len:4] [labels_json] [data_len:4] [data]
        let mut raw = Vec::new();
        for entry in entries {
            let labels_json = serde_json::to_vec(&entry.labels)
                .map_err(|e| TsDbError::Io(e.to_string()))?;
            raw.extend_from_slice(&entry.ts.to_le_bytes());
            raw.extend_from_slice(&(labels_json.len() as u32).to_le_bytes());
            raw.extend_from_slice(&labels_json);
            raw.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
            raw.extend_from_slice(&entry.data);
        }

        let uncompressed_size = raw.len() as u64;

        // Compress with zstd.
        let compressed = zstd::encode_all(raw.as_slice(), 3)
            .map_err(|e| TsDbError::Compression(e.to_string()))?;

        let compressed_size = compressed.len() as u64;

        // Write block file.
        let filename = format!("blk-{}-{}.zst", min_ts, max_ts);
        let block_path = archive_dir.join(&filename);
        fs::write(&block_path, &compressed).map_err(|e| TsDbError::Io(e.to_string()))?;

        // Build label index.
        let mut label_index: HashMap<String, Vec<String>> = HashMap::new();
        for entry in entries {
            for (key, value) in &entry.labels {
                let values = label_index.entry(key.clone()).or_default();
                if !values.contains(value) {
                    values.push(value.clone());
                }
            }
        }
        for values in label_index.values_mut() {
            values.sort();
        }

        let block_meta = BlockMeta {
            filename,
            min_ts,
            max_ts,
            entry_count: entries.len() as u64,
            compressed_size,
            uncompressed_size,
            label_index,
        };

        manifest.add_block(block_meta.clone());

        Ok(Some(block_meta))
    }
}
