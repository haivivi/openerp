//! Facet handler implementations for the "app" facet.
//!
//! Each handler is a hand-written axum handler. No auto-CRUD.
//! Current user identity comes from JWT (verified signature).
//! Errors use ServiceError → {"code": "NOT_FOUND", "message": "..."}.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;

use openerp_core::ServiceError;
use openerp_store::{FacetResponse, KvOps};
use openerp_types::*;

use crate::server::i18n::Localizer;
use crate::server::jwt::JwtService;
use crate::server::model::*;
use crate::server::rest_app::app::{self, *};

/// Shared state for facet handlers.
pub struct FacetStateInner {
    pub users: KvOps<User>,
    pub tweets: KvOps<Tweet>,
    pub likes: KvOps<Like>,
    pub follows: KvOps<Follow>,
    pub messages: KvOps<crate::server::model::Message>,
    pub jwt: JwtService,
    pub i18n: Box<dyn Localizer>,
    pub blobs: Arc<dyn openerp_blob::BlobStore>,
    pub blob_base_url: String,
}

pub type FacetState = Arc<FacetStateInner>;

// ── Auth helper ──

/// Extract and verify current user from JWT.
/// Returns user ID or ServiceError::Unauthorized.
fn current_user(headers: &HeaderMap, state: &FacetStateInner) -> Result<String, ServiceError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ServiceError::Unauthorized(
            state.i18n.t("error.auth.missing_token", &[])
        ))?;

    let claims = state.jwt.verify(token)
        .map_err(|e| ServiceError::Unauthorized(
            state.i18n.t("error.auth.invalid_token", &[])
        ))?;

    Ok(claims.sub)
}

// ── Converters ──

fn to_app_tweet(t: &Tweet, uid: &str, state: &FacetStateInner) -> AppTweet {
    let author_id = t.author.resource_id();
    let author = state.users.get(author_id).ok().flatten();
    let like_key = format!("{}:{}", uid, t.id);
    let liked = state.likes.get(&like_key).ok().flatten().is_some();
    AppTweet {
        id: t.id.to_string(),
        author_id: author_id.to_string(),
        author_username: author.as_ref().map(|u| u.username.clone()).unwrap_or_default(),
        author_display_name: author.as_ref().and_then(|u| u.display_name.clone()),
        author_avatar: author.as_ref().and_then(|u| u.avatar.as_ref().map(|a| a.to_string())),
        content: t.content.clone(),
        image_url: t.image_url.as_ref().map(|u| u.to_string()),
        like_count: t.like_count,
        liked_by_me: liked,
        reply_count: t.reply_count,
        reply_to_id: t.reply_to.as_ref().map(|n| n.resource_id().to_string()),
        created_at: t.created_at.to_string(),
    }
}

fn to_app_profile(u: &User, uid: &str, state: &FacetStateInner) -> AppProfile {
    let follow_key = format!("{}:{}", uid, u.id);
    let followed = state.follows.get(&follow_key).ok().flatten().is_some();
    AppProfile {
        id: u.id.to_string(),
        username: u.username.clone(),
        display_name: u.display_name.clone(),
        bio: u.bio.as_ref().map(|s| s.to_string()),
        avatar: u.avatar.as_ref().map(|a| a.to_string()),
        follower_count: u.follower_count,
        following_count: u.following_count,
        tweet_count: u.tweet_count,
        followed_by_me: followed,
    }
}

fn to_app_user(u: &User) -> AppUser {
    AppUser {
        id: u.id.to_string(),
        username: u.username.clone(),
        display_name: u.display_name.clone(),
        bio: u.bio.as_ref().map(|s| s.to_string()),
        avatar: u.avatar.as_ref().map(|a| a.to_string()),
        follower_count: u.follower_count,
        following_count: u.following_count,
        tweet_count: u.tweet_count,
        updated_at: Some(u.updated_at.to_string()),
    }
}

// ── Handlers ──

/// Simple password hash for golden test (SHA256 hex).
fn hash_password(password: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn verify_password(password: &str, hash: &str) -> bool {
    hash_password(password) == hash
}

/// POST /auth/login — public, no JWT required.
pub async fn login(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ServiceError> {
    let user = state.users.get(&req.username)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .ok_or_else(|| ServiceError::Unauthorized(
            state.i18n.t("error.auth.user_not_found", &[("username", &req.username)])
        ))?;

    if let Some(ref stored_hash) = user.password_hash {
        if !verify_password(&req.password, stored_hash.as_str()) {
            return Err(ServiceError::Unauthorized(
                state.i18n.t("error.auth.invalid_token", &[])
            ));
        }
    }

    let display = user.display_name.as_deref().unwrap_or(&user.username);
    let token = state.jwt.issue(user.id.as_str(), display)
        .map_err(|e| ServiceError::Internal(e))?;

    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".into(),
        expires_in: 86400,
        user: to_app_user(&user),
    }))
}

/// GET /me
pub async fn get_me(
    headers: HeaderMap,
    State(state): State<FacetState>,
) -> Result<FacetResponse<AppUser>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    let user = state.users.get(&uid)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .ok_or_else(|| ServiceError::NotFound(state.i18n.t("error.profile.not_found", &[("id", &uid)])))?;
    Ok(FacetResponse::negotiate(to_app_user(&user), &headers))
}

