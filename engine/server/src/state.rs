//! Server transaction composition. Domain state is typed; wire envelopes remain here.
use base64::{Engine, engine::general_purpose::STANDARD};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use workflow_environment::{
    error::{Error, Result},
    json::{OpaqueJson, OpaqueObject, strict_json},
    persist::now,
    store::{
        Store,
        keys::{CONFIG, DECLARATION, WORKSPACE_STATE},
    },
};
use workflow_filework::{
    self as filework, ArchiveDecision, ArchiveOutcome, ComposerDraft, ConflictResolution,
    FileOperation, FileOutcome, FileVersion, FileWork, FileWorkState,
};
use workflow_workspace::{
    self as workspace, LayoutAction, ResourceRef, Target, Workbench, WorkspaceState,
};

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
#[serde(default)]
pub struct BackendDefaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentConfig {
    pub backend: String,
    pub backends: BTreeMap<String, BackendDefaults>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            backend: "codex".into(),
            backends: Default::default(),
            permissions: None,
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
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TerminalConfig {
    pub extra_keys_pinned: bool,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClientConfig {
    pub version: u64,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub overlay: OverlayConfig,
    #[serde(default = "default_launcher")]
    pub launcher: Vec<String>,
    #[serde(default)]
    pub terminal: TerminalConfig,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
fn default_launcher() -> Vec<String> {
    vec!["workbench".into(), "proxy".into(), "settings".into()]
}
impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            version: 2,
            appearance: Default::default(),
            agent: Default::default(),
            overlay: Default::default(),
            launcher: default_launcher(),
            terminal: Default::default(),
            extra: Default::default(),
        }
    }
}
impl ClientConfig {
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
        if self.agent.backend.trim().is_empty() {
            return Err(Error::invalid("agent.backend: expected nonempty string"));
        }
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
    pub fn patch(&mut self, p: ConfigPatch) -> Result<()> {
        p.version.assign(&mut self.version);
        if let FieldPatch::Value(p) = p.appearance {
            p.theme.assign(&mut self.appearance.theme);
            p.density.assign(&mut self.appearance.density);
            p.font_scale.assign(&mut self.appearance.font_scale);
            p.mono_font_size.assign(&mut self.appearance.mono_font_size);
            merge_extra(&mut self.appearance.extra, p.extra)
        }
        if let FieldPatch::Value(p) = p.agent {
            p.backend.assign(&mut self.agent.backend);
            p.permissions.assign(&mut self.agent.permissions);
            if let FieldPatch::Value(backends) = p.backends {
                for (id, p) in backends {
                    let b = self.agent.backends.entry(id).or_default();
                    p.model.assign(&mut b.model);
                    p.effort.assign(&mut b.effort);
                    merge_extra(&mut b.extra, p.extra)
                }
            }
            merge_extra(&mut self.agent.extra, p.extra)
        }
        if let FieldPatch::Value(p) = p.overlay {
            p.enabled.assign(&mut self.overlay.enabled);
            p.extra_apps.assign(&mut self.overlay.extra_apps);
            merge_extra(&mut self.overlay.extra, p.extra)
        }
        p.launcher.assign(&mut self.launcher);
        if let FieldPatch::Value(p) = p.terminal {
            p.extra_keys_pinned
                .assign(&mut self.terminal.extra_keys_pinned);
            merge_extra(&mut self.terminal.extra, p.extra)
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
fn merge_extra(to: &mut OpaqueObject, from: OpaqueObject) {
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
    fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
    fn assign(self, to: &mut T) {
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
#[serde(default)]
pub struct BackendPatch {
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub model: FieldPatch<Option<String>>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub effort: FieldPatch<Option<String>>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AgentPatch {
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub backend: FieldPatch<String>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub backends: FieldPatch<BTreeMap<String, BackendPatch>>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub permissions: FieldPatch<Option<String>>,
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
#[serde(default, rename_all = "camelCase")]
pub struct TerminalPatch {
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub extra_keys_pinned: FieldPatch<bool>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ConfigPatch {
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub version: FieldPatch<u64>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub appearance: FieldPatch<AppearancePatch>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub agent: FieldPatch<AgentPatch>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub overlay: FieldPatch<OverlayPatch>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub launcher: FieldPatch<Vec<String>>,
    #[serde(skip_serializing_if = "FieldPatch::is_missing")]
    pub terminal: FieldPatch<TerminalPatch>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Notice {
    pub id: String,
    pub kind: String,
    pub message: String,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub status: String,
    pub failure: Option<String>,
    #[serde(flatten)]
    pub workspace: WorkspaceState,
    #[serde(flatten)]
    pub files: FileWorkState,
    pub config: ClientConfig,
    pub config_problem: Option<String>,
    pub notices: Vec<Notice>,
    pub write_error: Option<String>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
impl Default for State {
    fn default() -> Self {
        Self {
            status: "ready".into(),
            failure: None,
            workspace: Default::default(),
            files: Default::default(),
            config: Default::default(),
            config_problem: None,
            notices: vec![],
            write_error: None,
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StateDocument {
    pub format: u64,
    pub revision: u64,
    pub state: State,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Snapshot {
    pub revision: u64,
    pub state: State,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CommandReply {
    pub revision: u64,
    pub state: State,
    pub value: CommandValue,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CommandValue {
    Null(()),
    Boolean(bool),
    Text(String),
    Count(usize),
    Workbench(Workbench),
    OpenFile(filework::OpenFile),
    Save(filework::SaveOutcome),
    Archive(ArchiveOutcome),
    File(FileOutcome),
    Entries(Vec<filework::DirectoryEntry>),
    Composer(ComposerDraft),
    Config(ConfigOutcome),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConfigOutcome {
    Updated { config: ClientConfig },
    Conflict { revision: u64 },
    Blocked { problem: String },
    Failed { message: String },
}
#[derive(Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub revision: u64,
    pub state: State,
    pub writable: bool,
    pub store: Store,
    pub filework: FileWork,
    document_extra: OpaqueObject,
}
#[derive(Deserialize)]
struct FormatProbe {
    format: u64,
}
#[derive(Deserialize)]
struct ResourceFormats {
    state: FormatState,
}
#[derive(Deserialize)]
struct FormatState {
    drafts: BTreeMap<String, FormatProbe>,
    composers: BTreeMap<String, FormatProbe>,
}
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
#[serde(
    tag = "name",
    content = "args",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    EnterWorkbench {},
    CreateSession {
        #[serde(default)]
        name: Option<String>,
    },
    OpenInSeparateSession {
        target: Target,
    },
    ActivateSession {
        id: String,
    },
    RestoreSession {
        id: String,
    },
    RenameSession {
        id: String,
        name: String,
    },
    ArchiveSession {
        id: String,
        #[serde(default)]
        decision: Option<ArchiveDecision>,
    },
    PinSession {
        id: String,
        #[serde(default)]
        index: Option<i64>,
    },
    UnpinSession {
        id: String,
    },
    RunMaintenance {},
    Flush {},
    DismissNotice {
        id: String,
    },
    ApplyLayout {
        #[serde(default)]
        session_id: Option<String>,
        op: LayoutAction,
    },
    OpenFile {
        path: String,
    },
    EditFile {
        path: String,
        text: String,
        shown: FileVersion,
    },
    SaveFile {
        path: String,
        #[serde(default)]
        text: Option<String>,
    },
    ResolveConflict {
        path: String,
        resolution: ConflictResolution,
    },
    DiscardDraft {
        path: String,
    },
    DeletePath {
        path: String,
    },
    TrashPath {
        path: String,
    },
    CreateDirectory {
        path: String,
    },
    MovePath {
        from: String,
        to: String,
    },
    CopyPath {
        from: String,
        to: String,
    },
    RestoreFromTrash {
        id: String,
    },
    PurgeTrash {},
    CreateFile {
        path: String,
        #[serde(default)]
        data: String,
    },
    ListDirectory {
        path: String,
        #[serde(default)]
        show_hidden: bool,
    },
    EditComposer {
        draft: ComposerDraft,
        expected_revision: u64,
    },
    AcknowledgeComposer {
        submitted: ComposerDraft,
    },
    DiscardComposer {
        conversation_id: String,
    },
    RemoveConversation {
        conversation_id: String,
    },
    UpdateConfig {
        config: ConfigPatch,
        expected_revision: u64,
    },
}
impl<'de> Deserialize<'de> for Command {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Header {
            name: String,
            #[serde(default)]
            args: OpaqueObject,
        }
        let header = Header::deserialize(d)?;
        let bytes = serde_json::to_vec(&header.args).map_err(serde::de::Error::custom)?;
        macro_rules! command {($variant:ident{$($(#[$meta:meta])* $field:ident:$kind:ty),*$(,)?})=>{{
            #[derive(Deserialize)]#[serde(rename_all="camelCase")]struct Args{$($(#[$meta])* $field:$kind),*}
            let a:Args=strict_json(&bytes).map_err(serde::de::Error::custom)?;
            Self::$variant{$($field:a.$field),*}
        }}}
        Ok(match header.name.as_str() {
            "enterWorkbench" => Self::EnterWorkbench {},
            "createSession" => command!(CreateSession{name:Option<String>}),
            "openInSeparateSession" => command!(OpenInSeparateSession { target: Target }),
            "activateSession" => command!(ActivateSession { id: String }),
            "restoreSession" => command!(RestoreSession { id: String }),
            "renameSession" => command!(RenameSession {
                id: String,
                name: String
            }),
            "archiveSession" => {
                command!(ArchiveSession{id:String,decision:Option<ArchiveDecision>})
            }
            "pinSession" => command!(PinSession{id:String,index:Option<i64>}),
            "unpinSession" => command!(UnpinSession { id: String }),
            "runMaintenance" => Self::RunMaintenance {},
            "flush" => Self::Flush {},
            "dismissNotice" => command!(DismissNotice { id: String }),
            "applyLayout" => command!(ApplyLayout{session_id:Option<String>,op:LayoutAction}),
            "openFile" => command!(OpenFile { path: String }),
            "editFile" => command!(EditFile {
                path: String,
                text: String,
                shown: FileVersion
            }),
            "saveFile" => command!(SaveFile{path:String,text:Option<String>}),
            "resolveConflict" => command!(ResolveConflict {
                path: String,
                resolution: ConflictResolution
            }),
            "discardDraft" => command!(DiscardDraft { path: String }),
            "deletePath" => command!(DeletePath { path: String }),
            "trashPath" => command!(TrashPath { path: String }),
            "createDirectory" => command!(CreateDirectory { path: String }),
            "movePath" => command!(MovePath {
                from: String,
                to: String
            }),
            "copyPath" => command!(CopyPath {
                from: String,
                to: String
            }),
            "restoreFromTrash" => command!(RestoreFromTrash { id: String }),
            "purgeTrash" => Self::PurgeTrash {},
            "createFile" => command!(CreateFile {
                path: String,
                #[serde(default)]
                data: String
            }),
            "listDirectory" => command!(ListDirectory {
                path: String,
                #[serde(default)]
                show_hidden: bool
            }),
            "editComposer" => command!(EditComposer {
                draft: ComposerDraft,
                expected_revision: u64
            }),
            "acknowledgeComposer" => command!(AcknowledgeComposer {
                submitted: ComposerDraft
            }),
            "discardComposer" => command!(DiscardComposer {
                conversation_id: String
            }),
            "removeConversation" => command!(RemoveConversation {
                conversation_id: String
            }),
            "updateConfig" => command!(UpdateConfig {
                config: ConfigPatch,
                expected_revision: u64
            }),
            _ => return Err(serde::de::Error::custom("unknown workspace command")),
        })
    }
}
impl Command {
    #[cfg(test)]
    pub fn decode<A: Serialize>(name: &str, args: &A) -> Result<Self> {
        #[derive(Serialize)]
        struct Input<'a, A> {
            name: &'a str,
            args: &'a A,
        }
        let bytes = serde_json::to_vec(&Input { name, args })
            .map_err(|e| Error::invalid(&e.to_string()))?;
        strict_json(&bytes).map_err(|e| Error::invalid(&e.to_string()))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClientConfiguration {
    pub appearance: Appearance,
    pub overlay: OverlayConfig,
    pub launcher: Vec<String>,
    pub terminal: TerminalConfig,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClientConfigReply {
    pub revision: u64,
    pub config: ClientConfiguration,
}
impl Workspace {
    pub fn load(root: PathBuf) -> Result<Self> {
        let store = Store::open(&root)?;
        for key in store.list("uploads")? {
            if store.metadata(&key)?.is_some_and(|m| m.is_file) {
                let _ = store.remove(&key);
            }
        }
        let mut out = Self {
            filework: FileWork::new(root.clone(), store.clone()),
            root,
            revision: 0,
            state: State::default(),
            writable: true,
            store,
            document_extra: Default::default(),
        };
        if out.store.exists(WORKSPACE_STATE)? {
            match out.store.read::<FormatProbe>(WORKSPACE_STATE) {
                Ok(Some(p)) if p.format != 1 => {
                    out.read_only("Unsupported workspace state format; source preserved read-only");
                    out.notice(
                        "newer_format",
                        "Unsupported workspace state format; source preserved",
                    );
                    return Ok(out);
                }
                Ok(Some(_)) => match out.read_state(WORKSPACE_STATE) {
                    Ok(Some(v)) => out.accept_document(v),
                    Err(e) if e.kind == "unsupported_format" => {
                        out.read_only(&e.message);
                        return Ok(out);
                    }
                    _ => out.recover()?,
                },
                _ => out.recover()?,
            }
        }
        if !out.store.exists(CONFIG)? {
            out.store.write(CONFIG, &out.state.config)?;
        }
        out.reload_config();
        if !out.store.exists(DECLARATION)? {
            out.store.write(
                DECLARATION,
                &workflow_environment::EnvironmentSpec {
                    packages: Some(vec![]),
                    env: Some(Default::default()),
                    post_scripts: Some(vec![]),
                    ..Default::default()
                },
            )?;
        }
        out.maintenance()?;
        out.persist()?;
        Ok(out)
    }
    fn read_only(&mut self, message: &str) {
        self.writable = false;
        self.state.status = "failed".into();
        self.state.failure = Some(message.into())
    }
    fn read_state(&self, key: &str) -> Result<Option<StateDocument>> {
        let Some(formats) = self.store.read::<ResourceFormats>(key)? else {
            return Ok(None);
        };
        if formats.state.drafts.values().any(|p| p.format != 1) {
            return Err(Error::business(
                "unsupported_format",
                "unknown draft format; source preserved read-only",
            ));
        }
        if formats.state.composers.values().any(|p| p.format != 1) {
            return Err(Error::business(
                "unsupported_format",
                "unknown composer format; source preserved read-only",
            ));
        }
        let Some(mut v) = self.store.read::<StateDocument>(key)? else {
            return Ok(None);
        };
        if v.format != 1 {
            return Err(Error::invalid("invalid state envelope"));
        }
        v.state.workspace.validate()?;
        v.state.files.validate()?;
        Ok(Some(v))
    }
    fn accept_document(&mut self, v: StateDocument) {
        self.revision = v.revision;
        self.state = v.state;
        self.document_extra = v.extra;
        self.state.status = "ready".into();
        self.state.failure = None;
    }
    fn recover(&mut self) -> Result<()> {
        self.store.quarantine(WORKSPACE_STATE)?;
        if let Ok(Some(v)) = self.read_state("state/workspace.json.bak") {
            self.accept_document(v)
        }
        self.notice(
            "recovered",
            "Damaged workspace state preserved; latest valid backup restored when available",
        );
        Ok(())
    }
    pub fn notice(&mut self, kind: &str, message: &str) {
        self.state.notices.push(Notice {
            id: uuid::Uuid::new_v4().to_string(),
            kind: kind.into(),
            message: message.into(),
            extra: Default::default(),
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: self.revision,
            state: self.state.clone(),
        }
    }
    pub fn client_config(&self) -> ClientConfigReply {
        let c = &self.state.config;
        ClientConfigReply {
            revision: self.revision,
            config: ClientConfiguration {
                appearance: c.appearance.clone(),
                overlay: c.overlay.clone(),
                launcher: c.launcher.clone(),
                terminal: c.terminal.clone(),
            },
        }
    }
    pub fn services_config(&self) -> Option<OpaqueJson> {
        self.state.config.extra.get("services").cloned()
    }
    pub fn terminal_references(&self) -> std::collections::HashSet<String> {
        self.state.workspace.terminal_references()
    }
    pub fn persist(&self) -> Result<()> {
        if !self.writable {
            return Err(Error::business(
                "read_only",
                "workspace format is unsupported",
            ));
        }
        let document = StateDocument {
            format: 1,
            revision: self.revision,
            state: self.state.clone(),
            extra: self.document_extra.clone(),
        };
        let bytes = serde_json::to_vec(&document)
            .map_err(|e| Error::business("serialize", &e.to_string()))?;
        if bytes.len() > 24 * 1024 * 1024 {
            return Err(Error::business(
                "state_limit",
                "workspace state exceeds 24 MiB; save or discard drafts",
            ));
        }
        self.store.backup(WORKSPACE_STATE)?;
        self.store.write(WORKSPACE_STATE, &document)
    }
    pub fn commit(&mut self, mut candidate: Self) -> Result<()> {
        candidate.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::business("overflow", "revision overflow"))?;
        candidate.state.write_error = None;
        if let Err(e) = candidate.persist() {
            self.state.write_error = Some(e.message.clone());
            return Err(e);
        }
        *self = candidate;
        Ok(())
    }
    #[cfg(test)]
    pub fn command<A: Serialize>(&mut self, name: &str, args: &A) -> Result<CommandReply> {
        self.command_typed(Command::decode(name, args)?)
    }
    pub fn command_typed(&mut self, command: Command) -> Result<CommandReply> {
        if !self.writable {
            return Err(Error::business("read_only", "workspace is read only"));
        }
        let flush = matches!(command, Command::Flush { .. });
        let mut next = self.clone();
        let value = next.execute(command)?;
        if next.state != self.state || flush {
            self.commit(next)?
        }
        Ok(CommandReply {
            revision: self.revision,
            state: self.state.clone(),
            value,
        })
    }
    pub fn reload_config(&mut self) {
        match self.store.read::<ConfigPatch>(CONFIG).and_then(|p| {
            let Some(p) = p else {
                return Err(Error::invalid("configuration is missing"));
            };
            if !matches!(p.version, FieldPatch::Value(2)) {
                return Err(Error::invalid("config.version must be 2"));
            }
            let mut config = ClientConfig::default();
            config.patch(p)?;
            Ok(config)
        }) {
            Ok(config) => {
                self.state.config = config;
                self.state.config_problem = None
            }
            Err(e) => self.state.config_problem = Some(e.message),
        }
        if let Ok(v) = self.filework.version(".workspace/config.json") {
            self.state
                .files
                .disk
                .insert(".workspace/config.json".into(), v);
        }
    }
    pub fn refresh(&mut self) -> Result<()> {
        let paths = self
            .state
            .workspace
            .sessions
            .iter()
            .filter(|s| s.archived_at.is_none())
            .flat_map(|s| s.resources())
            .filter_map(|r| match r {
                ResourceRef::File { path } => Some(path),
                _ => None,
            });
        self.filework.refresh(&mut self.state.files, paths)?;
        self.reload_config();
        Ok(())
    }
    pub fn maintenance(&mut self) -> Result<()> {
        let candidates: Vec<_> = self
            .state
            .workspace
            .sessions
            .iter()
            .filter(|s| s.archived_at.is_none())
            .map(|s| filework::ArchiveCandidate {
                id: s.id.clone(),
                last_used_at: s.last_used_at,
                created_at: s.created_at,
                resources: s.resources().into_iter().map(file_resource).collect(),
            })
            .collect();
        let protected = filework::protected_sessions(&self.state.files, &candidates);
        self.state.workspace.maintain(&protected, now());
        self.refresh()?;
        self.filework.purge_trash()?;
        Ok(())
    }
    fn archive(&mut self, id: &str, decision: Option<ArchiveDecision>) -> Result<ArchiveOutcome> {
        let Some(s) = self.state.workspace.session(id) else {
            return Ok(ArchiveOutcome::NotFound);
        };
        if s.archived_at.is_some() {
            return Ok(ArchiveOutcome::Archived {
                saved_paths: vec![],
            });
        }
        let refs = s
            .resources()
            .into_iter()
            .map(file_resource)
            .collect::<Vec<_>>();
        let result =
            self.filework
                .archive(&mut self.state.files, &refs, decision, validate_text)?;
        if let ArchiveOutcome::Archived { saved_paths } = &result {
            self.state.workspace.archive(id, now());
            if saved_paths.iter().any(|p| p == ".workspace/config.json") {
                self.reload_config();
            }
        }
        Ok(result)
    }
    fn file_operation(&mut self, op: FileOperation) -> Result<FileOutcome> {
        let result = self.filework.file_operation(&mut self.state.files, &op);
        if !matches!(result, FileOutcome::Failed { .. }) {
            match &op {
                FileOperation::Delete { path } | FileOperation::Trash { path } => self
                    .state
                    .workspace
                    .after_removal(path, &self.state.files.draft_paths()),
                FileOperation::Move { from, to } => self.state.workspace.rename_path(from, to),
                _ => {}
            }
            if !matches!(
                op,
                FileOperation::CreateDirectory { .. } | FileOperation::Copy { .. }
            ) {
                self.refresh()?;
            }
        }
        Ok(result)
    }
    fn execute(&mut self, command: Command) -> Result<CommandValue> {
        use CommandValue as V;
        Ok(match command {
            Command::EnterWorkbench {} => V::Text(self.state.workspace.enter_workbench()?),
            Command::CreateSession { name } => V::Text(
                self.state
                    .workspace
                    .create_session(name.as_deref(), Workbench::default())?,
            ),
            Command::OpenInSeparateSession { target } => {
                if !workspace::layout::valid_target(&target) {
                    return Err(Error::invalid("invalid target"));
                }
                let w = workspace::layout::apply(
                    &Workbench::default(),
                    &LayoutAction::EnterSolo { target },
                );
                V::Text(self.state.workspace.create_session(None, w)?)
            }
            Command::ActivateSession { id } | Command::RestoreSession { id } => {
                V::Boolean(self.state.workspace.activate(&id))
            }
            Command::RenameSession { id, name } => {
                V::Boolean(self.state.workspace.rename(&id, &name)?)
            }
            Command::ArchiveSession { id, decision } => V::Archive(self.archive(&id, decision)?),
            Command::PinSession { id, index } => {
                V::Boolean(self.state.workspace.pin(&id, index, true))
            }
            Command::UnpinSession { id } => V::Boolean(self.state.workspace.pin(&id, None, false)),
            Command::RunMaintenance {} => {
                self.maintenance()?;
                V::Null(())
            }
            Command::Flush {} => V::Boolean(true),
            Command::DismissNotice { id } => {
                self.state.notices.retain(|n| n.id != id);
                V::Null(())
            }
            Command::ApplyLayout { session_id, op } => self
                .state
                .workspace
                .apply_layout(session_id.as_deref(), &op)
                .map(V::Workbench)
                .unwrap_or(V::Null(())),
            Command::OpenFile { path } => {
                V::OpenFile(self.filework.open(&mut self.state.files, &path)?)
            }
            Command::EditFile { path, text, shown } => {
                let changed = self
                    .state
                    .files
                    .drafts
                    .get(&path)
                    .is_none_or(|d| d.text != text);
                self.filework
                    .edit(&mut self.state.files, &path, &text, &shown)?;
                if changed {
                    self.state
                        .workspace
                        .touch_resource(&ResourceRef::File { path });
                }
                V::Null(())
            }
            Command::SaveFile { path, text } => {
                let changed = text.as_ref().is_some_and(|text| {
                    self.state
                        .files
                        .drafts
                        .get(&path)
                        .is_none_or(|d| d.text != *text)
                });
                let outcome = self.filework.save(
                    &mut self.state.files,
                    &path,
                    text.as_deref(),
                    validate_text,
                )?;
                if changed {
                    self.state
                        .workspace
                        .touch_resource(&ResourceRef::File { path: path.clone() })
                }
                if path == ".workspace/config.json"
                    && matches!(outcome, filework::SaveOutcome::Saved { .. })
                {
                    self.reload_config()
                }
                V::Save(outcome)
            }
            Command::ResolveConflict { path, resolution } => {
                self.filework
                    .resolve_conflict(&mut self.state.files, &path, resolution)?;
                V::Null(())
            }
            Command::DiscardDraft { path } => {
                self.filework.discard(&mut self.state.files, &path)?;
                V::Null(())
            }
            Command::DeletePath { path } => {
                V::File(self.file_operation(FileOperation::Delete { path })?)
            }
            Command::TrashPath { path } => {
                V::File(self.file_operation(FileOperation::Trash { path })?)
            }
            Command::CreateDirectory { path } => {
                V::File(self.file_operation(FileOperation::CreateDirectory { path })?)
            }
            Command::MovePath { from, to } => {
                V::File(self.file_operation(FileOperation::Move { from, to })?)
            }
            Command::CopyPath { from, to } => {
                V::File(self.file_operation(FileOperation::Copy { from, to })?)
            }
            Command::RestoreFromTrash { id } => {
                V::File(self.file_operation(FileOperation::Restore { id })?)
            }
            Command::PurgeTrash {} => V::Count(self.filework.purge_trash()?),
            Command::CreateFile { path, data } => V::File(match decode_blob(&data, 65536) {
                Ok(data) => self.file_operation(FileOperation::CreateFile { path, data })?,
                Err(e) => FileOutcome::Failed { message: e.message },
            }),
            Command::ListDirectory { path, show_hidden } => {
                V::Entries(self.filework.list_directory(&path, show_hidden)?)
            }
            Command::EditComposer {
                draft,
                expected_revision,
            } => {
                let previous = self.state.files.composer(&draft.conversation_id);
                let draft =
                    self.filework
                        .edit_composer(&mut self.state.files, draft, expected_revision)?;
                if draft != previous {
                    self.state
                        .workspace
                        .touch_resource(&ResourceRef::Conversation {
                            id: draft.conversation_id.clone(),
                        });
                }
                V::Composer(draft)
            }
            Command::AcknowledgeComposer { submitted } => {
                V::Composer(self.state.files.acknowledge_composer(submitted)?)
            }
            Command::DiscardComposer { conversation_id } => {
                V::Composer(self.state.files.clear_composer(&conversation_id)?)
            }
            Command::RemoveConversation { conversation_id } => {
                self.state.files.clear_composer(&conversation_id)?;
                self.state.workspace.remove_conversation(&conversation_id);
                V::Null(())
            }
            Command::UpdateConfig {
                config,
                expected_revision,
            } => V::Config(self.update_config(config, expected_revision)?),
        })
    }
    fn update_config(
        &mut self,
        patch: ConfigPatch,
        expected_revision: u64,
    ) -> Result<ConfigOutcome> {
        if expected_revision != self.revision {
            return Ok(ConfigOutcome::Conflict {
                revision: self.revision,
            });
        }
        let previous = self.state.config.clone();
        self.reload_config();
        if let Some(problem) = &self.state.config_problem {
            return Ok(ConfigOutcome::Blocked {
                problem: problem.clone(),
            });
        }
        if self.state.config != previous {
            return Ok(ConfigOutcome::Conflict {
                revision: self
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| Error::business("overflow", "revision overflow"))?,
            });
        }
        let mut input = self.state.config.clone();
        match input.patch(patch) {
            Ok(()) => {
                self.store.write(CONFIG, &input)?;
                self.reload_config();
                Ok(ConfigOutcome::Updated { config: input })
            }
            Err(e) => Ok(ConfigOutcome::Failed { message: e.message }),
        }
    }
}
fn file_resource(r: ResourceRef) -> filework::ResourceRef {
    match r {
        ResourceRef::File { path } => filework::ResourceRef::File { path },
        ResourceRef::Conversation { id } => filework::ResourceRef::Conversation { id },
    }
}
fn decode_blob(s: &str, max: usize) -> Result<Vec<u8>> {
    if s.len() > max.div_ceil(3) * 4 {
        return Err(Error::invalid("blob exceeds limit"));
    }
    let bytes = STANDARD
        .decode(s)
        .map_err(|_| Error::invalid("invalid base64"))?;
    if bytes.len() > max {
        return Err(Error::invalid("blob exceeds limit"));
    }
    Ok(bytes)
}
fn validate_text(path: &str, text: &str) -> Result<()> {
    if path == ".workspace/config.json" {
        let patch: ConfigPatch =
            strict_json(text.as_bytes()).map_err(|e| Error::invalid(&e.to_string()))?;
        if !matches!(patch.version, FieldPatch::Value(2)) {
            return Err(Error::invalid("config.version must be 2"));
        }
        ClientConfig::default().patch(patch)?;
    }
    if path == ".workspace/env.json" {
        let spec = strict_json(text.as_bytes()).map_err(|e| Error::invalid(&e.to_string()))?;
        workflow_environment::validate(&spec)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Workspace) {
        let d = tempfile::tempdir().unwrap();
        let w = Workspace::load(d.path().to_owned()).unwrap();
        (d, w)
    }
    fn session(w: &mut Workspace) -> String {
        let CommandValue::Text(id) = w
            .command_typed(Command::CreateSession { name: None })
            .unwrap()
            .value
        else {
            panic!()
        };
        id
    }
    #[test]
    fn closes_and_archives_keep_drafts_and_maintenance_protects_newest_session() {
        let (_, mut w) = fixture();
        let a = session(&mut w);
        let b = session(&mut w);
        for id in [&a, &b] {
            w.command_typed(Command::ApplyLayout {
                session_id: Some(id.clone()),
                op: LayoutAction::Open {
                    target: Target::file("dirty"),
                    placement: Default::default(),
                    focus: true,
                },
            })
            .unwrap();
        }
        w.command_typed(Command::EditFile {
            path: "dirty".into(),
            text: "draft".into(),
            shown: Default::default(),
        })
        .unwrap();
        let time = now() - 2 * 86_400_000;
        w.state.workspace.sessions[0].last_used_at = time - 1;
        w.state.workspace.sessions[1].last_used_at = time;
        w.maintenance().unwrap();
        assert!(w.state.workspace.sessions[0].archived_at.is_some());
        assert!(w.state.workspace.sessions[1].archived_at.is_none());
        w.command_typed(Command::ApplyLayout {
            session_id: Some(b),
            op: LayoutAction::Close {
                panel_ids: vec!["p1".into()],
            },
        })
        .unwrap();
        assert_eq!(w.state.files.drafts["dirty"].text, "draft");
    }
    #[test]
    fn corrupted_state_recovers_backup_and_unknown_formats_are_read_only() {
        let (d, mut w) = fixture();
        w.command_typed(Command::CreateSession {
            name: Some("retained".into()),
        })
        .unwrap();
        w.command_typed(Command::Flush {}).unwrap();
        w.store.write_bytes(WORKSPACE_STATE, b"{truncated").unwrap();
        let restored = Workspace::load(d.path().to_owned()).unwrap();
        assert_eq!(
            restored.state.workspace.sessions[0].name.as_deref(),
            Some("retained")
        );
        assert!(!restored.store.list("corrupt").unwrap().is_empty());
        restored
            .store
            .write_bytes(WORKSPACE_STATE, br#"{"format":77}"#)
            .unwrap();
        let mut unknown = Workspace::load(d.path().to_owned()).unwrap();
        assert!(!unknown.writable);
        assert!(
            unknown
                .command_typed(Command::CreateSession { name: None })
                .is_err()
        );
        assert_eq!(
            unknown
                .store
                .read_bytes(WORKSPACE_STATE, 100)
                .unwrap()
                .unwrap(),
            br#"{"format":77}"#
        );
    }
    #[test]
    fn unknown_draft_format_is_detected_before_decoding_new_fields() {
        let (d, w) = fixture();
        w.store.write_bytes(WORKSPACE_STATE,br#"{"format":1,"revision":3,"state":{"drafts":{"a":{"format":2,"path":"a","text":"future"}},"composers":{}}}"#).unwrap();
        let loaded = Workspace::load(d.path().to_owned()).unwrap();
        assert!(!loaded.writable);
        assert_eq!(loaded.state.status, "failed");
        assert!(loaded.store.list("corrupt").unwrap().is_empty());
    }
    #[test]
    fn typed_roundtrip_preserves_unknown_fields_at_every_owned_level() {
        let (d, mut w) = fixture();
        let id = session(&mut w);
        let opaque = OpaqueJson(
            serde_json::from_str(r#"{"nested":{"number":123456789012345678901234567890}}"#)
                .unwrap(),
        );
        w.state.extra.insert("futureState".into(), opaque.clone());
        w.document_extra
            .insert("futureEnvelope".into(), opaque.clone());
        w.state
            .workspace
            .session_mut(&id)
            .unwrap()
            .extra
            .insert("futureSession".into(), opaque.clone());
        w.state
            .config
            .appearance
            .extra
            .insert("futureAppearance".into(), opaque.clone());
        w.store.write(CONFIG, &w.state.config).unwrap();
        w.persist().unwrap();
        let restored = Workspace::load(d.path().to_owned()).unwrap();
        assert_eq!(restored.state.extra.get("futureState"), Some(&opaque));
        assert_eq!(restored.document_extra.get("futureEnvelope"), Some(&opaque));
        assert_eq!(
            restored
                .state
                .workspace
                .session(&id)
                .unwrap()
                .extra
                .get("futureSession"),
            Some(&opaque)
        );
        assert_eq!(
            restored
                .state
                .config
                .appearance
                .extra
                .get("futureAppearance"),
            Some(&opaque)
        );
    }
    #[test]
    fn external_configuration_wins_cas_and_invalid_source_is_preserved() {
        let (_, mut w) = fixture();
        let stale = w.state.config.clone();
        let mut external = stale.clone();
        external.appearance.theme = "dark".into();
        w.store.write(CONFIG, &external).unwrap();
        let config: ConfigPatch =
            serde_json::from_slice(&serde_json::to_vec(&stale).unwrap()).unwrap();
        let reply = w
            .command_typed(Command::UpdateConfig {
                config: config.clone(),
                expected_revision: w.revision,
            })
            .unwrap();
        assert!(matches!(
            reply.value,
            CommandValue::Config(ConfigOutcome::Conflict { .. })
        ));
        assert_eq!(w.state.config.appearance.theme, "dark");
        w.store.write_bytes(CONFIG, b"{invalid").unwrap();
        let reply = w
            .command_typed(Command::UpdateConfig {
                config,
                expected_revision: w.revision,
            })
            .unwrap();
        assert!(matches!(
            reply.value,
            CommandValue::Config(ConfigOutcome::Blocked { .. })
        ));
        assert_eq!(
            w.store.read_text(CONFIG).unwrap().as_deref(),
            Some("{invalid")
        );
    }
    #[test]
    fn command_decode_defaults_composer_format_and_rejects_bad_shapes() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Args {
            draft: DraftInput,
            expected_revision: u64,
        }
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct DraftInput {
            conversation_id: String,
            revision: u64,
            text: String,
            attachments: Vec<filework::Attachment>,
        }
        let c = Command::decode(
            "editComposer",
            &Args {
                draft: DraftInput {
                    conversation_id: "c".into(),
                    revision: 1,
                    text: "x".into(),
                    attachments: vec![],
                },
                expected_revision: 0,
            },
        )
        .unwrap();
        assert!(matches!(
            c,
            Command::EditComposer {
                draft: ComposerDraft { format: 1, .. },
                ..
            }
        ));
        assert!(Command::decode("unknown", &()).is_err());
    }
}
#[cfg(test)]
mod decoding_tests {
    use super::*;
    #[test]
    fn command_numbers_decode_without_tagged_content_rounding() {
        let source=br#"{"name":"applyLayout","args":{"op":{"type":"resizeSplit","splitId":"x1","weights":[1e0,0.2500]}}}"#;
        let c: Command = strict_json(source).unwrap();
        assert!(
            matches!(c,Command::ApplyLayout{op:LayoutAction::ResizeSplit{weights,..},..}if weights==vec![1.0,0.25])
        );
        let c:Command=strict_json(br#"{"name":"updateConfig","args":{"expectedRevision":0,"config":{"appearance":{"fontScale":1e0}}}}"#).unwrap();
        let Command::UpdateConfig { config, .. } = c else {
            panic!()
        };
        let mut current = ClientConfig::default();
        current.patch(config).unwrap();
        assert_eq!(current.appearance.font_scale, 1.0);
    }
}
