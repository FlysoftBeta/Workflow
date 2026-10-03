# 代理后端（`:agent`）

对话界面同时接入 Codex App-Server 与 Claude Code。`:agent` 是纯 JVM 模块，提供与后端无关的对话模型、两个适配器、协议传输和协议覆盖清单；界面只渲染这里的状态。架构位置见 [architecture.md](architecture.md) §1、§4，用户可见行为见 [product.md](product.md) §8。

## 1. 结构

```
agent/src/main/kotlin/top/flysoftbeta/workflow/agent/
  AgentBackend.kt        后端接口、AgentStateStore（事件 → 状态）、ThreadOptions、SendMode
  ProtocolCoverage.kt    覆盖分类（生成 agent/protocol-coverage.md）
  model/                 中立模型：State、Items、Requests、Events、AgentReducer（纯函数）、
                         Catalog（模型/推理强度滑块）、Account、ConversationIndex、StreamText
  process/               ProcessLauncher 端口（宿主测试实现仅用于测试）
  transport/             JSONL 分帧、JSON-RPC（Codex）、stream-json 控制协议（Claude）
  codex/                 CodexProtocol（生成）、CodexItems、CodexEvents、CodexParams、CodexBackend
  claude/                ClaudeLaunch、ClaudeMapper、ClaudeRequests、ClaudeTranscript、ClaudeBackend
agent/src/main/resources/protocol/
  codex/                 Codex JSON Schema + inventory.json（当前 codex-cli 0.157.1）
  claude/inventory.json  Claude Agent SDK 0.3.283 的控制子类型与消息种类
```

## 2. 后端抽象

- **数据流**：适配器把线上帧映射为 `AgentEvent`，交给 `AgentEventSink`；`AgentStateStore` 同步地用 `AgentReducer.reduce` 折叠成不可变的 `AgentState`（`StateFlow`）。进程级 `AgentHub`（`:app`）持有一个 store，界面只读状态、只通过 `AgentBackend` 发命令。对话菜单提供压缩、用量/MCP 状态、高级控制台和确认删除；删除同步移除所有 Session 引用和该对话草稿，不影响其他资源。
- **状态**：`AgentState = backends（进程、账户、限额、模型、MCP、通知、未知帧）+ threads（ThreadKey → ThreadState）+ requests（RequestKey → PendingRequest）`。
- **Thread / Turn / Item**：
  - Codex：thread = `thread.id`，turn = `turn.id`，item = `item.id`。
  - Claude：thread = `session_id`，turn = 客户端生成的 prompt uuid（同时是 `command_uuid` 与 `result.user_message_uuid`），文本/思考 item = `<message.id>:<块序号>`，工具 item = `tool_use.id`。
  - Item 类型：用户消息（附件）、助手消息（流式、`COMMENTARY`/`FINAL` 阶段）、推理（摘要/正文/隐藏）、计划/待办、命令（流式输出、退出码）、文件修改（diff）、工具/MCP 调用、子代理、网页搜索、图片、标记（压缩、中断、hook、本地命令……）、通知、未知（原样 JSON）。
