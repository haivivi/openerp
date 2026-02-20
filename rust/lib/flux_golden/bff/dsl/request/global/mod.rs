//! Global request definitions.
//!
//! Each struct is a typed request payload with a `PATH` const.
//! In Phase 2, `#[request("path")]` macro generates the const,
//! and enforces that a matching handler exists (compile error if not).

pub mod app;
pub mod auth;
pub mod inbox;
pub mod search;
pub mod settings;
pub mod tweet;
pub mod user;

pub use app::{ComposeUpdateReq, InitializeReq, SetLocaleReq, TimelineLoadReq};
pub use auth::{LoginReq, LogoutReq};
pub use inbox::{InboxLoadReq, InboxMarkReadReq};
pub use search::{SearchClearReq, SearchReq};
pub use settings::{ChangePasswordReq, SettingsLoadReq, SettingsSaveReq};
pub use tweet::{CreateTweetReq, LikeTweetReq, LoadTweetReq, UnlikeTweetReq};
pub use user::{FollowUserReq, LoadProfileReq, UnfollowUserReq};
