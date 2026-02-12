use std::collections::HashMap;

use openerp_core::{ListParams, ListResult, merge_patch, now_rfc3339};
use openerp_sql::Value;

use crate::model::{CreateProvider, CreateUser, Provider, ProviderPublic};
use crate::service::{AuthError, AuthService};

impl AuthService {
    /// Create a new OAuth provider.
    pub fn create_provider(&self, input: CreateProvider) -> Result<Provider, AuthError> {
        if input.id.is_empty() {
            return Err(AuthError::Validation("provider id cannot be empty".into()));
        }

        let now = now_rfc3339();
        let provider = Provider {
            id: input.id,
            name: input.name,
            provider_type: input.provider_type,
            client_id: input.client_id,
            client_secret: Some(input.client_secret),
            auth_url: input.auth_url,
            token_url: input.token_url,
            userinfo_url: input.userinfo_url,
            scopes: input.scopes,
            redirect_url: input.redirect_url,
            enabled: input.enabled,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        self.insert_record(
            "providers",
            &provider.id,
            &provider,
            &[
                ("name", Value::Text(provider.name.clone())),
                ("enabled", Value::Integer(if provider.enabled { 1 } else { 0 })),
                ("created_at", Value::Text(now.clone())),
                ("updated_at", Value::Text(now)),
            ],
        )?;

        Ok(provider)
    }

    /// Get a provider by id (with secret, internal use only).
    pub fn get_provider(&self, id: &str) -> Result<Provider, AuthError> {
        self.get_record("providers", id)
    }

    /// Get a provider by id (public, no secret).
    pub fn get_provider_public(&self, id: &str) -> Result<ProviderPublic, AuthError> {
        let provider: Provider = self.get_record("providers", id)?;
        Ok(provider.into())
    }

    /// List providers (public, no secrets).
    pub fn list_providers(&self, params: &ListParams) -> Result<ListResult<ProviderPublic>, AuthError> {
        let (items, total): (Vec<Provider>, usize) =
            self.list_records("providers", &[], params.limit, params.offset)?;
        let public_items: Vec<ProviderPublic> = items.into_iter().map(|p| p.into()).collect();
        Ok(ListResult {
            items: public_items,
            total,
        })
    }

    /// Update a provider with JSON merge-patch.
    pub fn update_provider(
        &self,
        id: &str,
        patch: serde_json::Value,
    ) -> Result<ProviderPublic, AuthError> {
        let current: Provider = self.get_record("providers", id)?;
        let now = now_rfc3339();

        let mut base = serde_json::to_value(&current)
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        merge_patch(&mut base, &patch);
        base["updated_at"] = serde_json::json!(now);
        base["id"] = serde_json::json!(current.id);
        base["created_at"] = serde_json::json!(current.created_at);

        let updated: Provider = serde_json::from_value(base)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        self.update_record(
            "providers",
            id,
            &updated,
            &[
                ("name", Value::Text(updated.name.clone())),
                ("enabled", Value::Integer(if updated.enabled { 1 } else { 0 })),
                ("updated_at", Value::Text(now)),
            ],
        )?;

        Ok(updated.into())
    }

    /// Delete a provider by id.
    pub fn delete_provider(&self, id: &str) -> Result<(), AuthError> {
        self.delete_record("providers", id)
    }

    // ── OAuth Flow ──

    /// Build the OAuth authorization URL for a provider.
    /// The caller should redirect the user's browser to this URL.
    pub fn oauth_authorize_url(&self, provider_id: &str, state: &str) -> Result<String, AuthError> {
        let provider: Provider = self.get_record("providers", provider_id)?;
        if !provider.enabled {
            return Err(AuthError::Validation(format!(
                "provider '{}' is disabled",
                provider_id
            )));
        }

        let scopes = provider.scopes.join(" ");
        let url = format!(
            "{}?client_id={}&redirect_uri={}&scope={}&state={}&response_type=code",
            provider.auth_url,
            urlencoded(&provider.client_id),
            urlencoded(&provider.redirect_url),
            urlencoded(&scopes),
            urlencoded(state),
        );

        Ok(url)
    }

    /// Exchange an OAuth authorization code for user info.
    /// Returns (provider_user_id, user_name, user_email) from the provider.
    ///
    /// This performs:
    /// 1. POST to token_url to exchange code for access_token
    /// 2. GET to userinfo_url to fetch user profile
    pub async fn oauth_callback(
        &self,
        provider_id: &str,
        code: &str,
    ) -> Result<OAuthUserInfo, AuthError> {
        let provider: Provider = self.get_record("providers", provider_id)?;
        if !provider.enabled {
            return Err(AuthError::Validation(format!(
                "provider '{}' is disabled",
                provider_id
            )));
        }

        let client_secret = provider
            .client_secret
            .as_deref()
            .ok_or_else(|| AuthError::Internal("provider missing client_secret".into()))?;

        // Step 1: Exchange code for token
        let client = reqwest::Client::new();
        let token_resp = client
            .post(&provider.token_url)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("client_id", &provider.client_id),
                ("client_secret", client_secret),
                ("redirect_uri", &provider.redirect_url),
            ])
            .send()
            .await
            .map_err(|e| AuthError::Internal(format!("token exchange failed: {}", e)))?;

