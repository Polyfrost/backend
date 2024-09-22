pub mod artifacts;

use actix_web::{get, web::{self, ServiceConfig}, HttpResponse, Responder};
use utoipa::OpenApi;

pub struct ApiData {
    /// The maven URL prefix to expose publicly, for example https://repo.polyfrost.org/
    pub public_maven_url: String,
    /// The maven URL prefix to resolve artifacts internally, for example https://172.19.0.3:8912/
    pub internal_maven_url: Option<String>,
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

pub fn configure(data: &crate::AppCommand) -> impl FnOnce(&mut ServiceConfig) + '_ {
    move |config| {
        config.service(
            web::scope("/v1")
                .app_data(web::Data::new(ApiData {
                    internal_maven_url: data.internal_maven_url.clone().map(|url| url.to_string()),
                    public_maven_url: data.public_maven_url.to_string()
                }))
                .service(openapi_json)
                .configure(artifacts::configure()),
        );
    }
}
