//! Typed declarations, installation metadata and persisted lifecycle state.
use crate::json::OpaqueObject;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EnvironmentSpec {
    pub version: u64,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "Vec<String>")]
    pub python: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "Vec<String>")]
    pub node: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "Vec<String>")]
    pub packages: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "BTreeMap<String, String>")]
    pub env: Option<BTreeMap<String, String>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "Vec<PostScript>")]
    pub post_scripts: Option<Vec<PostScript>>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Default for EnvironmentSpec {
    fn default() -> Self {
        Self {
            version: 1,
            python: None,
            node: None,
            packages: None,
            env: None,
            post_scripts: None,
            extra: OpaqueObject::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PostScript {
    pub id: String,
    pub run: String,
    pub user: GuestUser,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GuestUser {
    Work,
    Root,
}
impl GuestUser {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::Root => "root",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Activation {
    pub generation: String,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub config: EnvironmentSpec,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default)]
    pub image_sha256: String,
    #[serde(default)]
    pub verified: Verification,
    #[serde(default)]
    pub verified_at: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Verification {
    pub profile: String,
    pub python: Vec<String>,
    pub node: Vec<String>,
    pub packages: BTreeMap<String, String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Failure {
    pub stage: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Failure {
    pub(crate) fn new(
        stage: &str,
        message: impl Into<String>,
        fingerprint: Option<String>,
    ) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
            fingerprint,
            extra: OpaqueObject::new(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStatus {
    #[default]
    Unavailable,
    Ready,
    PendingRestart,
    Failed,
    Applying,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentState {
    pub format: u64,
    pub active: Option<Activation>,
    pub pending: Option<Activation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<Activation>,
    pub failure: Option<Failure>,
    pub status: LifecycleStatus,
    pub stage: String,
    pub step: usize,
    pub steps: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_spec_hash: Option<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Default for EnvironmentState {
    fn default() -> Self {
        Self {
            format: 1,
            active: None,
            pending: None,
            previous: None,
            failure: None,
            status: LifecycleStatus::Unavailable,
            stage: "idle".into(),
            step: 0,
            steps: 0,
            job_id: None,
            requested_spec_hash: None,
            extra: OpaqueObject::new(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentPhase {
    NotInstalled,
    Installing,
    Building,
    Ready,
    NeedsRestart,
    Failed,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentStatus {
    #[serde(flatten)]
    pub state: EnvironmentState,
    pub running_processes: usize,
    pub environment_available: bool,
    pub usable: bool,
    pub phase: EnvironmentPhase,
    pub progress: Option<f64>,
    pub error: Option<String>,
    pub architecture: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ImageIndex {
    pub sha256: String,
    pub metadata: ImageMetadata,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageMetadata {
    pub format: String,
    pub format_version: u64,
    pub profile: String,
    #[serde(rename = "type")]
    pub image_type: String,
    pub type_version: u64,
    pub defaults: LanguageDefaults,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub stores: BTreeMap<String, SeedStore>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct LanguageDefaults {
    pub python: VersionDefaults,
    pub node: VersionDefaults,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub(crate) enum VersionDefaults {
    One(String),
    Many(Vec<String>),
}
impl VersionDefaults {
    pub(crate) fn versions(&self) -> Vec<String> {
        match self {
            Self::One(v) => vec![v.clone()],
            Self::Many(v) => v.clone(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub(crate) struct SeedStore {
    pub store: String,
    pub seed: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}

// Missing declarations inherit image defaults; an explicit null is not an array/object.
fn present<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}
