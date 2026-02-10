use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::TsDbError;
use crate::traits::LogEntry;

/// On-disk format for a single WAL record:
///
/// ```text
/// [ts: 8 bytes LE] [labels_len: 4 bytes LE] [labels_json: N bytes]
/// [data_len: 4 bytes LE] [data: M bytes] [crc32: 4 bytes LE]
/// ```
///
/// Each segment file is named `wal-{seq}.log` where seq is a monotonic counter.

const WAL_RECORD_HEADER_SIZE: usize = 8 + 4; // ts + labels_len
const MAX_SEGMENT_SIZE: u64 = 16 * 1024 * 1024; // 16 MB per segment

/// A WAL segment writer.
struct WalSegment {
    #[allow(dead_code)]
    path: PathBuf,
    writer: BufWriter<File>,
    size: u64,
    entry_count: u64,
    min_ts: u64,
    max_ts: u64,
}

/// WAL manages write-ahead log segments for a single stream.
pub struct Wal {
    dir: PathBuf,
    current: Option<WalSegment>,
    next_seq: u64,
}

impl Wal {
    /// Open or create a WAL directory for a stream.
    pub fn open(dir: &Path) -> Result<Self, TsDbError> {
        fs::create_dir_all(dir).map_err(|e| TsDbError::Io(e.to_string()))?;

        // Find the highest existing sequence number.
        let mut max_seq: u64 = 0;
        let entries = fs::read_dir(dir).map_err(|e| TsDbError::Io(e.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|e| TsDbError::Io(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(seq_str) = name.strip_prefix("wal-").and_then(|s| s.strip_suffix(".log")) {
                if let Ok(seq) = seq_str.parse::<u64>() {
                    max_seq = max_seq.max(seq);
                }
            }
        }

        Ok(Self {
            dir: dir.to_path_buf(),
            current: None,
            next_seq: max_seq + 1,
        })
    }

    /// Append an entry to the WAL. Rotates segments when size exceeds threshold.
    pub fn append(&mut self, entry: &LogEntry) -> Result<(), TsDbError> {
        // Rotate if current segment is too large.
        if let Some(ref seg) = self.current {
            if seg.size >= MAX_SEGMENT_SIZE {
                self.rotate()?;
            }
        }

        // Create segment if none.
        if self.current.is_none() {
            self.new_segment()?;
        }

        let seg = self.current.as_mut().unwrap();

        let labels_json = serde_json::to_vec(&entry.labels)
            .map_err(|e| TsDbError::Io(e.to_string()))?;

        // Write record.
        let mut record = Vec::with_capacity(
            WAL_RECORD_HEADER_SIZE + labels_json.len() + 4 + entry.data.len() + 4,
        );
        record.extend_from_slice(&entry.ts.to_le_bytes());
        record.extend_from_slice(&(labels_json.len() as u32).to_le_bytes());
        record.extend_from_slice(&labels_json);
        record.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
        record.extend_from_slice(&entry.data);

        let crc = crc32fast::hash(&record);
        record.extend_from_slice(&crc.to_le_bytes());

        seg.writer
            .write_all(&record)
            .map_err(|e| TsDbError::Io(e.to_string()))?;
        seg.writer
            .flush()
            .map_err(|e| TsDbError::Io(e.to_string()))?;

        seg.size += record.len() as u64;
        seg.entry_count += 1;
        if seg.min_ts == 0 || entry.ts < seg.min_ts {
            seg.min_ts = entry.ts;
        }
        if entry.ts > seg.max_ts {
            seg.max_ts = entry.ts;
        }

        Ok(())
    }

    /// Read all entries from all WAL segments (for query merging).
    pub fn read_all(&self) -> Result<Vec<LogEntry>, TsDbError> {
        let mut all_entries = Vec::new();
        let mut segment_files = self.list_segment_files()?;
        segment_files.sort();

        for path in segment_files {
            let data = fs::read(&path).map_err(|e| TsDbError::Io(e.to_string()))?;
            let entries = Self::decode_segment(&data)?;
            all_entries.extend(entries);
        }

        Ok(all_entries)
    }

    /// List all WAL segment file paths.
    pub fn list_segment_files(&self) -> Result<Vec<PathBuf>, TsDbError> {
        let mut files = Vec::new();
        if !self.dir.is_dir() {
            return Ok(files);
        }
        let entries = fs::read_dir(&self.dir).map_err(|e| TsDbError::Io(e.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|e| TsDbError::Io(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("wal-") && name.ends_with(".log") {
                files.push(entry.path());
            }
        }
        Ok(files)
    }

    /// Remove specific segment files (after compaction).
    pub fn remove_segments(&self, paths: &[PathBuf]) -> Result<(), TsDbError> {
        for path in paths {
            if path.is_file() {
                fs::remove_file(path).map_err(|e| TsDbError::Io(e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Get total size of all WAL segments.
    pub fn total_size(&self) -> Result<u64, TsDbError> {
        let mut total = 0u64;
        for path in self.list_segment_files()? {
            let meta = fs::metadata(&path).map_err(|e| TsDbError::Io(e.to_string()))?;
            total += meta.len();
        }
        Ok(total)
    }

    /// Get number of WAL segment files.
    pub fn segment_count(&self) -> Result<u64, TsDbError> {
        Ok(self.list_segment_files()?.len() as u64)
    }

    /// Flush the current segment.
    pub fn flush(&mut self) -> Result<(), TsDbError> {
        if let Some(ref mut seg) = self.current {
            seg.writer
                .flush()
                .map_err(|e| TsDbError::Io(e.to_string()))?;
        }
        Ok(())
    }

    fn new_segment(&mut self) -> Result<(), TsDbError> {
        let seq = self.next_seq;
        self.next_seq += 1;

        let path = self.dir.join(format!("wal-{:08}.log", seq));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| TsDbError::Io(e.to_string()))?;

        self.current = Some(WalSegment {
            path,
            writer: BufWriter::new(file),
            size: 0,
            entry_count: 0,
            min_ts: 0,
            max_ts: 0,
        });

        Ok(())
    }

    fn rotate(&mut self) -> Result<(), TsDbError> {
        if let Some(ref mut seg) = self.current {
            seg.writer
                .flush()
                .map_err(|e| TsDbError::Io(e.to_string()))?;
        }
        self.current = None;
        self.new_segment()
    }

    /// Decode all entries from a raw segment byte buffer.
    fn decode_segment(data: &[u8]) -> Result<Vec<LogEntry>, TsDbError> {
        let mut entries = Vec::new();
        let mut pos = 0;

        while pos + WAL_RECORD_HEADER_SIZE <= data.len() {
            // ts
            let ts = u64::from_le_bytes(
                data[pos..pos + 8]
                    .try_into()
                    .map_err(|_| TsDbError::Corrupt("bad ts".into()))?,
            );
            pos += 8;

            // labels_len
            let labels_len = u32::from_le_bytes(
                data[pos..pos + 4]
                    .try_into()
                    .map_err(|_| TsDbError::Corrupt("bad labels_len".into()))?,
            ) as usize;
            pos += 4;

            if pos + labels_len > data.len() {
                break; // Truncated segment, stop.
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
                    .map_err(|_| TsDbError::Corrupt("bad data_len".into()))?,
            ) as usize;
            pos += 4;

            if pos + data_len > data.len() {
                break;
            }
            let entry_data = data[pos..pos + data_len].to_vec();
            pos += data_len;

            // crc32
            if pos + 4 > data.len() {
                break;
            }
            let stored_crc = u32::from_le_bytes(
                data[pos..pos + 4]
                    .try_into()
                    .map_err(|_| TsDbError::Corrupt("bad crc".into()))?,
            );
            pos += 4;

            // Verify CRC over the record (everything before the CRC field).
            let record_end = pos - 4;
            let record_start = record_end - 8 - 4 - labels_len - 4 - data_len;
            let computed_crc = crc32fast::hash(&data[record_start..record_end]);
            if computed_crc != stored_crc {
                // CRC mismatch — skip this record, stop reading.
                break;
            }

            let labels: std::collections::HashMap<String, String> =
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
