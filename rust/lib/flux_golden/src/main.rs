//! Twitter server — standalone binary with admin dashboard.
//!
//! Usage: bazel run //rust/lib/flux_golden:twitterd

use std::sync::Arc;

use axum::routing::{get, post};
use axum::{Json, Router};
use tracing::info;

use openerp_store::KvOps;
use openerp_types::*;

use flux_golden::server::model::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // In-memory storage (temp directory).
    let dir = tempfile::tempdir()?;
    let db_path = dir.path().join("twitter.redb");
    info!("Database: {}", db_path.display());

    let kv: Arc<dyn openerp_kv::KVStore> =
        Arc::new(openerp_kv::RedbStore::open(&db_path)?);

    // Seed test data.
    seed_data(&kv);
    info!("Seeded test data");

    // AllowAll auth — no JWT needed for admin API.
    let auth: Arc<dyn openerp_core::Authenticator> = Arc::new(openerp_core::AllowAll);

    // Build schema from DSL.
    let schema_json = openerp_store::build_schema("Twitter", vec![
        flux_golden::server::schema_def(),
    ]);

    // Build router.
    let twitter_admin = flux_golden::server::admin_router(kv, auth);

    let schema = schema_json.clone();
    let app = Router::new()
        .route("/", get(|| async {
            axum::response::Html(openerp_web::login_html())
        }))
        .route("/dashboard", get(|| async {
            axum::response::Html(openerp_web::dashboard_html())
        }))
        .route("/meta/schema", get(move || {
            let s = schema.clone();
            async move { Json(s) }
        }))
        .route("/health", get(|| async {
            Json(serde_json::json!({"status": "ok"}))
        }))
        .route("/auth/login", post(login_handler))
        .nest("/admin/twitter", twitter_admin);

    let listen = "0.0.0.0:3000";
    info!("Twitter server listening on http://{}", listen);
    info!("Dashboard: http://localhost:3000/dashboard");
    info!("Login: root / any password");

    let listener = tokio::net::TcpListener::bind(listen).await?;
    axum::serve(listener, app).await?;

    // Keep temp dir alive for server lifetime.
    drop(dir);
    Ok(())
}

// ── Login handler — accepts any password, returns a simple JWT ──

#[derive(serde::Deserialize)]
struct LoginReq {
    username: String,
    #[allow(dead_code)]
    password: String,
}

async fn login_handler(Json(body): Json<LoginReq>) -> Json<serde_json::Value> {
    // Simple JWT: header.payload.signature (HS256 with "secret").
    // The admin routes use AllowAll, so the token content doesn't matter.
    // The dashboard just needs it to exist in localStorage.
    let header = base64_url_encode(r#"{"alg":"HS256","typ":"JWT"}"#);
    let now = chrono::Utc::now().timestamp();
    let payload_json = serde_json::json!({
        "sub": body.username,
        "name": body.username,
        "roles": ["admin"],
        "iat": now,
        "exp": now + 86400,
    });
    let payload = base64_url_encode(&payload_json.to_string());
    // Fake signature — dashboard doesn't validate, admin uses AllowAll.
    let signature = base64_url_encode("golden-test-signature");
    let token = format!("{}.{}.{}", header, payload, signature);

    Json(serde_json::json!({
        "access_token": token,
        "token_type": "Bearer",
        "expires_in": 86400,
    }))
}

fn base64_url_encode(input: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input.as_bytes())
}

// ── Seed data ──

