use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::TsDbError;
use crate::manifest::BlockMeta;
use crate::traits::LogEntry;

/// Reader decompresses and decodes archive blocks.
pub struct Reader;

impl Reader {
    /// Read and decode entries from a compressed block file.
    pub fn read_block(archive_dir: &Path, block: &BlockMeta) -> Result<Vec<LogEntry>, TsDbError> {
        let block_path = archive_dir.join(&block.filename);
        let compressed =
            fs::read(&block_path).map_err(|e| TsDbError::Io(e.to_string()))?;

        let decompressed = zstd::decode_all(compressed.as_slice())
            .map_err(|e| TsDbError::Compression(e.to_string()))?;

        Self::decode_entries(&decompressed)
    }

    /// Read a block and filter entries by labels and time range.
    pub fn read_block_filtered(
        archive_dir: &Path,
        block: &BlockMeta,
        label_filters: &HashMap<String, String>,
        start: Option<u64>,
        end: Option<u64>,
    ) -> Result<Vec<LogEntry>, TsDbError> {
        let entries = Self::read_block(archive_dir, block)?;

        let filtered = entries
            .into_iter()
            .filter(|entry| {
                // Time range.
                if let Some(start_ts) = start {
                    if entry.ts < start_ts {
                        return false;
                    }
                }
                if let Some(end_ts) = end {
                    if entry.ts > end_ts {
                        return false;
                    }
                }

                // Label matching.
                for (key, value) in label_filters {
                    match entry.labels.get(key) {
                        Some(v) if v == value => {}
                        _ => return false,
                    }
                }

                true
            })
            .collect();

        Ok(filtered)
    }

    /// Decode entries from raw (uncompressed) block data.
    /// Format per entry: [ts:8] [labels_len:4] [labels_json] [data_len:4] [data]
    fn decode_entries(data: &[u8]) -> Result<Vec<LogEntry>, TsDbError> {
        let mut entries = Vec::new();
        let mut pos = 0;

        while pos + 12 <= data.len() {
            // ts
            let ts = u64::from_le_bytes(
                data[pos..pos + 8]
                    .try_into()
                    .map_err(|_| TsDbError::Corrupt("bad ts in block".into()))?,
            );
            pos += 8;

            // labels_len
            let labels_len = u32::from_le_bytes(
                data[pos..pos + 4]
                    .try_into()
                    .map_err(|_| TsDbError::Corrupt("bad labels_len in block".into()))?,
            ) as usize;
            pos += 4;

            if pos + labels_len > data.len() {
                break;
            }
            let labels_json = &data[pos..pos + labels_len];
            pos += labels_len;

            // data_len
            if pos + 4 > data.len() {
                break;
            }
            let data_len = u32::from_le_bytes(
                data[pos..pos + 4]
                    .try_into()
                    .map_err(|_| TsDbError::Corrupt("bad data_len in block".into()))?,
            ) as usize;
            pos += 4;

            if pos + data_len > data.len() {
                break;
            }
            let entry_data = data[pos..pos + data_len].to_vec();
            pos += data_len;

            let labels: HashMap<String, String> =
                serde_json::from_slice(labels_json).unwrap_or_default();

            entries.push(LogEntry {
                ts,
                labels,
                data: entry_data,
            });
        }

        Ok(entries)
    }
}
