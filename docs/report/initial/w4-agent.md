# W4 report: `:agent` module (conversation model, Codex + Claude Code adapters)

Date: 2026-09-27. Status: **done** for the model, adapters, transports and replay tests. The UI and the on-device `ProcessLauncher` belong to later workstreams. The long-term doc is [docs/agents.md](../../agents.md).

## 1. Delivered

| Area | Files (`agent/src/main/kotlin/top/flysoftbeta/workflow/agent/`) |
|---|---|
| API | `AgentBackend.kt`: `AgentBackend`, `AgentStateStore`, `AgentEventSink`, `ThreadOptions`, `SendMode`, `ThreadSummary`, `UnknownRequestPolicy` |
| Neutral model | `model/`: `State`, `Items`, `Requests`, `Events`, `AgentReducer` (pure), `Catalog` (slider), `Account`, `ConversationIndex`, `StreamText`, `Inputs` |
| Process port | `process/ProcessLauncher.kt`: `LaunchSpec`, `AgentProcess`, `JvmProcessLauncher` (host) |
| Transport | `transport/`: `JsonLines` (size-capped UTF-8 framing, bounded channel, never drops), `JsonRpcConnection` (per-direction pending maps, raw ids), `ControlConnection` (echo-safe) |
| Codex | `codex/`: generated `CodexProtocol`, `CodexItems`, `CodexEvents`, `CodexParams`, `CodexBackend` |
| Claude Code | `claude/`: `ClaudeLaunch`, `ClaudeMapper`, `ClaudeRequests`, `ClaudeTranscript`, `ClaudeBackend` |
| Coverage | `ProtocolCoverage.kt` → golden `agent/protocol-coverage.md` |

**Protocol assets.**

- `runtime/schemas` moved to `agent/src/main/resources/protocol/codex/`. The TypeScript bindings are not packaged; they are regenerated into the ignored `artifacts/agent/`.
- `tools/update-protocol.py` was rewritten for the new location and also generates `CodexProtocol.kt`. It has an offline `--kotlin-only` mode.
- The new `tools/update-claude-protocol.py` extracts `protocol/claude/inventory.json` from the Agent SDK typings.

**Codex bump.**

- `third_party/codex/manifest.json` now pins **0.157.1** from the official release. The archive sha256 values match GitHub's published digests. The arm64-v8a entry was replaced and an **x86_64** entry was added.
- `app/build.gradle.kts` `fetchPrebuilts.abis` now lists `arm64-v8a, x86_64`. This is a one-line change and also packages the already pinned Mihomo x86_64.
- `:app:fetchPrebuilts` extracted and verified both ABIs.
- Schemas regenerated from the real 0.157.1 binary are byte-identical to 0.157.0 except for the version string.

**Other build changes.**

- `agent/build.gradle.kts` dropped the unused `api(project(":core"))`. `:core` was failing to compile because another workstream was mid-edit, and nothing in `:agent` uses it.
- `agent/build.gradle.kts` adds `coroutines-test`, the golden-update property and the `:agent:smoke` JavaExec task.

**`:app` is untouched.** Nothing in `:app` uses `:agent` yet, and the old Codex path is still wired into `RuntimeUi`, which the UI workstream will switch over.

## 2. Verification

**`flock artifacts/.gradle.lock ./gradlew :agent:test`** runs 46 tests, all green in 4 consecutive clean runs, with no warnings. `verifyPlatformIndependence` passes.

| Test | What it covers |
|---|---|
| `CodexBackendReplayTest` | Real `CodexBackend` against the recorded 0.157.0 session: markdown/math, approval (id 0 answered only after `respond`; an unoffered `decline` is rejected), interrupt with dangling message → `INCOMPLETE`, history, list, rename, fork, archive. Separately, the unmodified recording (`auto_review` echoed) **blocks sending**. |
| `ClaudeBackendReplayTest` | Real `ClaudeBackend` against the 2.1.283 frames: image + math, app hook callback, `can_use_tool` allow and deny (→ `DECLINED`), set_model/effort/mode/rename, interrupt, fork with a client-chosen id, not-logged-in. |
| Unit tests | Reducer invariants, transports (overlapping ids, string ids, strays, EOF, echo, cancel), decision mapping, params, slider, Claude requests/launch/transcript hydration, coverage/golden. |

**Host smoke runs.** Logs are in `artifacts/agent/smoke/`.

- **Claude:** the real CLI 2.1.283 against the research mock API (no model, dummy key). All checks pass. This confirms that `--resume X --fork-session --session-id <ours>` is accepted, that permission prompts stay pending until answered, and that transcript hydration works.
- **Codex:** the real `codex app-server` 0.157.0 with the user's login. It used an ephemeral thread and 2 turns on `gpt-reserve`. The server echoed `approvalsReviewer:"user"` despite config `auto_review`. The math turn completed. The approval stayed pending until the smoke answered `cancel`, the command was `DECLINED`, and no file was written.

**Bugs found by replay and fixed:**

- the optimistic user message lost its composed attachments;
- a user answer racing `serverRequest/resolved` or the end of the turn showed as RESOLVED/EXPIRED instead of ANSWERED;
- `resolve()` kept an effort chosen for a different model.

## 3. Open items

- **Device:** needs the engine's `ProcessLauncher`. No on-device run yet.
- **Not live-captured, unit-tested only:** `requestUserInput`, elicitation, steer/queue, sub-agents, command output deltas.
- **Console-only today:**
  - Codex `thread/queue/changed` refresh; review mode and revert.
  - Claude context meter; @-file suggestions.
- **Stale doc:** `docs/runtime.md:113` still names `runtime/schemas`. It is a legacy doc owned elsewhere.
- **APK size (optional):** switching to the standalone `codex-app-server` asset would shrink the APK.