- **Reducer 规则**：增量按 item id 追加（`StreamText` 为 O(1) 追加）；完成帧以权威内容替换，但保留载荷缺失的已流式内容；turn 结束时所有进行中的 item 变为 `INCOMPLETE`，该 turn 的待决请求变为 `EXPIRED`（卡片保留但不可操作）；未知阶段在 turn 结束时解析为“最后一条 = FINAL”；进程退出使该后端运行中的 turn 失败并使请求过期；reducer 从不回答请求。
- **乐观发送**：`send` 先产生 `TurnSubmitted`（本地 QUEUED turn），随后以 `TurnBound`/回显绑定到后端 turn，两种到达顺序结果相同；回显不覆盖用户组合的附件（工作区路径）。
- **历史**：Codex 用 `thread/turns/list`（`itemsView: full`，新→旧分页，模型内旧→新）；Claude 解析会话文件 `$CLAUDE_CONFIG_DIR/projects/<cwd>/<session>.jsonl`（`ClaudeTranscriptSource` 由应用提供读取）。历史合并时保留更完整的实时 turn。
- **对话索引**：`ConversationIndex`（接口）+ `ConversationEntry`：应用自有 id、后端、后端 thread id、标题、cwd、时间、归档、fork 来源、上次模型/推理强度、预览。`ConversationIndexFile` 只维护类型化索引，所有持久化经 Engine 的 opaque documents 接口，客户端不写工作区文件；`ConversationIndexing.refresh/search/sorted` 为纯函数。Claude 无后端列表与归档，归档/删除在索引层完成。
- **模型 + 推理强度滑块**：`ModelCatalog.slider(current)` 给出分段（模型）与档位（推理强度）及默认档；隐藏模型（如 `gpt-reserve`）只在当前正使用时出现；`resolve()` 把记忆的位置夹到当前可用值。选择在下一轮生效：Codex 写入 `turn/start` 的 `model`/`effort`；Claude 在发送前 `set_model` + `apply_flag_settings{effortLevel}`。
- **附件**（路径均为环境内 `/workspace/...`）：Codex 图片 → `localImage`，其他文件 → 文本中的路径引用；Claude 图片 → base64 `image`，PDF → base64 `document`，其他文件 → `@path`（由 CLI 读取）。`inputModalities` 用于禁用不支持的附件。

## 3. 进程与启动契约

`ProcessLauncher.launch(LaunchSpec(argv, env, cwd, label))` 返回 `AgentProcess`（stdin/stdout/stderr 字节流、`awaitExit`、`kill`）。生产环境唯一实现为 `EngineProcessLauncher`，通过 Engine process API 在定制 Debian 环境内启动；环境未就绪时等待，失败时显示错误，不能回退到 Android 宿主或演示后端。`env` 是子进程的完整环境，不得混入宿主环境。两个后端都以 stdio 承载协议，stderr 只做诊断（保留末尾 32 KiB）。

| | Codex | Claude Code |
|---|---|---|
| argv | `/opt/workflow/bin/codex app-server` | `claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --replay-user-messages --permission-prompt-tool stdio --permission-mode <mode> [--model m] [--effort e] (--session-id id \| --resume id [--fork-session --session-id new [--resume-session-at uuid]])` |
| 进程 | 一个进程服务多个 thread | 每个 session 一个进程；`start()` 预热的空闲进程成为下一个新会话 |
| 应用自有目录 | `CODEX_HOME` | `CLAUDE_CONFIG_DIR`、`TMPDIR`、`CLAUDE_CODE_TMPDIR` |
| 过滤的变量 | `OPENAI_*`、`CODEX_*`、`ANTHROPIC_*`、`CLAUDE_*`、`LD_PRELOAD`、`LD_LIBRARY_PATH` | 同左并加 `CLAUDECODE`；另设 `DISABLE_AUTOUPDATER/TELEMETRY/ERROR_REPORTING`、`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1` |
| 凭据 | Codex 自己存在 `CODEX_HOME` | `ClaudeConfig.credentials`（`CLAUDE_CODE_OAUTH_TOKEN` / `ANTHROPIC_API_KEY`）或 `$CLAUDE_CONFIG_DIR/.credentials.json` |
| 登录 | 设备码（首选）、浏览器回调、API Key | 终端标签页运行 `claude auth login` / `claude setup-token`；API Key 仅在内存中；持久登录由 guest home 中的 CLI 文件拥有 |

Codex 二进制由 `third_party/codex/manifest.json` 固定（0.157.1，arm64-v8a 与 x86_64 musl 静态版本，校验 sha256）。

## 4. 安全不变量

