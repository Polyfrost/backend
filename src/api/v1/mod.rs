pub mod artifacts;
pub mod responses;

use std::sync::Arc;

use actix_web::web::{self, ServiceConfig};
use artifacts::OneConfigQuery;
use moka::future::Cache;

#[derive(Hash, PartialEq, Eq)]
pub enum CacheKey {
	OneConfigArtifacts(OneConfigQuery)
}

pub struct ApiData {
	/// The maven URL prefix to expose publicly, for example https://repo.polyfrost.org/
	pub public_maven_url: String,
	/// The maven URL prefix to resolve artifacts internally, for example https://172.19.0.3:8080/
	pub internal_maven_url: Option<String>,
	/// A reqwest client to use to fetch maven data
	pub client: Arc<reqwest::Client>,
	/// The internal cache used to cache artifact responses. The key is (Cache
	/// Type, Cache ID)
	pub cache: Cache<CacheKey, String>
}

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
	move |config| {
		config.service(web::scope("/v1").configure(artifacts::configure()));
	}
}
