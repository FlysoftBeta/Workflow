# Agent backends research: Codex App-Server + Claude Code (2026-09-26/27)

Role: agent-backend protocol researcher. Scope: what the chat UI must speak to reach "complete
functionality" with **both** Codex App-Server and Claude Code, what their real wire format looks
like, whether Claude Code can run on the Android 9 tablet, and one backend-neutral conversation
model the UI can render.

Raw evidence lives in `artifacts/agent-research/` (git-ignored). Nothing was executed on the tablet.

## 0. Key findings (read this first)

1. **Codex schema is current.** Bundled/host Codex is 0.157.0. The latest stable, 0.157.1 (published
   2026-09-26), generates a **byte-identical** experimental schema (167 client requests / 11 server
   requests / 83 notifications / 1 client notification / 795 definitions). The 0.159.0-alpha.4
   schema adds or removes no methods; 2 definitions are added, 8 `Plugin*` definitions are removed,
   and 5 definitions change. No inventory update is needed for 0.157.1.
2. **Codex real turns captured** (`codex-session.jsonl`): markdown + LaTeX, a command approval
   round-trip (accepted inside the sandbox; file really written), interrupt, and thread
   read/list/name/fork/archive. Four protocol facts that the current client must respect:
   - **`approvalsReviewer` is inherited from `config.toml`.** The host config resolved to
     `"auto_review"`, so a subagent approves on the user's behalf. The app must send
     `approvalsReviewer: "user"` explicitly on thread/start, resume, fork and turn/start, or it
     silently violates "never auto-approve".
   - **Server request IDs share the numeric space with client IDs.** The first approval arrived
     with `"id": 0`. Pending maps must be keyed by direction.
   - **`availableDecisions` varies per request.** This one offered `accept`,
     `acceptWithExecpolicyAmendment` and `cancel`, but no `decline` or `acceptForSession`. Render
     only the listed decisions.
   - **An interrupted turn can leave a dangling item.** `turn/completed{status:"interrupted", items:[]}`
     arrived with an `agentMessage` that was started but never completed. The reducer must
     finalize it.
3. **Claude Code:** the host CLI is 2.1.220 and is **not logged in**. This agent's own session auth
   comes from the desktop app and was deliberately not reused. Latest is 2.1.283 (stable channel
   2.1.274); Agent SDK is 0.3.283. I drove the **real 2.1.283 CLI** in stream-json/control mode
   against a **local scripted Messages-API mock** with a dummy key. Every CLI frame is genuine, the
   model replies are scripted, and no real model call was made. The capture covers a
   markdown/math + image turn, `can_use_tool` allow and deny, a hook callback, model/effort/mode
   switches, interrupt, rename, and resume + fork. A separate no-auth run records the
   "Not logged in" shape.
