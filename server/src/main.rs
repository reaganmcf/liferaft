use actix::{Actor, Addr};
use actix_web::{middleware::Logger, web, App, HttpServer};
use admin::get_state;
use clap::Parser;
use dotenv::dotenv;
use rpc::{raft_request_vote, raft_append_entries};
use state::State;

mod admin;
mod messages;
mod rpc;
mod state;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    port: u16,
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
            .service(get_state)
            .service(raft_request_vote)
            .service(raft_append_entries)
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