fn seed_data(kv: &Arc<dyn openerp_kv::KVStore>) {
    let users_ops = KvOps::<User>::new(kv.clone());
    let tweets_ops = KvOps::<Tweet>::new(kv.clone());
    let likes_ops = KvOps::<Like>::new(kv.clone());
    let follows_ops = KvOps::<Follow>::new(kv.clone());

    // ── Users ──
    let users = vec![
        ("alice", "Alice Wang", "Rust developer & open source enthusiast. Building the future with zero-cost abstractions."),
        ("bob", "Bob Li", "Product designer at Haivivi. Dark mode advocate."),
        ("carol", "Carol Zhang", "Full-stack engineer. Cap'n Proto fan. 🚀"),
        ("dave", "Dave Chen", "New to Twitter. Just here to lurk."),
        ("eve", "Eve Liu", "DevOps engineer. Kubernetes wrangler. ☁️"),
    ];
    for &(username, display, bio) in &users {
        users_ops.save_new(User {
            id: Id::default(),
            username: username.to_string(),
            bio: Some(bio.to_string()),
            avatar: Some(Avatar::new(&format!("https://api.dicebear.com/7.x/initials/svg?seed={}", username))),
            follower_count: 0,
            following_count: 0,
            tweet_count: 0,
            display_name: Some(display.to_string()),
            description: Some(format!("@{}", username)),
            metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();
    }

    // ── Tweets (with delays for ordering) ──
    let tweets_data: Vec<(&str, &str, Option<&str>)> = vec![
        ("alice", "Just shipped a new feature in Rust! The borrow checker is my best friend. 🦀", None),
        ("bob", "New design system is looking great. Dark mode coming soon. 🌙", None),
        ("carol", "TIL: Arc<dyn Any> is basically free for zero-copy state sharing. Mind blown.", None),
        ("alice", "Anyone else excited about Cap'n Proto for FFI? Zero-copy across languages! No more serde overhead.", None),
        ("dave", "Hello Twitter! This is my first tweet. 👋", None),
        ("eve", "Just automated our entire deployment pipeline. 15 minutes → 2 minutes. 📉", None),
        ("bob", "Design tip: always test your UI with real data, not lorem ipsum. The difference is night and day.", None),
        ("carol", "Hot take: Bazel > Cargo for large Rust monorepos. Fight me.", None),
        ("alice", "Working on a cross-platform state engine called Flux. Rust holds all the state, each platform just renders. 🔥", None),
        ("eve", "Pro tip: `kubectl get pods -o wide` is your best friend when debugging networking issues.", None),
    ];

    let mut tweet_ids = Vec::new();
    for &(author, content, reply_to) in &tweets_data {
        let tweet = Tweet {
            id: Id::default(),
            author: Name::new(&format!("twitter/users/{}", author)),
            content: content.to_string(),
            image_url: None,
            like_count: 0,
            reply_count: 0,
            reply_to: reply_to.map(|s| Name::new(&format!("twitter/tweets/{}", s))),
            display_name: None,
            description: None,
            metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
        };
        let created = tweets_ops.save_new(tweet).unwrap();
        tweet_ids.push(created.id.to_string());

        // Update author tweet count.
        if let Ok(Some(mut user)) = users_ops.get(author) {
            user.tweet_count += 1;
            let _ = users_ops.save(user);
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    // ── Replies ──
    let replies = vec![
        ("bob", "Congrats Alice! What feature did you ship?", 0),    // reply to tweet[0]
        ("carol", "Totally agree! Flux sounds amazing.", 8),          // reply to tweet[8]
        ("dave", "Can you share some resources about Cap'n Proto?", 3), // reply to tweet[3]
        ("eve", "Bazel is great but the learning curve is real 😅", 7),  // reply to tweet[7]
        ("alice", "Thanks Bob! It's a path-based state engine for cross-platform apps.", 0), // reply to tweet[0]
    ];
    for &(author, content, parent_idx) in &replies {
        let parent_id = &tweet_ids[parent_idx];
        tweets_ops.save_new(Tweet {
            id: Id::default(),
            author: Name::new(&format!("twitter/users/{}", author)),
            content: content.to_string(),
            image_url: None,
            like_count: 0,
            reply_count: 0,
            reply_to: Some(Name::new(&format!("twitter/tweets/{}", parent_id))),
            display_name: None,
            description: None,
            metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
        }).unwrap();
        // Increment parent reply count.
        if let Ok(Some(mut parent)) = tweets_ops.get(parent_id) {
            parent.reply_count += 1;
            let _ = tweets_ops.save(parent);
        }
        // Increment author tweet count.
        if let Ok(Some(mut user)) = users_ops.get(author) {
            user.tweet_count += 1;
            let _ = users_ops.save(user);
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    // ── Likes ──
    let likes = vec![
        ("bob", 0), ("carol", 0), ("eve", 0),    // 3 likes on alice's first tweet
        ("alice", 1), ("carol", 1),                // 2 likes on bob's design tweet
        ("alice", 2), ("bob", 2),                  // 2 likes on carol's Arc<dyn Any> tweet
        ("bob", 3), ("eve", 3),                    // 2 likes on alice's capnp tweet
        ("alice", 5),                              // 1 like on eve's deployment tweet
        ("carol", 7), ("eve", 7), ("alice", 7),   // 3 likes on carol's bazel hot take
        ("bob", 8), ("carol", 8), ("dave", 8), ("eve", 8), // 4 likes on alice's flux tweet
    ];
    for &(liker, tweet_idx) in &likes {
        let tweet_id = &tweet_ids[tweet_idx];
        let _ = likes_ops.save_new(Like {
            id: Id::default(),
            user: Name::new(&format!("twitter/users/{}", liker)),
            tweet: Name::new(&format!("twitter/tweets/{}", tweet_id)),
            display_name: None,
            description: None,
            metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
        });
        // Increment tweet like count.
        if let Ok(Some(mut tweet)) = tweets_ops.get(tweet_id) {
            tweet.like_count += 1;
            let _ = tweets_ops.save(tweet);
        }
    }

    // ── Follows ──
    let follow_pairs = vec![
        ("bob", "alice"), ("carol", "alice"), ("dave", "alice"), ("eve", "alice"), // 4 follow alice
        ("alice", "bob"), ("carol", "bob"),                                        // 2 follow bob
        ("alice", "carol"), ("bob", "carol"), ("eve", "carol"),                   // 3 follow carol
        ("alice", "eve"),                                                          // 1 follow eve
    ];
    for &(follower, followee) in &follow_pairs {
        let _ = follows_ops.save_new(Follow {
            id: Id::default(),
            follower: Name::new(&format!("twitter/users/{}", follower)),
            followee: Name::new(&format!("twitter/users/{}", followee)),
            display_name: None,
            description: None,
            metadata: None, created_at: DateTime::default(), updated_at: DateTime::default(),
        });
        // Update counts.
        if let Ok(Some(mut user)) = users_ops.get(follower) {
            user.following_count += 1;
            let _ = users_ops.save(user);
        }
        if let Ok(Some(mut user)) = users_ops.get(followee) {
            user.follower_count += 1;
            let _ = users_ops.save(user);
        }
    }

    // ── Messages (站内信) ── demonstrate LocalizedText
    let messages_ops = KvOps::<Message>::new(kv.clone());

    let mut welcome_title = LocalizedText::new();
    welcome_title.set("en", "Welcome to TwitterFlux!");
    welcome_title.set("zh-CN", "欢迎来到 TwitterFlux！");
    welcome_title.set("ja", "TwitterFlux へようこそ！");
    welcome_title.set("es", "¡Bienvenido a TwitterFlux!");

    let mut welcome_body = LocalizedText::new();
    welcome_body.set("en", "Thanks for joining our community. Start by following some users and posting your first tweet!");
    welcome_body.set("zh-CN", "感谢加入我们的社区。快去关注一些用户，发你的第一条推文吧！");
    welcome_body.set("ja", "コミュニティへの参加ありがとうございます。ユーザーをフォローして、最初のツイートを投稿しましょう！");
    welcome_body.set("es", "Gracias por unirte a nuestra comunidad. ¡Empieza siguiendo a algunos usuarios y publicando tu primer tweet!");

    // Broadcast to all users
    let _ = messages_ops.save_new(Message {
        id: Id::default(), kind: "broadcast".into(),
        sender: None, recipient: None,
        title: welcome_title, body: welcome_body, read: false,
        display_name: None, description: None, metadata: None,
        created_at: DateTime::default(), updated_at: DateTime::default(),
    });

    let mut update_title = LocalizedText::new();
    update_title.set("en", "New Feature: Multi-language Support");
    update_title.set("zh-CN", "新功能：多语言支持");
    update_title.set("ja", "新機能：多言語サポート");
    update_title.set("es", "Nueva función: Soporte multilingüe");

    let mut update_body = LocalizedText::new();
    update_body.set("en", "You can now switch between English, Chinese, Japanese, and Spanish in Settings. Your messages will be displayed in your preferred language.");
    update_body.set("zh-CN", "现在你可以在设置中切换英文、中文、日文和西班牙文。站内信会以你选择的语言显示。");
    update_body.set("ja", "設定から英語、中国語、日本語、スペイン語を切り替えられるようになりました。メッセージは選択した言語で表示されます。");
    update_body.set("es", "Ahora puedes cambiar entre inglés, chino, japonés y español en Configuración. Los mensajes se mostrarán en tu idioma preferido.");

    let _ = messages_ops.save_new(Message {
        id: Id::default(), kind: "system".into(),
        sender: None, recipient: None,
        title: update_title, body: update_body, read: false,
        display_name: None, description: None, metadata: None,
        created_at: DateTime::default(), updated_at: DateTime::default(),
    });

    // Personal message to alice
    let mut personal_title = LocalizedText::en("Your account has been verified");
    personal_title.set("zh-CN", "你的账号已通过认证");
    personal_title.set("ja", "アカウントが認証されました");
    personal_title.set("es", "Tu cuenta ha sido verificada");

    let mut personal_body = LocalizedText::en("Congratulations! Your developer account has been verified. You now have access to the API dashboard.");
    personal_body.set("zh-CN", "恭喜！你的开发者账号已通过认证。现在你可以访问 API 管理面板了。");
    personal_body.set("ja", "おめでとうございます！開発者アカウントが認証されました。APIダッシュボードにアクセスできるようになりました。");
    personal_body.set("es", "¡Felicitaciones! Tu cuenta de desarrollador ha sido verificada. Ahora tienes acceso al panel de API.");

    let _ = messages_ops.save_new(Message {
        id: Id::default(), kind: "personal".into(),
        sender: None, recipient: Some(Name::new("twitter/users/alice")),
        title: personal_title, body: personal_body, read: false,
        display_name: None, description: None, metadata: None,
        created_at: DateTime::default(), updated_at: DateTime::default(),
    });

    info!("Seeded: {} users, {} tweets (+ {} replies), {} likes, {} follows, 3 messages",
        users.len(), tweets_data.len(), replies.len(), likes.len(), follow_pairs.len());
}