4. **Claude on Android.** No Android build is published. The code contains a `linux-arm64-android`
   platform stub, but `isAndroidEnvironment()` is compiled to `false`. Both arm64 artifacts are
   Bun 1.4.3 single-file executables and are **dynamically linked**:
   - glibc build: interpreter `/lib/ld-linux-aarch64.so.1`, needs GLIBC ≤ 2.26 symbols.
   - musl build: interpreter `/lib/ld-musl-aarch64.so.1`, needs `libc.musl-aarch64.so.1` only.

   The Bash tool requires **bash or zsh** (Android's mksh is rejected). Recommended path: run
   Claude Code **inside the Debian guest** (native engine or opt-in proot). A container-less path
   (bundled musl loader + `libclaude.so`) is possible in principle; the explicit-loader launch was
   verified on the host. It is high-risk on Android 9 because the app seccomp filter sends
   `SIGSYS` rather than `ENOSYS` for syscalls outside the API-28 allowlist, and it needs a device
   probe first.
5. **One conversation model covers both backends.** Codex is item-centric (`item/*` lifecycle).
   Claude is content-block-centric (`stream_event` + `assistant`/`user` messages + `control_request`).
   §5 gives a neutral `Thread/Turn/Item/ServerRequest` model with mapping tables (§6) and a
   prioritized "complete functionality" checklist (§7).

## 1. Versions and sources

| Component | Host | Bundled | Latest | Source |
|---|---|---|---|---|
| Codex CLI | `codex-cli 0.157.0` (`~/.local/bin/codex`, logged in with ChatGPT, plan `prolite`) | 0.157.0 aarch64 musl static (`runtime/vendor/codex`, `jniLibs/arm64-v8a/libcodex.so`) | **0.157.1** stable (2026-09-26); `0.159.0-alpha.4` alpha | `npm view @openai/codex dist-tags`; `gh release view rust-v0.157.1 -R openai/codex` |
| Codex aarch64 musl assets (0.157.1) | — | — | `codex-…` 100 MB gz; **`codex-app-server-…` 77.5 MB gz** (standalone app-server); `codex-code-mode-host-…` 26 MB gz; `bwrap-…` | GitHub release assets |
| Claude Code | `2.1.220` native (`~/.local/share/claude/versions/2.1.220`), **not logged in** | — | **2.1.283** latest, 2.1.274 stable | `npm view @anthropic-ai/claude-code`; `https://downloads.claude.ai/claude-code-releases/{latest,stable}` |
| Claude Agent SDK (TS) | desktop host uses 0.3.281 | — | 0.3.283 | `npm pack @anthropic-ai/claude-agent-sdk` → `sdk.d.ts`, `sdk.mjs` |
| Claude native arm64 | — | — | `linux-arm64` 240.9 MB, `linux-arm64-musl` 233.3 MB (checksums match signed-manifest values) | `npm pack @anthropic-ai/claude-code-linux-arm64{,-musl}@2.1.283`; `claude-manifest-2.1.283.json` |

Official docs consulted: `code.claude.com/docs/en/headless`, `/setup`, `/agent-sdk/user-input`;
Codex docs (`learn.chatgpt.com/docs/app-server`, referenced by the existing inventory) plus the
generated schema. Bun: "runs on kernels as old as 3.10 … with graceful degradation of newer
syscalls" (bun.sh/docs/installation). That degradation assumes `ENOSYS`, which Android's filter
does not return (see §4).

## 2. Codex App-Server

### 2.1 Schema diff

Command:

```
codex app-server generate-json-schema --experimental --out artifacts/agent-research/schema/<ver>
python3 artifacts/agent-research/schema_diff.py 0.157.0 0.157.1 0.159.0-alpha.4
```

Output: `artifacts/agent-research/schema/diff.json`.

| Version | Methods added / removed (all four groups) | Definitions |
|---|---|---|
| 0.157.0 (host) | none (identical to `runtime/schemas/inventory.json`) | 795, identical |
| **0.157.1** | none; `diff -r` against `runtime/schemas` is byte-identical | 795, identical |
| 0.159.0-alpha.4 | none | 789. Added `ThreadItemsListAnchor`, `ThreadItemsListCursor`. Removed `PluginEntrypoint`, `PluginExtensions`, `PluginIcon`, `PluginQuickAction`, `PluginQuickActionTarget`, `PluginSearchProvider`, `PluginSearchProviderCall`, `PluginSettings`. Changed `CodexErrorInfo`, `EnvironmentAddParams`, `PlanType`, `PluginSummary`, `ThreadItemsListParams`. |

Action: 0.157.1 is a drop-in binary update; keep the 0.157.0 inventory, or regenerate with
`tools/update-protocol.py` after switching. When moving to 0.159, re-check `CodexErrorInfo`
(error rendering) and `thread/items/list` paging, which the history view depends on.

### 2.2 Live run: what the wire actually looks like

Driver: `artifacts/agent-research/codex_probe.py`. It spawns `codex app-server` over stdio with
cwd `sandbox/codex-run`. It accepts a command approval only if the cwd is inside the sandbox and
the command matches a safe pattern. Everything else is declined or answered with a JSON-RPC
error. Every in/out frame is logged, with email, account ID and installation ID redacted.

Transcripts:

- `codex-handshake.jsonl`: initialize, account, rate limits, model list.
- `codex-handshake-hidden.jsonl`: `includeHidden` models and `supportsLunaReserve`.
- `codex-session-usage-limit.jsonl`: the first attempt. All 3 turns failed immediately with
  `usageLimitExceeded`, before any inference.
- **`codex-session.jsonl`**: 3 small real turns on `gpt-reserve`, effort `low`.

Real model use was 3 small turns: about 45k input tokens (mostly cached) and 367 output tokens,
plus one interrupted turn.

Observed sequence (condensed; `→` is client to server):

```
→ initialize {clientInfo, capabilities:{experimentalApi:true}}
← result {userAgent, codexHome, platformFamily, platformOs}
← remoteControl/status/changed            (notification can precede `initialized`)
→ initialized
→ account/read {refreshToken:false}        ← account/updated {authMode:"chatgpt", planType}
                                            ← result {account:{type,email,planType}, requiresOpenaiAuth, workspaceRouting}
→ account/rateLimits/read                  ← {ordinaryUsageAllowed:false, rateLimits{primary{usedPercent:100,…}},
                                              rateLimitsByLimitId{codex, base_model_inference("gpt-reserve")},
                                              rateLimitUpsell{banner_type:"luna_reserve", title, ctas…}}
→ model/list                               ← data[] {id, displayName, isDefault, defaultReasoningEffort,
                                              supportedReasoningEfforts[{reasoningEffort, description}],
                                              inputModalities:["text","image"], serviceTiers[{id:"priority",name:"Fast"}]}
→ thread/start {cwd, model, approvalPolicy:"untrusted", sandbox:"read-only", ephemeral:false}
← result {thread{id, sessionId, path, historyMode:"paginated", status, cwd, cliVersion, source:"vscode",…},
          model, approvalPolicy, approvalsReviewer:"auto_review" (!), sandbox, reasoningEffort:"max" (from config),
          instructionSources:[~/.codex/AGENTS.md, <git root>/AGENTS.md]}
← thread/started ; mcpServer/startupStatus/updated (starting→ready, per configured MCP server)
→ turn/start {threadId, input:[{type:"text", text, text_elements:[]}], model, effort:"low", summary:"auto"}
← thread/settings/updated {threadSettings{… effort:"low", collaborationMode…}}
← result {turn{id, status:"inProgress", itemsView:"notLoaded"}}
← thread/status/changed {active, activeFlags:[]} ; turn/started
← item/started+completed userMessage
← item/started reasoning ; item/reasoning/summaryPartAdded ; item/reasoning/summaryTextDelta ; item/completed reasoning
← item/started agentMessage {phase:"final_answer", text:""} ; item/agentMessage/delta ×49 ; item/completed agentMessage
← thread/tokenUsage/updated {total,last,modelContextWindow} ; account/rateLimits/updated
← thread/status/changed {idle} ; turn/completed {turn{status:"completed", items:[final agentMessage], durationMs}}
```

The final text of turn 1:

```
## Key Formulas
- Euler’s identity: $e^{i\pi}+1=0$
- Quadratic formula:
$$x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}$$
```

That is `$…$` inline and `$$…$$` display math, exactly what the KaTeX pipeline must handle while
it is still streaming.

**Approval turn** (turn 2, approval policy `untrusted`, read-only sandbox):

```
← item/completed agentMessage {phase:"commentary", text:"I’ll create the file with `printf`…"}
← thread/status/changed {active, activeFlags:["waitingOnApproval"]}
← item/started commandExecution {id:"exec-…", command:"/bin/zsh -lc \"printf …\"", cwd, commandActions:[{type:"unknown",…}],
                                 status:"inProgress", source:"agent"}
← {"method":"item/commandExecution/requestApproval","id":0,"params":{kind:"command", threadId, turnId, itemId,
    reason, command, cwd, commandActions, proposedExecpolicyAmendment:[…],
    availableDecisions:["accept", {"acceptWithExecpolicyAmendment":{…}}, "cancel"]}}
→ {"id":0,"result":{"decision":"accept"}}
← serverRequest/resolved {threadId, requestId:0}
← thread/status/changed {activeFlags:[]}
← item/completed commandExecution {status:"completed", exitCode:0, processId, aggregatedOutput:null}
← … second reasoning + agentMessage(final_answer) … turn/completed
```

**Interrupt** (turn 3): `turn/interrupt {threadId, turnId}` returned `{}`. Then:

- `thread/status/changed idle`
- `turn/completed {status:"interrupted", items:[]}`
- **no** `item/completed` for the agent message that had already streamed about 60 deltas.

**Thread operations** (no model):

| Request | Observed response |
|---|---|
| `thread/read{includeTurns:true}` | Works, but first emits `deprecationNotice` ("Full-history hydration is deprecated…"). Use `thread/turns/list` + `thread/items/list`. |
| `thread/turns/list` | `{data, nextCursor, backwardsCursor}` |
| `thread/list{cwd}` | Works |
| `thread/name/set` | `{}` then `thread/name/updated` |
| `thread/fork{ephemeral:true, excludeTurns:true}` | New thread plus `thread/started` |
| `thread/archive` | `{}`, `thread/status/changed notLoaded`, `thread/archived` |

**Usage-limit failure shape** (first attempt):

```
← account/rateLimits/updated
← thread/status/changed {type:"systemError"}
← error {error{message, codexErrorInfo:"usageLimitExceeded"}, willRetry:false, threadId, turnId}
← turn/completed {status:"failed", error{…}}
```

The UI needs a rate-limit/upsell surface. `rateLimitUpsell` is backend-authored copy (render it as
data). The hidden `gpt-reserve` model is the ChatGPT "Luna Reserve" fallback that
`supportsLunaReserve` refers to.

### 2.3 Rules for the Workflow Codex client (derived)

- Always send `approvalsReviewer:"user"`. Choose `approvalPolicy` and `sandbox` explicitly; never
  inherit them.
- Run the app's Codex with its own `CODEX_HOME` under `.workspace`, as `LocalCodexSession`
  already does, so the user's desktop `config.toml` (MCP servers, `auto_review`,
  `model_reasoning_effort=max`) does not leak in.
