use std::collections::HashSet;

use openerp_core::{ListParams, ListResult, merge_patch, new_id, now_rfc3339};
use openerp_sql::Value;

use crate::model::{AddGroupMember, CreateGroup, Group, GroupMember};
use crate::service::{AuthError, AuthService};

impl AuthService {
    /// Create a new group.
    pub fn create_group(&self, input: CreateGroup) -> Result<Group, AuthError> {
        // Validate parent exists if specified
        if let Some(ref parent_id) = input.parent_id {
            let _parent: Group = self.get_record("groups", parent_id)?;
        }

        let now = now_rfc3339();
        let group = Group {
            id: new_id(),
            name: input.name,
            description: input.description,
            parent_id: input.parent_id.clone(),
            external_source: input.external_source,
            external_id: input.external_id,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let mut indexes: Vec<(&str, Value)> = vec![
            ("name", Value::Text(group.name.clone())),
            ("created_at", Value::Text(now.clone())),
            ("updated_at", Value::Text(now)),
        ];
        if let Some(ref pid) = group.parent_id {
            indexes.push(("parent_id", Value::Text(pid.clone())));
        }

        self.insert_record("groups", &group.id, &group, &indexes)?;
        Ok(group)
    }

    /// Get a group by id.
    pub fn get_group(&self, id: &str) -> Result<Group, AuthError> {
        self.get_record("groups", id)
    }

    /// List groups with pagination.
    pub fn list_groups(&self, params: &ListParams) -> Result<ListResult<Group>, AuthError> {
        let (items, total) = self.list_records("groups", &[], params.limit, params.offset)?;
        Ok(ListResult { items, total })
    }

    /// Update a group with JSON merge-patch.
    pub fn update_group(&self, id: &str, patch: serde_json::Value) -> Result<Group, AuthError> {
        let current: Group = self.get_record("groups", id)?;
        let now = now_rfc3339();

        // If parent_id is being changed, check for cycles
        if let Some(new_parent) = patch.get("parent_id").and_then(|v| v.as_str()) {
            if !new_parent.is_empty() {
                self.check_cycle(id, new_parent)?;
            }
        }

        let mut base = serde_json::to_value(&current)
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        merge_patch(&mut base, &patch);
        base["updated_at"] = serde_json::json!(now);
        base["id"] = serde_json::json!(current.id);
        base["created_at"] = serde_json::json!(current.created_at);

        let updated: Group = serde_json::from_value(base)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        let mut indexes: Vec<(&str, Value)> = vec![
            ("name", Value::Text(updated.name.clone())),
            ("updated_at", Value::Text(now)),
        ];
        if let Some(ref pid) = updated.parent_id {
            indexes.push(("parent_id", Value::Text(pid.clone())));
        }

        self.update_record("groups", id, &updated, &indexes)?;

        // Invalidate group caches since hierarchy changed
        self.group_cache.invalidate_all();

        Ok(updated)
    }

    /// Delete a group by id.
    pub fn delete_group(&self, id: &str) -> Result<(), AuthError> {
        // Remove all memberships in this group
        self.sql
            .exec(
                "DELETE FROM group_members WHERE group_id = ?1",
                &[Value::Text(id.to_string())],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Remove membership references to this group from other groups
        self.sql
            .exec(
                "DELETE FROM group_members WHERE member_ref = ?1",
                &[Value::Text(format!("group:{}", id))],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Remove policies referencing this group
        self.sql
            .exec(
                "DELETE FROM policies WHERE who = ?1",
                &[Value::Text(format!("group:{}", id))],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Orphan child groups (set parent_id to NULL)
        self.sql
            .exec(
                "UPDATE groups SET parent_id = NULL WHERE parent_id = ?1",
                &[Value::Text(id.to_string())],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        self.delete_record("groups", id)?;
        self.group_cache.invalidate_all();
        Ok(())
    }

    // ── Group Members ──

    /// Add a member (user or sub-group) to a group.
    pub fn add_group_member(
        &self,
        group_id: &str,
        input: AddGroupMember,
    ) -> Result<GroupMember, AuthError> {
        // Validate group exists
        let _group: Group = self.get_record("groups", group_id)?;

        // Validate member ref
        let (kind, ref_id) = parse_member_ref(&input.member_ref)?;
        match kind {
            "user" => {
                let _: crate::model::User = self.get_record("users", ref_id)?;
            }
            "group" => {
                let _: Group = self.get_record("groups", ref_id)?;
                // Cycle detection: adding group X as member of group Y
                // must not create a cycle. Check if group_id is a descendant of ref_id.
                self.check_cycle(ref_id, group_id)?;
            }
            _ => {
                return Err(AuthError::Validation(format!(
                    "invalid member_ref kind: {}, expected 'user:' or 'group:'",
                    kind
                )));
            }
        }

        let now = now_rfc3339();
        let member = GroupMember {
            group_id: group_id.to_string(),
            member_ref: input.member_ref,
            added_at: now.clone(),
        };

        self.sql
            .exec(
                "INSERT OR IGNORE INTO group_members (group_id, member_ref, added_at) VALUES (?1, ?2, ?3)",
                &[
                    Value::Text(member.group_id.clone()),
                    Value::Text(member.member_ref.clone()),
                    Value::Text(now),
                ],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Invalidate caches
        self.group_cache.invalidate_all();

        Ok(member)
    }

    /// Remove a member from a group.
    pub fn remove_group_member(&self, group_id: &str, member_ref: &str) -> Result<(), AuthError> {
        let affected = self.sql
            .exec(
                "DELETE FROM group_members WHERE group_id = ?1 AND member_ref = ?2",
                &[
                    Value::Text(group_id.to_string()),
                    Value::Text(member_ref.to_string()),
                ],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        if affected == 0 {
            return Err(AuthError::NotFound(format!(
                "group_members/{}/{}",
                group_id, member_ref
            )));
        }

        self.group_cache.invalidate_all();
        Ok(())
    }

    /// List members of a group.
    pub fn list_group_members(&self, group_id: &str) -> Result<Vec<GroupMember>, AuthError> {
        // Validate group exists
        let _: Group = self.get_record("groups", group_id)?;

        let rows = self.sql
            .query(
                "SELECT group_id, member_ref, added_at FROM group_members WHERE group_id = ?1 ORDER BY added_at",
                &[Value::Text(group_id.to_string())],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let mut members = Vec::new();
        for row in &rows {
            members.push(GroupMember {
                group_id: row.get_str("group_id").unwrap_or_default().to_string(),
                member_ref: row.get_str("member_ref").unwrap_or_default().to_string(),
                added_at: row.get_str("added_at").unwrap_or_default().to_string(),
            });
        }
        Ok(members)
    }

    // ── Cycle Detection ──

    /// Check if making `parent_candidate` a parent/ancestor of `child_id` would
    /// create a cycle. Uses DFS up the parent chain from `parent_candidate`.
    fn check_cycle(&self, child_id: &str, parent_candidate: &str) -> Result<(), AuthError> {
        if child_id == parent_candidate {
            return Err(AuthError::Validation(
                "cycle detected: a group cannot be its own parent".to_string(),
            ));
        }

        let mut visited = HashSet::new();
        visited.insert(child_id.to_string());

        let mut current = parent_candidate.to_string();
        loop {
            if visited.contains(&current) {
                return Err(AuthError::Validation(format!(
                    "cycle detected: adding this relationship would create a loop through group {}",
                    current
                )));
            }
            visited.insert(current.clone());

            // Get the parent of `current`
            let rows = self.sql
                .query(
                    "SELECT parent_id FROM groups WHERE id = ?1",
                    &[Value::Text(current.clone())],
                )
                .map_err(|e| AuthError::Storage(e.to_string()))?;

            match rows.first().and_then(|r| r.get_str("parent_id")) {
                Some(pid) if !pid.is_empty() => {
                    current = pid.to_string();
                }
                _ => break, // Reached root, no cycle
            }
        }

        // Also check group membership links (group:X as member of group:Y)
        // to detect indirect cycles through membership.
        let mut member_visited = HashSet::new();
        member_visited.insert(child_id.to_string());
        self.check_membership_cycle(parent_candidate, &mut member_visited)?;

        Ok(())
    }

    /// DFS through group membership to detect cycles.
    fn check_membership_cycle(
        &self,
        group_id: &str,
        visited: &mut HashSet<String>,
    ) -> Result<(), AuthError> {
        if visited.contains(group_id) {
            return Err(AuthError::Validation(format!(
                "cycle detected: membership chain loops through group {}",
                group_id
            )));
        }
        visited.insert(group_id.to_string());

        // Find all groups that this group is a member of
        let member_ref = format!("group:{}", group_id);
        let rows = self.sql
            .query(
                "SELECT group_id FROM group_members WHERE member_ref = ?1",
                &[Value::Text(member_ref)],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        for row in &rows {
            if let Some(parent_group_id) = row.get_str("group_id") {
                self.check_membership_cycle(parent_group_id, visited)?;
            }
        }

        Ok(())
    }

    /// Get all groups a user directly belongs to.
    pub fn get_user_direct_groups(&self, user_id: &str) -> Result<Vec<Group>, AuthError> {
        let member_ref = format!("user:{}", user_id);
        let rows = self.sql
            .query(
                "SELECT group_id FROM group_members WHERE member_ref = ?1",
                &[Value::Text(member_ref)],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let mut groups = Vec::new();
        for row in &rows {
            if let Some(gid) = row.get_str("group_id") {
                if let Ok(group) = self.get_record::<Group>("groups", gid) {
                    groups.push(group);
                }
            }
        }
        Ok(groups)
    }

    /// Get all ancestor group ids for a given group (walking up parent_id chain).
    pub fn get_ancestor_group_ids(&self, group_id: &str) -> Result<Vec<String>, AuthError> {
        let mut ancestors = Vec::new();
        let mut current = group_id.to_string();
        let mut visited = HashSet::new();

        loop {
            if visited.contains(&current) {
                break; // Safety: avoid infinite loop if data is corrupted
            }
            visited.insert(current.clone());

            let rows = self.sql
                .query(
                    "SELECT parent_id FROM groups WHERE id = ?1",
                    &[Value::Text(current.clone())],
                )
                .map_err(|e| AuthError::Storage(e.to_string()))?;

            match rows.first().and_then(|r| r.get_str("parent_id")) {
                Some(pid) if !pid.is_empty() => {
                    ancestors.push(pid.to_string());
                    current = pid.to_string();
                }
                _ => break,
            }
        }

        Ok(ancestors)
    }
}

/// Parse a member reference like "user:abc123" or "group:def456".
fn parse_member_ref(member_ref: &str) -> Result<(&str, &str), AuthError> {
    match member_ref.split_once(':') {
        Some((kind, id)) if !id.is_empty() => Ok((kind, id)),
        _ => Err(AuthError::Validation(format!(
            "invalid member_ref format: '{}', expected 'user:{{id}}' or 'group:{{id}}'",
            member_ref
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CreateUser;
    use crate::service::AuthConfig;
    use openerp_sql::sqlite::SqliteStore;

    fn test_service() -> std::sync::Arc<AuthService> {
        let sql = Box::new(SqliteStore::open_in_memory().unwrap());
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let kv = Box::new(openerp_kv::redb::RedbStore::open(tmp.path()).unwrap());
        AuthService::new(sql, kv, AuthConfig::default()).unwrap()
    }

    #[test]
    fn test_group_crud() {
        let svc = test_service();

        // Create top-level group
        let g1 = svc.create_group(CreateGroup {
            name: "Engineering".to_string(),
            description: Some("Engineering department".to_string()),
            parent_id: None,
            external_source: None,
            external_id: None,
        }).unwrap();

        // Create child group
        let g2 = svc.create_group(CreateGroup {
            name: "Firmware".to_string(),
            description: None,
            parent_id: Some(g1.id.clone()),
            external_source: None,
            external_id: None,
        }).unwrap();
        assert_eq!(g2.parent_id, Some(g1.id.clone()));

        // List
        let list = svc.list_groups(&ListParams::default()).unwrap();
        assert_eq!(list.total, 2);

        // Update
        let updated = svc.update_group(&g2.id, serde_json::json!({"name": "Firmware Team"})).unwrap();
        assert_eq!(updated.name, "Firmware Team");

        // Delete
        svc.delete_group(&g2.id).unwrap();
        let list = svc.list_groups(&ListParams::default()).unwrap();
        assert_eq!(list.total, 1);
    }

    #[test]
    fn test_group_members() {
        let svc = test_service();

        let group = svc.create_group(CreateGroup {
            name: "Team".to_string(),
            description: None,
            parent_id: None,
            external_source: None,
            external_id: None,
        }).unwrap();

        let user = svc.create_user(CreateUser {
            name: "Alice".to_string(),
            email: None,
            avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        // Add member
        let member = svc.add_group_member(&group.id, AddGroupMember {
            member_ref: format!("user:{}", user.id),
        }).unwrap();
        assert_eq!(member.group_id, group.id);

        // List members
        let members = svc.list_group_members(&group.id).unwrap();
        assert_eq!(members.len(), 1);

        // Get user's groups
        let user_groups = svc.get_user_direct_groups(&user.id).unwrap();
        assert_eq!(user_groups.len(), 1);
        assert_eq!(user_groups[0].id, group.id);

        // Remove member
        svc.remove_group_member(&group.id, &format!("user:{}", user.id)).unwrap();
        let members = svc.list_group_members(&group.id).unwrap();
        assert_eq!(members.len(), 0);
    }

    #[test]
    fn test_cycle_detection_self() {
        let svc = test_service();

        let group = svc.create_group(CreateGroup {
            name: "Self".to_string(),
            description: None,
            parent_id: None,
            external_source: None,
            external_id: None,
        }).unwrap();

        // Try to set parent to self
        let result = svc.update_group(&group.id, serde_json::json!({"parent_id": group.id}));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cycle"));
    }

    #[test]
    fn test_cycle_detection_indirect() {
        let svc = test_service();

        // A -> B -> C, then try C -> A (cycle)
        let a = svc.create_group(CreateGroup {
            name: "A".to_string(),
            description: None,
            parent_id: None,
            external_source: None,
            external_id: None,
        }).unwrap();

        let b = svc.create_group(CreateGroup {
            name: "B".to_string(),
            description: None,
            parent_id: Some(a.id.clone()),
            external_source: None,
            external_id: None,
        }).unwrap();

        let c = svc.create_group(CreateGroup {
            name: "C".to_string(),
            description: None,
            parent_id: Some(b.id.clone()),
            external_source: None,
            external_id: None,
        }).unwrap();

        // Try to make A a child of C (would create cycle)
        let result = svc.update_group(&a.id, serde_json::json!({"parent_id": c.id}));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cycle"));
    }

    #[test]
    fn test_ancestor_groups() {
        let svc = test_service();

        let a = svc.create_group(CreateGroup {
            name: "Company".to_string(),
            description: None,
            parent_id: None,
            external_source: None,
            external_id: None,
        }).unwrap();

        let b = svc.create_group(CreateGroup {
            name: "Engineering".to_string(),
            description: None,
            parent_id: Some(a.id.clone()),
            external_source: None,
            external_id: None,
        }).unwrap();

        let c = svc.create_group(CreateGroup {
            name: "Firmware".to_string(),
            description: None,
            parent_id: Some(b.id.clone()),
            external_source: None,
            external_id: None,
        }).unwrap();

        let ancestors = svc.get_ancestor_group_ids(&c.id).unwrap();
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0], b.id);
        assert_eq!(ancestors[1], a.id);
    }
}
