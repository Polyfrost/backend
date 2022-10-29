use reqwest::Client;
use serde::Serialize;

pub struct AppState {
    pub maven_url: String,
    pub http_client: Client,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct MavenDataResponse {
    pub url: String,
    pub sha256: String
}

#[derive(Serialize)]
pub struct OneconfigDataResponse {
    pub release: MavenDataResponse,
    pub snapshot: MavenDataResponse,
    pub loader: MavenDataResponse
}