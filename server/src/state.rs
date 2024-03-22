use actix::prelude::*;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::messages::{AppendEntries, AppendEntriesResult, RequestVote, RequestVoteResult};
use crate::models::*;
use crate::rpc::send_vote_request;

#[derive(Debug, Clone, Serialize, Deserialize, MessageResponse)]
pub struct State {
    id: NodeId,
    other_nodes: HashSet<NodeId>,

    role: Role,
    // persistent state on all servers
    current_term: Term,
    voted_for: Option<NodeId>,
    log: Vec<LogEntry>,

    // volatile state on all servers
    commit_index: u128,
    last_applied: LogIndex,

    // volatile state on leaders
    // (Reinitialized after election)
    next_index: HashMap<NodeId, LogIndex>,
    match_index: HashMap<NodeId, LogIndex>,

    // Heartbeat
    #[serde(skip_serializing, skip_deserializing, default = "Instant::now")]
    last_heartbeat: Instant,
}

impl State {
    pub fn initialize(id: NodeId, other_nodes: HashSet<NodeId>) -> State {
        State {
            id,
            other_nodes, 

            role: Role::Follower,
            current_term: 0,
            voted_for: None,
            log: Vec::default(),

            commit_index: 0,
            last_applied: 0,

            next_index: HashMap::default(),
            match_index: HashMap::default(),

            last_heartbeat: Instant::now(),
        }
    }

    fn heartbeat(&mut self) {
        todo!();
    }

    fn check_heartbeat(&mut self) {
        if self.last_heartbeat.elapsed() > Duration::from_secs(1) {
            warn!("Leader has timed out");
            self.role = Role::Candidate;
            self.start_election();
        }
    }

    fn start_election(&mut self) {
        debug!("Starting election");

        self.role = Role::Candidate;
        self.current_term += 1;
        self.voted_for = Some(self.id.clone());

        for node_id in self.other_nodes.iter() {
            let request_vote = RequestVote {
                term: self.current_term,
                candidate_id: self.id.clone(),
                last_log_index: self.log.len() as u64,
                last_log_term: self.log.last().map_or(0, |entry| entry.term),
            };

            let cloned = node_id.clone();
            actix_web::rt::spawn(send_vote_request(cloned, request_vote));
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct CheckTimeout;

impl Actor for State {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(1), |_act, ctx| ctx.notify(CheckTimeout));
    }
}

impl Handler<CheckTimeout> for State {
    type Result = ();
    fn handle(&mut self, _msg: CheckTimeout, _ctx: &mut Self::Context) -> Self::Result {
        if self.role.is_leader() {
            self.heartbeat();
        } else {
            self.check_heartbeat();
        }
    }
}

impl Handler<RequestVote> for State {
    type Result = RequestVoteResult;

    fn handle(&mut self, msg: RequestVote, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received a vote request from {}", msg.candidate_id);
        debug!("{:#?}", msg);

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
