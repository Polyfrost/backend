use actix_web::web;
use anyhow::{anyhow, bail};
use semver::Version;

use crate::{
    api::v1::ApiData,
    types::{
        gradle_module_metadata::{Dependency, GradleModuleMetadata},
        maven_metadata::MavenMetadata,
    },
};

#[inline]
pub fn get_dep_url(state: &web::Data<ApiData>, repository: &str, dep: &Dependency) -> String {
    let filename = match dep
        .third_party_compatibility
        .as_ref()
        .map(|i| &i.artifact_selector)
    {
        Some(Some(selector)) => format!(
            "{name}-{classifier}.{extension}",
            name = selector.name,
            classifier = selector.classifier,
            extension = selector.extension
        ),
        _ => format!(
            "{artifact}-{version}.jar",
            artifact = dep.module,
            version = dep.version.requires
        ),
    };
    format!(
        "{url}{repository}/{group}/{artifact}/{version}/{filename}",
        url = state
            .internal_maven_url
            .clone()
            .unwrap_or(state.public_maven_url.clone()),
        group = dep.group.replace('.', "/"),
        artifact = dep.module,
        version = dep.version.requires
    )
}

pub async fn fetch_maven_metadata(
    state: &web::Data<ApiData>,
    repository: &str,
    group: &str,
    artifact: &str,
) -> anyhow::Result<MavenMetadata> {
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

    if group != result.group_id {
        bail!("Fetched group ID did not match parameters")
    }
    if artifact != result.artifact_id {
        bail!("Fetched group ID did not match parameters")
    }

    Ok(result)
}

pub async fn fetch_latest_artifact(
    state: &web::Data<ApiData>,
    repository: &str,
    group: &str,
    artifact: &str,
) -> anyhow::Result<Version> {
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
        .ok_or(anyhow!("No maximum version found"))
}

pub async fn fetch_module_metadata(
    state: &web::Data<ApiData>,
    repository: &str,
    group: &str,
    artifact: &str,
    version: &str,
) -> anyhow::Result<GradleModuleMetadata> {
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
