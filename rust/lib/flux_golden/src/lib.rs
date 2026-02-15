//! Flux Golden Test — Twitter-like app.
//!
//! Full stack in one process, all in-memory:
//! 1. Backend: DSL-defined models + KvStore (redb) — REST-like data layer
//! 2. BFF: Flux state engine — handlers call backend, update UI state
//! 3. Tests: User stories validating the complete flow

#[cfg(test)]
mod twitter {
    use std::sync::Arc;

    use openerp_flux::{Flux, StateStore};
    use openerp_macro::model;
    use openerp_store::{KvOps, KvStore};
    use openerp_types::*;

    // =====================================================================
    // 1. Backend Models — DSL-defined, stored in embedded KV
    // =====================================================================

    #[model(module = "twitter")]
    pub struct TwitterUser {
        pub id: Id,
        pub username: String,
        pub bio: Option<String>,
        pub avatar: Option<Avatar>,
        pub follower_count: u32,
        pub following_count: u32,
        pub tweet_count: u32,
    }

    #[model(module = "twitter")]
    pub struct Tweet {
        pub id: Id,
        pub author_id: Id,
        pub content: String,
        pub like_count: u32,
        pub reply_count: u32,
        pub reply_to_id: Option<Id>,
    }

    #[model(module = "twitter")]
    pub struct Like {
        pub id: Id,
        pub user_id: Id,
        pub tweet_id: Id,
    }

    #[model(module = "twitter")]
    pub struct Follow {
        pub id: Id,
        pub follower_id: Id,
        pub followee_id: Id,
    }

    // =====================================================================
    // 2. KvStore Implementations
    // =====================================================================

