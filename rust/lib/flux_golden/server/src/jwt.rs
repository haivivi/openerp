//! JWT service — issue and verify tokens.
//!
//! Shared by login handler (issue) and all facet handlers (verify).
//! Golden test uses a hardcoded secret; production reads from config.

use serde::{Deserialize, Serialize};

/// JWT claims — what's inside the token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — user ID (e.g. "alice").
    pub sub: String,
    /// Display name.
    pub name: String,
    /// Issued at (unix timestamp).
    pub iat: i64,
    /// Expiration (unix timestamp).
    pub exp: i64,
}

/// JWT service — issue and verify tokens.
///
/// Other services only need `verify()` — they don't need the secret.
/// In production, this could use asymmetric keys (RS256) so only the
/// auth service holds the private key.
#[derive(Clone)]
pub struct JwtService {
    encoding_key: jsonwebtoken::EncodingKey,
    decoding_key: jsonwebtoken::DecodingKey,
    validation: jsonwebtoken::Validation,
    expire_secs: i64,
}

/// Golden test secret — hardcoded, not for production.
pub const GOLDEN_TEST_SECRET: &str = "golden-test-jwt-secret";

impl JwtService {
    /// Create a new JwtService with an HMAC secret.
    pub fn new(secret: &str, expire_secs: i64) -> Self {
        Self {
            encoding_key: jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
            validation: jsonwebtoken::Validation::default(),
            expire_secs,
        }
    }

    /// Create with the golden test secret (24h expiry).
    pub fn golden_test() -> Self {
        Self::new(GOLDEN_TEST_SECRET, 86400)
    }

    /// Issue a signed JWT for a user.
    pub fn issue(&self, user_id: &str, display_name: &str) -> Result<String, String> {
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: user_id.to_string(),
            name: display_name.to_string(),
            iat: now,
            exp: now + self.expire_secs,
        };
        jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &self.encoding_key)
            .map_err(|e| format!("jwt encode: {}", e))
    }

    /// Verify a JWT and extract claims.
    /// Returns Err if the token is invalid, expired, or tampered with.
    pub fn verify(&self, token: &str) -> Result<Claims, String> {
        jsonwebtoken::decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| format!("jwt verify: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_verify() {
        let svc = JwtService::golden_test();
        let token = svc.issue("alice", "Alice Wang").unwrap();
        let claims = svc.verify(&token).unwrap();
        assert_eq!(claims.sub, "alice");
        assert_eq!(claims.name, "Alice Wang");
    }

    #[test]
    fn verify_invalid_token_rejected() {
        let svc = JwtService::golden_test();
        let result = svc.verify("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn verify_wrong_secret_rejected() {
        let issuer = JwtService::new("secret-a", 3600);
        let verifier = JwtService::new("secret-b", 3600);
        let token = issuer.issue("alice", "Alice").unwrap();
        let result = verifier.verify(&token);
        assert!(result.is_err());
    }

    #[test]
    fn verify_expired_token_rejected() {
        let svc = JwtService::new(GOLDEN_TEST_SECRET, -120); // Expired 2 minutes ago (past leeway).
        let token = svc.issue("alice", "Alice").unwrap();
        let result = svc.verify(&token);
        assert!(result.is_err());
    }
}
