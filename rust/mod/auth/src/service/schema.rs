use openerp_sql::SQLStore;

use crate::service::AuthError;

/// Initialize the SQLite schema for all auth resources.
pub fn init_schema(sql: &dyn SQLStore) -> Result<(), AuthError> {
    let statements = [
        // Users table: core identity
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT,
            active INTEGER NOT NULL DEFAULT 1,
            data TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        "CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)",
        "CREATE INDEX IF NOT EXISTS idx_users_name ON users(name)",

        // Groups table: hierarchical org units
        "CREATE TABLE IF NOT EXISTS groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            parent_id TEXT,
            data TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (parent_id) REFERENCES groups(id)
        )",
        "CREATE INDEX IF NOT EXISTS idx_groups_parent ON groups(parent_id)",
        "CREATE INDEX IF NOT EXISTS idx_groups_name ON groups(name)",

        // Group members: user/group membership in a group
        "CREATE TABLE IF NOT EXISTS group_members (
            group_id TEXT NOT NULL,
            member_ref TEXT NOT NULL,
            added_at TEXT NOT NULL,
            PRIMARY KEY (group_id, member_ref),
            FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
        )",
        "CREATE INDEX IF NOT EXISTS idx_group_members_ref ON group_members(member_ref)",

        // Providers table: OAuth provider configuration
        "CREATE TABLE IF NOT EXISTS providers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            data TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",

        // Roles table: permission sets
        "CREATE TABLE IF NOT EXISTS roles (
            id TEXT PRIMARY KEY,
            service TEXT,
            data TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        "CREATE INDEX IF NOT EXISTS idx_roles_service ON roles(service)",

        // Policies table: ACL entries
        "CREATE TABLE IF NOT EXISTS policies (
            id TEXT PRIMARY KEY,
            who TEXT NOT NULL,
            what TEXT NOT NULL DEFAULT '',
            how TEXT NOT NULL,
            expires_at TEXT,
            data TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        "CREATE INDEX IF NOT EXISTS idx_policies_who ON policies(who)",
        "CREATE INDEX IF NOT EXISTS idx_policies_what ON policies(what)",
        "CREATE INDEX IF NOT EXISTS idx_policies_how ON policies(how)",

        // Sessions table: JWT issuance records
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            revoked INTEGER NOT NULL DEFAULT 0,
            data TEXT NOT NULL,
            issued_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        "CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id)",
    ];

    for stmt in &statements {
        sql.exec(stmt, &[])
            .map_err(|e| AuthError::Storage(e.to_string()))?;
    }

    Ok(())
}
