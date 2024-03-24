use actix::prelude::*;
use log::{debug, error, info, warn};
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use crate::log::{Log, LogIndex};
use crate::messages::{AppendEntries, AppendEntriesResponse, RequestVote, RequestVoteResponse};
use crate::models::*;
use crate::rpc::RpcClient;

pub struct Node {
    pub id: NodeId,
    pub peers: HashSet<NodeId>,
    pub role: Role,

    pub state: State,
    pub election_timeout_duration: Duration,
    pub election_timer: Option<SpawnHandle>,

    pub rpc_client: Arc<RpcClient>,
}

impl Node {
    pub fn initialize(id: NodeId, peers: HashSet<NodeId>) -> Self {
        let election_timeout_duration = {
            if id == String::from("1234") {
                Duration::from_secs(3)
            } else {
                Duration::from_secs(thread_rng().gen_range(5..=40))
            }
        };

        debug!("Election timeout set to {:?}", election_timeout_duration);

        Self {
            id,
            peers,

            role: Role::Follower,
            state: State {
                current_term: 0,
                voted_for: None,
                log: Log::new(),

                commit_index: 0,
                last_applied: 0,

                next_index: HashMap::default(),
                match_index: HashMap::default(),
            },
            election_timeout_duration,
            election_timer: None,
            rpc_client: Arc::new(RpcClient::new()),
        }
    }

    fn start_election(&mut self, ctx: &mut Context<Self>) {
        debug!("Starting election");

        self.role = Role::candidate(&self.peers);
        self.state.current_term += 1;
        self.state.voted_for = Some(self.id.clone());

        let addr = ctx.address();

        for node_id in self.peers.iter() {
            let request_vote = RequestVote {
                term: self.state.current_term,
                candidate_id: self.id.clone(),
                last_log_index: self.state.log.len() as u64,
                last_log_term: self.state.log.last().map_or(0, |entry| entry.term),
            };

            let cloned = node_id.clone();
            let cloned_addr = addr.clone();
            let rpc_client = self.rpc_client.clone();
            actix_web::rt::spawn(async move {
                let future = rpc_client.send_request_vote(cloned, request_vote);
                match future.await {
                    Ok(response) => cloned_addr.do_send(response),
                    Err(e) => error!("{}", e),
                }
            });
        }
    }

    fn qourum(&self) -> usize {
        self.peers.len() / 2 + 1
    }

