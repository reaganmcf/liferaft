use actix::{Message, MessageResponse};
use serde::{Deserialize, Serialize};

use crate::state::Term;

#[derive(Deserialize, Serialize, Debug, Message)]
#[rtype(result = "RequestVoteResult")]
pub struct RequestVote {
    term: u64,
    candidate_id: String,
    last_log_index: u64,
    last_log_term: u64,
}

#[derive(Deserialize, Serialize, Debug, MessageResponse)]
pub struct RequestVoteResult {
    pub term: Term,
    pub vote_granted: bool
}
