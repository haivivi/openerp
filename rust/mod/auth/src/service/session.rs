use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use openerp_core::new_id;
use openerp_sql::Value;

use crate::model::{Claims, Session, TokenPair, User};
use crate::service::{AuthError, AuthService};

impl AuthService {
    /// Issue a JWT token pair (access + refresh) for a user.
    ///
    /// Creates a session record and returns signed tokens.
    pub fn issue_tokens(&self, user: &User) -> Result<TokenPair, AuthError> {
        let session_id = new_id();
        let now = chrono::Utc::now();
        let access_exp = now + chrono::Duration::seconds(self.config.access_token_ttl);
        let refresh_exp = now + chrono::Duration::seconds(self.config.refresh_token_ttl);

        // Expand user groups and roles for JWT claims
        let group_names = self.expand_user_group_names(&user.id)?;
        let roles = self.get_user_roles(&user.id)?;

        // Build claims
        let access_claims = Claims {
            sub: user.id.clone(),
            name: user.name.clone(),
            groups: group_names.clone(),
            roles: roles.clone(),
            sid: session_id.clone(),
            iat: now.timestamp(),
            exp: access_exp.timestamp(),
        };

        let refresh_claims = Claims {
            sub: user.id.clone(),
            name: user.name.clone(),
            groups: group_names,
            roles,
            sid: session_id.clone(),
            iat: now.timestamp(),
            exp: refresh_exp.timestamp(),
        };

        // Sign tokens
        let access_token = encode(
            &Header::default(),
            &access_claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(|e| AuthError::Internal(format!("JWT encode failed: {}", e)))?;

        let refresh_token = encode(
            &Header::default(),
            &refresh_claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(|e| AuthError::Internal(format!("JWT encode failed: {}", e)))?;

        // Store session
        let session = Session {
            id: session_id,
            user_id: user.id.clone(),
            issued_at: now.to_rfc3339(),
            expires_at: refresh_exp.to_rfc3339(),
            revoked: false,
            user_agent: None,
            ip_address: None,
        };

        self.insert_record(
            "sessions",
            &session.id,
            &session,
            &[
                ("user_id", Value::Text(session.user_id.clone())),
                ("revoked", Value::Integer(0)),
                ("issued_at", Value::Text(session.issued_at.clone())),
                ("expires_at", Value::Text(session.expires_at.clone())),
            ],
        )?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_ttl,
        })
    }

