use actix::prelude::*;
use futures::channel::oneshot;
use log::{debug, error, info, warn};
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::log::{KvChange, Log, LogEntry, LogEntryContent, LogIndex};
use crate::messages::{
    AppendEntries, AppendEntriesResponse, GetKey, GetKeyResponse, RequestVote, RequestVoteResponse,
    SetKey, SetKeyActorResponse, SetKeyResponse,
};
use crate::models::*;
use crate::rpc::RpcClient;

struct InflightAppendEntries {
    pub tx: oneshot::Sender<SetKeyResponse>,
    pub success_nodes: HashSet<NodeId>,
}

pub struct Node {
    pub id: NodeId,
    pub peers: HashSet<NodeId>,
    pub role: Role,

    pub state: State,
    pub election_timeout_interval: Duration,
    pub election_timer: Option<SpawnHandle>,
    pub heartbeat_interval: Duration,

    pub rpc_client: Arc<RpcClient>,
    inflight_append_entries: HashMap<(Term, LogIndex), InflightAppendEntries>,
}

impl Node {
    pub fn initialize(id: NodeId, peers: HashSet<NodeId>, log_path: PathBuf, heartbeat_interval: Duration) -> Self {
        // TODO - fix this hack
        let election_timeout_interval = {
            if id == *"1234" {
                Duration::from_secs(3)
            } else {
                Duration::from_secs(thread_rng().gen_range(5..=40))
            }
        };

        debug!("Election timeout set to {:?}", election_timeout_interval);

        let log = Log::new(log_path);
        let commit_index = log.len() as u128;

        Self {
            id,
            peers,

            role: Role::Follower,
            state: State {
                current_term: 0,
                voted_for: None,
                log,

                commit_index,
                last_applied: 0,

                next_index: HashMap::default(),
                match_index: HashMap::default(),
            },
            election_timeout_interval,
            election_timer: None,
            heartbeat_interval,
            rpc_client: Arc::new(RpcClient::new()),
            inflight_append_entries: HashMap::default(),
        }
    }