    fn reset_election_timer(&mut self, ctx: &mut Context<Self>) {
        info!("Resetting election timer");
        if let Some(handle) = self.election_timer.take() {
            if ctx.cancel_future(handle) == false {
                panic!("Failed to cancel election timer");
            }
        }

        self.election_timer = Some(ctx.run_later(self.election_timeout_duration, |_, ctx| {
            ctx.notify(ElectionTimeout);
        }));
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct SendHeartbeatToFollowers;

impl Actor for Node {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let interval = Duration::from_secs(3);

        // Heartbeat timer for leaders
        ctx.run_interval(interval, |_act, ctx| ctx.notify(SendHeartbeatToFollowers));

        // Election timer for followers and candidates
        self.reset_election_timer(ctx);
    }
}

impl Handler<SendHeartbeatToFollowers> for Node {
    type Result = ();
    fn handle(&mut self, _msg: SendHeartbeatToFollowers, ctx: &mut Self::Context) -> Self::Result {
        if let Role::Leader { .. } = self.role {
            debug!("Sending heartbeat to followers");

            let addr = ctx.address();
            for node_id in self.peers.iter() {
                let append_entries = AppendEntries {
                    term: self.state.current_term,
                    leader_id: self.id.clone(),
                    prev_log_index: self.state.log.len() as u128,
                    prev_log_term: self.state.log.last().map_or(0, |entry| entry.term),
                    entries: Vec::default(),
                    leader_commit: self.state.commit_index,
                };

                let cloned = node_id.clone();
                let cloned_addr = addr.clone();
                let rpc_client = self.rpc_client.clone();
                actix_web::rt::spawn(async move {
                    let future = rpc_client.send_append_entries(cloned, append_entries);
                    match future.await {
                        Ok(response) => cloned_addr.do_send(response),
                        Err(e) => error!("{}", e),
                    }
                });
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ElectionTimeout;

impl Handler<ElectionTimeout> for Node {
    type Result = ();

    fn handle(&mut self, _msg: ElectionTimeout, ctx: &mut Self::Context) -> Self::Result {
        match &self.role {
            Role::Candidate { .. } => {
                self.start_election(ctx);
            }
            Role::Follower => {
                self.role = Role::candidate(&self.peers);
                self.start_election(ctx);
            }
            Role::Leader { .. } => {
                // Do nothing
            }
        }
    }
}

impl Handler<RequestVote> for Node {
    type Result = RequestVoteResponse;

    fn handle(&mut self, msg: RequestVote, ctx: &mut Self::Context) -> Self::Result {
        debug!("Received a vote request from {}", msg.candidate_id);
        debug!("{:#?}", msg);

        if msg.term > self.state.current_term {
            self.state.current_term = msg.term;
            self.role = Role::Follower;
            self.state.voted_for = None;
        }

        let last_term = self.state.log.last().map_or(0, |entry| entry.term);

        let log_ok = msg.last_log_term > last_term
            || (msg.last_log_term == last_term
                && msg.last_log_index >= self.state.log.len() as u64);

        if msg.term == self.state.current_term && log_ok && self.state.voted_for.is_none() {
            self.state.voted_for = Some(msg.candidate_id.clone());

            debug!("Voted for {}", msg.candidate_id);
            self.reset_election_timer(ctx);
            RequestVoteResponse::granted(self.id.clone(), self.state.current_term)
        } else {
            RequestVoteResponse::denied(self.id.clone(), self.state.current_term)
        }
    }
}

impl Handler<AppendEntries> for Node {
    type Result = AppendEntriesResponse;

    fn handle(&mut self, msg: AppendEntries, ctx: &mut Self::Context) -> Self::Result {
        warn!("{:#?}", msg);

        if msg.term < self.state.current_term {
            debug!(
                "msg.term < self.state.current_term : {} < {}",
                msg.term, self.state.current_term
            );
            return AppendEntriesResponse::failure(self.id.clone(), self.state.current_term);
        }

        match self.state.log.get(msg.prev_log_index) {
            None => {
                debug!("prev_log_index of {} does not exist", msg.prev_log_index);
                return AppendEntriesResponse::failure(self.id.clone(), self.state.current_term);
            }
            Some(entry) => {
                if entry.term != msg.prev_log_term {
                    debug!(
                        "entry.term != msg.prev_log_term : {} != {}",
                        entry.term, msg.prev_log_term
                    );
                    return AppendEntriesResponse::failure(
                        self.id.clone(),
                        self.state.current_term,
                    );
                }
            }
        };

        for (offset, entry) in msg.entries.iter().enumerate() {
            let index = msg.prev_log_index + (offset as u128) + 1;

            match self.state.log.get(index) {
                None => {
                    self.state.log.append(entry.clone());
                }
                Some(existing) => {
                    if existing.term != entry.term {
                        self.state.log.truncate(index);
                        self.state.log.append(entry.clone());
                    }
                }
            }
        }

        if msg.leader_commit > self.state.commit_index {
            self.state.commit_index =
                std::cmp::min(msg.leader_commit, self.state.log.len() as u128);
        }

        self.reset_election_timer(ctx);
        AppendEntriesResponse::success(self.id.clone(), self.state.current_term)
    }
}

impl Handler<RequestVoteResponse> for Node {
    type Result = ();

    fn handle(&mut self, msg: RequestVoteResponse, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received a vote result, {:#?}", msg);

        match &mut self.role {
            Role::Candidate { votes_received } => {
                if msg.term == self.state.current_term && msg.vote_granted {
                    votes_received.insert(msg.from, true);
                    debug!("Votes received: {:#?}", votes_received);
                    let yes_votes = votes_received.iter().map(|(_, vote)| *vote).len();
                    if yes_votes >= self.qourum() {
                        // TODO - is this right???
                        self.role = Role::Leader {
                            next_index: self
                                .peers
                                .iter()
                                .map(|node_id| (node_id.clone(), self.state.log.len() as u128))
                                .collect(),
                            match_index: self
                                .peers
                                .iter()
                                .map(|node_id| (node_id.clone(), 0))
                                .collect(),
                        };

                        debug!("Elected as leader");

                        for node_id in self.peers.iter() {
                            self.state
                                .next_index
                                .insert(node_id.clone(), self.state.log.len() as u128);
                            self.state.match_index.insert(node_id.clone(), 0);

                            // TODO - replicate log entries to all nodes
                        }
                    }
                }
            }
            _ => {
                if msg.term > self.state.current_term {
                    self.state.current_term = msg.term;
                    self.role = Role::Follower;
                    self.state.voted_for = None;
                }
                self.reset_election_timer(_ctx);
            }
        }
    }
}

impl Handler<AppendEntriesResponse> for Node {
    type Result = ();

    fn handle(&mut self, msg: AppendEntriesResponse, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Received an append entries result, {:#?}", msg);
        error!("AppendEntriesResult not implemented");
        // panic!();
    }
}