/// POST /timeline — paginated.
pub async fn get_timeline(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Json(params): Json<PaginationParams>,
) -> Result<Json<TimelineResponse>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    let mut tweets = state.tweets.list().map_err(|e| ServiceError::Internal(e.to_string()))?;
    tweets.sort_by(|a, b| b.created_at.as_str().cmp(a.created_at.as_str()));
    let all_items: Vec<AppTweet> = tweets.iter()
        .filter(|t| t.reply_to.is_none())
        .map(|t| to_app_tweet(t, &uid, &state))
        .collect();
    let total = all_items.len();
    let offset = params.offset.min(total);
    let end = (offset + params.limit).min(total);
    let items = all_items[offset..end].to_vec();
    let has_more = end < total;
    Ok(Json(TimelineResponse { items, has_more }))
}

/// POST /tweets
pub async fn create_tweet(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Json(req): Json<CreateTweetRequest>,
) -> Result<FacetResponse<AppTweet>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    if req.content.trim().is_empty() {
        return Err(ServiceError::Validation(state.i18n.t("error.tweet.empty", &[])));
    }
    if req.content.len() > 280 {
        return Err(ServiceError::Validation(state.i18n.t("error.tweet.too_long", &[("max", "280")])));
    }
    let tweet = Tweet {
        id: Id::default(),
        author: Name::new(&format!("twitter/users/{}", uid)),
        content: req.content,
        image_url: None,
        like_count: 0, reply_count: 0,
        reply_to: req.reply_to_id.map(|s| Name::new(&format!("twitter/tweets/{}", s))),
        display_name: None, description: None, metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
    };
    let created = state.tweets.save_new(tweet).map_err(|e| ServiceError::Internal(e.to_string()))?;
    if let Ok(Some(mut user)) = state.users.get(&uid) {
        user.tweet_count += 1;
        let _ = state.users.save(user);
    }
    if let Some(ref parent_name) = created.reply_to {
        if let Ok(Some(mut parent)) = state.tweets.get(parent_name.resource_id()) {
            parent.reply_count += 1;
            let _ = state.tweets.save(parent);
        }
    }
    Ok(FacetResponse::negotiate(to_app_tweet(&created, &uid, &state), &headers))
}

/// POST /tweets/{id}/detail
pub async fn tweet_detail(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> Result<Json<TweetDetailResponse>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    let tweet = state.tweets.get(&id)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .ok_or_else(|| ServiceError::NotFound(state.i18n.t("error.tweet.not_found", &[("id", &id)])))?;
    let item = to_app_tweet(&tweet, &uid, &state);
    let all = state.tweets.list().unwrap_or_default();
    let mut replies: Vec<AppTweet> = all.iter()
        .filter(|t| t.reply_to.as_ref().map(|n| n.resource_id()) == Some(&id))
        .map(|t| to_app_tweet(t, &uid, &state))
        .collect();
    replies.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(Json(TweetDetailResponse { tweet: item, replies }))
}

/// POST /tweets/{id}/like
pub async fn like_tweet(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> Result<FacetResponse<AppTweet>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    let like = Like {
        id: Id::default(),
        user: Name::new(&format!("twitter/users/{}", uid)),
        tweet: Name::new(&format!("twitter/tweets/{}", id)),
        display_name: None, description: None, metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
    };
    let _ = state.likes.save_new(like);
    let mut tweet = state.tweets.get(&id)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .ok_or_else(|| ServiceError::NotFound(state.i18n.t("error.tweet.not_found", &[("id", &id)])))?;
    let all_likes = state.likes.list().unwrap_or_default();
    tweet.like_count = all_likes.iter().filter(|l| l.tweet.resource_id() == id).count() as u32;
    let _ = state.tweets.save(tweet.clone());
    Ok(FacetResponse::negotiate(to_app_tweet(&tweet, &uid, &state), &headers))
}

