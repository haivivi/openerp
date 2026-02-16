//! Facet handler implementations for the "app" facet.
//!
//! Each handler is a hand-written axum handler. No auto-CRUD.
//! Current user identity comes from JWT (x-user-id header for golden test).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;

use openerp_store::KvOps;
use openerp_types::*;

use crate::server::model::*;
use crate::server::rest_app::app::*;

/// Shared state for facet handlers.
/// Wrapped in Arc inside the router (axum State requires Clone).
pub struct FacetStateInner {
    pub users: KvOps<User>,
    pub tweets: KvOps<Tweet>,
    pub likes: KvOps<Like>,
    pub follows: KvOps<Follow>,
}

pub type FacetState = Arc<FacetStateInner>;

/// Extract current user ID from request headers.
/// Golden test: uses "x-user-id" header directly.
/// Production: would decode JWT from "Authorization: Bearer <token>".
fn current_user(headers: &HeaderMap) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    // Try x-user-id header first (golden test shortcut).
    if let Some(uid) = headers.get("x-user-id").and_then(|v| v.to_str().ok()) {
        return Ok(uid.to_string());
    }
    // Try JWT from Authorization header.
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            // Simple JWT decode (no verification for golden test).
            if let Some(payload) = token.split('.').nth(1) {
                if let Ok(bytes) = base64_decode(payload) {
                    if let Ok(claims) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                        if let Some(sub) = claims["sub"].as_str() {
                            return Ok(sub.to_string());
                        }
                    }
                }
            }
        }
    }
    Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "not authenticated"}))))
}

fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(input)
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(input))
        .map_err(|_| ())
}

/// Build an AppTweet from a backend Tweet.
fn to_app_tweet(t: &Tweet, uid: &str, state: &FacetState) -> AppTweet {
    let author = state.users.get(&t.author_id).ok().flatten();
    let like_key = format!("{}:{}", uid, t.id);
    let liked = state.likes.get(&like_key).ok().flatten().is_some();

    AppTweet {
        id: t.id.to_string(),
        author_id: t.author_id.to_string(),
        author_username: author.as_ref().map(|u| u.username.clone()).unwrap_or_default(),
        author_display_name: author.as_ref().and_then(|u| u.display_name.clone()),
        author_avatar: author.as_ref().and_then(|u| u.avatar.as_ref().map(|a| a.to_string())),
        content: t.content.clone(),
        like_count: t.like_count,
        liked_by_me: liked,
        reply_count: t.reply_count,
        reply_to_id: t.reply_to_id.as_ref().map(|s| s.to_string()),
        created_at: t.created_at.to_string(),
    }
}

fn to_app_profile(u: &User, uid: &str, state: &FacetState) -> AppProfile {
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
    }
}

// ── Handlers ──

/// POST /auth/login
pub async fn login(
    State(state): State<FacetState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    match state.users.get(&req.username) {
        Ok(Some(user)) => {
            // Issue a simple JWT (golden test — no real password check).
            let now = chrono::Utc::now().timestamp();
            let header = b64("{}");
            let payload = b64(&serde_json::json!({
                "sub": user.id.as_str(),
                "name": user.display_name.as_deref().unwrap_or(&user.username),
                "iat": now, "exp": now + 86400,
            }).to_string());
            let sig = b64("golden-test");
            let token = format!("{}.{}.{}", header, payload, sig);

            (StatusCode::OK, Json(serde_json::to_value(LoginResponse {
                access_token: token,
                token_type: "Bearer".into(),
                expires_in: 86400,
                user: to_app_user(&user),
            }).unwrap()))
        }
        _ => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "user not found"}))),
    }
}

/// GET /me
pub async fn get_me(
    headers: HeaderMap,
    State(state): State<FacetState>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;
    match state.users.get(&uid) {
        Ok(Some(user)) => Ok(Json(to_app_user(&user))),
        _ => Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "user not found"})))),
    }
}

/// POST /timeline
pub async fn get_timeline(
    headers: HeaderMap,
    State(state): State<FacetState>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;
    let mut tweets = state.tweets.list().unwrap_or_default();
    tweets.sort_by(|a, b| b.created_at.as_str().cmp(a.created_at.as_str()));

    let items: Vec<AppTweet> = tweets.iter()
        .filter(|t| t.reply_to_id.is_none())
        .map(|t| to_app_tweet(t, &uid, &state))
        .collect();

    Ok::<_, (StatusCode, Json<serde_json::Value>)>(Json(TimelineResponse {
        items,
        has_more: false,
    }))
}

/// POST /tweets
pub async fn create_tweet(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Json(req): Json<CreateTweetRequest>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;

    if req.content.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "content cannot be empty"}))));
    }
    if req.content.len() > 280 {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "exceeds 280 characters"}))));
    }

    let tweet = Tweet {
        id: Id::default(),
        author_id: Id::new(&uid),
        content: req.content,
        like_count: 0, reply_count: 0,
        reply_to_id: req.reply_to_id.map(|s| Id::new(&s)),
        display_name: None, description: None, metadata: None,
        created_at: DateTime::default(), updated_at: DateTime::default(), rev: 0,
    };

    match state.tweets.save_new(tweet) {
        Ok(created) => {
            if let Ok(Some(mut user)) = state.users.get(&uid) {
                user.tweet_count += 1;
                let _ = state.users.save(user);
            }
            if let Some(ref pid) = created.reply_to_id {
                if let Ok(Some(mut parent)) = state.tweets.get(pid.as_str()) {
                    parent.reply_count += 1;
                    let _ = state.tweets.save(parent);
                }
            }
            Ok(Json(to_app_tweet(&created, &uid, &state)))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))),
    }
}

