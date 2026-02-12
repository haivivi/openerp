use openerp_core::{ListParams, ListResult, merge_patch, now_rfc3339};
use openerp_sql::Value;

use crate::model::{CreateRole, Role};
use crate::service::{AuthError, AuthService};

impl AuthService {
    /// Create a new role.
    pub fn create_role(&self, input: CreateRole) -> Result<Role, AuthError> {
        if input.id.is_empty() {
            return Err(AuthError::Validation("role id cannot be empty".into()));
        }
        if input.permissions.is_empty() {
            return Err(AuthError::Validation(
                "role must have at least one permission".into(),
            ));
        }

        let now = now_rfc3339();
        let role = Role {
            id: input.id,
            description: input.description,
            permissions: input.permissions,
            service: input.service.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let mut indexes: Vec<(&str, Value)> = vec![
            ("created_at", Value::Text(now.clone())),
            ("updated_at", Value::Text(now)),
        ];
        if let Some(ref svc) = role.service {
            indexes.push(("service", Value::Text(svc.clone())));
        }

        self.insert_record("roles", &role.id, &role, &indexes)?;
        Ok(role)
    }

    /// Get a role by id.
    pub fn get_role(&self, id: &str) -> Result<Role, AuthError> {
        self.get_record("roles", id)
    }

    /// List roles with pagination.
    pub fn list_roles(&self, params: &ListParams) -> Result<ListResult<Role>, AuthError> {
        let (items, total) = self.list_records("roles", &[], params.limit, params.offset)?;
        Ok(ListResult { items, total })
    }

    /// Update a role with JSON merge-patch.
    pub fn update_role(&self, id: &str, patch: serde_json::Value) -> Result<Role, AuthError> {
        let current: Role = self.get_record("roles", id)?;
        let now = now_rfc3339();

        let mut base = serde_json::to_value(&current)
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        merge_patch(&mut base, &patch);
        base["updated_at"] = serde_json::json!(now);
        base["id"] = serde_json::json!(current.id);
        base["created_at"] = serde_json::json!(current.created_at);

        let updated: Role = serde_json::from_value(base)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        let mut indexes: Vec<(&str, Value)> = vec![
            ("updated_at", Value::Text(now)),
        ];
        if let Some(ref svc) = updated.service {
            indexes.push(("service", Value::Text(svc.clone())));
        }

        self.update_record("roles", id, &updated, &indexes)?;
        Ok(updated)
    }

    /// Delete a role by id.
    pub fn delete_role(&self, id: &str) -> Result<(), AuthError> {
        // Also remove policies that reference this role
        self.sql
            .exec(
                "DELETE FROM policies WHERE how = ?1",
                &[Value::Text(id.to_string())],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        self.delete_record("roles", id)
    }

    /// Get all permissions for a role (expanding the permission list).
    pub fn get_role_permissions(&self, role_id: &str) -> Result<Vec<String>, AuthError> {
        let role: Role = self.get_record("roles", role_id)?;
        Ok(role.permissions)
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
    fn test_role_crud() {
        let svc = test_service();

        let role = svc.create_role(CreateRole {
            id: "pms:admin".to_string(),
            description: Some("PMS administrator".to_string()),
            permissions: vec![
                "pms:device:read".to_string(),
                "pms:device:write".to_string(),
                "pms:batch:create".to_string(),
            ],
            service: Some("pms".to_string()),
        }).unwrap();

        assert_eq!(role.id, "pms:admin");
        assert_eq!(role.permissions.len(), 3);

        // Get
        let fetched = svc.get_role("pms:admin").unwrap();
        assert_eq!(fetched.service, Some("pms".to_string()));

        // Update
        let updated = svc.update_role("pms:admin", serde_json::json!({
            "permissions": ["pms:device:read", "pms:device:write", "pms:batch:create", "pms:batch:delete"]
        })).unwrap();
        assert_eq!(updated.permissions.len(), 4);

        // List
        let list = svc.list_roles(&ListParams::default()).unwrap();
        assert_eq!(list.total, 1);

        // Get permissions
        let perms = svc.get_role_permissions("pms:admin").unwrap();
        assert_eq!(perms.len(), 4);

        // Delete
        svc.delete_role("pms:admin").unwrap();
        assert!(svc.get_role("pms:admin").is_err());
    }

    #[test]
    fn test_role_validation() {
        let svc = test_service();

        // Empty id
        let result = svc.create_role(CreateRole {
            id: "".to_string(),
            description: None,
            permissions: vec!["read".to_string()],
            service: None,
        });
        assert!(result.is_err());

        // Empty permissions
        let result = svc.create_role(CreateRole {
            id: "test".to_string(),
            description: None,
            permissions: vec![],
            service: None,
        });
        assert!(result.is_err());
    }
}
