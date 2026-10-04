//! Workspace-delivered configuration, validation and patch policy.
//! Domain sections are generic to avoid depending on Chat or Terminal.
use crate::{EnvironmentSpec, Error, Result, Store, json::OpaqueObject, store::keys::CONFIG};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub trait ConfigSection: Default {
    type Patch: Default;
    fn validate(&self) -> Result<()>;
    fn apply(&mut self, patch: Self::Patch) -> Result<()>;
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub theme: String,
    pub density: String,
    pub font_scale: f64,
    pub mono_font_size: f64,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            density: "compact".into(),
            font_scale: 1.0,
            mono_font_size: 13.0,
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct OverlayConfig {
    pub enabled: bool,
    pub extra_apps: Vec<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "ClientConfig",
    bound = "A: JsonSchema + Default + Serialize, T: JsonSchema + Default + Serialize"
)]
#[serde(bound(deserialize = "A: Deserialize<'de> + Default, T: Deserialize<'de> + Default"))]
pub struct ClientConfig<A: Default, T: Default> {
    pub version: u64,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub agent: A,
    #[serde(default)]
    pub overlay: OverlayConfig,
    #[serde(default = "default_launcher")]
    pub launcher: Vec<String>,
    #[serde(default)]
    pub terminal: T,
    /// The environment declaration. Environment builds from this section alone, so changing the
    /// rest of the file never rebuilds the environment; an absent section declares the image
    /// defaults and stays absent when other settings are saved. Its semantic validation belongs
    /// to the build and to explicit saves, so a bad declaration never blocks unrelated settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Box<EnvironmentSpec>>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
/// A new workspace spells out the empty lists so the declaration is discoverable when edited.
fn starter_environment() -> EnvironmentSpec {
    EnvironmentSpec {
        packages: Some(vec![]),
        env: Some(Default::default()),
        post_scripts: Some(vec![]),
        ..Default::default()
    }
}
fn default_launcher() -> Vec<String> {
    vec!["workbench".into(), "proxy".into(), "settings".into()]
}
impl<A: Default, T: Default> Default for ClientConfig<A, T> {
    fn default() -> Self {
        Self {
            version: 2,
            appearance: Default::default(),
            agent: Default::default(),
            overlay: Default::default(),
            launcher: default_launcher(),
            terminal: Default::default(),
            environment: Some(Box::new(starter_environment())),
            extra: Default::default(),
        }
    }
}
impl<A: ConfigSection, T: ConfigSection> ClientConfig<A, T> {
    pub fn validate(&mut self) -> Result<()> {
        if self.version != 2 {
            return Err(Error::invalid("config.version must be 2"));
        }
        if !matches!(self.appearance.theme.as_str(), "system" | "light" | "dark") {
            return Err(Error::invalid("appearance.theme: invalid value"));
        }
        if !matches!(self.appearance.density.as_str(), "compact" | "standard") {
            return Err(Error::invalid("appearance.density: invalid value"));
        }
        if !(0.8..=1.3).contains(&self.appearance.font_scale) {
            return Err(Error::invalid("appearance.fontScale: out of range"));
        }
        if !(10.0..=20.0).contains(&self.appearance.mono_font_size) {
            return Err(Error::invalid("appearance.monoFontSize: out of range"));
        }
        self.agent.validate()?;
        self.terminal.validate()?;
        if !self.overlay.extra_apps.iter().all(|id| app_ref(id)) {
            return Err(Error::invalid("overlay.extraApps: invalid application"));
        }
        if !self
            .launcher
            .iter()
            .all(|id| matches!(id.as_str(), "workbench" | "proxy" | "settings") || app_ref(id))
        {
            return Err(Error::invalid("launcher: invalid application"));
        }
        let mut entries = vec![];
        for id in &self.launcher {
            if !entries.contains(id) {
                entries.push(id.clone())
            }
        }
        if !entries.iter().any(|id| id == "workbench") {
            entries.insert(0, "workbench".into())
        }
        for required in ["proxy", "settings"] {
            if !entries.iter().any(|id| id == required) {
                entries.push(required.into())
            }
        }
        self.launcher = entries;
        Ok(())
    }
    pub fn patch(&mut self, p: ConfigPatch<A::Patch, T::Patch>) -> Result<()> {
        p.version.assign(&mut self.version);
        if let FieldPatch::Value(p) = p.appearance {
            p.theme.assign(&mut self.appearance.theme);
            p.density.assign(&mut self.appearance.density);
            p.font_scale.assign(&mut self.appearance.font_scale);
            p.mono_font_size.assign(&mut self.appearance.mono_font_size);
            merge_extra(&mut self.appearance.extra, p.extra)
        }
        if let FieldPatch::Value(p) = p.agent {
            self.agent.apply(p)?;
        }
        if let FieldPatch::Value(p) = p.overlay {
            p.enabled.assign(&mut self.overlay.enabled);
            p.extra_apps.assign(&mut self.overlay.extra_apps);
            merge_extra(&mut self.overlay.extra, p.extra)
        }
        p.launcher.assign(&mut self.launcher);
        if let FieldPatch::Value(p) = p.terminal {
            self.terminal.apply(p)?;
        }
        if let FieldPatch::Value(environment) = p.environment {
            self.environment = Some(Box::new(environment));
        }
        merge_extra(&mut self.extra, p.extra);
        self.validate()
    }
}
fn app_ref(id: &str) -> bool {
    let mut parts = id.split('/');
    let pkg = parts.next().unwrap_or("");
    let activity = parts.next();
    parts.next().is_none()
        && pkg.contains('.')
        && pkg
            .split('.')
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
        && activity.is_none_or(|s| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'$'))
        })
}
/// Unknown extension objects merge recursively; known configuration fields never enter this path.
pub fn merge_extra(to: &mut OpaqueObject, from: OpaqueObject) {
    for (k, v) in from {
        if let Some(old) = to.get_mut(&k) {
            old.merge(v)
        } else {
            to.insert(k, v);
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum FieldPatch<T> {
    Value(T),
    Missing,
}
impl<T: Serialize> Serialize for FieldPatch<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Value(v) => v.serialize(s),
            Self::Missing => s.serialize_unit(),
        }
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for FieldPatch<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        T::deserialize(d).map(Self::Value)
    }
}
impl<T: JsonSchema> JsonSchema for FieldPatch<T> {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        T::schema_name()
    }
    fn json_schema(g: &mut schemars::SchemaGenerator) -> schemars::Schema {
        T::json_schema(g)
    }
}
impl<T> Default for FieldPatch<T> {
    fn default() -> Self {
        Self::Missing
    }
}
impl<T> FieldPatch<T> {
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
    pub fn assign(self, to: &mut T) {
        if let Self::Value(v) = self {
            *to = v
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct AppearancePatch {
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub theme: FieldPatch<String>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub density: FieldPatch<String>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub font_scale: FieldPatch<f64>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub mono_font_size: FieldPatch<f64>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct OverlayPatch {
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub enabled: FieldPatch<bool>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub extra_apps: FieldPatch<Vec<String>>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
#[schemars(rename = "ConfigPatch")]
#[serde(bound(deserialize = "A: Deserialize<'de> + Default, T: Deserialize<'de> + Default"))]
pub struct ConfigPatch<A: Default, T: Default> {
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub version: FieldPatch<u64>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub appearance: FieldPatch<AppearancePatch>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub agent: FieldPatch<A>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub overlay: FieldPatch<OverlayPatch>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub launcher: FieldPatch<Vec<String>>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub terminal: FieldPatch<T>,
    /// Replaces the whole declaration; its lists have no element-wise merge.
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub environment: FieldPatch<EnvironmentSpec>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}

/// Read the authoritative file without replacing malformed or newer documents.
pub fn load<A, T>(store: &Store) -> Result<ClientConfig<A, T>>
where
    A: ConfigSection,
    T: ConfigSection,
    A::Patch: DeserializeOwned,
    T::Patch: DeserializeOwned,
{
    let patch: ConfigPatch<A::Patch, T::Patch> = store
        .read(CONFIG)?
        .ok_or_else(|| Error::invalid("configuration is missing"))?;
    if !matches!(patch.version, FieldPatch::Value(2)) {
        return Err(Error::invalid("config.version must be 2"));
    }
    // Only a new workspace starts from the written-out declaration; a file without the section
    // keeps it absent.
    let mut config = ClientConfig {
        environment: None,
        ..ClientConfig::default()
    };
    config.patch(patch)?;
    Ok(config)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[schemars(
    rename = "ConfigOutcome",
    bound = "A: JsonSchema + Default + Serialize, T: JsonSchema + Default + Serialize"
)]
#[serde(bound(deserialize = "A: Deserialize<'de> + Default, T: Deserialize<'de> + Default"))]
pub enum ConfigOutcome<A: Default, T: Default> {
    Updated { config: ClientConfig<A, T> },
    Conflict { revision: u64 },
    Blocked { problem: String },
    Failed { message: String },
}
pub struct ConfigChange<A: Default, T: Default> {
    pub outcome: ConfigOutcome<A, T>,
    pub reloaded: bool,
}
/// Configuration conflict and mutation policy. The Server supplies its transaction
/// revision, then refreshes its discardable aggregate projection when requested.
pub fn update<A, T>(
    store: &Store,
    observed: &ClientConfig<A, T>,
    revision: u64,
    expected_revision: u64,
    patch: ConfigPatch<A::Patch, T::Patch>,
) -> Result<ConfigChange<A, T>>
where
    A: ConfigSection + Clone + PartialEq + Serialize,
    T: ConfigSection + Clone + PartialEq + Serialize,
    A::Patch: DeserializeOwned,
    T::Patch: DeserializeOwned,
{
    if expected_revision != revision {
        return Ok(ConfigChange {
            outcome: ConfigOutcome::Conflict { revision },
            reloaded: false,
        });
    }
    let mut current = match load::<A, T>(store) {
        Ok(config) => config,
        Err(error) => {
            return Ok(ConfigChange {
                outcome: ConfigOutcome::Blocked {
                    problem: error.message,
                },
                reloaded: true,
            });
        }
    };
    if &current != observed {
        return Ok(ConfigChange {
            outcome: ConfigOutcome::Conflict {
                revision: revision
                    .checked_add(1)
                    .ok_or_else(|| Error::business("overflow", "revision overflow"))?,
            },
            reloaded: true,
        });
    }
    let outcome = match current.patch(patch) {
        Ok(()) => {
            store.write(CONFIG, &current)?;
            ConfigOutcome::Updated { config: current }
        }
        Err(error) => ConfigOutcome::Failed {
            message: error.message,
        },
    };
    Ok(ConfigChange {
        outcome,
        reloaded: true,
    })
}
