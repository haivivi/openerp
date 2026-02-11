use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// An ACL policy entry: (who, what?, how, time?).
///
/// - **who**: Subject reference, e.g. "user:alice" or "group:engineering"
/// - **what**: Optional resource path, e.g. "pms:batch:B001" (empty = global)
/// - **how**: Role id that grants the permission
/// - **expires_at**: Optional expiration (None = permanent)
///
/// ID is derived from hash(who + what + how) — same triple = same policy.
/// Re-submitting updates the expiration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Deterministic id: hex(sha256(who + ":" + what + ":" + how)), first 32 chars.
    pub id: String,

    /// Subject: "user:{id}" or "group:{id}".
    pub who: String,

    /// Resource path (empty string = global).
    #[serde(default)]
    pub what: String,

    /// Role id that this policy grants.
    pub how: String,

    /// Optional expiration (RFC 3339). None = permanent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,

    /// RFC 3339 creation timestamp.
    pub created_at: String,

    /// RFC 3339 last update timestamp.
    pub updated_at: String,
}

/// Input for creating/upserting a policy.
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePolicy {
    pub who: String,
    #[serde(default)]
    pub what: String,
    pub how: String,
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Parameters for querying policies.
#[derive(Debug, Clone, Deserialize)]
pub struct PolicyQuery {
    #[serde(default)]
    pub who: Option<String>,
    #[serde(default)]
    pub what: Option<String>,
    #[serde(default)]
    pub how: Option<String>,
}

/// Result of a permission check.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub allowed: bool,
    /// The matching policy id, if allowed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
}

/// Parameters for the /auth/check endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct CheckParams {
    pub who: String,
    #[serde(default)]
    pub what: String,
    pub how: String,
}

/// Compute the deterministic policy id from (who, what, how).
pub fn policy_id(who: &str, what: &str, how: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(who.as_bytes());
    hasher.update(b":");
    hasher.update(what.as_bytes());
    hasher.update(b":");
    hasher.update(how.as_bytes());
    let result = hasher.finalize();
    // Take first 16 bytes (32 hex chars) for a reasonably short, unique id.
    hex_encode(&result[..16])
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_id_deterministic() {
        let id1 = policy_id("user:alice", "pms:batch", "pms:admin");
        let id2 = policy_id("user:alice", "pms:batch", "pms:admin");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 32);
    }

    #[test]
    fn test_policy_id_different_inputs() {
        let id1 = policy_id("user:alice", "pms:batch", "pms:admin");
        let id2 = policy_id("user:bob", "pms:batch", "pms:admin");
        assert_ne!(id1, id2);
    }
}
