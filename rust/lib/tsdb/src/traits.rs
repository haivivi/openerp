use std::collections::HashMap;

use crate::error::TsDbError;

/// A single log entry with timestamp, labels, and arbitrary JSON data.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Nanosecond Unix timestamp.
    pub ts: u64,
    /// Key-value labels for indexing/filtering (e.g. type:heartbeat, fw:1.2.3).
    pub labels: HashMap<String, String>,
    /// Arbitrary JSON payload.
    pub data: Vec<u8>,
}

/// Query parameters for log retrieval.
#[derive(Debug, Clone)]
pub struct LogQuery {
    /// Stream key (e.g. device SN).
    pub stream: String,
    /// Label matchers: only entries whose labels are a superset of these are returned.
    pub labels: HashMap<String, String>,
    /// Maximum number of entries to return.
    pub limit: usize,
    /// If true, return newest entries first.
    pub desc: bool,
    /// Optional start timestamp (inclusive, nanoseconds).
    pub start: Option<u64>,
    /// Optional end timestamp (inclusive, nanoseconds).
    pub end: Option<u64>,
}

/// Statistics for a stream.
#[derive(Debug, Clone)]
pub struct StreamStats {
    /// Total number of entries across WAL and archive.
    pub total_entries: u64,
    /// Total bytes on disk (WAL + compressed blocks).
    pub total_bytes: u64,
    /// Number of compressed archive blocks.
    pub block_count: u64,
    /// Number of active WAL segments.
    pub wal_segments: u64,
}

/// TsDb provides a Loki-inspired log storage engine.
///
/// Data is organized by **stream** (e.g. device SN). Each stream has:
/// - A WAL (write-ahead log) for hot data
/// - Compressed archive blocks for cold data
/// - A manifest with block metadata and label indexes
pub trait TsDb: Send + Sync {
    /// Write a log entry to a stream.
    fn write(&self, stream: &str, entry: LogEntry) -> Result<(), TsDbError>;

    /// Write a batch of log entries to a stream.
    fn write_batch(&self, stream: &str, entries: Vec<LogEntry>) -> Result<(), TsDbError>;

    /// Query log entries from a stream.
    fn query(&self, query: &LogQuery) -> Result<Vec<LogEntry>, TsDbError>;

    /// Get all known label keys and their values for a stream.
    fn labels(&self, stream: &str) -> Result<HashMap<String, Vec<String>>, TsDbError>;

    /// Get statistics for a stream.
    fn stats(&self, stream: &str) -> Result<StreamStats, TsDbError>;
}
