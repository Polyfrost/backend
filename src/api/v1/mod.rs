pub mod artifacts;
pub mod responses;

use std::sync::Arc;

use actix_web::{
	get,
	web::{self, ServiceConfig},
	HttpResponse,
	Responder
};
use artifacts::OneConfigQuery;
use moka::future::Cache;
use utoipa::OpenApi;

#[derive(Hash, PartialEq, Eq)]
pub enum CacheKey {
	OneConfigArtifacts(OneConfigQuery)
}

pub struct ApiData {
	/// The maven URL prefix to expose publicly, for example https://repo.polyfrost.org/
	pub public_maven_url: String,
	/// The maven URL prefix to resolve artifacts internally, for example https://172.19.0.3:8912/
	pub internal_maven_url: Option<String>,
	/// A reqwest client to use to fetch maven data
	pub client: Arc<reqwest::Client>,
	/// The internal cache used to cache artifact responses. The key is (Cache
	/// Type, Cache ID)
	pub cache: Cache<CacheKey, String>
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Polyfrost API",
        description = "An API used to help with updating Polyfrost software",
        contact(
            name = "Tyler Beckman",
            email = "ty@myriation.xyz",
            url = "https://polyfrost.org"
        ),
        version = "v1"
    ),
    components(
        schemas(
            artifacts::ModLoader,
            artifacts::ArtifactResponse
        )
    ),
    paths(artifacts::oneconfig),
    servers(
        (
            url = "http://localhost:8080/v1",
            description = "Local API"
        ),
        (
            url = "https://repo.polyfrost.org/v1",
            description = "Official API"
        )
    )
)]
struct ApiDoc;

#[get("/openapi.json")]
pub async fn openapi_json() -> impl Responder {
	HttpResponse::Ok().json(ApiDoc::openapi())
}

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
	move |config| {
		config.service(
			web::scope("/v1")
				.service(openapi_json)
				.configure(artifacts::configure())
		);
	}
}
