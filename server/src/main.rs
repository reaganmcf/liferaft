use actix::{Actor, Addr};
use actix_web::{get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use dotenv::dotenv;
use rpc::request_vote;
use state::State;

mod rpc;
mod state;
mod messages;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    port: u16,
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

pub struct AppData {
    state_actor: Addr<State>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();
    let args = Args::parse();

    let state_addr = state::State::initialize().start();

    HttpServer::new(move || {
        App::new()
            .service(hello)
            .service(request_vote)
            .wrap(Logger::default())
            .app_data(web::Data::new(AppData {
                state_actor: state_addr.clone(),
            }))
    })
    .bind(("127.0.0.1", args.port))?
    .workers(1)
    .run()
    .await
}
