//! Twitter app i18n translations — 4 languages.
//!
//! Registers all UI text with the I18nStore.
//! Golden test: hand-written. Production: generated from DSL definitions.

use std::collections::HashMap;
use std::sync::Arc;

use openerp_flux::{I18nHandler, I18nStore, QueryParams};

/// Register all Twitter app translations.
pub fn register_all(i18n: &I18nStore) {
    i18n.handle("ui/#", Arc::new(UiStrings::new()));
    i18n.handle("error/#", Arc::new(ErrorStrings::new()));
    i18n.handle("format/#", Arc::new(FormatStrings));
}

// ── UI Strings ──

struct UiStrings {
    data: HashMap<&'static str, [&'static str; 4]>, // [en, zh-CN, ja, es]
}

const EN: usize = 0;
const ZH: usize = 1;
const JA: usize = 2;
const ES: usize = 3;

fn locale_index(locale: &str) -> usize {
    match locale {
        "zh-CN" | "zh" => ZH,
        "ja" => JA,
        "es" => ES,
        _ => EN,
    }
}

impl UiStrings {
    fn new() -> Self {
        let mut m = HashMap::new();

        // Login
        m.insert("ui/login/title", ["Welcome back", "欢迎回来", "おかえりなさい", "Bienvenido"]);
        m.insert("ui/login/subtitle", ["Sign in to your account", "登录你的账号", "アカウントにサインイン", "Inicia sesión en tu cuenta"]);
        m.insert("ui/login/username", ["Username", "用户名", "ユーザー名", "Usuario"]);
        m.insert("ui/login/password", ["Password", "密码", "パスワード", "Contraseña"]);
        m.insert("ui/login/button", ["Sign In", "登录", "サインイン", "Iniciar sesión"]);
        m.insert("ui/login/hint", ["Try: alice, bob, or carol", "试试: alice, bob, carol", "alice, bob, carol を試してください", "Prueba: alice, bob o carol"]);

        // Tab bar
        m.insert("ui/tab/home", ["Home", "首页", "ホーム", "Inicio"]);
        m.insert("ui/tab/search", ["Search", "搜索", "検索", "Buscar"]);
        m.insert("ui/tab/me", ["Me", "我", "マイページ", "Yo"]);

        // Home
        m.insert("ui/home/title", ["Home", "首页", "ホーム", "Inicio"]);
        m.insert("ui/home/empty", ["No tweets yet", "还没有推文", "ツイートはまだありません", "Aún no hay tweets"]);
        m.insert("ui/home/empty_hint", ["Be the first to tweet!", "来发第一条推文吧！", "最初のツイートを投稿しよう！", "¡Sé el primero en tuitear!"]);

        // Compose
        m.insert("ui/compose/title", ["Compose", "发推", "ツイート作成", "Redactar"]);
        m.insert("ui/compose/placeholder", ["What's happening?", "有什么新鲜事？", "いまどうしてる？", "¿Qué está pasando?"]);
        m.insert("ui/compose/post", ["Post", "发布", "投稿", "Publicar"]);
        m.insert("ui/compose/cancel", ["Cancel", "取消", "キャンセル", "Cancelar"]);
        m.insert("ui/compose/reply_placeholder", ["Write a reply...", "写回复...", "返信を書く...", "Escribe una respuesta..."]);

        // Profile
        m.insert("ui/profile/followers", ["Followers", "粉丝", "フォロワー", "Seguidores"]);
        m.insert("ui/profile/following", ["Following", "关注", "フォロー中", "Siguiendo"]);
        m.insert("ui/profile/tweets", ["Tweets", "推文", "ツイート", "Tweets"]);
        m.insert("ui/profile/follow", ["Follow", "关注", "フォロー", "Seguir"]);
        m.insert("ui/profile/unfollow", ["Unfollow", "取消关注", "フォロー解除", "Dejar de seguir"]);
        m.insert("ui/profile/no_tweets", ["No tweets yet", "还没有推文", "ツイートはまだありません", "Aún no hay tweets"]);

        // Tweet detail
        m.insert("ui/tweet/reply", ["Reply", "回复", "返信", "Responder"]);
        m.insert("ui/tweet/no_replies", ["No replies yet", "还没有回复", "返信はまだありません", "Aún no hay respuestas"]);

        // Me / Settings
        m.insert("ui/me/title", ["Me", "我", "マイページ", "Yo"]);
        m.insert("ui/me/edit_profile", ["Edit Profile", "编辑资料", "プロフィール編集", "Editar perfil"]);
        m.insert("ui/me/change_password", ["Change Password", "修改密码", "パスワード変更", "Cambiar contraseña"]);
        m.insert("ui/me/sign_out", ["Sign Out", "退出登录", "サインアウト", "Cerrar sesión"]);
        m.insert("ui/me/admin_dashboard", ["Open Admin Dashboard", "打开管理面板", "管理ダッシュボードを開く", "Abrir panel de admin"]);
        m.insert("ui/me/settings", ["Settings", "设置", "設定", "Configuración"]);
        m.insert("ui/me/developer", ["Developer", "开发者", "開発者", "Desarrollador"]);

        // Edit profile
        m.insert("ui/edit/display_name", ["Display Name", "显示名称", "表示名", "Nombre"]);
        m.insert("ui/edit/bio", ["Bio", "简介", "自己紹介", "Biografía"]);
        m.insert("ui/edit/save", ["Save Changes", "保存修改", "変更を保存", "Guardar cambios"]);
        m.insert("ui/edit/saved", ["Saved!", "已保存！", "保存しました！", "¡Guardado!"]);

        // Change password
        m.insert("ui/password/current", ["Current Password", "当前密码", "現在のパスワード", "Contraseña actual"]);
        m.insert("ui/password/new", ["New Password", "新密码", "新しいパスワード", "Nueva contraseña"]);
        m.insert("ui/password/confirm", ["Confirm Password", "确认密码", "パスワード確認", "Confirmar contraseña"]);
        m.insert("ui/password/change", ["Change Password", "修改密码", "パスワード変更", "Cambiar contraseña"]);
        m.insert("ui/password/changed", ["Password changed!", "密码已修改！", "パスワードを変更しました！", "¡Contraseña cambiada!"]);

        // Search
        m.insert("ui/search/title", ["Search", "搜索", "検索", "Buscar"]);
        m.insert("ui/search/placeholder", ["Search users or tweets...", "搜索用户或推文...", "ユーザーやツイートを検索...", "Buscar usuarios o tweets..."]);
        m.insert("ui/search/users", ["Users", "用户", "ユーザー", "Usuarios"]);
        m.insert("ui/search/tweets_section", ["Tweets", "推文", "ツイート", "Tweets"]);
        m.insert("ui/search/no_results", ["No results", "没有结果", "結果なし", "Sin resultados"]);

        // Language
        m.insert("ui/me/language", ["Language", "语言", "言語", "Idioma"]);
        m.insert("ui/lang/current", ["English", "中文", "日本語", "Español"]);
        m.insert("ui/lang/code", ["en", "zh-CN", "ja", "es"]);

        // Inbox
        m.insert("ui/tab/inbox", ["Inbox", "站内信", "受信箱", "Bandeja"]);
        m.insert("ui/inbox/title", ["Inbox", "站内信", "受信箱", "Bandeja de entrada"]);
        m.insert("ui/inbox/empty", ["No messages", "没有消息", "メッセージはありません", "Sin mensajes"]);
        m.insert("ui/inbox/mark_read", ["Mark as read", "标记已读", "既読にする", "Marcar como leído"]);
        m.insert("ui/inbox/unread", ["Unread", "未读", "未読", "No leído"]);

        // Common
        m.insert("ui/common/loading", ["Loading...", "加载中...", "読み込み中...", "Cargando..."]);
        m.insert("ui/common/error", ["Something went wrong", "出了点问题", "エラーが発生しました", "Algo salió mal"]);
        m.insert("ui/common/retry", ["Retry", "重试", "再試行", "Reintentar"]);

        Self { data: m }
    }
}

