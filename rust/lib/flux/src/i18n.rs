//! I18nStore — synchronous path-based translation with Trie routing.
//!
//! Sits alongside Flux's async state store. UI reads translations
//! via `i18n.get("path?args")` — synchronous, zero allocation for
//! cached lookups.
//!
//! Handlers are registered per path pattern (Trie matching with +/#):
//!   i18n.handle("error/#", error_handler);
//!   i18n.handle("button/sign_in", static_handler);
//!
//! # Example
//!
//! ```ignore
//! let i18n = I18nStore::new("en");
//! i18n.handle("button/sign_in", Arc::new(StaticI18n(hashmap!{
//!     "en" => "Sign In", "zh-CN" => "登录",
//! })));
//! assert_eq!(i18n.get("button/sign_in"), "Sign In");
//! i18n.set_locale("zh-CN");
//! assert_eq!(i18n.get("button/sign_in"), "登录");
//! ```

use std::sync::{Arc, RwLock};

use crate::trie::Trie;

// ── QueryParams ──

/// Parsed URL query string: `count=3&name=alice`.
#[derive(Debug, Clone)]
pub struct QueryParams(Vec<(String, String)>);

impl QueryParams {
    /// Parse a query string (without the leading `?`).
    pub fn parse(query: &str) -> Self {
        Self(
            query
                .split('&')
                .filter(|s| !s.is_empty())
                .filter_map(|pair| {
                    let (k, v) = pair.split_once('=')?;
                    Some((k.to_string(), v.to_string()))
                })
                .collect(),
        )
    }

    /// Empty params.
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ── I18nHandler trait ──

/// A translation handler — like Go's `http.Handler`.
///
/// Registered per path pattern. Receives the matched path,
/// query params, and current locale. Returns a translated string.
pub trait I18nHandler: Send + Sync + 'static {
    fn translate(&self, path: &str, query: &QueryParams, locale: &str) -> String;
}

/// Convenience: closures implement I18nHandler.
impl<F> I18nHandler for F
where
    F: Fn(&str, &QueryParams, &str) -> String + Send + Sync + 'static,
{
    fn translate(&self, path: &str, query: &QueryParams, locale: &str) -> String {
        (self)(path, query, locale)
    }
}

// ── I18nStore ──

/// Synchronous translation store with Trie-based handler routing.
pub struct I18nStore {
    trie: Trie<Arc<dyn I18nHandler>>,
    locale: RwLock<String>,
}

impl I18nStore {
    /// Create a new I18nStore with the given default locale.
    pub fn new(locale: &str) -> Self {
        Self {
            trie: Trie::new(),
            locale: RwLock::new(locale.to_string()),
        }
    }

    /// Register a translation handler for a path pattern.
    ///
    /// Supports Trie wildcards:
    /// - `"button/sign_in"` — exact match
    /// - `"error/+"` — single-level wildcard
    /// - `"error/#"` — multi-level wildcard
    pub fn handle(&self, pattern: &str, handler: Arc<dyn I18nHandler>) {
        self.trie.insert(pattern, handler);
    }

    /// Get a translated string for a path (with optional query params).
    ///
    /// URL format: `"path"` or `"path?key=value&key2=value2"`.
    ///
    /// Matches the path against registered handlers via Trie.
    /// First matching handler's result is returned.
    /// If no handler matches, returns the path as-is.
    pub fn get(&self, url: &str) -> String {
        let (path, query) = split_url(url);
        let params = if query.is_empty() {
            QueryParams::empty()
        } else {
            QueryParams::parse(query)
        };
        let locale = self.locale.read().unwrap().clone();

        let handlers = self.trie.match_topic(path);
        if let Some(handler) = handlers.first() {
            handler.translate(path, &params, &locale)
        } else {
            path.to_string()
        }
    }

    /// Set the current locale.
    pub fn set_locale(&self, locale: &str) {
        *self.locale.write().unwrap() = locale.to_string();
    }

    /// Get the current locale.
    pub fn locale(&self) -> String {
        self.locale.read().unwrap().clone()
    }
}