    /// Verify and decode a JWT access token.
    /// Returns the claims if valid and the session is not revoked.
    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        let mut validation = Validation::default();
        validation.validate_exp = true;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| AuthError::Unauthorized(format!("invalid token: {}", e)))?;

        let claims = token_data.claims;

        // Check if session is revoked
        if let Ok(session) = self.get_record::<Session>("sessions", &claims.sid) {
            if session.revoked {
                return Err(AuthError::Unauthorized("session has been revoked".into()));
            }
        }

        Ok(claims)
    }

    /// Refresh an access token using a refresh token.
    /// Validates the refresh token, revokes the old session, and issues a new pair.
    pub fn refresh_tokens(&self, refresh_token: &str) -> Result<TokenPair, AuthError> {
        let claims = self.verify_token(refresh_token)?;

        // Get the user
        let user: User = self.get_record("users", &claims.sub)
            .map_err(|_| AuthError::Unauthorized("user not found".into()))?;

        if !user.active {
            return Err(AuthError::Unauthorized("user is deactivated".into()));
        }

        // Revoke old session
        self.revoke_session(&claims.sid)?;

        // Issue new tokens
        self.issue_tokens(&user)
    }

    /// Revoke a session (token becomes invalid).
    pub fn revoke_session(&self, session_id: &str) -> Result<(), AuthError> {
        let mut session: Session = self.get_record("sessions", session_id)?;
        session.revoked = true;

        self.update_record(
            "sessions",
            session_id,
            &session,
            &[("revoked", Value::Integer(1))],
        )?;

        Ok(())
    }

    /// Revoke all sessions for a user.
    pub fn revoke_all_user_sessions(&self, user_id: &str) -> Result<u64, AuthError> {
        let affected = self.sql
            .exec(
                "UPDATE sessions SET revoked = 1, data = REPLACE(data, '\"revoked\":false', '\"revoked\":true') WHERE user_id = ?1 AND revoked = 0",
                &[Value::Text(user_id.to_string())],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(affected)
    }

    /// Get a session by id.
    pub fn get_session(&self, id: &str) -> Result<Session, AuthError> {
        self.get_record("sessions", id)
    }

    /// List active sessions for a user.
    pub fn list_user_sessions(&self, user_id: &str) -> Result<Vec<Session>, AuthError> {
        let rows = self.sql
            .query(
                "SELECT data FROM sessions WHERE user_id = ?1 AND revoked = 0 ORDER BY issued_at DESC",
                &[Value::Text(user_id.to_string())],
            )
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let mut sessions = Vec::new();
        for row in &rows {
            if let Some(data) = row.get_str("data") {
                let session: Session = serde_json::from_str(data)
                    .map_err(|e| AuthError::Internal(e.to_string()))?;
                sessions.push(session);
            }
        }
        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::CreateUser;
    use crate::service::{AuthConfig, AuthService};
    use openerp_sql::sqlite::SqliteStore;

    fn test_service() -> std::sync::Arc<AuthService> {
        let sql = Box::new(SqliteStore::open_in_memory().unwrap());
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let kv = Box::new(openerp_kv::redb::RedbStore::open(tmp.path()).unwrap());
        AuthService::new(sql, kv, AuthConfig::default()).unwrap()
    }

    #[test]
    fn test_issue_and_verify_token() {
        let svc = test_service();

        let user = svc.create_user(CreateUser {
            name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
            avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        let tokens = svc.issue_tokens(&user).unwrap();
        assert!(!tokens.access_token.is_empty());
        assert!(!tokens.refresh_token.is_empty());
        assert_eq!(tokens.token_type, "Bearer");
        assert_eq!(tokens.expires_in, 86400);

        // Verify access token
        let claims = svc.verify_token(&tokens.access_token).unwrap();
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.name, "Alice");
    }

    #[test]
    fn test_refresh_token() {
        let svc = test_service();

        let user = svc.create_user(CreateUser {
            name: "Bob".to_string(),
            email: None, avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        let tokens1 = svc.issue_tokens(&user).unwrap();

        // Refresh
        let tokens2 = svc.refresh_tokens(&tokens1.refresh_token).unwrap();
        assert_ne!(tokens2.access_token, tokens1.access_token);

        // Old token should be revoked
        let claims1 = svc.verify_token(&tokens1.access_token);
        assert!(claims1.is_err());

        // New token should work
        let claims2 = svc.verify_token(&tokens2.access_token).unwrap();
        assert_eq!(claims2.sub, user.id);
    }

    #[test]
    fn test_revoke_session() {
        let svc = test_service();

        let user = svc.create_user(CreateUser {
            name: "Charlie".to_string(),
            email: None, avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        let tokens = svc.issue_tokens(&user).unwrap();

        // Verify works before revoke
        let claims = svc.verify_token(&tokens.access_token).unwrap();

        // Revoke
        svc.revoke_session(&claims.sid).unwrap();

        // Token should now be invalid
        assert!(svc.verify_token(&tokens.access_token).is_err());
    }

    #[test]
    fn test_revoke_all_user_sessions() {
        let svc = test_service();

        let user = svc.create_user(CreateUser {
            name: "Dave".to_string(),
            email: None, avatar: None,
            linked_accounts: Default::default(),
            metadata: None,
        }).unwrap();

        let tokens1 = svc.issue_tokens(&user).unwrap();
        let tokens2 = svc.issue_tokens(&user).unwrap();

        // Both should work
        assert!(svc.verify_token(&tokens1.access_token).is_ok());
        assert!(svc.verify_token(&tokens2.access_token).is_ok());

        // List sessions
        let sessions = svc.list_user_sessions(&user.id).unwrap();
        assert_eq!(sessions.len(), 2);

        // Revoke all
        let count = svc.revoke_all_user_sessions(&user.id).unwrap();
        assert_eq!(count, 2);

        // Both should be invalid
        assert!(svc.verify_token(&tokens1.access_token).is_err());
        assert!(svc.verify_token(&tokens2.access_token).is_err());
    }

    #[test]
    fn test_invalid_token() {
        let svc = test_service();

        let result = svc.verify_token("this.is.not.a.valid.jwt");
        assert!(result.is_err());
    }
}
