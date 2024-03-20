use actix_web::{get, middleware::Logger, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use dotenv::dotenv;

mod rpc;
mod state;

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();
    let args = Args::parse();

    HttpServer::new(|| App::new().service(hello).wrap(Logger::default()))
        .bind(("127.0.0.1", args.port))?
        .run()
        .await
}
