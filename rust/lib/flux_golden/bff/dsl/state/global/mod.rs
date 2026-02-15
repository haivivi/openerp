//! Global state definitions.
//!
//! Each file defines one state type stored at a well-known path.
//! In Phase 2, `#[state("path")]` macro generates the `PATH` const
//! and Cap'n Proto schema.

pub mod app;
pub mod auth;
pub mod compose;
pub mod profile;
pub mod search;
pub mod settings;
pub mod timeline;
pub mod tweet_detail;

pub use app::AppRoute;
pub use auth::{AuthPhase, AuthState, UserProfile};
pub use compose::ComposeState;
pub use profile::ProfilePage;
pub use search::SearchState;
pub use settings::{PasswordState, SettingsState};
pub use timeline::{FeedItem, TimelineFeed};
pub use tweet_detail::TweetDetail;