impl I18nHandler for UiStrings {
    fn translate(&self, path: &str, _query: &QueryParams, locale: &str) -> String {
        let idx = locale_index(locale);
        self.data.get(path)
            .map(|t| t[idx].to_string())
            .unwrap_or_else(|| {
                // Fallback to English.
                self.data.get(path).map(|t| t[EN].to_string()).unwrap_or_else(|| path.to_string())
            })
    }
}

// ── Error Strings ──

struct ErrorStrings {
    data: HashMap<&'static str, [&'static str; 4]>,
}

impl ErrorStrings {
    fn new() -> Self {
        let mut m = HashMap::new();

        m.insert("error/auth/missing_token", ["Missing Authorization header", "缺少认证信息", "認証ヘッダーがありません", "Falta el encabezado de autorización"]);
        m.insert("error/auth/invalid_token", ["Invalid or expired token", "无效或过期的令牌", "無効または期限切れのトークン", "Token inválido o expirado"]);
        m.insert("error/auth/user_not_found", ["User not found", "用户不存在", "ユーザーが見つかりません", "Usuario no encontrado"]);
        m.insert("error/tweet/empty", ["Tweet cannot be empty", "推文不能为空", "ツイートは空にできません", "El tweet no puede estar vacío"]);
        m.insert("error/tweet/too_long", ["Tweet exceeds 280 characters", "推文超过280个字符", "ツイートは280文字を超えています", "El tweet supera los 280 caracteres"]);
        m.insert("error/profile/name_empty", ["Display name cannot be empty", "显示名称不能为空", "表示名は空にできません", "El nombre no puede estar vacío"]);
        m.insert("error/profile/not_found", ["User not found", "用户不存在", "ユーザーが見つかりません", "Usuario no encontrado"]);
        m.insert("error/tweet/not_found", ["Tweet not found", "推文不存在", "ツイートが見つかりません", "Tweet no encontrado"]);
        m.insert("error/password/too_short", ["Password must be at least 6 characters", "密码至少需要6个字符", "パスワードは6文字以上必要です", "La contraseña debe tener al menos 6 caracteres"]);
        m.insert("error/password/same", ["New password must be different", "新密码不能和旧密码相同", "新しいパスワードは異なる必要があります", "La nueva contraseña debe ser diferente"]);
        m.insert("error/upload/empty", ["File cannot be empty", "文件不能为空", "ファイルは空にできません", "El archivo no puede estar vacío"]);
        m.insert("error/upload/too_large", ["File exceeds 5MB limit", "文件超过5MB限制", "ファイルは5MBを超えています", "El archivo supera el límite de 5MB"]);

        Self { data: m }
    }
}

