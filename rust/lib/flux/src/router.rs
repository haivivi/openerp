use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::store::StateStore;
use crate::trie::Trie;

/// A boxed, `Send`-able future returned by request handlers.
pub type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Type-erased handler function stored in the router.
///
/// Takes owned values so the returned future can be `'static`:
/// - `String` — the matched request path
/// - `Arc<dyn Any + Send + Sync>` — type-erased request payload
/// - `Arc<StateStore>` — the state store for reading/writing state
type ErasedHandler =
    Arc<dyn Fn(String, Arc<dyn Any + Send + Sync>, Arc<StateStore>) -> BoxFuture + Send + Sync>;

/// Request router — maps path patterns to async handlers via Trie matching.
///
/// Handlers are registered with `on(pattern, handler)` and dispatched
/// with `dispatch(path, payload, store)`. Multiple handlers can match
/// a single path (via wildcards), and all are called sequentially.
///
/// # Examples
///
/// ```ignore
/// let router = Router::new();
/// router.on("auth/login", |path, payload, store| async move {
///     let req = payload.downcast_ref::<LoginRequest>().unwrap();
///     store.set("auth/state", AuthState { phase: "authenticated" });
/// });
///
/// router.dispatch("auth/login", Arc::new(req), store).await;
/// ```
pub struct Router {
    trie: Trie<ErasedHandler>,
}

impl Router {
    /// Create a new empty router.
    pub fn new() -> Self {
        Self {
            trie: Trie::new(),
        }
    }

    /// Register an async handler for a path pattern.
    ///
    /// Pattern supports MQTT-style wildcards:
    /// - `"auth/login"` — exact match
    /// - `"auth/+"` — matches any single level under `auth/`
    /// - `"auth/#"` — matches any levels under `auth/`
    /// - `"#"` — matches everything
    pub fn on<F, Fut>(&self, pattern: &str, handler: F)
    where
        F: Fn(String, Arc<dyn Any + Send + Sync>, Arc<StateStore>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handler: ErasedHandler = Arc::new(
            move |path: String,
                  payload: Arc<dyn Any + Send + Sync>,
                  store: Arc<StateStore>|
                  -> BoxFuture { Box::pin(handler(path, payload, store)) },
        );
        self.trie.insert(pattern, handler);
    }

    /// Dispatch a request to all matching handlers.
    ///
    /// Handlers are called sequentially in the order they match.
    /// If no handler matches, this is a no-op (no error).
    pub async fn dispatch(
        &self,
        path: &str,
        payload: Arc<dyn Any + Send + Sync>,
        store: Arc<StateStore>,
    ) {
        let handlers = self.trie.match_topic(path);
        for handler in handlers {
            handler(path.to_string(), Arc::clone(&payload), Arc::clone(&store)).await;
        }
    }

    /// Check if any handler is registered for the exact pattern.
    pub fn has_handler(&self, pattern: &str) -> bool {
        self.trie.has_pattern(pattern)
    }

