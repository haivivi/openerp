//! Twitter BFF — Flux state engine layer.
//!
//! `TwitterBff` holds the facet client (AppClient).
//! Handlers call the facet API (not admin), then update Flux state.

pub mod global;
pub mod i18n_strings;

use std::sync::Arc;

use flux_derive::flux_handlers;
use openerp_flux::StateStore;

use crate::request::*;
use crate::state::*;
use crate::server::rest_app::app::{self, AppClient, AppTweet};
use self::global::helpers;

/// Mutable token source — stores the JWT from login, used for subsequent calls.
pub struct MutableToken {
    token: tokio::sync::RwLock<Option<String>>,
}

impl MutableToken {
    pub fn new() -> Self {
        Self { token: tokio::sync::RwLock::new(None) }
    }
    pub async fn set(&self, token: String) {
        *self.token.write().await = Some(token);
    }
}

#[async_trait::async_trait]
impl openerp_client::TokenSource for MutableToken {
    async fn token(&self) -> Result<Option<String>, openerp_client::ApiError> {
        Ok(self.token.read().await.clone())
    }
}

/// BFF context — holds the facet client + mutable token + locale.
pub struct TwitterBff {
    pub client: AppClient,
    pub token: Arc<MutableToken>,
    pub server_url: String,
    pub locale: tokio::sync::RwLock<String>,
    http: reqwest::Client,
}

impl TwitterBff {
    pub fn new(base_url: &str) -> Self {
        let token = Arc::new(MutableToken::new());
        let ts: Arc<dyn openerp_client::TokenSource> = token.clone();
        Self {
            client: AppClient::new(base_url, ts),
            token,
            server_url: base_url.to_string(),
            locale: tokio::sync::RwLock::new("en".into()),
            http: reqwest::Client::new(),
        }
    }

    /// Fetch current user profile (GET /me with JWT).
    async fn fetch_me(&self) -> Option<app::AppUser> {
        let token = self.token.token.read().await.clone();
        let url = format!("{}/app/twitter/me", self.server_url);
        let mut req = self.http.get(&url);
        if let Some(t) = &token {
            req = req.header("authorization", format!("Bearer {}", t));
        }
        req.send().await.ok()?.json().await.ok()
    }

