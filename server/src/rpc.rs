use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use log::{error, debug};

use crate::{
    messages::{AppendEntries, RequestVote},
    AppData, models::NodeId,
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

pub async fn send_vote_request(node_id: NodeId, request_vote: RequestVote) {
    let url = format!("http://localhost:{}/raft-request-vote", node_id);

    debug!("Sending vote request to {}", node_id);

    let client = awc::Client::default();
    if client.post(url).send_json(&request_vote).await.is_err() {
        error!("Failed to send vote request to {}", node_id);
    }
}
