use actix_web::{
	HttpResponse,
	Responder,
	body::MessageBody,
	dev::{ServiceRequest, ServiceResponse},
	get,
	middleware::Next,
	web::{self, ServiceConfig}
};
use documented::DocumentedFields;
use prometheus_client::{
	encoding::{EncodeLabelSet, text::encode},
	metrics::{counter::Counter, family::Family},
	registry::Registry
};

use crate::api::v1::{
	ApiData,
	endpoints::artifacts::{ArtifactQuery, OneConfigVersionInfo}
};

/// A macro that automatically initializes and registers a metric using inferred
/// types and doc comments from the ApiMetrics struct
macro_rules! make_api_metric {
	($registry:expr, $name:ident) => {
		make_api_metric!($registry, $name, Family)
	};
	($registry:expr, $name:ident, $type:ident) => {
		let $name = $type::default();
		let name_str = stringify!($name);
		$registry.register(
			name_str,
			ApiMetrics::get_field_docs(name_str)
				.expect(&format!("No doc comment for '{}' field", name_str)),
			$name.clone()
		);
	};
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, EncodeLabelSet)]
struct PlatformAgnosticArtifactLabels {
	r#type: String
}

/// A struct containing all of the metrics state for the API
#[derive(DocumentedFields)]
pub struct ApiMetrics {
	/// The registry used for storing metrics
	registry: Registry,
	/// The amount of OneConfig artifacts requests, by version and loader
	oneconfig_artifacts_requests: Family<OneConfigVersionInfo, Counter>,
	/// The amount of platform-agnostic artifacts requests, by type
	platform_agnostic_artifacts_requests: Family<PlatformAgnosticArtifactLabels, Counter>
}

/// Configures the metrics endpoint. In addition to this, the metrics middleware
/// must be registered AFTER all possible request configuration to ensure all
/// requests are handled and logged to metrics, even cached ones.
pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
	|config| {
		config.service(metrics_endpoint);
	}
}

/// Initializes the [ApiMetrics] struct with all metrics at their default values
pub fn init_metrics() -> ApiMetrics {
	let mut registry = <Registry>::default();

	make_api_metric!(registry, oneconfig_artifacts_requests);
	make_api_metric!(registry, platform_agnostic_artifacts_requests);

	ApiMetrics {
		registry,
		oneconfig_artifacts_requests,
		platform_agnostic_artifacts_requests
	}
}

/// The endpoint to allow scraping metrics
#[get("/metrics")]
async fn metrics_endpoint(state: web::Data<ApiData>) -> impl Responder {
	let mut body = String::new();
	if let Err(e) = encode(&mut body, &state.metrics.registry) {
		return HttpResponse::InternalServerError()
			.content_type("text/plain")
			.body(format!("Error encoding metrics: {e}"));
	}

	HttpResponse::Ok()
		.content_type("application/openmetrics-text; version=1.0.0; charset=utf-8")
		.body(body)
}

/// A middleware to increment all metrics per-request
pub async fn middleware(
	mut service_request: ServiceRequest,
	next: Next<impl MessageBody>
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
	if let Some(pattern) = service_request.match_pattern() {
		let data = service_request.extract::<web::Data<ApiData>>().await?;

		match pattern.as_str() {
			"/v1/artifacts/oneconfig" => {
				data.metrics
					.oneconfig_artifacts_requests
					.get_or_create(
						&service_request
							.extract::<web::Query<ArtifactQuery<OneConfigVersionInfo>>>()
							.await?
							.version_info
					)
					.inc();
			}
			"/v1/artifacts/{artifact:stage1|relaunch}" => {
				data.metrics
					.platform_agnostic_artifacts_requests
					.get_or_create(&PlatformAgnosticArtifactLabels {
						// Unfortunately actix makes it difficult to extract the real
						// parsed URL parameter, so just substring instead as a substitute
						r#type: service_request.uri().path()
							[const { "/v1/artifacts/".len() }..]
							.to_string()
					})
					.inc();
			}
			_ => ()
		};
	}

	// Let the real request handler continue
	return next.call(service_request).await;
}
