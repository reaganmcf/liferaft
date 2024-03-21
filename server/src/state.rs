use actix::prelude::*;
use std::collections::HashMap;
use log::warn;

use crate::messages::{RequestVote, RequestVoteResult};

pub type NodeId = u8;
pub type LogIndex = u128;
pub type Term = u128;

#[derive(Debug, Clone)]
pub struct LogEntry {
    key: String,
    value: String,
}

#[derive(Debug, Clone)]
pub enum Role {
    Follower,
    Candidate,
    Leader {
        next_index: HashMap<NodeId, LogIndex>,
        match_index: HashMap<NodeId, LogIndex>,
    },
}

#[derive(Debug, Clone)]
pub struct State {
    role: Role,
    // persistent state on all servers
    current_term: Term,
    voted_for: Option<u128>,
    log: Vec<LogEntry>,

    // volatile state on all servers
    commit_index: u128,
    last_applied: u128,

    // volatile state on leaders
    // (Reinitialized after election)
    next_index: Vec<u128>,
    match_index: Vec<u128>,
}

impl State {
    pub fn initialize() -> State {
        State {
            role: Role::Follower,
            current_term: 0,
            voted_for: None,
            log: Vec::new(),

            commit_index: 0,
            last_applied: 0,

            next_index: Vec::new(),
            match_index: Vec::new(),
        }
    }
}

impl Actor for State {
    type Context = Context<Self>;
}

impl Handler<RequestVote> for State {
    type Result = RequestVoteResult;

    fn handle(&mut self, msg: RequestVote, _ctx: &mut Self::Context) -> Self::Result {
        warn!("{:#?}", msg);
        
        RequestVoteResult {
            term: 0,
            vote_granted: false
        }
    }
}
