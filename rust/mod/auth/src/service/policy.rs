use openerp_core::{ListParams, ListResult, now_rfc3339};
use openerp_sql::Value;

use crate::model::{CheckParams, CheckResult, CreatePolicy, Policy, PolicyQuery, Role, policy_id};
use crate::service::{AuthError, AuthService};

impl AuthService {
    /// Create or upsert a policy.
    /// Same (who, what, how) triple → update expiration instead of creating duplicate.
    pub fn create_policy(&self, input: CreatePolicy) -> Result<Policy, AuthError> {
        if input.who.is_empty() {
            return Err(AuthError::Validation("policy 'who' cannot be empty".into()));
        }
        if input.how.is_empty() {
            return Err(AuthError::Validation("policy 'how' cannot be empty".into()));
        }

        // Validate that the role exists
        let _role: Role = self.get_record("roles", &input.how)
            .map_err(|_| AuthError::Validation(format!("role '{}' does not exist", input.how)))?;

        let id = policy_id(&input.who, &input.what, &input.how);
        let now = now_rfc3339();

        // Check if policy already exists (upsert)
        if let Ok(mut existing) = self.get_record::<Policy>("policies", &id) {
            existing.expires_at = input.expires_at;
            existing.updated_at = now.clone();

            self.update_record(
                "policies",
                &id,
                &existing,
                &[
                    ("expires_at", match &existing.expires_at {
                        Some(e) => Value::Text(e.clone()),
                        None => Value::Null,
                    }),
                    ("updated_at", Value::Text(now)),
                ],
            )?;
            return Ok(existing);
        }

        let policy = Policy {
            id: id.clone(),
            who: input.who,
            what: input.what,
            how: input.how,
            expires_at: input.expires_at.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        self.insert_record(
            "policies",
            &id,
            &policy,
            &[
                ("who", Value::Text(policy.who.clone())),
                ("what", Value::Text(policy.what.clone())),
                ("how", Value::Text(policy.how.clone())),
                ("expires_at", match &policy.expires_at {
                    Some(e) => Value::Text(e.clone()),
                    None => Value::Null,
                }),
                ("created_at", Value::Text(now.clone())),
                ("updated_at", Value::Text(now)),
            ],
        )?;

        Ok(policy)
    }

    /// Get a policy by id.
    pub fn get_policy(&self, id: &str) -> Result<Policy, AuthError> {
        self.get_record("policies", id)
    }

    /// List policies with pagination.
    pub fn list_policies(&self, params: &ListParams) -> Result<ListResult<Policy>, AuthError> {
        let (items, total) = self.list_records("policies", &[], params.limit, params.offset)?;
        Ok(ListResult { items, total })
    }

    /// Query policies by who/what/how filters.
    pub fn query_policies(&self, query: &PolicyQuery) -> Result<Vec<Policy>, AuthError> {
        let mut where_clauses = Vec::new();
        let mut params = Vec::new();
        let mut idx = 1;

        if let Some(ref who) = query.who {
            where_clauses.push(format!("who = ?{}", idx));
            params.push(Value::Text(who.clone()));
            idx += 1;
        }
        if let Some(ref what) = query.what {
            where_clauses.push(format!("what = ?{}", idx));
            params.push(Value::Text(what.clone()));
            idx += 1;
        }
        if let Some(ref how) = query.how {
            where_clauses.push(format!("how = ?{}", idx));
            params.push(Value::Text(how.clone()));
            let _ = idx; // suppress unused warning
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        let sql = format!("SELECT data FROM policies{} ORDER BY created_at DESC", where_sql);
        let rows = self.sql
            .query(&sql, &params)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let mut policies = Vec::new();
        for row in &rows {
            if let Some(data) = row.get_str("data") {
                let policy: Policy =
                    serde_json::from_str(data).map_err(|e| AuthError::Internal(e.to_string()))?;
                // Filter out expired policies
                if !is_expired(&policy) {
                    policies.push(policy);
                }
            }
        }

        Ok(policies)
    }

    /// Delete a policy by id.
    pub fn delete_policy(&self, id: &str) -> Result<(), AuthError> {
        self.delete_record("policies", id)
    }

    /// Delete a policy by (who, what, how) triple.
    pub fn delete_policy_by_triple(
        &self,
        who: &str,
        what: &str,
        how: &str,
    ) -> Result<(), AuthError> {
        let id = policy_id(who, what, how);
        self.delete_record("policies", &id)
    }

    /// Check if a subject has a specific permission.
    ///
    /// Flow:
    /// 1. Expand user's groups (including ancestors)
    /// 2. Find matching policies for all identities (user + groups)
    /// 3. Check path-based `what` matching (more specific wins)
    /// 4. Verify role contains the requested permission
    pub fn check_permission(&self, params: &CheckParams) -> Result<CheckResult, AuthError> {
        // Build the list of "who" identities to check
        let mut identities = vec![params.who.clone()];

        // If subject is a user, expand their groups
        if let Some(user_id) = params.who.strip_prefix("user:") {
            let groups = self.expand_user_groups(user_id)?;
            for group_name in &groups {
                identities.push(format!("group:{}", group_name));
            }
        }

        // Build `what` path prefixes to check (from most specific to global)
        // e.g. "pms:batch:B001" → ["pms:batch:B001", "pms:batch", "pms", ""]
        let what_paths = build_what_paths(&params.what);

        // Query all policies for these identities
        let placeholders: Vec<String> = identities
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT data FROM policies WHERE who IN ({})",
            placeholders.join(", ")
        );
        let sql_params: Vec<Value> = identities
            .iter()
            .map(|id| Value::Text(id.clone()))
            .collect();

        let rows = self.sql
            .query(&sql, &sql_params)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Check each policy
        for row in &rows {
            if let Some(data) = row.get_str("data") {
                if let Ok(policy) = serde_json::from_str::<Policy>(data) {
                    // Skip expired
                    if is_expired(&policy) {
                        continue;
                    }

                    // Check if policy's `what` matches any of our path prefixes
                    if !what_paths.contains(&policy.what) {
                        continue;
                    }

                    // Check if the role grants the requested permission
                    if self.role_grants_permission(&policy.how, &params.how)? {
                        return Ok(CheckResult {
                            allowed: true,
                            policy_id: Some(policy.id),
                        });
                    }
                }
            }
        }

        Ok(CheckResult {
            allowed: false,
            policy_id: None,
        })
    }

    /// Check if a role (by id) grants a specific permission.
    /// The `how` in a policy is a role id. We expand it to permissions
    /// and check if any matches the requested permission.
    fn role_grants_permission(&self, role_id: &str, permission: &str) -> Result<bool, AuthError> {
        // Direct match: if how == permission (role id == permission string)
        if role_id == permission {
            return Ok(true);
        }

        // Expand role to permissions
        if let Ok(role) = self.get_record::<Role>("roles", role_id) {
            for perm in &role.permissions {
                if perm == permission {
                    return Ok(true);
                }
                // Wildcard matching: "pms:device:*" matches "pms:device:read"
                if perm.ends_with(":*") {
                    let prefix = &perm[..perm.len() - 1]; // "pms:device:"
                    if permission.starts_with(prefix) {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Get all roles assigned to a user (through policies).
    pub fn get_user_roles(&self, user_id: &str) -> Result<Vec<String>, AuthError> {
        let mut identities = vec![format!("user:{}", user_id)];

        let groups = self.expand_user_groups(user_id)?;
        for group_name in &groups {
            identities.push(format!("group:{}", group_name));
        }

        let placeholders: Vec<String> = identities
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT DISTINCT how FROM policies WHERE who IN ({})",
            placeholders.join(", ")
        );
        let sql_params: Vec<Value> = identities
            .iter()
            .map(|id| Value::Text(id.clone()))
            .collect();

        let rows = self.sql
            .query(&sql, &sql_params)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let mut roles = Vec::new();
        for row in &rows {
            if let Some(how) = row.get_str("how") {
                roles.push(how.to_string());
            }
        }

        Ok(roles)
    }

    /// Get all permissions for a user (expanding roles).
    pub fn get_user_permissions(&self, user_id: &str) -> Result<Vec<String>, AuthError> {
        let role_ids = self.get_user_roles(user_id)?;
        let mut permissions = std::collections::HashSet::new();

        for role_id in &role_ids {
            if let Ok(role) = self.get_record::<Role>("roles", role_id) {
                for perm in &role.permissions {
                    permissions.insert(perm.clone());
                }
            }
        }

        let mut result: Vec<String> = permissions.into_iter().collect();
        result.sort();
        Ok(result)
    }
}

/// Check if a policy has expired.
fn is_expired(policy: &Policy) -> bool {
    if let Some(ref expires_at) = policy.expires_at {
        if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(expires_at) {
            return expires < chrono::Utc::now();
        }
    }
    false
}

/// Build the path prefixes for `what` matching.
/// "pms:batch:B001" → ["pms:batch:B001", "pms:batch", "pms", ""]
fn build_what_paths(what: &str) -> Vec<String> {
    let mut paths = vec![what.to_string()];
    let mut current = what.to_string();
    while let Some(pos) = current.rfind(':') {
        current = current[..pos].to_string();
        paths.push(current.clone());
    }
    if !what.is_empty() {
        paths.push(String::new()); // Global/empty path
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AddGroupMember, CreateGroup, CreateRole, CreateUser};
    use crate::service::AuthConfig;
    use openerp_sql::sqlite::SqliteStore;

    fn test_service() -> std::sync::Arc<AuthService> {
        let sql = Box::new(SqliteStore::open_in_memory().unwrap());
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let kv = Box::new(openerp_kv::redb::RedbStore::open(tmp.path()).unwrap());
        AuthService::new(sql, kv, AuthConfig::default()).unwrap()
    }

    #[test]
    fn test_policy_crud() {
        let svc = test_service();

        // Create role first
        svc.create_role(CreateRole {
            id: "pms:admin".to_string(),
            description: None,
            permissions: vec!["pms:device:read".to_string(), "pms:device:write".to_string()],
            service: Some("pms".to_string()),
        }).unwrap();

        // Create policy
        let policy = svc.create_policy(CreatePolicy {
            who: "user:alice".to_string(),
            what: "pms:batch".to_string(),
            how: "pms:admin".to_string(),
            expires_at: None,
        }).unwrap();
        assert_eq!(policy.who, "user:alice");

        // Upsert same triple → updates expiration
        let policy2 = svc.create_policy(CreatePolicy {
            who: "user:alice".to_string(),
            what: "pms:batch".to_string(),
            how: "pms:admin".to_string(),
            expires_at: Some("2030-01-01T00:00:00Z".to_string()),
        }).unwrap();
        assert_eq!(policy2.id, policy.id);
        assert_eq!(policy2.expires_at, Some("2030-01-01T00:00:00Z".to_string()));

        // List
        let list = svc.list_policies(&ListParams::default()).unwrap();
        assert_eq!(list.total, 1);

        // Query
        let queried = svc.query_policies(&PolicyQuery {
            who: Some("user:alice".to_string()),
            what: None,
            how: None,
        }).unwrap();
        assert_eq!(queried.len(), 1);

        // Delete by triple
        svc.delete_policy_by_triple("user:alice", "pms:batch", "pms:admin").unwrap();
        let list = svc.list_policies(&ListParams::default()).unwrap();
        assert_eq!(list.total, 0);
    }

    #[test]
    fn test_permission_check_direct_user() {
        let svc = test_service();

        // Setup: role + policy for user:alice
        svc.create_role(CreateRole {
            id: "pms:admin".to_string(),
            description: None,
            permissions: vec!["pms:device:read".to_string(), "pms:device:write".to_string()],
            service: Some("pms".to_string()),
        }).unwrap();

        // Create user so expand_user_groups works
        let alice = svc.create_user(CreateUser {
            name: "Alice".to_string(),
            email: None,
            avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        svc.create_policy(CreatePolicy {
            who: format!("user:{}", alice.id),
            what: "pms".to_string(),
            how: "pms:admin".to_string(),
            expires_at: None,
        }).unwrap();

        // Check: alice can read pms:device
        let result = svc.check_permission(&CheckParams {
            who: format!("user:{}", alice.id),
            what: "pms:device".to_string(),
            how: "pms:device:read".to_string(),
        }).unwrap();
        assert!(result.allowed);

        // Check: alice cannot do something not in role
        let result = svc.check_permission(&CheckParams {
            who: format!("user:{}", alice.id),
            what: "pms:device".to_string(),
            how: "pms:device:delete".to_string(),
        }).unwrap();
        assert!(!result.allowed);
    }

    #[test]
    fn test_permission_check_via_group() {
        let svc = test_service();

        // Setup: role + group + policy for group:engineering
        svc.create_role(CreateRole {
            id: "release:viewer".to_string(),
            description: None,
            permissions: vec!["release:view".to_string()],
            service: Some("release".to_string()),
        }).unwrap();

        let group = svc.create_group(CreateGroup {
            name: "engineering".to_string(),
            description: None,
            parent_id: None,
            external_source: None,
            external_id: None,
        }).unwrap();

        let user = svc.create_user(CreateUser {
            name: "Bob".to_string(),
            email: None,
            avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        svc.add_group_member(&group.id, AddGroupMember {
            member_ref: format!("user:{}", user.id),
        }).unwrap();

        svc.create_policy(CreatePolicy {
            who: format!("group:{}", group.id),
            what: String::new(),
            how: "release:viewer".to_string(),
            expires_at: None,
        }).unwrap();

        // Check: bob gets permission via group membership
        let result = svc.check_permission(&CheckParams {
            who: format!("user:{}", user.id),
            what: "release:any".to_string(),
            how: "release:view".to_string(),
        }).unwrap();
        assert!(result.allowed);
    }

    #[test]
    fn test_what_path_matching() {
        let paths = build_what_paths("pms:batch:B001");
        assert_eq!(paths, vec![
            "pms:batch:B001".to_string(),
            "pms:batch".to_string(),
            "pms".to_string(),
            "".to_string(),
        ]);

        let paths = build_what_paths("");
        assert_eq!(paths, vec!["".to_string()]);
    }

    #[test]
    fn test_wildcard_permissions() {
        let svc = test_service();

        svc.create_role(CreateRole {
            id: "pms:full".to_string(),
            description: None,
            permissions: vec!["pms:device:*".to_string()],
            service: Some("pms".to_string()),
        }).unwrap();

        let user = svc.create_user(CreateUser {
            name: "Charlie".to_string(),
            email: None, avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        svc.create_policy(CreatePolicy {
            who: format!("user:{}", user.id),
            what: String::new(),
            how: "pms:full".to_string(),
            expires_at: None,
        }).unwrap();

        // Wildcard "pms:device:*" should match "pms:device:read"
        let result = svc.check_permission(&CheckParams {
            who: format!("user:{}", user.id),
            what: "pms:device".to_string(),
            how: "pms:device:read".to_string(),
        }).unwrap();
        assert!(result.allowed);

        // But not "pms:batch:read"
        let result = svc.check_permission(&CheckParams {
            who: format!("user:{}", user.id),
            what: "pms:batch".to_string(),
            how: "pms:batch:read".to_string(),
        }).unwrap();
        assert!(!result.allowed);
    }

    #[test]
    fn test_user_roles_and_permissions() {
        let svc = test_service();

        svc.create_role(CreateRole {
            id: "pms:admin".to_string(),
            description: None,
            permissions: vec!["pms:device:read".to_string(), "pms:device:write".to_string()],
            service: Some("pms".to_string()),
        }).unwrap();

        let user = svc.create_user(CreateUser {
            name: "Dave".to_string(),
            email: None, avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        svc.create_policy(CreatePolicy {
            who: format!("user:{}", user.id),
            what: String::new(),
            how: "pms:admin".to_string(),
            expires_at: None,
        }).unwrap();

        let roles = svc.get_user_roles(&user.id).unwrap();
        assert_eq!(roles, vec!["pms:admin"]);

        let perms = svc.get_user_permissions(&user.id).unwrap();
        assert!(perms.contains(&"pms:device:read".to_string()));
        assert!(perms.contains(&"pms:device:write".to_string()));
    }
}
