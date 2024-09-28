use std::fmt::Display;

use actix_web::{
	get,
	web::{self, ServiceConfig},
	HttpResponse,
	Responder
};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;
use utoipa::{IntoParams, ToSchema};

use crate::{
	api::v1::{
		responses::{consts::*, ErrorResponse},
		ApiData,
		CacheKey
	},
	maven::{self, MavenError},
	types::gradle_module_metadata::{GradleModuleMetadata, Variant}
};

const POLYFROST_GROUP: &str = "org.polyfrost";
const ONECONFIG_GROUP: &str = "org.polyfrost.oneconfig";

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
	|config| {
		config.service(web::scope("/artifacts").service(oneconfig));
	}
}

#[derive(Serialize, Deserialize, ToSchema, Debug, Hash, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ModLoader {
	Forge,
	Fabric
}

impl Display for ModLoader {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(match self {
			Self::Fabric => "fabric",
			Self::Forge => "forge"
		})
	}
}

#[derive(Serialize, Deserialize, IntoParams, Debug, Hash, PartialEq, Eq, Clone)]
pub struct OneConfigQuery {
	/// The minecraft version to fetch artifacts for
	#[param(example = "1.8.9")]
	version: String,
	/// The mod loader to fetch artifacts for
	#[param(example = "forge")]
	loader: ModLoader,
	/// Whether or not to use snapshots instead of official releases
	#[param(example = "false")]
	#[serde(default)]
	snapshots: bool
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct ArtifactResponse {
	#[schema(example = "org.polyfrost.oneconfig")]
	group: String,
	#[schema(example = "1.8.9-forge")]
	name: String,
	#[schema(
		example = "8a7240ae4a1327a4a8a5c5e3bf15292e2a9bcc7c267d8710e05e2f191cba1a53"
	)]
	checksum: String,
	#[schema(
		example = "https://repo.polyfrost.org/snapshots/org/polyfrost/oneconfig/1.8.9-forge/1.0.0-alpha.21/1.8.9-forge-1.0.0-alpha.21.jar"
	)]
	url: String // signatures: TODO
}

