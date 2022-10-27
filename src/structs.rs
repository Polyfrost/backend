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
