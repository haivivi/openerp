use std::path::Path;
use std::sync::Arc;

use redb::{Database, TableDefinition};

use crate::error::KVError;
use crate::traits::KVStore;

const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("kv");

/// RedbStore is a KVStore implementation backed by redb — a pure-Rust embedded
/// key-value database. All keys are read-write (not read-only).
pub struct RedbStore {
    db: Arc<Database>,
}

impl RedbStore {
    /// Open or create a redb database at the given path.
    pub fn open(path: &Path) -> Result<Self, KVError> {
        let db = Database::create(path).map_err(|e| KVError::Storage(e.to_string()))?;

        // Ensure the table exists by doing a write transaction.
        let write_txn = db
            .begin_write()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        {
            let _table = write_txn
                .open_table(TABLE)
                .map_err(|e| KVError::Storage(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| KVError::Storage(e.to_string()))?;

        Ok(Self {
            db: Arc::new(db),
        })
    }
}

impl KVStore for RedbStore {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, KVError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        let table = read_txn
            .open_table(TABLE)
            .map_err(|e| KVError::Storage(e.to_string()))?;

        match table.get(key) {
            Ok(Some(val)) => Ok(Some(val.value().to_vec())),
            Ok(None) => Ok(None),
            Err(e) => Err(KVError::Storage(e.to_string())),
        }
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), KVError> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(TABLE)
                .map_err(|e| KVError::Storage(e.to_string()))?;
            table
                .insert(key, value)
                .map_err(|e| KVError::Storage(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), KVError> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(TABLE)
                .map_err(|e| KVError::Storage(e.to_string()))?;
            table
                .remove(key)
                .map_err(|e| KVError::Storage(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        Ok(())
    }

    fn batch_set(&self, entries: &[(&str, &[u8])]) -> Result<(), KVError> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(TABLE)
                .map_err(|e| KVError::Storage(e.to_string()))?;
            for (key, value) in entries {
                table
                    .insert(*key, *value)
                    .map_err(|e| KVError::Storage(e.to_string()))?;
            }
        }
        write_txn
            .commit()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        Ok(())
    }

    fn batch_delete(&self, keys: &[&str]) -> Result<(), KVError> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(TABLE)
                .map_err(|e| KVError::Storage(e.to_string()))?;
            for key in keys {
                table
                    .remove(*key)
                    .map_err(|e| KVError::Storage(e.to_string()))?;
            }
        }
        write_txn
            .commit()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        Ok(())
    }

    fn scan(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>, KVError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| KVError::Storage(e.to_string()))?;
        let table = read_txn
            .open_table(TABLE)
            .map_err(|e| KVError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        let iter = table
            .range(prefix..)
            .map_err(|e| KVError::Storage(e.to_string()))?;

        for entry in iter {
            let entry = entry.map_err(|e| KVError::Storage(e.to_string()))?;
            let key = entry.0.value().to_string();
            if !key.starts_with(prefix) {
                break;
            }
            let value = entry.1.value().to_vec();
            results.push((key, value));
        }

        Ok(results)
    }

    fn is_readonly(&self, _key: &str) -> bool {
        false
    }
}
