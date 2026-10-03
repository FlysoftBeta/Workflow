//! A single typed method catalog drives decoding and schema export.
pub mod documents;
use crate::{local_services, protocol::*, state};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use workflow_environment::{EnvironmentStatus, runtime, tools};
use workflow_filework as filework;
use workflow_terminal as terminal;
#[derive(Serialize)]
pub struct MethodSchema {
    pub params: Schema,
    pub result: Schema,
}
macro_rules! catalog {
    ($($variant:ident : $name:literal => $params:ty => $result:ty),+ $(,)?)=>{
        #[derive(Debug,Serialize,Deserialize,JsonSchema)]
        #[serde(tag="method",content="params")]
        pub enum Call {$(#[serde(rename=$name)]$variant($params)),+}
        pub const METHODS:&[&str]=&[$($name),+];
        impl Call {
            pub fn decode(method:&str,params:&OpaqueObject)->Result<Self>{
                let bytes=serde_json::to_vec(params).map_err(|e|Error::invalid(&e.to_string()))?;
                match method { $($name => strict_json::<$params>(&bytes).map(Self::$variant).map_err(|e|Error::invalid(&e.to_string())),)+ _=>Err(unknown_method()) }
            }
        }
        pub fn method_schemas(generator:&mut SchemaGenerator)->BTreeMap<String,MethodSchema>{
            BTreeMap::from([$(($name.into(),MethodSchema{params:generator.subschema_for::<$params>(),result:generator.subschema_for::<$result>()})),+])
        }
    }
}
catalog! {
    Hello:"hello"=>HelloParams=>HelloResult,
    Snapshot:"workspace.snapshot"=>Empty=>state::Snapshot,
    Watch:"workspace.watch"=>WatchParams=>state::Snapshot,
    Command:"workspace.command"=>state::Command=>state::CommandReply,
    ReadFile:"files.read"=>ReadFileParams=>BlobResult,
    BeginUpload:"files.upload.begin"=>UploadBeginParams=>filework::UploadStarted,
    UploadChunk:"files.upload.chunk"=>UploadChunkParams=>filework::UploadProgress,
    CommitUpload:"files.upload.commit"=>UploadIdParams=>filework::UploadOutcome,
    CancelUpload:"files.upload.cancel"=>UploadIdParams=>CancelResult,
    ReadDocument:"documents.read"=>DocumentParams=>DocumentResult,
    WriteDocument:"documents.write"=>DocumentParams=>Revision,
    QuarantineDocument:"documents.quarantine"=>DocumentParams=>Revision,
    EnvironmentStatus:"environment.status"=>Empty=>EnvironmentStatus,
    Reconcile:"environment.reconcile"=>RetryParams=>EnvironmentStatus,
    RestartEnvironment:"environment.restart"=>Empty=>EnvironmentStatus,
    Spawn:"process.spawn"=>runtime::SpawnOptions=>runtime::Spawned,
    ReadProcess:"process.read"=>ProcessReadParams=>ProcessReadResult,
    WriteProcess:"process.write"=>ProcessWriteParams=>WrittenResult,
    ResizeProcess:"process.resize"=>ProcessResizeParams=>runtime::Dimensions,
    StopProcess:"process.stop"=>ProcessStopParams=>StoppingResult,
    WaitProcess:"process.wait"=>ProcessWaitParams=>runtime::WaitStatus,
    ServicesStatus:"services.status"=>Empty=>ServicesResult,
    ServiceReport:"services.report"=>ServiceReportParams=>ServiceReportReply,
    ClientConfig:"client.config"=>ClientConfigParams=>state::ClientConfigReply,
    ToolsStatus:"environment.tools.status"=>Empty=>tools::ToolsStatus,
    InstallTool:"environment.tools.install"=>ToolInstallParams=>tools::ToolsStatus,
    ChatSnapshot:"chat.snapshot"=>ChatSnapshotParams=>OpaqueJson,
    ChatWatch:"chat.watch"=>ChatWatchParams=>OpaqueJson,
    ChatCommand:"chat.command"=>ChatCommandParams=>OpaqueJson,
    CreateTerminal:"terminal.create"=>terminal::StartOptions=>terminal::Metadata,
    AttachTerminal:"terminal.attach"=>TerminalStartParams=>terminal::Metadata,
    RestartTerminal:"terminal.restart"=>TerminalStartParams=>terminal::Metadata,
    TerminalStatus:"terminal.status"=>TerminalIdParams=>terminal::Metadata,
    ReadTerminal:"terminal.read"=>TerminalReadParams=>TerminalReadResult,
    WriteTerminal:"terminal.write"=>TerminalWriteParams=>WrittenResult,
    ResizeTerminal:"terminal.resize"=>TerminalResizeParams=>runtime::Dimensions,
    StopTerminal:"terminal.stop"=>TerminalStopParams=>StoppingResult,
    WaitTerminal:"terminal.wait"=>TerminalWaitParams=>runtime::WaitStatus,
    RenameTerminal:"terminal.rename"=>TerminalRenameParams=>terminal::Metadata,
    ClearTerminal:"terminal.clear"=>TerminalIdParams=>terminal::Metadata,
    RegisterExecutor:"services.executor.register"=>local_services::RegisterRequest=>local_services::Reply,
    RetireExecutor:"services.executor.retire"=>local_services::LeaseRequest=>local_services::Reply,
    ServiceCommand:"services.command"=>local_services::CommandRequest=>local_services::Reply,
    CompleteService:"services.complete"=>local_services::CompleteRequest=>local_services::Reply,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ServiceReportReply {
    Proxy(local_services::Reply),
    Other(ServiceReportResult),
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Reply {
    Snapshot(state::Snapshot),
    Command(state::CommandReply),
    Blob(BlobResult),
    UploadStarted(filework::UploadStarted),
    UploadProgress(filework::UploadProgress),
    UploadOutcome(filework::UploadOutcome),
    Cancel(CancelResult),
    Document(DocumentResult),
    Revision(Revision),
    Services(ServicesResult),
    Service(local_services::Reply),
    ServiceReport(ServiceReportReply),
    ClientConfig(state::ClientConfigReply),
    Environment(EnvironmentStatus),
    Tools(tools::ToolsStatus),
    Terminal(terminal::Metadata),
    TerminalRead(TerminalReadResult),
    ProcessRead(ProcessReadResult),
    Spawned(runtime::Spawned),
    Dimensions(runtime::Dimensions),
    Wait(runtime::WaitStatus),
    Written(WrittenResult),
    Stopping(StoppingResult),
    Chat(OpaqueJson),
}
#[derive(Serialize, JsonSchema)]
pub struct KnownRequest {
    pub jsonrpc: Version,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RpcId>,
    #[serde(flatten)]
    pub call: Call,
}
impl Call {
    pub fn requires_writable(&self) -> bool {
        matches!(
            self,
            Self::WriteDocument(_)
                | Self::QuarantineDocument(_)
                | Self::ServiceReport(_)
                | Self::BeginUpload(_)
                | Self::UploadChunk(_)
                | Self::CommitUpload(_)
                | Self::CancelUpload(_)
                | Self::Reconcile(_)
                | Self::RestartEnvironment(_)
                | Self::Spawn(_)
                | Self::ChatCommand(_)
                | Self::InstallTool(_)
                | Self::CreateTerminal(_)
                | Self::AttachTerminal(_)
                | Self::RestartTerminal(_)
                | Self::RenameTerminal(_)
                | Self::ClearTerminal(_)
                | Self::RegisterExecutor(_)
                | Self::RetireExecutor(_)
                | Self::ServiceCommand(_)
                | Self::CompleteService(_)
        )
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Contract {
    protocol: &'static str,
    engine_version: &'static str,
    transport: &'static str,
    max_frame_bytes: usize,
    max_blob_chunk_bytes: usize,
    methods: &'static [&'static str],
    schema_file: &'static str,
    method_schemas: BTreeMap<String, MethodSchema>,
    compatibility: &'static str,
    ownership: Ownership,
    embedded_lifetime: &'static str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Ownership {
    android: &'static str,
    engine: &'static str,
    guest_chat: &'static str,
    local_services: &'static str,
}
#[derive(Serialize)]
struct Golden {
    name: &'static str,
    message: OpaqueJson,
}
fn pretty(value: &impl Serialize) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    text
}
pub fn artifacts() -> BTreeMap<&'static str, String> {
    let mut generator = SchemaGenerator::default();
    let method_schemas = method_schemas(&mut generator);
    generator.subschema_for::<Response<OpaqueJson>>();
    generator.subschema_for::<Notification<Empty>>();
    let schema = generator.into_root_schema_for::<KnownRequest>();
    let contract = Contract {
        protocol: PROTOCOL,
        engine_version: "1.0.0",
        transport: "UTF-8 JSON-RPC 2.0, one object per line, no batches",
        max_frame_bytes: MAX_FRAME,
        max_blob_chunk_bytes: MAX_BLOB,
        methods: METHODS,
        schema_file: "schema.json",
        method_schemas,
        compatibility: "Exact protocol match is mandatory. No legacy migration or protocol fallback.",
        ownership: Ownership {
            android: "Workbench presentation, connection profiles, workspace-delivered local configuration, disposable projections and local Android capability execution",
            engine: "Typed Environment, Workspace, FileWork and Terminal domains composed by Server; chat adapters and send ledger remain in the supervised guest service until round 2",
            guest_chat: "Engine-supervised JVM service; opaque ChatWire bodies inside typed private RPC, no writable private-state mount",
            local_services: "Boot-bound executor epochs and measured operation receipts; no automatic replay of unknown outcomes",
        },
        embedded_lifetime: "Stdio transport closure stops the Server and owned processes. Reconnect rehydrates committed state; there is no detached or remote execution promise.",
    };
    let mut fixtures = Vec::new();
    let mut add = |name, message: OpaqueJson| fixtures.push(Golden { name, message });
    add(
        "hello-request",
        opaque(&KnownRequest {
            jsonrpc: Version::V2,
            id: Some(RpcId::string("client-1")),
            call: Call::Hello(HelloParams {
                protocol: PROTOCOL.into(),
                client_id: "fixture".into(),
                extra: Default::default(),
            }),
        })
        .unwrap(),
    );
    add(
        "hello-response",
        opaque(&Response::new(
            RpcId::string("client-1"),
            Ok(HelloResult::new(std::path::Path::new("/workspace"))),
        ))
        .unwrap(),
    );
    add(
        "unknown-method-error",
        opaque(&Response::<()>::new(
            RpcId::string("missing"),
            Err(unknown_method()),
        ))
        .unwrap(),
    );
    add(
        "null-id-response",
        opaque(&Response::new(RpcId::null(), Ok(Revision { revision: 7 }))).unwrap(),
    );
    add(
        "precise-id-response",
        opaque(&Response::new(
            RpcId(
                serde_json::value::RawValue::from_string("123456789012345678901234567890".into())
                    .unwrap(),
            ),
            Ok(Revision { revision: 8 }),
        ))
        .unwrap(),
    );
    add(
        "workspace-watch",
        opaque(&KnownRequest {
            jsonrpc: Version::V2,
            id: Some(RpcId::string("watch-1")),
            call: Call::Watch(WatchParams {
                after_revision: 4,
                timeout_ms: 30000,
                extra: Default::default(),
            }),
        })
        .unwrap(),
    );
    add(
        "notification",
        opaque(&Notification {
            jsonrpc: Version::V2,
            method: "future.notification".into(),
            params: Empty::default(),
        })
        .unwrap(),
    );
    add(
        "empty-workspace",
        opaque(&Response::new(
            RpcId::string("snapshot"),
            Ok(state::Snapshot {
                revision: 0,
                state: state::State::default(),
            }),
        ))
        .unwrap(),
    );
    add(
        "create-session",
        opaque(&KnownRequest {
            jsonrpc: Version::V2,
            id: Some(RpcId::string("create")),
            call: Call::Command(state::Command::CreateSession {
                name: Some("Example".into()),
            }),
        })
        .unwrap(),
    );
    add(
        "session-created",
        opaque(&Response::new(
            RpcId::string("create"),
            Ok(state::CommandReply {
                revision: 1,
                state: state::State::default(),
                value: state::CommandValue::Text("session-fixture".into()),
            }),
        ))
        .unwrap(),
    );
    add(
        "file-chunk",
        opaque(&Response::new(
            RpcId::string("read"),
            Ok(BlobResult {
                data: encode_blob(&[0, 255, 3]),
                next_offset: 3,
                eof: true,
                size: 3,
            }),
        ))
        .unwrap(),
    );
    add(
        "network-report",
        opaque(&KnownRequest {
            jsonrpc: Version::V2,
            id: Some(RpcId::string("network")),
            call: Call::ServiceReport(ServiceReportParams::Network(NetworkReportParams {
                service_id: NetworkTag::Network,
                state: NetworkState {
                    connected: true,
                    dns_servers: vec!["127.0.0.1".into()],
                    extra: Default::default(),
                },
                extra: Default::default(),
            })),
        })
        .unwrap(),
    );
    add(
        "terminal-output",
        opaque(&Response::new(
            RpcId::string("terminal"),
            Ok(TerminalReadResult {
                data: encode_blob(b"hello\r\n"),
                start_offset: 0,
                next_offset: 7,
                eof: false,
                exit_code: None,
                reset: false,
                terminal: terminal::Metadata {
                    id: "terminal-fixture".into(),
                    ordinal: 1,
                    generation: 1,
                    cwd: "/workspace".into(),
                    title: None,
                    custom_title: None,
                    status: terminal::Status::Running,
                    exit_code: None,
                    error: None,
                    rows: 24,
                    columns: 80,
                    extra: Default::default(),
                },
            }),
        ))
        .unwrap(),
    );
    BTreeMap::from([
        ("contract.json", pretty(&contract)),
        ("schema.json", pretty(&schema)),
        ("golden.json", pretty(&fixtures)),
    ])
}
pub fn export(directory: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(directory)?;
    for (name, bytes) in artifacts() {
        std::fs::write(directory.join(name), bytes)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_contract_has_not_drifted() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../protocol");
        for (name, expected) in artifacts() {
            assert!(
                std::fs::read_to_string(root.join(name)).unwrap_or_default() == expected,
                "Regenerate with workflow-engine export-contract --out engine/protocol; artifact {name}"
            );
        }
    }
    #[test]
    fn catalog_rejects_unknown_and_invalid_known_requests() {
        assert_eq!(
            Call::decode("not-a-method", &Default::default())
                .unwrap_err()
                .code,
            -32601
        );
        assert_eq!(
            Call::decode("files.read", &Default::default())
                .unwrap_err()
                .code,
            -32602
        );
        assert_eq!(
            METHODS.len(),
            METHODS
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
        );
    }
}
