use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use log::error;

use crate::{messages::RequestVote, AppData};

#[post("/raft-request-vote")]
async fn request_vote(Json(body): Json<RequestVote>, data: Data<AppData>) -> impl Responder {
    match data.state_actor.send(body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
