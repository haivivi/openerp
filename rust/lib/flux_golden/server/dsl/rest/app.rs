//! "app" facet — API surface for the Twitter mobile/web app.
//!
//! All endpoints operate on the **current authenticated user** (from JWT).
//! No user ID needed in requests — identity comes from the token.
//!
//! Resources are read-only projections; mutations are actions.
//! Handlers are hand-written in `server/src/facet_handlers.rs`.

#[openerp_macro::facet(name = "app", module = "twitter")]
pub mod app {
    // ── Resource projections (read-only) ────────────────────────────

    /// Current user's own profile.
    /// GET /app/twitter/me
    #[resource(path = "/me", pk = "id")]
    pub struct AppUser {
        pub id: String,
        pub username: String,
        pub display_name: Option<String>,
        pub bio: Option<String>,
        pub avatar: Option<String>,
        pub follower_count: u32,
        pub following_count: u32,
        pub tweet_count: u32,
        pub updated_at: Option<String>,
    }

    /// A tweet in the timeline or detail view.
    #[resource(path = "/tweets", pk = "id")]
    pub struct AppTweet {
        pub id: String,
        pub author_id: String,
        pub author_username: String,
        pub author_display_name: Option<String>,
        pub author_avatar: Option<String>,
        pub content: String,
        pub image_url: Option<String>,
        pub like_count: u32,
        pub liked_by_me: bool,
        pub reply_count: u32,
        pub reply_to_id: Option<String>,
        pub created_at: String,
    }

    /// A user profile (for viewing other users).
    #[resource(path = "/users", pk = "id")]
    pub struct AppProfile {
        pub id: String,
        pub username: String,
        pub display_name: Option<String>,
        pub bio: Option<String>,
        pub avatar: Option<String>,
        pub follower_count: u32,
        pub following_count: u32,
        pub tweet_count: u32,
        pub followed_by_me: bool,
    }

    /// An in-app message (站内信).
    /// The title/body are pre-resolved to the user's language by the handler.
    #[resource(path = "/messages", pk = "id")]
    pub struct AppMessage {
        pub id: String,
        pub kind: String,
        pub title: String,
        pub body: String,
        pub read: bool,
        pub created_at: String,
    }

    // ── Request/Response types ────────────────────────────────────

    /// Login request.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct LoginRequest {
        pub username: String,
        pub password: String,
    }

    /// Login response — contains JWT token.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LoginResponse {
        pub access_token: String,
        pub token_type: String,
        pub expires_in: u64,
        pub user: AppUser,
    }

    /// Create tweet request.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateTweetRequest {
        pub content: String,
        pub reply_to_id: Option<String>,
    }

    /// Timeline response — paginated tweets.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TimelineResponse {
        pub items: Vec<AppTweet>,
        pub has_more: bool,
    }

    /// Tweet detail response — tweet + replies.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct TweetDetailResponse {
        pub tweet: AppTweet,
        pub replies: Vec<AppTweet>,
    }

    /// User profile response — profile + tweets.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct UserProfileResponse {
        pub user: AppProfile,
        pub tweets: Vec<AppTweet>,
    }

    /// Update profile request — includes updatedAt for optimistic locking.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateProfileRequest {
        pub display_name: String,
        pub bio: String,
        /// For optimistic locking — must match server's updatedAt.
        pub updated_at: Option<String>,
    }

    /// Search response.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct SearchResponse {
        pub users: Vec<AppProfile>,
        pub tweets: Vec<AppTweet>,
    }

    // ── Action signatures ────────────────────────────────────────

    /// Login — returns JWT + user profile.
    #[action(method = "POST", path = "/auth/login")]
    pub type Login = fn(req: LoginRequest) -> LoginResponse;

    /// Pagination params for timeline.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PaginationParams {
        #[serde(default = "default_limit")]
        pub limit: usize,
        #[serde(default)]
        pub offset: usize,
    }
    fn default_limit() -> usize { 20 }

    /// Get my timeline (paginated).
    #[action(method = "POST", path = "/timeline")]
    pub type Timeline = fn(req: PaginationParams) -> TimelineResponse;

    /// Create a tweet (author = current user from JWT).
    #[action(method = "POST", path = "/tweets")]
    pub type CreateTweet = fn(req: CreateTweetRequest) -> AppTweet;

    /// Get tweet detail with replies.
    #[action(method = "POST", path = "/tweets/{id}/detail")]
    pub type TweetDetail = fn(id: String) -> TweetDetailResponse;

    /// Like a tweet.
    #[action(method = "POST", path = "/tweets/{id}/like")]
    pub type LikeTweet = fn(id: String) -> AppTweet;

    /// Unlike a tweet.
    #[action(method = "DELETE", path = "/tweets/{id}/like")]
    pub type UnlikeTweet = fn(id: String);

    /// Follow a user.
    #[action(method = "POST", path = "/users/{id}/follow")]
    pub type FollowUser = fn(id: String) -> AppProfile;

    /// Unfollow a user.
    #[action(method = "DELETE", path = "/users/{id}/follow")]
    pub type UnfollowUser = fn(id: String);

    /// Get user profile with their tweets.
    #[action(method = "POST", path = "/users/{id}/profile")]
    pub type UserProfile = fn(id: String) -> UserProfileResponse;

    /// Update my profile.
    #[action(method = "PUT", path = "/me/profile")]
    pub type UpdateProfile = fn(req: UpdateProfileRequest) -> AppUser;

    /// Change password request.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ChangePasswordRequest {
        pub old_password: String,
        pub new_password: String,
    }

    /// Change password response.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ChangePasswordResponse {
        pub ok: bool,
    }

    /// Change current user's password.
    #[action(method = "PUT", path = "/me/password")]
    pub type ChangePassword = fn(req: ChangePasswordRequest) -> ChangePasswordResponse;

    /// Search request.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct SearchRequest {
        pub query: String,
    }

    /// Upload an image attachment. Returns the URL.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UploadResponse {
        pub url: String,
        pub size: u64,
    }

    /// Upload image for a tweet.
    #[action(method = "POST", path = "/upload")]
    pub type Upload = fn() -> UploadResponse;

    /// Search users and tweets.
    #[action(method = "POST", path = "/search")]
    pub type Search = fn(req: SearchRequest) -> SearchResponse;

    /// Inbox response — user's messages.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InboxResponse {
        pub messages: Vec<AppMessage>,
        pub unread_count: usize,
    }

    /// Get inbox (messages for current user).
    /// Language code in header `Accept-Language` selects LocalizedText field.
    #[action(method = "POST", path = "/inbox")]
    pub type Inbox = fn() -> InboxResponse;

    /// Mark a message as read.
    #[action(method = "POST", path = "/messages/{id}/read")]
    pub type MarkRead = fn(id: String) -> AppMessage;
}
