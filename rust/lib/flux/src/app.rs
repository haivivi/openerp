use std::any::Any;
use std::future::Future;
use std::sync::Arc;

use crate::router::Router;
use crate::store::StateStore;
use crate::value::{StateValue, SubscriptionId};

/// Flux — the cross-platform state engine.
///
/// Three primitives, all path-based:
/// - `get(path)` — read state at a path (Arc, zero-copy)
/// - `emit(path, payload)` — send a request, Trie-routed to handler(s)
/// - `subscribe(pattern)` — subscribe to state changes, Trie-matched notifications
///
/// # Examples
///
/// ```ignore
/// let flux = Flux::new();
///
/// // Register a handler.
/// flux.on("auth/login", |path, payload, store| async move {
///     store.set("auth/state", "authenticated".to_string());
/// });
///
/// // Subscribe to state changes.
/// flux.subscribe("auth/#", |path, value| {
///     println!("{} changed", path);
/// });
///
/// // Emit a request.
/// flux.emit("auth/login", LoginRequest { .. }).await;
///
/// // Read state.
/// let state = flux.get("auth/state").unwrap();
/// ```
pub struct Flux {
    store: Arc<StateStore>,
    router: Router,
}

impl Flux {
    /// Create a new Flux instance with empty state and no handlers.
    pub fn new() -> Self {
        Self {
            store: Arc::new(StateStore::new()),
            router: Router::new(),
        }
    }

    // ====================================================================
    // State — read
    // ====================================================================

    /// Read the state value at a path.
    ///
    /// Returns `None` if no value is set. The returned `StateValue` is an
    /// Arc clone (cheap, no data copy). Caller can downcast:
    ///
    /// ```ignore
    /// let v = flux.get("auth/state")?;
    /// let auth = v.downcast_ref::<AuthState>()?;
    /// ```
    pub fn get(&self, path: &str) -> Option<StateValue> {
        self.store.get(path)
    }

    /// Scan all state entries under a prefix path.
    ///
    /// Returns entries whose path starts with `{prefix}/`.
    /// Does NOT include the exact `prefix` path itself.
    /// Results are ordered by path.
    pub fn scan(&self, prefix: &str) -> Vec<(String, StateValue)> {
        self.store.scan(prefix)
    }

    /// Check if a state value exists at the given path.
    pub fn contains(&self, path: &str) -> bool {
        self.store.contains(path)
    }

    /// Get the total number of state entries.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the state store is empty.
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Get a snapshot of all state entries.
    pub fn snapshot(&self) -> Vec<(String, StateValue)> {
        self.store.snapshot()
    }

    // ====================================================================
    // Requests — emit
    // ====================================================================

    /// Emit a request and wait for handler(s) to complete.
    ///
    /// The payload is wrapped in `Arc` and routed to all handlers
    /// matching the path via Trie pattern matching. Handlers execute
    /// sequentially.
    ///
    /// If no handler matches, this is a silent no-op.
    pub async fn emit<T: Any + Send + Sync>(&self, path: &str, payload: T) {
        self.router
            .dispatch(path, Arc::new(payload), Arc::clone(&self.store))
            .await;
    }

    /// Emit a request with a pre-built Arc payload.
    pub async fn emit_arc(&self, path: &str, payload: Arc<dyn Any + Send + Sync>) {
        self.router
            .dispatch(path, payload, Arc::clone(&self.store))
            .await;
    }

    // ====================================================================
    // Requests — register handlers
    // ====================================================================

    /// Register an async request handler for a path pattern.
    ///
    /// The handler receives:
    /// - `path: String` — the matched request path
    /// - `payload: Arc<dyn Any + Send + Sync>` — type-erased payload (downcast inside)
    /// - `store: Arc<StateStore>` — state store for reading/writing state
    ///
    /// Pattern supports MQTT-style wildcards (`+`, `#`).
    pub fn on<F, Fut>(&self, pattern: &str, handler: F)
    where
        F: Fn(String, Arc<dyn Any + Send + Sync>, Arc<StateStore>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.router.on(pattern, handler);
    }

    /// Check if any handler would match the given path.
    pub fn has_handler(&self, path: &str) -> bool {
        self.router.matches(path)
    }

    // ====================================================================
    // Subscriptions — observe state changes
    // ====================================================================

