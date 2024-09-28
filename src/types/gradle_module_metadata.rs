use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GradleModuleMetadata {
	pub variants: Vec<Variant>
}

#[derive(Debug, Deserialize)]
#[serde(tag = "name", rename_all = "camelCase")]
pub enum Variant {
	RuntimeElements {
		dependencies: Vec<Dependency>
	},
	#[serde(other)]
	Other
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
	pub group: String,
	pub module: String,
	pub version: VersionRequirement,
	pub third_party_compatibility: Option<ThirdPartyCompatibility>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThirdPartyCompatibility {
	pub artifact_selector: Option<ArtifactSelector>
}

#[derive(Debug, Deserialize)]
pub struct ArtifactSelector {
	pub name: String,
	pub extension: String,
	pub classifier: String
}

#[derive(Debug, Deserialize)]
pub struct VersionRequirement {
	pub requires: String
}