        if !token_resp.status().is_success() {
            let status = token_resp.status();
            let body = token_resp.text().await.unwrap_or_default();
            return Err(AuthError::Internal(format!(
                "token exchange returned {}: {}",
                status, body
            )));
        }

        let token_json: serde_json::Value = token_resp
            .json()
            .await
            .map_err(|e| AuthError::Internal(format!("token response parse failed: {}", e)))?;

        let access_token = token_json["access_token"]
            .as_str()
            .ok_or_else(|| AuthError::Internal("missing access_token in response".into()))?;

        // Step 2: Fetch user info
        let userinfo_url = provider
            .userinfo_url
            .as_deref()
            .ok_or_else(|| AuthError::Internal("provider missing userinfo_url".into()))?;

        let userinfo_resp = client
            .get(userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| AuthError::Internal(format!("userinfo fetch failed: {}", e)))?;

        if !userinfo_resp.status().is_success() {
            let status = userinfo_resp.status();
            let body = userinfo_resp.text().await.unwrap_or_default();
            return Err(AuthError::Internal(format!(
                "userinfo returned {}: {}",
                status, body
            )));
        }

        let userinfo: serde_json::Value = userinfo_resp
            .json()
            .await
            .map_err(|e| AuthError::Internal(format!("userinfo parse failed: {}", e)))?;

        // Extract user info — different providers use different field names
        let user_id = extract_provider_user_id(&userinfo, provider_id);
        let name = userinfo["name"]
            .as_str()
            .or_else(|| userinfo["login"].as_str())
            .or_else(|| userinfo["display_name"].as_str())
            .unwrap_or("Unknown")
            .to_string();
        let email = userinfo["email"].as_str().map(|s| s.to_string());
        let avatar = userinfo["avatar_url"]
            .as_str()
            .or_else(|| userinfo["picture"].as_str())
            .or_else(|| userinfo["avatar"].as_str())
            .map(|s| s.to_string());

        Ok(OAuthUserInfo {
            provider_user_id: user_id,
            name,
            email,
            avatar,
            raw: userinfo,
        })
    }

    /// Find or create a user from OAuth callback info.
    /// If the user already exists (by linked account), update their info.
    /// Otherwise create a new user with the linked account.
    pub fn find_or_create_oauth_user(
        &self,
        provider_id: &str,
        info: &OAuthUserInfo,
    ) -> Result<crate::model::User, AuthError> {
        // Try to find existing user by linked account
        if let Some(user) = self.find_user_by_linked_account(provider_id, &info.provider_user_id)? {
            // Update user info from provider
            let mut patch = serde_json::json!({});
            if !info.name.is_empty() {
                patch["name"] = serde_json::json!(info.name);
            }
            if let Some(ref email) = info.email {
                patch["email"] = serde_json::json!(email);
            }
            if let Some(ref avatar) = info.avatar {
                patch["avatar"] = serde_json::json!(avatar);
            }
            return self.update_user(&user.id, patch);
        }

        // Create new user with linked account
        let mut linked_accounts = HashMap::new();
        linked_accounts.insert(provider_id.to_string(), info.provider_user_id.clone());

        self.create_user(CreateUser {
            name: info.name.clone(),
            email: info.email.clone(),
            avatar: info.avatar.clone(),
            linked_accounts,
            metadata: None,
        })
    }
}

