//! Twitter BFF — Flux state engine layer.
//!
//! `TwitterBff` holds HTTP client (not KvOps). Handlers call the
//! backend server via typed REST API, then update Flux state.

pub mod global;

use flux_derive::flux_handlers;
use openerp_client::ResourceClient;
use openerp_flux::StateStore;

use crate::request::*;
use crate::state::*;
use crate::server::model;
use self::global::helpers;

/// BFF context — holds typed HTTP clients for each resource.
/// The backend runs as an HTTP server; BFF talks to it over HTTP.
pub struct TwitterBff {
    pub users: ResourceClient<model::User>,
    pub tweets: ResourceClient<model::Tweet>,
    pub likes: ResourceClient<model::Like>,
    pub follows: ResourceClient<model::Follow>,
    /// The server's URL (for opening admin dashboard in browser).
    pub server_url: String,
}

impl TwitterBff {
    /// Create a new BFF connected to a backend at the given URL.
    pub fn new(base_url: &str) -> Self {
        let http = reqwest::Client::new();
        let admin = format!("{}/admin/twitter", base_url);
        Self {
            users: ResourceClient::new(&http, &admin, "users"),
            tweets: ResourceClient::new(&http, &admin, "tweets"),
            likes: ResourceClient::new(&http, &admin, "likes"),
            follows: ResourceClient::new(&http, &admin, "follows"),
            server_url: base_url.to_string(),
        }
    }
}

// ── Handler methods ──

#[flux_handlers]
impl TwitterBff {
    fn current_user_id(&self, store: &StateStore) -> String {
        store.get(AuthState::PATH)
            .and_then(|v| v.downcast_ref::<AuthState>()
                .and_then(|a| a.user.as_ref().map(|u| u.id.clone())))
            .unwrap_or_default()
    }

    async fn reload_timeline(&self, uid: &str, store: &StateStore) {
        let mut tweets = self.tweets.list().await.unwrap_or_default();
        let users = self.users.list().await.unwrap_or_default();
        let likes = self.likes.list().await.unwrap_or_default();
        let feed = helpers::build_timeline(uid, &mut tweets, &users, &likes);
        store.set(TimelineFeed::PATH, feed);
    }

    // ── App lifecycle ──

    #[handle(InitializeReq)]
    pub async fn handle_initialize(&self, _req: &InitializeReq, store: &StateStore) {
        store.set(AuthState::PATH, AuthState {
            phase: AuthPhase::Unauthenticated, user: None, busy: false, error: None,
        });
        store.set(AppRoute::PATH, AppRoute("/login".into()));
    }

    // ── Auth ──

