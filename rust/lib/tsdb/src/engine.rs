use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::compactor::Compactor;
use crate::error::TsDbError;
use crate::manifest::Manifest;
use crate::reader::Reader;
use crate::traits::{LogEntry, LogQuery, StreamStats, TsDb};
use crate::wal::Wal;

/// Minimum WAL size before compaction is triggered (1 MB).
const COMPACT_THRESHOLD: u64 = 1024 * 1024;

/// Per-stream state.
struct StreamState {
    wal: Wal,
    manifest: Manifest,
}

/// WalEngine is the default TsDb implementation.
///
/// Directory layout per stream:
/// ```text
/// {base_dir}/{stream}/
/// ├── wal/                    # WAL segments
/// │   ├── wal-00000001.log
/// │   └── wal-00000002.log
/// ├── archive/                # Compressed blocks
/// │   ├── blk-{min}-{max}.zst
/// │   └── manifest.json       # Block metadata + label index
/// ```
pub struct WalEngine {
    base_dir: PathBuf,
    streams: Mutex<HashMap<String, StreamState>>,
}

impl WalEngine {
    /// Open or create a WalEngine at the given directory.
    pub fn open(base_dir: &Path) -> Result<Self, TsDbError> {
        std::fs::create_dir_all(base_dir).map_err(|e| TsDbError::Io(e.to_string()))?;

        Ok(Self {
            base_dir: base_dir.to_path_buf(),
            streams: Mutex::new(HashMap::new()),
        })
    }

    /// Get or create stream state.
    fn with_stream<F, R>(&self, stream: &str, f: F) -> Result<R, TsDbError>
    where
        F: FnOnce(&mut StreamState) -> Result<R, TsDbError>,
    {
        let mut streams = self
            .streams
            .lock()
            .map_err(|e| TsDbError::Io(e.to_string()))?;

        if !streams.contains_key(stream) {
            let stream_dir = self.base_dir.join(stream);
            let wal_dir = stream_dir.join("wal");
            let archive_dir = stream_dir.join("archive");

            let wal = Wal::open(&wal_dir)?;
            let manifest = Manifest::load(&archive_dir, stream)?;

            streams.insert(
                stream.to_string(),
                StreamState { wal, manifest },
            );
        }

        let state = streams.get_mut(stream).unwrap();
        f(state)
    }

    /// Try to compact WAL segments into archive blocks.
    fn maybe_compact(&self, stream: &str) -> Result<(), TsDbError> {
        self.with_stream(stream, |state| {
            let wal_size = state.wal.total_size()?;
            if wal_size < COMPACT_THRESHOLD {
                return Ok(());
            }

            // Read all WAL entries.
            let entries = state.wal.read_all()?;
            if entries.is_empty() {
                return Ok(());
            }

            // Compact into archive.
            let archive_dir = self.base_dir.join(stream).join("archive");
            Compactor::compact(&archive_dir, &entries, &mut state.manifest)?;

            // Save updated manifest.
            state.manifest.save(&archive_dir)?;

            // Remove compacted WAL segments.
            let segments = state.wal.list_segment_files()?;
            state.wal.remove_segments(&segments)?;

            Ok(())
        })
    }
}

impl TsDb for WalEngine {
    fn write(&self, stream: &str, entry: LogEntry) -> Result<(), TsDbError> {
        self.with_stream(stream, |state| {
            state.wal.append(&entry)
        })?;

        // Check if compaction is needed (non-blocking attempt).
        let _ = self.maybe_compact(stream);

        Ok(())
    }

    fn write_batch(&self, stream: &str, entries: Vec<LogEntry>) -> Result<(), TsDbError> {
        self.with_stream(stream, |state| {
            for entry in &entries {
                state.wal.append(entry)?;
            }
            state.wal.flush()?;
            Ok(())
        })?;

        let _ = self.maybe_compact(stream);

        Ok(())
    }

    fn query(&self, query: &LogQuery) -> Result<Vec<LogEntry>, TsDbError> {
        self.with_stream(&query.stream, |state| {
            let mut all_entries = Vec::new();

            // 1. Read from archive blocks (cold data).
            let archive_dir = self.base_dir.join(&query.stream).join("archive");
            let matching_blocks = state.manifest.find_blocks(
                &query.labels,
                query.start,
                query.end,
            );

            for block in matching_blocks {
                let entries = Reader::read_block_filtered(
                    &archive_dir,
                    block,
                    &query.labels,
                    query.start,
                    query.end,
                )?;
                all_entries.extend(entries);
            }

            // 2. Read from WAL (hot data).
            let wal_entries = state.wal.read_all()?;
            for entry in wal_entries {
                // Time range filter.
                if let Some(start) = query.start {
                    if entry.ts < start {
                        continue;
                    }
                }
                if let Some(end) = query.end {
                    if entry.ts > end {
                        continue;
                    }
                }

                // Label filter.
                let mut matches = true;
                for (key, value) in &query.labels {
                    match entry.labels.get(key) {
                        Some(v) if v == value => {}
                        _ => {
                            matches = false;
                            break;
                        }
                    }
                }
                if matches {
                    all_entries.push(entry);
                }
            }

            // 3. Sort.
            if query.desc {
                all_entries.sort_by(|a, b| b.ts.cmp(&a.ts));
            } else {
                all_entries.sort_by(|a, b| a.ts.cmp(&b.ts));
            }

            // 4. Limit.
            all_entries.truncate(query.limit);

            Ok(all_entries)
        })
    }

    fn labels(&self, stream: &str) -> Result<HashMap<String, Vec<String>>, TsDbError> {
        self.with_stream(stream, |state| {
            let mut result = state.manifest.all_labels();

            // Also scan WAL entries for labels.
            let wal_entries = state.wal.read_all()?;
            for entry in &wal_entries {
                for (key, value) in &entry.labels {
                    let values = result.entry(key.clone()).or_default();
                    if !values.contains(value) {
                        values.push(value.clone());
                    }
                }
            }

            for values in result.values_mut() {
                values.sort();
            }

            Ok(result)
        })
    }

    fn stats(&self, stream: &str) -> Result<StreamStats, TsDbError> {
        self.with_stream(stream, |state| {
            let wal_size = state.wal.total_size()?;
            let wal_segments = state.wal.segment_count()?;
            let wal_entries = state.wal.read_all()?.len() as u64;

            let archive_entries = state.manifest.total_entries();
            let archive_size = state.manifest.total_compressed_size();
            let block_count = state.manifest.blocks.len() as u64;

            Ok(StreamStats {
                total_entries: archive_entries + wal_entries,
                total_bytes: archive_size + wal_size,
                block_count,
                wal_segments,
            })
        })
    }
}
