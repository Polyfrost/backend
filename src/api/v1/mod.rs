pub mod artifacts;
pub mod middleware;
pub mod responses;

use std::sync::Arc;

use actix_web::web::{self, Bytes, ServiceConfig};
use middleware::etag_middleware;
use moka::future::Cache;

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct CacheKey {
	pub path: String,
	pub query: String
}

pub type ETagType = [u8; 32];

#[derive(Clone)]
pub struct CacheValue {
	pub response: Bytes,
	pub etag: ETagType
}

pub struct ApiData {
	/// The maven URL prefix to expose publicly, for example https://repo.polyfrost.org/
	pub public_maven_url: String,
	/// The maven URL prefix to resolve artifacts internally, for example https://172.19.0.3:8080/
	pub internal_maven_url: Option<String>,
	/// A reqwest client to use to fetch maven data
	pub client: Arc<reqwest::Client>,
	/// The internal cache used to cache artifact responses.
	pub cache: Cache<CacheKey, CacheValue>
}

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
	move |config| {
		config.service(
			web::scope("/v1")
				.wrap(actix_web::middleware::from_fn(etag_middleware))
				.configure(artifacts::configure())
		);
	}
}
