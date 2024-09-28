use actix_web::web;
use reqwest::Client;
use semver::Version;
use thiserror::Error;

use crate::{
	api::v1::ApiData,
	types::{
		gradle_module_metadata::{Dependency, GradleModuleMetadata},
		maven_metadata::MavenMetadata
	}
};

#[derive(Error, Debug)]
pub enum MavenError {
	#[error("A response/request error from reqwest")]
	Reqwest(#[from] reqwest::Error),
	#[error("An error occurred while trying to parse an XML response")]
	XmlParse(#[from] quick_xml::DeError),
	#[error("An error occurred while trying to parse versions with SemVer")]
	Semver(#[from] semver::Error),
	#[error("There were no artifact versions in the maven-metadata.xml file")]
	NoVersions,
	#[error(
		"There was a mismatch between the requested module ID or group and the response"
	)]
	ArtifactMismatch
}

#[inline]
pub fn get_dep_url(url: &str, repository: &str, dep: &Dependency) -> String {
	let filename = match dep
		.third_party_compatibility
		.as_ref()
		.map(|i| &i.artifact_selector)
	{
		Some(Some(selector)) => format!(
			"{name}-{version}-{classifier}.{extension}",
			name = selector.name,
			version = dep.version.requires,
			classifier = selector.classifier,
			extension = selector.extension
		),
		_ => format!(
			"{artifact}-{version}.jar",
			artifact = dep.module,
			version = dep.version.requires
		)
	};
	format!(
		"{url}{repository}/{group}/{artifact}/{version}/{filename}",
		group = dep.group.replace('.', "/"),
		artifact = dep.module,
		version = dep.version.requires
	)
}

pub async fn fetch_maven_metadata(
	state: &web::Data<ApiData>,
	repository: &str,
	group: &str,
	artifact: &str
) -> Result<MavenMetadata, MavenError> {
	let xml = state
		.client
		.get(format!(
			"{url}{repository}/{group}/{artifact}/maven-metadata.xml",
			url = state
				.internal_maven_url
				.clone()
				.unwrap_or(state.public_maven_url.clone()),
			group = group.replace('.', "/")
		))
		.send()
		.await?
		.error_for_status()?
		.text()
		.await?;

	let result: MavenMetadata = quick_xml::de::from_str(&xml)?;

	if group != result.group_id || artifact != result.artifact_id {
		return Err(MavenError::ArtifactMismatch);
	}

	Ok(result)
}

pub async fn fetch_latest_artifact(
	state: &web::Data<ApiData>,
	repository: &str,
	group: &str,
	artifact: &str
) -> Result<Version, MavenError> {
	let metadata = fetch_maven_metadata(state, repository, group, artifact).await?;
	metadata
		.versioning
		.versions
		.versions
		.into_iter()
		.map(|v| semver::Version::parse(&v))
		.collect::<Result<Vec<_>, semver::Error>>()?
		.into_iter()
		.max()
		.ok_or(MavenError::NoVersions)
}

pub async fn fetch_module_metadata(
	state: &web::Data<ApiData>,
	repository: &str,
	group: &str,
	artifact: &str,
	version: &str
) -> Result<GradleModuleMetadata, MavenError> {
	Ok(state
		.client
		.get(format!(
			"{url}{repository}/{group}/{artifact}/{version}/{artifact}-{version}.module",
			url = state
				.internal_maven_url
				.clone()
				.unwrap_or(state.public_maven_url.clone()),
			group = group.replace('.', "/")
		))
		.send()
		.await?
		.error_for_status()?
		.json()
		.await?)
}

pub async fn fetch_checksum(client: Client, url: String) -> Result<String, MavenError> {
	Ok(client
		.get(format!("{url}.sha1"))
		.send()
		.await?
		.error_for_status()?
		.text()
		.await?)
}
