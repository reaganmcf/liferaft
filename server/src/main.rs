use std::collections::HashSet;

use actix::{Actor, Addr};
use actix_web::{middleware::Logger, web, App, HttpServer};
use admin::get_state;
use clap::Parser;
use dotenv::dotenv;
use models::NodeId;
use node::Node;
use rpc::{raft_append_entries, raft_request_vote};

mod admin;
mod log;
mod messages;
mod models;
mod node;
mod rpc;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    id: String,

    #[arg(long)]
    peers: Vec<String>,

    #[arg(long)]
    port: u16,
}

pub struct AppData {
    node_actor: Addr<Node>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();
    let args = Args::parse();

    if args.peers.is_empty() {
        panic!("At least one peer must be provided");
    }

    let node_addr =
        node::Node::initialize(args.id, args.peers.into_iter().collect::<HashSet<NodeId>>())
            .start();

    HttpServer::new(move || {
        App::new()
            .service(get_state)
            .service(raft_request_vote)
            .service(raft_append_entries)
            .wrap(Logger::default())
            .app_data(web::Data::new(AppData {
                node_actor: node_addr.clone(),
            }))
    })
    .bind(("127.0.0.1", args.port))?
    .workers(1)
    .run()
    .await
}
