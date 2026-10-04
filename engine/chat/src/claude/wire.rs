//! Tolerant typed projections of Claude Code stream-json frames, transcript entries and control
//! bodies (Claude Agent SDK 0.3.283 inventory). Every field is optional (see [`crate::wire`]); the
//! complete frame is retained separately as the neutral model's `raw`.
use crate::wire::{Arr, Bool, Double, Json, Long, Obj, Str, Strings};
use serde::Deserialize;

/// One stdout frame or transcript entry. Each frame type reads its own subset.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Frame {
    #[serde(rename = "type")]
    pub kind: Str,
    pub subtype: Str,
    pub session_id: Str,
    pub uuid: Str,
    pub user_message_uuid: Str,
    pub parent_tool_use_id: Str,
    pub cwd: Str,
    pub model: Str,
    #[serde(rename = "permissionMode")]
    pub permission_mode: Str,
    pub status: Str,
    pub state: Str,
    pub estimated_tokens: Long,
    pub attempt: Long,
    pub max_retries: Long,
    pub error: Str,
    pub error_status: Long,
    pub compact_metadata: Obj<CompactMetadata>,
    pub hook_event: Str,
    pub hook_event_name: Str,
    pub hook_name: Str,
    pub outcome: Str,
    pub content: Json,
    pub output: Json,
    pub tool_use_id: Str,
    pub message: Json,
    pub text: Str,
    pub title: Str,
    pub description: Str,
    pub summary: Str,
    pub command_uuid: Str,
    pub terminal_reason: Str,
    pub is_error: Bool,
    pub result: Str,
    pub errors: Strings,
    pub api_error_status: Json,
    pub usage: Json,
    pub permission_denials: Arr<Obj<Denial>>,
    pub duration_ms: Long,
    pub total_cost_usd: Double,
    pub event: Json,
    pub is_api_error_message: Bool,
    pub aborted: Bool,
    #[serde(rename = "isAbortedMidStream")]
    pub aborted_mid_stream: Bool,
    #[serde(rename = "apiBlockIndex")]
    pub api_block_index: Long,
    #[serde(rename = "isMeta")]
    pub is_meta: Bool,
    #[serde(rename = "isReplay")]
    pub is_replay: Bool,
    pub tool_use_result: Json,
    #[serde(rename = "toolUseResult")]
    pub tool_use_result_camel: Json,
    pub tool_result_meta: Arr<Obj<ResultMeta>>,
    #[serde(rename = "toolDenialKind")]
    pub tool_denial_kind: Str,
    pub rate_limit_info: Json,
    #[serde(rename = "isAuthenticating")]
    pub is_authenticating: Bool,
    pub elapsed_time_seconds: Double,
    #[serde(rename = "isSidechain")]
    pub is_sidechain: Bool,
    #[serde(rename = "customTitle")]
    pub custom_title: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct CompactMetadata {
    pub trigger: Str,
    pub pre_tokens: Long,
    pub post_tokens: Long,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Denial {
    pub tool_use_id: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ResultMeta {
    pub id: Str,
    pub non_execution_kind: Str,
}
/// A Messages-API message (`assistant` and `user` frames).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Message {
    pub id: Str,
    pub content: Json,
}
/// One content block.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Block {
    #[serde(rename = "type")]
    pub kind: Str,
    pub text: Str,
    pub thinking: Str,
    pub id: Str,
    pub name: Str,
    pub input: Json,
    pub tool_use_id: Str,
    pub content: Json,
    pub is_error: Bool,
    pub source: Obj<Source>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Source {
    #[serde(rename = "type")]
    pub kind: Str,
    pub media_type: Str,
    pub data: Str,
    pub url: Str,
}
/// A raw Messages-API streaming event inside `stream_event`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct StreamEvent {
    #[serde(rename = "type")]
    pub kind: Str,
    pub message: Obj<Message>,
    pub index: Long,
    pub content_block: Json,
    pub delta: Obj<Delta>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Delta {
    #[serde(rename = "type")]
    pub kind: Str,
    pub text: Str,
    pub thinking: Str,
    pub partial_json: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Usage {
    pub input_tokens: Long,
    pub cache_read_input_tokens: Long,
    pub output_tokens: Long,
    pub output_tokens_details: Obj<OutputDetails>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct OutputDetails {
    pub thinking_tokens: Long,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RateLimitInfo {
    pub utilization: Double,
    pub rate_limit_type: Str,
    pub resets_at: Long,
    pub status: Str,
}
/// `tool_use_result`: the CLI's structured tool output.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Structured {
    pub stdout: Str,
    pub stderr: Str,
    pub interrupted: Bool,
    #[serde(rename = "filePath")]
    pub file_path: Str,
    #[serde(rename = "type")]
    pub kind: Str,
    #[serde(rename = "structuredPatch")]
    pub structured_patch: Json,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Hunk {
    pub old_start: Long,
    pub old_lines: Long,
    pub new_start: Long,
    pub new_lines: Long,
    pub lines: Strings,
}
/// Tool `input` fields used to render built-in tools.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ToolInput {
    pub command: Str,
    pub description: Str,
    pub file_path: Str,
    pub notebook_path: Str,
    pub old_string: Str,
    pub new_string: Str,
    pub edits: Arr<Obj<Edit>>,
    pub todos: Arr<Obj<Todo>>,
    pub prompt: Str,
    pub subagent_type: Str,
    pub model: Str,
    pub query: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Edit {
    pub old_string: Str,
    pub new_string: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Todo {
    pub content: Str,
    pub status: Str,
}
/// The `initialize` control response.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Initialized {
    pub account: Json,
    pub models: Json,
    pub pending_permission_requests: Arr<Json>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Account {
    pub token_source: Str,
    pub api_key_source: Str,
    pub email: Str,
    pub subscription_type: Str,
    pub organization: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Model {
    pub value: Str,
    pub display_name: Str,
    pub description: Str,
    pub supports_effort: Bool,
    pub supported_effort_levels: Strings,
    pub resolved_model: Str,
}
/// A re-armed `pending_permission_requests` member and inbound control request bodies.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct PendingFrame {
    pub request_id: Str,
    pub request: Json,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ControlBody {
    pub subtype: Str,
    pub callback_id: Str,
    pub input: Json,
    pub mode: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct HookInput {
    pub hook_event_name: Str,
    pub tool_name: Str,
}
/// `get_usage` response.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct UsageResponse {
    pub rate_limits: crate::wire::Members,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct UsageWindow {
    pub utilization: Json,
}
