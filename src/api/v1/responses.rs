use actix_web::{http::StatusCode, HttpResponse, HttpResponseBuilder};
use serde::Serialize;

pub mod consts {
    pub const INVALID_ONECONFIG_VERSION_TITLE: &str =
        "The requested OneConfig version could not be found";
    pub const INVALID_ONECONFIG_VERSION_INSTANCE_PREFIX: &str =
        "https://api.polyfrost.org/v1/problems/invalid-oneconfig-version/instance";
}

/// An enum of error responses following RFC9457
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ErrorResponse {
    #[serde(rename = "https://api.polyfrost.org/v1/problems/invalid-oneconfig-version")]
    InvalidOneConfigVersion {
        title: String,
        detail: String,
        instance: String,
    },
}

impl From<ErrorResponse> for HttpResponse {
    fn from(value: ErrorResponse) -> Self {
        HttpResponseBuilder::new(match &value {
            ErrorResponse::InvalidOneConfigVersion { .. } => StatusCode::NOT_FOUND,
        })
        .content_type("application/json")
        .json(value)
    }
}
