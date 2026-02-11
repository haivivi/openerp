use openerp_core::{ListParams, ListResult, merge_patch, new_id, now_rfc3339};
use openerp_sql::Value;

use crate::model::{CreateUser, User};
use crate::service::{AuthError, AuthService};

impl AuthService {
    /// Create a new user.
    pub fn create_user(&self, input: CreateUser) -> Result<User, AuthError> {
        let now = now_rfc3339();
        let user = User {
            id: new_id(),
            name: input.name,
            email: input.email.clone(),
            avatar: input.avatar,
            active: true,
            linked_accounts: input.linked_accounts,
            metadata: input.metadata,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let mut indexes: Vec<(&str, Value)> = vec![
            ("name", Value::Text(user.name.clone())),
            ("created_at", Value::Text(now.clone())),
            ("updated_at", Value::Text(now)),
            ("active", Value::Integer(1)),
        ];
        if let Some(ref email) = user.email {
            indexes.push(("email", Value::Text(email.clone())));
        }

        self.insert_record("users", &user.id, &user, &indexes)?;
        Ok(user)
    }

    /// Get a user by id.
    pub fn get_user(&self, id: &str) -> Result<User, AuthError> {
        self.get_record("users", id)
    }

    /// List users with pagination.
    pub fn list_users(&self, params: &ListParams) -> Result<ListResult<User>, AuthError> {
        let (items, total) = self.list_records("users", &[], params.limit, params.offset)?;
        Ok(ListResult { items, total })
    }

    /// Update a user with JSON merge-patch semantics.
    pub fn update_user(&self, id: &str, patch: serde_json::Value) -> Result<User, AuthError> {
        let mut current: User = self.get_record("users", id)?;
        let now = now_rfc3339();

        let mut base = serde_json::to_value(&current)
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        merge_patch(&mut base, &patch);
        // Force updated_at and preserve id/created_at
        base["updated_at"] = serde_json::json!(now);
        base["id"] = serde_json::json!(current.id);
        base["created_at"] = serde_json::json!(current.created_at);

        current = serde_json::from_value(base)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        let mut indexes: Vec<(&str, Value)> = vec![
            ("name", Value::Text(current.name.clone())),
            ("updated_at", Value::Text(now)),
            ("active", Value::Integer(if current.active { 1 } else { 0 })),
        ];
        if let Some(ref email) = current.email {
            indexes.push(("email", Value::Text(email.clone())));
        }

        self.update_record("users", id, &current, &indexes)?;
        Ok(current)
    }

    /// Delete a user by id.
    pub fn delete_user(&self, id: &str) -> Result<(), AuthError> {
        // Also clean up group memberships
        self.sql
            .exec(
                "DELETE FROM group_members WHERE member_ref = ?1",
                &[Value::Text(format!("user:{}", id))],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Clean up sessions
        self.sql
            .exec(
                "DELETE FROM sessions WHERE user_id = ?1",
                &[Value::Text(id.to_string())],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Clean up policies referencing this user
        self.sql
            .exec(
                "DELETE FROM policies WHERE who = ?1",
                &[Value::Text(format!("user:{}", id))],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        self.delete_record("users", id)
    }

    /// Find a user by linked account (provider_id, external_user_id).
    /// Used during OAuth callback to find or create a user.
    pub fn find_user_by_linked_account(
        &self,
        provider_id: &str,
        external_id: &str,
    ) -> Result<Option<User>, AuthError> {
        // Scan all users and check linked_accounts.
        // For small-to-medium user bases this is fine; at scale we'd add an index table.
        let rows = self.sql
            .query("SELECT data FROM users WHERE active = 1", &[])
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        for row in &rows {
            if let Some(data) = row.get_str("data") {
                if let Ok(user) = serde_json::from_str::<User>(data) {
                    if user.linked_accounts.get(provider_id).map(|s| s.as_str()) == Some(external_id)
                    {
                        return Ok(Some(user));
                    }
                }
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::AuthConfig;
    use openerp_sql::sqlite::SqliteStore;

    fn test_service() -> std::sync::Arc<AuthService> {
        let sql = Box::new(SqliteStore::open_in_memory().unwrap());
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let kv = Box::new(openerp_kv::redb::RedbStore::open(tmp.path()).unwrap());
        AuthService::new(sql, kv, AuthConfig::default()).unwrap()
    }

    #[test]
    fn test_user_crud() {
        let svc = test_service();

        // Create
        let user = svc.create_user(CreateUser {
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
            avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();
        assert_eq!(user.name, "Alice");
        assert!(user.active);

        // Get
        let fetched = svc.get_user(&user.id).unwrap();
        assert_eq!(fetched.email, Some("alice@example.com".to_string()));

        // Update
        let updated = svc.update_user(&user.id, serde_json::json!({"name": "Alice W."})).unwrap();
        assert_eq!(updated.name, "Alice W.");
        assert_eq!(updated.id, user.id);

        // List
        let list = svc.list_users(&ListParams::default()).unwrap();
        assert_eq!(list.total, 1);
        assert_eq!(list.items[0].name, "Alice W.");

        // Delete
        svc.delete_user(&user.id).unwrap();
        assert!(svc.get_user(&user.id).is_err());
    }

    #[test]
    fn test_find_by_linked_account() {
        let svc = test_service();

        let mut accounts = std::collections::HashMap::new();
        accounts.insert("github".to_string(), "gh-12345".to_string());

        let user = svc.create_user(CreateUser {
            name: "Bob".to_string(),
            email: None,
            avatar: None,
            linked_accounts: accounts,
            metadata: None,
        }).unwrap();

        let found = svc.find_user_by_linked_account("github", "gh-12345").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, user.id);

        let not_found = svc.find_user_by_linked_account("github", "unknown").unwrap();
        assert!(not_found.is_none());
    }
}