- Key pending server requests by the raw JSON `id` value (int or string, echoed verbatim) in a
  map separate from client requests. Clear a request's card on `serverRequest/resolved`, on
  `turn/completed` or on process exit. Unknown server-request methods get a JSON-RPC error
  (`-32601`), never a default "accept".
- `availableDecisions` drives the buttons. `acceptWithExecpolicyAmendment` / `acceptForSession` are
  **persistent grants** and need distinct, explicit wording.
- `agentMessage.phase`:
  - `commentary` is interim narration (the "during answer" view).
  - `final_answer` is the final response (the ChatGPT "final" view).
  - Treat unknown phases as final.
- `item/started` for `commandExecution` precedes the approval request for the same `itemId`, so
  the approval card attaches to an existing item.
- Login options:
  - `account/login/start{type:"chatgptDeviceCode"}` returns `{verificationUrl, userCode}`. This is
    the most tablet-friendly.
  - `{type:"chatgpt"}` returns `authUrl` and needs the localhost callback.
  - `{type:"apiKey"}`.
- DNS/CA on Android is already solved by `CodexNetworkBridge` (loopback CONNECT + `SSL_CERT_FILE`).
- The standalone `codex-app-server` aarch64 build (77.5 MB gz vs 100 MB) could shrink the APK.
  `codex-code-mode-host` is optional (`features.code_mode_host`).

## 3. Claude Code headless protocol

### 3.1 How it was captured

`artifacts/agent-research/claude_probe.py` runs the real CLI 2.1.283
(`bin/cc/claude-code-linux-x64-2.1.283/package/claude`) with:

- a **clean environment**: none of the desktop host's `CLAUDE_CODE_*` / messaging tokens;
- isolated `HOME` and `CLAUDE_CONFIG_DIR` under `sandbox/claude-home`;
- cwd `sandbox/claude-run`;
- `ANTHROPIC_BASE_URL=http://127.0.0.1:18765` and a dummy `ANTHROPIC_API_KEY`, served by
  `claude_mock_api.py`.

The mock streams genuine Messages-API SSE: thinking + signature, text, and `input_json_delta` tool
use. It logs every request the CLI makes (`claude-mock-api-requests.jsonl`, with system prompt and
tool schemas elided).

Transcripts:

- `claude-session-mock.jsonl`: the main session.
- `claude-session-mock-fork.jsonl`: `--resume <id> --fork-session`.
- `claude-session-noauth.jsonl`: no key, which gives the "Not logged in" shape.

### 3.2 Launch contract

```
claude -p --input-format stream-json --output-format stream-json --verbose \
       --include-partial-messages --replay-user-messages \
       --permission-prompt-tool stdio --permission-mode default \
       [--model <m>] [--effort low|medium|high|xhigh|max] [--resume <id> [--fork-session] [--resume-session-at <uuid>]] \
       [--session-id <uuid>] [--mcp-config <json>] [--add-dir <d>] [--setting-sources …] [--bare]
```

- `--permission-prompt-tool stdio` is what the SDK's `canUseTool` sets. It is hidden from
  `--help` but present in `sdk.mjs`.
- stdin and stdout carry NDJSON; stderr is diagnostics only.
- Recommended environment: `CLAUDE_CONFIG_DIR` inside `.workspace`, `DISABLE_AUTOUPDATER=1`,
  `DISABLE_TELEMETRY=1`, `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`, `TMPDIR` and
  `CLAUDE_CODE_TMPDIR`.
- The CLI opens a messaging socket under `$TMPDIR` (seen as `/tmp/cc-socks/<pid>.sock`).

### 3.3 Frames: client → CLI

**User turn:**

```json
{"type":"user","uuid":"<client uuid>","session_id":"","parent_tool_use_id":null,
 "message":{"role":"user","content":[{"type":"text","text":"…"},
   {"type":"image","source":{"type":"base64","media_type":"image/png","data":"…"}}]}}
```

- The image reached the API unchanged (verified in the mock log).
- `document` blocks (PDF) follow the same Messages-API shape.
- Other files: reference them by workspace path in the text (`@path`); the CLI expands it.
- Optional fields: `priority:"now"|"next"|"later"` (queue position) and `shouldQuery:false`
  (append without starting a turn).
- The client `uuid` is echoed back in `command_lifecycle`, the replayed user message and
  `result.user_message_uuid`. That is the turn correlation key.

**Control request:**

```json
{"type":"control_request","request_id":"<unique>","request":{"subtype":…}}
```

Answered by:

```json
{"type":"control_response","response":{"subtype":"success"|"error","request_id":…,"response":{…}}}
```

Subtypes exercised (all succeeded):

| Subtype | Response observed |
|---|---|
| `initialize{hooks?, …}` | `{commands[43], agents[], output_style, available_output_styles, models[], account{tokenSource, apiKeySource, apiProvider}, pid, current_permission_mode, hooks_applied, fast_mode_state, session_state, …}`. Since 2.1.268, also `pending_permission_requests` and `pending_user_dialog_requests` for re-attaching. |
| `mcp_status` | `{mcpServers:[]}` |
| `list_models` | `ModelInfo{value, resolvedModel, displayName, description, supportsEffort, supportedEffortLevels[], supportsAdaptiveThinking, supportsFastMode, supportsAutoMode}` |
| `get_settings` | `{effective, sources, applied{model, effort}}` |
| `get_context_usage{detail:"summary"}` | Category breakdown (context meter) |
| `set_model{model:"sonnet"}` | Success. Also emits a synthetic user message with `<local-command-stdout>Set model to …`. |
| `apply_flag_settings{settings:{effortLevel:"high"}}` | Success; this is the **per-session effort** control |
| `set_max_thinking_tokens` | Success. No effect on the adaptive-thinking model: the request still used `thinking:{type:"adaptive"}`. |
| `set_permission_mode{mode}` | `{mode}` plus `system/status{permissionMode}` |
| `rename_session{title, source:"host"}` | Success; persisted as `custom-title` + `agent-name` in the transcript |
| `interrupt{cancel_queued?}` | `{still_queued:[]}` |

