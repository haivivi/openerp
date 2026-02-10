use std::collections::HashMap;

use crate::error::SearchError;

/// A single search result with its document ID and relevance score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub fields: HashMap<String, String>,
}

/// SearchEngine provides full-text search over indexed documents.
///
/// Documents are organized by collection (e.g. "devices", "batches").
/// Each document has an ID and a set of string fields that are indexed.
pub trait SearchEngine: Send + Sync {
    /// Index a document. If a document with the same ID already exists in the
    /// collection, it is replaced.
    fn index(
        &self,
        collection: &str,
        id: &str,
        doc: HashMap<String, String>,
    ) -> Result<(), SearchError>;

    /// Delete a document by ID from a collection.
    fn delete(&self, collection: &str, id: &str) -> Result<(), SearchError>;

    /// Search a collection with a query string. Returns up to `limit` results
    /// ordered by relevance score (highest first).
    fn search(
        &self,
        collection: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError>;
}
