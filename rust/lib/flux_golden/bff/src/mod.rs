//! Twitter BFF — Flux state engine layer.
//!
//! `TwitterBff` holds backend dependencies. Handler methods use `&self`
//! to access them. `register()` wires each handler to the Flux router.
//!
//! In Phase 2, `#[flux_handlers]` macro generates `register()` from
//! the `#[handle(ReqType)]` annotations on each method.

pub mod global;

use flux_derive::flux_handlers;
use openerp_flux::StateStore;
use openerp_store::KvOps;

use crate::request::*;
use crate::state::*;
use crate::server::model;
use self::global::helpers;

/// BFF context — holds backend KvOps.
/// Handler methods live on this struct via `&self`.
pub struct TwitterBff {
    pub users: KvOps<model::User>,
    pub tweets: KvOps<model::Tweet>,
    pub likes: KvOps<model::Like>,
    pub follows: KvOps<model::Follow>,
}

// ── Handler methods ──
// Each method is annotated with the request type it handles.
// The #[flux_handlers] macro reads these annotations to generate register().

#[flux_handlers]
impl TwitterBff {
    fn current_user_id(&self, store: &StateStore) -> String {
        store.get(AuthState::PATH)
            .and_then(|v| v.downcast_ref::<AuthState>()
                .and_then(|a| a.user.as_ref().map(|u| u.id.clone())))
            .unwrap_or_default()
    }

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
        match self.users.get(&req.username) {
            Ok(Some(user)) => {
                let profile = helpers::user_to_profile(&user);
                let uid = profile.id.clone();
                store.set(AuthState::PATH, AuthState {
                    phase: AuthPhase::Authenticated, user: Some(profile),
                    busy: false, error: None,
                });
                store.set(AppRoute::PATH, AppRoute("/home".into()));
                store.set(TimelineFeed::PATH, helpers::build_timeline(
                    &uid, &self.tweets, &self.users, &self.likes,
                ));
            }
            _ => {
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
        let uid = self.current_user_id(store);
        store.set(TimelineFeed::PATH, TimelineFeed {
            items: vec![], loading: true, has_more: false, error: None,
        });
        store.set(TimelineFeed::PATH, helpers::build_timeline(
            &uid, &self.tweets, &self.users, &self.likes,
        ));
    }

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
        match self.tweets.save_new(tweet) {
            Ok(_) => {
                if let Ok(Some(mut user)) = self.users.get(&uid) {
                    user.tweet_count += 1;
                    let _ = self.users.save(user);
                }
                if let Some(ref parent_id) = req.reply_to_id {
                    if let Ok(Some(mut parent)) = self.tweets.get(parent_id) {
                        parent.reply_count += 1;
                        let _ = self.tweets.save(parent);
                    }
                }
                store.set(ComposeState::PATH, ComposeState::empty());
                store.set(TimelineFeed::PATH, helpers::build_timeline(
                    &uid, &self.tweets, &self.users, &self.likes,
                ));
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
        if self.likes.save_new(like).is_ok() {
            if let Ok(Some(mut tweet)) = self.tweets.get(&req.tweet_id) {
                tweet.like_count += 1;
                let _ = self.tweets.save(tweet);
            }
            store.set(TimelineFeed::PATH, helpers::build_timeline(
                &uid, &self.tweets, &self.users, &self.likes,
            ));
        }
    }

    #[handle(UnlikeTweetReq)]
    pub async fn handle_unlike(&self, req: &UnlikeTweetReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        let like_key = format!("{}:{}", uid, req.tweet_id);
        if self.likes.delete(&like_key).is_ok() {
            if let Ok(Some(mut tweet)) = self.tweets.get(&req.tweet_id) {
                tweet.like_count = tweet.like_count.saturating_sub(1);
                let _ = self.tweets.save(tweet);
            }
            store.set(TimelineFeed::PATH, helpers::build_timeline(
                &uid, &self.tweets, &self.users, &self.likes,
            ));
        }
    }

    #[handle(LoadTweetReq)]
    pub async fn handle_load_tweet(&self, req: &LoadTweetReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        if let Ok(Some(tweet)) = self.tweets.get(&req.tweet_id) {
            let item = helpers::tweet_to_feed_item(&tweet, &uid, &self.users, &self.likes);
            let mut replies: Vec<model::Tweet> = self.tweets.list().unwrap_or_default()
                .into_iter()
                .filter(|t| t.reply_to_id.as_ref().map(|s| s.as_str()) == Some(&req.tweet_id))
                .collect();
            replies.sort_by(|a, b| a.created_at.as_str().cmp(b.created_at.as_str()));
            let reply_items: Vec<FeedItem> = replies.iter()
                .map(|t| helpers::tweet_to_feed_item(t, &uid, &self.users, &self.likes))
                .collect();
            store.set(&TweetDetail::path(&req.tweet_id), TweetDetail {
                tweet: item, replies: reply_items, loading: false,
            });
            store.set(AppRoute::PATH, AppRoute(format!("/tweet/{}", req.tweet_id)));
        }
    }

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
        if self.follows.save_new(follow).is_ok() {
            if let Ok(Some(mut me)) = self.users.get(&uid) {
                me.following_count += 1;
                let _ = self.users.save(me);
            }
            if let Ok(Some(mut them)) = self.users.get(&req.user_id) {
                them.follower_count += 1;
                let _ = self.users.save(them);
            }
            self.refresh_auth_profile(store, &uid);
        }
    }

    #[handle(UnfollowUserReq)]
    pub async fn handle_unfollow(&self, req: &UnfollowUserReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        let key = format!("{}:{}", uid, req.user_id);
        if self.follows.delete(&key).is_ok() {
            if let Ok(Some(mut me)) = self.users.get(&uid) {
                me.following_count = me.following_count.saturating_sub(1);
                let _ = self.users.save(me);
            }
            if let Ok(Some(mut them)) = self.users.get(&req.user_id) {
                them.follower_count = them.follower_count.saturating_sub(1);
                let _ = self.users.save(them);
            }
            self.refresh_auth_profile(store, &uid);
        }
    }

    #[handle(LoadProfileReq)]
    pub async fn handle_load_profile(&self, req: &LoadProfileReq, store: &StateStore) {
        let uid = self.current_user_id(store);
        if let Ok(Some(user)) = self.users.get(&req.user_id) {
            let profile = helpers::user_to_profile(&user);
            let mut user_tweets: Vec<model::Tweet> = self.tweets.list().unwrap_or_default()
                .into_iter()
                .filter(|t| t.author_id.as_str() == req.user_id)
                .collect();
            user_tweets.sort_by(|a, b| b.created_at.as_str().cmp(a.created_at.as_str()));
            let tweet_items: Vec<FeedItem> = user_tweets.iter()
                .map(|t| helpers::tweet_to_feed_item(t, &uid, &self.users, &self.likes))
                .collect();
            let follow_key = format!("{}:{}", uid, req.user_id);
            let followed_by_me = self.follows.get(&follow_key).ok().flatten().is_some();
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

    fn refresh_auth_profile(&self, store: &StateStore, uid: &str) {
        if let Ok(Some(me)) = self.users.get(uid) {
            let profile = helpers::user_to_profile(&me);
            store.set(AuthState::PATH, AuthState {
                phase: AuthPhase::Authenticated, user: Some(profile),
                busy: false, error: None,
            });
        }
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

        // Search users by username or display_name.
        let users: Vec<UserProfile> = self.users.list().unwrap_or_default()
            .iter()
            .filter(|u| u.username.to_lowercase().contains(&query)
                || u.display_name.as_deref().unwrap_or("").to_lowercase().contains(&query))
            .map(|u| helpers::user_to_profile(u))
            .collect();

        // Search tweets by content.
        let tweets: Vec<FeedItem> = self.tweets.list().unwrap_or_default()
            .iter()
            .filter(|t| t.content.to_lowercase().contains(&query))
            .map(|t| helpers::tweet_to_feed_item(t, &uid, &self.users, &self.likes))
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
        if let Ok(Some(user)) = self.users.get(&uid) {
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
                display_name: req.display_name.clone(),
                bio: req.bio.clone(),
                busy: false, saved: false,
                error: Some("Display name cannot be empty".into()),
            });
            return;
        }

        store.set(SettingsState::PATH, SettingsState {
            display_name: req.display_name.clone(),
            bio: req.bio.clone(),
            busy: true, saved: false, error: None,
        });

        if let Ok(Some(mut user)) = self.users.get(&uid) {
            user.display_name = Some(req.display_name.clone());
            user.bio = Some(req.bio.clone());
            let _ = self.users.save(user);

            // Refresh auth profile.
            self.refresh_auth_profile(store, &uid);

            store.set(SettingsState::PATH, SettingsState {
                display_name: req.display_name.clone(),
                bio: req.bio.clone(),
                busy: false, saved: true, error: None,
            });
        }
    }

    #[handle(ChangePasswordReq)]
    pub async fn handle_change_password(&self, req: &ChangePasswordReq, store: &StateStore) {
        // Golden test: simple validation (no real password storage).
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
}

// register() is now GENERATED by #[flux_handlers] macro above.
// It produces code identical to what was previously hand-written here.