Others in the SDK union: `cancel_async_message`, `stop_task`, `background_tasks`,
`rewind_files`, `mcp_toggle`, `mcp_reconnect`, `mcp_set_servers`, `mcp_call`,
`mcp_read_resource`, `get_usage` (plan limits: five_hour / seven_day utilization),
`get_session_cost`, `list_permission_rules`, `file_suggestions` (@-mention autocomplete),
`read_file`, `reload_*`, `update_settings`, `get_binary_version`.

Either side may withdraw a request with
`{"type":"control_cancel_request","request_id":…}`; it gets no reply.

### 3.4 Frames: CLI → client

| `type` / `subtype` | Meaning | Seen |
|---|---|---|
| `system/init` | Per turn: `session_id`, `model`, `permissionMode`, `tools[]`, `mcp_servers[]`, `skills`, `agents`, `plugins`, `capabilities:["interrupt_receipt_v1","interrupt_cancel_queued_v1","msg_lifecycle_v1","mcp_read_resource_v1","mcp_tool_ui_meta_v1"]`, `claude_code_version`, `apiKeySource`, `memory_paths`, `per_turn_effort_active` | yes |
| `command_lifecycle{command_uuid, state}` | Per user message: `queued` → `started` → `completed` / `cancelled` | yes |
| `system/status{status:"requesting"\|null, permissionMode?}` | Activity and mode change | yes |
| `system/thinking_tokens{estimated_tokens, …}` | Thinking progress counter | yes |
| `system/dev_intent{kind:"android_app"}` | Project scan hint. Harmless; preserve as unknown. | yes |
| `stream_event{event}` | Raw Anthropic SSE: `message_start`, `content_block_start`, `content_block_delta` (`text_delta` / `thinking_delta` / `signature_delta` / `input_json_delta`), `content_block_stop`, `message_delta`, `message_stop` | yes |
| `assistant{message{id, content:[ONE block]}}` | One frame **per completed content block**, all sharing `message.id` | yes |
| `user` | Replay of our message (`isReplay:true`, same uuid), `tool_result` blocks (`is_error` on deny), `[Request interrupted by user]`, local-command output | yes |
| `result{subtype, is_error, result, usage, modelUsage, total_cost_usd, permission_denials[], terminal_reason, user_message_uuid(s), num_turns, ttft_ms…}` | End of a turn. `success`, or one of `error_during_execution`, `error_max_turns`, `error_max_budget_usd`, `error_max_structured_output_retries`. | yes |
| `control_request{can_use_tool \| hook_callback \| elicitation \| request_user_dialog \| mcp_message}` | Server requests | `can_use_tool`, `hook_callback` |
| `system/api_retry`, `system/compact_boundary`, `system/hook_*`, `system/task_started` / `_updated` / `_progress` / `_notification`, `system/background_tasks_changed`, `system/permission_denied`, `system/session_state_changed{idle\|running\|requires_action}`, `tool_progress`, `rate_limit_event{rate_limit_info{status, rateLimitType, utilization, resetsAt…}}`, `auth_status`, `prompt_suggestion`, `system/control_request_progress` | From `sdk.d.ts` (SDKMessage union of about 40 kinds) | typed only |

**Echo quirk.** The CLI re-emitted our own `control_response` frames on stdout (seen for
`hook_callback` and `can_use_tool`). A client must ignore control responses whose `request_id` it
is not waiting on. The SDK documents exactly this rule.

### 3.5 Permission round-trip (observed)

```
← assistant {content:[{type:"tool_use", id:"toolu_…", name:"Write", input:{file_path, content}}]}
← control_request {request_id:"9181…", request:{subtype:"hook_callback", callback_id:"hook_pre_1",
                   input:{hook_event_name:"PreToolUse", tool_name, tool_input, permission_mode, effort, transcript_path…}}}
→ control_response {success, request_id:"9181…", response:{continue:true}}
← control_request {request_id:"c071…", request:{subtype:"can_use_tool", tool_name:"Write", display_name:"Write",
                   input:{…}, description:"hello.txt", tool_use_id,
                   permission_suggestions:[{type:"setMode", mode:"acceptEdits", destination:"session"}]}}
→ control_response {success, request_id:"c071…", response:{behavior:"allow", updatedInput:{…}}}
← user {content:[{type:"tool_result", tool_use_id, content:"File created successfully at: …"}]}
```

The file was really written in the sandbox.

**Deny path (Bash):**

- The request carried `blocked_path` and two suggestions: `addRules{toolName:"Bash",
  ruleContent:"printf 'x' > bash.txt"}` to `localSettings`, and `addDirectories` to `session`.
- Reply: `{behavior:"deny", message}`.
- Effect: a `tool_result{is_error:true, content:message}` and `result.permission_denials[]`
  listing the call.

Other `can_use_tool` fields to render:

- `title`, `decision_reason` (can contain ANSI escapes; sanitize), `decision_reason_type`,
  `classifier_approvable`
- `suppress_always_allow_rule`: hide "always allow"
- `default_to_no`: do not preselect approve
- `requires_user_interaction`, `matched_ask_rule`, `agent_id`, `mcp_server`

The same channel carries two tools that are really UI requests:

- **`AskUserQuestion`.** Input is `questions[{question, header, options[{label, description,
  preview?}], multiSelect}]`. Answer with `allow` and
  `updatedInput:{questions, answers:{<question text>: <label | [labels] | free text>}, response?}`.
- **`ExitPlanMode`.** A plan-approval card; `input.plan` is markdown.

A pending prompt blocks the turn indefinitely; there is no timeout.

"Approve and remember" means echoing a suggestion into `updatedPermissions`. Default to one-shot
approval, and require an explicit user action for any `localSettings` / `userSettings`
persistence.

### 3.6 Model, effort and thinking

- `list_models` gives the slider data:

  | `value` | Resolves to | Notes |
  |---|---|---|
  | `default` | `claude-opus-5-5[1m]` | |
  | `opus[1m]` | | |
  | `claude-fable-5-1` | | |
  | `sonnet` | | |
  | `sonnet[1m]` | | |
  | … | | each row has `supportedEffortLevels` = low / medium / high / xhigh / max |

- The API requests the CLI generated confirm the mapping. It sends
  `thinking:{type:"adaptive"}` plus `output_config:{effort:<level>}` and the `effort-2025-11-24`
  beta. After `set_model sonnet` + `effortLevel high`, requests went to `claude-sonnet-5` with
  `effort:"high"` and `max_tokens` 64000.
