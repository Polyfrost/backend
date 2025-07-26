pub mod caching;
pub mod endpoints;
pub mod metrics;
pub mod responses;

use std::{collections::HashSet, sync::Arc};

use actix_web::{
	http::header::HeaderMap,
	web::{self, Bytes, ServiceConfig}
};
use moka::future::Cache;

use crate::api::v1::metrics::ApiMetrics;

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct CacheKey {
	pub path: String,
	pub query: String
}

pub type ETagType = [u8; 32];

#[derive(Clone)]
pub struct CacheValue {
	pub response: Bytes,
	pub headers: HeaderMap,
	pub etag: ETagType
}

pub struct ApiData {
	/// The maven URL prefix to expose publicly, for example https://repo.polyfrost.org/
	pub public_maven_url: String,
	/// The maven URL prefix to resolve artifacts internally, for example https://172.19.0.3:8080/
	pub internal_maven_url: Option<String>,
	/// A reqwest client to use to fetch maven data
	pub client: Arc<reqwest::Client>,
	/// The allowlist of paths that should be cached
	pub cache_allowlist: HashSet<&'static str>,
	/// The internal cache used to cache artifact responses.
	pub cache: Cache<CacheKey, CacheValue>,
	/// All the metrics objects used for encoding and recording metrics
	pub metrics: ApiMetrics
}

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
	move |config| {
		config.service(
			web::scope("/v1")
				.wrap(actix_web::middleware::from_fn(caching::middleware))
				.wrap(actix_web::middleware::from_fn(metrics::middleware))
				.configure(metrics::configure())
				.configure(endpoints::artifacts::configure())
		);
	}
}