/// Split `"path?query"` into `("path", "query")`.
fn split_url(url: &str) -> (&str, &str) {
    match url.find('?') {
        Some(idx) => (&url[..idx], &url[idx + 1..]),
        None => (url, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ── QueryParams ──

    #[test]
    fn parse_query_params() {
        let q = QueryParams::parse("count=3&name=alice");
        assert_eq!(q.get("count"), Some("3"));
        assert_eq!(q.get("name"), Some("alice"));
        assert_eq!(q.get("missing"), None);
    }

    #[test]
    fn parse_empty_query() {
        let q = QueryParams::parse("");
        assert!(q.is_empty());
    }

    #[test]
    fn parse_single_param() {
        let q = QueryParams::parse("locale=zh-CN");
        assert_eq!(q.get("locale"), Some("zh-CN"));
    }

    // ── split_url ──

    #[test]
    fn split_with_query() {
        assert_eq!(split_url("tweet/like_count?count=3"), ("tweet/like_count", "count=3"));
    }

    #[test]
    fn split_without_query() {
        assert_eq!(split_url("button/sign_in"), ("button/sign_in", ""));
    }

    // ── I18nStore basic ──

    #[test]
    fn exact_match() {
        let i18n = I18nStore::new("en");
        i18n.handle("button/sign_in", Arc::new(|_: &str, _: &QueryParams, locale: &str| {
            match locale {
                "zh-CN" => "登录".into(),
                _ => "Sign In".into(),
            }
        }));

        assert_eq!(i18n.get("button/sign_in"), "Sign In");
        i18n.set_locale("zh-CN");
        assert_eq!(i18n.get("button/sign_in"), "登录");
    }

    #[test]
    fn no_match_returns_path() {
        let i18n = I18nStore::new("en");
        assert_eq!(i18n.get("unknown/key"), "unknown/key");
    }

    // ── Query params ──

    #[test]
    fn handler_receives_query_params() {
        let i18n = I18nStore::new("en");
        i18n.handle("tweet/like_count", Arc::new(|_: &str, q: &QueryParams, locale: &str| {
            let count = q.get("count").unwrap_or("0");
            match locale {
                "zh-CN" => format!("{} 人赞了", count),
                _ => format!("{} likes", count),
            }
        }));

        assert_eq!(i18n.get("tweet/like_count?count=3"), "3 likes");
        i18n.set_locale("zh-CN");
        assert_eq!(i18n.get("tweet/like_count?count=3"), "3 人赞了");
    }

    #[test]
    fn multiple_query_params() {
        let i18n = I18nStore::new("en");
        i18n.handle("notification/follow", Arc::new(|_: &str, q: &QueryParams, _: &str| {
            let follower = q.get("follower").unwrap_or("?");
            let followee = q.get("followee").unwrap_or("?");
            format!("{} followed {}", follower, followee)
        }));

        assert_eq!(
            i18n.get("notification/follow?follower=alice&followee=bob"),
            "alice followed bob"
        );
    }

    // ── Wildcard handlers (module-level routing) ──

    #[test]
    fn wildcard_handler() {
        let i18n = I18nStore::new("en");

        // Module handler for all error/* paths.
        i18n.handle("error/#", Arc::new(|path: &str, _: &QueryParams, locale: &str| {
            match (path, locale) {
                ("error/tweet/empty", "zh-CN") => "推文不能为空".into(),
                ("error/tweet/empty", _) => "Tweet cannot be empty".into(),
                ("error/tweet/too_long", _) => "Too long".into(),
                _ => format!("[{}]", path),
            }
        }));

        assert_eq!(i18n.get("error/tweet/empty"), "Tweet cannot be empty");
        assert_eq!(i18n.get("error/tweet/too_long"), "Too long");
        assert_eq!(i18n.get("error/unknown"), "[error/unknown]");

        i18n.set_locale("zh-CN");
        assert_eq!(i18n.get("error/tweet/empty"), "推文不能为空");
    }

    #[test]
    fn multiple_module_handlers() {
        let i18n = I18nStore::new("en");

        i18n.handle("auth/#", Arc::new(|path: &str, _: &QueryParams, _: &str| {
            match path {
                "auth/login" => "Sign In".into(),
                "auth/logout" => "Sign Out".into(),
                _ => path.into(),
            }
        }));

        i18n.handle("tweet/#", Arc::new(|path: &str, _: &QueryParams, _: &str| {
            match path {
                "tweet/compose" => "What's happening?".into(),
                _ => path.into(),
            }
        }));

        assert_eq!(i18n.get("auth/login"), "Sign In");
        assert_eq!(i18n.get("tweet/compose"), "What's happening?");
        assert_eq!(i18n.get("other/key"), "other/key"); // no match → path
    }

    // ── I18nHandler trait with struct ──

    struct StaticI18n {
        translations: HashMap<String, HashMap<String, String>>, // locale → {key → text}
    }

    impl I18nHandler for StaticI18n {
        fn translate(&self, path: &str, _query: &QueryParams, locale: &str) -> String {
            self.translations
                .get(locale)
                .and_then(|m| m.get(path))
                .cloned()
                .unwrap_or_else(|| {
                    // Fallback to "en".
                    self.translations
                        .get("en")
                        .and_then(|m| m.get(path))
                        .cloned()
                        .unwrap_or_else(|| path.to_string())
                })
        }
    }

    #[test]
    fn struct_handler() {
        let mut en = HashMap::new();
        en.insert("ui/home".into(), "Home".into());
        en.insert("ui/search".into(), "Search".into());

        let mut zh = HashMap::new();
        zh.insert("ui/home".into(), "首页".into());
        zh.insert("ui/search".into(), "搜索".into());

        let mut translations = HashMap::new();
        translations.insert("en".into(), en);
        translations.insert("zh-CN".into(), zh);

        let handler = Arc::new(StaticI18n { translations });

        let i18n = I18nStore::new("en");
        i18n.handle("ui/#", handler);

        assert_eq!(i18n.get("ui/home"), "Home");
        assert_eq!(i18n.get("ui/search"), "Search");

        i18n.set_locale("zh-CN");
        assert_eq!(i18n.get("ui/home"), "首页");
        assert_eq!(i18n.get("ui/search"), "搜索");
    }

    // ── Locale switching ──

    #[test]
    fn locale_default() {
        let i18n = I18nStore::new("en");
        assert_eq!(i18n.locale(), "en");
    }

    #[test]
    fn locale_switch() {
        let i18n = I18nStore::new("en");
        i18n.set_locale("ja");
        assert_eq!(i18n.locale(), "ja");
    }

    // ── Thread safety ──

    #[test]
    fn concurrent_get() {
        use std::thread;

        let i18n = Arc::new(I18nStore::new("en"));
        i18n.handle("test", Arc::new(|_: &str, _: &QueryParams, _: &str| "ok".into()));

        let mut handles = vec![];
        for _ in 0..10 {
            let i18n = Arc::clone(&i18n);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    assert_eq!(i18n.get("test"), "ok");
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }
}
