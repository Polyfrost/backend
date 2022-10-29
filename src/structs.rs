use reqwest::Client;
use serde::Serialize;

pub struct AppState {
    pub public_maven_url: String,
    pub internal_maven_url: String,
    pub http_client: Client,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Serialize, Clone)]
pub struct MavenDataResponse {
    pub url: String,
    pub sha256: String,
}

#[derive(Serialize)]
pub struct OneconfigDataResponse {
    pub release: MavenDataResponse,
    pub snapshot: MavenDataResponse,
    pub loader: MavenDataResponse,
}
