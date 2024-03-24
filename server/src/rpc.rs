use anyhow::{anyhow, Result};

use anyhow::Context;
use log::debug;

use crate::{
    messages::{AppendEntries, AppendEntriesResponse, RequestVote, RequestVoteResponse},
    models::NodeId,
};

#[derive(Clone)]
pub struct RpcClient {
    client: awc::Client,
}

// Safety: RpcClient is safe to send and share between threads
unsafe impl Send for RpcClient {}
unsafe impl Sync for RpcClient {}

impl RpcClient {
    pub fn new() -> Self {
        Self {
            client: awc::Client::default(),
        }
    }

    pub async fn send_request_vote(
        &self,
        node_id: NodeId,
        request_vote: RequestVote,
    ) -> Result<RequestVoteResponse> {
        debug!("Sending vote request to {}", node_id);

        self.send_request(node_id, "raft-request-vote", request_vote)
            .await
            .with_context(|| "vote request failed")
    }

    pub async fn send_append_entries(
        &self,
        node_id: NodeId,
        append_entries: AppendEntries,
    ) -> Result<AppendEntriesResponse> {
        debug!("Sending append entries to {}", node_id);

        self.send_request(node_id, "raft-append-entries", append_entries)
            .await
            .with_context(|| "append entries failed")
    }

    async fn send_request<D: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        node_id: NodeId,
        path: &str,
        body: D,
    ) -> Result<R> {
        let url = format!("http://localhost:{}/{}", node_id, path);

        let mut response = self
            .client
            .post(url)
            .send_json(&body)
            .await
            .map_err(|e| anyhow!("Failed to send request to {}: {}", node_id, e))?;
        response
            .json::<R>()
            .await
            .with_context(|| format!("Failed to send request to {}", node_id))
    }
}
