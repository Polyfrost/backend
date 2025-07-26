pub mod endpoints;
pub mod metrics;
pub mod responses;

use actix_web::web::{self, ServiceConfig};

pub fn configure(config: &mut ServiceConfig) {
	config.service(
		web::scope("/v1")
			.configure(endpoints::artifacts::configure)
			.wrap(actix_web::middleware::from_fn(metrics::middleware))
	);
}
