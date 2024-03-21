use actix::{Message, MessageResponse};
use serde::{Deserialize, Serialize};

use crate::state::{Term, NodeId, LogIndex, LogEntry};

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
    pub vote_granted: bool,
}

#[derive(Deserialize, Serialize, Debug, Message)]
#[rtype(result = "AppendEntriesResult")]
pub struct AppendEntries {
    pub term: Term,
    pub leader_id: NodeId,
    pub prev_log_index: LogIndex,
    pub prev_log_term: Term,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u128,
}

#[derive(Deserialize, Serialize, Debug, MessageResponse)]
pub struct AppendEntriesResult {
    term: Term,
    success: bool
}

impl AppendEntriesResult {
    pub fn success(term: Term) -> Self {
        Self {
            term,
            success: true
        }
    }

    pub fn failure(term: Term) -> Self {
        Self {
            term,
            success: false
        }
    }
}