| 不变量 | 实现 | 测试 |
|---|---|---|
| 从不自动批准；审批只来自用户对卡片的操作 | reducer 不回答；适配器仅在 `respond()` 时写回；主机能力请求（`currentTime/read`）只回答事实，动态工具调用回答 `success:false` | `AgentReducerTest.reducerNeverAnswersRequests`、两个 `*BackendReplayTest` |
| Codex 每个 thread/turn 显式 `approvalsReviewer:"user"`；服务器回报其他值时阻止发送 | `CodexParams`（含通用控制台 `enforceReviewer`）、`CodexBackend.checkReviewer` | `CodexBackendReplayTest.inheritedAutoReviewerBlocksSending`、`CodexEventsTest.paramsAlwaysRouteApprovalsToTheUser`；宿主实测服务器回显 `user` |
| 只提供服务器允许的决定；持久授权单独标记（`ALLOW_SESSION`/`ALLOW_PERSISTENT`） | `availableDecisions` 原样映射；缺省时仅一次性接受/拒绝/取消；Claude 按 `permission_suggestions`，`suppress_always_allow_rule` 时不提供；`requires_user_interaction` 时不提供一键批准 | `CodexEventsTest`、`ClaudeUnitTest` |
| 两个方向的请求 id 分开；服务器请求 id 原样回传（int 或 string） | `JsonRpcConnection` 双 map；`RequestKey(backend, rawId)` | `TransportTest.jsonRpcKeepsDirectionsApart…` |
| 未知方法/通知/字段保留并以通用形式呈现 | 帧保持 `JsonElement`；`UnknownRecord`/`UnknownItem`；未知服务器请求默认以协议错误拒绝（可配置为通用卡片） | `TransportTest`、`AgentReducerTest.unknownEventsArePreservedBounded` |
| 中断时收尾半流式 item | turn 结束 → `INCOMPLETE` | 两个回放测试、`AgentReducerTest` |
| 忽略 Claude 回显的控制响应 | `ControlConnection` 只接受在等待的 `request_id` | `TransportTest.controlProtocolIgnoresEchoes…` |
| Claude 不使用 `bypassPermissions`/`auto`/`--dangerously-skip-permissions`；未声明的对话类型从不回答 | `ClaudeLaunch.ALLOWED_MODES`、`extraArgs` 检查；`request_user_dialog` 不回答 | `ClaudeUnitTest` |
| 宿主凭据不进入代理环境；日志与 fixture 不含令牌/邮箱 | 环境过滤；fixture 已脱敏 | `*BackendReplayTest` 启动契约断言 |

## 5. 协议资产与覆盖

- 更新 Codex：`CODEX=<codex 二进制> tools/update-protocol.py`（生成 schema、`inventory.json`、`CodexProtocol.kt`；TypeScript 绑定写入被忽略的 `artifacts/agent/`）。仅重生成常量：`--kotlin-only`。
- 更新 Claude：`tools/update-claude-protocol.py <claude-agent-sdk/package>`。
- `ProtocolCoverageTest` 校验常量与清单一致、分类不含过期项，并比较生成的 [agent/protocol-coverage.md](../agent/protocol-coverage.md)；更新：`flock artifacts/.gradle.lock ./gradlew :agent:test -Pagent.updateGolden=true`。
- 覆盖级别：`model` 映射为中立事件；`adapter` 适配器发送；`transport` 由传输/适配器消费；`card` 以卡片交给用户；`auto` 自动回答事实或否定结果；`reject` 协议错误；`never-answer` 不得回答；`generic` 原样保留，经通用控制台（`rawRequest`）可用。

## 6. 完整功能清单

P0 = 基础功能，P1 = 完整控制，P2 = 进阶。UI 列表示提供的入口；设备验收与登录边界单独见 status.md。

