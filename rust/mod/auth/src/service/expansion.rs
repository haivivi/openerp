use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use openerp_sql::Value;

use crate::model::Group;
use crate::service::{AuthError, AuthService};

/// Cached entry for a user's expanded group list.
struct CacheEntry {
    groups: Vec<String>,
    inserted_at: Instant,
}

/// In-memory cache for user -> expanded groups with TTL.
pub struct GroupCache {
    ttl: Duration,
    entries: RwLock<HashMap<String, CacheEntry>>,
}

impl GroupCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Get cached groups for a user. Returns None if expired or missing.
    pub fn get(&self, user_id: &str) -> Option<Vec<String>> {
        let entries = self.entries.read().unwrap();
        entries.get(user_id).and_then(|entry| {
            if entry.inserted_at.elapsed() < self.ttl {
                Some(entry.groups.clone())
            } else {
                None
            }
        })
    }

    /// Get stale groups for a user (expired but still cached). Returns None if missing.
    pub fn get_stale(&self, user_id: &str) -> Option<Vec<String>> {
        let entries = self.entries.read().unwrap();
        entries.get(user_id).map(|entry| entry.groups.clone())
    }

    /// Store groups for a user.
    pub fn set(&self, user_id: &str, groups: Vec<String>) {
        let mut entries = self.entries.write().unwrap();
        entries.insert(
            user_id.to_string(),
            CacheEntry {
                groups,
                inserted_at: Instant::now(),
            },
        );
    }

    /// Invalidate a specific user's cache entry.
    pub fn invalidate(&self, user_id: &str) {
        let mut entries = self.entries.write().unwrap();
        entries.remove(user_id);
    }

    /// Invalidate all cache entries.
    pub fn invalidate_all(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.clear();
    }
}

impl AuthService {
    /// Expand a user's groups: direct memberships + ancestor groups via parent_id chain.
    ///
    /// Uses a TTL cache:
    /// - Cache hit (fresh) → return immediately
    /// - Cache miss → compute synchronously, store in cache
    ///
    /// Returns group IDs (not names).
    pub fn expand_user_groups(&self, user_id: &str) -> Result<Vec<String>, AuthError> {
        // Check cache
        if let Some(cached) = self.group_cache.get(user_id) {
            return Ok(cached);
        }

        // Compute
        let groups = self.compute_user_groups(user_id)?;

        // Store in cache
        self.group_cache.set(user_id, groups.clone());

        Ok(groups)
    }

