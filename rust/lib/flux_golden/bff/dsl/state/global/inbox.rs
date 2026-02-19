use flux_derive::state;
use serde::{Deserialize, Serialize};

/// In-app inbox state — messages for the current user.
#[state("inbox/state")]
#[derive(Serialize, Deserialize)]
pub struct InboxState {
    pub messages: Vec<InboxMessage>,
    pub unread_count: usize,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxMessage {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub read: bool,
    pub created_at: String,
}