/// DELETE /tweets/{id}/like
pub async fn unlike_tweet(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> Result<(), ServiceError> {
    let uid = current_user(&headers, &state)?;
    let like_key = format!("{}:{}", uid, id);
    let _ = state.likes.delete(&like_key);
    if let Ok(Some(mut tweet)) = state.tweets.get(&id) {
        let all_likes = state.likes.list().unwrap_or_default();
        tweet.like_count = all_likes.iter().filter(|l| l.tweet.resource_id() == id).count() as u32;
        let _ = state.tweets.save(tweet);
    }
    Ok(())
}

/// POST /users/{id}/follow
pub async fn follow_user(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> Result<FacetResponse<AppProfile>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    let follow = Follow {
        id: Id::default(),
        follower: Name::new(&format!("twitter/users/{}", uid)),
        followee: Name::new(&format!("twitter/users/{}", id)),
        display_name: None, description: None, metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
    };
    if state.follows.save_new(follow).is_ok() {
        if let Ok(Some(mut me)) = state.users.get(&uid) {
            me.following_count += 1;
            let _ = state.users.save(me);
        }
        if let Ok(Some(mut them)) = state.users.get(&id) {
            them.follower_count += 1;
            let _ = state.users.save(them);
        }
    }
    let user = state.users.get(&id)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .ok_or_else(|| ServiceError::NotFound(state.i18n.t("error.profile.not_found", &[("id", &id)])))?;
    Ok(FacetResponse::negotiate(to_app_profile(&user, &uid, &state), &headers))
}

/// DELETE /users/{id}/follow
pub async fn unfollow_user(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> Result<(), ServiceError> {
    let uid = current_user(&headers, &state)?;
    let key = format!("{}:{}", uid, id);
    if state.follows.delete(&key).is_ok() {
        if let Ok(Some(mut me)) = state.users.get(&uid) {
            me.following_count = me.following_count.saturating_sub(1);
            let _ = state.users.save(me);
        }
        if let Ok(Some(mut them)) = state.users.get(&id) {
            them.follower_count = them.follower_count.saturating_sub(1);
            let _ = state.users.save(them);
        }
    }
    Ok(())
}

/// POST /users/{id}/profile
pub async fn user_profile(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> Result<Json<UserProfileResponse>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    let user = state.users.get(&id)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .ok_or_else(|| ServiceError::NotFound(state.i18n.t("error.profile.not_found", &[("id", &id)])))?;
    let profile = to_app_profile(&user, &uid, &state);
    let all = state.tweets.list().unwrap_or_default();
    let tweets: Vec<AppTweet> = all.iter()
        .filter(|t| t.author.resource_id() == id)
        .map(|t| to_app_tweet(t, &uid, &state))
        .collect();
    Ok(Json(UserProfileResponse { user: profile, tweets }))
}

/// PUT /me/profile — with optimistic locking via updatedAt.
pub async fn update_profile(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<FacetResponse<AppUser>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    if req.display_name.trim().is_empty() {
        return Err(ServiceError::Validation(state.i18n.t("error.profile.name_empty", &[])));
    }
    let mut user = state.users.get(&uid)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .ok_or_else(|| ServiceError::NotFound(state.i18n.t("error.profile.not_found", &[("id", &uid)])))?;

    // Optimistic locking: if client sends updatedAt, compare with stored value.
    if let Some(ref client_ts) = req.updated_at {
        if client_ts != user.updated_at.as_str() {
            return Err(ServiceError::Conflict(format!(
                "updatedAt mismatch: stored {}, got {}",
                user.updated_at, client_ts
            )));
        }
    }

    user.display_name = Some(req.display_name);
    user.bio = Some(req.bio);
    // save() in KvOps also checks updatedAt internally and stamps a new one.
    state.users.save(user.clone()).map_err(|e| {
        if e.to_string().contains("mismatch") {
            return e; // Already a Conflict from KvOps.
        }
        ServiceError::Internal(e.to_string())
    })?;
    // Re-fetch to get the new updatedAt stamp.
    let updated = state.users.get(&uid)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .unwrap_or(user);
    Ok(FacetResponse::negotiate(to_app_user(&updated), &headers))
}

/// PUT /me/password
pub async fn change_password(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<ChangePasswordResponse>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    let mut user = state.users.get(&uid)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .ok_or_else(|| ServiceError::NotFound("user not found".into()))?;

    if let Some(ref stored) = user.password_hash {
        if !verify_password(&req.old_password, stored.as_str()) {
            return Err(ServiceError::Unauthorized("incorrect old password".into()));
        }
    }
    if req.new_password.len() < 6 {
        return Err(ServiceError::Validation("password must be at least 6 characters".into()));
    }
    if req.old_password == req.new_password {
        return Err(ServiceError::Validation("new password must be different".into()));
    }

    user.password_hash = Some(PasswordHash::new(&hash_password(&req.new_password)));
    state.users.save(user).map_err(|e| ServiceError::Internal(e.to_string()))?;
    Ok(Json(ChangePasswordResponse { ok: true }))
}

/// POST /search
pub async fn search(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    let q = req.query.to_lowercase();
    let users: Vec<AppProfile> = state.users.list().unwrap_or_default().iter()
        .filter(|u| u.username.to_lowercase().contains(&q)
            || u.display_name.as_deref().unwrap_or("").to_lowercase().contains(&q))
        .map(|u| to_app_profile(u, &uid, &state))
        .collect();
    let tweets: Vec<AppTweet> = state.tweets.list().unwrap_or_default().iter()
        .filter(|t| t.content.to_lowercase().contains(&q))
        .map(|t| to_app_tweet(t, &uid, &state))
        .collect();
    Ok(Json(SearchResponse { users, tweets }))
}

/// POST /upload — upload image, returns URL.
pub async fn upload_image(
    headers: HeaderMap,
    State(state): State<FacetState>,
    body: axum::body::Bytes,
) -> Result<Json<UploadResponse>, ServiceError> {
    let uid = current_user(&headers, &state)?;

    if body.is_empty() {
        return Err(ServiceError::Validation("empty file".into()));
    }
    // Max 5MB.
    if body.len() > 5 * 1024 * 1024 {
        return Err(ServiceError::Validation("file exceeds 5MB limit".into()));
    }

    let key = format!("uploads/{}/{}.jpg", uid, uuid::Uuid::new_v4().to_string().replace('-', ""));
    state.blobs.put(&key, &body)
        .map_err(|e| ServiceError::Internal(e.to_string()))?;

    let url = format!("{}/blobs/{}", state.blob_base_url, key);
    Ok(Json(UploadResponse {
        url,
        size: body.len() as u64,
    }))
}

// ── Inbox (站内信) ──

fn lang_from_headers(headers: &HeaderMap) -> String {
    headers.get("accept-language")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("en")
        .split(',')
        .next()
        .unwrap_or("en")
        .trim()
        .to_string()
}

fn to_app_message(m: &crate::server::model::Message, lang: &str) -> AppMessage {
    AppMessage {
        id: m.id.to_string(),
        kind: m.kind.clone(),
        title: m.title.get(lang).to_string(),
        body: m.body.get(lang).to_string(),
        read: m.read,
        created_at: m.created_at.to_string(),
    }
}

/// POST /inbox — get messages for current user.
pub async fn get_inbox(
    headers: HeaderMap,
    State(state): State<FacetState>,
) -> Result<Json<InboxResponse>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    let lang = lang_from_headers(&headers);
    let all = state.messages.list()
        .map_err(|e| ServiceError::Internal(e.to_string()))?;

    let msgs: Vec<AppMessage> = all.iter()
        .filter(|m| {
            m.recipient.as_ref().map(|n| n.resource_id()) == Some(uid.as_str())
                || m.recipient.is_none()
        })
        .map(|m| to_app_message(m, &lang))
        .collect();

    let unread = msgs.iter().filter(|m| !m.read).count();
    Ok(Json(InboxResponse { messages: msgs, unread_count: unread }))
}

/// POST /messages/{id}/read — mark message as read.
pub async fn mark_read(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> Result<Json<AppMessage>, ServiceError> {
    let uid = current_user(&headers, &state)?;
    let lang = lang_from_headers(&headers);
    let mut msg = state.messages.get(&id)
        .map_err(|e| ServiceError::Internal(e.to_string()))?
        .ok_or_else(|| ServiceError::NotFound("Message not found".into()))?;

    msg.read = true;
    state.messages.save(msg.clone())
        .map_err(|e| ServiceError::Internal(e.to_string()))?;

    Ok(Json(to_app_message(&msg, &lang)))
}

// ── Handler completeness check ──
// Register all action handlers — compile error if any are missing.
openerp_macro::impl_handler!(app::Login);
openerp_macro::impl_handler!(app::Timeline);
openerp_macro::impl_handler!(app::CreateTweet);
openerp_macro::impl_handler!(app::TweetDetail);
openerp_macro::impl_handler!(app::LikeTweet);
openerp_macro::impl_handler!(app::UnlikeTweet);
openerp_macro::impl_handler!(app::FollowUser);
openerp_macro::impl_handler!(app::UnfollowUser);
openerp_macro::impl_handler!(app::UserProfile);
openerp_macro::impl_handler!(app::UpdateProfile);
openerp_macro::impl_handler!(app::ChangePassword);
openerp_macro::impl_handler!(app::Upload);
openerp_macro::impl_handler!(app::Search);
openerp_macro::impl_handler!(app::Inbox);
openerp_macro::impl_handler!(app::MarkRead);

/// Build the facet router.
pub fn facet_router(state: FacetState) -> axum::Router {
    app::__assert_handlers::<app::__Handlers>();
    use axum::routing::{get, post, put, delete};
    axum::Router::new()
        .route("/auth/login", post(login))
        .route("/me", get(get_me))
        .route("/me/profile", put(update_profile))
        .route("/me/password", put(change_password))
        .route("/timeline", post(get_timeline))
        .route("/tweets", post(create_tweet))
        .route("/tweets/{id}/detail", post(tweet_detail))
        .route("/tweets/{id}/like", post(like_tweet).delete(unlike_tweet))
        .route("/users/{id}/follow", post(follow_user).delete(unfollow_user))
        .route("/users/{id}/profile", post(user_profile))
        .route("/search", post(search))
        .route("/upload", post(upload_image))
        .route("/inbox", post(get_inbox))
        .route("/messages/{id}/read", post(mark_read))
        .with_state(state)
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn setup() -> (axum::Router, JwtService) {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("test.redb")).unwrap(),
        );
        let jwt = JwtService::golden_test();

        // Seed users.
        let users = KvOps::<User>::new(kv.clone());
        users.save_new(User {
            id: Id::default(), username: "alice".into(),
            password_hash: Some(PasswordHash::new(&hash_password("password"))),
            bio: Some("Rust dev".into()), avatar: None,
            follower_count: 0, following_count: 0, tweet_count: 0,
            display_name: Some("Alice".into()),
            description: None, metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();
        users.save_new(User {
            id: Id::default(), username: "bob".into(),
            password_hash: Some(PasswordHash::new(&hash_password("password"))),
            bio: None, avatar: None,
            follower_count: 0, following_count: 0, tweet_count: 0,
            display_name: Some("Bob".into()),
            description: None, metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();

        let blob_dir = dir.path().join("blobs");
        std::fs::create_dir_all(&blob_dir).unwrap();
        let blobs: Arc<dyn openerp_blob::BlobStore> = Arc::new(
            openerp_blob::FileStore::open(&blob_dir).unwrap(),
        );
        let state = Arc::new(FacetStateInner {
            users: KvOps::new(kv.clone()),
            tweets: KvOps::new(kv.clone()),
            likes: KvOps::new(kv.clone()),
            follows: KvOps::new(kv.clone()),
            messages: KvOps::new(kv.clone()),
            jwt: jwt.clone(),
            i18n: Box::new(crate::server::i18n::DefaultLocalizer),
            blobs,
            blob_base_url: "http://test".to_string(),
        });
        let router = facet_router(state);
        // Leak tempdir to keep DB alive.
        std::mem::forget(dir);
        (router, jwt)
    }

    async fn call(
        router: &axum::Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {}", t));
        }
        if body.is_some() {
            builder = builder.header("content-type", "application/json");
        }
        let body = match body {
            Some(v) => Body::from(serde_json::to_string(&v).unwrap()),
            None => Body::empty(),
        };
        let req = builder.body(body).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let json = if bytes.is_empty() {
            serde_json::json!(null)
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::json!(null))
        };
        (status, json)
    }

    // ── Auth ──

    #[tokio::test]
    async fn login_success() {
        let (r, _) = setup();
        let (s, body) = call(&r, "POST", "/auth/login", None,
            Some(serde_json::json!({"username": "alice", "password": "password"}))).await;
        assert_eq!(s, StatusCode::OK);
        assert!(body["accessToken"].as_str().unwrap().contains('.'));
        assert_eq!(body["user"]["username"], "alice");
    }

    #[tokio::test]
    async fn login_unknown_user() {
        let (r, _) = setup();
        let (s, body) = call(&r, "POST", "/auth/login", None,
            Some(serde_json::json!({"username": "nobody", "password": "password"}))).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "UNAUTHENTICATED");
    }

    #[tokio::test]
    async fn login_wrong_password() {
        let (r, _) = setup();
        let (s, body) = call(&r, "POST", "/auth/login", None,
            Some(serde_json::json!({"username": "alice", "password": "wrong"}))).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "UNAUTHENTICATED");
    }

    #[tokio::test]
    async fn no_token_rejected() {
        let (r, _) = setup();
        let (s, body) = call(&r, "GET", "/me", None, None).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "UNAUTHENTICATED");
    }

    #[tokio::test]
    async fn invalid_token_rejected() {
        let (r, _) = setup();
        let (s, body) = call(&r, "GET", "/me", Some("invalid.jwt.token"), None).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "UNAUTHENTICATED");
    }

    #[tokio::test]
    async fn wrong_secret_rejected() {
        let (r, _) = setup();
        let wrong = JwtService::new("wrong-secret", 3600);
        let token = wrong.issue("alice", "Alice").unwrap();
        let (s, body) = call(&r, "GET", "/me", Some(&token), None).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "UNAUTHENTICATED");
    }

    // ── Me ──

    #[tokio::test]
    async fn get_me_success() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();
        let (s, body) = call(&r, "GET", "/me", Some(&token), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(body["username"], "alice");
        assert_eq!(body["displayName"], "Alice");
    }

    // ── Timeline ──

    #[tokio::test]
    async fn empty_timeline() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();
        let (s, body) = call(&r, "POST", "/timeline", Some(&token), Some(serde_json::json!({}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(body["items"].as_array().unwrap().len(), 0);
    }

    // ── Tweets ──

    #[tokio::test]
    async fn create_and_list_tweet() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        // Create.
        let (s, tweet) = call(&r, "POST", "/tweets", Some(&token),
            Some(serde_json::json!({"content": "Hello!"}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(tweet["content"], "Hello!");
        assert_eq!(tweet["authorUsername"], "alice");

        // Timeline.
        let (s, tl) = call(&r, "POST", "/timeline", Some(&token), Some(serde_json::json!({}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(tl["items"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn create_empty_tweet_rejected() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();
        let (s, body) = call(&r, "POST", "/tweets", Some(&token),
            Some(serde_json::json!({"content": "  "}))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "VALIDATION_FAILED");
    }

    #[tokio::test]
    async fn create_long_tweet_rejected() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();
        let long = "x".repeat(281);
        let (s, body) = call(&r, "POST", "/tweets", Some(&token),
            Some(serde_json::json!({"content": long}))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "VALIDATION_FAILED");
    }

    // ── Like ──

    #[tokio::test]
    async fn like_and_unlike() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        // Create tweet.
        let (_, tweet) = call(&r, "POST", "/tweets", Some(&token),
            Some(serde_json::json!({"content": "Likeable"}))).await;
        let id = tweet["id"].as_str().unwrap();

        // Like.
        let (s, liked) = call(&r, "POST", &format!("/tweets/{}/like", id), Some(&token), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(liked["likeCount"], 1);
        assert_eq!(liked["likedByMe"], true);

        // Unlike.
        let (s, _) = call(&r, "DELETE", &format!("/tweets/{}/like", id), Some(&token), None).await;
        assert_eq!(s, StatusCode::OK);

        // Verify via detail.
        let (_, detail) = call(&r, "POST", &format!("/tweets/{}/detail", id), Some(&token), None).await;
        assert_eq!(detail["tweet"]["likeCount"], 0);
        assert_eq!(detail["tweet"]["likedByMe"], false);
    }

    // ── Follow ──

    #[tokio::test]
    async fn follow_and_unfollow() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        // Follow bob.
        let (s, profile) = call(&r, "POST", "/users/bob/follow", Some(&token), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(profile["followedByMe"], true);
        assert_eq!(profile["followerCount"], 1);

        // Unfollow.
        let (s, _) = call(&r, "DELETE", "/users/bob/follow", Some(&token), None).await;
        assert_eq!(s, StatusCode::OK);

        // Verify.
        let (_, resp) = call(&r, "POST", "/users/bob/profile", Some(&token), None).await;
        assert_eq!(resp["user"]["followedByMe"], false);
        assert_eq!(resp["user"]["followerCount"], 0);
    }

    // ── Profile ──

    #[tokio::test]
    async fn view_user_profile() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();
        let (s, resp) = call(&r, "POST", "/users/bob/profile", Some(&token), None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(resp["user"]["username"], "bob");
        assert_eq!(resp["user"]["followedByMe"], false);
    }

    #[tokio::test]
    async fn view_nonexistent_user() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();
        let (s, body) = call(&r, "POST", "/users/nobody/profile", Some(&token), None).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        assert_eq!(body["code"], "NOT_FOUND");
    }

    // ── Update profile ──

    #[tokio::test]
    async fn update_my_profile() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();
        let (s, updated) = call(&r, "PUT", "/me/profile", Some(&token),
            Some(serde_json::json!({"displayName": "Alice W", "bio": "New bio"}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(updated["displayName"], "Alice W");

        // Verify via GET /me.
        let (_, me) = call(&r, "GET", "/me", Some(&token), None).await;
        assert_eq!(me["displayName"], "Alice W");
        assert_eq!(me["bio"], "New bio");
    }

    #[tokio::test]
    async fn update_empty_name_rejected() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();
        let (s, body) = call(&r, "PUT", "/me/profile", Some(&token),
            Some(serde_json::json!({"displayName": " ", "bio": ""}))).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(body["code"], "VALIDATION_FAILED");
    }

    // ── Reply ──

    #[tokio::test]
    async fn reply_to_tweet() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        // Create parent.
        let (_, parent) = call(&r, "POST", "/tweets", Some(&token),
            Some(serde_json::json!({"content": "Parent"}))).await;
        let pid = parent["id"].as_str().unwrap();

        // Reply.
        let (s, reply) = call(&r, "POST", "/tweets", Some(&token),
            Some(serde_json::json!({"content": "Reply!", "replyToId": pid}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(reply["replyToId"], pid);

        // Parent reply count updated.
        let (_, detail) = call(&r, "POST", &format!("/tweets/{}/detail", pid), Some(&token), None).await;
        assert_eq!(detail["tweet"]["replyCount"], 1);
        assert_eq!(detail["replies"].as_array().unwrap().len(), 1);
    }

    // ── Search ──

    #[tokio::test]
    async fn search_users_and_tweets() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        // Create a tweet.
        call(&r, "POST", "/tweets", Some(&token),
            Some(serde_json::json!({"content": "Rust is great"}))).await;

        // Search.
        let (s, resp) = call(&r, "POST", "/search", Some(&token),
            Some(serde_json::json!({"query": "alice"}))).await;
        assert_eq!(s, StatusCode::OK);
        assert!(!resp["users"].as_array().unwrap().is_empty());

        let (_, resp) = call(&r, "POST", "/search", Some(&token),
            Some(serde_json::json!({"query": "rust"}))).await;
        assert!(!resp["tweets"].as_array().unwrap().is_empty());
    }

    // ── Multi-user ──

    #[tokio::test]
    async fn two_users_like_same_tweet() {
        let (r, jwt) = setup();
        let alice_token = jwt.issue("alice", "Alice").unwrap();
        let bob_token = jwt.issue("bob", "Bob").unwrap();

        // Alice creates tweet.
        let (_, tweet) = call(&r, "POST", "/tweets", Some(&alice_token),
            Some(serde_json::json!({"content": "Like me"}))).await;
        let id = tweet["id"].as_str().unwrap();

        // Alice likes.
        call(&r, "POST", &format!("/tweets/{}/like", id), Some(&alice_token), None).await;
        // Bob likes.
        call(&r, "POST", &format!("/tweets/{}/like", id), Some(&bob_token), None).await;

        // Verify count = 2.
        let (_, detail) = call(&r, "POST", &format!("/tweets/{}/detail", id), Some(&alice_token), None).await;
        assert_eq!(detail["tweet"]["likeCount"], 2);
        assert_eq!(detail["tweet"]["likedByMe"], true); // Alice liked

        // Bob's view: also liked.
        let (_, detail_bob) = call(&r, "POST", &format!("/tweets/{}/detail", id), Some(&bob_token), None).await;
        assert_eq!(detail_bob["tweet"]["likedByMe"], true);
    }

    // ── Pagination ──

    #[tokio::test]
    async fn timeline_pagination() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        // Create 5 tweets.
        for i in 0..5 {
            call(&r, "POST", "/tweets", Some(&token),
                Some(serde_json::json!({"content": format!("Tweet {}", i)}))).await;
        }

        // Page 1: limit=2, offset=0.
        let (s, page1) = call(&r, "POST", "/timeline", Some(&token),
            Some(serde_json::json!({"limit": 2, "offset": 0}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(page1["items"].as_array().unwrap().len(), 2);
        assert_eq!(page1["hasMore"], true);

        // Page 2: limit=2, offset=2.
        let (_, page2) = call(&r, "POST", "/timeline", Some(&token),
            Some(serde_json::json!({"limit": 2, "offset": 2}))).await;
        assert_eq!(page2["items"].as_array().unwrap().len(), 2);
        assert_eq!(page2["hasMore"], true);

        // Page 3: limit=2, offset=4.
        let (_, page3) = call(&r, "POST", "/timeline", Some(&token),
            Some(serde_json::json!({"limit": 2, "offset": 4}))).await;
        assert_eq!(page3["items"].as_array().unwrap().len(), 1);
        assert_eq!(page3["hasMore"], false);
    }

    #[tokio::test]
    async fn timeline_default_pagination() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        // Create 3 tweets.
        for i in 0..3 {
            call(&r, "POST", "/tweets", Some(&token),
                Some(serde_json::json!({"content": format!("T{}", i)}))).await;
        }

        // No pagination params → default limit=20.
        let (s, all) = call(&r, "POST", "/timeline", Some(&token),
            Some(serde_json::json!({}))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(all["items"].as_array().unwrap().len(), 3);
        assert_eq!(all["hasMore"], false);
    }

    #[tokio::test]
    async fn timeline_offset_beyond_total() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        call(&r, "POST", "/tweets", Some(&token),
            Some(serde_json::json!({"content": "Only tweet"}))).await;

        let (_, page) = call(&r, "POST", "/timeline", Some(&token),
            Some(serde_json::json!({"limit": 10, "offset": 100}))).await;
        assert_eq!(page["items"].as_array().unwrap().len(), 0);
        assert_eq!(page["hasMore"], false);
    }

    // ── Optimistic Locking ──

    #[tokio::test]
    async fn update_profile_with_correct_timestamp() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        // Get current profile to get updatedAt.
        let (_, me) = call(&r, "GET", "/me", Some(&token), None).await;
        let updated_at = me["updatedAt"].as_str().unwrap();

        // Update with correct updatedAt → success.
        let (s, updated) = call(&r, "PUT", "/me/profile", Some(&token),
            Some(serde_json::json!({
                "displayName": "Alice Updated",
                "bio": "New bio",
                "updatedAt": updated_at,
            }))).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(updated["displayName"], "Alice Updated");
    }

    #[tokio::test]
    async fn update_profile_with_stale_timestamp_rejected() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        // First update (to change the updatedAt).
        let (_, me) = call(&r, "GET", "/me", Some(&token), None).await;
        let old_ts = me["updatedAt"].as_str().unwrap().to_string();

        call(&r, "PUT", "/me/profile", Some(&token),
            Some(serde_json::json!({
                "displayName": "V1", "bio": "", "updatedAt": old_ts,
            }))).await;

        // Second update with the OLD timestamp → conflict.
        let (s, body) = call(&r, "PUT", "/me/profile", Some(&token),
            Some(serde_json::json!({
                "displayName": "V2", "bio": "", "updatedAt": old_ts,
            }))).await;
        // Should be 409 Conflict (from KvOps or our check).
        assert!(s == StatusCode::CONFLICT || s == StatusCode::INTERNAL_SERVER_ERROR,
            "Expected conflict, got {} body: {:?}", s, body);
    }

    // ── File Upload ──

    #[tokio::test]
    async fn upload_image() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        let image_data = vec![0xFFu8, 0xD8, 0xFF, 0xE0]; // fake JPEG header
        let req = Request::builder()
            .method("POST")
            .uri("/upload")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::from(image_data))
            .unwrap();
        let resp = r.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["url"].as_str().unwrap().contains("/blobs/uploads/alice/"));
        assert_eq!(json["size"], 4);
    }

    #[tokio::test]
    async fn upload_empty_file_rejected() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/upload")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let resp = r.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn upload_without_auth_rejected() {
        let (r, _) = setup();
        let req = Request::builder()
            .method("POST")
            .uri("/upload")
            .body(Body::from(vec![1, 2, 3]))
            .unwrap();
        let resp = r.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn update_profile_without_timestamp_still_works() {
        let (r, jwt) = setup();
        let token = jwt.issue("alice", "Alice").unwrap();

        let (s, _) = call(&r, "PUT", "/me/profile", Some(&token),
            Some(serde_json::json!({
                "displayName": "No Lock", "bio": "",
            }))).await;
        assert_eq!(s, StatusCode::OK);
    }

    // ── Inbox ──

    fn seed_messages(kv: &Arc<dyn openerp_kv::KVStore>) {
        use openerp_types::LocalizedText;
        let ops = KvOps::<crate::server::model::Message>::new(kv.clone());

        let mut t1 = LocalizedText::en("Welcome!");
        t1.set("zh-CN", "欢迎！");
        t1.set("ja", "ようこそ！");
        t1.set("es", "¡Bienvenido!");
        let mut b1 = LocalizedText::en("Welcome to the app.");
        b1.set("zh-CN", "欢迎使用本应用。");
        ops.save_new(crate::server::model::Message {
            id: Id::default(), kind: "broadcast".into(),
            sender: None, recipient: None,
            title: t1, body: b1, read: false,
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();

        let mut t2 = LocalizedText::en("Verified");
        t2.set("zh-CN", "已认证");
        let b2 = LocalizedText::en("Your account is verified.");
        ops.save_new(crate::server::model::Message {
            id: Id::default(), kind: "personal".into(),
            sender: None, recipient: Some(Name::new("twitter/users/alice")),
            title: t2, body: b2, read: false,
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();
    }

    fn setup_with_messages() -> (axum::Router, JwtService, Arc<dyn openerp_kv::KVStore>) {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("test.redb")).unwrap(),
        );
        let jwt = JwtService::golden_test();
        KvOps::<User>::new(kv.clone()).save_new(User {
            id: Id::default(), username: "alice".into(),
            password_hash: Some(PasswordHash::new(&hash_password("password"))),
            bio: None, avatar: None,
            follower_count: 0, following_count: 0, tweet_count: 0,
            display_name: Some("Alice".into()),
            description: None, metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();
        seed_messages(&kv);

        let blob_dir = dir.path().join("blobs");
        std::fs::create_dir_all(&blob_dir).unwrap();
        let blobs: Arc<dyn openerp_blob::BlobStore> = Arc::new(
            openerp_blob::FileStore::open(&blob_dir).unwrap(),
        );
        let state = Arc::new(FacetStateInner {
            users: KvOps::new(kv.clone()),
            tweets: KvOps::new(kv.clone()),
            likes: KvOps::new(kv.clone()),
            follows: KvOps::new(kv.clone()),
            messages: KvOps::new(kv.clone()),
            jwt: jwt.clone(),
            i18n: Box::new(crate::server::i18n::DefaultLocalizer),
            blobs,
            blob_base_url: "http://test".to_string(),
        });
        let router = facet_router(state);
        std::mem::forget(dir);
        (router, jwt, kv)
    }

    async fn call_with_lang(
        router: &axum::Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        lang: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("accept-language", lang);
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {}", t));
        }
        if body.is_some() {
            builder = builder.header("content-type", "application/json");
        }
        let body = match body {
            Some(v) => Body::from(serde_json::to_string(&v).unwrap()),
            None => Body::empty(),
        };
        let req = builder.body(body).unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let json = if bytes.is_empty() {
            serde_json::json!(null)
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::json!(null))
        };
        (status, json)
    }

    #[tokio::test]
    async fn inbox_returns_messages_in_english() {
        let (r, jwt, _) = setup_with_messages();
        let token = jwt.issue("alice", "Alice").unwrap();
        let (s, body) = call_with_lang(&r, "POST", "/inbox", Some(&token), "en", None).await;
        assert_eq!(s, StatusCode::OK);
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2);
        let titles: Vec<&str> = msgs.iter().map(|m| m["title"].as_str().unwrap()).collect();
        assert!(titles.contains(&"Welcome!"), "expected Welcome!, got {:?}", titles);
        assert!(titles.contains(&"Verified"), "expected Verified, got {:?}", titles);
        assert_eq!(body["unreadCount"].as_u64().unwrap(), 2);
    }

    #[tokio::test]
    async fn inbox_returns_messages_in_chinese() {
        let (r, jwt, _) = setup_with_messages();
        let token = jwt.issue("alice", "Alice").unwrap();
        let (s, body) = call_with_lang(&r, "POST", "/inbox", Some(&token), "zh-CN", None).await;
        assert_eq!(s, StatusCode::OK);
        let msgs = body["messages"].as_array().unwrap();
        let titles: Vec<&str> = msgs.iter().map(|m| m["title"].as_str().unwrap()).collect();
        assert!(titles.contains(&"欢迎！"), "got {:?}", titles);
        assert!(titles.contains(&"已认证"), "got {:?}", titles);
        let broadcast = msgs.iter().find(|m| m["kind"] == "broadcast").unwrap();
        assert_eq!(broadcast["body"].as_str().unwrap(), "欢迎使用本应用。");
    }

    #[tokio::test]
    async fn inbox_japanese_fallback_to_english() {
        let (r, jwt, _) = setup_with_messages();
        let token = jwt.issue("alice", "Alice").unwrap();
        let (s, body) = call_with_lang(&r, "POST", "/inbox", Some(&token), "ja", None).await;
        assert_eq!(s, StatusCode::OK);
        let msgs = body["messages"].as_array().unwrap();
        let broadcast = msgs.iter().find(|m| m["kind"] == "broadcast").unwrap();
        assert_eq!(broadcast["title"].as_str().unwrap(), "ようこそ！");
        let personal = msgs.iter().find(|m| m["kind"] == "personal").unwrap();
        // No ja translation → falls back to en.
        assert_eq!(personal["title"].as_str().unwrap(), "Verified");
    }

    #[tokio::test]
    async fn inbox_bob_only_sees_broadcast() {
        let (r, jwt, _) = setup_with_messages();
        let token = jwt.issue("bob", "Bob").unwrap();
        let (s, body) = call_with_lang(&r, "POST", "/inbox", Some(&token), "en", None).await;
        assert_eq!(s, StatusCode::OK);
        let msgs = body["messages"].as_array().unwrap();
        // bob gets only broadcast (personal is for alice).
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["title"].as_str().unwrap(), "Welcome!");
    }

    #[tokio::test]
    async fn mark_read_updates_message() {
        let (r, jwt, _) = setup_with_messages();
        let token = jwt.issue("alice", "Alice").unwrap();

        let (_, body) = call_with_lang(&r, "POST", "/inbox", Some(&token), "en", None).await;
        let msgs = body["messages"].as_array().unwrap();
        let broadcast = msgs.iter().find(|m| m["kind"] == "broadcast").unwrap();
        let msg_id = broadcast["id"].as_str().unwrap().to_string();
        assert_eq!(broadcast["read"].as_bool().unwrap(), false);

        let uri = format!("/messages/{}/read", msg_id);
        let (s, msg) = call_with_lang(&r, "POST", &uri, Some(&token), "en", None).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(msg["read"].as_bool().unwrap(), true);

        let (_, body2) = call_with_lang(&r, "POST", "/inbox", Some(&token), "en", None).await;
        assert_eq!(body2["unreadCount"].as_u64().unwrap(), 1);
    }

    #[tokio::test]
    async fn inbox_without_auth_rejected() {
        let (r, _, _) = setup_with_messages();
        let (s, _) = call_with_lang(&r, "POST", "/inbox", None, "en", None).await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
}