    /// Check if any handler would match the given topic path.
    pub fn matches(&self, path: &str) -> bool {
        !self.trie.match_topic(path).is_empty()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Helper: create a store wrapped in Arc.
    fn test_store() -> Arc<StateStore> {
        Arc::new(StateStore::new())
    }

    // ========================================================================
    // Basic dispatch
    // ========================================================================

    #[tokio::test]
    async fn dispatch_exact_match() {
        let router = Router::new();
        let called = Arc::new(AtomicU64::new(0));
        let called_c = called.clone();

        router.on("auth/login", move |_path, _payload, _store| {
            let called = called_c.clone();
            async move {
                called.fetch_add(1, Ordering::Relaxed);
            }
        });

        let store = test_store();
        router
            .dispatch("auth/login", Arc::new(()), store)
            .await;

        assert_eq!(called.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn dispatch_no_match_is_noop() {
        let router = Router::new();
        let called = Arc::new(AtomicU64::new(0));
        let called_c = called.clone();

        router.on("auth/login", move |_path, _payload, _store| {
            let called = called_c.clone();
            async move {
                called.fetch_add(1, Ordering::Relaxed);
            }
        });

        let store = test_store();
        router
            .dispatch("auth/logout", Arc::new(()), store)
            .await;

        assert_eq!(called.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn dispatch_receives_correct_path() {
        let router = Router::new();
        let received_path = Arc::new(std::sync::RwLock::new(String::new()));
        let rp = received_path.clone();

        router.on("auth/login", move |path, _payload, _store| {
            let rp = rp.clone();
            async move {
                *rp.write().unwrap() = path;
            }
        });

        let store = test_store();
        router
            .dispatch("auth/login", Arc::new(()), store)
            .await;

        assert_eq!(*received_path.read().unwrap(), "auth/login");
    }

    // ========================================================================
    // Typed payload
    // ========================================================================

    #[tokio::test]
    async fn handler_receives_typed_payload() {
        #[allow(dead_code)]
        struct LoginRequest {
            phone: String,
            password: String,
        }

        let router = Router::new();
        let received_phone = Arc::new(std::sync::RwLock::new(String::new()));
        let rp = received_phone.clone();

        router.on("auth/login", move |_path, payload, _store| {
            let rp = rp.clone();
            async move {
                let req = payload.downcast_ref::<LoginRequest>().unwrap();
                *rp.write().unwrap() = req.phone.clone();
            }
        });

        let payload = LoginRequest {
            phone: "13800138000".to_string(),
            password: "secret".to_string(),
        };
        let store = test_store();
        router
            .dispatch("auth/login", Arc::new(payload), store)
            .await;

        assert_eq!(*received_phone.read().unwrap(), "13800138000");
    }

    #[tokio::test]
    async fn handler_downcasts_wrong_type_safely() {
        let router = Router::new();
        let got_none = Arc::new(AtomicU64::new(0));
        let gn = got_none.clone();

        router.on("test", move |_path, payload, _store| {
            let gn = gn.clone();
            async move {
                // Payload is u32, try to downcast to String.
                if payload.downcast_ref::<String>().is_none() {
                    gn.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        let store = test_store();
        router
            .dispatch("test", Arc::new(42u32), store)
            .await;

        assert_eq!(got_none.load(Ordering::Relaxed), 1);
    }

    // ========================================================================
    // Handler updates state
    // ========================================================================

    #[tokio::test]
    async fn handler_updates_store() {
        let router = Router::new();

        router.on("auth/login", |_path, _payload, store: Arc<StateStore>| async move {
            store.set("auth/state", "authenticated".to_string());
        });

        let store = test_store();
        router
            .dispatch("auth/login", Arc::new(()), Arc::clone(&store))
            .await;

        let state = store.get("auth/state").unwrap();
        assert_eq!(
            state.downcast_ref::<String>(),
            Some(&"authenticated".to_string())
        );
    }

    #[tokio::test]
    async fn handler_reads_and_updates_store() {
        let router = Router::new();

        // First set initial state.
        let store = test_store();
        store.set("counter", 0u32);

        router.on("increment", |_path, _payload, store: Arc<StateStore>| async move {
            let current = store
                .get("counter")
                .and_then(|v| v.downcast_ref::<u32>().copied())
                .unwrap_or(0);
            store.set("counter", current + 1);
        });

        router
            .dispatch("increment", Arc::new(()), Arc::clone(&store))
            .await;
        router
            .dispatch("increment", Arc::new(()), Arc::clone(&store))
            .await;
        router
            .dispatch("increment", Arc::new(()), Arc::clone(&store))
            .await;

        let v = store.get("counter").unwrap();
        assert_eq!(v.downcast_ref::<u32>(), Some(&3));
    }

    // ========================================================================
    // Wildcard dispatch
    // ========================================================================

    #[tokio::test]
    async fn dispatch_single_wildcard() {
        let router = Router::new();
        let called = Arc::new(AtomicU64::new(0));
        let called_c = called.clone();

        router.on("auth/+", move |_path, _payload, _store| {
            let called = called_c.clone();
            async move {
                called.fetch_add(1, Ordering::Relaxed);
            }
        });

        let store = test_store();
        router
            .dispatch("auth/login", Arc::new(()), Arc::clone(&store))
            .await;
        router
            .dispatch("auth/logout", Arc::new(()), Arc::clone(&store))
            .await;

        assert_eq!(called.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn dispatch_multi_wildcard() {
        let router = Router::new();
        let called = Arc::new(AtomicU64::new(0));
        let called_c = called.clone();

        router.on("auth/#", move |_path, _payload, _store| {
            let called = called_c.clone();
            async move {
                called.fetch_add(1, Ordering::Relaxed);
            }
        });

        let store = test_store();
        router
            .dispatch("auth/login", Arc::new(()), Arc::clone(&store))
            .await;
        router
            .dispatch("auth/deep/nested", Arc::new(()), Arc::clone(&store))
            .await;
        router
            .dispatch("home/something", Arc::new(()), Arc::clone(&store))
            .await; // no match

        assert_eq!(called.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn dispatch_root_wildcard() {
        let router = Router::new();
        let count = Arc::new(AtomicU64::new(0));
        let count_c = count.clone();

        router.on("#", move |_path, _payload, _store| {
            let count = count_c.clone();
            async move {
                count.fetch_add(1, Ordering::Relaxed);
            }
        });

        let store = test_store();
        router
            .dispatch("anything", Arc::new(()), Arc::clone(&store))
            .await;
        router
            .dispatch("a/b/c", Arc::new(()), Arc::clone(&store))
            .await;

        assert_eq!(count.load(Ordering::Relaxed), 2);
    }

    // ========================================================================
    // Multiple handlers
    // ========================================================================

    #[tokio::test]
    async fn multiple_handlers_all_called() {
        let router = Router::new();
        let exact = Arc::new(AtomicU64::new(0));
        let wild = Arc::new(AtomicU64::new(0));
        let all = Arc::new(AtomicU64::new(0));
        let e = exact.clone();
        let w = wild.clone();
        let a = all.clone();

        router.on("auth/login", move |_, _, _| {
            let e = e.clone();
            async move { e.fetch_add(1, Ordering::Relaxed); }
        });
        router.on("auth/+", move |_, _, _| {
            let w = w.clone();
            async move { w.fetch_add(1, Ordering::Relaxed); }
        });
        router.on("#", move |_, _, _| {
            let a = a.clone();
            async move { a.fetch_add(1, Ordering::Relaxed); }
        });

        let store = test_store();
        router
            .dispatch("auth/login", Arc::new(()), store)
            .await;

        assert_eq!(exact.load(Ordering::Relaxed), 1);
        assert_eq!(wild.load(Ordering::Relaxed), 1);
        assert_eq!(all.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn multiple_handlers_same_pattern() {
        let router = Router::new();
        let count = Arc::new(AtomicU64::new(0));
        let c1 = count.clone();
        let c2 = count.clone();

        router.on("test", move |_, _, _| {
            let c = c1.clone();
            async move { c.fetch_add(1, Ordering::Relaxed); }
        });
        router.on("test", move |_, _, _| {
            let c = c2.clone();
            async move { c.fetch_add(1, Ordering::Relaxed); }
        });

        let store = test_store();
        router.dispatch("test", Arc::new(()), store).await;

        assert_eq!(count.load(Ordering::Relaxed), 2);
    }

    // ========================================================================
    // has_handler / matches
    // ========================================================================

    #[test]
    fn has_handler_exact() {
        let router = Router::new();
        router.on("auth/login", |_, _, _| async {});

        assert!(router.has_handler("auth/login"));
        assert!(!router.has_handler("auth/logout"));
    }

    #[test]
    fn has_handler_wildcard() {
        let router = Router::new();
        router.on("auth/+", |_, _, _| async {});

        assert!(router.has_handler("auth/+"));
        assert!(!router.has_handler("auth/login")); // has_handler checks exact pattern
    }

    #[test]
    fn matches_with_wildcards() {
        let router = Router::new();
        router.on("auth/+", |_, _, _| async {});

        assert!(router.matches("auth/login")); // matches via wildcard
        assert!(router.matches("auth/logout"));
        assert!(!router.matches("home/devices"));
    }

    #[test]
    fn matches_empty_router() {
        let router = Router::new();
        assert!(!router.matches("anything"));
    }

    // ========================================================================
    // Default trait
    // ========================================================================

    #[test]
    fn default_creates_empty_router() {
        let router = Router::default();
        assert!(!router.matches("anything"));
    }

    // ========================================================================
    // Handler sequencing
    // ========================================================================

    #[tokio::test]
    async fn handlers_execute_sequentially() {
        let router = Router::new();
        let order = Arc::new(std::sync::Mutex::new(Vec::<u32>::new()));
        let o1 = order.clone();
        let o2 = order.clone();

        router.on("test", move |_, _, _| {
            let o = o1.clone();
            async move {
                o.lock().unwrap().push(1);
            }
        });
        router.on("test", move |_, _, _| {
            let o = o2.clone();
            async move {
                o.lock().unwrap().push(2);
            }
        });

        let store = test_store();
        router.dispatch("test", Arc::new(()), store).await;

        let order = order.lock().unwrap();
        assert_eq!(*order, vec![1, 2]);
    }

    // ========================================================================
    // Handler with unit payload
    // ========================================================================

    #[tokio::test]
    async fn unit_payload() {
        let router = Router::new();
        let called = Arc::new(AtomicU64::new(0));
        let called_c = called.clone();

        router.on("ping", move |_, payload, _| {
            let called = called_c.clone();
            async move {
                assert!(payload.downcast_ref::<()>().is_some());
                called.fetch_add(1, Ordering::Relaxed);
            }
        });

        let store = test_store();
        router.dispatch("ping", Arc::new(()), store).await;
        assert_eq!(called.load(Ordering::Relaxed), 1);
    }
}