/// POST /tweets/{id}/detail
pub async fn tweet_detail(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;
    match state.tweets.get(&id) {
        Ok(Some(tweet)) => {
            let item = to_app_tweet(&tweet, &uid, &state);
            let all = state.tweets.list().unwrap_or_default();
            let mut replies: Vec<AppTweet> = all.iter()
                .filter(|t| t.reply_to_id.as_ref().map(|s| s.as_str()) == Some(&id))
                .map(|t| to_app_tweet(t, &uid, &state))
                .collect();
            replies.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            Ok(Json(TweetDetailResponse { tweet: item, replies }))
        }
        _ => Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "tweet not found"})))),
    }
}

/// POST /tweets/{id}/like
pub async fn like_tweet(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;
    let like = Like {
        id: Id::default(),
        user_id: Id::new(&uid),
        tweet_id: Id::new(&id),
        display_name: None, description: None, metadata: None,
        created_at: DateTime::default(), updated_at: DateTime::default(), rev: 0,
    };
    let _ = state.likes.save_new(like); // Idempotent — ignore duplicate.
    if let Ok(Some(mut tweet)) = state.tweets.get(&id) {
        // Recount likes for accuracy.
        let all_likes = state.likes.list().unwrap_or_default();
        tweet.like_count = all_likes.iter().filter(|l| l.tweet_id.as_str() == id).count() as u32;
        let _ = state.tweets.save(tweet.clone());
        Ok(Json(to_app_tweet(&tweet, &uid, &state)))
    } else {
        Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "tweet not found"}))))
    }
}

/// DELETE /tweets/{id}/like
pub async fn unlike_tweet(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;
    let like_key = format!("{}:{}", uid, id);
    let _ = state.likes.delete(&like_key);
    if let Ok(Some(mut tweet)) = state.tweets.get(&id) {
        let all_likes = state.likes.list().unwrap_or_default();
        tweet.like_count = all_likes.iter().filter(|l| l.tweet_id.as_str() == id).count() as u32;
        let _ = state.tweets.save(tweet);
    }
    Ok::<_, (StatusCode, Json<serde_json::Value>)>(StatusCode::OK)
}

/// POST /users/{id}/follow
pub async fn follow_user(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;
    let follow = Follow {
        id: Id::default(),
        follower_id: Id::new(&uid),
        followee_id: Id::new(&id),
        display_name: None, description: None, metadata: None,
        created_at: DateTime::default(), updated_at: DateTime::default(), rev: 0,
    };
    if state.follows.save_new(follow).is_ok() {
        if let Ok(Some(mut me)) = state.users.get(&uid) {
            me.following_count += 1;
            let _ = state.users.save(me);
        }
        if let Ok(Some(mut them)) = state.users.get(&id) {
            them.follower_count += 1;
            let _ = state.users.save(them.clone());
            return Ok(Json(to_app_profile(&them, &uid, &state)));
        }
    }
    Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "user not found"}))))
}

/// DELETE /users/{id}/follow
pub async fn unfollow_user(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;
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
    Ok::<_, (StatusCode, Json<serde_json::Value>)>(StatusCode::OK)
}

/// POST /users/{id}/profile
pub async fn user_profile(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;
    match state.users.get(&id) {
        Ok(Some(user)) => {
            let profile = to_app_profile(&user, &uid, &state);
            let all = state.tweets.list().unwrap_or_default();
            let tweets: Vec<AppTweet> = all.iter()
                .filter(|t| t.author_id.as_str() == id)
                .map(|t| to_app_tweet(t, &uid, &state))
                .collect();
            Ok(Json(UserProfileResponse { user: profile, tweets }))
        }
        _ => Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "user not found"})))),
    }
}

/// PUT /me/profile
pub async fn update_profile(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Json(req): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;
    if req.display_name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "display name cannot be empty"}))));
    }
    match state.users.get(&uid) {
        Ok(Some(mut user)) => {
            user.display_name = Some(req.display_name);
            user.bio = Some(req.bio);
            let _ = state.users.save(user.clone());
            Ok(Json(to_app_user(&user)))
        }
        _ => Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "user not found"})))),
    }
}

/// POST /search
pub async fn search(
    headers: HeaderMap,
    State(state): State<FacetState>,
    Json(req): Json<SearchRequest>,
) -> impl IntoResponse {
    let uid = current_user(&headers)?;
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

    Ok::<_, (StatusCode, Json<serde_json::Value>)>(Json(SearchResponse { users, tweets }))
}

/// Build the facet router.
pub fn facet_router(state: FacetState) -> axum::Router {
    use axum::routing::{get, post, put, delete};

    axum::Router::new()
        .route("/auth/login", post(login))
        .route("/me", get(get_me))
        .route("/me/profile", put(update_profile))
        .route("/timeline", post(get_timeline))
        .route("/tweets", post(create_tweet))
        .route("/tweets/{id}/detail", post(tweet_detail))
        .route("/tweets/{id}/like", post(like_tweet).delete(unlike_tweet))
        .route("/users/{id}/follow", post(follow_user).delete(unfollow_user))
        .route("/users/{id}/profile", post(user_profile))
        .route("/search", post(search))
        .with_state(state)
}

fn b64(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s.as_bytes())
}
