use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use actix::{MessageResponse, SpawnHandle};
use serde::{Deserialize, Serialize};

use crate::log::{Log, LogIndex};

pub type NodeId = String;
pub type Term = u128;

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
    pub fn candidate(other_nodes: &HashSet<NodeId>) -> Role {
        Role::Candidate {
            votes_received: other_nodes
                .iter()
                .map(|node_id| (node_id.clone(), false))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, MessageResponse)]
pub struct State {
    // persistent state on all servers
    pub current_term: Term,
    pub voted_for: Option<NodeId>,
    pub log: Log,

    // volatile state on all servers
    pub commit_index: u128,
    pub last_applied: LogIndex,

    // volatile state on leaders
    // (Reinitialized after election)
    pub next_index: HashMap<NodeId, LogIndex>,
    pub match_index: HashMap<NodeId, LogIndex>,
}
