use actix::{Message, MessageResponse};
use actix_web::web;
use serde::{Deserialize, Serialize};

use crate::{
    log::{LogEntry, LogIndex},
    models::*,
};

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

#[derive(Deserialize, Serialize, Debug, Clone, Message)]
#[rtype(result = "SetKeyResponse")]
pub struct SetKey {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, MessageResponse)]
pub struct SetKeyResponse {
    success: bool,
}

impl SetKeyResponse {
    pub fn success() -> Self {
        Self { success: true }
    }

    pub fn failure() -> Self {
        Self { success: false }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Message)]
#[rtype(result = "GetKeyResponse")]
pub struct GetKey {
    pub key: String,
}

impl From<web::Path<String>> for GetKey {
    fn from(path: web::Path<String>) -> Self {
        Self {
            key: path.into_inner(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, MessageResponse)]
pub struct GetKeyResponse {
    pub success: bool,
    pub value: Option<String>,
}

impl GetKeyResponse {
    pub fn success(value: Option<String>) -> Self {
        Self {
            success: true,
            value,
        }
    }

    pub fn failure() -> Self {
        Self {
            success: false,
            value: None,
        }
    }
}