- These **persist in the session**: the forked, resumed process kept sonnet/high.
- Per-turn change: send `set_model` and `apply_flag_settings{effortLevel}` before the user
  message. `system/init.per_turn_effort_active:true` indicates per-turn effort support.

### 3.7 Sessions

- Transcripts live at `$CLAUDE_CONFIG_DIR/projects/<cwd with "/"→"-">/<sessionId>.jsonl`.
- Entry types seen: `user`, `assistant`, `attachment`, `queue-operation`, `last-prompt`,
  `custom-title`, `agent-name`, `cost-state`, `atis-latch`.
- **List, get messages, rename, tag, delete and fork are SDK-side file operations** (`listSessions`,
  `getSessionMessages`, `forkSession` in `sdk.mjs`), not control requests.
- Wire equivalents: `--resume <id>`; `--fork-session`; `--resume-session-at <message uuid>`
  (branch from a point); `rename_session` (live).
- Recommendation: keep the app's own thread index in the repository (id, title, backend, cwd,
  timestamps, archived flag). Parse transcripts tolerantly, only for history hydration, and treat
  unknown entry types as opaque. Claude has no archive concept, so archive is app-level.

### 3.8 Interrupt (observed)

```
→ interrupt
← control_response {still_queued:[]}
← assistant {partial text block}
← user "[Request interrupted by user]"
← result {subtype:"error_during_execution", is_error:true, terminal_reason:"aborted_streaming"}
← command_lifecycle cancelled
```

Map this to Turn `interrupted`, not `failed`: `terminal_reason` distinguishes them. Also:

- `cancel_queued:true` drops queued follow-ups.
- `cancel_async_message{message_uuid}` removes one queued message; this is the "queue / steer"
  control.

### 3.9 Auth

**Unauthenticated shape:**

- `assistant{error:"authentication_failed", is_api_error_message:true, content:[text "Not logged in · Please run /login"]}`
- then `result{subtype:"success", is_error:true, terminal_reason:"api_error"}`
- `initialize.account.tokenSource:"none"` detects this state before any turn.

**Login options for the app:**

1. `claude auth login` (`--claudeai` / `--console` / `--sso`) run in a Workflow **terminal tab**.
   The user opens the printed URL with the xterm link handler and pastes the code back.
2. `claude setup-token`: a long-lived token passed as `CLAUDE_CODE_OAUTH_TOKEN`, stored by the app.
3. `ANTHROPIC_API_KEY`.

Credentials land in `$CLAUDE_CONFIG_DIR/.credentials.json` (mode 0600, app-private). Never log
them.

## 4. Android feasibility: Claude Code on the tablet (Android 9, arm64, kernel 4.14, app UID)

Static inspection of the official 2.1.283 artifacts (`readelf`/`file`/`strings`; not executed on
the device):

| Artifact | Type | Interpreter / NEEDED | Notes |
|---|---|---|---|
| `linux-arm64` | ELF aarch64, dynamic, Bun 1.4.3 standalone (`.bun` section) | `/lib/ld-linux-aarch64.so.1`; libc, libm, libpthread, libdl, librt; max symbol GLIBC_2.26 | 240.9 MB |
| `linux-arm64-musl` | ELF aarch64, dynamic, Bun | `/lib/ld-musl-aarch64.so.1`; `libc.musl-aarch64.so.1` only | 233.3 MB. Docs also list libgcc/libstdc++ and `USE_BUILTIN_RIPGREP=0` + system `rg` for musl. |
| Android build | **not published** (npm 404; the release manifest lists 8 desktop platforms) | — | Code has `linux-${arch}-android` platform naming, but `isAndroidEnvironment(){return false}` in this build |

Host check: `ld-linux-x86-64.so.2 ./claude --version` works for the x64 build. So a Bun
single-file binary starts correctly under **explicit loader invocation**, even though
`/proc/self/exe` then points at the loader. This is the only technique that avoids a guest
rootfs with targetSdk 37 W^X, because executables must live in `nativeLibraryDir`.

Options, ranked:

1. **Inside the Debian trixie guest (recommended).** Install the glibc `linux-arm64` build, or
   `npm i -g @anthropic-ai/claude-code` with the guest's Node 24 per `research-image.md`. The
   guest provides bash, rg, git and `/tmp`, and shares `.workspace` through the bind map. Blocked
   on the native engine (G1) or the opt-in proot backend. Android 9's seccomp traps for glibc
   2.41's `clone3` / `rseq` / `faccessat2` / `close_range` / `statx` must be converted to `ENOSYS`
   by the engine (see `research-engine.md` §seccomp).
2. **Container-less (experimental; needs a device probe first).** Ship:
   - musl `libc.so` as `libld-musl.so`;
   - claude as `libclaude.so` (+233 MB APK);
   - a bionic- or musl-built `bash` and `rg` in `jniLibs`.

   Exec `libld-musl.so libclaude.so -p …` with `SHELL` / `CLAUDE_CODE_SHELL` pointing at that
   bash, plus `USE_BUILTIN_RIPGREP=0`, `TMPDIR`, `CLAUDE_CODE_TMPDIR`, `CLAUDE_CONFIG_DIR`, and
   `HTTPS_PROXY` + `NODE_EXTRA_CA_CERTS` / `SSL_CERT_FILE` from `CodexNetworkBridge`, because musl
   has no Android DNS. Risks:
   - **Android 9 app seccomp sends SIGSYS instead of ENOSYS** for syscalls newer than the API-28
     allowlist, so Bun/JSC's "graceful degradation" (it expects `ENOSYS`) may kill the process.
   - JIT `execmem` under `untrusted_app`.
   - Kernel 4.14 (Bun recommends ≥ 5.6, supports 3.10+ with fallbacks).
   - Size.
3. **Remote/other host.** Out of scope: violates "no separate workspace".

Proposed device probe (for the runtime/engine owner; do not run casually on the daily device):

1. Package the musl loader + claude as test-APK `jniLibs`.
2. In app UID, run `--version`, then `-p` with `--input-format stream-json` and an `initialize`
   control request only (no network, no auth).
3. Record the exit signal and `Seccomp:` state, and any `SIGSYS` syscall number from `logcat`
   (bionic prints the trapped syscall).
4. Only then try a mock-API turn over the loopback bridge, reusing `claude_mock_api.py`.

**Codex on Android** is already viable (static musl; handshake and account read validated
on-device per `docs/physical-validation.md`). A real login and model turn on the device are still
unverified.

## 5. Backend-neutral conversation model (for `core/`, no Android deps)