| # | 功能 | P | 适配/传输 | 模型 | UI |
|---|---|---|---|---|---|
| 1 | 每个对话选择后端；启动/重连进程；健康状态；未知帧保留 | P0 | ✓（EngineProcessLauncher） | ✓ `ProcessState`、`UnknownRecord` | 已实现 |
| 2 | 登录状态与流程（Codex 设备码/浏览器/API Key；Claude 终端登录/setup-token/API Key）、登出 | P0 | ✓ | ✓ `AccountState`、`LoginFlow` | 已实现 |
| 3 | 原生 Compose Markdown + JLaTeXMath；进行中/完成后两种布局 | P0 | ✓ | ✓ 增量、`MessagePhase` | 已实现 |
| 4 | 模型 + 推理强度滑块，每轮生效 | P0 | ✓ | ✓ `ModelCatalog.slider` | 已实现 |
| 5 | 附件：图片、PDF、其他文件；按模态禁用 | P0 | ✓ | ✓ `UserPart` | 已实现 |
| 6 | 审批卡片：精确决定集合、持久授权区分、按 id 回答、过期 | P0 | ✓ | ✓ | 已实现 |
| 7 | 中断（停止键）并正确收尾 | P0 | ✓ | ✓ | 已实现 |
| 8 | 推理摘要、命令实时输出、文件 diff、工具/MCP 行、网页搜索、错误/重试 | P0 | ✓ | ✓ | 已实现 |
| 9 | 对话列表（应用索引）、恢复、重命名、归档、删除、重启后历史水合 | P0 | ✓ | ✓ `ConversationIndex` 接口（持久化在 Engine documents） | 已实现 |
| 10 | 用量 / 限额 / upsell；token 与上下文 | P1 | ✓（上下文细分仅控制台 `get_context_usage`） | ✓ `RateLimitState`、`TokenUsage` | 已实现 |
| 11 | 用户提问（多选 + 自由文本）、MCP elicitation、计划审批 | P1 | ✓（未有实录，单元测试覆盖） | ✓ | 已实现 |
| 12 | 计划 / 待办 | P1 | ✓ | ✓ `TurnPlan`、`PlanItem` | 已实现 |
| 13 | 运行中引导 / 排队 / 取消排队 | P1 | ✓（未有实录） | ✓ | 已实现 |
| 14 | fork、压缩、turn 级 diff、审阅模式 | P1 | fork/压缩/diff ✓；审阅模式经控制台 | ✓ | 分叉/压缩菜单、diff、显式高级控制台 |
| 15 | 权限模式切换（4 个预设） | P1 | ✓ | ✓ `PermissionPreset` | 已实现 |
| 16 | 子代理嵌套 | P1 | ✓（未有实录） | ✓ `SubAgentItem`、`parentId` | 已实现 |
| 17 | MCP 状态、技能/斜杠命令、@ 文件建议 | P2 | MCP 启动状态 ✓；其余仅控制台 | 部分 | 状态对话框；其余高级控制台 |
| 18 | 图片生成/查看、实时语音、后台终端、独立执行、插件等 | P2 | 图片 ✓；其余 generic | 部分 | 图片渲染；其余高级控制台 |
| 19 | 通用 RPC 控制台 | P2 | ✓ `rawRequest`（保留安全约束） | ✓ | 已实现 |

“完整”要求全部 P0 + P1 通过回放测试，且每个后端至少一次设备上的真实对话。上述 UI/协议实现不替代真实账户验收。原生渲染的设备证据见 [报告](report/rewrite/native-chat.md)。

## 7. 验证

- `flock artifacts/.gradle.lock ./gradlew :agent:test`：纯函数与传输单元测试；`CodexBackendReplayTest`、`ClaudeBackendReplayTest` 用 `ScriptedServer` 把研究实录（`agent/src/test/resources/fixtures/`）作为假进程回放，驱动真实后端（数学公式、审批接受/拒绝、hook、模型切换、中断、线程操作、fork、未登录）。
- 宿主冒烟（不在 `test` 中）：`flock artifacts/.gradle.lock ./gradlew :agent:smoke -PsmokeArgs="codex <codex> <CODEX_HOME> <dir>"`（真实模型，最多 2 轮）与 `-PsmokeArgs="claude <claude> <mock 端口> <dir>"`（配合 `artifacts/agent-research/claude_mock_api.py`，无真实模型）。

## 8. 已知限制

- Codex 与 Claude 的实际启动已由容器集成测试覆盖；没有把未登录启动或协议回放当成真实模型对话验收。
- Claude 的 `refreshAccount` 通过重启空闲进程读取新凭据；API Key 登录只保存在内存。
- Codex 队列变化通过分页读取同步；取消使用后端 submission id，开始运行的 turn 不会被迟到快照退回队列。审阅模式、回滚、@ 文件建议、上下文细分经显式高级控制台。
- `requestUserInput`、elicitation、steer、子代理、命令输出增量尚无实录，只有单元测试。
