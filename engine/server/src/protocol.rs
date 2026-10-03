//! JSON-RPC envelopes and transport framing. Domain types never depend on this module.
use base64::Engine;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Write},
    sync::{Arc, Mutex},
};
#[derive(Clone, Debug)]
pub struct Error {
    pub code: i32,
    pub kind: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
impl Error {
    pub fn invalid(message: &str) -> Self {
        Self {
            code: -32602,
            kind: "invalid_params".into(),
            message: message.into(),
        }
    }
    pub fn business(kind: &str, message: &str) -> Self {
        Self {
            code: -32000,
            kind: kind.into(),
            message: message.into(),
        }
    }
}
impl From<workflow_environment::Error> for Error {
    fn from(error: workflow_environment::Error) -> Self {
        Self {
            code: if error.kind == "invalid_params" {
                -32602
            } else {
                -32000
            },
            kind: error.kind,
            message: error.message,
        }
    }
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::business("io", &error.to_string())
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}
impl std::error::Error for Error {}
pub use workflow_environment::json::{OpaqueJson, OpaqueObject, strict_json};
pub const PROTOCOL: &str = "workflow.workspace/1";
pub const MAX_FRAME: usize = 32 * 1024 * 1024;
pub const MAX_BLOB: usize = 65_536;

/// Retains the numeric token itself, including arbitrary precision and exponent spelling.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct RpcId(#[schemars(with = "IdSchema")] pub Box<serde_json::value::RawValue>);
#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum IdSchema {
    String(String),
    Number(serde_json::Number),
    Null(()),
}
impl RpcId {
    pub fn null() -> Self {
        Self(serde_json::value::RawValue::from_string("null".into()).unwrap())
    }
    pub fn string(id: &str) -> Self {
        Self(serde_json::value::to_raw_value(id).unwrap())
    }
    pub fn valid(&self) -> bool {
        matches!(
            self.0.get().as_bytes().first(),
            Some(b'"' | b'n' | b'-' | b'0'..=b'9')
        )
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub enum Version {
    #[serde(rename = "2.0")]
    V2,
}
impl Default for Version {
    fn default() -> Self {
        Self::V2
    }
}
fn present_id<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<RpcId>, D::Error> {
    RpcId::deserialize(d).map(Some)
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Request {
    pub jsonrpc: Version,
    #[serde(
        default,
        deserialize_with = "present_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<RpcId>,
    pub method: String,
    #[serde(default)]
    pub params: OpaqueObject,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ErrorData {
    pub kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: ErrorData,
}
impl From<Error> for RpcError {
    fn from(error: Error) -> Self {
        Self {
            code: error.code,
            message: error.message,
            data: ErrorData { kind: error.kind },
        }
    }
}
impl From<RpcError> for Error {
    fn from(error: RpcError) -> Self {
        Self {
            code: error.code,
            kind: error.data.kind,
            message: error.message,
        }
    }
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Response<T> {
    Success {
        jsonrpc: Version,
        id: RpcId,
        result: T,
    },
    Failure {
        jsonrpc: Version,
        id: RpcId,
        error: RpcError,
    },
}
impl<T> Response<T> {
    pub fn new(id: RpcId, result: Result<T>) -> Self {
        match result {
            Ok(result) => Self::Success {
                jsonrpc: Version::V2,
                id,
                result,
            },
            Err(error) => Self::Failure {
                jsonrpc: Version::V2,
                id,
                error: error.into(),
            },
        }
    }
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Notification<T> {
    pub jsonrpc: Version,
    pub method: String,
    pub params: T,
}

pub fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    strict_json(bytes).map_err(|e| Error::invalid(&e.to_string()))
}
pub fn params<T: serde::de::DeserializeOwned>(value: &OpaqueObject) -> Result<T> {
    strict_json(&serde_json::to_vec(value).map_err(|e| Error::invalid(&e.to_string()))?)
        .map_err(|e| Error::invalid(&e.to_string()))
}
#[cfg(test)]
pub fn object<T: Serialize>(value: &T) -> Result<OpaqueObject> {
    strict_json(&serde_json::to_vec(value).map_err(|e| Error::invalid(&e.to_string()))?)
        .map_err(|e| Error::invalid(&e.to_string()))
}
pub fn opaque<T: Serialize>(value: &T) -> Result<OpaqueJson> {
    strict_json(&serde_json::to_vec(value).map_err(|e| Error::invalid(&e.to_string()))?)
        .map_err(|e| Error::invalid(&e.to_string()))
}
pub fn decode_blob(s: &str, max: usize) -> Result<Vec<u8>> {
    if s.len() > max.div_ceil(3) * 4 {
        return Err(Error::invalid("blob exceeds chunk limit"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|_| Error::invalid("invalid base64"))?;
    if bytes.len() > max {
        return Err(Error::invalid("blob exceeds chunk limit"));
    }
    Ok(bytes)
}
pub fn encode_blob(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}
pub fn response<T: Serialize>(writer: &Arc<Mutex<io::Stdout>>, id: RpcId, result: Result<T>) {
    let mut bytes = serde_json::to_vec(&Response::new(id.clone(), result)).unwrap();
    if bytes.len() > MAX_FRAME {
        bytes = serde_json::to_vec(&Response::<()>::new(
            id,
            Err(Error::business("too_large", "response exceeds frame limit")),
        ))
        .unwrap();
    }
    let mut stdout = writer.lock().unwrap();
    let _ = stdout.write_all(&bytes);
    let _ = stdout.write_all(b"\n");
    let _ = stdout.flush();
}
pub fn line(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut out = Vec::new();
    let mut oversized = false;
    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            return if out.is_empty() && !oversized {
                Ok(None)
            } else if oversized {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "frame too large",
                ))
            } else {
                Ok(Some(out))
            };
        }
        let take = buf
            .iter()
            .position(|b| *b == b'\n')
            .map(|p| p + 1)
            .unwrap_or(buf.len());
        let ended = buf[take - 1] == b'\n';
        if out.len() + take > MAX_FRAME + 1 {
            oversized = true;
        }
        if !oversized {
            out.extend_from_slice(&buf[..take]);
        }
        reader.consume(take);
        if ended {
            if oversized {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "frame too large",
                ));
            }
            out.pop();
            if out.last() == Some(&b'\r') {
                out.pop();
            }
            return Ok(Some(out));
        }
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Empty {
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HelloParams {
    pub protocol: String,
    pub client_id: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub workspace: bool,
    pub files: bool,
    pub documents: bool,
    pub environment: bool,
    pub processes: bool,
    pub pty: bool,
    pub services: bool,
    pub chat: bool,
    pub max_frame_bytes: usize,
    pub max_blob_chunk_bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HelloResult {
    pub protocol: String,
    pub engine_version: String,
    pub workspace_root: String,
    pub capabilities: Capabilities,
}
impl HelloResult {
    pub fn new(root: &std::path::Path) -> Self {
        Self {
            protocol: PROTOCOL.into(),
            engine_version: "1.0.0".into(),
            workspace_root: root.to_string_lossy().into_owned(),
            capabilities: Capabilities {
                workspace: true,
                files: true,
                documents: true,
                environment: true,
                processes: true,
                pty: true,
                services: true,
                chat: true,
                max_frame_bytes: MAX_FRAME,
                max_blob_chunk_bytes: MAX_BLOB,
            },
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WatchParams {
    pub after_revision: u64,
    #[serde(default = "watch_timeout")]
    pub timeout_ms: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
fn watch_timeout() -> u64 {
    30_000
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DocumentParams {
    pub namespace: String,
    pub key: String,
    #[serde(default)]
    pub document: Option<String>,
    #[serde(default)]
    pub expected_revision: Option<u64>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DocumentResult {
    pub document: Option<String>,
    pub revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Revision {
    pub revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientConfigParams {
    pub client_id: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct RetryParams {
    #[serde(default)]
    pub retry: bool,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolInstallParams {
    pub tool_id: String,
    #[serde(default)]
    pub retry: bool,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadFileParams {
    pub path: String,
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "blob_limit")]
    pub length: usize,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
fn blob_limit() -> usize {
    MAX_BLOB
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BlobResult {
    pub data: String,
    pub next_offset: u64,
    pub eof: bool,
    pub size: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadChunkParams {
    pub upload_id: String,
    pub offset: u64,
    pub data: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadIdParams {
    pub upload_id: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct CancelResult {
    pub cancelled: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChatSnapshotParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChatWatchParams {
    pub epoch: String,
    pub after_revision: u64,
    #[serde(default = "watch_timeout")]
    pub timeout_ms: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
/// The guest Kotlin service owns its command/body schema until the round-2 Rust port.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChatCommandParams {
    pub name: String,
    #[serde(default)]
    pub args: OpaqueObject,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalIdParams {
    pub terminal_id: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalReadParams {
    pub terminal_id: String,
    #[serde(default)]
    pub generation: Option<u64>,
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "blob_limit")]
    pub max_bytes: usize,
    #[serde(default = "read_wait")]
    pub wait_ms: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalWriteParams {
    pub terminal_id: String,
    pub data: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalResizeParams {
    pub terminal_id: String,
    pub rows: u16,
    pub columns: u16,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStopParams {
    pub terminal_id: String,
    #[serde(default)]
    pub force: bool,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalWaitParams {
    pub terminal_id: String,
    #[serde(default = "read_wait")]
    pub timeout_ms: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRenameParams {
    pub terminal_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct WrittenResult {
    pub written: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct StoppingResult {
    pub stopping: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_id_and_notification_distinction() {
        for id in [
            "null",
            "\"client\"",
            "123456789012345678901234567890",
            "1.2300e+20",
        ] {
            let source = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"hello\"}}");
            let r: Request = serde_json::from_str(&source).unwrap();
            let response =
                serde_json::to_string(&Response::new(r.id.unwrap(), Ok(Empty::default()))).unwrap();
            assert!(response.contains(&format!("\"id\":{id},")), "{response}");
        }
        let r: Request = serde_json::from_str(r#"{"jsonrpc":"2.0","method":"x"}"#).unwrap();
        assert!(r.id.is_none());
        assert!(
            serde_json::from_str::<Request>(r#"{"jsonrpc":"2.0","method":"x","params":[]}"#)
                .is_err()
        );
    }
    #[test]
    fn unknown_fields_roundtrip_and_duplicate_rejection() {
        let bytes=br#"{"jsonrpc":"2.0","method":"extension","vendor":{"precise":123456789012345678901234567890},"params":{"future":{"x":true}}}"#;
        let req: Request = parse(bytes).unwrap();
        let again: Request = parse(&serde_json::to_vec(&req).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_string(&req.extra).unwrap(),
            serde_json::to_string(&again.extra).unwrap()
        );
        assert!(parse::<Request>(br#"{"jsonrpc":"2.0","method":"a","method":"b"}"#).is_err());
    }
    #[test]
    fn bounded_frames_and_blobs() {
        let mut r = io::Cursor::new(b"{}\r\n{}\n".to_vec());
        assert_eq!(line(&mut r).unwrap().unwrap(), b"{}");
        assert_eq!(line(&mut r).unwrap().unwrap(), b"{}");
        assert!(line(&mut r).unwrap().is_none());
        assert!(decode_blob(&encode_blob(&vec![0; 65537]), 65536).is_err());
        assert_eq!(
            decode_blob(&encode_blob(&[0, 255, 3]), 65536).unwrap(),
            vec![0, 255, 3]
        );
        assert!(line(&mut io::Cursor::new(vec![b'a'; MAX_FRAME + 2])).is_err());
    }
}

pub fn unknown_method() -> Error {
    Error {
        code: -32601,
        kind: "unknown_method".into(),
        message: "Method not found".into(),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessReadParams {
    pub process_id: String,
    #[serde(default)]
    pub stream: workflow_environment::runtime::OutputStream,
    #[serde(default)]
    pub offset: u64,
    #[serde(default = "blob_limit")]
    pub max_bytes: usize,
    #[serde(default)]
    pub wait_ms: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
fn read_wait() -> u64 {
    1000
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessWriteParams {
    pub process_id: String,
    pub data: String,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessResizeParams {
    pub process_id: String,
    pub rows: u16,
    pub columns: u16,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessStopParams {
    pub process_id: String,
    #[serde(default)]
    pub force: bool,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessWaitParams {
    pub process_id: String,
    #[serde(default)]
    pub timeout_ms: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProcessReadResult {
    pub data: String,
    pub start_offset: u64,
    pub next_offset: u64,
    pub eof: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}
impl From<workflow_environment::runtime::ReadOutput> for ProcessReadResult {
    fn from(v: workflow_environment::runtime::ReadOutput) -> Self {
        Self {
            data: encode_blob(&v.data),
            start_offset: v.start_offset,
            next_offset: v.next_offset,
            eof: v.eof,
            exit_code: v.exit_code,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStartParams {
    pub terminal_id: String,
    #[serde(flatten)]
    pub options: workflow_terminal::StartOptions,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminalReadResult {
    pub data: String,
    pub start_offset: u64,
    pub next_offset: u64,
    pub eof: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub reset: bool,
    pub terminal: workflow_terminal::Metadata,
}
impl From<workflow_terminal::ReadResult> for TerminalReadResult {
    fn from(v: workflow_terminal::ReadResult) -> Self {
        Self {
            data: encode_blob(&v.data),
            start_offset: v.start_offset,
            next_offset: v.next_offset,
            eof: v.eof,
            exit_code: v.exit_code,
            reset: v.reset,
            terminal: v.terminal,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkState {
    pub connected: bool,
    pub dns_servers: Vec<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ServiceReportParams {
    Network(NetworkReportParams),
    Proxy(ProxyReportParams),
    Extension(ExtensionReportParams),
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub enum NetworkTag {
    #[serde(rename = "network")]
    Network,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub enum ProxyTag {
    #[serde(rename = "proxy")]
    Proxy,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkReportParams {
    pub service_id: NetworkTag,
    pub state: NetworkState,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProxyReportParams {
    pub service_id: ProxyTag,
    pub epoch: String,
    pub state: crate::local_services::ProxyState,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct ExtensionServiceId(
    #[schemars(regex(pattern = "^(?!proxy$|network$)[A-Za-z0-9_.-]{1,160}$"))] pub String,
);
impl<'de> Deserialize<'de> for ExtensionServiceId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let id = String::deserialize(d)?;
        if matches!(id.as_str(), "network" | "proxy") {
            return Err(serde::de::Error::custom(
                "known service requires its typed report",
            ));
        }
        workflow_environment::persist::identifier(&id).map_err(serde::de::Error::custom)?;
        Ok(Self(id))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionReportParams {
    pub service_id: ExtensionServiceId,
    pub state: OpaqueObject,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceReport<T> {
    pub format: u32,
    pub state: T,
    pub reported_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ReportedState {
    Network(NetworkState),
    Extension(OpaqueObject),
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum StoredReport {
    Network(ServiceReport<NetworkState>),
    Proxy(crate::local_services::Measurement),
    Extension(ServiceReport<OpaqueObject>),
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceReportResult {
    pub revision: u64,
    pub service_id: String,
    pub state: ReportedState,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServiceControl {
    pub proxy: crate::local_services::Control,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServicesResult {
    pub services: BTreeMap<String, StoredReport>,
    pub desired: Option<OpaqueJson>,
    pub control: ServiceControl,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct UploadBeginParams {
    #[serde(flatten)]
    pub destination: workflow_filework::UploadDestination,
    pub size: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}

#[cfg(test)]
mod service_tests {
    use super::*;
    #[test]
    fn known_reports_cannot_fall_through_to_an_opaque_extension() {
        assert!(parse::<ServiceReportParams>(br#"{"serviceId":"network","state":{}}"#).is_err());
        assert!(
            parse::<ServiceReportParams>(br#"{"serviceId":"proxy","state":{"running":false}}"#)
                .is_err()
        );
        assert!(
            parse::<ServiceReportParams>(
                br#"{"serviceId":"network","state":{"connected":true,"dnsServers":[]}}"#
            )
            .is_ok()
        );
    }
    #[test]
    fn extension_measurements_preserve_precise_numbers_and_reserved_looking_objects() {
        let source=br#"{"serviceId":"extension","state":{"huge":123456789012345678901234567890,"decimal":0.12345678901234567890123456789,"literal":{"$serde_json::private::Number":"plain text"}}}"#;
        let report: ServiceReportParams = parse(source).unwrap();
        let encoded = serde_json::to_string(&report).unwrap();
        assert!(encoded.contains("123456789012345678901234567890"));
        assert!(encoded.contains("0.12345678901234567890123456789"));
        assert!(encoded.contains(r#"{"$serde_json::private::Number":"plain text"}"#));
    }
}
