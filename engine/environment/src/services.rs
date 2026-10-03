//! Typed Engine-owned service intent and measured executor receipts.
//! Server supplies canonical documents and transaction revision commits through ServicesContext.
use crate::{Error, Result, Store, json::OpaqueObject, persist::now, store::keys};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

const MAX_CONFIG: usize = 16 * 1024 * 1024;
const MAX_REPORT: usize = 1024 * 1024;
static BOOT: OnceLock<String> = OnceLock::new();
fn boot() -> &'static str {
    BOOT.get_or_init(|| uuid::Uuid::new_v4().to_string())
}
fn control_key() -> String {
    keys::service_control("proxy").expect("proxy is a valid service identifier")
}
fn measurement_key() -> String {
    keys::service_report("proxy").expect("proxy is a valid service identifier")
}

/// The Server holds its workspace lock while implementing these operations.
pub trait ServicesContext {
    fn store(&self) -> &Store;
    fn writable(&self) -> bool;
    fn commit(&mut self) -> Result<u64>;
    /// Always addresses a canonical services.proxy document through FileWork.
    fn document(&mut self, request: DocumentRequest) -> Result<Document>;
}
#[derive(Clone, Debug)]
pub enum DocumentRequest {
    Read {
        key: &'static str,
    },
    Write {
        key: &'static str,
        document: String,
        expected_revision: u64,
    },
}
#[derive(Clone, Debug)]
pub struct Document {
    pub document: Option<String>,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    Rule,
    Global,
    Direct,
}
impl ProxyMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Rule => "rule",
            Self::Global => "global",
            Self::Direct => "direct",
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Desired {
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub mode: Option<ProxyMode>,
    #[serde(default)]
    pub selections: BTreeMap<String, String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CommandName {
    Start,
    Stop,
    CheckConfig,
    RefreshProviders,
    SetMode,
    Select,
    TestNode,
    TestGroup,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum OperationStatus {
    Pending,
    Completed,
    Failed,
    Interrupted,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub name: CommandName,
    pub status: OperationStatus,
    pub epoch: String,
    pub requested_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Executor {
    id: String,
    epoch: String,
    boot: String,
    #[serde(flatten)]
    extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlState {
    format: u32,
    desired: Desired,
    #[serde(default)]
    executor: Option<Executor>,
    #[serde(default)]
    operation: Option<Operation>,
    #[serde(default)]
    measured_epoch: Option<String>,
    #[serde(flatten)]
    extra: OpaqueObject,
}
impl Default for ControlState {
    fn default() -> Self {
        Self {
            format: 1,
            desired: Desired::default(),
            executor: None,
            operation: None,
            measured_epoch: None,
            extra: OpaqueObject::default(),
        }
    }
}
impl ControlState {
    fn active(&self) -> bool {
        self.executor
            .as_ref()
            .is_some_and(|executor| executor.boot == boot())
    }
    fn validate(&self, epoch: &str) -> Result<()> {
        if !self.active()
            || self
                .executor
                .as_ref()
                .map(|executor| executor.epoch.as_str())
                != Some(epoch)
        {
            return Err(Error::business(
                "stale_executor",
                "proxy executor lease is no longer active",
            ));
        }
        Ok(())
    }
    fn interrupt(&mut self) {
        if let Some(operation) = self
            .operation
            .as_mut()
            .filter(|operation| operation.status == OperationStatus::Pending)
        {
            operation.status = OperationStatus::Interrupted;
        }
    }
    fn view(&self) -> Control {
        let epoch = self
            .executor
            .as_ref()
            .map(|executor| executor.epoch.clone());
        Control {
            desired: self.desired.clone(),
            operation: self.operation.clone(),
            measured_confirmed: self.active() && self.measured_epoch == epoch,
            executor: ExecutorView {
                epoch,
                active: self.active(),
            },
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExecutorView {
    pub epoch: Option<String>,
    pub active: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Control {
    pub desired: Desired,
    pub operation: Option<Operation>,
    pub executor: ExecutorView,
    pub measured_confirmed: bool,
}
fn load(store: &Store) -> Result<ControlState> {
    let state = store
        .read::<ControlState>(&control_key())?
        .unwrap_or_default();
    if state.format != 1 {
        return Err(Error::business(
            "unsupported_format",
            "invalid local service state",
        ));
    }
    Ok(state)
}
fn save(context: &mut dyn ServicesContext, state: &ControlState) -> Result<u64> {
    context.store().write(&control_key(), state)?;
    context.commit()
}
pub fn projection(store: &Store) -> Result<Control> {
    Ok(load(store)?.view())
}

/// A measured field distinguishes missing, explicit null, and a typed value.
/// Reports are echoed and persisted without inventing fields the executor did not send.
#[derive(Clone, Debug, PartialEq)]
pub struct ReportField<T>(Option<Option<T>>);
impl<T> Default for ReportField<T> {
    fn default() -> Self {
        Self(None)
    }
}
impl<T> ReportField<T> {
    pub fn is_missing(&self) -> bool {
        self.0.is_none()
    }
    pub fn as_ref(&self) -> Option<&T> {
        self.0.as_ref().and_then(Option::as_ref)
    }
    pub fn as_deref(&self) -> Option<&T::Target>
    where
        T: std::ops::Deref,
    {
        self.as_ref().map(std::ops::Deref::deref)
    }
    pub fn copied(&self) -> Option<T>
    where
        T: Copy,
    {
        self.as_ref().copied()
    }
}
impl<T: Serialize> Serialize for ReportField<T> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.0.as_ref().unwrap_or(&None).serialize(serializer)
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for ReportField<T> {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        Option::<T>::deserialize(deserializer).map(|value| Self(Some(value)))
    }
}
impl<T: JsonSchema> JsonSchema for ReportField<T> {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        format!("ReportField_{}", T::schema_name()).into()
    }
    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        generator.subschema_for::<Option<T>>()
    }
}

// Known, deliberately redacted fields from the local proxy executor. Open string
// observations and flattened additions remain intact for newer executors.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyState {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub phase: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub progress: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub error: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub stop_unconfirmed: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub pid: ReportField<u64>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub version: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub controller_reachable: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub mode: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub capabilities: ReportField<Capabilities>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub config: ReportField<ConfigState>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub conflict: ReportField<Conflict>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub tun: ReportField<Tun>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub groups: ReportField<Groups>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub delays: ReportField<BTreeMap<String, Delay>>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub testing: ReportField<Vec<String>>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub traffic: ReportField<Traffic>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub providers: ReportField<Vec<Provider>>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Capabilities {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub kernel: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub guardian: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub root: ReportField<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ConfigState {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub exists: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub error: ReportField<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Conflict {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub vpn_active: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub foreign_interfaces: ReportField<Vec<String>>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub device_taken: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub rule_collisions: ReportField<Vec<String>>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Tun {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub device: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub up: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub routed: ReportField<bool>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Groups {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub groups: ReportField<Vec<Group>>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub global: ReportField<Group>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Group {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub name: ReportField<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub kind: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub now: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub hidden: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub fixed: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub members: ReportField<Vec<Node>>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Node {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub name: ReportField<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub kind: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub alive: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub delay_ms: ReportField<i64>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub udp: ReportField<bool>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub now: ReportField<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Delay {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub kind: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub ms: ReportField<i64>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub message: ReportField<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Traffic {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub up: ReportField<i64>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub down: ReportField<i64>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub up_total: ReportField<i64>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub down_total: ReportField<i64>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Provider {
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub name: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub kind: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub vehicle_type: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub updated_at: ReportField<String>,
    #[serde(skip_serializing_if = "ReportField::is_missing")]
    pub size: ReportField<i64>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Measurement {
    pub format: u32,
    pub state: ProxyState,
    pub reported_at: u64,
    pub epoch: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub service_id: String,
    pub executor_id: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LeaseRequest {
    pub service_id: String,
    pub epoch: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportRequest {
    pub service_id: String,
    pub epoch: String,
    pub state: ProxyState,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequest {
    pub service_id: String,
    pub epoch: String,
    pub operation_id: String,
    pub success: bool,
    pub state: ProxyState,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequest {
    pub service_id: String,
    pub epoch: String,
    #[serde(flatten)]
    pub command: Command,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl<'de> Deserialize<'de> for CommandRequest {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Fields {
            service_id: String,
            epoch: String,
            #[serde(flatten)]
            command: Command,
            #[serde(flatten)]
            extra: OpaqueObject,
        }
        let mut fields = Fields::deserialize(deserializer)?;
        // serde's flattened internally-tagged enum borrows its fields, leaving
        // them visible to the remainder map. They already have typed owners.
        fields.extra.remove("name");
        fields.extra.remove("args");
        Ok(Self {
            service_id: fields.service_id,
            epoch: fields.epoch,
            command: fields.command,
            extra: fields.extra,
        })
    }
}
#[derive(Clone, Debug)]
pub enum Request {
    Register(RegisterRequest),
    Retire(LeaseRequest),
    Report(ReportRequest),
    Complete(CompleteRequest),
    Command(CommandRequest),
}
impl Request {
    fn service_id(&self) -> &str {
        match self {
            Self::Register(request) => &request.service_id,
            Self::Retire(request) => &request.service_id,
            Self::Report(request) => &request.service_id,
            Self::Complete(request) => &request.service_id,
            Self::Command(request) => &request.service_id,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct EmptyArgs {
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextArgs {
    pub text: String,
    pub expected_revision: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ModeArgs {
    pub mode: ProxyMode,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct SelectArgs {
    pub group: String,
    pub node: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestNodeArgs {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default = "default_test_timeout")]
    pub timeout_ms: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestGroupArgs {
    pub group: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default = "default_test_timeout")]
    pub timeout_ms: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
fn default_test_timeout() -> u64 {
    5000
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "name", rename_all = "camelCase")]
pub enum Command {
    Start {
        #[serde(default)]
        args: EmptyArgs,
    },
    Stop {
        #[serde(default)]
        args: EmptyArgs,
    },
    CheckConfig {
        #[serde(default)]
        args: EmptyArgs,
    },
    RefreshProviders {
        #[serde(default)]
        args: EmptyArgs,
    },
    SetMode {
        args: ModeArgs,
    },
    Select {
        args: SelectArgs,
    },
    TestNode {
        args: TestNodeArgs,
    },
    TestGroup {
        args: TestGroupArgs,
    },
    EnsureConfig {
        #[serde(default)]
        args: EmptyArgs,
    },
    ImportConfig {
        args: TextArgs,
    },
    PublishLog {
        args: TextArgs,
    },
}
impl Command {
    fn name(&self) -> Option<CommandName> {
        Some(match self {
            Self::Start { .. } => CommandName::Start,
            Self::Stop { .. } => CommandName::Stop,
            Self::CheckConfig { .. } => CommandName::CheckConfig,
            Self::RefreshProviders { .. } => CommandName::RefreshProviders,
            Self::SetMode { .. } => CommandName::SetMode,
            Self::Select { .. } => CommandName::Select,
            Self::TestNode { .. } => CommandName::TestNode,
            Self::TestGroup { .. } => CommandName::TestGroup,
            _ => return None,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    pub text: Option<String>,
    pub revision: u64,
}
impl From<Document> for Config {
    fn from(document: Document) -> Self {
        Self {
            text: document.document,
            revision: document.revision,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Ticket {
    pub id: String,
    #[serde(flatten)]
    pub command: Command,
    pub config: Option<Config>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct CommandReply {
    pub operation: Option<Ticket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<Config>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    pub control: Control,
}
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Reply {
    Registered {
        epoch: String,
        control: Control,
    },
    Retired {
        control: Control,
    },
    Measured {
        #[serde(rename = "serviceId")]
        service_id: String,
        state: ProxyState,
        revision: u64,
        control: Control,
    },
    Command(CommandReply),
}

fn bounded(value: &str, max: usize) -> Result<()> {
    if value.is_empty() || value.len() > max || value.contains('\0') {
        return Err(Error::invalid("invalid service command argument"));
    }
    Ok(())
}
fn template() -> String {
    // UUID v4 uses the operating system's cryptographic random source. No private
    // configuration or host filesystem access is needed to make a fresh secret.
    let secret = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    include_str!("../resources/proxy-template.yaml").replace("@SECRET@", &secret)
}
fn measured(
    store: &Store,
    control: &mut ControlState,
    epoch: &str,
    state: &ProxyState,
) -> Result<()> {
    if serde_json::to_vec(state)
        .map_err(|_| Error::invalid("invalid measured state"))?
        .len()
        > MAX_REPORT
    {
        return Err(Error::invalid("proxy report exceeds 1 MiB"));
    }
    store.write(
        &measurement_key(),
        &Measurement {
            format: 1,
            state: state.clone(),
            reported_at: now(),
            epoch: epoch.into(),
            extra: OpaqueObject::default(),
        },
    )?;
    control.measured_epoch = Some(epoch.into());
    Ok(())
}
fn verify_completion(
    control: &ControlState,
    operation: &Operation,
    observed: &ProxyState,
) -> Result<()> {
    let failure = match operation.name {
        CommandName::Start
            if observed.phase.as_deref() != Some("running")
                || observed.pid.copied().unwrap_or(0) == 0 =>
        {
            Some("executor did not measure a running proxy")
        }
        CommandName::Stop
            if observed.phase.as_deref() != Some("stopped")
                || observed.pid.as_ref().is_some()
                || observed.stop_unconfirmed.copied() == Some(true) =>
        {
            Some("executor did not confirm proxy shutdown")
        }
        CommandName::SetMode
            if observed.mode.as_deref() != control.desired.mode.map(ProxyMode::as_str) =>
        {
            Some("executor did not confirm selected mode")
        }
        CommandName::Select => {
            let group = operation.target_group.as_deref().unwrap_or("");
            let expected = control.desired.selections.get(group);
            let observed = observed.groups.as_ref().and_then(|groups| {
                groups
                    .groups
                    .as_ref()
                    .into_iter()
                    .flatten()
                    .find(|entry| entry.name.as_deref() == Some(group))
                    .or(groups.global.as_ref())
            });
            if observed.is_some_and(|observed| {
                observed.name.as_deref() == Some(group) && observed.now.as_ref() == expected
            }) {
                None
            } else {
                Some("executor did not confirm selected node")
            }
        }
        _ => None,
    };
    if let Some(message) = failure {
        return Err(Error::business("unconfirmed", message));
    }
    Ok(())
}

/// Commit intent before returning a device ticket; only a fenced receipt records completion.
pub fn call(context: &mut dyn ServicesContext, request: Request) -> Result<Reply> {
    if request.service_id() != "proxy" {
        return Err(Error::invalid("unsupported local service"));
    }
    if !context.writable() {
        return Err(Error::business("read_only", "workspace is not writable"));
    }
    let mut control = load(context.store())?;
    match request {
        Request::Register(request) => {
            bounded(&request.executor_id, 200)?;
            if control.active()
                && control
                    .executor
                    .as_ref()
                    .is_some_and(|executor| executor.id == request.executor_id)
            {
                return Ok(Reply::Registered {
                    epoch: control.executor.as_ref().unwrap().epoch.clone(),
                    control: control.view(),
                });
            }
            control.interrupt();
            let epoch = uuid::Uuid::new_v4().to_string();
            control.executor = Some(Executor {
                id: request.executor_id,
                epoch: epoch.clone(),
                boot: boot().into(),
                extra: OpaqueObject::default(),
            });
            control.measured_epoch = None;
            save(context, &control)?;
            Ok(Reply::Registered {
                epoch,
                control: control.view(),
            })
        }
        Request::Retire(request) => {
            control.validate(&request.epoch)?;
            control.interrupt();
            control.executor = None;
            control.measured_epoch = None;
            save(context, &control)?;
            Ok(Reply::Retired {
                control: control.view(),
            })
        }
        Request::Report(request) => {
            control.validate(&request.epoch)?;
            measured(
                context.store(),
                &mut control,
                &request.epoch,
                &request.state,
            )?;
            let revision = save(context, &control)?;
            Ok(Reply::Measured {
                service_id: "proxy".into(),
                state: request.state,
                revision,
                control: control.view(),
            })
        }
        Request::Complete(request) => {
            control.validate(&request.epoch)?;
            let operation = control
                .operation
                .as_ref()
                .filter(|operation| {
                    operation.status == OperationStatus::Pending
                        && operation.id == request.operation_id
                        && operation.epoch == request.epoch
                })
                .ok_or_else(|| {
                    Error::business("stale_operation", "proxy operation is no longer pending")
                })?;
            if request.success {
                verify_completion(&control, operation, &request.state)?;
            }
            measured(
                context.store(),
                &mut control,
                &request.epoch,
                &request.state,
            )?;
            let operation = control.operation.as_mut().unwrap();
            operation.status = if request.success {
                OperationStatus::Completed
            } else {
                OperationStatus::Failed
            };
            operation.completed_at = Some(now());
            // Executor exceptions and command arguments can contain secrets. Never persist them.
            let revision = save(context, &control)?;
            Ok(Reply::Measured {
                service_id: "proxy".into(),
                state: request.state,
                revision,
                control: control.view(),
            })
        }
        Request::Command(request) => {
            control.validate(&request.epoch)?;
            command(context, &mut control, request)
        }
    }
}

fn command(
    context: &mut dyn ServicesContext,
    control: &mut ControlState,
    request: CommandRequest,
) -> Result<Reply> {
    match &request.command {
        Command::PublishLog { args } => {
            if args.text.len() > MAX_REPORT {
                return Err(Error::invalid("invalid bounded log publication"));
            }
            let receipt = context.document(DocumentRequest::Write {
                key: "runtime.log",
                document: args.text.clone(),
                expected_revision: args.expected_revision,
            })?;
            return Ok(Reply::Command(CommandReply {
                operation: None,
                config: None,
                revision: Some(receipt.revision),
                control: control.view(),
            }));
        }
        Command::EnsureConfig { .. } | Command::ImportConfig { .. } => {
            let mut config = context.document(DocumentRequest::Read { key: "config.yaml" })?;
            if let Command::ImportConfig { args } = &request.command {
                if args.text.is_empty() || args.text.len() > MAX_CONFIG {
                    return Err(Error::invalid(
                        "proxy configuration must be 1 byte to 16 MiB",
                    ));
                }
                context.document(DocumentRequest::Write {
                    key: "config.yaml",
                    document: args.text.clone(),
                    expected_revision: args.expected_revision,
                })?;
                config = context.document(DocumentRequest::Read { key: "config.yaml" })?;
            } else if config.document.is_none() {
                context.document(DocumentRequest::Write {
                    key: "config.yaml",
                    document: template(),
                    expected_revision: config.revision,
                })?;
                config = context.document(DocumentRequest::Read { key: "config.yaml" })?;
            }
            return Ok(Reply::Command(CommandReply {
                operation: None,
                config: Some(config.into()),
                revision: None,
                control: control.view(),
            }));
        }
        _ => {}
    }
    if control
        .operation
        .as_ref()
        .is_some_and(|operation| operation.status == OperationStatus::Pending)
    {
        return Err(Error::business(
            "busy",
            "a proxy operation still needs a measured completion",
        ));
    }
    let mut target_group = None;
    match &request.command {
        Command::Start { .. } => control.desired.running = true,
        Command::Stop { .. } => control.desired.running = false,
        Command::SetMode { args } => control.desired.mode = Some(args.mode),
        Command::Select { args } => {
            bounded(&args.group, 1024)?;
            bounded(&args.node, 1024)?;
            control
                .desired
                .selections
                .insert(args.group.clone(), args.node.clone());
            target_group = Some(args.group.clone());
        }
        Command::TestNode { args } => {
            bounded(&args.name, 1024)?;
            validate_test(args.url.as_deref(), args.timeout_ms)?;
        }
        Command::TestGroup { args } => {
            bounded(&args.group, 1024)?;
            validate_test(args.url.as_deref(), args.timeout_ms)?;
        }
        _ => {}
    }
    let config = if matches!(
        &request.command,
        Command::Start { .. } | Command::CheckConfig { .. } | Command::RefreshProviders { .. }
    ) {
        let document = context.document(DocumentRequest::Read { key: "config.yaml" })?;
        if document.document.is_none() {
            return Err(Error::business(
                "missing_config",
                "proxy configuration does not exist",
            ));
        }
        Some(document.into())
    } else {
        None
    };
    let id = uuid::Uuid::new_v4().to_string();
    control.operation = Some(Operation {
        id: id.clone(),
        name: request.command.name().unwrap(),
        status: OperationStatus::Pending,
        epoch: request.epoch,
        requested_at: now(),
        target_group,
        completed_at: None,
        extra: OpaqueObject::default(),
    });
    save(context, control)?;
    Ok(Reply::Command(CommandReply {
        operation: Some(Ticket {
            id,
            command: request.command,
            config,
        }),
        config: None,
        revision: None,
        control: control.view(),
    }))
}
fn validate_test(url: Option<&str>, timeout_ms: u64) -> Result<()> {
    if let Some(url) = url {
        if url.len() > 8192
            || !(url.starts_with("https://") || url.starts_with("http://"))
            || url.contains(['\r', '\n', '\0'])
        {
            return Err(Error::invalid("invalid test URL"));
        }
    }
    if !(1..=60000).contains(&timeout_ms) {
        return Err(Error::invalid("invalid test timeout"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Harness {
        _dir: tempfile::TempDir,
        store: Store,
        revision: u64,
        documents: BTreeMap<String, Document>,
    }
    impl ServicesContext for Harness {
        fn store(&self) -> &Store {
            &self.store
        }
        fn writable(&self) -> bool {
            true
        }
        fn commit(&mut self) -> Result<u64> {
            self.revision += 1;
            Ok(self.revision)
        }
        fn document(&mut self, request: DocumentRequest) -> Result<Document> {
            match request {
                DocumentRequest::Read { key } => {
                    Ok(self.documents.get(key).cloned().unwrap_or(Document {
                        document: None,
                        revision: 0,
                    }))
                }
                DocumentRequest::Write {
                    key,
                    document,
                    expected_revision,
                } => {
                    let revision = self
                        .documents
                        .get(key)
                        .map(|document| document.revision)
                        .unwrap_or(0);
                    if expected_revision != revision {
                        return Err(Error::business("conflict", "service document changed"));
                    }
                    let document = Document {
                        document: Some(document),
                        revision: revision + 1,
                    };
                    self.documents.insert(key.into(), document.clone());
                    self.commit()?;
                    Ok(document)
                }
            }
        }
    }
    impl Harness {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::open(dir.path()).unwrap();
            Self {
                _dir: dir,
                store,
                revision: 0,
                documents: BTreeMap::new(),
            }
        }
        fn register(&mut self, id: &str) -> String {
            let reply = call(
                self,
                Request::Register(RegisterRequest {
                    service_id: "proxy".into(),
                    executor_id: id.into(),
                    extra: OpaqueObject::default(),
                }),
            )
            .unwrap();
            match reply {
                Reply::Registered { epoch, .. } => epoch,
                _ => panic!("expected registration"),
            }
        }
        fn command(&mut self, epoch: &str, command: Command) -> Result<CommandReply> {
            match call(
                self,
                Request::Command(CommandRequest {
                    service_id: "proxy".into(),
                    epoch: epoch.into(),
                    command,
                    extra: OpaqueObject::default(),
                }),
            )? {
                Reply::Command(reply) => Ok(reply),
                _ => panic!("expected command reply"),
            }
        }
        fn complete(
            &mut self,
            epoch: &str,
            id: &str,
            success: bool,
            state: ProxyState,
        ) -> Result<Reply> {
            call(
                self,
                Request::Complete(CompleteRequest {
                    service_id: "proxy".into(),
                    epoch: epoch.into(),
                    operation_id: id.into(),
                    success,
                    state,
                    extra: OpaqueObject::default(),
                }),
            )
        }
    }
    fn start_command() -> Command {
        Command::Start {
            args: EmptyArgs::default(),
        }
    }
    fn ensure_command() -> Command {
        Command::EnsureConfig {
            args: EmptyArgs::default(),
        }
    }
    fn state(text: &str) -> ProxyState {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn config_creation_is_inactive_unique_and_cas_preserves_external_edits() {
        let mut harness = Harness::new();
        let epoch = harness.register("one");
        let first = harness.command(&epoch, ensure_command()).unwrap();
        let config = first.config.unwrap();
        let text = config.text.as_ref().unwrap();
        assert!(text.contains("enable: false"));
        assert!(text.contains("127.0.0.1:19090"));
        assert!(text.contains("device: workflow-tun"));
        assert!(text.contains("iproute2-table-index: 9500"));
        assert!(text.contains("auto-redirect: false"));
        assert!(!text.contains("@SECRET@"));
        assert!(first.operation.is_none());
        assert_eq!(
            harness
                .command(&epoch, ensure_command())
                .unwrap()
                .config
                .unwrap()
                .text,
            config.text
        );
        assert!(
            harness
                .store
                .read::<Measurement>(&measurement_key())
                .unwrap()
                .is_none()
        );
        harness.documents.insert(
            "config.yaml".into(),
            Document {
                document: Some("mode: direct\n".into()),
                revision: config.revision + 1,
            },
        );
        let error = harness
            .command(
                &epoch,
                Command::ImportConfig {
                    args: TextArgs {
                        text: "mode: rule\n".into(),
                        expected_revision: config.revision,
                        extra: OpaqueObject::default(),
                    },
                },
            )
            .unwrap_err();
        assert_eq!(error.kind, "conflict");
        assert_eq!(
            harness.documents["config.yaml"].document.as_deref(),
            Some("mode: direct\n")
        );
        assert_ne!(template(), template());
    }
    #[test]
    fn pending_intent_is_not_a_measured_success_and_stale_executors_cannot_complete() {
        let mut harness = Harness::new();
        let one = harness.register("one");
        assert_eq!(harness.register("one"), one);
        assert_eq!(
            harness.command(&one, start_command()).unwrap_err().kind,
            "missing_config"
        );
        harness.command(&one, ensure_command()).unwrap();
        let ticket = harness.command(&one, start_command()).unwrap();
        assert!(ticket.control.desired.running);
        assert_eq!(
            ticket.control.operation.as_ref().unwrap().status,
            OperationStatus::Pending
        );
        assert!(!ticket.control.measured_confirmed);
        assert_eq!(
            harness.command(&one, start_command()).unwrap_err().kind,
            "busy"
        );
        let id = ticket.operation.unwrap().id;
        assert_eq!(
            harness
                .complete(&one, &id, true, state(r#"{"phase":"stopped","pid":null}"#))
                .unwrap_err()
                .kind,
            "unconfirmed"
        );
        let two = harness.register("two");
        assert_ne!(one, two);
        assert_eq!(
            call(
                &mut harness,
                Request::Report(ReportRequest {
                    service_id: "proxy".into(),
                    epoch: one.clone(),
                    state: state(r#"{"phase":"running"}"#),
                    extra: OpaqueObject::default()
                })
            )
            .unwrap_err()
            .kind,
            "stale_executor"
        );
        assert_eq!(
            harness
                .complete(&one, &id, true, state(r#"{"phase":"running","pid":99}"#))
                .unwrap_err()
                .kind,
            "stale_executor"
        );
        assert_eq!(
            projection(&harness.store)
                .unwrap()
                .operation
                .unwrap()
                .status,
            OperationStatus::Interrupted
        );
    }
    #[test]
    fn previous_boot_epoch_is_inactive_and_invalid_commands_do_not_create_tickets() {
        let mut harness = Harness::new();
        let epoch = harness.register("one");
        assert!(
            serde_json::from_str::<Command>(r#"{"name":"setMode","args":{"mode":"arbitrary"}}"#)
                .is_err()
        );
        assert!(
            harness
                .command(
                    &epoch,
                    Command::TestNode {
                        args: TestNodeArgs {
                            name: "node".into(),
                            url: Some("file:///secret".into()),
                            timeout_ms: 5000,
                            extra: OpaqueObject::default()
                        }
                    }
                )
                .is_err()
        );
        assert!(
            harness
                .command(
                    &epoch,
                    Command::TestNode {
                        args: TestNodeArgs {
                            name: "node".into(),
                            url: None,
                            timeout_ms: 60001,
                            extra: OpaqueObject::default()
                        }
                    }
                )
                .is_err()
        );
        let mut control = load(&harness.store).unwrap();
        assert!(control.operation.is_none());
        control.executor.as_mut().unwrap().boot = "previous-engine-process".into();
        save(&mut harness, &control).unwrap();
        assert!(!projection(&harness.store).unwrap().executor.active);
        assert_eq!(
            harness
                .command(
                    &epoch,
                    Command::Stop {
                        args: EmptyArgs::default()
                    }
                )
                .unwrap_err()
                .kind,
            "stale_executor"
        );
    }
    #[test]
    fn failure_receipt_records_measurement_without_erasing_desired_intent() {
        let mut harness = Harness::new();
        let epoch = harness.register("one");
        harness.command(&epoch, ensure_command()).unwrap();
        let id = harness
            .command(&epoch, start_command())
            .unwrap()
            .operation
            .unwrap()
            .id;
        match harness
            .complete(&epoch, &id, false, state(r#"{"phase":"error","pid":null}"#))
            .unwrap()
        {
            Reply::Measured { state, control, .. } => {
                assert_eq!(control.operation.unwrap().status, OperationStatus::Failed);
                assert!(control.desired.running);
                assert_eq!(state.phase.as_deref(), Some("error"));
                assert!(control.measured_confirmed);
            }
            _ => panic!("expected measured receipt"),
        }
    }
    #[test]
    fn selected_node_requires_matching_readback_and_test_urls_are_not_persisted() {
        let mut harness = Harness::new();
        let epoch = harness.register("one");
        let id = harness
            .command(
                &epoch,
                Command::Select {
                    args: SelectArgs {
                        group: "choice".into(),
                        node: "chosen".into(),
                        extra: OpaqueObject::default(),
                    },
                },
            )
            .unwrap()
            .operation
            .unwrap()
            .id;
        assert_eq!(
            harness
                .complete(
                    &epoch,
                    &id,
                    true,
                    state(
                        r#"{"groups":{"groups":[{"name":"choice","now":"other"}],"global":null}}"#
                    )
                )
                .unwrap_err()
                .kind,
            "unconfirmed"
        );
        harness
            .complete(
                &epoch,
                &id,
                true,
                state(r#"{"groups":{"groups":[{"name":"choice","now":"chosen"}],"global":null}}"#),
            )
            .unwrap();
        assert_eq!(
            projection(&harness.store)
                .unwrap()
                .operation
                .unwrap()
                .status,
            OperationStatus::Completed
        );
        let url = "https://example.invalid/ping?token=fixture-private";
        let reply = harness
            .command(
                &epoch,
                Command::TestNode {
                    args: TestNodeArgs {
                        name: "node".into(),
                        url: Some(url.into()),
                        timeout_ms: 1000,
                        extra: OpaqueObject::default(),
                    },
                },
            )
            .unwrap();
        match reply.operation.unwrap().command {
            Command::TestNode { args } => assert_eq!(args.url.as_deref(), Some(url)),
            _ => panic!("expected testNode ticket"),
        }
        assert!(
            !serde_json::to_string(&load(&harness.store).unwrap())
                .unwrap()
                .contains("fixture-private")
        );
    }
    #[test]
    fn successful_stop_and_mode_tickets_require_their_actual_measurements() {
        let mut harness = Harness::new();
        let epoch = harness.register("one");
        let id = harness
            .command(
                &epoch,
                Command::Stop {
                    args: EmptyArgs::default(),
                },
            )
            .unwrap()
            .operation
            .unwrap()
            .id;
        assert_eq!(
            harness
                .complete(
                    &epoch,
                    &id,
                    true,
                    state(r#"{"phase":"stopped","pid":null,"stopUnconfirmed":true}"#)
                )
                .unwrap_err()
                .kind,
            "unconfirmed"
        );
        harness
            .complete(
                &epoch,
                &id,
                true,
                state(r#"{"phase":"stopped","pid":null,"stopUnconfirmed":false}"#),
            )
            .unwrap();
        let id = harness
            .command(
                &epoch,
                Command::SetMode {
                    args: ModeArgs {
                        mode: ProxyMode::Direct,
                        extra: OpaqueObject::default(),
                    },
                },
            )
            .unwrap()
            .operation
            .unwrap()
            .id;
        assert_eq!(
            harness
                .complete(&epoch, &id, true, state(r#"{"mode":"rule"}"#))
                .unwrap_err()
                .kind,
            "unconfirmed"
        );
        harness
            .complete(&epoch, &id, true, state(r#"{"mode":"direct"}"#))
            .unwrap();
    }
    #[test]
    fn retirement_interrupts_intent_and_preserves_unknown_measurement_fields() {
        let mut harness = Harness::new();
        let epoch = harness.register("one");
        harness
            .command(
                &epoch,
                Command::Stop {
                    args: EmptyArgs::default(),
                },
            )
            .unwrap();
        let original = state(
            r#"{"phase":"running","pid":7,"future":{"precision":123456789012345678901234567890},"groups":{"groups":[],"vendorFlag":true}}"#,
        );
        call(
            &mut harness,
            Request::Report(ReportRequest {
                service_id: "proxy".into(),
                epoch: epoch.clone(),
                state: original.clone(),
                extra: OpaqueObject::default(),
            }),
        )
        .unwrap();
        assert_eq!(
            harness
                .store
                .read::<Measurement>(&measurement_key())
                .unwrap()
                .unwrap()
                .state,
            original
        );
        call(
            &mut harness,
            Request::Retire(LeaseRequest {
                service_id: "proxy".into(),
                epoch,
                extra: OpaqueObject::default(),
            }),
        )
        .unwrap();
        let control = projection(&harness.store).unwrap();
        assert_eq!(
            control.operation.unwrap().status,
            OperationStatus::Interrupted
        );
        assert!(!control.measured_confirmed && !control.executor.active);
    }
    #[test]
    fn typed_command_envelopes_preserve_additions_and_default_only_documented_values() {
        let command: CommandRequest = serde_json::from_str(r#"{"serviceId":"proxy","epoch":"fixture","name":"testNode","args":{"name":"node","future":true},"futureRequest":{"id":12}}"#).unwrap();
        assert!(command.extra.contains_key("futureRequest"));
        assert!(!command.extra.contains_key("name"));
        assert!(!command.extra.contains_key("args"));
        if let Command::TestNode { args } = &command.command {
            assert_eq!(args.timeout_ms, 5000);
            assert!(args.extra.contains_key("future"));
        } else {
            panic!("expected testNode");
        }
        let encoded = serde_json::to_string(&command).unwrap();
        assert!(encoded.contains("\"futureRequest\""));
        assert!(encoded.contains("\"future\":true"));
        let start: Command = serde_json::from_str(r#"{"name":"start"}"#).unwrap();
        assert!(matches!(start, Command::Start { .. }));
        assert!(serde_json::from_str::<Command>(r#"{"name":"start","args":[]}"#).is_err());
        assert!(serde_json::from_str::<CompleteRequest>(r#"{"serviceId":"proxy","epoch":"fixture","operationId":"ticket","success":true,"state":[]}"#).is_err());
    }

    #[test]
    fn measured_state_retains_missing_fields_explicit_null_and_nested_extensions() {
        let source = r#"{"phase":"stopped","pid":null,"groups":{"groups":[{"name":"choice","now":null,"future":9}],"global":null},"delays":{"node":{"kind":"timeout"}}}"#;
        let measured = state(source);
        assert!(measured.progress.is_missing());
        assert!(!measured.pid.is_missing());
        assert!(measured.pid.as_ref().is_none());
        assert_eq!(serde_json::to_string(&measured).unwrap(), source);
    }
}
