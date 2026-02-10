use axum::Router;

/// A service module that contributes HTTP routes.
///
/// Each business module (auth, pms, task, ...) implements this trait
/// to register its API endpoints. The binary entry point collects all
/// modules and merges their routes into a single Router.
pub trait Module: Send + Sync {
    /// Module name, used for logging and route prefixes.
    fn name(&self) -> &str;

    /// Return the module's routes, to be nested under `/{name}`.
    fn routes(&self) -> Router;
}
