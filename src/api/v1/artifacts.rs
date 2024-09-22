use actix_web::{
    get,
    web::{self, ServiceConfig},
    HttpResponse,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
    |config| {
        config.service(web::scope("/artifacts").service(oneconfig));
    }
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ModLoader {
    Forge,
    Fabric,
}

#[derive(Serialize, Deserialize, IntoParams, Debug)]
pub struct OneConfigQuery {
    /// The minecraft version to fetch artifacts for
    version: String,
    /// The mod loader to fetch artifacts for
    loader: ModLoader,
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct ArtifactResponse {
    #[schema(example = "org.polyfrost.oneconfig")]
    group: String,
    #[schema(example = "1.8.9-forge")]
    name: String,
    #[schema(example = "8a7240ae4a1327a4a8a5c5e3bf15292e2a9bcc7c267d8710e05e2f191cba1a53")]
    checksum: String,
    #[schema(example = "https://repo.polyfrost.org/snapshots/org/polyfrost/oneconfig/1.8.9-forge/1.0.0-alpha.21/1.8.9-forge-1.0.0-alpha.21.jar")]
    url: String,
    // signatures: TODO
}

#[utoipa::path(
    get,
    context_path = "/artifacts",
    params(
        OneConfigQuery
    ),
    responses(
        (status = 200, description = "Lists the necessary artifacts for a specific oneconfig download", body = [ArtifactResponse])
    )
)]
#[get("/oneconfig")]
async fn oneconfig(query: web::Query<OneConfigQuery>) -> HttpResponse {
    let artifacts = Vec::<ArtifactResponse>::new();

    HttpResponse::Ok().json(artifacts)
}