    /// Compute the full expanded set of group IDs for a user.
    ///
    /// 1. Find all groups the user is a direct member of
    /// 2. For each group, walk up the parent_id chain to collect ancestors
    /// 3. For each group, also find groups that contain this group as a member
    /// 4. Return deduplicated set of all group IDs
    fn compute_user_groups(&self, user_id: &str) -> Result<Vec<String>, AuthError> {
        let mut all_group_ids = HashSet::new();

        // Step 1: Direct group memberships
        let member_ref = format!("user:{}", user_id);
        let rows = self.sql
            .query(
                "SELECT group_id FROM group_members WHERE member_ref = ?1",
                &[Value::Text(member_ref)],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let mut direct_groups = Vec::new();
        for row in &rows {
            if let Some(gid) = row.get_str("group_id") {
                direct_groups.push(gid.to_string());
                all_group_ids.insert(gid.to_string());
            }
        }

        // Step 2: Walk up parent_id chain for each direct group
        for gid in &direct_groups {
            let ancestors = self.get_ancestor_group_ids(gid)?;
            for ancestor in ancestors {
                all_group_ids.insert(ancestor);
            }
        }

        // Step 3: Walk up "group as member of group" chain
        let mut to_check: Vec<String> = all_group_ids.iter().cloned().collect();
        let mut checked = HashSet::new();
        while let Some(gid) = to_check.pop() {
            if checked.contains(&gid) {
                continue;
            }
            checked.insert(gid.clone());

            let group_ref = format!("group:{}", gid);
            let rows = self.sql
                .query(
                    "SELECT group_id FROM group_members WHERE member_ref = ?1",
                    &[Value::Text(group_ref)],
                )
                .map_err(|e| AuthError::Storage(e.to_string()))?;

            for row in &rows {
                if let Some(parent_gid) = row.get_str("group_id") {
                    if all_group_ids.insert(parent_gid.to_string()) {
                        to_check.push(parent_gid.to_string());
                        // Also walk up parent chain for this new group
                        let ancestors = self.get_ancestor_group_ids(parent_gid)?;
                        for ancestor in ancestors {
                            if all_group_ids.insert(ancestor.clone()) {
                                to_check.push(ancestor);
                            }
                        }
                    }
                }
            }
        }

        let mut result: Vec<String> = all_group_ids.into_iter().collect();
        result.sort();
        Ok(result)
    }

    /// Get expanded group names (not ids) for a user. Used in JWT claims.
    pub fn expand_user_group_names(&self, user_id: &str) -> Result<Vec<String>, AuthError> {
        let group_ids = self.expand_user_groups(user_id)?;
        let mut names = Vec::new();
        for gid in &group_ids {
            if let Ok(group) = self.get_record::<Group>("groups", gid) {
                names.push(group.name);
            }
        }
        names.sort();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{AddGroupMember, CreateGroup, CreateUser};
    use crate::service::{AuthConfig, AuthService};
    use openerp_sql::sqlite::SqliteStore;

    fn test_service() -> std::sync::Arc<AuthService> {
        let sql = Box::new(SqliteStore::open_in_memory().unwrap());
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let kv = Box::new(openerp_kv::redb::RedbStore::open(tmp.path()).unwrap());
        AuthService::new(sql, kv, AuthConfig::default()).unwrap()
    }

    #[test]
    fn test_expand_user_groups_simple() {
        let svc = test_service();

        let user = svc.create_user(CreateUser {
            name: "Alice".to_string(),
            email: None, avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        let group = svc.create_group(CreateGroup {
            name: "engineering".to_string(),
            description: None,
            parent_id: None,
            external_source: None,
            external_id: None,
        }).unwrap();

        svc.add_group_member(&group.id, AddGroupMember {
            member_ref: format!("user:{}", user.id),
        }).unwrap();

        let groups = svc.expand_user_groups(&user.id).unwrap();
        assert_eq!(groups.len(), 1);
        assert!(groups.contains(&group.id));
    }

    #[test]
    fn test_expand_user_groups_with_hierarchy() {
        let svc = test_service();

        // company → engineering → firmware
        let company = svc.create_group(CreateGroup {
            name: "company".to_string(),
            description: None,
            parent_id: None,
            external_source: None,
            external_id: None,
        }).unwrap();

        let eng = svc.create_group(CreateGroup {
            name: "engineering".to_string(),
            description: None,
            parent_id: Some(company.id.clone()),
            external_source: None,
            external_id: None,
        }).unwrap();

        let fw = svc.create_group(CreateGroup {
            name: "firmware".to_string(),
            description: None,
            parent_id: Some(eng.id.clone()),
            external_source: None,
            external_id: None,
        }).unwrap();

        let user = svc.create_user(CreateUser {
            name: "Bob".to_string(),
            email: None, avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        // User is member of firmware only
        svc.add_group_member(&fw.id, AddGroupMember {
            member_ref: format!("user:{}", user.id),
        }).unwrap();

        // Expansion should include firmware + engineering + company
        let groups = svc.expand_user_groups(&user.id).unwrap();
        assert_eq!(groups.len(), 3);
        assert!(groups.contains(&fw.id));
        assert!(groups.contains(&eng.id));
        assert!(groups.contains(&company.id));

        // Group names
        let names = svc.expand_user_group_names(&user.id).unwrap();
        assert!(names.contains(&"firmware".to_string()));
        assert!(names.contains(&"engineering".to_string()));
        assert!(names.contains(&"company".to_string()));
    }

    #[test]
    fn test_expand_caching() {
        let svc = test_service();

        let user = svc.create_user(CreateUser {
            name: "Cache".to_string(),
            email: None, avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        let group = svc.create_group(CreateGroup {
            name: "team".to_string(),
            description: None,
            parent_id: None,
            external_source: None,
            external_id: None,
        }).unwrap();

        svc.add_group_member(&group.id, AddGroupMember {
            member_ref: format!("user:{}", user.id),
        }).unwrap();

        // First call: compute and cache
        let groups1 = svc.expand_user_groups(&user.id).unwrap();
        assert_eq!(groups1.len(), 1);

        // Second call: should hit cache
        let groups2 = svc.expand_user_groups(&user.id).unwrap();
        assert_eq!(groups1, groups2);
    }
}
