#![feature(duration_constructors_lite)]

mod api;
mod maven;
mod types;

use std::{collections::HashSet, net::SocketAddr, time::Duration};

use actix_web::{App, HttpServer, web};
use api::v1::{ApiData, CacheKey, CacheValue, ETagType};
use clap::Parser;
use moka::future::Cache;
use url::Url;

/// The main command that starts the backend HTTP server. The server can be
/// configured either with flags or environment variables, listed in the help
/// message.
#[derive(Parser, Clone)]
#[clap(version, about, long_about = None)]
struct AppCommand {
	/// The addresses (IP and port) for the HTTP server to bind to.
	///
	/// Multiple bind addresses can be either split by comma, or passed as
	/// multiple flags.
	#[clap(
		long,
		env = "BACKEND_LISTEN_PORT",
		value_delimiter = ',',
		default_value = "[::]:8080"
	)]
	pub bind: Vec<SocketAddr>,
	/// If passed, the server will be downgraded to HTTP/1.1 rather than HTTP/2
	#[clap(long, env = "BACKEND_USE_HTTP1", default_value_t = false)]
	pub http1: bool,
	/// Sets the maven root server url that will be advertised for public
	/// downloads through the API.
	#[clap(long, env = "BACKEND_PUBLIC_MAVEN_URL")]
	pub public_maven_url: Url,
	/// If set, the maven root server url that will be used for maven requests
	/// (such as checksum requests), but not publicly advertised via the API. If
	/// unset, defaults to the public maven url. If maven is running on the
	/// same host as this backend, then this can be set to a local IP to
	/// greatly speed up requests.
	#[clap(long, env = "BACKEND_INTERNAL_MAVEN_URL")]
	pub internal_maven_url: Option<Url>
}

#[tokio::main]
#[allow(clippy::needless_return)] // Clippy seems to be hallucinating a return statement at the end of main()
async fn main() {
	env_logger::init();

	let args = AppCommand::parse();
	let data = web::Data::new(ApiData {
		internal_maven_url: args.internal_maven_url.map(|url| url.to_string()),
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
		cache_allowlist: HashSet::from([
			"/v1/artifacts/oneconfig",
			"/v1/artifacts/{artifact:stage1|relaunch}"
		]),
		cache: Cache::builder()
			.time_to_live(Duration::from_mins(2))
			.weigher(|k: &CacheKey, v: &CacheValue| {
				(k.path.len()
					+ k.query.len() + const { std::mem::size_of::<ETagType>() }
					+ v.response.len()
					+ std::mem::size_of_val(&v.headers))
				.try_into()
				.unwrap_or(u32::MAX)
			})
			.max_capacity(/* 10 MiB */ const { 10 * 1024 * 1024 })
			.build(),
		metrics: api::v1::metrics::init_metrics()
	});

	let mut server = HttpServer::new(move || {
		App::new()
			.app_data(data.clone())
			.configure(api::v1::configure())
	});

	// Call .bind for each address, as using multiple in the same call can silently
	// fail
	for bind_addr in args.bind {
		server = server
			.bind_auto_h2c(bind_addr)
			.expect("Unable to bind on specified address");
	}

	server.run().await.expect("Unable to start HTTP server");
}
