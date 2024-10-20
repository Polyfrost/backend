use std::fmt::Display;

use actix_web::{
	get,
	web::{self, ServiceConfig},
	HttpResponse,
	Responder
};
use itertools::Itertools as _;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

use crate::{
	api::v1::{
		responses::{consts::*, ArtifactResponse, Checksum, ChecksumType, ErrorResponse},
		ApiData
	},
	maven::{self, MavenError},
	types::gradle_module_metadata::{
		ArtifactSelector,
		Dependency,
		GradleModuleMetadata,
		ThirdPartyCompatibility,
		Variant,
		VersionRequirement
	}
};

const POLYFROST_GROUP: &str = "org.polyfrost";
const ONECONFIG_GROUP: &str = "org.polyfrost.oneconfig";

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
	|config| {
		config.service(
			web::scope("/artifacts")
				.service(oneconfig)
				.service(platform_agnostic_artifacts)
		);
	}
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub struct OneConfigVersionInfo {
	/// The minecraft version to fetch artifacts for
	version: String,
	/// The mod loader to fetch artifacts for
	loader: ModLoader
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ArtifactQuery<V = ()> {
	/// Whether or not to use snapshots instead of official releases
	#[serde(default)]
	snapshots: bool,
	/// Extra version information
	#[serde(flatten)]
	version_info: V
}

#[get("/oneconfig")]
async fn oneconfig(
	state: web::Data<ApiData>,
	query: web::Query<ArtifactQuery<OneConfigVersionInfo>>
) -> impl Responder {
	let mut artifacts = Vec::<ArtifactResponse>::new();
	let repository = if query.snapshots {
		"snapshots"
	} else {
		"releases"
	};
	let oneconfig_variant = format!(
		"{}-{}",
		query.version_info.version, query.version_info.loader
	);
	let maven_url = state
		.internal_maven_url
		.clone()
		.unwrap_or(state.public_maven_url.clone());

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
					version = query.version_info.version,
					loader = query.version_info.loader
				)
			}
			.into(),
		Err(_) => unreachable!() // TODO add Semver handling, and NoVersions
	};

	// Add oneconfig itself to the artifacts
	let latest_oneconfig_url = format!(
		"{maven_url}{repository}/{group}/{artifact}/{version}/{artifact}-{version}.jar",
		group = ONECONFIG_GROUP.replace('.', "/"),
		artifact = format!(
			"{}-{}",
			query.version_info.version, query.version_info.loader
		),
		version = latest_oneconfig_version,
	);

	let Ok(checksum) = maven::fetch_checksum(&state.client, &latest_oneconfig_url).await
	else {
		return HttpResponse::InternalServerError()
			.body("unable to fetch checksum for oneconfig");
	};
	artifacts.push(ArtifactResponse {
		name: format!(
			"{}-{}",
			query.version_info.version, query.version_info.loader
		),
		group: ONECONFIG_GROUP.to_string(),
		checksum: Checksum {
			r#type: ChecksumType::Sha256,
			hash: checksum
		},
		url: latest_oneconfig_url
	});

	// Resolve all relevant dependency bundles of the proper oneconfig version
	let Ok(dependency) = maven::fetch_module_metadata(
		&state,
		repository,
		ONECONFIG_GROUP,
		&format!(
			"{}-{}",
			query.version_info.version, query.version_info.loader
		),
		&latest_oneconfig_version.to_string()
	)
	.await
	else {
		return HttpResponse::InternalServerError()
			.content_type("text/plain")
			.body(format!(
				"Error fetching module metadata for {}:{}-{}:{}",
				ONECONFIG_GROUP,
				query.version_info.version,
				query.version_info.loader,
				latest_oneconfig_version
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

	// Takes the bundles, resolves all their relevant dependencies, and concurrently
	// resolves all checksums
	let dependencies_result = bundles
        .into_iter()
        // Resolve all relevant dependencies of the bundles
        .flat_map(|b| b.variants)
        .filter_map(|v| match v {
            Variant::RuntimeElements { dependencies } => Some(dependencies),
            _ => None,
        })
        .flatten()
        .filter(|d| d.group.starts_with(POLYFROST_GROUP))
        .unique()
        // Concurrently resolve all checksums
        .map(|dep| {
            let internal_dep_url = maven::get_dep_url(
                &state
                    .internal_maven_url
                    .clone()
                    .unwrap_or(state.public_maven_url.clone()),
                repository,
                &dep,
            );
			let dep_url = maven::get_dep_url(
                &state.public_maven_url,
                repository,
                &dep,
            );
            let client = state.client.clone();
            async move {
				(
					dep,
					maven::fetch_checksum(&client, &internal_dep_url).await,
					dep_url
				)
			}
        })
        .fold(JoinSet::new(), |mut acc, future| {
            acc.spawn(future);
            acc
        })
        .join_all()
        .await
        .into_iter()
        .map(|(dep, checksum, dep_url)| {
            Ok::<_, MavenError>(ArtifactResponse {
                checksum: Checksum {
					r#type: ChecksumType::Sha256,
					hash: checksum?
				},
                name: dep.module,
                group: dep.group,
                url: dep_url,
            })
        })
        .try_collect();

	match dependencies_result {
		Ok(mut deps) => artifacts.append(&mut deps),
		Err(e) =>
			return HttpResponse::InternalServerError()
				.content_type("text/plain")
				.body(format!("Error resolving dependency {e}")),
	}

	// Convert artifacts to JSON and insert a copy into the cache
	let Ok(response) = serde_json::to_string(&artifacts) else {
		return HttpResponse::InternalServerError().body("huh");
	};

	HttpResponse::Ok()
		.content_type("application/json")
		.body(response)
}

#[get("/{artifact:stage1|relaunch}")]
async fn platform_agnostic_artifacts(
	state: web::Data<ApiData>,
	query: web::Query<ArtifactQuery>,
	path: web::Path<(String,)>
) -> impl Responder {
	let artifact = path.into_inner().0;
	let repository = if query.snapshots {
		"snapshots"
	} else {
		"releases"
	};
	// Fetch the latest artifact version
	let latest_stage1_version = match maven::fetch_latest_artifact(
		&state,
		repository,
		ONECONFIG_GROUP,
		&artifact
	)
	.await
	{
		Ok(latest) => latest,
		Err(e) => {
			return HttpResponse::InternalServerError()
				.content_type("text/plain")
				.body(format!("Error resolving latest {artifact} version: {e}"));
		}
	};

	// Resolve URL and checksum
	let dep = Dependency {
		group: ONECONFIG_GROUP.to_string(),
		module: artifact.clone(),
		version: VersionRequirement {
			requires: latest_stage1_version.to_string()
		},
		third_party_compatibility: Some(ThirdPartyCompatibility {
			artifact_selector: Some(ArtifactSelector {
				classifier: "all".to_string(),
				extension: "jar".to_string(),
				name: artifact.clone()
			})
		})
	};

	let checksum = match maven::fetch_checksum(
		&state.client,
		&maven::get_dep_url(
			&state
				.internal_maven_url
				.clone()
				.unwrap_or(state.public_maven_url.clone()),
			repository,
			&dep
		)
	)
	.await
	{
		Ok(checksum) => checksum,
		Err(e) =>
			return HttpResponse::InternalServerError()
				.content_type("text/plain")
				.body(format!(
					"Error resolving latest {artifact} version checksum: {e}"
				)),
	};

	let response = match serde_json::to_string(&ArtifactResponse {
		name: artifact.clone(),
		group: ONECONFIG_GROUP.to_string(),
		checksum: Checksum {
			r#type: ChecksumType::Sha256,
			hash: checksum
		},
		url: maven::get_dep_url(&state.public_maven_url, repository, &dep)
	}) {
		Ok(response) => response,
		Err(e) => {
			return HttpResponse::InternalServerError()
				.content_type("text/plain")
				.body(format!("Error constructing latest {artifact} version: {e}"));
		}
	};

	HttpResponse::Ok()
		.content_type("application/json")
		.body(response)
}
