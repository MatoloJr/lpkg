use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub canonical_id: String,
    pub name: String,
    pub version: String,
    pub backend: String,
    pub backend_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(default)]
    pub outdated: bool,
    #[serde(default)]
    pub is_app: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub canonical_id: String,
    pub name: String,
    pub version: Option<String>,
    pub backend: String,
    pub backend_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAvailable {
    pub backend_id: String,
    pub backend: String,
    pub current_version: String,
    pub latest_version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanupReport {
    pub removed: Vec<String>,
    pub freed_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub canonical_id: String,
    pub backend: String,
    pub backend_id: String,
    pub version: String,
    pub installed_at: DateTime<Utc>,
    #[serde(default)]
    pub binary_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    pub version: u32,
    pub packages: Vec<ManifestPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPackage {
    pub id: String,
    pub backend: String,
    pub backend_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinEntry {
    pub canonical_id: String,
    pub backend: String,
    pub backend_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
