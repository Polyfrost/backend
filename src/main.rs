#![feature(duration_constructors)]

mod api;
mod maven;
mod types;

use std::{net::Ipv4Addr, time::Duration};

use actix_web::{web, App, HttpServer};
use api::v1::ApiData;
use clap::Parser;
use moka::future::Cache;
use url::Url;

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
	pub public_maven_url: Url
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
	let args = AppCommand::parse();
	let listen_args = (args.host, args.port);
	let data = web::Data::new(ApiData {
		internal_maven_url: args.internal_maven_url.clone().map(|url| url.to_string()),
		public_maven_url: args.public_maven_url.to_string(),
		client: reqwest::ClientBuilder::new()
			.user_agent(concat!(
				env!("CARGO_PKG_NAME"),
				"/",
				env!("CARGO_PKG_VERSION"),
				" (",
				env!("CARGO_PKG_REPOSITORY"),
				")"
			))
			.build()
			.unwrap()
			.into(),
		cache: Cache::builder()
			.max_capacity(500)
			.time_to_idle(Duration::from_hours(5))
			.build()
	});

	HttpServer::new(move || {
		App::new()
			.app_data(data.clone())
			.configure(api::v1::configure())
	})
	.bind(listen_args)?
	.run()
	.await
}