/// User info extracted from an OAuth provider.
pub struct OAuthUserInfo {
    pub provider_user_id: String,
    pub name: String,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub raw: serde_json::Value,
}

/// Extract the provider-specific user id from userinfo JSON.
fn extract_provider_user_id(userinfo: &serde_json::Value, provider_id: &str) -> String {
    match provider_id {
        "github" => userinfo["id"].as_i64().map(|id| id.to_string()),
        "feishu" => userinfo["open_id"]
            .as_str()
            .or_else(|| userinfo["user_id"].as_str())
            .map(|s| s.to_string()),
        "google" => userinfo["sub"].as_str().map(|s| s.to_string()),
        _ => userinfo["id"]
            .as_str()
            .or_else(|| userinfo["sub"].as_str())
            .map(|s| s.to_string()),
    }
    .unwrap_or_else(|| "unknown".to_string())
}

/// Simple URL encoding for query parameters.
fn urlencoded(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(ch),
            ' ' => result.push('+'),
            _ => {
                let mut buf = [0u8; 4];
                let encoded = ch.encode_utf8(&mut buf);
                for byte in encoded.bytes() {
                    result.push('%');
                    result.push_str(&format!("{:02X}", byte));
                }
            }
        }
    }
    result
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
    fn test_provider_crud() {
        let svc = test_service();

        let provider = svc.create_provider(CreateProvider {
            id: "github".to_string(),
            name: "GitHub".to_string(),
            provider_type: "oauth2".to_string(),
            client_id: "test-client-id".to_string(),
            client_secret: "test-secret".to_string(),
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            userinfo_url: Some("https://api.github.com/user".to_string()),
            scopes: vec!["user:email".to_string()],
            redirect_url: "http://localhost:8080/auth/callback/github".to_string(),
            enabled: true,
        }).unwrap();

        assert_eq!(provider.id, "github");

        // Get (public, no secret)
        let public = svc.get_provider_public("github").unwrap();
        assert_eq!(public.client_id, "test-client-id");

        // List
        let list = svc.list_providers(&ListParams::default()).unwrap();
        assert_eq!(list.total, 1);

        // Update
        let updated = svc.update_provider("github", serde_json::json!({"enabled": false})).unwrap();
        assert!(!updated.enabled);

        // Delete
        svc.delete_provider("github").unwrap();
        assert!(svc.get_provider("github").is_err());
    }

    #[test]
    fn test_oauth_authorize_url() {
        let svc = test_service();

        svc.create_provider(CreateProvider {
            id: "github".to_string(),
            name: "GitHub".to_string(),
            provider_type: "oauth2".to_string(),
            client_id: "my-client".to_string(),
            client_secret: "secret".to_string(),
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            userinfo_url: Some("https://api.github.com/user".to_string()),
            scopes: vec!["user:email".to_string()],
            redirect_url: "http://localhost:8080/callback".to_string(),
            enabled: true,
        }).unwrap();

        let url = svc.oauth_authorize_url("github", "random-state").unwrap();
        assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
        assert!(url.contains("client_id=my-client"));
        assert!(url.contains("state=random-state"));
        assert!(url.contains("response_type=code"));
    }

    #[test]
    fn test_find_or_create_oauth_user() {
        let svc = test_service();

        let info = OAuthUserInfo {
            provider_user_id: "gh-999".to_string(),
            name: "Alice".to_string(),
            email: Some("alice@github.com".to_string()),
            avatar: Some("https://avatar.url".to_string()),
            raw: serde_json::json!({}),
        };

        // First call creates the user
        let user1 = svc.find_or_create_oauth_user("github", &info).unwrap();
        assert_eq!(user1.name, "Alice");
        assert_eq!(user1.linked_accounts.get("github"), Some(&"gh-999".to_string()));

        // Second call finds and updates the same user
        let info2 = OAuthUserInfo {
            provider_user_id: "gh-999".to_string(),
            name: "Alice Updated".to_string(),
            email: Some("alice@github.com".to_string()),
            avatar: None,
            raw: serde_json::json!({}),
        };
        let user2 = svc.find_or_create_oauth_user("github", &info2).unwrap();
        assert_eq!(user2.id, user1.id);
        assert_eq!(user2.name, "Alice Updated");
    }
}
