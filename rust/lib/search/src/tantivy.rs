use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, STORED, STRING, TEXT};
use tantivy::schema::Value as TantivyValue;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};

use crate::error::SearchError;
use crate::traits::{SearchEngine, SearchResult};

/// Per-collection index state.
struct CollectionIndex {
    index: Index,
    reader: IndexReader,
    writer: RwLock<IndexWriter>,
    id_field: Field,
    body_field: Field,
    fields_field: Field,
}

/// TantivyEngine is a SearchEngine implementation backed by Tantivy.
///
/// Each collection gets its own Tantivy index in a subdirectory.
/// Documents have three fields:
/// - `_id` (STRING | STORED): exact-match document ID, untokenized
/// - `_body` (TEXT | STORED): concatenated field values for full-text search
/// - `_fields` (STORED only): JSON of original fields for retrieval, not indexed
pub struct TantivyEngine {
    base_dir: std::path::PathBuf,
    collections: RwLock<HashMap<String, CollectionIndex>>,
}

impl TantivyEngine {
    /// Create a new TantivyEngine with indexes stored under `base_dir`.
    pub fn open(base_dir: &Path) -> Result<Self, SearchError> {
        std::fs::create_dir_all(base_dir)
            .map_err(|e| SearchError::Index(e.to_string()))?;

        Ok(Self {
            base_dir: base_dir.to_path_buf(),
            collections: RwLock::new(HashMap::new()),
        })
    }

    /// Get or create a collection index.
    fn get_or_create_collection(
        &self,
        collection: &str,
    ) -> Result<(), SearchError> {
        // Fast path: already exists.
        {
            let collections = self.collections.read().unwrap();
            if collections.contains_key(collection) {
                return Ok(());
            }
        }

        // Slow path: create.
        let mut collections = self.collections.write().unwrap();
        if collections.contains_key(collection) {
            return Ok(());
        }

        let col_dir = self.base_dir.join(collection);
        std::fs::create_dir_all(&col_dir)
            .map_err(|e| SearchError::Index(e.to_string()))?;

        let mut schema_builder = Schema::builder();
        let id_field = schema_builder.add_text_field("_id", STRING | STORED);
        let body_field = schema_builder.add_text_field("_body", TEXT);
        let fields_field = schema_builder.add_text_field("_fields", STORED);
        let schema = schema_builder.build();

        let dir = tantivy::directory::MmapDirectory::open(&col_dir)
            .map_err(|e| SearchError::Index(e.to_string()))?;

        let index = Index::open_or_create(dir, schema.clone())
            .map_err(|e| SearchError::Index(e.to_string()))?;

        let writer = index
            .writer(15_000_000) // 15 MB heap
            .map_err(|e| SearchError::Index(e.to_string()))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e: tantivy::TantivyError| SearchError::Index(e.to_string()))?;

        collections.insert(
            collection.to_string(),
            CollectionIndex {
                index,
                reader,
                writer: RwLock::new(writer),
                id_field,
                body_field,
                fields_field,
            },
        );

        Ok(())
    }
}

impl SearchEngine for TantivyEngine {
    fn index(
        &self,
        collection: &str,
        id: &str,
        doc_fields: HashMap<String, String>,
    ) -> Result<(), SearchError> {
        self.get_or_create_collection(collection)?;

        let collections = self.collections.read().unwrap();
        let col = collections
            .get(collection)
            .ok_or_else(|| SearchError::Index("collection not found".into()))?;

        // _body: concatenated field values only (no JSON keys polluting the index).
        let body_text: String = doc_fields.values().cloned().collect::<Vec<_>>().join(" ");
        // _fields: JSON for retrieval on the read path (STORED only, not indexed).
        let fields_json = serde_json::to_string(&doc_fields)
            .map_err(|e| SearchError::Index(e.to_string()))?;

        let mut writer = col.writer.write().unwrap();

        // Delete existing document with same ID (upsert).
        let term = tantivy::Term::from_field_text(col.id_field, id);
        writer.delete_term(term);

        writer
            .add_document(doc!(
                col.id_field => id,
                col.body_field => body_text,
                col.fields_field => fields_json,
            ))
            .map_err(|e| SearchError::Index(e.to_string()))?;

        writer
            .commit()
            .map_err(|e| SearchError::Index(e.to_string()))?;

        Ok(())
    }

    fn delete(&self, collection: &str, id: &str) -> Result<(), SearchError> {
        self.get_or_create_collection(collection)?;

        let collections = self.collections.read().unwrap();
        let col = collections
            .get(collection)
            .ok_or_else(|| SearchError::Index("collection not found".into()))?;

        let mut writer = col.writer.write().unwrap();
        let term = tantivy::Term::from_field_text(col.id_field, id);
        writer.delete_term(term);
        writer
            .commit()
            .map_err(|e| SearchError::Index(e.to_string()))?;

        Ok(())
    }

    fn search(
        &self,
        collection: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        self.get_or_create_collection(collection)?;

        let collections = self.collections.read().unwrap();
        let col = collections
            .get(collection)
            .ok_or_else(|| SearchError::Index("collection not found".into()))?;

        // Reload the reader to pick up latest commits.
        col.reader
            .reload()
            .map_err(|e| SearchError::Query(e.to_string()))?;

        let searcher = col.reader.searcher();
        // Only search the _body field by default. _id is STRING (untokenized)
        // and not suitable for full-text queries.
        let query_parser =
            QueryParser::for_index(&col.index, vec![col.body_field]);

        let parsed = query_parser
            .parse_query(query)
            .map_err(|e| SearchError::Query(e.to_string()))?;

        let top_docs = searcher
            .search(&parsed, &TopDocs::with_limit(limit))
            .map_err(|e| SearchError::Query(e.to_string()))?;

        let mut results = Vec::new();
        for (score, doc_addr) in top_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_addr)
                .map_err(|e| SearchError::Query(e.to_string()))?;

            let id = doc
                .get_first(col.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            // Read stored fields JSON from the dedicated _fields field.
            let fields_json = doc
                .get_first(col.fields_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let fields = serde_json::from_str::<HashMap<String, String>>(fields_json)
                .unwrap_or_default();

            results.push(SearchResult { id, score, fields });
        }

        Ok(results)
    }
}
