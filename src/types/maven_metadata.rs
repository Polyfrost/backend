use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MavenMetadata {
	pub group_id: String,
	pub artifact_id: String,
	pub versioning: MavenMetadataVersioning
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MavenMetadataVersioning {
	// pub latest: String,
	// pub release: String,
	pub versions: Versions // pub last_updated: String,
}

#[derive(Debug, Deserialize)]
pub struct Versions {
	#[serde(rename = "version")]
	pub versions: Vec<String>
}
