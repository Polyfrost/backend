use actix_web::web::ServiceConfig;

pub mod endpoint;
pub mod types;
pub mod utils;

pub fn configure(config: &mut ServiceConfig) { config.service(endpoint::oneconfig); }
