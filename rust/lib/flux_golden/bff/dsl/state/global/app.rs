//! App-level state — stored at `app/route`.

use flux_derive::state;

/// Navigation route.
#[state("app/route")]
pub struct AppRoute(pub String);