impl I18nHandler for ErrorStrings {
    fn translate(&self, path: &str, query: &QueryParams, locale: &str) -> String {
        let idx = locale_index(locale);
        let base = self.data.get(path)
            .map(|t| t[idx].to_string())
            .unwrap_or_else(|| path.to_string());

        // Substitute query params: "User not found" + ?username=alice → "User 'alice' not found"
        // Simple approach: if handler has a param, append it.
        if let Some(username) = query.get("username") {
            return base.replace("not found", &format!("'{}' not found", username))
                .replace("不存在", &format!("'{}'不存在", username))
                .replace("見つかりません", &format!("'{}'が見つかりません", username))
                .replace("no encontrado", &format!("'{}' no encontrado", username));
        }
        if let Some(max) = query.get("max") {
            return base.replace("280", max);
        }
        if let Some(min) = query.get("min") {
            return base.replace("6", min);
        }

        base
    }
}

// ── Format Strings (dynamic content with params) ──

struct FormatStrings;

impl I18nHandler for FormatStrings {
    fn translate(&self, path: &str, query: &QueryParams, locale: &str) -> String {
        let idx = locale_index(locale);
        match path {
            "format/like_count" => {
                let count = query.get("count").unwrap_or("0");
                match idx {
                    ZH => format!("{} 人赞了", count),
                    JA => format!("{}件のいいね", count),
                    ES => format!("{} me gusta", count),
                    _ => format!("{} likes", count),
                }
            }
            "format/reply_count" => {
                let count = query.get("count").unwrap_or("0");
                match idx {
                    ZH => format!("{} 条回复", count),
                    JA => format!("{}件の返信", count),
                    ES => format!("{} respuestas", count),
                    _ => format!("{} replies", count),
                }
            }
            "format/follower_count" => {
                let count = query.get("count").unwrap_or("0");
                match idx {
                    ZH => format!("{} 粉丝", count),
                    JA => format!("{}人のフォロワー", count),
                    ES => format!("{} seguidores", count),
                    _ => format!("{} followers", count),
                }
            }
            "format/tweet_count" => {
                let count = query.get("count").unwrap_or("0");
                match idx {
                    ZH => format!("{} 条推文", count),
                    JA => format!("{}件のツイート", count),
                    ES => format!("{} tweets", count),
                    _ => format!("{} tweets", count),
                }
            }
            "format/char_count" => {
                let current = query.get("current").unwrap_or("0");
                let max = query.get("max").unwrap_or("280");
                format!("{}/{}", current, max)
            }
            "format/greeting" => {
                let name = query.get("name").unwrap_or("");
                match idx {
                    ZH => format!("你好，{}", name),
                    JA => format!("こんにちは、{}", name),
                    ES => format!("Hola, {}", name),
                    _ => format!("Hello, {}", name),
                }
            }
            _ => path.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> I18nStore {
        let i18n = I18nStore::new("en");
        register_all(&i18n);
        i18n
    }

    #[test]
    fn english_default() {
        let i18n = setup();
        assert_eq!(i18n.get("ui/login/button"), "Sign In");
        assert_eq!(i18n.get("ui/tab/home"), "Home");
        assert_eq!(i18n.get("error/tweet/empty"), "Tweet cannot be empty");
    }

    #[test]
    fn chinese() {
        let i18n = setup();
        i18n.set_locale("zh-CN");
        assert_eq!(i18n.get("ui/login/button"), "登录");
        assert_eq!(i18n.get("ui/tab/home"), "首页");
        assert_eq!(i18n.get("ui/compose/placeholder"), "有什么新鲜事？");
        assert_eq!(i18n.get("error/tweet/empty"), "推文不能为空");
    }

    #[test]
    fn japanese() {
        let i18n = setup();
        i18n.set_locale("ja");
        assert_eq!(i18n.get("ui/login/button"), "サインイン");
        assert_eq!(i18n.get("ui/compose/placeholder"), "いまどうしてる？");
        assert_eq!(i18n.get("ui/me/sign_out"), "サインアウト");
    }

    #[test]
    fn spanish() {
        let i18n = setup();
        i18n.set_locale("es");
        assert_eq!(i18n.get("ui/login/button"), "Iniciar sesión");
        assert_eq!(i18n.get("ui/tab/search"), "Buscar");
        assert_eq!(i18n.get("ui/profile/follow"), "Seguir");
    }

    #[test]
    fn format_with_params() {
        let i18n = setup();
        assert_eq!(i18n.get("format/like_count?count=42"), "42 likes");
        i18n.set_locale("zh-CN");
        assert_eq!(i18n.get("format/like_count?count=42"), "42 人赞了");
        i18n.set_locale("ja");
        assert_eq!(i18n.get("format/like_count?count=42"), "42件のいいね");
        i18n.set_locale("es");
        assert_eq!(i18n.get("format/like_count?count=42"), "42 me gusta");
    }

    #[test]
    fn format_greeting() {
        let i18n = setup();
        assert_eq!(i18n.get("format/greeting?name=Alice"), "Hello, Alice");
        i18n.set_locale("zh-CN");
        assert_eq!(i18n.get("format/greeting?name=Alice"), "你好，Alice");
        i18n.set_locale("ja");
        assert_eq!(i18n.get("format/greeting?name=Alice"), "こんにちは、Alice");
    }

    #[test]
    fn error_with_username_param() {
        let i18n = setup();
        let text = i18n.get("error/auth/user_not_found?username=alice");
        assert!(text.contains("alice"), "got: {}", text);
    }

    #[test]
    fn unknown_key_returns_path() {
        let i18n = setup();
        assert_eq!(i18n.get("ui/nonexistent/key"), "ui/nonexistent/key");
    }

    #[test]
    fn locale_switch_all_languages() {
        let i18n = setup();
        let key = "ui/me/sign_out";
        
        i18n.set_locale("en");
        assert_eq!(i18n.get(key), "Sign Out");
        i18n.set_locale("zh-CN");
        assert_eq!(i18n.get(key), "退出登录");
        i18n.set_locale("ja");
        assert_eq!(i18n.get(key), "サインアウト");
        i18n.set_locale("es");
        assert_eq!(i18n.get(key), "Cerrar sesión");
    }

    #[test]
    fn char_count_format() {
        let i18n = setup();
        assert_eq!(i18n.get("format/char_count?current=120&max=280"), "120/280");
    }
}
