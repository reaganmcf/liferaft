use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use log::{debug, error};

use crate::{
    messages::{AppendEntries, AppendEntriesResponse, RequestVote, RequestVoteResponse},
    models::NodeId,
    AppData,
};

#[post("/raft-request-vote")]
async fn raft_request_vote(Json(body): Json<RequestVote>, data: Data<AppData>) -> impl Responder {
    match data.state_actor.send(body).await {
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
    match data.state_actor.send(body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub async fn send_vote_request(node_id: NodeId, request_vote: RequestVote) -> RequestVoteResponse {
    let url = format!("http://localhost:{}/raft-request-vote", node_id);

    debug!("Sending vote request to {}", node_id);

    let client = awc::Client::default();
    match client.post(url).send_json(&request_vote).await {
        Ok(mut response) => response.json::<RequestVoteResponse>().await.unwrap(),
        Err(_) => {
            error!("Failed to send vote request to {}", node_id);
            panic!();
        }
    }
}

pub async fn send_append_entries(
    node_id: NodeId,
    append_entries: AppendEntries,
) -> AppendEntriesResponse {
    let url = format!("http://localhost:{}/raft-append-entries", node_id);

    debug!("Sending append entries to {}", node_id);

    let client = awc::Client::default();
    match client.post(url).send_json(&append_entries).await {
        Ok(mut response) => response.json::<AppendEntriesResponse>().await.unwrap(),
        Err(_) => {
            error!("Failed to send append entries to {}", node_id);
            panic!();
        }
    }
}
