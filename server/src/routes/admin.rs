use actix::{Handler, Message};
use actix_web::{get, web::Data, HttpResponse, Responder};
use log::error;

use crate::{models::State, node::Node, AppData};

#[get("/state")]
async fn get_state(data: Data<AppData>) -> impl Responder {
    match data.node_actor.send(AdminGetState).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("{}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[derive(Message)]
#[rtype(result = "State")]
pub struct AdminGetState;

impl Handler<AdminGetState> for Node {
    type Result = State;

    fn handle(&mut self, _msg: AdminGetState, _ctx: &mut Self::Context) -> Self::Result {
        self.state.clone()
    }
}