```kotlin
// Identity: app-owned ids + backend ids; raw payloads always retained for unknown/forward-compat rendering.
enum class Backend { CODEX, CLAUDE }
data class AgentThread(
  val id: ThreadId, val backend: Backend, val backendThreadId: String?,   // Codex thread.id | Claude session_id
  val title: String?, val cwd: String, val createdAt: Instant, val updatedAt: Instant,
  val archived: Boolean, val forkedFrom: ForkRef?,                       // thread + turn/message anchor
  val settings: TurnSettings,                                            // last used model/effort/mode
  val runState: RunState)                                               // idle | running | waitingApproval | waitingInput | error | notLoaded
data class TurnSettings(val model: String?, val effort: String?, val permission: PermissionProfile, val summary: String?)
data class Turn(val id: TurnId, val backendTurnId: String?, val clientMessageId: String,   // Claude user uuid / Codex clientUserMessageId
  val status: TurnStatus /* pending|queued|running|completed|interrupted|failed */, val error: AgentError?,
  val items: List<ItemId>, val usage: Usage?, val startedAt: Instant?, val completedAt: Instant?)

sealed interface Item { val id: ItemId; val turnId: TurnId; val status: ItemStatus /* streaming|completed|incomplete|failed|declined */; val raw: JsonElement? }
  UserMessage(parts: List<UserPart>)          // Text, Image(path|uri|base64), File(path), Mention(path), Skill(name)
  AgentMessage(markdown: StringBuilder, phase: Phase /* commentary|final|unknown */)
  Reasoning(summary: List<String>, text: String?, hidden: Boolean)
  Plan(steps: List<Step(text, status)>, explanation: String?, markdown: String?)   // todo/plan
  CommandExec(command, cwd, output: Appendable, exitCode?, durationMs?, processId?, source)
  FileChange(changes: List<FileDelta(path, kind add|delete|update|move, unifiedDiff?)>)
  ToolCall(kind: MCP|DYNAMIC|BUILTIN, server?, tool, arguments: JsonElement, result: JsonElement?, error?, progress: List<String>)
  WebSearch(query, results?)
  Image(source: path|uri, generatedBy?)
  SubAgent(parentToolId, agentName?, threadRef?, status)
  Marker(kind: COMPACTION|REVIEW_START|REVIEW_END|HOOK|MODEL_REROUTE|INTERRUPTED, detail)
  Notice(level: INFO|WARNING|ERROR, code?, message, retrying: Boolean)
  Unknown(backendType: String, raw: JsonElement)        // rendered as collapsible JSON, never dropped

sealed interface ServerRequest {                           // shown as blocking cards; NEVER auto-resolved
  val key: PendingKey /* backend + raw id JSON (int|string) verbatim */; val threadId; val turnId?; val itemId?; val raw: JsonElement
  CommandApproval(command, cwd, reason?, decisions: List<Decision>)      // Decision incl. persistent flag
  FileChangeApproval(paths/diff, reason?, grantRoot?, decisions)
  ToolApproval(tool, displayName, input, title?, description?, blockedPath?, suggestions, defaultToNo, suppressAlways)  // Claude can_use_tool generic
  PermissionGrant(requested: JsonElement, scopes: turn|session)
  UserQuestions(questions: List<Question(id|text, header, options, multiSelect, allowFreeText)>)  // Codex requestUserInput / Claude AskUserQuestion
  PlanApproval(planMarkdown)                                                 // Claude ExitPlanMode
  Elicitation(server, message, mode form|url, schema?, url?)
  ClientToolCall(tool, arguments)                                             // Codex item/tool/call (dynamic tools)
  AuthRefresh / HostCapability(method)                                       // chatgptAuthTokens/refresh, currentTime/read, attestation
  Unknown(method, params)                                                    // answer only via explicit user choice or protocol error
}
sealed interface TurnControl { Start(parts, settings); Steer(parts); Queue(parts); Dequeue(clientMessageId); Interrupt(cancelQueued: Boolean) }
sealed interface ThreadOp { New; List(page); Read(page); Resume; Fork(atTurn?); Rename; Archive; Unarchive; Delete; Compact; Review; Rollback }
data class AccountState(val backend, val loggedIn, val method, val plan, val rateLimits: List<Limit>, val upsell: JsonElement?)
data class BackendCapabilities(val models: List<ModelOption(id, label, efforts, defaultEffort, modalities)>, val features: Set<String>, val raw)
```

Reducer rules:

- Deltas append into the item keyed by `(backend, backendItemId)` (Claude:
  `message.id + content index`).
- `item/completed` (Codex) or the `assistant` block frame (Claude) replaces the accumulated text
  with the authoritative text.
- On turn completion, every still-`streaming` item becomes `incomplete`, and every pending
  request of that turn becomes `expired`. The card stays visible but disabled.
- History hydration (Codex `thread/turns/list` + `items/list`; Claude transcript) produces the
  same items.
- Drafts and composer attachments are Working Resources and are never discarded on panel or
  backend switch (AGENTS.md).

## 6. Mapping tables

### 6.1 Codex → model

