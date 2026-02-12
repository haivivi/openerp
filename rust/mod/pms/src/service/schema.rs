use openerp_core::ServiceError;
use openerp_sql::SQLStore;

/// SQL DDL statements to initialize the PMS database schema.
///
/// Each table stores the full JSON document in a `data` TEXT column,
/// with indexed columns extracted for efficient filtering and uniqueness.
const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS models (
        id TEXT PRIMARY KEY,
        data TEXT NOT NULL,
        code INTEGER UNIQUE,
        series_name TEXT,
        create_at TEXT,
        update_at TEXT
    )",
    "CREATE TABLE IF NOT EXISTS firmwares (
        id TEXT PRIMARY KEY,
        data TEXT NOT NULL,
        model INTEGER,
        semver TEXT,
        build INTEGER,
        status TEXT,
        create_at TEXT,
        update_at TEXT,
        UNIQUE(model, semver)
    )",
    "CREATE TABLE IF NOT EXISTS batches (
        id TEXT PRIMARY KEY,
        data TEXT NOT NULL,
        name TEXT,
        model INTEGER,
        status TEXT,
        create_at TEXT,
        update_at TEXT
    )",
    "CREATE TABLE IF NOT EXISTS devices (
        id TEXT PRIMARY KEY,
        data TEXT NOT NULL,
        sn TEXT UNIQUE,
        secret TEXT UNIQUE,
        model INTEGER,
        batch_id TEXT,
        status TEXT,
        create_at TEXT,
        update_at TEXT
    )",
    "CREATE TABLE IF NOT EXISTS licenses (
        id TEXT PRIMARY KEY,
        data TEXT NOT NULL,
        license_type TEXT,
        number TEXT,
        import_id TEXT,
        sn TEXT,
        status TEXT,
        create_at TEXT,
        update_at TEXT,
        UNIQUE(license_type, number)
    )",
    "CREATE TABLE IF NOT EXISTS license_imports (
        id TEXT PRIMARY KEY,
        data TEXT NOT NULL,
        license_type TEXT,
        source TEXT,
        name TEXT,
        create_at TEXT,
        update_at TEXT
    )",
    // Indexes
    "CREATE INDEX IF NOT EXISTS idx_dev_status ON devices(status)",
    "CREATE INDEX IF NOT EXISTS idx_dev_model ON devices(model)",
    "CREATE INDEX IF NOT EXISTS idx_dev_batch ON devices(batch_id)",
    "CREATE INDEX IF NOT EXISTS idx_fw_model ON firmwares(model)",
    "CREATE INDEX IF NOT EXISTS idx_fw_status ON firmwares(status)",
    "CREATE INDEX IF NOT EXISTS idx_lic_type ON licenses(license_type)",
    "CREATE INDEX IF NOT EXISTS idx_lic_status ON licenses(status)",
    "CREATE INDEX IF NOT EXISTS idx_lic_sn ON licenses(sn)",
    "CREATE INDEX IF NOT EXISTS idx_lic_import ON licenses(import_id)",
    "CREATE INDEX IF NOT EXISTS idx_batch_status ON batches(status)",
    "CREATE INDEX IF NOT EXISTS idx_batch_model ON batches(model)",
];

pub fn init_schema(sql: &dyn SQLStore) -> Result<(), ServiceError> {
    for stmt in SCHEMA {
        sql.exec(stmt, &[])
            .map_err(|e| ServiceError::Storage(format!("schema init failed: {}", e)))?;
    }
    Ok(())
}
