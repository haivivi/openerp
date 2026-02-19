use flux_derive::request;
use serde::{Deserialize, Serialize};

#[request("inbox/load")]
#[derive(Serialize, Deserialize)]
pub struct InboxLoadReq;

#[request("inbox/mark-read")]
#[derive(Serialize, Deserialize)]
pub struct InboxMarkReadReq {
    pub message_id: String,
}
