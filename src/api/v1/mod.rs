pub mod artifacts;
pub mod responses;

use std::sync::Arc;

use actix_web::web::{self, ServiceConfig};
use artifacts::{ArtifactQuery, OneConfigVersionInfo};
use moka::future::Cache;

#[derive(Hash, PartialEq, Eq)]
pub enum CacheKey {
	ArtifactsOneConfig(ArtifactQuery<OneConfigVersionInfo>),
	ArtifactsStage1(ArtifactQuery)
}

#[derive(Clone)]
pub struct CacheValue {
	pub response: String,
	pub etag: String
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
		config.service(web::scope("/v1").configure(artifacts::configure()));
	}
}