    #[handle(LoginReq)]
    pub async fn handle_login(&self, req: &LoginReq, store: &StateStore) {
        store.set(AuthState::PATH, AuthState {
            phase: AuthPhase::Unauthenticated, user: None, busy: true, error: None,
        });
        match self.users.get(&req.username).await {
            Ok(user) => {
                let profile = helpers::user_to_profile(&user);
                let uid = profile.id.clone();
                store.set(AuthState::PATH, AuthState {
                    phase: AuthPhase::Authenticated, user: Some(profile),
                    busy: false, error: None,
                });
                store.set(AppRoute::PATH, AppRoute("/home".into()));
                self.reload_timeline(&uid, store).await;
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

    // ── Timeline ──

    #[handle(TimelineLoadReq)]
    pub async fn handle_timeline_load(&self, _req: &TimelineLoadReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        store.set(TimelineFeed::PATH, TimelineFeed {
            items: vec![], loading: true, has_more: false, error: None,
        });
        self.reload_timeline(&uid, store).await;
    }

    // ── Tweet CRUD ──

    #[handle(CreateTweetReq)]
    pub async fn handle_create_tweet(&self, req: &CreateTweetReq, store: &StateStore) {
        let uid = self.current_user_id(store);
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

        let tweet = model::Tweet {
            id: openerp_types::Id::default(),
            author_id: openerp_types::Id::new(&uid),
            content: req.content.clone(),
            like_count: 0, reply_count: 0,
            reply_to_id: req.reply_to_id.as_ref().map(|s| openerp_types::Id::new(s)),
            display_name: None, description: None, metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };

        match self.tweets.create(&tweet).await {
            Ok(created) => {
                // Update author tweet count.
                if let Ok(mut user) = self.users.get(&uid).await {
                    user.tweet_count += 1;
                    let _ = self.users.update(&uid, &user).await;
                }
                // Update parent reply count.
                if let Some(ref parent_id) = req.reply_to_id {
                    if let Ok(mut parent) = self.tweets.get(parent_id).await {
                        parent.reply_count += 1;
                        let _ = self.tweets.update(parent_id, &parent).await;
                    }
                }
                store.set(ComposeState::PATH, ComposeState::empty());
                self.reload_timeline(&uid, store).await;
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
        let uid = self.current_user_id(store);
        let like = model::Like {
            id: openerp_types::Id::default(),
            user_id: openerp_types::Id::new(&uid),
            tweet_id: openerp_types::Id::new(&req.tweet_id),
            display_name: None, description: None, metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };
        if self.likes.create(&like).await.is_ok() {
            if let Ok(mut tweet) = self.tweets.get(&req.tweet_id).await {
                tweet.like_count += 1;
                let _ = self.tweets.update(&req.tweet_id, &tweet).await;
            }
            self.reload_timeline(&uid, store).await;
        }
    }

    #[handle(UnlikeTweetReq)]
    pub async fn handle_unlike(&self, req: &UnlikeTweetReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        let like_key = format!("{}:{}", uid, req.tweet_id);
        if self.likes.delete(&like_key).await.is_ok() {
            if let Ok(mut tweet) = self.tweets.get(&req.tweet_id).await {
                tweet.like_count = tweet.like_count.saturating_sub(1);
                let _ = self.tweets.update(&req.tweet_id, &tweet).await;
            }
            self.reload_timeline(&uid, store).await;
        }
    }

    #[handle(LoadTweetReq)]
    pub async fn handle_load_tweet(&self, req: &LoadTweetReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        if let Ok(tweet) = self.tweets.get(&req.tweet_id).await {
            let users = self.users.list().await.unwrap_or_default();
            let likes = self.likes.list().await.unwrap_or_default();
            let item = helpers::tweet_to_feed_item(&tweet, &uid, &users, &likes);

            let mut all_tweets = self.tweets.list().await.unwrap_or_default();
            let replies: Vec<FeedItem> = all_tweets.iter()
                .filter(|t| t.reply_to_id.as_ref().map(|s| s.as_str()) == Some(&req.tweet_id))
                .map(|t| helpers::tweet_to_feed_item(t, &uid, &users, &likes))
                .collect();

            store.set(&TweetDetail::path(&req.tweet_id), TweetDetail {
                tweet: item, replies, loading: false,
            });
            store.set(AppRoute::PATH, AppRoute(format!("/tweet/{}", req.tweet_id)));
        }
    }

    // ── Social ──

    #[handle(FollowUserReq)]
    pub async fn handle_follow(&self, req: &FollowUserReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        let follow = model::Follow {
            id: openerp_types::Id::default(),
            follower_id: openerp_types::Id::new(&uid),
            followee_id: openerp_types::Id::new(&req.user_id),
            display_name: None, description: None, metadata: None,
            created_at: openerp_types::DateTime::default(),
            updated_at: openerp_types::DateTime::default(),
        };
        if self.follows.create(&follow).await.is_ok() {
            if let Ok(mut me) = self.users.get(&uid).await {
                me.following_count += 1;
                let _ = self.users.update(&uid, &me).await;
            }
            if let Ok(mut them) = self.users.get(&req.user_id).await {
                them.follower_count += 1;
                let _ = self.users.update(&req.user_id, &them).await;
            }
            self.refresh_auth_profile(store, &uid).await;
        }
    }

    #[handle(UnfollowUserReq)]
    pub async fn handle_unfollow(&self, req: &UnfollowUserReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        let key = format!("{}:{}", uid, req.user_id);
        if self.follows.delete(&key).await.is_ok() {
            if let Ok(mut me) = self.users.get(&uid).await {
                me.following_count = me.following_count.saturating_sub(1);
                let _ = self.users.update(&uid, &me).await;
            }
            if let Ok(mut them) = self.users.get(&req.user_id).await {
                them.follower_count = them.follower_count.saturating_sub(1);
                let _ = self.users.update(&req.user_id, &them).await;
            }
            self.refresh_auth_profile(store, &uid).await;
        }
    }

    #[handle(LoadProfileReq)]
    pub async fn handle_load_profile(&self, req: &LoadProfileReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        if let Ok(user) = self.users.get(&req.user_id).await {
            let profile = helpers::user_to_profile(&user);
            let users = self.users.list().await.unwrap_or_default();
            let likes = self.likes.list().await.unwrap_or_default();
            let mut all_tweets = self.tweets.list().await.unwrap_or_default();
            let tweet_items: Vec<FeedItem> = all_tweets.iter()
                .filter(|t| t.author_id.as_str() == req.user_id)
                .map(|t| helpers::tweet_to_feed_item(t, &uid, &users, &likes))
                .collect();

            let follow_key = format!("{}:{}", uid, req.user_id);
            let followed_by_me = self.follows.get(&follow_key).await.is_ok();

            store.set(&ProfilePage::path(&req.user_id), ProfilePage {
                user: profile, tweets: tweet_items, followed_by_me, loading: false,
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

    // ── Search ──

    #[handle(SearchReq)]
    pub async fn handle_search(&self, req: &SearchReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        let query = req.query.to_lowercase();
        if query.is_empty() {
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

        let all_users = self.users.list().await.unwrap_or_default();
        let all_likes = self.likes.list().await.unwrap_or_default();
        let users: Vec<UserProfile> = all_users.iter()
            .filter(|u| u.username.to_lowercase().contains(&query)
                || u.display_name.as_deref().unwrap_or("").to_lowercase().contains(&query))
            .map(|u| helpers::user_to_profile(u))
            .collect();

        let all_tweets = self.tweets.list().await.unwrap_or_default();
        let tweets: Vec<FeedItem> = all_tweets.iter()
            .filter(|t| t.content.to_lowercase().contains(&query))
            .map(|t| helpers::tweet_to_feed_item(t, &uid, &all_users, &all_likes))
            .collect();

        store.set(SearchState::PATH, SearchState {
            query: req.query.clone(), users, tweets,
            loading: false, error: None,
        });
    }

    #[handle(SearchClearReq)]
    pub async fn handle_search_clear(&self, _req: &SearchClearReq, store: &StateStore) {
        store.remove(SearchState::PATH);
    }

    // ── Settings ──

    #[handle(SettingsLoadReq)]
    pub async fn handle_settings_load(&self, _req: &SettingsLoadReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        if let Ok(user) = self.users.get(&uid).await {
            store.set(SettingsState::PATH, SettingsState {
                display_name: user.display_name.clone().unwrap_or_default(),
                bio: user.bio.as_ref().map(|s| s.to_string()).unwrap_or_default(),
                busy: false, saved: false, error: None,
            });
        }
        store.set(AppRoute::PATH, AppRoute("/settings".into()));
    }

    #[handle(SettingsSaveReq)]
    pub async fn handle_settings_save(&self, req: &SettingsSaveReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        if req.display_name.trim().is_empty() {
            store.set(SettingsState::PATH, SettingsState {
                display_name: req.display_name.clone(), bio: req.bio.clone(),
                busy: false, saved: false,
                error: Some("Display name cannot be empty".into()),
            });
            return;
        }
        store.set(SettingsState::PATH, SettingsState {
            display_name: req.display_name.clone(), bio: req.bio.clone(),
            busy: true, saved: false, error: None,
        });
        if let Ok(mut user) = self.users.get(&uid).await {
            user.display_name = Some(req.display_name.clone());
            user.bio = Some(req.bio.clone());
            let _ = self.users.update(&uid, &user).await;
            self.refresh_auth_profile(store, &uid).await;
            store.set(SettingsState::PATH, SettingsState {
                display_name: req.display_name.clone(), bio: req.bio.clone(),
                busy: false, saved: true, error: None,
            });
        }
    }

    #[handle(ChangePasswordReq)]
    pub async fn handle_change_password(&self, req: &ChangePasswordReq, store: &StateStore) {
        if req.new_password.len() < 6 {
            store.set(PasswordState::PATH, PasswordState {
                busy: false, success: false,
                error: Some("Password must be at least 6 characters".into()),
            });
            return;
        }
        if req.old_password == req.new_password {
            store.set(PasswordState::PATH, PasswordState {
                busy: false, success: false,
                error: Some("New password must be different".into()),
            });
            return;
        }
        store.set(PasswordState::PATH, PasswordState {
            busy: false, success: true, error: None,
        });
    }

    async fn refresh_auth_profile(&self, store: &StateStore, uid: &str) {
        if let Ok(me) = self.users.get(uid).await {
            let profile = helpers::user_to_profile(&me);
            store.set(AuthState::PATH, AuthState {
                phase: AuthPhase::Authenticated, user: Some(profile),
                busy: false, error: None,
            });
        }
    }
}

// register() is GENERATED by #[flux_handlers] macro above.
