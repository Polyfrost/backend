#![feature(try_blocks, duration_constructors)]

mod api;
mod maven;
mod types;

use std::net::Ipv4Addr;

use actix_web::{web, App, HttpServer};
use clap::Parser;
use url::Url;
use utoipa_swagger_ui::{Config, SwaggerUi};

#[derive(Parser, Clone)]
#[clap(version, about, long_about = None)]
struct AppCommand {
    #[clap(long, env = "BACKEND_LISTEN_PORT")]
    pub port: u16,
    #[clap(long, env = "BACKEND_LISTEN_HOST")]
    pub host: Ipv4Addr,
    #[clap(long, env = "BACKEND_INTERNAL_MAVEN_URL")]
    pub internal_maven_url: Option<Url>,
    #[clap(long, env = "BACKEND_PUBLIC_MAVEN_URL")]
    pub public_maven_url: Url,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = AppCommand::parse();
    let listen_args = (args.host, args.port);

    HttpServer::new(move || {
        App::new()
            .configure(api::v1::configure(&args))
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").config(
                Config::new(["/v1/openapi.json"])
            ))
            .service(web::redirect("/", "/swagger-ui/"))
    })
    .bind(listen_args)?
    .run()
    .await
}