| Codex | Model |
|---|---|
| `thread/start` / `resume` / `fork` result `.thread`; `thread/started` | `AgentThread` (backendThreadId = `thread.id`; title = `name` ?: `preview`; `path`, `forkedFromId`) |
| `thread/status/changed` (`idle`, `active{activeFlags:waitingOnApproval \| waitingOnUserInput}`, `systemError`, `notLoaded`) | `runState` |
| `thread/name/updated`, `thread/settings/updated`, `thread/archived` / `unarchived` / `deleted` / `closed`, `thread/tokenUsage/updated` | thread fields / `Usage` |
| `turn/started`, `turn/completed{status completed\|interrupted\|failed, error{message, codexErrorInfo}}` | `Turn` |
| item `userMessage` (`text`, `image`, `localImage`, `skill`, `mention`, `audio`) | `UserMessage` |
| item `agentMessage` + `item/agentMessage/delta` (`phase` commentary / final_answer) | `AgentMessage` |
| item `reasoning` + `summaryPartAdded` / `summaryTextDelta` / `textDelta` | `Reasoning` |
| item `plan` + `item/plan/delta`; `turn/plan/updated{plan[{step, status}], explanation}` | `Plan` |
| item `commandExecution` + `commandExecution/outputDelta`, `terminalInteraction` | `CommandExec` |
| item `fileChange{changes[{path, kind, diff}]}` + `fileChange/outputDelta`, `patchUpdated`; `turn/diff/updated` | `FileChange` (+ turn-level diff) |
| item `mcpToolCall` + `mcpToolCall/progress`; `dynamicToolCall`; `functionCallOutput` | `ToolCall` |
| item `collabAgentToolCall`, `subAgentActivity` | `SubAgent` |
| item `webSearch` | `WebSearch` |
| item `imageView`, `imageGeneration` | `Image` |
| item `contextCompaction`, `thread/compacted`, `enteredReviewMode` / `exitedReviewMode`, `hookPrompt`, `hook/started` / `completed`, `sleep`, `model/rerouted` | `Marker` |
| `error{willRetry}`, `warning`, `configWarning`, `deprecationNotice`, `guardianWarning`, `model/verification`, `model/safetyBuffering/updated`, `turn/moderationMetadata` | `Notice` |
| `item/commandExecution/requestApproval` (+ legacy `execCommandApproval`) | `CommandApproval` (decisions = `availableDecisions`) |
| `item/fileChange/requestApproval` (+ legacy `applyPatchApproval`) | `FileChangeApproval` (accept / acceptForSession / decline / cancel) |
| `item/permissions/requestApproval` | `PermissionGrant` (respond with granted subset + scope) |
| `item/tool/requestUserInput` | `UserQuestions` |
| `mcpServer/elicitation/request` | `Elicitation` (accept / decline / cancel + content) |
| `item/tool/call` | `ClientToolCall` (only if the app registers dynamic tools; otherwise error) |
| `account/chatgptAuthTokens/refresh`, `currentTime/read`, `attestation/generate` | `AuthRefresh` / `HostCapability` |
| `serverRequest/resolved` | close pending card |
| `item/autoApprovalReview/*`, `autoApprovalReview/strictReviewRequired` | `Notice` (should not occur with `approvalsReviewer:user`) |
| `turn/start{input, model, effort, summary, approvalPolicy, sandboxPolicy, approvalsReviewer:"user", clientUserMessageId}` | `TurnControl.Start` |
| `turn/steer{expectedTurnId}` / `thread/queue/*` / `turn/interrupt` | `Steer` / `Queue` / `Dequeue` / `Interrupt` |
| `thread/list`, `thread/search`, `thread/read`, `thread/turns/list`, `thread/items/list`, `thread/resume`, `thread/fork`, `thread/name/set`, `thread/archive` / `unarchive` / `delete`, `thread/compact/start`, `review/start`, `thread/revert` | `ThreadOp` |
| `model/list` (`supportedReasoningEfforts`, `inputModalities`, `serviceTiers`) | `BackendCapabilities.models` |
| `account/read`, `account/login/start` / `cancel`, `account/logout`, `account/rateLimits/read` + `updated`, `account/login/completed` | `AccountState` |
| `mcpServerStatus/list`, `mcpServer/startupStatus/updated`, `skills/list`, `config/read` | capability panels |
| any other method / notification | `Unknown` (generic RPC console; preserved verbatim) |

### 6.2 Claude Code → model

