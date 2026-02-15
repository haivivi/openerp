//! App-level state — stored at `app/route`.

/// Navigation route.
// #[state("app/route")]
#[derive(Debug, Clone, PartialEq)]
pub struct AppRoute(pub String);

impl AppRoute {
    pub const PATH: &'static str = "app/route";
}