#[utoipa::path(
    get,
    context_path = "/artifacts",
    params(
        OneConfigQuery
    ),
    responses(
        (status = 200, description = "Lists the necessary artifacts for a specific oneconfig download", body = [ArtifactResponse]),
        (status = 500, description = "An error occurred while trying to resolve all artifacts for the requested OneConfig version", body = String)
    )
)]
#[get("/oneconfig")]
async fn oneconfig(
	state: web::Data<ApiData>,
	query: web::Query<OneConfigQuery>
) -> impl Responder {
	// Check cache for a valid response, and if so skip everything else
	let cache_key = CacheKey::OneConfigArtifacts(query.0.clone());
	if let Some(cached) = state.cache.get(&cache_key).await {
		return HttpResponse::Ok()
			.content_type("application/json")
			.body(cached);
	}

	let mut artifacts = Vec::<ArtifactResponse>::new();
	let repository = if query.snapshots {
		"snapshots"
	} else {
		"releases"
	};
	let oneconfig_variant = format!("{}-{}", query.version, query.loader);

	let latest_oneconfig_version = match maven::fetch_latest_artifact(
		&state,
		repository,
		ONECONFIG_GROUP,
		&oneconfig_variant
	)
	.await
	{
		Ok(v) => v,
		Err(MavenError::Reqwest(e)) if e.status().is_some_and(|c| c == 404) =>
			return ErrorResponse::InvalidOneConfigVersion {
				title: INVALID_ONECONFIG_VERSION_TITLE.to_string(),
				detail: format!(
					"The requested version {oneconfig_variant} could not be found in \
					 the requested {repository} repository"
				),
				instance: format!(
					"{INVALID_ONECONFIG_VERSION_INSTANCE_PREFIX}?version={version}&\
					 loader={loader}&repository={repository}",
					version = query.version,
					loader = query.loader
				)
			}
			.into(),
		Err(_) => unreachable!() // TODO add Semver handling, and NoVersions
	};

	// Add oneconfig itself to the artifacts
	let latest_oneconfig_url = format!(
		"{url}{repository}/{group}/{artifact}/{version}/{artifact}-{version}.jar",
		url = state
			.internal_maven_url
			.clone()
			.unwrap_or(state.public_maven_url.clone()),
		group = ONECONFIG_GROUP.replace('.', "/"),
		artifact = format!("{}-{}", query.version, query.loader),
		version = latest_oneconfig_version,
	);

	let Ok(checksum) =
		maven::fetch_checksum(state.client.clone(), latest_oneconfig_url.clone()).await
	else {
		return HttpResponse::InternalServerError()
			.body("unable to fetch checksum for oneconfig");
	};
	artifacts.push(ArtifactResponse {
		name: format!("{}-{}", query.version, query.loader),
		group: ONECONFIG_GROUP.to_string(),
		checksum,
		url: latest_oneconfig_url
	});

	// Resolve all relevant dependency bundles of the proper oneconfig version
	let Ok(dependency) = maven::fetch_module_metadata(
		&state,
		repository,
		ONECONFIG_GROUP,
		&format!("{}-{}", query.version, query.loader),
		&latest_oneconfig_version.to_string()
	)
	.await
	else {
		return HttpResponse::InternalServerError()
			.content_type("text/plain")
			.body(format!(
				"Error fetching module metadata for {}:{}-{}:{}",
				ONECONFIG_GROUP, query.version, query.loader, latest_oneconfig_version
			));
	};

	let mut bundles = Vec::<GradleModuleMetadata>::with_capacity(4);
	for variant in dependency.variants {
		let Variant::RuntimeElements { dependencies } = variant else {
			continue;
		};
		for dep in dependencies {
			if !dep.group.starts_with(ONECONFIG_GROUP) {
				continue;
			}

			let Ok(metadata) = maven::fetch_module_metadata(
				&state,
				repository,
				&dep.group,
				&dep.module,
				&dep.version.requires
			)
			.await
			else {
				return HttpResponse::InternalServerError()
					.content_type("text/plain")
					.body(format!(
						"Error resolving dependency {}:{}:{}",
						dep.group, dep.module, dep.version.requires
					));
			};
			bundles.push(metadata);
		}
	}

	// Resolve all dependencies of all bundles and add to `artifacts` vec
	let dependencies = bundles
		.into_iter()
		.flat_map(|b| b.variants)
		.filter_map(|v| match v {
			Variant::RuntimeElements { dependencies } => Some(dependencies),
			_ => None
		})
		.flatten()
		.filter(|d| d.group.starts_with(POLYFROST_GROUP));

	let mut set = JoinSet::new();
	for dep in dependencies {
		let url = maven::get_dep_url(
			&state
				.internal_maven_url
				.clone()
				.unwrap_or(state.public_maven_url.clone()),
			"mirror",
			&dep
		);
		let client = state.client.clone();
		set.spawn(
			async move { (dep, maven::fetch_checksum(client, url.clone()).await, url) }
		);
	}

	for (dep, checksum, url) in set.join_all().await {
		let Ok(checksum) = checksum else {
			return HttpResponse::InternalServerError().body(format!(
				"Unable to resolve checksum for dependent {}:{}:{}",
				dep.group, dep.module, dep.version.requires
			));
		};

		artifacts.push(ArtifactResponse {
			checksum,
			name: dep.module,
			group: dep.group,
			url
		});
	}

	// Convert artifacts to JSON and insert a copy into the cache
	let Ok(response) = serde_json::to_string(&artifacts) else {
		return HttpResponse::InternalServerError().body("huh");
	};
	state.cache.insert(cache_key, response.clone()).await;
	HttpResponse::Ok()
		.content_type("application/json")
		.body(response)
}
