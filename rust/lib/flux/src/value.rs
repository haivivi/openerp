use std::any::{Any, TypeId};
use std::fmt;
use std::sync::Arc;

/// A type-erased, reference-counted state value.
///
/// Wraps `Arc<dyn Any + Send + Sync>` for zero-copy sharing across
/// multiple readers. Clone is cheap — just an atomic increment.
///
/// In Phase 2, the macro layer adds Cap'n Proto serialization for FFI
/// boundary crossing (Rust <-> Swift/Kotlin).
#[derive(Clone)]
pub struct StateValue {
    inner: Arc<dyn Any + Send + Sync>,
}

impl StateValue {
    /// Create a new StateValue from any `Send + Sync` type.
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self {
            inner: Arc::new(value),
        }
    }

    /// Try to downcast to a concrete type reference.
    ///
    /// Returns `None` if the stored type doesn't match `T`.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }

    /// Check if the stored value is of type `T`.
    pub fn is<T: Any>(&self) -> bool {
        self.inner.is::<T>()
    }

    /// Get the `TypeId` of the stored value.
    pub fn type_id(&self) -> TypeId {
        (*self.inner).type_id()
    }

    /// Get the number of strong references to the underlying value.
    ///
    /// Useful for verifying zero-copy behavior in tests.
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }
}

impl fmt::Debug for StateValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateValue")
            .field("type_id", &(*self.inner).type_id())
            .finish()
    }
}

/// Unique handle for a subscription, returned by `StateStore::subscribe()`.
///
/// Use this to unsubscribe later via `StateStore::unsubscribe()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub(crate) u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_downcast_correct_type() {
        let v = StateValue::new(42u32);
        assert_eq!(v.downcast_ref::<u32>(), Some(&42u32));
    }

    #[test]
    fn downcast_wrong_type_returns_none() {
        let v = StateValue::new(42u32);
        assert_eq!(v.downcast_ref::<i32>(), None);
        assert_eq!(v.downcast_ref::<String>(), None);
        assert_eq!(v.downcast_ref::<bool>(), None);
    }

    #[test]
    fn downcast_string() {
        let v = StateValue::new("hello".to_string());
        assert_eq!(v.downcast_ref::<String>(), Some(&"hello".to_string()));
        assert_eq!(v.downcast_ref::<&str>(), None); // &str != String
    }

    #[test]
    fn downcast_struct() {
        #[derive(Debug, PartialEq)]
        struct AuthState {
            phase: String,
            busy: bool,
        }

        let state = AuthState {
            phase: "authenticated".to_string(),
            busy: false,
        };
        let v = StateValue::new(state);
        let got = v.downcast_ref::<AuthState>().unwrap();
        assert_eq!(got.phase, "authenticated");
        assert!(!got.busy);
    }

    #[test]
    fn downcast_enum() {
        #[derive(Debug, PartialEq)]
        #[allow(dead_code)]
        enum Phase {
            Unknown,
            Authenticated,
        }
        let v = StateValue::new(Phase::Authenticated);
        assert_eq!(v.downcast_ref::<Phase>(), Some(&Phase::Authenticated));
    }

    #[test]
    fn downcast_vec() {
        let v = StateValue::new(vec![1u32, 2, 3]);
        let got = v.downcast_ref::<Vec<u32>>().unwrap();
        assert_eq!(got, &vec![1, 2, 3]);
    }

    #[test]
    fn downcast_option() {
        let v = StateValue::new(Some(42u32));
        assert_eq!(v.downcast_ref::<Option<u32>>(), Some(&Some(42u32)));

        let v = StateValue::new(None::<u32>);
        assert_eq!(v.downcast_ref::<Option<u32>>(), Some(&None));
    }

    #[test]
    fn is_correct_type() {
        let v = StateValue::new(42u32);
        assert!(v.is::<u32>());
        assert!(!v.is::<i32>());
        assert!(!v.is::<String>());
    }

    #[test]
    fn type_id_matches() {
        let v = StateValue::new(42u32);
        assert_eq!(v.type_id(), TypeId::of::<u32>());
        assert_ne!(v.type_id(), TypeId::of::<i32>());
    }

    #[test]
    fn clone_shares_arc() {
        let v1 = StateValue::new(42u32);
        assert_eq!(v1.ref_count(), 1);

        let v2 = v1.clone();
        assert_eq!(v1.ref_count(), 2);
        assert_eq!(v2.ref_count(), 2);

        // Both point to the same underlying data.
        let p1 = v1.downcast_ref::<u32>().unwrap() as *const u32;
        let p2 = v2.downcast_ref::<u32>().unwrap() as *const u32;
        assert_eq!(p1, p2);
    }

    #[test]
    fn clone_is_zero_copy() {
        // Store a large Vec. Clone should NOT duplicate the data.
        let big = vec![0u8; 1_000_000];
        let v1 = StateValue::new(big);
        assert_eq!(v1.ref_count(), 1);

        let v2 = v1.clone();
        assert_eq!(v1.ref_count(), 2);

        // Same pointer — the Vec data was NOT copied.
        let p1 = v1.downcast_ref::<Vec<u8>>().unwrap().as_ptr();
        let p2 = v2.downcast_ref::<Vec<u8>>().unwrap().as_ptr();
        assert_eq!(p1, p2);
    }

    #[test]
    fn drop_decrements_ref_count() {
        let v1 = StateValue::new(42u32);
        let v2 = v1.clone();
        assert_eq!(v1.ref_count(), 2);

        drop(v2);
        assert_eq!(v1.ref_count(), 1);
    }

    #[test]
    fn multiple_types_coexist() {
        let a = StateValue::new(42u32);
        let b = StateValue::new("hello".to_string());
        let c = StateValue::new(true);

        assert_eq!(a.downcast_ref::<u32>(), Some(&42));
        assert_eq!(b.downcast_ref::<String>(), Some(&"hello".to_string()));
        assert_eq!(c.downcast_ref::<bool>(), Some(&true));
    }

    #[test]
    fn unit_type() {
        let v = StateValue::new(());
        assert!(v.is::<()>());
        assert_eq!(v.downcast_ref::<()>(), Some(&()));
    }

    #[test]
    fn nested_arc() {
        // StateValue wrapping an Arc should still work.
        let inner = Arc::new(42u32);
        let v = StateValue::new(inner.clone());
        let got = v.downcast_ref::<Arc<u32>>().unwrap();
        assert_eq!(**got, 42);
    }

    #[test]
    fn debug_format() {
        let v = StateValue::new(42u32);
        let debug = format!("{:?}", v);
        assert!(debug.contains("StateValue"));
        assert!(debug.contains("type_id"));
    }

    #[test]
    fn subscription_id_equality() {
        let a = SubscriptionId(1);
        let b = SubscriptionId(1);
        let c = SubscriptionId(2);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn subscription_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SubscriptionId(1));
        set.insert(SubscriptionId(2));
        set.insert(SubscriptionId(1)); // duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn subscription_id_copy() {
        let a = SubscriptionId(1);
        let b = a; // Copy
        assert_eq!(a, b); // a is still valid (Copy, not moved)
    }

    #[test]
    fn subscription_id_debug() {
        let id = SubscriptionId(42);
        let debug = format!("{:?}", id);
        assert!(debug.contains("42"));
    }

    // Compile-time: StateValue must be Send + Sync.
    fn _assert_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<StateValue>();
        assert_sync::<StateValue>();
    }
}
