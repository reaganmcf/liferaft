use anyhow::{anyhow, Result};

use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use anyhow::Context;
use log::{debug, error};

use crate::{
    messages::{AppendEntries, AppendEntriesResponse, RequestVote, RequestVoteResponse},
    models::NodeId,
    AppData,
};

#[post("/raft-request-vote")]
async fn raft_request_vote(Json(body): Json<RequestVote>, data: Data<AppData>) -> impl Responder {
    match data.node_actor.send(body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/raft-append-entries")]
async fn raft_append_entries(
    Json(body): Json<AppendEntries>,
    data: Data<AppData>,
) -> impl Responder {
    match data.node_actor.send(body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[derive(Clone)]
pub struct RpcClient {
    client: awc::Client,
}

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
