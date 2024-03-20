use actix_web::{post, web::Json, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
struct RequestVote {
    term: u64,
    candidate_id: String,
    last_log_index: u64,
    last_log_term: u64,
}

#[post("/raft-request-vote")]
async fn request_vote(Json(body): Json<RequestVote>) -> impl Responder {
    HttpResponse::Ok().json(body)
}
