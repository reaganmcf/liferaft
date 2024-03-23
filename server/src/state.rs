use actix::prelude::*;
use log::{debug, error, warn};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::messages::{AppendEntries, AppendEntriesResponse, RequestVote, RequestVoteResponse};
use crate::models::*;
use crate::rpc::{send_append_entries, send_vote_request};

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
    election_timeout: Duration,
}

impl State {
    pub fn initialize(id: NodeId, other_nodes: HashSet<NodeId>) -> State {
        let election_timeout = {
            if id == String::from("1234") {
                Duration::from_secs(3)
            } else {
                Duration::from_secs(thread_rng().gen_range(1..=40))
            }
        };

        debug!("Election timeout set to {:?}", election_timeout);

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
            election_timeout,
        }
    }

    fn check_heartbeat(&mut self, ctx: &mut Context<Self>) {
        if self.last_heartbeat.elapsed() > self.election_timeout {
            warn!("Leader has timed out");
            self.role = Role::candidate(&self.other_nodes);
            self.start_election(ctx);
        }
    }

    fn start_election(&mut self, ctx: &mut Context<Self>) {
        debug!("Starting election");

        self.role = Role::candidate(&self.other_nodes);
        self.current_term += 1;
        self.voted_for = Some(self.id.clone());

        let addr = ctx.address();

        for node_id in self.other_nodes.iter() {
            let request_vote = RequestVote {
                term: self.current_term,
                candidate_id: self.id.clone(),
                last_log_index: self.log.len() as u64,
                last_log_term: self.log.last().map_or(0, |entry| entry.term),
            };

            let cloned = node_id.clone();
            let cloned_addr = addr.clone();
            actix_web::rt::spawn(async move {
                let response = send_vote_request(cloned, request_vote).await;
                cloned_addr.do_send(response);
            });
        }
    }

    fn qourum(&self) -> usize {
        self.other_nodes.len() / 2 + 1
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct CheckTimeout;

impl Actor for State {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let interval = Duration::from_secs(3);
        ctx.run_interval(interval, |_act, ctx| ctx.notify(CheckTimeout));
    }
}

impl Handler<CheckTimeout> for State {
    type Result = ();
    fn handle(&mut self, _msg: CheckTimeout, ctx: &mut Self::Context) -> Self::Result {
        if self.role.is_leader() {
            self.last_heartbeat = Instant::now();

            let addr = ctx.address();
            for node_id in self.other_nodes.iter() {
                let append_entries = AppendEntries {
                    term: self.current_term,
                    leader_id: self.id.clone(),
                    prev_log_index: self.log.len() as u128,
                    prev_log_term: self.log.last().map_or(0, |entry| entry.term),
                    entries: Vec::default(),
                    leader_commit: self.commit_index,
                };

                let cloned = node_id.clone();
                let cloned_addr = addr.clone();
                actix_web::rt::spawn(async move {
                    let response = send_append_entries(cloned, append_entries).await;
                    cloned_addr.do_send(response);
                });
            }
        } else {
            self.check_heartbeat(ctx);
        }
    }
}

impl Handler<RequestVote> for State {
    type Result = RequestVoteResponse;

    fn handle(&mut self, msg: RequestVote, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received a vote request from {}", msg.candidate_id);
        debug!("{:#?}", msg);

        if msg.term > self.current_term {
            self.current_term = msg.term;
            self.role = Role::Follower;
            self.voted_for = None;
        }

        let last_term = self.log.last().map_or(0, |entry| entry.term);

        let log_ok = msg.last_log_term > last_term
            || (msg.last_log_term == last_term && msg.last_log_index >= self.log.len() as u64);

        if msg.term == self.current_term && log_ok && self.voted_for.is_none() {
            self.voted_for = Some(msg.candidate_id.clone());

            debug!("Voted for {}", msg.candidate_id);
            RequestVoteResponse::granted(self.id.clone(), self.current_term)
        } else {
            RequestVoteResponse::denied(self.id.clone(), self.current_term)
        }
    }
}

impl Handler<AppendEntries> for State {
    type Result = AppendEntriesResponse;

    fn handle(&mut self, msg: AppendEntries, _ctx: &mut Self::Context) -> Self::Result {
        warn!("{:#?}", msg);

        if msg.term < self.current_term {
            debug!(
                "msg.term < self.current_term : {} < {}",
                msg.term, self.current_term
            );
            return AppendEntriesResponse::failure(self.id.clone(), self.current_term);
        }

        match self.log.get(msg.prev_log_index as usize) {
            None => {
                debug!("prev_log_index of {} does not exist", msg.prev_log_index);
                return AppendEntriesResponse::failure(self.id.clone(), self.current_term);
            }
            Some(entry) => {
                if entry.term != msg.prev_log_term {
                    debug!(
                        "entry.term != msg.prev_log_term : {} != {}",
                        entry.term, msg.prev_log_term
                    );
                    return AppendEntriesResponse::failure(self.id.clone(), self.current_term);
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

        AppendEntriesResponse::success(self.id.clone(), self.current_term)
    }
}

impl Handler<RequestVoteResponse> for State {
    type Result = ();

    fn handle(&mut self, msg: RequestVoteResponse, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received a vote result, {:#?}", msg);

        match &mut self.role {
            Role::Candidate { votes_received } => {
                if msg.term == self.current_term && msg.vote_granted {
                    votes_received.insert(msg.from, true);
                    debug!("Votes received: {:#?}", votes_received);
                    let yes_votes = votes_received.iter().map(|(_, vote)| *vote).len();
                    if yes_votes >= self.qourum() {

                        // TODO - is this right???
                        self.role = Role::Leader {
                            next_index: self
                                .other_nodes
                                .iter()
                                .map(|node_id| (node_id.clone(), self.log.len() as u128))
                                .collect(),
                            match_index: HashMap::default(),
                        };

                        debug!("Elected as leader");

                        for node_id in self.other_nodes.iter() {
                            self.next_index
                                .insert(node_id.clone(), self.log.len() as u128);
                            self.match_index.insert(node_id.clone(), 0);

                            // TODO - replicate log entries to all nodes
                        }
                    }
                }
            }
            _ => {
                if msg.term > self.current_term {
                    self.current_term = msg.term;
                    self.role = Role::Follower;
                    self.voted_for = None;
                }

                // TODO - cancel election timer?
                self.last_heartbeat = Instant::now();
            }
        }
    }
}

impl Handler<AppendEntriesResponse> for State {
    type Result = ();

    fn handle(&mut self, msg: AppendEntriesResponse, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received an append entries result, {:#?}", msg);
        error!("AppendEntriesResult not implemented");
        panic!();
    }
}
