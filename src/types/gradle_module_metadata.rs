use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GradleModuleMetadata {
	pub variants: Vec<Variant>
}

#[derive(Debug, Deserialize)]
#[serde(tag = "name", rename_all = "camelCase")]
pub enum Variant {
	OneConfigModulesApiElements {
		dependencies: Vec<Dependency>
	},
	#[serde(other)]
	Other
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
	pub group: String,
	pub module: String,
	pub version: VersionRequirement,
	pub third_party_compatibility: Option<ThirdPartyCompatibility>,
	#[serde(default)]
	pub attributes: DependencyAttributes
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ThirdPartyCompatibility {
	pub artifact_selector: Option<ArtifactSelector>
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Hash, Default)]
pub struct DependencyAttributes {
	#[serde(rename = "org.polyfrost.oneconfig.loader.include", default)]
	pub loader_include: bool,
	#[serde(rename = "org.polyfrost.oneconfig.loader.jij", default)]
	pub jij: bool
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct ArtifactSelector {
	pub name: String,
	pub extension: String,
	pub classifier: String
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct VersionRequirement {
	pub requires: String
}
