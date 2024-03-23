use actix::{Message, MessageResponse};
use serde::{Deserialize, Serialize};

use crate::{models::*, log::{LogIndex, LogEntry}};

#[derive(Deserialize, Serialize, Debug, Clone, Message)]
#[rtype(result = "RequestVoteResponse")]
pub struct RequestVote {
    pub term: Term,
    pub candidate_id: NodeId,
    pub last_log_index: u64,
    pub last_log_term: Term,
}

#[derive(Deserialize, Serialize, Debug, Clone, Message, MessageResponse)]
#[rtype(result = "()")]
pub struct RequestVoteResponse {
    pub from: NodeId,
    pub term: Term,
    pub vote_granted: bool,
}

impl RequestVoteResponse {
    pub fn granted(from: NodeId, term: Term) -> Self {
        Self {
            from,
            term,
            vote_granted: true,
        }
    }

    pub fn denied(from: NodeId, term: Term) -> Self {
        Self {
            from,
            term,
            vote_granted: false,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Message)]
#[rtype(result = "AppendEntriesResponse")]
pub struct AppendEntries {
    pub term: Term,
    pub leader_id: NodeId,
    pub prev_log_index: LogIndex,
    pub prev_log_term: Term,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u128,
}

#[derive(Deserialize, Serialize, Debug, Clone, Message, MessageResponse)]
#[rtype(result = "()")]
pub struct AppendEntriesResponse {
    pub from: NodeId,
    pub term: Term,
    pub success: bool,
}

impl AppendEntriesResponse {
    pub fn success(from: NodeId, term: Term) -> Self {
        Self {
            from,
            term,
            success: true,
        }
    }

    pub fn failure(from: NodeId, term: Term) -> Self {
        Self {
            from,
            term,
            success: false,
        }
    }
}