    /// Fetch inbox with Accept-Language header set to current locale.
    async fn fetch_inbox(&self) -> Result<app::InboxResponse, String> {
        let locale = self.locale.read().await.clone();
        let token = self.token.token.read().await.clone();
        let url = format!("{}/app/twitter/inbox", self.server_url);
        let mut req = self.http.post(&url).header("accept-language", &locale);
        if let Some(t) = &token {
            req = req.header("authorization", format!("Bearer {}", t));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    /// Fetch and mark a message as read.
    async fn fetch_mark_read(&self, id: &str) -> Result<app::AppMessage, String> {
        let locale = self.locale.read().await.clone();
        let token = self.token.token.read().await.clone();
        let url = format!("{}/app/twitter/messages/{}/read", self.server_url, id);
        let mut req = self.http.post(&url).header("accept-language", &locale);
        if let Some(t) = &token {
            req = req.header("authorization", format!("Bearer {}", t));
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        resp.json().await.map_err(|e| e.to_string())
    }
}

/// Convert facet AppTweet to BFF FeedItem.
fn to_feed_item(t: &AppTweet) -> FeedItem {
    FeedItem {
        tweet_id: t.id.clone(),
        author: UserProfile {
            id: t.author_id.clone(),
            username: t.author_username.clone(),
            display_name: t.author_display_name.clone().unwrap_or_default(),
            bio: None,
            avatar: t.author_avatar.clone(),
            follower_count: 0,
            following_count: 0,
            tweet_count: 0,
        },
        content: t.content.clone(),
        like_count: t.like_count,
        liked_by_me: t.liked_by_me,
        reply_count: t.reply_count,
        reply_to_id: t.reply_to_id.clone(),
        created_at: t.created_at.clone(),
    }
}

fn to_user_profile(u: &app::AppUser) -> UserProfile {
    UserProfile {
        id: u.id.clone(),
        username: u.username.clone(),
        display_name: u.display_name.clone().unwrap_or_default(),
        bio: u.bio.clone(),
        avatar: u.avatar.clone(),
        follower_count: u.follower_count,
        following_count: u.following_count,
        tweet_count: u.tweet_count,
    }
}

#[flux_handlers]
impl TwitterBff {
    #[handle(InitializeReq)]
    pub async fn handle_initialize(&self, _req: &InitializeReq, store: &StateStore) {
        store.set(AuthState::PATH, AuthState {
            phase: AuthPhase::Unauthenticated, user: None, busy: false, error: None,
        });
        store.set(AppRoute::PATH, AppRoute("/login".into()));
    }

    #[handle(LoginReq)]
    pub async fn handle_login(&self, req: &LoginReq, store: &StateStore) {
        store.set(AuthState::PATH, AuthState {
            phase: AuthPhase::Unauthenticated, user: None, busy: true, error: None,
        });

        let login_req = app::LoginRequest {
            username: req.username.clone(),
            password: req.password.clone(),
        };

        match self.client.login(&login_req).await {
            Ok(resp) => {
                // Save JWT for subsequent authenticated requests.
                self.token.set(resp.access_token.clone()).await;

                let profile = to_user_profile(&resp.user);
                store.set(AuthState::PATH, AuthState {
                    phase: AuthPhase::Authenticated, user: Some(profile),
                    busy: false, error: None,
                });
                store.set(AppRoute::PATH, AppRoute("/home".into()));
                // Load timeline (now authenticated — token is set).
                if let Ok(tl) = self.client.timeline(&app::PaginationParams { limit: 50, offset: 0 }).await {
                    store.set(TimelineFeed::PATH, TimelineFeed {
                        items: tl.items.iter().map(to_feed_item).collect(),
                        loading: false, has_more: tl.has_more, error: None,
                    });
                }
            }
            Err(e) => {
                store.set(AuthState::PATH, AuthState {
                    phase: AuthPhase::Unauthenticated, user: None, busy: false,
                    error: Some(format!("User '{}' not found", req.username)),
                });
            }
        }
    }

    #[handle(LogoutReq)]
    pub async fn handle_logout(&self, _req: &LogoutReq, store: &StateStore) {
        store.set(AuthState::PATH, AuthState {
            phase: AuthPhase::Unauthenticated, user: None, busy: false, error: None,
        });
        store.set(AppRoute::PATH, AppRoute("/login".into()));
        store.remove(TimelineFeed::PATH);
        store.remove(ComposeState::PATH);
    }

    #[handle(TimelineLoadReq)]
    pub async fn handle_timeline_load(&self, _req: &TimelineLoadReq, store: &StateStore) {
        store.set(TimelineFeed::PATH, TimelineFeed {
            items: vec![], loading: true, has_more: false, error: None,
        });
        if let Ok(tl) = self.client.timeline(&app::PaginationParams { limit: 50, offset: 0 }).await {
            store.set(TimelineFeed::PATH, TimelineFeed {
                items: tl.items.iter().map(to_feed_item).collect(),
                loading: false, has_more: tl.has_more, error: None,
            });
        }
    }

    #[handle(CreateTweetReq)]
    pub async fn handle_create_tweet(&self, req: &CreateTweetReq, store: &StateStore) {
        if req.content.trim().is_empty() {
            store.set(ComposeState::PATH, ComposeState {
                content: req.content.clone(), reply_to_id: req.reply_to_id.clone(),
                busy: false, error: Some("Tweet cannot be empty".into()),
            });
            return;
        }
        if req.content.len() > 280 {
            store.set(ComposeState::PATH, ComposeState {
                content: req.content.clone(), reply_to_id: req.reply_to_id.clone(),
                busy: false, error: Some("Tweet exceeds 280 characters".into()),
            });
            return;
        }
        store.set(ComposeState::PATH, ComposeState {
            content: req.content.clone(), reply_to_id: req.reply_to_id.clone(),
            busy: true, error: None,
        });

        let create_req = app::CreateTweetRequest {
            content: req.content.clone(),
            reply_to_id: req.reply_to_id.clone(),
        };

        match self.client.create_tweet(&create_req).await {
            Ok(_) => {
                store.set(ComposeState::PATH, ComposeState::empty());
                if let Ok(tl) = self.client.timeline(&app::PaginationParams { limit: 50, offset: 0 }).await {
                    store.set(TimelineFeed::PATH, TimelineFeed {
                        items: tl.items.iter().map(to_feed_item).collect(),
                        loading: false, has_more: tl.has_more, error: None,
                    });
                }
            }
            Err(e) => {
                store.set(ComposeState::PATH, ComposeState {
                    content: req.content.clone(), reply_to_id: req.reply_to_id.clone(),
                    busy: false, error: Some(e.to_string()),
                });
            }
        }
    }

    #[handle(LikeTweetReq)]
    pub async fn handle_like(&self, req: &LikeTweetReq, store: &StateStore) {
        let _ = self.client.like_tweet(&req.tweet_id).await;
        if let Ok(tl) = self.client.timeline(&app::PaginationParams { limit: 50, offset: 0 }).await {
            store.set(TimelineFeed::PATH, TimelineFeed {
                items: tl.items.iter().map(to_feed_item).collect(),
                loading: false, has_more: tl.has_more, error: None,
            });
        }
    }

    #[handle(UnlikeTweetReq)]
    pub async fn handle_unlike(&self, req: &UnlikeTweetReq, store: &StateStore) {
        let _ = self.client.unlike_tweet(&req.tweet_id).await;
        if let Ok(tl) = self.client.timeline(&app::PaginationParams { limit: 50, offset: 0 }).await {
            store.set(TimelineFeed::PATH, TimelineFeed {
                items: tl.items.iter().map(to_feed_item).collect(),
                loading: false, has_more: tl.has_more, error: None,
            });
        }
    }

    #[handle(LoadTweetReq)]
    pub async fn handle_load_tweet(&self, req: &LoadTweetReq, store: &StateStore) {
        if let Ok(detail) = self.client.tweet_detail(&req.tweet_id).await {
            store.set(&TweetDetail::path(&req.tweet_id), TweetDetail {
                tweet: to_feed_item(&detail.tweet),
                replies: detail.replies.iter().map(to_feed_item).collect(),
                loading: false,
            });
            store.set(AppRoute::PATH, AppRoute(format!("/tweet/{}", req.tweet_id)));
        }
    }

    #[handle(FollowUserReq)]
    pub async fn handle_follow(&self, req: &FollowUserReq, store: &StateStore) {
        let _ = self.client.follow_user(&req.user_id).await;
        // Refresh my profile.
        if let Some(me) = self.fetch_me().await {
            store.set(AuthState::PATH, AuthState {
                phase: AuthPhase::Authenticated,
                user: Some(to_user_profile(&me)),
                busy: false, error: None,
            });
        }
    }

    #[handle(UnfollowUserReq)]
    pub async fn handle_unfollow(&self, req: &UnfollowUserReq, store: &StateStore) {
        let _ = self.client.unfollow_user(&req.user_id).await;
        if let Some(me) = self.fetch_me().await {
            store.set(AuthState::PATH, AuthState {
                phase: AuthPhase::Authenticated,
                user: Some(to_user_profile(&me)),
                busy: false, error: None,
            });
        }
    }

    #[handle(LoadProfileReq)]
    pub async fn handle_load_profile(&self, req: &LoadProfileReq, store: &StateStore) {
        if let Ok(resp) = self.client.user_profile(&req.user_id).await {
            let profile = UserProfile {
                id: resp.user.id.clone(),
                username: resp.user.username.clone(),
                display_name: resp.user.display_name.clone().unwrap_or_default(),
                bio: resp.user.bio.clone(),
                avatar: resp.user.avatar.clone(),
                follower_count: resp.user.follower_count,
                following_count: resp.user.following_count,
                tweet_count: resp.user.tweet_count,
            };
            store.set(&ProfilePage::path(&req.user_id), ProfilePage {
                user: profile,
                tweets: resp.tweets.iter().map(to_feed_item).collect(),
                followed_by_me: resp.user.followed_by_me,
                loading: false,
            });
            store.set(AppRoute::PATH, AppRoute(format!("/profile/{}", req.user_id)));
        }
    }

    #[handle(ComposeUpdateReq)]
    pub async fn handle_compose_update(&self, req: &ComposeUpdateReq, store: &StateStore) {
        let mut state = store.get(ComposeState::PATH)
            .and_then(|v| v.downcast_ref::<ComposeState>().cloned())
            .unwrap_or_else(ComposeState::empty);
        match req.field.as_str() {
            "content" => state.content = req.value.clone(),
            _ => {}
        }
        state.error = None;
        store.set(ComposeState::PATH, state);
    }

    #[handle(SearchReq)]
    pub async fn handle_search(&self, req: &SearchReq, store: &StateStore) {
        if req.query.is_empty() {
            store.set(SearchState::PATH, SearchState {
                query: String::new(), users: vec![], tweets: vec![],
                loading: false, error: None,
            });
            return;
        }
        store.set(SearchState::PATH, SearchState {
            query: req.query.clone(), users: vec![], tweets: vec![],
            loading: true, error: None,
        });
        let search_req = app::SearchRequest { query: req.query.clone() };
        if let Ok(resp) = self.client.search(&search_req).await {
            let users: Vec<UserProfile> = resp.users.iter().map(|u| UserProfile {
                id: u.id.clone(), username: u.username.clone(),
                display_name: u.display_name.clone().unwrap_or_default(),
                bio: u.bio.clone(), avatar: u.avatar.clone(),
                follower_count: u.follower_count, following_count: u.following_count,
                tweet_count: u.tweet_count,
            }).collect();
            store.set(SearchState::PATH, SearchState {
                query: req.query.clone(), users,
                tweets: resp.tweets.iter().map(to_feed_item).collect(),
                loading: false, error: None,
            });
        }
    }

    #[handle(SearchClearReq)]
    pub async fn handle_search_clear(&self, _req: &SearchClearReq, store: &StateStore) {
        store.remove(SearchState::PATH);
    }

    #[handle(SettingsLoadReq)]
    pub async fn handle_settings_load(&self, _req: &SettingsLoadReq, store: &StateStore) {
        if let Some(me) = self.fetch_me().await {
            store.set(SettingsState::PATH, SettingsState {
                display_name: me.display_name.clone().unwrap_or_default(),
                bio: me.bio.clone().unwrap_or_default(),
                busy: false, saved: false, error: None,
            });
        }
        store.set(AppRoute::PATH, AppRoute("/settings".into()));
    }

    #[handle(SettingsSaveReq)]
    pub async fn handle_settings_save(&self, req: &SettingsSaveReq, store: &StateStore) {
        if req.display_name.trim().is_empty() {
            store.set(SettingsState::PATH, SettingsState {
                display_name: req.display_name.clone(), bio: req.bio.clone(),
                busy: false, saved: false,
                error: Some("Display name cannot be empty".into()),
            });
            return;
        }
        let update_req = app::UpdateProfileRequest {
            display_name: req.display_name.clone(),
            bio: req.bio.clone(),
            updated_at: None, // BFF doesn't do optimistic locking (for simplicity).
        };
        match self.client.update_profile(&update_req).await {
            Ok(user) => {
                store.set(AuthState::PATH, AuthState {
                    phase: AuthPhase::Authenticated,
                    user: Some(to_user_profile(&user)),
                    busy: false, error: None,
                });
                store.set(SettingsState::PATH, SettingsState {
                    display_name: req.display_name.clone(), bio: req.bio.clone(),
                    busy: false, saved: true, error: None,
                });
            }
            Err(e) => {
                store.set(SettingsState::PATH, SettingsState {
                    display_name: req.display_name.clone(), bio: req.bio.clone(),
                    busy: false, saved: false, error: Some(e.to_string()),
                });
            }
        }
    }

    #[handle(SetLocaleReq)]
    pub async fn handle_set_locale(&self, req: &SetLocaleReq, store: &StateStore) {
        *self.locale.write().await = req.locale.clone();
        // Reload inbox with new language.
        if let Ok(resp) = self.fetch_inbox().await {
            let msgs: Vec<InboxMessage> = resp.messages.iter().map(|m| InboxMessage {
                id: m.id.clone(), kind: m.kind.clone(),
                title: m.title.clone(), body: m.body.clone(),
                read: m.read, created_at: m.created_at.clone(),
            }).collect();
            let unread = msgs.iter().filter(|m| !m.read).count();
            store.set(InboxState::PATH, InboxState {
                messages: msgs, unread_count: unread, loading: false, error: None,
            });
        }
    }

    #[handle(InboxLoadReq)]
    pub async fn handle_inbox_load(&self, _req: &InboxLoadReq, store: &StateStore) {
        store.set(InboxState::PATH, InboxState {
            messages: vec![], unread_count: 0, loading: true, error: None,
        });
        match self.fetch_inbox().await {
            Ok(resp) => {
                let msgs: Vec<InboxMessage> = resp.messages.iter().map(|m| InboxMessage {
                    id: m.id.clone(), kind: m.kind.clone(),
                    title: m.title.clone(), body: m.body.clone(),
                    read: m.read, created_at: m.created_at.clone(),
                }).collect();
                let unread = msgs.iter().filter(|m| !m.read).count();
                store.set(InboxState::PATH, InboxState {
                    messages: msgs, unread_count: unread, loading: false, error: None,
                });
            }
            Err(e) => {
                store.set(InboxState::PATH, InboxState {
                    messages: vec![], unread_count: 0, loading: false,
                    error: Some(e),
                });
            }
        }
    }

    #[handle(InboxMarkReadReq)]
    pub async fn handle_inbox_mark_read(&self, req: &InboxMarkReadReq, store: &StateStore) {
        let _ = self.fetch_mark_read(&req.message_id).await;
        if let Ok(resp) = self.fetch_inbox().await {
            let msgs: Vec<InboxMessage> = resp.messages.iter().map(|m| InboxMessage {
                id: m.id.clone(), kind: m.kind.clone(),
                title: m.title.clone(), body: m.body.clone(),
                read: m.read, created_at: m.created_at.clone(),
            }).collect();
            let unread = msgs.iter().filter(|m| !m.read).count();
            store.set(InboxState::PATH, InboxState {
                messages: msgs, unread_count: unread, loading: false, error: None,
            });
        }
    }

    #[handle(ChangePasswordReq)]
    pub async fn handle_change_password(&self, req: &ChangePasswordReq, store: &StateStore) {
        let change_req = app::ChangePasswordRequest {
            old_password: req.old_password.clone(),
            new_password: req.new_password.clone(),
        };
        match self.client.change_password(&change_req).await {
            Ok(_) => {
                store.set(PasswordState::PATH, PasswordState {
                    busy: false, success: true, error: None,
                });
            }
            Err(e) => {
                store.set(PasswordState::PATH, PasswordState {
                    busy: false, success: false, error: Some(e.to_string()),
                });
            }
        }
    }
}

// register() is GENERATED by #[flux_handlers] macro.