| Claude | Model |
|---|---|
| `system/init.session_id` (+ the app's own index) | `AgentThread` (backendThreadId = session id; title from `rename_session` / `custom-title`) |
| `system/session_state_changed`, `system/status`, `command_lifecycle` | `runState` / `Turn.status` (queued / running / completed / cancelled) |
| client user `uuid` → `result{user_message_uuid, subtype, terminal_reason, usage, modelUsage, total_cost_usd}` | `Turn` (`terminal_reason` `aborted_*` → interrupted; `is_error` → failed) |
| user content `text` / `image` / `document` / `@path` | `UserMessage` |
| `stream_event` `text_delta` → `assistant` text block | `AgentMessage` (phase: `commentary` if the same message later has `tool_use`, `final` for the last text before `result`) |
| `thinking_delta` / `thinking` block; `redacted_thinking`; `system/thinking_tokens` | `Reasoning` (hidden when redacted or `thinking_display:"omitted"`) |
| `tool_use Bash` (+ `tool_result`, `tool_use_result{stdout, stderr, interrupted}`, `tool_progress`, `task_*` for background) | `CommandExec` |
| `tool_use Write` / `Edit` / `MultiEdit` / `NotebookEdit` (+ `tool_use_result` structured patch) | `FileChange` (diff from `old_string` / `new_string` or `structuredPatch`) |
| `tool_use TodoWrite` / task-list tools, `EnterPlanMode` | `Plan` |
| `tool_use Task` / `Agent`; messages with `parent_tool_use_id`; `task_started` / `progress` / `notification` | `SubAgent` |
| `tool_use mcp__<server>__<tool>` | `ToolCall(MCP)` |
| `tool_use WebSearch` / `WebFetch` | `WebSearch` / `ToolCall` |
| `tool_use Read` / `Glob` / `Grep` / `Skill` / other | `ToolCall(BUILTIN)` (compact row) |
| image blocks in `tool_result` | `Image` |
| `system/compact_boundary`, `system/hook_*`, `set_model` local-command output, `[Request interrupted by user]` | `Marker` |
| `system/api_retry`, `system/permission_denied`, `assistant.error` (`authentication_failed`, …), `result.errors` | `Notice` |
| `control_request can_use_tool` (generic tool) | `ToolApproval` (allow + `updatedInput` / deny + message; persistence only via explicit `updatedPermissions`) |
| `can_use_tool` with `tool_name == "AskUserQuestion"` | `UserQuestions` (answers in `updatedInput.answers`) |
| `can_use_tool` with `tool_name == "ExitPlanMode"` | `PlanApproval` |
| `control_request elicitation` | `Elicitation` (`{action, content}`) |
| `control_request request_user_dialog` | Declare no `supportedDialogKinds` initially (the CLI then fails closed); later `Unknown` |
| `control_request hook_callback` / `mcp_message` | Not used (register no SDK hooks or SDK MCP servers). If received: error response. |
| `control_cancel_request` | close / expire pending card |
| `rate_limit_event`, `get_usage` | `AccountState.rateLimits` |
| user message / `priority` / `shouldQuery` | `Start` / `Queue` / `Steer` (Claude steers by queueing; there is no mid-turn inject) |
| `interrupt{cancel_queued}`, `cancel_async_message` | `Interrupt` / `Dequeue` |
| `set_model`, `apply_flag_settings{effortLevel}`, `set_permission_mode`, `set_max_thinking_tokens` | `TurnSettings` |
| spawn `--resume`, `--fork-session`, `--resume-session-at`; `rename_session`; transcript files | `ThreadOp` (list / archive / delete are app-level) |
| `initialize` response (`models`, `commands`, `agents`, `account`, `pending_*`), `list_models`, `mcp_status`, `get_context_usage` | `BackendCapabilities` / `AccountState` / context meter |
| any unknown `type` / `subtype` / control subtype | `Unknown` (control: error response, never allow) |

## 7. What "complete functionality" should mean (checklist)

- **P0**: required for "usable and correct".
- **P1**: required for "complete".
- **P2**: advanced / parity.

Each item is accepted by a recorded transcript replay test, plus device UI evidence where marked
(UI).

| # | Feature | Codex | Claude | P |
|---|---|---|---|---|
| 1 | Backend picker per thread; start / attach process; health + restart; unknown frames preserved | ✓ | ✓ | P0 |
| 2 | Login state, login flows (Codex device code / browser / API key; Claude `auth login` in terminal / setup-token / API key), logout (UI) | ✓ | ✓ | P0 |
| 3 | Streaming Markdown with KaTeX (inline `$`, display `$$`, code fences), incremental and flicker-free while streaming; commentary vs final layout (UI) | ✓ | ✓ | P0 |
| 4 | Model + effort slider fed by `model/list` / `list_models` (`supportedEffortLevels`, defaults, hidden or reserve models); applied per turn | ✓ | ✓ | P0 |
| 5 | Attachments: camera / gallery / file picker / drag-drop from the file tree → workspace file → image input (`localImage` path / base64 image block), PDF (Claude `document`), other files as path mention; modality gating by `inputModalities` | ✓ | ✓ | P0 |
| 6 | Approvals: command / file / tool / permission cards with exact decision set; persistent-grant options visually distinct; never auto-approve; `approvalsReviewer:user`; answer by preserved id; expire on turn end / cancel | ✓ | ✓ | P0 |
| 7 | Interrupt (stop button) with correct incomplete-item finalization | ✓ | ✓ | P0 |
| 8 | Reasoning summary (collapsible "Thought for Ns"), command exec with live output, file change with diff, tool/MCP call rows, web search rows, error / retry banners | ✓ | ✓ | P0 |
| 9 | Thread list (app index), resume, rename, archive, delete; history hydration after app restart | ✓ | ✓ | P0 |
| 10 | Usage / rate limits / upsell surface; token or context meter | ✓ | ✓ | P1 |
| 11 | User-input questions (`requestUserInput` / `AskUserQuestion`, multi-select + free text), MCP elicitation forms / URL, plan approval (`ExitPlanMode`) | ✓ | ✓ | P1 |
| 12 | Plan / todo panel (`turn/plan/updated`, `TodoWrite`) | ✓ | ✓ | P1 |
| 13 | Steer / queue while running (`turn/steer`, `thread/queue/*`; Claude queued messages + cancel) | ✓ | ✓ | P1 |
| 14 | Fork (from turn / message), compact, turn-level aggregated diff, review mode (`review/start`) | ✓ | ✓(fork) | P1 |
| 15 | Permission mode switch (Claude default / acceptEdits / plan / dontAsk; Codex approval policy + sandbox presets) | ✓ | ✓ | P1 |
| 16 | Sub-agent / collab rendering (nested by `parent_tool_use_id` / `collabAgentToolCall`) | ✓ | ✓ | P1 |
| 17 | MCP server status, skills / slash commands list, `@`-file suggestions (`fuzzyFileSearch` / `file_suggestions`) | ✓ | ✓ | P2 |
| 18 | Image generation / view items, realtime voice (`thread/realtime/*`), background terminals, `command/exec` standalone, plugins / apps / marketplace, remote control, projects / sections, memory, goals | ✓ | partial | P2 |
| 19 | Generic RPC / debug console for every schema method (transport layer; already partly present) | ✓ | ✓ | P2 |

"Complete" = all P0 + P1 green on replay tests against the recorded transcripts **and** at least
one real turn per backend on the device. Rows marked partial: Claude realtime and review have no
direct equivalent; they are hidden per capability, not faked.

## 8. Invariants (to encode in tests)

1. Never send an approval / allow / accept without an explicit user action on that card. The
   research scripts' policy-based acceptance is test tooling only.
2. Raw server request IDs (Codex int or string, Claude `request_id`) are echoed verbatim and kept
   in per-backend, per-direction maps.
3. Unknown notifications, items and message types are stored and rendered generically. Unknown
   server requests get a protocol error unless the user answers via the generic card.
4. Codex: always send `approvalsReviewer:"user"`, and use an app-owned `CODEX_HOME`. Claude:
   never pass `--dangerously-skip-permissions` / `bypassPermissions` / `--permission-prompts none`
   in normal flows, and keep an app-owned `CLAUDE_CONFIG_DIR`.
5. Ignore control responses with unknown `request_id` (Claude echo quirk).
6. Credentials only in backend-owned files or Android-protected storage; transcripts and logs
   redact tokens and emails.

## 9. Artifacts and reproduction

`artifacts/agent-research/` (ignored):

| File | What it is |
|---|---|
| `codex_probe.py` | `list` \| `run MODEL EFFORT`; uses the host `codex`, sandbox `sandbox/codex-run` |
| `codex-handshake*.jsonl`, `codex-session.jsonl`, `codex-session-usage-limit.jsonl` | Codex transcripts; envelope `{t, phase, dir, msg}` |
| `schema/<ver>/`, `schema/diff.json`, `schema_diff.py` | Generated schemas and the diff tool |
| `claude_probe.py` (`mock` \| `noauth`), `claude_mock_api.py` | Claude driver and mock API |
| `claude-session-mock.jsonl`, `claude-session-mock-fork.jsonl`, `claude-session-noauth.jsonl`, `claude-mock-api-requests.jsonl` | Claude transcripts and the API request log |
| `claude-manifest-2.1.283.json` | Release manifest |
| `bin/` | Extracted official packages: Codex x64 0.157.1 / 0.159.0-alpha.4; Claude 2.1.283 x64 / arm64 / arm64-musl; agent-sdk 0.3.283 |

Sandboxes:

- `sandbox/codex-run/hello.txt` was written by the approved Codex command.
- `sandbox/claude-run/hello.txt` was written by the approved Claude `Write`.

Side effects outside the project:

- The Codex runs used the user's logged-in `~/.codex` (auth is not copied anywhere). Two research
  threads were created there (`01a0ddb5-c21e-…` from the usage-limited run and
  `01a0ddb6-d670-…`). The script renamed both "workflow protocol research" and archived them.
  Their forks were ephemeral.
- The Claude runs used only the sandbox config dir.

## 10. Not verified / open

- Real Claude model output (no CLI login available to this agent). Protocol frames are real;
  content, and real-model behaviours such as redacted thinking and server tools, are not.
  Re-running `claude_probe.py` with a logged-in `CLAUDE_CONFIG_DIR` and without the mock gives
  a real capture.
- Codex `item/fileChange/requestApproval`, `requestUserInput`, elicitation, `turn/steer` and
  command output deltas were not triggered live (schema only).
- Claude on Android: static analysis plus a host explicit-loader test only. The seccomp / JIT /
  kernel behaviour needs the probe in §4.
- Codex on the device: real login and model turn still pending (per existing docs).
