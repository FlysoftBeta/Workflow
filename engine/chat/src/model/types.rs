//! Phase-one model. Names and defaults follow the client's Kotlin ChatWire codec.
use super::pairs::Pairs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use workflow_environment::json::{OpaqueJson, OpaqueObject};
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackendKind {
    #[default]
    Codex,
    Claude,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PermissionPreset {
    #[default]
    Ask,
    AutoEdit,
    Plan,
    DenyUnlisted,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ItemStatus {
    #[default]
    InProgress,
    Completed,
    Incomplete,
    Failed,
    Declined,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessagePhase {
    #[default]
    Commentary,
    Final,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanStepStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileChangeKind {
    #[default]
    Add,
    Delete,
    Update,
    Move,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolKind {
    #[default]
    Mcp,
    Dynamic,
    Builtin,
    FunctionOutput,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ImageKind {
    #[default]
    View,
    Generated,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarkerKind {
    #[default]
    Compaction,
    ReviewEntered,
    ReviewExited,
    Hook,
    HookPrompt,
    ModelRerouted,
    Interrupted,
    LocalCommand,
    Sleep,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NoticeLevel {
    #[default]
    Info,
    Warning,
    Error,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecisionKind {
    #[default]
    AllowOnce,
    AllowSession,
    AllowPersistent,
    Deny,
    Abort,
    Other,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequestStatus {
    #[default]
    Pending,
    Answered,
    Resolved,
    Cancelled,
    Expired,
    Rejected,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunState {
    #[default]
    NotLoaded,
    Idle,
    Running,
    WaitingApproval,
    WaitingInput,
    Error,
    Closed,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TurnStatus {
    #[default]
    Queued,
    Running,
    Completed,
    Interrupted,
    Failed,
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LoginState {
    #[default]
    Unknown,
    LoggedOut,
    LoggingIn,
    LoggedIn,
}
#[derive(Clone, Debug, PartialEq, Eq, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LoginMethod {
    #[default]
    CodexDeviceCode,
    CodexBrowser,
    CodexApiKey,
    ClaudeTerminalLogin,
    ClaudeSetupToken,
    ClaudeApiKey,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct TurnSettings {
    pub model: Option<String>,
    pub effort: Option<String>,
    pub permissions: Option<PermissionPreset>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type", rename_all_fields = "camelCase")]
pub enum UserPart {
    Text {
        text: String,
    },
    Image {
        path: String,
        mime_type: Option<String>,
    },
    File {
        path: String,
        mime_type: Option<String>,
    },
    InlineData {
        kind: String,
        media_type: Option<String>,
        base64: String,
    },
    ImageUrl {
        url: String,
    },
    Reference {
        kind: String,
        name: String,
        path: String,
    },
    Unknown {
        raw: OpaqueJson,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type", rename_all_fields = "camelCase")]
pub enum RequestResponse {
    Decide {
        decision_id: String,
        message: Option<String>,
    },
    Answer {
        answers: BTreeMap<String, Vec<String>>,
    },
    Elicit {
        action: String,
        content: Option<OpaqueObject>,
    },
    RawResult {
        result: OpaqueJson,
    },
    Reject {
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UserMessageItem {
    pub id: String,
    pub parts: Vec<UserPart>,
    pub client_message_id: Option<String>,
    pub local: bool,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for UserMessageItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            parts: Default::default(),
            client_message_id: None,
            local: false,
            status: ItemStatus::Completed,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AgentMessageItem {
    pub id: String,
    pub text: String,
    pub phase: MessagePhase,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for AgentMessageItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            text: Default::default(),
            phase: MessagePhase::Unknown,
            status: ItemStatus::InProgress,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ReasoningItem {
    pub id: String,
    pub summary: Vec<String>,
    pub content: Vec<String>,
    pub redacted: bool,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for ReasoningItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            summary: Default::default(),
            content: Default::default(),
            redacted: false,
            status: ItemStatus::InProgress,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct PlanStep {
    pub text: String,
    pub status: PlanStepStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlanItem {
    pub id: String,
    pub text: String,
    pub steps: Vec<PlanStep>,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for PlanItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            text: Default::default(),
            steps: Default::default(),
            status: ItemStatus::InProgress,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct CommandAction {
    pub kind: String,
    pub command: String,
    pub path: Option<String>,
    pub query: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CommandItem {
    pub id: String,
    pub command: String,
    pub cwd: Option<String>,
    pub output: String,
    pub output_truncated: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub process_id: Option<String>,
    pub actions: Vec<CommandAction>,
    pub description: Option<String>,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for CommandItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            command: Default::default(),
            cwd: None,
            output: Default::default(),
            output_truncated: false,
            exit_code: None,
            duration_ms: None,
            process_id: None,
            actions: Default::default(),
            description: None,
            status: ItemStatus::InProgress,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct FileDelta {
    pub path: String,
    pub kind: FileChangeKind,
    pub diff: Option<String>,
    pub move_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FileChangeItem {
    pub id: String,
    pub changes: Vec<FileDelta>,
    pub output: String,
    pub tool: Option<String>,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for FileChangeItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            changes: Default::default(),
            output: Default::default(),
            tool: None,
            status: ItemStatus::InProgress,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ToolCallItem {
    pub id: String,
    pub kind: ToolKind,
    pub tool: String,
    pub server: Option<String>,
    pub arguments: Option<OpaqueJson>,
    pub arguments_text: String,
    pub result: Option<OpaqueJson>,
    pub result_text: Option<String>,
    pub error: Option<String>,
    pub progress: Vec<String>,
    pub duration_ms: Option<i64>,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for ToolCallItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            kind: Default::default(),
            tool: Default::default(),
            server: None,
            arguments: None,
            arguments_text: Default::default(),
            result: None,
            result_text: None,
            error: None,
            progress: Default::default(),
            duration_ms: None,
            status: ItemStatus::InProgress,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SubAgentItem {
    pub id: String,
    pub tool: String,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub agent_type: Option<String>,
    pub model: Option<String>,
    pub thread_ids: Vec<String>,
    pub result: Option<String>,
    pub progress: Vec<String>,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for SubAgentItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            tool: Default::default(),
            description: None,
            prompt: None,
            agent_type: None,
            model: None,
            thread_ids: Default::default(),
            result: None,
            progress: Default::default(),
            status: ItemStatus::InProgress,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WebSearchItem {
    pub id: String,
    pub query: String,
    pub action: Option<String>,
    pub url: Option<String>,
    pub results: Option<OpaqueJson>,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for WebSearchItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            query: Default::default(),
            action: None,
            url: None,
            results: None,
            status: ItemStatus::InProgress,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ImageItem {
    pub id: String,
    pub kind: ImageKind,
    pub path: Option<String>,
    pub url: Option<String>,
    pub prompt: Option<String>,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for ImageItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            kind: Default::default(),
            path: None,
            url: None,
            prompt: None,
            status: ItemStatus::Completed,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MarkerItem {
    pub id: String,
    pub kind: MarkerKind,
    pub text: Option<String>,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for MarkerItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            kind: Default::default(),
            text: None,
            status: ItemStatus::Completed,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct Notice {
    pub level: NoticeLevel,
    pub message: String,
    pub code: Option<String>,
    pub will_retry: bool,
    pub detail: Option<String>,
    pub raw: Option<OpaqueJson>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NoticeItem {
    pub id: String,
    pub notice: Notice,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for NoticeItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            notice: Default::default(),
            status: ItemStatus::Completed,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UnknownItem {
    pub id: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub status: ItemStatus,
    pub parent_id: Option<String>,
    pub raw: Option<OpaqueJson>,
}
impl Default for UnknownItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            type_name: Default::default(),
            status: ItemStatus::Completed,
            parent_id: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct RequestKey {
    pub backend: BackendKind,
    pub raw_id: OpaqueJson,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct Decision {
    pub id: String,
    pub kind: DecisionKind,
    pub detail: Option<String>,
    pub wire: Option<OpaqueJson>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct QuestionOption {
    pub label: String,
    pub description: Option<String>,
    pub preview: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct Question {
    pub id: String,
    pub question: String,
    pub header: Option<String>,
    pub options: Vec<QuestionOption>,
    pub multi_select: bool,
    pub allow_free_text: bool,
    pub secret: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PendingRequest {
    pub key: RequestKey,
    pub method: String,
    pub kind: RequestKind,
    pub decisions: Vec<Decision>,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub item_id: Option<String>,
    pub status: RequestStatus,
    pub answer: Option<String>,
    pub received_at_ms: Option<i64>,
    pub raw: Option<OpaqueJson>,
}
impl Default for PendingRequest {
    fn default() -> Self {
        Self {
            key: Default::default(),
            method: Default::default(),
            kind: Default::default(),
            decisions: Default::default(),
            thread_id: None,
            turn_id: None,
            item_id: None,
            status: RequestStatus::Pending,
            answer: None,
            received_at_ms: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type", rename_all_fields = "camelCase")]
pub enum RequestKind {
    CommandApproval {
        command: Option<String>,
        cwd: Option<String>,
        reason: Option<String>,
        actions: Vec<CommandAction>,
        network_host: Option<String>,
    },
    FileChangeApproval {
        reason: Option<String>,
        grant_root: Option<String>,
        changes: Vec<FileDelta>,
    },
    PermissionsApproval {
        reason: Option<String>,
        cwd: Option<String>,
        permissions: OpaqueJson,
    },
    ToolApproval {
        tool: String,
        display_name: Option<String>,
        title: Option<String>,
        description: Option<String>,
        input: Option<OpaqueJson>,
        blocked_path: Option<String>,
        reason: Option<String>,
        reason_type: Option<String>,
        default_to_no: bool,
        requires_user_interaction: bool,
        tool_use_id: Option<String>,
        agent_id: Option<String>,
        mcp_server: Option<OpaqueJson>,
    },
    UserInput {
        questions: Vec<Question>,
        auto_resolution_ms: Option<i64>,
    },
    PlanApproval {
        plan: String,
    },
    Elicitation {
        server: Option<String>,
        message: String,
        mode: String,
        url: Option<String>,
        schema: Option<OpaqueJson>,
        elicitation_id: Option<String>,
    },
    UserDialog {
        dialog_kind: String,
        payload: Option<OpaqueJson>,
    },
    Unknown {
        method: String,
        params: Option<OpaqueJson>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct ThreadKey {
    pub backend: BackendKind,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct McpServerStatus {
    pub name: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct UnknownRecord {
    pub kind: String,
    pub thread_id: Option<String>,
    pub raw: OpaqueJson,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BackendStatus {
    pub backend: BackendKind,
    pub process: ProcessState,
    pub server_info: Option<OpaqueJson>,
    pub account: AccountState,
    pub rate_limits: Option<RateLimitState>,
    pub models: Option<ModelCatalog>,
    pub mcp_servers: BTreeMap<String, McpServerStatus>,
    pub notices: Vec<Notice>,
    pub unknown: Vec<UnknownRecord>,
}
impl Default for BackendStatus {
    fn default() -> Self {
        Self {
            backend: Default::default(),
            process: ProcessState::Stopped {},
            server_info: None,
            account: Default::default(),
            rate_limits: None,
            models: None,
            mcp_servers: Default::default(),
            notices: Default::default(),
            unknown: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct ThreadSettings {
    pub model: Option<String>,
    pub effort: Option<String>,
    pub approval_policy: Option<String>,
    pub sandbox: Option<String>,
    pub approvals_reviewer: Option<String>,
    pub raw: Option<OpaqueJson>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub total_tokens: i64,
    pub context_window: Option<i64>,
    pub cost_usd: Option<f64>,
    pub raw: Option<OpaqueJson>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct TurnError {
    pub message: String,
    pub code: Option<String>,
    pub detail: Option<String>,
    pub raw: Option<OpaqueJson>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct TurnPlan {
    pub steps: Vec<PlanStep>,
    pub explanation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Turn {
    pub id: String,
    pub client_message_id: Option<String>,
    pub status: TurnStatus,
    pub items: Vec<Item>,
    pub error: Option<TurnError>,
    pub plan: Option<TurnPlan>,
    pub diff: Option<String>,
    pub usage: Option<TokenUsage>,
    pub settings: Option<TurnSettings>,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub thinking_tokens: Option<i64>,
    pub bound: bool,
}
impl Default for Turn {
    fn default() -> Self {
        Self {
            id: Default::default(),
            client_message_id: None,
            status: TurnStatus::Running,
            items: Default::default(),
            error: None,
            plan: None,
            diff: None,
            usage: None,
            settings: None,
            started_at_ms: None,
            completed_at_ms: None,
            duration_ms: None,
            thinking_tokens: None,
            bound: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ThreadState {
    pub key: ThreadKey,
    pub title: Option<String>,
    pub preview: Option<String>,
    pub cwd: Option<String>,
    pub path: Option<String>,
    pub forked_from: Option<String>,
    pub ephemeral: bool,
    pub archived: bool,
    pub deleted: bool,
    pub run_state: RunState,
    pub settings: ThreadSettings,
    pub turns: Vec<Turn>,
    pub usage: Option<TokenUsage>,
    pub notices: Vec<Notice>,
    pub unknown: Vec<UnknownRecord>,
    pub history_cursor: Option<String>,
    pub created_at_sec: Option<i64>,
    pub updated_at_sec: Option<i64>,
    pub raw: Option<OpaqueJson>,
}
impl Default for ThreadState {
    fn default() -> Self {
        Self {
            key: Default::default(),
            title: None,
            preview: None,
            cwd: None,
            path: None,
            forked_from: None,
            ephemeral: false,
            archived: false,
            deleted: false,
            run_state: RunState::Idle,
            settings: Default::default(),
            turns: Default::default(),
            usage: None,
            notices: Default::default(),
            unknown: Default::default(),
            history_cursor: None,
            created_at_sec: None,
            updated_at_sec: None,
            raw: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct AgentState {
    pub backends: BTreeMap<BackendKind, BackendStatus>,
    pub threads: Pairs<ThreadKey, ThreadState>,
    pub requests: Pairs<RequestKey, PendingRequest>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type", rename_all_fields = "camelCase")]
pub enum ProcessState {
    Stopped {},
    Starting {},
    Ready {},
    Exited {
        exit_code: Option<i32>,
        stderr_tail: String,
    },
    Failed {
        message: String,
        stderr_tail: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountState {
    pub state: LoginState,
    pub method: Option<String>,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub organization: Option<String>,
    pub requires_auth: Option<bool>,
    pub login: Option<LoginFlow>,
    pub raw: Option<OpaqueJson>,
    /// The latest failed or unanswered account read. Any answered read clears it; it never changes `state`.
    pub check_error: Option<String>,
}
impl Default for AccountState {
    fn default() -> Self {
        Self {
            state: LoginState::Unknown,
            method: None,
            email: None,
            plan: None,
            organization: None,
            requires_auth: None,
            login: None,
            raw: None,
            check_error: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct RateLimitWindow {
    pub used_percent: Option<f64>,
    pub window_minutes: Option<i64>,
    pub resets_at_epoch_sec: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct RateLimit {
    pub id: String,
    pub name: Option<String>,
    pub primary: Option<RateLimitWindow>,
    pub secondary: Option<RateLimitWindow>,
    pub status: Option<String>,
    pub reached: bool,
    pub raw: Option<OpaqueJson>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct RateLimitState {
    pub limits: Vec<RateLimit>,
    pub ordinary_usage_allowed: Option<bool>,
    pub upsell: Option<OpaqueJson>,
    pub raw: Option<OpaqueJson>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type", rename_all_fields = "camelCase")]
pub enum LoginFlow {
    DeviceCode {
        login_id: Option<String>,
        verification_url: String,
        user_code: String,
    },
    Browser {
        login_id: Option<String>,
        auth_url: String,
    },
    Terminal {
        argv: Vec<String>,
        env: BTreeMap<String, String>,
    },
    Progress {
        login_id: Option<String>,
        output: Vec<String>,
        error: Option<String>,
    },
    Completed {
        login_id: Option<String>,
        success: bool,
        error: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct EffortOption {
    pub id: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct ModelOption {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub efforts: Vec<EffortOption>,
    pub default_effort: Option<String>,
    pub is_default: bool,
    pub hidden: bool,
    pub input_modalities: Vec<String>,
    pub resolved_model: Option<String>,
    pub upgrade_to: Option<String>,
    pub upgrade_message: Option<String>,
    pub raw: Option<OpaqueJson>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct ModelCatalog {
    pub backend: BackendKind,
    pub models: Vec<ModelOption>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct ConversationEntry {
    pub id: String,
    pub backend: BackendKind,
    pub backend_thread_id: Option<String>,
    pub title: Option<String>,
    pub cwd: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub archived: bool,
    pub forked_from: Option<String>,
    pub forked_at: Option<String>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub preview: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct QueuedMessage {
    pub client_message_id: String,
    pub parts: Vec<UserPart>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type", rename_all_fields = "camelCase")]
pub enum ItemDelta {
    AgentText { text: String },
    ReasoningSummaryPart { index: i32 },
    ReasoningSummary { index: i32, text: String },
    ReasoningText { index: i32, text: String },
    PlanText { text: String },
    CommandOutput { text: String },
    TerminalInput { text: String },
    FileChangeOutput { text: String },
    FileChangePatch { changes: Vec<FileDelta> },
    ToolArguments { partial_json: String },
    ToolProgress { message: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type", rename_all_fields = "camelCase")]
pub enum AgentEvent {
    ProcessChanged {
        backend: BackendKind,
        state: ProcessState,
    },
    ServerInfo {
        backend: BackendKind,
        info: OpaqueJson,
    },
    AccountChanged {
        backend: BackendKind,
        account: AccountState,
    },
    LoginChanged {
        backend: BackendKind,
        flow: Option<LoginFlow>,
    },
    AccountCheckFailed {
        backend: BackendKind,
        message: String,
    },
    RateLimitsChanged {
        backend: BackendKind,
        limits: RateLimitState,
        merge: bool,
    },
    ModelsChanged {
        backend: BackendKind,
        catalog: ModelCatalog,
    },
    McpServerChanged {
        backend: BackendKind,
        status: McpServerStatus,
    },
    BackendNotice {
        backend: BackendKind,
        notice: Notice,
    },
    Unknown {
        backend: BackendKind,
        kind: String,
        thread_id: Option<String>,
        raw: OpaqueJson,
    },
    ThreadUpserted {
        backend: BackendKind,
        thread_id: String,
        title: Option<String>,
        preview: Option<String>,
        cwd: Option<String>,
        path: Option<String>,
        forked_from: Option<String>,
        ephemeral: Option<bool>,
        run_state: Option<RunState>,
        settings: Option<ThreadSettings>,
        created_at_sec: Option<i64>,
        updated_at_sec: Option<i64>,
        raw: Option<OpaqueJson>,
    },
    ThreadStatusChanged {
        backend: BackendKind,
        thread_id: String,
        run_state: RunState,
    },
    ThreadRenamed {
        backend: BackendKind,
        thread_id: String,
        title: Option<String>,
    },
    ThreadArchived {
        backend: BackendKind,
        thread_id: String,
        archived: bool,
    },
    ThreadDeleted {
        backend: BackendKind,
        thread_id: String,
    },
    ThreadClosed {
        backend: BackendKind,
        thread_id: String,
    },
    ThreadSettingsChanged {
        backend: BackendKind,
        thread_id: String,
        settings: ThreadSettings,
    },
    TokenUsageChanged {
        backend: BackendKind,
        thread_id: String,
        turn_id: Option<String>,
        usage: TokenUsage,
    },
    ThreadNotice {
        backend: BackendKind,
        thread_id: String,
        notice: Notice,
    },
    HistoryLoaded {
        backend: BackendKind,
        thread_id: String,
        turns: Vec<Turn>,
        prepend: bool,
        cursor: Option<String>,
    },
    QueueUpdated {
        backend: BackendKind,
        thread_id: String,
        messages: Vec<QueuedMessage>,
        previously_queued: Vec<String>,
    },
    TurnSubmitted {
        backend: BackendKind,
        thread_id: String,
        client_message_id: String,
        parts: Vec<UserPart>,
        settings: Option<TurnSettings>,
        at_ms: Option<i64>,
    },
    TurnBound {
        backend: BackendKind,
        thread_id: String,
        client_message_id: String,
        turn_id: String,
    },
    TurnStarted {
        backend: BackendKind,
        thread_id: String,
        turn_id: String,
        client_message_id: Option<String>,
        at_ms: Option<i64>,
    },
    TurnCancelled {
        backend: BackendKind,
        thread_id: String,
        turn_id: String,
    },
    TurnCompleted {
        backend: BackendKind,
        thread_id: String,
        turn_id: String,
        status: TurnStatus,
        error: Option<TurnError>,
        items: Vec<Item>,
        duration_ms: Option<i64>,
        usage: Option<TokenUsage>,
        at_ms: Option<i64>,
    },
    TurnPlanUpdated {
        backend: BackendKind,
        thread_id: String,
        turn_id: String,
        plan: TurnPlan,
    },
    TurnDiffUpdated {
        backend: BackendKind,
        thread_id: String,
        turn_id: String,
        diff: String,
    },
    TurnProgress {
        backend: BackendKind,
        thread_id: String,
        turn_id: String,
        thinking_tokens: Option<i64>,
    },
    TurnNotice {
        backend: BackendKind,
        thread_id: String,
        turn_id: Option<String>,
        notice: Notice,
    },
    ItemStarted {
        backend: BackendKind,
        thread_id: String,
        turn_id: String,
        item: Item,
    },
    ItemUpdated {
        backend: BackendKind,
        thread_id: String,
        turn_id: String,
        item_id: String,
        delta: ItemDelta,
    },
    ItemCompleted {
        backend: BackendKind,
        thread_id: String,
        turn_id: String,
        item: Item,
    },
    ItemDeclined {
        backend: BackendKind,
        thread_id: String,
        turn_id: Option<String>,
        item_id: String,
    },
    RequestOpened {
        request: PendingRequest,
    },
    RequestClosed {
        key: RequestKey,
        status: RequestStatus,
        answer: Option<String>,
    },
}

impl AgentEvent {
    pub fn backend(&self) -> BackendKind {
        match self {
            Self::ProcessChanged { backend, .. } => *backend,
            Self::ServerInfo { backend, .. } => *backend,
            Self::AccountChanged { backend, .. } => *backend,
            Self::LoginChanged { backend, .. } => *backend,
            Self::AccountCheckFailed { backend, .. } => *backend,
            Self::RateLimitsChanged { backend, .. } => *backend,
            Self::ModelsChanged { backend, .. } => *backend,
            Self::McpServerChanged { backend, .. } => *backend,
            Self::BackendNotice { backend, .. } => *backend,
            Self::Unknown { backend, .. } => *backend,
            Self::ThreadUpserted { backend, .. } => *backend,
            Self::ThreadStatusChanged { backend, .. } => *backend,
            Self::ThreadRenamed { backend, .. } => *backend,
            Self::ThreadArchived { backend, .. } => *backend,
            Self::ThreadDeleted { backend, .. } => *backend,
            Self::ThreadClosed { backend, .. } => *backend,
            Self::ThreadSettingsChanged { backend, .. } => *backend,
            Self::TokenUsageChanged { backend, .. } => *backend,
            Self::ThreadNotice { backend, .. } => *backend,
            Self::HistoryLoaded { backend, .. } => *backend,
            Self::QueueUpdated { backend, .. } => *backend,
            Self::TurnSubmitted { backend, .. } => *backend,
            Self::TurnBound { backend, .. } => *backend,
            Self::TurnStarted { backend, .. } => *backend,
            Self::TurnCancelled { backend, .. } => *backend,
            Self::TurnCompleted { backend, .. } => *backend,
            Self::TurnPlanUpdated { backend, .. } => *backend,
            Self::TurnDiffUpdated { backend, .. } => *backend,
            Self::TurnProgress { backend, .. } => *backend,
            Self::TurnNotice { backend, .. } => *backend,
            Self::ItemStarted { backend, .. } => *backend,
            Self::ItemUpdated { backend, .. } => *backend,
            Self::ItemCompleted { backend, .. } => *backend,
            Self::ItemDeclined { backend, .. } => *backend,
            Self::RequestOpened { request, .. } => request.key.backend,
            Self::RequestClosed { key, .. } => key.backend,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum Item {
    #[serde(rename = "UserMessageItem")]
    UserMessage(UserMessageItem),
    #[serde(rename = "AgentMessageItem")]
    AgentMessage(AgentMessageItem),
    #[serde(rename = "ReasoningItem")]
    Reasoning(ReasoningItem),
    #[serde(rename = "PlanItem")]
    Plan(PlanItem),
    #[serde(rename = "CommandItem")]
    Command(CommandItem),
    #[serde(rename = "FileChangeItem")]
    FileChange(FileChangeItem),
    #[serde(rename = "ToolCallItem")]
    ToolCall(ToolCallItem),
    #[serde(rename = "SubAgentItem")]
    SubAgent(SubAgentItem),
    #[serde(rename = "WebSearchItem")]
    WebSearch(WebSearchItem),
    #[serde(rename = "ImageItem")]
    Image(ImageItem),
    #[serde(rename = "MarkerItem")]
    Marker(MarkerItem),
    #[serde(rename = "NoticeItem")]
    Notice(NoticeItem),
    #[serde(rename = "UnknownItem")]
    Unknown(UnknownItem),
}

impl Item {
    pub fn id(&self) -> &str {
        match self {
            Self::UserMessage(v) => &v.id,
            Self::AgentMessage(v) => &v.id,
            Self::Reasoning(v) => &v.id,
            Self::Plan(v) => &v.id,
            Self::Command(v) => &v.id,
            Self::FileChange(v) => &v.id,
            Self::ToolCall(v) => &v.id,
            Self::SubAgent(v) => &v.id,
            Self::WebSearch(v) => &v.id,
            Self::Image(v) => &v.id,
            Self::Marker(v) => &v.id,
            Self::Notice(v) => &v.id,
            Self::Unknown(v) => &v.id,
        }
    }
}

impl Item {
    pub fn status(&self) -> ItemStatus {
        match self {
            Self::UserMessage(v) => v.status,
            Self::AgentMessage(v) => v.status,
            Self::Reasoning(v) => v.status,
            Self::Plan(v) => v.status,
            Self::Command(v) => v.status,
            Self::FileChange(v) => v.status,
            Self::ToolCall(v) => v.status,
            Self::SubAgent(v) => v.status,
            Self::WebSearch(v) => v.status,
            Self::Image(v) => v.status,
            Self::Marker(v) => v.status,
            Self::Notice(v) => v.status,
            Self::Unknown(v) => v.status,
        }
    }
}

impl Item {
    pub fn parent_id(&self) -> Option<&str> {
        match self {
            Self::UserMessage(v) => v.parent_id.as_deref(),
            Self::AgentMessage(v) => v.parent_id.as_deref(),
            Self::Reasoning(v) => v.parent_id.as_deref(),
            Self::Plan(v) => v.parent_id.as_deref(),
            Self::Command(v) => v.parent_id.as_deref(),
            Self::FileChange(v) => v.parent_id.as_deref(),
            Self::ToolCall(v) => v.parent_id.as_deref(),
            Self::SubAgent(v) => v.parent_id.as_deref(),
            Self::WebSearch(v) => v.parent_id.as_deref(),
            Self::Image(v) => v.parent_id.as_deref(),
            Self::Marker(v) => v.parent_id.as_deref(),
            Self::Notice(v) => v.parent_id.as_deref(),
            Self::Unknown(v) => v.parent_id.as_deref(),
        }
    }
}

impl Item {
    pub fn set_status(&mut self, status: ItemStatus) {
        match self {
            Self::UserMessage(v) => v.status = status,
            Self::AgentMessage(v) => v.status = status,
            Self::Reasoning(v) => v.status = status,
            Self::Plan(v) => v.status = status,
            Self::Command(v) => v.status = status,
            Self::FileChange(v) => v.status = status,
            Self::ToolCall(v) => v.status = status,
            Self::SubAgent(v) => v.status = status,
            Self::WebSearch(v) => v.status = status,
            Self::Image(v) => v.status = status,
            Self::Marker(v) => v.status = status,
            Self::Notice(v) => v.status = status,
            Self::Unknown(v) => v.status = status,
        }
    }
}
impl LoginFlow {
    pub fn login_id(&self) -> Option<&str> {
        match self {
            Self::DeviceCode { login_id, .. }
            | Self::Browser { login_id, .. }
            | Self::Progress { login_id, .. }
            | Self::Completed { login_id, .. } => login_id.as_deref(),
            Self::Terminal { .. } => None,
        }
    }
    /// A waiting attempt: device code, browser, or an unanswered start (`Progress` with an ID and no error).
    pub fn is_pending(flow: Option<&Self>) -> bool {
        matches!(
            flow,
            Some(Self::DeviceCode { .. })
                | Some(Self::Browser { .. })
                | Some(Self::Progress {
                    login_id: Some(_),
                    error: None,
                    ..
                })
        )
    }
    /// A successful completion reported for the current process.
    pub fn is_confirmed_success(flow: Option<&Self>) -> bool {
        matches!(flow, Some(Self::Completed { success: true, .. }))
    }
}
impl Default for ProcessState {
    fn default() -> Self {
        Self::Stopped {}
    }
}
impl Default for RequestKind {
    fn default() -> Self {
        Self::Unknown {
            method: String::new(),
            params: None,
        }
    }
}
impl BackendKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}
impl TurnStatus {
    pub fn is_final(self) -> bool {
        !matches!(self, Self::Queued | Self::Running)
    }
}
impl ItemStatus {
    pub fn is_final(self) -> bool {
        self != Self::InProgress
    }
}
impl UserMessageItem {
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                UserPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
impl ThreadState {
    pub fn new(key: ThreadKey) -> Self {
        Self {
            key,
            run_state: RunState::Idle,
            ..Self::default()
        }
    }
}
