//! App-level state — stored at `app/route`.

use flux_derive::state;
use serde::{Deserialize, Serialize};

/// Navigation route.
#[state("app/route")]
#[derive(Serialize, Deserialize)]
pub struct AppRoute(pub String);
