use std::collections::{HashMap, HashSet};

use serde::{Serialize, Deserialize};

pub type NodeId = String;
pub type LogIndex = u128;
pub type Term = u128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub term: Term,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Follower,
    Candidate {
        votes_received: HashMap<NodeId, bool>,
    },
    Leader {
        next_index: HashMap<NodeId, LogIndex>,
        match_index: HashMap<NodeId, LogIndex>,
    },
}

impl Role {
    pub fn is_leader(&self) -> bool {
        matches!(self, Role::Leader { .. })
    }

    pub fn candidate(other_nodes: &HashSet<NodeId>) -> Role {
        Role::Candidate {
            votes_received: other_nodes.iter().map(|node_id| (node_id.clone(), false)).collect(),
        }
    }
}