    /// Subscribe to state changes matching a Trie pattern.
    ///
    /// The handler is called synchronously on the thread that calls `set`.
    /// Pattern supports MQTT-style wildcards (`+`, `#`).
    ///
    /// Returns a `SubscriptionId` for unsubscribing.
    pub fn subscribe<F>(&self, pattern: &str, handler: F) -> SubscriptionId
    where
        F: Fn(&str, &StateValue) + Send + Sync + 'static,
    {
        self.store.subscribe(pattern, handler)
    }

    /// Unsubscribe a handler by its ID and the pattern it was registered with.
    pub fn unsubscribe(&self, pattern: &str, id: SubscriptionId) {
        self.store.unsubscribe(pattern, id);
    }

    // ====================================================================
    // Advanced
    // ====================================================================

    /// Get a reference to the underlying StateStore.
    ///
    /// Useful for handlers that need direct store access, or for testing.
    pub fn store(&self) -> &Arc<StateStore> {
        &self.store
    }
}

impl Default for Flux {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // ========================================================================
    // Construction
    // ========================================================================

    #[test]
    fn new_creates_empty_flux() {
        let flux = Flux::new();
        assert!(flux.is_empty());
        assert_eq!(flux.len(), 0);
        assert!(flux.get("anything").is_none());
    }

    #[test]
    fn default_creates_empty_flux() {
        let flux = Flux::default();
        assert!(flux.is_empty());
    }

    // ========================================================================
    // State: get / contains / len
    // ========================================================================

    #[test]
    fn get_after_store_set() {
        let flux = Flux::new();
        flux.store().set("counter", 42u32);

        let v = flux.get("counter").unwrap();
        assert_eq!(v.downcast_ref::<u32>(), Some(&42));
    }

    #[test]
    fn contains_after_set() {
        let flux = Flux::new();
        flux.store().set("auth/state", 1u32);

        assert!(flux.contains("auth/state"));
        assert!(!flux.contains("auth/terms"));
    }

    #[test]
    fn len_tracks_entries() {
        let flux = Flux::new();
        assert_eq!(flux.len(), 0);

        flux.store().set("a", 1u32);
        flux.store().set("b", 2u32);
        assert_eq!(flux.len(), 2);
    }

    // ========================================================================
    // State: scan
    // ========================================================================

