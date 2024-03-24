use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use log::error;

use crate::{
    messages::{GetKey, SetKey},
    AppData,
};

#[post("/kv/set")]
async fn set(data: Data<AppData>, Json(body): Json<SetKey>) -> impl Responder {
    match data.node_actor.send(body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/kv/get")]
async fn get(data: Data<AppData>, Json(body): Json<GetKey>) -> impl Responder {
    match data.node_actor.send(body).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
