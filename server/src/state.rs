use actix::prelude::*;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::messages::{AppendEntries, AppendEntriesResult, RequestVote, RequestVoteResult};

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
    Candidate,
    Leader {
        next_index: HashMap<NodeId, LogIndex>,
        match_index: HashMap<NodeId, LogIndex>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, MessageResponse)]
pub struct State {
    role: Role,
    // persistent state on all servers
    current_term: Term,
    voted_for: Option<u128>,
    log: Vec<LogEntry>,

    // volatile state on all servers
    commit_index: u128,
    last_applied: LogIndex,

    // volatile state on leaders
    // (Reinitialized after election)
    next_index: HashMap<NodeId, LogIndex>,
    match_index: HashMap<NodeId, LogIndex>,
}

impl State {
    pub fn initialize() -> State {
        State {
            role: Role::Follower,
            current_term: 0,
            voted_for: None,
            log: Vec::default(),

            commit_index: 0,
            last_applied: 0,

            next_index: HashMap::default(),
            match_index: HashMap::default(),
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
            vote_granted: false,
        }
    }
}

impl Handler<AppendEntries> for State {
    type Result = AppendEntriesResult;

    fn handle(&mut self, msg: AppendEntries, _ctx: &mut Self::Context) -> Self::Result {
        warn!("{:#?}", msg);

        if msg.term < self.current_term {
            debug!(
                "msg.term < self.current_term : {} < {}",
                msg.term, self.current_term
            );
            return AppendEntriesResult::failure(self.current_term);
        }

        match self.log.get(msg.prev_log_index as usize) {
            None => {
                debug!("prev_log_index of {} does not exist", msg.prev_log_index);
                return AppendEntriesResult::failure(self.current_term);
            }
            Some(entry) => {
                if entry.term != msg.prev_log_term {
                    debug!(
                        "entry.term != msg.prev_log_term : {} != {}",
                        entry.term, msg.prev_log_term
                    );
                    return AppendEntriesResult::failure(self.current_term);
                }
            }
        };

        for (offset, entry) in msg.entries.iter().enumerate() {
            let index = msg.prev_log_index as usize + offset + 1;

            match self.log.get(index) {
                None => {
                    self.log.push(entry.clone());
                }
                Some(existing) => {
                    if existing.term != entry.term {
                        self.log.truncate(index);
                        self.log.push(entry.clone());
                    }
                }
            }
        }

        if msg.leader_commit > self.commit_index {
            self.commit_index = std::cmp::min(msg.leader_commit, self.log.len() as u128);
        }

        AppendEntriesResult::success(self.current_term)
    }
}