    #[test]
    fn scan_children() {
        let flux = Flux::new();
        flux.store().set("items/1", "a".to_string());
        flux.store().set("items/2", "b".to_string());
        flux.store().set("items/3", "c".to_string());
        flux.store().set("other", "x".to_string());

        let results = flux.scan("items");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn scan_empty() {
        let flux = Flux::new();
        assert!(flux.scan("anything").is_empty());
    }

    // ========================================================================
    // State: snapshot
    // ========================================================================

    #[test]
    fn snapshot_returns_all_entries() {
        let flux = Flux::new();
        flux.store().set("a", 1u32);
        flux.store().set("b", 2u32);

        let snap = flux.snapshot();
        assert_eq!(snap.len(), 2);
    }

    // ========================================================================
    // Emit + Handler: basic flow
    // ========================================================================

    #[tokio::test]
    async fn emit_routes_to_handler() {
        let flux = Flux::new();
        let called = Arc::new(AtomicU64::new(0));
        let called_c = called.clone();

        flux.on("ping", move |_, _, _| {
            let c = called_c.clone();
            async move {
                c.fetch_add(1, Ordering::Relaxed);
            }
        });

        flux.emit("ping", ()).await;
        assert_eq!(called.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn emit_no_handler_is_silent() {
        let flux = Flux::new();
        // Should not panic.
        flux.emit("nonexistent", ()).await;
    }

    #[tokio::test]
    async fn emit_typed_payload() {
        #[derive(Debug)]
        struct LoginReq {
            phone: String,
        }

        let flux = Flux::new();
        let received = Arc::new(std::sync::RwLock::new(String::new()));
        let r = received.clone();

        flux.on("auth/login", move |_, payload, _| {
            let r = r.clone();
            async move {
                let req = payload.downcast_ref::<LoginReq>().unwrap();
                *r.write().unwrap() = req.phone.clone();
            }
        });

        flux.emit(
            "auth/login",
            LoginReq {
                phone: "13800138000".into(),
            },
        )
        .await;

        assert_eq!(*received.read().unwrap(), "13800138000");
    }

    // ========================================================================
    // Emit + Handler: state updates
    // ========================================================================

    #[tokio::test]
    async fn handler_sets_state() {
        let flux = Flux::new();

        flux.on("auth/login", |_, _, store: Arc<StateStore>| async move {
            store.set("auth/state", "authenticated".to_string());
        });

        flux.emit("auth/login", ()).await;

        let v = flux.get("auth/state").unwrap();
        assert_eq!(
            v.downcast_ref::<String>(),
            Some(&"authenticated".to_string())
        );
    }

    #[tokio::test]
    async fn handler_sets_multiple_states() {
        let flux = Flux::new();

        flux.on("app/initialize", |_, _, store: Arc<StateStore>| async move {
            store.set("auth/state", "unauthenticated".to_string());
            store.set("auth/terms", false);
            store.set("app/route", "/onboarding".to_string());
        });

        flux.emit("app/initialize", ()).await;

        assert_eq!(
            flux.get("auth/state")
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "unauthenticated"
        );
        assert_eq!(
            flux.get("auth/terms")
                .unwrap()
                .downcast_ref::<bool>()
                .unwrap(),
            &false
        );
        assert_eq!(
            flux.get("app/route")
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "/onboarding"
        );
    }

    #[tokio::test]
    async fn handler_reads_and_updates_state() {
        let flux = Flux::new();
        flux.store().set("counter", 0u32);

        flux.on("increment", |_, _, store: Arc<StateStore>| async move {
            let current = store
                .get("counter")
                .and_then(|v| v.downcast_ref::<u32>().copied())
                .unwrap_or(0);
            store.set("counter", current + 1);
        });

        flux.emit("increment", ()).await;
        flux.emit("increment", ()).await;
        flux.emit("increment", ()).await;

        assert_eq!(
            flux.get("counter").unwrap().downcast_ref::<u32>(),
            Some(&3)
        );
    }

    // ========================================================================
    // Subscribe: notifications from emit
    // ========================================================================

    #[tokio::test]
    async fn subscribe_notified_by_handler_set() {
        let flux = Flux::new();
        let notified = Arc::new(AtomicU64::new(0));
        let n = notified.clone();

        flux.subscribe("auth/state", move |_path, _value| {
            n.fetch_add(1, Ordering::Relaxed);
        });

        flux.on("auth/login", |_, _, store: Arc<StateStore>| async move {
            store.set("auth/state", "authenticated".to_string());
        });

        flux.emit("auth/login", ()).await;
        assert_eq!(notified.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn subscribe_wildcard_catches_handler_updates() {
        let flux = Flux::new();
        let paths_changed = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let pc = paths_changed.clone();

        flux.subscribe("#", move |path, _value| {
            pc.lock().unwrap().push(path.to_string());
        });

        flux.on("app/initialize", |_, _, store: Arc<StateStore>| async move {
            store.set("auth/state", "unauthenticated".to_string());
            store.set("auth/terms", false);
            store.set("app/route", "/onboarding".to_string());
        });

        flux.emit("app/initialize", ()).await;

        let paths = paths_changed.lock().unwrap();
        assert_eq!(paths.len(), 3);
        assert!(paths.contains(&"auth/state".to_string()));
        assert!(paths.contains(&"auth/terms".to_string()));
        assert!(paths.contains(&"app/route".to_string()));
    }

    #[tokio::test]
    async fn subscribe_receives_correct_value() {
        let flux = Flux::new();
        let received = Arc::new(std::sync::RwLock::new(None::<String>));
        let r = received.clone();

        flux.subscribe("auth/state", move |_path, value| {
            let s = value.downcast_ref::<String>().unwrap().clone();
            *r.write().unwrap() = Some(s);
        });

        flux.on("auth/login", |_, _, store: Arc<StateStore>| async move {
            store.set("auth/state", "authenticated".to_string());
        });

        flux.emit("auth/login", ()).await;
        assert_eq!(*received.read().unwrap(), Some("authenticated".to_string()));
    }

    // ========================================================================
    // Unsubscribe
    // ========================================================================

    #[tokio::test]
    async fn unsubscribe_stops_notifications() {
        let flux = Flux::new();
        let count = Arc::new(AtomicU64::new(0));
        let c = count.clone();

        let id = flux.subscribe("auth/state", move |_, _| {
            c.fetch_add(1, Ordering::Relaxed);
        });

        flux.on("update", |_, _, store: Arc<StateStore>| async move {
            store.set("auth/state", "x".to_string());
        });

        flux.emit("update", ()).await;
        assert_eq!(count.load(Ordering::Relaxed), 1);

        flux.unsubscribe("auth/state", id);
        flux.emit("update", ()).await;
        assert_eq!(count.load(Ordering::Relaxed), 1); // not incremented
    }

    // ========================================================================
    // has_handler
    // ========================================================================

    #[test]
    fn has_handler_check() {
        let flux = Flux::new();
        flux.on("auth/login", |_, _, _| async {});

        assert!(flux.has_handler("auth/login"));
        assert!(!flux.has_handler("auth/logout"));
    }

    #[test]
    fn has_handler_wildcard() {
        let flux = Flux::new();
        flux.on("auth/#", |_, _, _| async {});

        assert!(flux.has_handler("auth/login"));
        assert!(flux.has_handler("auth/deep/path"));
        assert!(!flux.has_handler("home/devices"));
    }

    // ========================================================================
    // emit_arc
    // ========================================================================

    #[tokio::test]
    async fn emit_arc_payload() {
        let flux = Flux::new();
        let called = Arc::new(AtomicU64::new(0));
        let called_c = called.clone();

        flux.on("test", move |_, payload, _| {
            let called = called_c.clone();
            async move {
                assert_eq!(payload.downcast_ref::<u32>(), Some(&42));
                called.fetch_add(1, Ordering::Relaxed);
            }
        });

        let payload: Arc<dyn Any + Send + Sync> = Arc::new(42u32);
        flux.emit_arc("test", payload).await;
        assert_eq!(called.load(Ordering::Relaxed), 1);
    }

    // ========================================================================
    // Full flow: bff-alpha style
    // ========================================================================

    #[tokio::test]
    async fn full_flow_initialize_accept_terms_login() {
        #[derive(Debug, Clone, PartialEq)]
        struct AuthState {
            phase: String,
            busy: bool,
        }

        #[derive(Debug, Clone, PartialEq)]
        struct TermsState {
            accepted: bool,
        }

        #[derive(Debug)]
        struct AcceptTermsReq {
            accepted: bool,
        }

        let flux = Flux::new();

        // Handler: app/initialize
        flux.on("app/initialize", |_, _, store: Arc<StateStore>| async move {
            store.set(
                "auth/state",
                AuthState {
                    phase: "unauthenticated".into(),
                    busy: false,
                },
            );
            store.set("auth/terms", TermsState { accepted: false });
            store.set("app/route", "/onboarding".to_string());
        });

        // Handler: auth/accept-terms
        flux.on(
            "auth/accept-terms",
            |_, payload, store: Arc<StateStore>| async move {
                let req = payload.downcast_ref::<AcceptTermsReq>().unwrap();
                store.set("auth/terms", TermsState { accepted: req.accepted });
                if req.accepted {
                    store.set("app/route", "/login".to_string());
                }
            },
        );

        // Handler: auth/login
        flux.on("auth/login", |_, _, store: Arc<StateStore>| async move {
            store.set(
                "auth/state",
                AuthState {
                    phase: "authenticated".into(),
                    busy: false,
                },
            );
            store.set("app/route", "/home".to_string());
        });

        // Track all state changes.
        let timeline = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let tl = timeline.clone();
        flux.subscribe("#", move |path, _| {
            tl.lock().unwrap().push(path.to_string());
        });

        // === Execute flow ===

        // 1. Initialize
        flux.emit("app/initialize", ()).await;

        let auth = flux.get("auth/state").unwrap();
        assert_eq!(
            auth.downcast_ref::<AuthState>().unwrap().phase,
            "unauthenticated"
        );
        let terms = flux.get("auth/terms").unwrap();
        assert!(!terms.downcast_ref::<TermsState>().unwrap().accepted);
        assert_eq!(
            flux.get("app/route")
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "/onboarding"
        );

        // 2. Accept terms
        flux.emit("auth/accept-terms", AcceptTermsReq { accepted: true })
            .await;

        let terms = flux.get("auth/terms").unwrap();
        assert!(terms.downcast_ref::<TermsState>().unwrap().accepted);
        assert_eq!(
            flux.get("app/route")
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "/login"
        );

        // 3. Login
        flux.emit("auth/login", ()).await;

        let auth = flux.get("auth/state").unwrap();
        assert_eq!(
            auth.downcast_ref::<AuthState>().unwrap().phase,
            "authenticated"
        );
        assert_eq!(
            flux.get("app/route")
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "/home"
        );

        // Verify timeline captured all changes.
        let tl = timeline.lock().unwrap();
        assert!(tl.len() >= 7); // at least 3 + 2 + 2 state changes
    }

    // ========================================================================
    // Compile-time: Flux is Send + Sync
    // ========================================================================

    fn _assert_flux_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Flux>();
        assert_sync::<Flux>();
    }
}
