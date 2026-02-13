//! Integration tests for DSL proc macros.

use openerp_dsl_macro::{model, persistent};

// ── Definitions (at crate level so they're visible to tests) ──

#[model(module = "auth")]
#[key(id)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub created_at: Option<String>,
}

#[model(module = "pms")]
#[key(model_code, semver)]
pub struct Firmware {
    pub model_code: u32,
    pub semver: String,
    pub build: u64,
}

#[persistent(User, store = "kv")]
#[key(id)]
#[unique(email)]
pub struct UserDB {
    #[auto(uuid)]
    pub id: String,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    #[auto(create_timestamp)]
    pub created_at: String,
    #[auto(update_timestamp)]
    pub updated_at: String,
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn model_has_serde() {
        let user = User {
            id: "u1".into(),
            name: "Alice".into(),
            email: Some("alice@test.com".into()),
            created_at: None,
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"name\":\"Alice\""));
        assert!(json.contains("createdAt")); // camelCase

        let back: User = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Alice");
    }

    #[test]
    fn model_has_ir_metadata() {
        let ir: openerp_ir::ModelIR = serde_json::from_str(User::__DSL_IR).unwrap();
        assert_eq!(ir.name, "User");
        assert_eq!(ir.module, "auth");
        assert_eq!(ir.key.fields, vec!["id"]);
        assert_eq!(ir.fields.len(), 4);
    }

    #[test]
    fn compound_key_model() {
        let ir: openerp_ir::ModelIR = serde_json::from_str(Firmware::__DSL_IR).unwrap();
        assert_eq!(ir.key.fields, vec!["model_code", "semver"]);
        assert!(ir.key.is_compound());
    }

    #[test]
    fn persistent_crud() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("test.redb")).unwrap(),
        );

        let store = UserStore::new(kv);

        // Create.
        let user = UserDB {
            id: String::new(),
            name: "Alice".into(),
            email: "alice@test.com".into(),
            password_hash: "hash123".into(),
            created_at: String::new(),
            updated_at: String::new(),
        };

        let created = store.create(user).unwrap();
        assert!(!created.id.is_empty(), "ID should be auto-generated");
        assert!(!created.created_at.is_empty(), "created_at auto-filled");
        assert!(!created.updated_at.is_empty(), "updated_at auto-filled");

        // Get.
        let fetched = store.get_or_err(&created.id).unwrap();
        assert_eq!(fetched.name, "Alice");
        assert_eq!(fetched.password_hash, "hash123");

        // List.
        let all = store.list().unwrap();
        assert_eq!(all.len(), 1);

        // Update.
        let mut updated = fetched.clone();
        updated.name = "Alice Updated".into();
        let updated = store.update(&created.id, updated).unwrap();
        assert_eq!(updated.name, "Alice Updated");

        // Delete.
        store.delete(&created.id).unwrap();
        assert!(store.get(&created.id).unwrap().is_none());
    }

    #[test]
    fn persistent_unique_constraint() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("test2.redb")).unwrap(),
        );

        let store = UserStore::new(kv);

        let user1 = UserDB {
            id: String::new(),
            name: "Alice".into(),
            email: "same@test.com".into(),
            password_hash: "h1".into(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        store.create(user1).unwrap();

        let user2 = UserDB {
            id: String::new(),
            name: "Bob".into(),
            email: "same@test.com".into(),
            password_hash: "h2".into(),
            created_at: String::new(),
            updated_at: String::new(),
        };
        let result = store.create(user2);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("already exists"),
            "expected unique violation"
        );
    }

    #[test]
    fn persistent_ir_metadata() {
        let ir: openerp_ir::PersistentIR =
            serde_json::from_str(UserDB::__DSL_PERSISTENT_IR).unwrap();
        assert_eq!(ir.model, "User");
        assert_eq!(ir.store, openerp_ir::StoreType::Kv);
        assert_eq!(ir.key.fields, vec!["id"]);
        assert_eq!(ir.indexes.len(), 1);
    }
}
