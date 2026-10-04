//! Typed projections of Codex App-Server bodies (pinned 0.157.1 schemas under `engine/chat/protocol`).
//!
//! Every field is tolerant (see [`crate::wire`]); the complete vendor body is retained separately as
//! the neutral model's `raw`, so unknown fields are preserved without being read here.
use crate::wire::{Arr, Bool, Double, Json, Long, Members, Obj, Str, Strings};
use serde::Deserialize;

/// Parameters of every modelled server notification. Each method reads only its own fields.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Notification {
    pub thread_id: Str,
    pub turn_id: Str,
    pub item_id: Str,
    pub error: Json,
    pub will_retry: Bool,
    pub thread: Json,
    pub status: Json,
    pub thread_name: Str,
    pub thread_settings: Json,
    pub token_usage: Json,
    pub turn: Json,
    pub run: Obj<HookRun>,
    pub diff: Str,
    pub plan: Json,
    pub explanation: Str,
    pub item: Json,
    pub delta: Str,
    pub stdin: Str,
    pub changes: Json,
    pub message: Str,
    pub summary_index: Long,
    pub content_index: Long,
    pub request_id: Json,
    pub success: Bool,
    pub name: Str,
    pub rate_limits: Json,
    pub from_model: Str,
    pub to_model: Str,
    pub reason: Json,
    pub show_buffering_ui: Bool,
    pub reasons: Strings,
    pub summary: Str,
    pub details: Str,
    pub path: Str,
    pub login_id: Str,
    pub auth_mode: Json,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HookRun {
    pub id: Str,
    pub event_name: Str,
    pub status: Str,
    pub status_message: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RunStatus {
    #[serde(rename = "type")]
    pub kind: Str,
    pub active_flags: Strings,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub model: Str,
    pub effort: Str,
    pub reasoning_effort: Str,
    pub approval_policy: Json,
    pub sandbox_policy: Json,
    pub sandbox: Json,
    pub approvals_reviewer: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Typed {
    #[serde(rename = "type")]
    pub kind: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Thread {
    pub id: Str,
    pub name: Str,
    pub preview: Str,
    pub cwd: Str,
    pub path: Str,
    pub forked_from_id: Str,
    pub ephemeral: Bool,
    pub status: Json,
    pub model: Str,
    pub reasoning_effort: Str,
    pub created_at: Long,
    pub updated_at: Long,
}
/// `thread/start`, `thread/resume` and `thread/fork` results: the thread plus the effective settings.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ThreadResult {
    pub thread: Json,
    pub approvals_reviewer: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TokenUsage {
    pub total: Obj<TokenTotals>,
    pub model_context_window: Long,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TokenTotals {
    pub input_tokens: Long,
    pub cached_input_tokens: Long,
    pub output_tokens: Long,
    pub reasoning_output_tokens: Long,
    pub total_tokens: Long,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Turn {
    pub id: Str,
    pub status: Str,
    pub items: Json,
    pub error: Json,
    pub started_at: Long,
    pub completed_at: Long,
    pub duration_ms: Long,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TurnError {
    pub message: Str,
    pub codex_error_info: Json,
    pub additional_details: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct PlanStep {
    pub step: Str,
    pub status: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Change {
    pub path: Str,
    pub kind: Obj<ChangeKind>,
    pub diff: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ChangeKind {
    #[serde(rename = "type")]
    pub kind: Str,
    pub move_path: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RateLimit {
    pub limit_id: Str,
    pub limit_name: Str,
    pub primary: Obj<RateLimitWindow>,
    pub secondary: Obj<RateLimitWindow>,
    pub rate_limit_reached_type: Json,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RateLimitWindow {
    pub used_percent: Double,
    pub window_duration_mins: Long,
    pub resets_at: Long,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RateLimits {
    pub rate_limits_by_limit_id: Members,
    pub rate_limits: Json,
    pub ordinary_usage_allowed: Bool,
    pub rate_limit_upsell: Json,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountRead {
    pub account: Json,
    pub requires_openai_auth: Bool,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Account {
    #[serde(rename = "type")]
    pub kind: Str,
    pub email: Str,
    pub plan_type: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LoginStart {
    #[serde(rename = "type")]
    pub kind: Str,
    pub login_id: Str,
    pub verification_url: Str,
    pub user_code: Str,
    pub auth_url: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Page {
    pub data: Arr<Json>,
    pub next_cursor: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Model {
    pub id: Str,
    pub model: Str,
    pub display_name: Str,
    pub description: Str,
    pub supported_reasoning_efforts: Arr<Effort>,
    pub default_reasoning_effort: Str,
    pub is_default: Bool,
    pub hidden: Bool,
    pub input_modalities: Strings,
    pub upgrade: Str,
    pub upgrade_info: Obj<Upgrade>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Effort {
    pub reasoning_effort: Str,
    pub description: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Upgrade {
    pub migration_markdown: Str,
}
/// One `ThreadItem`. Each item type reads its own subset.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Item {
    pub id: Str,
    #[serde(rename = "type")]
    pub kind: Str,
    pub content: Json,
    pub client_id: Str,
    pub fragments: Arr<Fragment>,
    pub text: Str,
    pub phase: Str,
    pub namespace: Str,
    pub name: Str,
    pub output: Json,
    pub summary: Strings,
    pub command: Str,
    pub cwd: Str,
    pub aggregated_output: Str,
    pub exit_code: Long,
    pub duration_ms: Long,
    pub process_id: Str,
    pub command_actions: Arr<Action>,
    pub status: Str,
    pub changes: Json,
    pub tool: Str,
    pub server: Str,
    pub arguments: Json,
    pub result: Json,
    pub error: Obj<Message>,
    pub content_items: Json,
    pub success: Bool,
    pub prompt: Str,
    pub model: Str,
    pub receiver_thread_ids: Strings,
    pub agent_path: Str,
    pub agent_thread_id: Str,
    #[serde(rename = "kind")]
    pub activity: Str,
    pub query: Str,
    pub action: Obj<SearchAction>,
    pub results: Json,
    pub path: Str,
    pub saved_path: Str,
    pub revised_prompt: Str,
    pub failure: Json,
    pub review: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Fragment {
    pub text: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Message {
    pub message: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Action {
    #[serde(rename = "type")]
    pub kind: Str,
    pub command: Str,
    pub path: Str,
    pub query: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SearchAction {
    #[serde(rename = "type")]
    pub kind: Str,
    pub url: Str,
}
/// A `UserInput` element (user messages and queued submissions).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Input {
    #[serde(rename = "type")]
    pub kind: Str,
    pub text: Str,
    pub path: Str,
    pub url: Str,
    pub name: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct QueuedSubmission {
    pub id: Str,
    pub client_user_message_id: Str,
    pub input: Arr<Json>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TurnStartResult {
    pub turn: Obj<Identified>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TurnSteerResult {
    pub turn_id: Str,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct QueueAddResult {
    pub queued_submission: Obj<Identified>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Identified {
    pub id: Str,
}