    impl KvStore for TwitterUser {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "twitter:user:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&self.username);
            }
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
        fn before_update(&mut self) {
            self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
        }
    }

    impl KvStore for Tweet {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "twitter:tweet:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new(&uuid::Uuid::new_v4().to_string().replace('-', ""));
            }
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
        fn before_update(&mut self) {
            self.updated_at = DateTime::new(&chrono::Utc::now().to_rfc3339());
        }
    }

    impl KvStore for Like {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "twitter:like:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            // Composite key: user_id:tweet_id — ensures uniqueness.
            if self.id.is_empty() {
                self.id = Id::new(&format!("{}:{}", self.user_id, self.tweet_id));
            }
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
    }

    impl KvStore for Follow {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "twitter:follow:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            // Composite key: follower_id:followee_id
            if self.id.is_empty() {
                self.id = Id::new(&format!("{}:{}", self.follower_id, self.followee_id));
            }
            let now = chrono::Utc::now().to_rfc3339();
            if self.created_at.is_empty() { self.created_at = DateTime::new(&now); }
            self.updated_at = DateTime::new(&now);
        }
    }

    // =====================================================================
    // 3. BFF State Types — what the UI renders
    // =====================================================================

    #[derive(Debug, Clone, PartialEq)]
    struct AuthState {
        phase: &'static str, // "unauthenticated", "authenticated"
        user: Option<UserProfile>,
        busy: bool,
        error: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct UserProfile {
        id: String,
        username: String,
        display_name: String,
        bio: Option<String>,
        avatar: Option<String>,
        follower_count: u32,
        following_count: u32,
        tweet_count: u32,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TimelineFeed {
        items: Vec<FeedItem>,
        loading: bool,
        has_more: bool,
        error: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct FeedItem {
        tweet_id: String,
        author: UserProfile,
        content: String,
        like_count: u32,
        liked_by_me: bool,
        reply_count: u32,
        reply_to_id: Option<String>,
        created_at: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct ComposeState {
        content: String,
        reply_to_id: Option<String>,
        busy: bool,
        error: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct ProfilePage {
        user: UserProfile,
        tweets: Vec<FeedItem>,
        followed_by_me: bool,
        loading: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TweetDetail {
        tweet: FeedItem,
        replies: Vec<FeedItem>,
        loading: bool,
    }

    // =====================================================================
    // 4. BFF Request Types — what the UI emits
    // =====================================================================

    #[derive(Debug)]
    struct LoginReq { username: String }
    #[derive(Debug)]
    struct CreateTweetReq { content: String, reply_to_id: Option<String> }
    #[derive(Debug)]
    struct LikeTweetReq { tweet_id: String }
    #[derive(Debug)]
    struct UnlikeTweetReq { tweet_id: String }
    #[derive(Debug)]
    struct FollowUserReq { user_id: String }
    #[derive(Debug)]
    struct UnfollowUserReq { user_id: String }
    #[derive(Debug)]
    struct LoadProfileReq { user_id: String }
    #[derive(Debug)]
    struct LoadTweetReq { tweet_id: String }
    #[derive(Debug)]
    struct UpdateFieldReq { field: String, value: String }

    // =====================================================================
    // 5. Backend — all-in-memory data layer
    // =====================================================================

    struct Backend {
        users: KvOps<TwitterUser>,
        tweets: KvOps<Tweet>,
        likes: KvOps<Like>,
        follows: KvOps<Follow>,
    }

    impl Backend {
        fn user_to_profile(&self, u: &TwitterUser) -> UserProfile {
            UserProfile {
                id: u.id.to_string(),
                username: u.username.clone(),
                display_name: u.display_name.clone().unwrap_or_else(|| u.username.clone()),
                bio: u.bio.as_ref().map(|s| s.to_string()),
                avatar: u.avatar.as_ref().map(|s| s.to_string()),
                follower_count: u.follower_count,
                following_count: u.following_count,
                tweet_count: u.tweet_count,
            }
        }

        fn tweet_to_feed_item(&self, t: &Tweet, current_user_id: &str) -> FeedItem {
            let author = self.users.get(&t.author_id).ok().flatten()
                .map(|u| self.user_to_profile(&u))
                .unwrap_or_else(|| UserProfile {
                    id: t.author_id.to_string(),
                    username: "unknown".into(),
                    display_name: "Unknown".into(),
                    bio: None, avatar: None,
                    follower_count: 0, following_count: 0, tweet_count: 0,
                });

            let like_key = format!("{}:{}", current_user_id, t.id);
            let liked_by_me = self.likes.get(&like_key).ok().flatten().is_some();

            FeedItem {
                tweet_id: t.id.to_string(),
                author,
                content: t.content.clone(),
                like_count: t.like_count,
                liked_by_me,
                reply_count: t.reply_count,
                reply_to_id: t.reply_to_id.as_ref().map(|s| s.to_string()),
                created_at: t.created_at.to_string(),
            }
        }

        fn load_timeline(&self, current_user_id: &str) -> TimelineFeed {
            let mut tweets = self.tweets.list().unwrap_or_default();
            // Sort by created_at descending (newest first).
            tweets.sort_by(|a, b| b.created_at.as_str().cmp(a.created_at.as_str()));

            // Only show top-level tweets (not replies) in timeline.
            let items: Vec<FeedItem> = tweets.iter()
                .filter(|t| t.reply_to_id.is_none())
                .map(|t| self.tweet_to_feed_item(t, current_user_id))
                .collect();

            TimelineFeed { items, loading: false, has_more: false, error: None }
        }

        fn load_user_tweets(&self, user_id: &str, current_user_id: &str) -> Vec<FeedItem> {
            let mut tweets: Vec<Tweet> = self.tweets.list().unwrap_or_default()
                .into_iter()
                .filter(|t| t.author_id.as_str() == user_id)
                .collect();
            tweets.sort_by(|a, b| b.created_at.as_str().cmp(a.created_at.as_str()));
            tweets.iter().map(|t| self.tweet_to_feed_item(t, current_user_id)).collect()
        }

        fn load_replies(&self, tweet_id: &str, current_user_id: &str) -> Vec<FeedItem> {
            let mut replies: Vec<Tweet> = self.tweets.list().unwrap_or_default()
                .into_iter()
                .filter(|t| t.reply_to_id.as_ref().map(|s| s.as_str()) == Some(tweet_id))
                .collect();
            replies.sort_by(|a, b| a.created_at.as_str().cmp(b.created_at.as_str()));
            replies.iter().map(|t| self.tweet_to_feed_item(t, current_user_id)).collect()
        }

        fn is_following(&self, follower_id: &str, followee_id: &str) -> bool {
            let key = format!("{}:{}", follower_id, followee_id);
            self.follows.get(&key).ok().flatten().is_some()
        }
    }

    // =====================================================================
    // 6. BFF Setup — Flux + handlers wired to backend
    // =====================================================================

    struct TwitterApp {
        flux: Flux,
        backend: Arc<Backend>,
        _dir: tempfile::TempDir,
    }

    fn current_user_id(store: &StateStore) -> Option<String> {
        store.get("auth/state")
            .and_then(|v| v.downcast_ref::<AuthState>().map(|a| a.user.as_ref().map(|u| u.id.clone())))
            .flatten()
    }

    fn setup_twitter() -> TwitterApp {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("twitter.redb")).unwrap(),
        );

        let backend = Arc::new(Backend {
            users: KvOps::new(kv.clone()),
            tweets: KvOps::new(kv.clone()),
            likes: KvOps::new(kv.clone()),
            follows: KvOps::new(kv.clone()),
        });

        let flux = Flux::new();

        // --- app/initialize ---
        flux.on("app/initialize", |_, _, store: Arc<StateStore>| async move {
            store.set("auth/state", AuthState {
                phase: "unauthenticated", user: None, busy: false, error: None,
            });
            store.set("app/route", "/login".to_string());
        });

        // --- auth/login ---
        {
            let b = backend.clone();
            flux.on("auth/login", move |_, payload, store: Arc<StateStore>| {
                let b = b.clone();
                async move {
                    let req = payload.downcast_ref::<LoginReq>().unwrap();
                    store.set("auth/state", AuthState {
                        phase: "unauthenticated", user: None, busy: true, error: None,
                    });

                    match b.users.get(&req.username) {
                        Ok(Some(user)) => {
                            let profile = b.user_to_profile(&user);
                            let uid = profile.id.clone();
                            store.set("auth/state", AuthState {
                                phase: "authenticated", user: Some(profile), busy: false, error: None,
                            });
                            store.set("app/route", "/home".to_string());
                            // Auto-load timeline.
                            let feed = b.load_timeline(&uid);
                            store.set("timeline/feed", feed);
                        }
                        _ => {
                            store.set("auth/state", AuthState {
                                phase: "unauthenticated", user: None, busy: false,
                                error: Some(format!("User '{}' not found", req.username)),
                            });
                        }
                    }
                }
            });
        }

        // --- auth/logout ---
        flux.on("auth/logout", |_, _, store: Arc<StateStore>| async move {
            store.set("auth/state", AuthState {
                phase: "unauthenticated", user: None, busy: false, error: None,
            });
            store.set("app/route", "/login".to_string());
            store.remove("timeline/feed");
            store.remove("compose/state");
        });

        // --- timeline/load ---
        {
            let b = backend.clone();
            flux.on("timeline/load", move |_, _, store: Arc<StateStore>| {
                let b = b.clone();
                async move {
                    let uid = current_user_id(&store).unwrap_or_default();
                    store.set("timeline/feed", TimelineFeed {
                        items: vec![], loading: true, has_more: false, error: None,
                    });
                    let feed = b.load_timeline(&uid);
                    store.set("timeline/feed", feed);
                }
            });
        }

        // --- tweet/create ---
        {
            let b = backend.clone();
            flux.on("tweet/create", move |_, payload, store: Arc<StateStore>| {
                let b = b.clone();
                async move {
                    let req = payload.downcast_ref::<CreateTweetReq>().unwrap();
                    let uid = current_user_id(&store).unwrap_or_default();

                    if req.content.trim().is_empty() {
                        store.set("compose/state", ComposeState {
                            content: req.content.clone(),
                            reply_to_id: req.reply_to_id.clone(),
                            busy: false,
                            error: Some("Tweet cannot be empty".into()),
                        });
                        return;
                    }
                    if req.content.len() > 280 {
                        store.set("compose/state", ComposeState {
                            content: req.content.clone(),
                            reply_to_id: req.reply_to_id.clone(),
                            busy: false,
                            error: Some("Tweet exceeds 280 characters".into()),
                        });
                        return;
                    }

                    store.set("compose/state", ComposeState {
                        content: req.content.clone(),
                        reply_to_id: req.reply_to_id.clone(),
                        busy: true, error: None,
                    });

                    let tweet = Tweet {
                        id: Id::default(),
                        author_id: Id::new(&uid),
                        content: req.content.clone(),
                        like_count: 0,
                        reply_count: 0,
                        reply_to_id: req.reply_to_id.as_ref().map(|s| Id::new(s)),
                        display_name: None, description: None, metadata: None,
                        created_at: DateTime::default(), updated_at: DateTime::default(),
                    };

                    match b.tweets.save_new(tweet) {
                        Ok(_) => {
                            // Increment author tweet count.
                            if let Ok(Some(mut user)) = b.users.get(&uid) {
                                user.tweet_count += 1;
                                let _ = b.users.save(user);
                            }
                            // If this is a reply, increment parent reply_count.
                            if let Some(ref parent_id) = req.reply_to_id {
                                if let Ok(Some(mut parent)) = b.tweets.get(parent_id) {
                                    parent.reply_count += 1;
                                    let _ = b.tweets.save(parent);
                                }
                            }
                            // Clear compose and refresh timeline.
                            store.set("compose/state", ComposeState {
                                content: String::new(), reply_to_id: None,
                                busy: false, error: None,
                            });
                            let feed = b.load_timeline(&uid);
                            store.set("timeline/feed", feed);
                        }
                        Err(e) => {
                            store.set("compose/state", ComposeState {
                                content: req.content.clone(),
                                reply_to_id: req.reply_to_id.clone(),
                                busy: false,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
            });
        }

        // --- tweet/like ---
        {
            let b = backend.clone();
            flux.on("tweet/like", move |_, payload, store: Arc<StateStore>| {
                let b = b.clone();
                async move {
                    let req = payload.downcast_ref::<LikeTweetReq>().unwrap();
                    let uid = current_user_id(&store).unwrap_or_default();

                    let like = Like {
                        id: Id::default(),
                        user_id: Id::new(&uid),
                        tweet_id: Id::new(&req.tweet_id),
                        display_name: None, description: None, metadata: None,
                        created_at: DateTime::default(), updated_at: DateTime::default(),
                    };

                    if b.likes.save_new(like).is_ok() {
                        // Increment like_count on the tweet.
                        if let Ok(Some(mut tweet)) = b.tweets.get(&req.tweet_id) {
                            tweet.like_count += 1;
                            let _ = b.tweets.save(tweet);
                        }
                        // Refresh timeline.
                        let feed = b.load_timeline(&uid);
                        store.set("timeline/feed", feed);
                    }
                }
            });
        }

        // --- tweet/unlike ---
        {
            let b = backend.clone();
            flux.on("tweet/unlike", move |_, payload, store: Arc<StateStore>| {
                let b = b.clone();
                async move {
                    let req = payload.downcast_ref::<UnlikeTweetReq>().unwrap();
                    let uid = current_user_id(&store).unwrap_or_default();
                    let like_key = format!("{}:{}", uid, req.tweet_id);

                    if b.likes.delete(&like_key).is_ok() {
                        if let Ok(Some(mut tweet)) = b.tweets.get(&req.tweet_id) {
                            tweet.like_count = tweet.like_count.saturating_sub(1);
                            let _ = b.tweets.save(tweet);
                        }
                        let feed = b.load_timeline(&uid);
                        store.set("timeline/feed", feed);
                    }
                }
            });
        }

        // --- user/follow ---
        {
            let b = backend.clone();
            flux.on("user/follow", move |_, payload, store: Arc<StateStore>| {
                let b = b.clone();
                async move {
                    let req = payload.downcast_ref::<FollowUserReq>().unwrap();
                    let uid = current_user_id(&store).unwrap_or_default();

                    let follow = Follow {
                        id: Id::default(),
                        follower_id: Id::new(&uid),
                        followee_id: Id::new(&req.user_id),
                        display_name: None, description: None, metadata: None,
                        created_at: DateTime::default(), updated_at: DateTime::default(),
                    };

                    if b.follows.save_new(follow).is_ok() {
                        // Update follower/following counts.
                        if let Ok(Some(mut me)) = b.users.get(&uid) {
                            me.following_count += 1;
                            let _ = b.users.save(me);
                        }
                        if let Ok(Some(mut them)) = b.users.get(&req.user_id) {
                            them.follower_count += 1;
                            let _ = b.users.save(them);
                        }
                        // Update auth state with new following count.
                        if let Ok(Some(me)) = b.users.get(&uid) {
                            let profile = b.user_to_profile(&me);
                            store.set("auth/state", AuthState {
                                phase: "authenticated", user: Some(profile),
                                busy: false, error: None,
                            });
                        }
                    }
                }
            });
        }

        // --- user/unfollow ---
        {
            let b = backend.clone();
            flux.on("user/unfollow", move |_, payload, store: Arc<StateStore>| {
                let b = b.clone();
                async move {
                    let req = payload.downcast_ref::<UnfollowUserReq>().unwrap();
                    let uid = current_user_id(&store).unwrap_or_default();
                    let key = format!("{}:{}", uid, req.user_id);

                    if b.follows.delete(&key).is_ok() {
                        if let Ok(Some(mut me)) = b.users.get(&uid) {
                            me.following_count = me.following_count.saturating_sub(1);
                            let _ = b.users.save(me);
                        }
                        if let Ok(Some(mut them)) = b.users.get(&req.user_id) {
                            them.follower_count = them.follower_count.saturating_sub(1);
                            let _ = b.users.save(them);
                        }
                        if let Ok(Some(me)) = b.users.get(&uid) {
                            let profile = b.user_to_profile(&me);
                            store.set("auth/state", AuthState {
                                phase: "authenticated", user: Some(profile),
                                busy: false, error: None,
                            });
                        }
                    }
                }
            });
        }

        // --- profile/load ---
        {
            let b = backend.clone();
            flux.on("profile/load", move |_, payload, store: Arc<StateStore>| {
                let b = b.clone();
                async move {
                    let req = payload.downcast_ref::<LoadProfileReq>().unwrap();
                    let uid = current_user_id(&store).unwrap_or_default();
                    let path = format!("profile/{}", req.user_id);

                    match b.users.get(&req.user_id) {
                        Ok(Some(user)) => {
                            let profile = b.user_to_profile(&user);
                            let tweets = b.load_user_tweets(&req.user_id, &uid);
                            let followed_by_me = b.is_following(&uid, &req.user_id);

                            store.set(&path, ProfilePage {
                                user: profile, tweets, followed_by_me, loading: false,
                            });
                            store.set("app/route", format!("/profile/{}", req.user_id));
                        }
                        _ => {}
                    }
                }
            });
        }

        // --- tweet/load ---
        {
            let b = backend.clone();
            flux.on("tweet/load", move |_, payload, store: Arc<StateStore>| {
                let b = b.clone();
                async move {
                    let req = payload.downcast_ref::<LoadTweetReq>().unwrap();
                    let uid = current_user_id(&store).unwrap_or_default();
                    let path = format!("tweet/{}", req.tweet_id);

                    match b.tweets.get(&req.tweet_id) {
                        Ok(Some(tweet)) => {
                            let item = b.tweet_to_feed_item(&tweet, &uid);
                            let replies = b.load_replies(&req.tweet_id, &uid);
                            store.set(&path, TweetDetail {
                                tweet: item, replies, loading: false,
                            });
                            store.set("app/route", format!("/tweet/{}", req.tweet_id));
                        }
                        _ => {}
                    }
                }
            });
        }

        // --- compose/update-field ---
        flux.on("compose/update-field", |_, payload, store: Arc<StateStore>| async move {
            let req = payload.downcast_ref::<UpdateFieldReq>().unwrap();
            let mut state: ComposeState = store.get("compose/state")
                .and_then(|v| v.downcast_ref::<ComposeState>().cloned())
                .unwrap_or(ComposeState {
                    content: String::new(), reply_to_id: None,
                    busy: false, error: None,
                });

            match req.field.as_str() {
                "content" => state.content = req.value.clone(),
                _ => {}
            }
            state.error = None; // clear error on edit
            store.set("compose/state", state);
        });

        TwitterApp { flux, backend, _dir: dir }
    }

    /// Seed test users into the backend.
    fn seed_users(app: &TwitterApp) {
        let users = vec![
            ("alice", "Alice Wang", Some("Rust developer & open source enthusiast")),
            ("bob", "Bob Li", Some("Product designer at Haivivi")),
            ("carol", "Carol Zhang", Some("Full-stack engineer")),
            ("dave", "Dave Chen", None),
        ];
        for (username, display, bio) in users {
            app.backend.users.save_new(TwitterUser {
                id: Id::default(),
                username: username.into(),
                bio: bio.map(|s| s.to_string()),
                avatar: Some(Avatar::new(&format!("https://img.test/{}.png", username))),
                follower_count: 0, following_count: 0, tweet_count: 0,
                display_name: Some(display.into()),
                description: None, metadata: None,
                created_at: DateTime::default(), updated_at: DateTime::default(),
            }).unwrap();
        }
    }

    /// Seed tweets into the backend.
    fn seed_tweets(app: &TwitterApp) {
        let tweets = vec![
            ("alice", "Just shipped a new feature in Rust! The borrow checker is my best friend."),
            ("bob", "New design system is looking great. Dark mode coming soon."),
            ("carol", "TIL: Arc<dyn Any> is basically free for zero-copy state sharing."),
            ("alice", "Anyone else excited about Cap'n Proto for FFI? Zero-copy across languages!"),
            ("dave", "Hello Twitter! First tweet."),
        ];
        for (author, content) in tweets {
            app.backend.tweets.save_new(Tweet {
                id: Id::default(),
                author_id: Id::new(author),
                content: content.into(),
                like_count: 0, reply_count: 0, reply_to_id: None,
                display_name: None, description: None, metadata: None,
                created_at: DateTime::default(), updated_at: DateTime::default(),
            }).unwrap();
            // Update tweet count.
            if let Ok(Some(mut user)) = app.backend.users.get(author) {
                user.tweet_count += 1;
                let _ = app.backend.users.save(user);
            }
            // Small delay for ordering.
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    // Helper: get typed state.
    fn get_auth(app: &TwitterApp) -> AuthState {
        app.flux.get("auth/state").unwrap().downcast_ref::<AuthState>().unwrap().clone()
    }
    fn get_route(app: &TwitterApp) -> String {
        app.flux.get("app/route").unwrap().downcast_ref::<String>().unwrap().clone()
    }
    fn get_feed(app: &TwitterApp) -> TimelineFeed {
        app.flux.get("timeline/feed").unwrap().downcast_ref::<TimelineFeed>().unwrap().clone()
    }
    fn get_compose(app: &TwitterApp) -> ComposeState {
        app.flux.get("compose/state").unwrap().downcast_ref::<ComposeState>().unwrap().clone()
    }

    // =====================================================================
    // 7. Golden Tests — User Stories
    // =====================================================================

    // --- Story 1: First Launch ---

    #[tokio::test]
    async fn story_first_launch() {
        let app = setup_twitter();
        app.flux.emit("app/initialize", ()).await;

        let auth = get_auth(&app);
        assert_eq!(auth.phase, "unauthenticated");
        assert!(auth.user.is_none());
        assert!(!auth.busy);
        assert_eq!(get_route(&app), "/login");
    }

    // --- Story 2: Login Success ---

    #[tokio::test]
    async fn story_login_success() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let auth = get_auth(&app);
        assert_eq!(auth.phase, "authenticated");
        assert!(!auth.busy);
        let user = auth.user.unwrap();
        assert_eq!(user.username, "alice");
        assert_eq!(user.display_name, "Alice Wang");
        assert_eq!(get_route(&app), "/home");

        // Timeline should be auto-loaded.
        let feed = get_feed(&app);
        assert!(!feed.loading);
        assert!(!feed.items.is_empty());
    }

    // --- Story 3: Login Failure ---

    #[tokio::test]
    async fn story_login_failure() {
        let app = setup_twitter();
        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "nonexistent".into() }).await;

        let auth = get_auth(&app);
        assert_eq!(auth.phase, "unauthenticated");
        assert!(auth.error.is_some());
        assert!(auth.error.unwrap().contains("not found"));
        assert_eq!(get_route(&app), "/login"); // stays on login
    }

    // --- Story 4: View Timeline ---

    #[tokio::test]
    async fn story_view_timeline() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let feed = get_feed(&app);
        assert_eq!(feed.items.len(), 5); // all 5 tweets (no replies yet)
        // Newest first.
        assert!(feed.items[0].content.contains("First tweet")); // dave's tweet (last seeded)
        // Each item has author info.
        for item in &feed.items {
            assert!(!item.author.username.is_empty());
            assert!(!item.tweet_id.is_empty());
        }
    }

    // --- Story 5: Post a Tweet ---

    #[tokio::test]
    async fn story_post_tweet() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        app.flux.emit("tweet/create", CreateTweetReq {
            content: "My first tweet via Flux!".into(),
            reply_to_id: None,
        }).await;

        // Compose cleared.
        let compose = get_compose(&app);
        assert!(compose.content.is_empty());
        assert!(!compose.busy);
        assert!(compose.error.is_none());

        // Timeline updated.
        let feed = get_feed(&app);
        assert_eq!(feed.items.len(), 1);
        assert_eq!(feed.items[0].content, "My first tweet via Flux!");
        assert_eq!(feed.items[0].author.username, "alice");

        // User tweet count incremented.
        let user = app.backend.users.get("alice").unwrap().unwrap();
        assert_eq!(user.tweet_count, 1);
    }

    // --- Story 6: Post Empty Tweet Rejected ---

    #[tokio::test]
    async fn story_post_empty_tweet_rejected() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        app.flux.emit("tweet/create", CreateTweetReq {
            content: "   ".into(), // whitespace only
            reply_to_id: None,
        }).await;

        let compose = get_compose(&app);
        assert!(compose.error.is_some());
        assert!(compose.error.unwrap().contains("empty"));
    }

    // --- Story 7: Post Too-Long Tweet Rejected ---

    #[tokio::test]
    async fn story_post_long_tweet_rejected() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let long_content = "x".repeat(281);
        app.flux.emit("tweet/create", CreateTweetReq {
            content: long_content, reply_to_id: None,
        }).await;

        let compose = get_compose(&app);
        assert!(compose.error.is_some());
        assert!(compose.error.unwrap().contains("280"));
    }

    // --- Story 8: Like and Unlike ---

    #[tokio::test]
    async fn story_like_and_unlike() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let feed = get_feed(&app);
        let tweet_id = feed.items[0].tweet_id.clone();
        assert_eq!(feed.items[0].like_count, 0);
        assert!(!feed.items[0].liked_by_me);

        // Like.
        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: tweet_id.clone() }).await;

        let feed = get_feed(&app);
        let item = feed.items.iter().find(|i| i.tweet_id == tweet_id).unwrap();
        assert_eq!(item.like_count, 1);
        assert!(item.liked_by_me);

        // Unlike.
        app.flux.emit("tweet/unlike", UnlikeTweetReq { tweet_id: tweet_id.clone() }).await;

        let feed = get_feed(&app);
        let item = feed.items.iter().find(|i| i.tweet_id == tweet_id).unwrap();
        assert_eq!(item.like_count, 0);
        assert!(!item.liked_by_me);
    }

    // --- Story 9: Follow and Unfollow ---

    #[tokio::test]
    async fn story_follow_and_unfollow() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Follow bob.
        app.flux.emit("user/follow", FollowUserReq { user_id: "bob".into() }).await;

        // Alice's following count increased.
        let auth = get_auth(&app);
        assert_eq!(auth.user.unwrap().following_count, 1);
        // Bob's follower count increased.
        let bob = app.backend.users.get("bob").unwrap().unwrap();
        assert_eq!(bob.follower_count, 1);

        // Unfollow bob.
        app.flux.emit("user/unfollow", UnfollowUserReq { user_id: "bob".into() }).await;

        let auth = get_auth(&app);
        assert_eq!(auth.user.unwrap().following_count, 0);
        let bob = app.backend.users.get("bob").unwrap().unwrap();
        assert_eq!(bob.follower_count, 0);
    }

    // --- Story 10: View Profile ---

    #[tokio::test]
    async fn story_view_profile() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // View bob's profile.
        app.flux.emit("profile/load", LoadProfileReq { user_id: "bob".into() }).await;

        let profile = app.flux.get("profile/bob").unwrap();
        let page = profile.downcast_ref::<ProfilePage>().unwrap();
        assert_eq!(page.user.username, "bob");
        assert_eq!(page.user.display_name, "Bob Li");
        assert_eq!(page.tweets.len(), 1); // bob has 1 tweet
        assert!(!page.followed_by_me);
        assert_eq!(get_route(&app), "/profile/bob");
    }

    // --- Story 11: Reply to Tweet ---

    #[tokio::test]
    async fn story_reply_to_tweet() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let feed = get_feed(&app);
        let target = feed.items[0].tweet_id.clone();
        assert_eq!(feed.items[0].reply_count, 0);

        // Reply.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Great point!".into(),
            reply_to_id: Some(target.clone()),
        }).await;

        // Parent reply count incremented.
        let parent = app.backend.tweets.get(&target).unwrap().unwrap();
        assert_eq!(parent.reply_count, 1);

        // Reply does NOT appear in timeline (filtered out).
        let feed = get_feed(&app);
        assert!(feed.items.iter().all(|i| i.content != "Great point!"));

        // Load tweet detail to see replies.
        app.flux.emit("tweet/load", LoadTweetReq { tweet_id: target.clone() }).await;

        let detail = app.flux.get(&format!("tweet/{}", target)).unwrap();
        let detail = detail.downcast_ref::<TweetDetail>().unwrap();
        assert_eq!(detail.replies.len(), 1);
        assert_eq!(detail.replies[0].content, "Great point!");
    }

    // --- Story 12: Logout ---

    #[tokio::test]
    async fn story_logout() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;
        assert_eq!(get_auth(&app).phase, "authenticated");

        app.flux.emit("auth/logout", ()).await;

        assert_eq!(get_auth(&app).phase, "unauthenticated");
        assert_eq!(get_route(&app), "/login");
        assert!(app.flux.get("timeline/feed").is_none());
    }

    // --- Story 13: Compose Field Updates ---

    #[tokio::test]
    async fn story_compose_field_updates() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Type content.
        app.flux.emit("compose/update-field", UpdateFieldReq {
            field: "content".into(), value: "Hello ".into(),
        }).await;
        assert_eq!(get_compose(&app).content, "Hello ");

        app.flux.emit("compose/update-field", UpdateFieldReq {
            field: "content".into(), value: "Hello world!".into(),
        }).await;
        assert_eq!(get_compose(&app).content, "Hello world!");
    }

    // --- Story 14: Full Flow — login, tweet, like, follow, profile, logout ---

    #[tokio::test]
    async fn story_full_flow() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;

        // Login as Alice.
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;
        assert_eq!(get_auth(&app).phase, "authenticated");
        assert_eq!(get_route(&app), "/home");

        // Post a tweet.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Building a Twitter clone with Flux!".into(),
            reply_to_id: None,
        }).await;

        let feed = get_feed(&app);
        assert_eq!(feed.items.len(), 1);
        let tweet_id = feed.items[0].tweet_id.clone();

        // Like own tweet.
        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: tweet_id.clone() }).await;
        let feed = get_feed(&app);
        assert_eq!(feed.items[0].like_count, 1);
        assert!(feed.items[0].liked_by_me);

        // Follow Bob.
        app.flux.emit("user/follow", FollowUserReq { user_id: "bob".into() }).await;
        assert_eq!(get_auth(&app).user.unwrap().following_count, 1);

        // View Bob's profile.
        app.flux.emit("profile/load", LoadProfileReq { user_id: "bob".into() }).await;
        let profile = app.flux.get("profile/bob").unwrap();
        let page = profile.downcast_ref::<ProfilePage>().unwrap();
        assert!(page.followed_by_me);
        assert_eq!(page.user.follower_count, 1);

        // Unfollow Bob.
        app.flux.emit("user/unfollow", UnfollowUserReq { user_id: "bob".into() }).await;
        assert_eq!(get_auth(&app).user.unwrap().following_count, 0);

        // Logout.
        app.flux.emit("auth/logout", ()).await;
        assert_eq!(get_auth(&app).phase, "unauthenticated");
        assert_eq!(get_route(&app), "/login");
    }

    // --- Story 15: Subscription captures all state changes ---

    #[tokio::test]
    async fn story_subscription_timeline() {
        let app = setup_twitter();
        seed_users(&app);

        let changes = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let c = changes.clone();
        app.flux.subscribe("#", move |path, _| {
            c.lock().unwrap().push(path.to_string());
        });

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let paths = changes.lock().unwrap();
        // Should have captured: auth/state (multiple), app/route, timeline/feed
        assert!(paths.contains(&"auth/state".to_string()));
        assert!(paths.contains(&"app/route".to_string()));
        assert!(paths.contains(&"timeline/feed".to_string()));
    }

    // --- Story 16: Multiple users posting ---

    #[tokio::test]
    async fn story_multi_user() {
        let app = setup_twitter();
        seed_users(&app);

        // Login as Alice, post tweet.
        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Alice's tweet".into(), reply_to_id: None,
        }).await;

        // Logout, login as Bob, post tweet.
        app.flux.emit("auth/logout", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "bob".into() }).await;
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Bob's tweet".into(), reply_to_id: None,
        }).await;

        // Bob sees both tweets in timeline.
        let feed = get_feed(&app);
        assert_eq!(feed.items.len(), 2);

        let authors: Vec<&str> = feed.items.iter()
            .map(|i| i.author.username.as_str()).collect();
        assert!(authors.contains(&"alice"));
        assert!(authors.contains(&"bob"));
    }

    // --- Story 17: Double like is idempotent ---

    #[tokio::test]
    async fn story_double_like_idempotent() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let feed = get_feed(&app);
        let tweet_id = feed.items[0].tweet_id.clone();

        // Like once.
        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: tweet_id.clone() }).await;
        // Like again — should be idempotent (save_new fails silently on duplicate).
        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: tweet_id.clone() }).await;

        let feed = get_feed(&app);
        let item = feed.items.iter().find(|i| i.tweet_id == tweet_id).unwrap();
        assert_eq!(item.like_count, 1); // only 1, not 2
    }

    // =====================================================================
    // ERROR SCENARIOS
    // =====================================================================

    // --- Error 1: Login with empty username ---

    #[tokio::test]
    async fn error_login_empty_username() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "".into() }).await;

        let auth = get_auth(&app);
        assert_eq!(auth.phase, "unauthenticated");
        assert!(auth.error.is_some());
        assert_eq!(get_route(&app), "/login");
    }

    // --- Error 2: Tweet when not logged in ---

    #[tokio::test]
    async fn error_tweet_not_logged_in() {
        let app = setup_twitter();
        app.flux.emit("app/initialize", ()).await;

        // No login — current_user_id returns empty string.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Ghost tweet".into(), reply_to_id: None,
        }).await;

        // Tweet is created with empty author_id — not ideal but shouldn't crash.
        // Compose should still be cleared since save_new succeeds.
        let compose = get_compose(&app);
        assert!(!compose.busy);
    }

    // --- Error 3: Like non-existent tweet ---

    #[tokio::test]
    async fn error_like_nonexistent_tweet() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Like a tweet that doesn't exist — like record is created, but
        // tweet like_count update silently skips.
        app.flux.emit("tweet/like", LikeTweetReq {
            tweet_id: "nonexistent-id".into(),
        }).await;

        // Should not crash. Timeline should still be valid.
        let feed = get_feed(&app);
        assert!(!feed.loading);
    }

    // --- Error 4: Unlike a tweet not liked ---

    #[tokio::test]
    async fn error_unlike_not_liked() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let feed = get_feed(&app);
        let tweet_id = feed.items[0].tweet_id.clone();

        // Unlike without liking first — delete fails silently.
        app.flux.emit("tweet/unlike", UnlikeTweetReq {
            tweet_id: tweet_id.clone(),
        }).await;

        // Like count should remain 0.
        let feed = get_feed(&app);
        let item = feed.items.iter().find(|i| i.tweet_id == tweet_id).unwrap();
        assert_eq!(item.like_count, 0);
        assert!(!item.liked_by_me);
    }

    // --- Error 5: Follow yourself ---

    #[tokio::test]
    async fn error_follow_self() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Follow yourself.
        app.flux.emit("user/follow", FollowUserReq { user_id: "alice".into() }).await;

        // Technically succeeds (no guard), but count updates on same user.
        let auth = get_auth(&app);
        let user = auth.user.unwrap();
        // Both follower and following count increment on the same user.
        assert_eq!(user.following_count, 1);
        let alice = app.backend.users.get("alice").unwrap().unwrap();
        assert_eq!(alice.follower_count, 1);
    }

    // --- Error 6: Double follow is idempotent ---

    #[tokio::test]
    async fn error_double_follow_idempotent() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Follow bob twice.
        app.flux.emit("user/follow", FollowUserReq { user_id: "bob".into() }).await;
        app.flux.emit("user/follow", FollowUserReq { user_id: "bob".into() }).await;

        // Second follow fails (duplicate key), so count stays at 1.
        let auth = get_auth(&app);
        assert_eq!(auth.user.unwrap().following_count, 1);
        let bob = app.backend.users.get("bob").unwrap().unwrap();
        assert_eq!(bob.follower_count, 1);
    }

    // --- Error 7: Unfollow someone not followed ---

    #[tokio::test]
    async fn error_unfollow_not_followed() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Unfollow bob without following first.
        app.flux.emit("user/unfollow", UnfollowUserReq { user_id: "bob".into() }).await;

        // Should not crash. Counts should remain 0.
        let auth = get_auth(&app);
        assert_eq!(auth.user.unwrap().following_count, 0);
        let bob = app.backend.users.get("bob").unwrap().unwrap();
        assert_eq!(bob.follower_count, 0);
    }

    // --- Error 8: Load profile of non-existent user ---

    #[tokio::test]
    async fn error_load_nonexistent_profile() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Load profile of user that doesn't exist.
        app.flux.emit("profile/load", LoadProfileReq { user_id: "nobody".into() }).await;

        // No profile state should be set.
        assert!(app.flux.get("profile/nobody").is_none());
        // Route should NOT change.
        assert_eq!(get_route(&app), "/home");
    }

    // --- Error 9: Load non-existent tweet detail ---

    #[tokio::test]
    async fn error_load_nonexistent_tweet() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        app.flux.emit("tweet/load", LoadTweetReq { tweet_id: "ghost-id".into() }).await;

        assert!(app.flux.get("tweet/ghost-id").is_none());
        assert_eq!(get_route(&app), "/home");
    }

    // --- Error 10: Emit to non-existent handler ---

    #[tokio::test]
    async fn error_emit_unknown_path() {
        let app = setup_twitter();
        app.flux.emit("app/initialize", ()).await;

        // Emit to a path with no handler — should be silent no-op.
        app.flux.emit("completely/unknown/path", ()).await;

        // App state unchanged.
        assert_eq!(get_auth(&app).phase, "unauthenticated");
    }

    // --- Error 11: Double logout ---

    #[tokio::test]
    async fn error_double_logout() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;
        app.flux.emit("auth/logout", ()).await;
        app.flux.emit("auth/logout", ()).await; // second logout

        // Should not crash. Still unauthenticated.
        assert_eq!(get_auth(&app).phase, "unauthenticated");
        assert_eq!(get_route(&app), "/login");
    }

    // --- Error 12: Login, logout, login again ---

    #[tokio::test]
    async fn error_relogin_fresh_state() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Like a tweet.
        let feed = get_feed(&app);
        let tweet_id = feed.items[0].tweet_id.clone();
        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: tweet_id.clone() }).await;

        // Logout.
        app.flux.emit("auth/logout", ()).await;
        assert!(app.flux.get("timeline/feed").is_none());

        // Login as different user.
        app.flux.emit("auth/login", LoginReq { username: "bob".into() }).await;

        let auth = get_auth(&app);
        assert_eq!(auth.user.as_ref().unwrap().username, "bob");

        // Timeline reloaded — Bob hasn't liked anything.
        let feed = get_feed(&app);
        let item = feed.items.iter().find(|i| i.tweet_id == tweet_id).unwrap();
        assert!(!item.liked_by_me); // Bob hasn't liked it
        assert_eq!(item.like_count, 1); // Alice's like persists in backend
    }

    // =====================================================================
    // EDGE CASES
    // =====================================================================

    // --- Edge 1: Tweet exactly 280 characters (boundary) ---

    #[tokio::test]
    async fn edge_tweet_exactly_280_chars() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let content = "x".repeat(280);
        app.flux.emit("tweet/create", CreateTweetReq {
            content: content.clone(), reply_to_id: None,
        }).await;

        let compose = get_compose(&app);
        assert!(compose.error.is_none(), "280 chars should be accepted");

        let feed = get_feed(&app);
        assert_eq!(feed.items.len(), 1);
        assert_eq!(feed.items[0].content.len(), 280);
    }

    // --- Edge 2: Tweet at 279 characters ---

    #[tokio::test]
    async fn edge_tweet_279_chars() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let content = "a".repeat(279);
        app.flux.emit("tweet/create", CreateTweetReq {
            content, reply_to_id: None,
        }).await;

        assert!(get_compose(&app).error.is_none());
        assert_eq!(get_feed(&app).items.len(), 1);
    }

    // --- Edge 3: Tweet at 281 characters (just over) ---

    #[tokio::test]
    async fn edge_tweet_281_chars() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let content = "b".repeat(281);
        app.flux.emit("tweet/create", CreateTweetReq {
            content, reply_to_id: None,
        }).await;

        assert!(get_compose(&app).error.is_some());
        // No tweet created.
        let feed = get_feed(&app);
        assert!(feed.items.is_empty());
    }

    // --- Edge 4: Tweet with unicode and emoji ---

    #[tokio::test]
    async fn edge_tweet_unicode_emoji() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let content = "Hello 你好 🌍 Привет مرحبا こんにちは 🚀🎉";
        app.flux.emit("tweet/create", CreateTweetReq {
            content: content.into(), reply_to_id: None,
        }).await;

        let feed = get_feed(&app);
        assert_eq!(feed.items.len(), 1);
        assert_eq!(feed.items[0].content, content);
    }

    // --- Edge 5: Tweet with only whitespace variants ---

    #[tokio::test]
    async fn edge_tweet_whitespace_variants() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Tab, newline, spaces.
        for content in &[" ", "\t", "\n", "  \t\n  "] {
            app.flux.emit("tweet/create", CreateTweetReq {
                content: content.to_string(), reply_to_id: None,
            }).await;

            let compose = get_compose(&app);
            assert!(compose.error.is_some(), "whitespace-only '{}' should be rejected", content.escape_debug());
        }

        // No tweets created.
        let feed = get_feed(&app);
        assert!(feed.items.is_empty());
    }

    // --- Edge 6: Empty timeline ---

    #[tokio::test]
    async fn edge_empty_timeline() {
        let app = setup_twitter();
        seed_users(&app);
        // No tweets seeded.

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let feed = get_feed(&app);
        assert!(feed.items.is_empty());
        assert!(!feed.loading);
        assert!(feed.error.is_none());
    }

    // --- Edge 7: Timeline shows only top-level (no replies) ---

    #[tokio::test]
    async fn edge_timeline_excludes_replies() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Post a tweet.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Original".into(), reply_to_id: None,
        }).await;

        let feed = get_feed(&app);
        let parent_id = feed.items[0].tweet_id.clone();

        // Reply to it.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Reply 1".into(), reply_to_id: Some(parent_id.clone()),
        }).await;
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Reply 2".into(), reply_to_id: Some(parent_id.clone()),
        }).await;

        // Timeline should still only show 1 item (original tweet).
        let feed = get_feed(&app);
        assert_eq!(feed.items.len(), 1);
        assert_eq!(feed.items[0].content, "Original");
        assert_eq!(feed.items[0].reply_count, 2);
    }

    // --- Edge 8: Profile with no tweets ---

    #[tokio::test]
    async fn edge_profile_no_tweets() {
        let app = setup_twitter();
        seed_users(&app);
        // dave has no tweets.

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        app.flux.emit("profile/load", LoadProfileReq { user_id: "dave".into() }).await;

        let profile = app.flux.get("profile/dave").unwrap();
        let page = profile.downcast_ref::<ProfilePage>().unwrap();
        assert_eq!(page.user.username, "dave");
        assert!(page.tweets.is_empty());
        assert_eq!(page.user.tweet_count, 0);
    }

    // --- Edge 9: Like-unlike-like cycle ---

    #[tokio::test]
    async fn edge_like_unlike_like_cycle() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let feed = get_feed(&app);
        let tweet_id = feed.items[0].tweet_id.clone();

        // Like → unlike → like.
        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: tweet_id.clone() }).await;
        let feed = get_feed(&app);
        assert_eq!(feed.items.iter().find(|i| i.tweet_id == tweet_id).unwrap().like_count, 1);

        app.flux.emit("tweet/unlike", UnlikeTweetReq { tweet_id: tweet_id.clone() }).await;
        let feed = get_feed(&app);
        assert_eq!(feed.items.iter().find(|i| i.tweet_id == tweet_id).unwrap().like_count, 0);

        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: tweet_id.clone() }).await;
        let feed = get_feed(&app);
        let item = feed.items.iter().find(|i| i.tweet_id == tweet_id).unwrap();
        assert_eq!(item.like_count, 1);
        assert!(item.liked_by_me);
    }

    // --- Edge 10: Follow-unfollow-follow cycle ---

    #[tokio::test]
    async fn edge_follow_unfollow_follow_cycle() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Follow → unfollow → follow.
        app.flux.emit("user/follow", FollowUserReq { user_id: "bob".into() }).await;
        assert_eq!(get_auth(&app).user.unwrap().following_count, 1);

        app.flux.emit("user/unfollow", UnfollowUserReq { user_id: "bob".into() }).await;
        assert_eq!(get_auth(&app).user.unwrap().following_count, 0);

        app.flux.emit("user/follow", FollowUserReq { user_id: "bob".into() }).await;
        assert_eq!(get_auth(&app).user.unwrap().following_count, 1);

        let bob = app.backend.users.get("bob").unwrap().unwrap();
        assert_eq!(bob.follower_count, 1);
    }

    // --- Edge 11: Follow multiple users ---

    #[tokio::test]
    async fn edge_follow_multiple_users() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        app.flux.emit("user/follow", FollowUserReq { user_id: "bob".into() }).await;
        app.flux.emit("user/follow", FollowUserReq { user_id: "carol".into() }).await;
        app.flux.emit("user/follow", FollowUserReq { user_id: "dave".into() }).await;

        let auth = get_auth(&app);
        assert_eq!(auth.user.unwrap().following_count, 3);

        // Each followee has 1 follower.
        for name in &["bob", "carol", "dave"] {
            let u = app.backend.users.get(name).unwrap().unwrap();
            assert_eq!(u.follower_count, 1, "{} should have 1 follower", name);
        }
    }

    // --- Edge 12: Multiple likes from different users ---

    #[tokio::test]
    async fn edge_multiple_users_like_same_tweet() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        // Alice likes.
        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;
        let feed = get_feed(&app);
        let tweet_id = feed.items[0].tweet_id.clone();
        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: tweet_id.clone() }).await;

        // Bob likes the same tweet.
        app.flux.emit("auth/logout", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "bob".into() }).await;
        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: tweet_id.clone() }).await;

        // Carol likes the same tweet.
        app.flux.emit("auth/logout", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "carol".into() }).await;
        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: tweet_id.clone() }).await;

        // Tweet should have 3 likes.
        let tweet = app.backend.tweets.get(&tweet_id).unwrap().unwrap();
        assert_eq!(tweet.like_count, 3);

        // Carol's view: liked_by_me = true.
        let feed = get_feed(&app);
        let item = feed.items.iter().find(|i| i.tweet_id == tweet_id).unwrap();
        assert!(item.liked_by_me);
        assert_eq!(item.like_count, 3);
    }

    // --- Edge 13: Reply to a reply (nested) ---

    #[tokio::test]
    async fn edge_reply_to_reply() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Original tweet.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Root tweet".into(), reply_to_id: None,
        }).await;
        let feed = get_feed(&app);
        let root_id = feed.items[0].tweet_id.clone();

        // Reply to root.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Reply level 1".into(), reply_to_id: Some(root_id.clone()),
        }).await;

        // Get the reply's ID from backend.
        let all_tweets = app.backend.tweets.list().unwrap();
        let reply1 = all_tweets.iter()
            .find(|t| t.content == "Reply level 1")
            .unwrap();
        let reply1_id = reply1.id.to_string();

        // Reply to the reply.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Reply level 2".into(), reply_to_id: Some(reply1_id.clone()),
        }).await;

        // Root has 1 direct reply.
        let root = app.backend.tweets.get(&root_id).unwrap().unwrap();
        assert_eq!(root.reply_count, 1);

        // Reply1 has 1 reply.
        let r1 = app.backend.tweets.get(&reply1_id).unwrap().unwrap();
        assert_eq!(r1.reply_count, 1);

        // Load root detail — shows only direct replies.
        app.flux.emit("tweet/load", LoadTweetReq { tweet_id: root_id.clone() }).await;
        let detail = app.flux.get(&format!("tweet/{}", root_id)).unwrap();
        let detail = detail.downcast_ref::<TweetDetail>().unwrap();
        assert_eq!(detail.replies.len(), 1);
        assert_eq!(detail.replies[0].content, "Reply level 1");
    }

    // --- Edge 14: Tweet with reply_to_id to non-existent tweet ---

    #[tokio::test]
    async fn edge_reply_to_nonexistent_tweet() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Reply to a non-existent parent.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Orphan reply".into(),
            reply_to_id: Some("nonexistent-parent".into()),
        }).await;

        // Tweet is created (no validation on parent existence).
        let compose = get_compose(&app);
        assert!(compose.error.is_none());

        // But it won't appear in timeline (it's a reply).
        let feed = get_feed(&app);
        assert!(feed.items.is_empty());
    }

    // --- Edge 15: Subscribe to specific patterns ---

    #[tokio::test]
    async fn edge_subscribe_specific_patterns() {
        let app = setup_twitter();
        seed_users(&app);

        let auth_changes = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let timeline_changes = Arc::new(std::sync::Mutex::new(0u32));
        let ac = auth_changes.clone();
        let tc = timeline_changes.clone();

        // Subscribe only to auth/*.
        app.flux.subscribe("auth/+", move |path, _| {
            ac.lock().unwrap().push(path.to_string());
        });
        // Subscribe only to timeline/*.
        app.flux.subscribe("timeline/+", move |_, _| {
            *tc.lock().unwrap() += 1;
        });

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let auth_paths = auth_changes.lock().unwrap();
        // auth/state changed multiple times (init + busy + result).
        assert!(auth_paths.len() >= 2);
        assert!(auth_paths.iter().all(|p| p.starts_with("auth/")));

        let tc = *timeline_changes.lock().unwrap();
        assert!(tc >= 1, "timeline/feed should have been set at least once");
    }

    // --- Edge 16: Unsubscribe then verify ---

    #[tokio::test]
    async fn edge_unsubscribe_verified() {
        let app = setup_twitter();
        seed_users(&app);

        let count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let c = count.clone();
        let id = app.flux.subscribe("auth/state", move |_, _| {
            c.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });

        app.flux.emit("app/initialize", ()).await;
        let after_init = count.load(std::sync::atomic::Ordering::Relaxed);
        assert!(after_init >= 1);

        app.flux.unsubscribe("auth/state", id);

        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;
        let after_login = count.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(after_init, after_login, "no more notifications after unsubscribe");
    }

    // --- Edge 17: Scan for all profile states ---

    #[tokio::test]
    async fn edge_scan_profile_states() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Load multiple profiles.
        app.flux.emit("profile/load", LoadProfileReq { user_id: "bob".into() }).await;
        app.flux.emit("profile/load", LoadProfileReq { user_id: "carol".into() }).await;

        // Scan for all profile/* states.
        let profiles = app.flux.scan("profile");
        assert_eq!(profiles.len(), 2);

        let paths: Vec<&str> = profiles.iter().map(|(k, _)| k.as_str()).collect();
        assert!(paths.contains(&"profile/bob"));
        assert!(paths.contains(&"profile/carol"));
    }

    // --- Edge 18: Snapshot captures complete state ---

    #[tokio::test]
    async fn edge_snapshot_complete() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let snap = app.flux.snapshot();
        let paths: Vec<&str> = snap.iter().map(|(k, _)| k.as_str()).collect();

        assert!(paths.contains(&"auth/state"));
        assert!(paths.contains(&"app/route"));
        assert!(paths.contains(&"timeline/feed"));
    }

    // --- Edge 19: Initialize can be called multiple times ---

    #[tokio::test]
    async fn edge_multiple_initialize() {
        let app = setup_twitter();

        app.flux.emit("app/initialize", ()).await;
        assert_eq!(get_auth(&app).phase, "unauthenticated");

        app.flux.emit("app/initialize", ()).await;
        assert_eq!(get_auth(&app).phase, "unauthenticated");
        assert_eq!(get_route(&app), "/login");
    }

    // --- Edge 20: Compose state cleared after successful tweet ---

    #[tokio::test]
    async fn edge_compose_cleared_after_tweet() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Type something, then post.
        app.flux.emit("compose/update-field", UpdateFieldReq {
            field: "content".into(), value: "Draft tweet".into(),
        }).await;
        assert_eq!(get_compose(&app).content, "Draft tweet");

        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Draft tweet".into(), reply_to_id: None,
        }).await;

        // Compose is cleared.
        let compose = get_compose(&app);
        assert!(compose.content.is_empty());
        assert!(compose.reply_to_id.is_none());
        assert!(!compose.busy);
        assert!(compose.error.is_none());
    }

    // --- Edge 21: Compose error cleared on next field update ---

    #[tokio::test]
    async fn edge_compose_error_cleared_on_edit() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Trigger an error.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "   ".into(), reply_to_id: None,
        }).await;
        assert!(get_compose(&app).error.is_some());

        // Edit field — error should clear.
        app.flux.emit("compose/update-field", UpdateFieldReq {
            field: "content".into(), value: "Fixed".into(),
        }).await;
        assert!(get_compose(&app).error.is_none());
        assert_eq!(get_compose(&app).content, "Fixed");
    }

    // --- Edge 22: Profile shows followed_by_me correctly ---

    #[tokio::test]
    async fn edge_profile_followed_state() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Not following → followed_by_me = false.
        app.flux.emit("profile/load", LoadProfileReq { user_id: "bob".into() }).await;
        let p = app.flux.get("profile/bob").unwrap();
        assert!(!p.downcast_ref::<ProfilePage>().unwrap().followed_by_me);

        // Follow.
        app.flux.emit("user/follow", FollowUserReq { user_id: "bob".into() }).await;

        // Re-load profile.
        app.flux.emit("profile/load", LoadProfileReq { user_id: "bob".into() }).await;
        let p = app.flux.get("profile/bob").unwrap();
        assert!(p.downcast_ref::<ProfilePage>().unwrap().followed_by_me);
        assert_eq!(p.downcast_ref::<ProfilePage>().unwrap().user.follower_count, 1);
    }

    // --- Edge 23: Many tweets pagination-like behavior ---

    #[tokio::test]
    async fn edge_many_tweets() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Post 50 tweets.
        for i in 0..50 {
            app.flux.emit("tweet/create", CreateTweetReq {
                content: format!("Tweet number {}", i),
                reply_to_id: None,
            }).await;
        }

        let feed = get_feed(&app);
        assert_eq!(feed.items.len(), 50);
        // All from alice.
        assert!(feed.items.iter().all(|i| i.author.username == "alice"));
        // User tweet count.
        let alice = app.backend.users.get("alice").unwrap().unwrap();
        assert_eq!(alice.tweet_count, 50);
    }

    // --- Edge 24: Tweet detail includes author info ---

    #[tokio::test]
    async fn edge_tweet_detail_author_info() {
        let app = setup_twitter();
        seed_users(&app);
        seed_tweets(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        let feed = get_feed(&app);
        let tweet_id = feed.items.iter()
            .find(|i| i.author.username == "bob")
            .unwrap().tweet_id.clone();

        app.flux.emit("tweet/load", LoadTweetReq { tweet_id: tweet_id.clone() }).await;

        let detail = app.flux.get(&format!("tweet/{}", tweet_id)).unwrap();
        let detail = detail.downcast_ref::<TweetDetail>().unwrap();
        assert_eq!(detail.tweet.author.username, "bob");
        assert_eq!(detail.tweet.author.display_name, "Bob Li");
        assert!(detail.tweet.author.avatar.is_some());
    }

    // --- Edge 25: State value ref counting (zero-copy verification) ---

    #[tokio::test]
    async fn edge_zero_copy_ref_counting() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Get the same state twice.
        let v1 = app.flux.get("auth/state").unwrap();
        let v2 = app.flux.get("auth/state").unwrap();

        // Both are Arc-cloned, sharing the same data.
        assert_eq!(v1.ref_count(), v2.ref_count());
        assert!(v1.ref_count() >= 2); // at least store + v1/v2
    }

    // --- Edge 26: Timeline refreshes after tweet/like ---

    #[tokio::test]
    async fn edge_timeline_refresh_consistency() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Post tweet.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "First".into(), reply_to_id: None,
        }).await;
        assert_eq!(get_feed(&app).items.len(), 1);

        // Post another.
        app.flux.emit("tweet/create", CreateTweetReq {
            content: "Second".into(), reply_to_id: None,
        }).await;
        assert_eq!(get_feed(&app).items.len(), 2);

        // Like first tweet.
        let first_id = get_feed(&app).items.iter()
            .find(|i| i.content == "First")
            .unwrap().tweet_id.clone();
        app.flux.emit("tweet/like", LikeTweetReq { tweet_id: first_id.clone() }).await;

        // Timeline still has 2 items, first has like.
        let feed = get_feed(&app);
        assert_eq!(feed.items.len(), 2);
        let first = feed.items.iter().find(|i| i.tweet_id == first_id).unwrap();
        assert_eq!(first.like_count, 1);
        assert!(first.liked_by_me);
    }

    // --- Edge 27: Explicit timeline/load refresh ---

    #[tokio::test]
    async fn edge_explicit_timeline_refresh() {
        let app = setup_twitter();
        seed_users(&app);

        app.flux.emit("app/initialize", ()).await;
        app.flux.emit("auth/login", LoginReq { username: "alice".into() }).await;

        // Empty timeline.
        assert!(get_feed(&app).items.is_empty());

        // Seed a tweet directly into backend (simulating another user posting).
        app.backend.tweets.save_new(Tweet {
            id: Id::default(),
            author_id: Id::new("bob"),
            content: "Backend-injected tweet".into(),
            like_count: 0, reply_count: 0, reply_to_id: None,
            display_name: None, description: None, metadata: None,
            created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();

        // Timeline still empty (not refreshed yet).
        // The old feed is still in store.
        let feed = get_feed(&app);
        assert!(feed.items.is_empty());

        // Explicitly refresh.
        app.flux.emit("timeline/load", ()).await;

        let feed = get_feed(&app);
        assert_eq!(feed.items.len(), 1);
        assert_eq!(feed.items[0].content, "Backend-injected tweet");
    }
}
