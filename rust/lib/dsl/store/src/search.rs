//! SearchStore trait + SearchOps for full-text search.
//!
//! Models impl `SearchStore` to declare which fields are searchable.
//! `SearchOps<T>` provides index/search/remove using a SearchEngine backend.

use oe_core::ServiceError;
use oe_types::Field;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Trait implemented by models that support full-text search.
pub trait SearchStore: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    /// Fields to index for full-text search.
    const FIELDS: &[Field];

    /// Collection name (usually "{module}:{resource}").
    fn search_collection() -> &'static str;

    /// Extract the document ID (usually the primary key value).
    fn search_id(&self) -> String;

    /// Extract searchable field values from this instance.
    /// Default: serialize to JSON and pick FIELDS by name.
    fn search_doc(&self) -> HashMap<String, String> {
        let json = serde_json::to_value(self).unwrap_or_default();
        let mut doc = HashMap::new();
        for field in Self::FIELDS {
            // Try snake_case and camelCase.
            let val = json
                .get(field.name)
                .or_else(|| json.get(&to_camel_case(field.name)))
                .and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Null => None,
                    other => Some(other.to_string()),
                });
            if let Some(v) = val {
                doc.insert(field.name.to_string(), v);
            }
        }
        doc
    }
}

/// Full-text search operations for a SearchStore model.
pub struct SearchOps<T: SearchStore> {
    engine: Arc<dyn oe_search::SearchEngine>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: SearchStore> SearchOps<T> {
    pub fn new(engine: Arc<dyn oe_search::SearchEngine>) -> Self {
        Self {
            engine,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Index a record for full-text search.
    pub fn index(&self, record: &T) -> Result<(), ServiceError> {
        let id = record.search_id();
        let doc = record.search_doc();
        self.engine
            .index(T::search_collection(), &id, doc)
            .map_err(|e| ServiceError::Storage(format!("search index: {}", e)))
    }

    /// Remove a record from the search index.
    pub fn remove(&self, id: &str) -> Result<(), ServiceError> {
        self.engine
            .delete(T::search_collection(), id)
            .map_err(|e| ServiceError::Storage(format!("search remove: {}", e)))
    }

    /// Search for records matching a query string.
    /// Returns (id, score) pairs. Caller fetches full records from primary store.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<(String, f32)>, ServiceError> {
        let results = self
            .engine
            .search(T::search_collection(), query, limit)
            .map_err(|e| ServiceError::Storage(format!("search: {}", e)))?;
        Ok(results.into_iter().map(|r| (r.id, r.score)).collect())
    }
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Article {
        id: String,
        title: String,
        body: String,
    }

    impl SearchStore for Article {
        const FIELDS: &[Field] = &[
            Field::new("title", "String", "text"),
            Field::new("body", "String", "textarea"),
        ];

        fn search_collection() -> &'static str {
            "test:article"
        }

        fn search_id(&self) -> String {
            self.id.clone()
        }
    }

    #[test]
    fn search_doc_extraction() {
        let article = Article {
            id: "a1".into(),
            title: "Hello World".into(),
            body: "This is the body text.".into(),
        };
        let doc = article.search_doc();
        assert_eq!(doc.get("title").unwrap(), "Hello World");
        assert_eq!(doc.get("body").unwrap(), "This is the body text.");
        assert!(!doc.contains_key("id")); // id not in FIELDS
    }
}