    fn start_election(&mut self, ctx: &mut Context<Self>) {
        info!("Starting election");

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
        debug!("Resetting election timer");
        if let Some(handle) = self.election_timer.take() {
            if !ctx.cancel_future(handle) {
                panic!("Failed to cancel election timer");
            }
        }

        self.election_timer = Some(ctx.run_later(self.election_timeout_interval, |_, ctx| {
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
        // Heartbeat timer for leaders
        ctx.run_interval(self.heartbeat_interval, |_act, ctx| {
            ctx.notify(SendHeartbeatToFollowers)
        });

        // Inflight AppendEntries checks
        ctx.run_interval(Duration::from_millis(250), |_act, ctx| {
            ctx.notify(CheckInflightAppendEntries)
        });

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
        info!("Received a vote request from {}", msg.candidate_id);
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

            info!("Voted for {}", msg.candidate_id);
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
        debug!("{:#?}", msg);

        let is_heartbeat = msg.entries.is_empty();

        if msg.term < self.state.current_term {
            debug!(
                "msg.term < self.state.current_term : {} < {}",
                msg.term, self.state.current_term
            );
            return AppendEntriesResponse::failure(
                self.id.clone(),
                self.state.current_term,
                msg.prev_log_index,
                is_heartbeat,
            );
        }

        match self.state.log.get(msg.prev_log_index) {
            None => {
                debug!("prev_log_index of {} does not exist", msg.prev_log_index);
                return AppendEntriesResponse::failure(
                    self.id.clone(),
                    self.state.current_term,
                    msg.prev_log_index,
                    is_heartbeat,
                );
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
                        msg.prev_log_index,
                        is_heartbeat,
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
            info!("Updated commit index to {}", self.state.commit_index);
            if let Err(e) = self.state.log.save() {
                error!("Failed to save log: {}", e);
                AppendEntriesResponse::failure(
                    self.id.clone(),
                    self.state.current_term,
                    msg.prev_log_index,
                    is_heartbeat,
                );
            }
        }

        self.reset_election_timer(ctx);
        AppendEntriesResponse::success(
            self.id.clone(),
            self.state.current_term,
            msg.prev_log_index,
            is_heartbeat,
        )
    }
}

impl Handler<RequestVoteResponse> for Node {
    type Result = ();

    fn handle(&mut self, msg: RequestVoteResponse, _ctx: &mut Self::Context) -> Self::Result {
        info!("Received a vote result from, {}", msg.from);
        debug!("{:#?}", msg);

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

                        info!("Elected as leader");

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
        if msg.was_heartbeat {
            return;
        }
        debug!("Received an append entries result, {:#?}", msg);

        match self
            .inflight_append_entries
            .get_mut(&(msg.term, msg.prev_log_index))
        {
            Some(inflight_request) => {
                inflight_request.success_nodes.insert(msg.from.clone());
                info!(
                    "Received append entries response from {} for term {} and prev_log_index {}",
                    msg.from, msg.term, msg.prev_log_index
                );
            }
            None => {
                error!(
                    "No inflight request found for term {} and prev_log_index {}",
                    msg.term, msg.prev_log_index
                );
            }
        }
    }
}

impl Handler<GetKey> for Node {
    type Result = GetKeyResponse;

    fn handle(&mut self, msg: GetKey, _ctx: &mut Self::Context) -> Self::Result {
        info!("Received a get key request, {:#?}", msg);
        let value = self.state.log.kv_get(msg.key);
        GetKeyResponse { value }
    }
}

impl Handler<SetKey> for Node {
    type Result = SetKeyActorResponse;

    fn handle(&mut self, msg: SetKey, ctx: &mut Self::Context) -> Self::Result {
        debug!("Received a set key request, {:#?}", msg);

        let log_entry = LogEntry {
            term: self.state.current_term,
            content: LogEntryContent::Kv(KvChange::Set(msg.key.clone(), msg.value.clone())),
        };
        let append_entry = AppendEntries {
            term: self.state.current_term,
            leader_id: self.id.clone(),
            prev_log_index: self.state.log.last_index(),
            prev_log_term: self.state.log.last_term(),
            entries: vec![log_entry.clone()],
            leader_commit: self.state.commit_index,
        };

        self.state.log.append(log_entry.clone());
        for node_id in self.peers.iter() {
            let cloned_addr = ctx.address().clone();
            let cloned_node_id = node_id.clone();
            let rpc_client = self.rpc_client.clone();
            let cloned_append_entry = append_entry.clone();
            actix_web::rt::spawn(async move {
                let future = rpc_client.send_append_entries(cloned_node_id, cloned_append_entry);
                match future.await {
                    Ok(response) => cloned_addr.do_send(response),
                    Err(e) => error!("{}", e),
                }
            });
        }

        let (tx, rx) = oneshot::channel();

        self.inflight_append_entries.insert(
            (append_entry.term, append_entry.prev_log_index),
            InflightAppendEntries {
                tx,
                success_nodes: HashSet::default(),
            },
        );

        SetKeyActorResponse(rx)
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct CheckInflightAppendEntries;

impl Handler<CheckInflightAppendEntries> for Node {
    type Result = ();

    fn handle(
        &mut self,
        _msg: CheckInflightAppendEntries,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        // TODO - handle some kind of timeout so we can retry
        let entries = std::mem::take(&mut self.inflight_append_entries);
        let mut kept: HashMap<(Term, LogIndex), InflightAppendEntries> = HashMap::default();

        for (key, inflight) in entries {
            if inflight.success_nodes.len() >= self.qourum() {
                info!(
                    "Received enough success' to commit log entry {}. ({} / {})",
                    key.1,
                    inflight.success_nodes.len(),
                    self.qourum()
                );

                self.state.commit_index = key.1;

                if inflight.tx.send(SetKeyResponse::success()).is_err() {
                    error!("Failed to send SetKeyResponse back to web handler");
                }
            } else {
                kept.insert(key, inflight);
            }
        }

        self.inflight_append_entries = kept;
    }
}
