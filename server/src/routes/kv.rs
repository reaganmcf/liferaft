use actix_web::{
    get,
    post,
    web::{Data, Json, Path},
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

#[get("/kv/get/{key}")]
async fn get(data: Data<AppData>, path: Path<String>) -> impl Responder {
    let request: GetKey = path.into();

    match data.node_actor.send(request).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
