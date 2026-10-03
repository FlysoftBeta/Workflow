# Runtime contract and verification

> Historical HTTP compatibility backend. Workflow 0.2 defaults to its own app-UID runtime, shared workspace, PTY, Codex and Mihomo management; no Termux or pairing is required. See [Android local runtime](android-local-runtime.md) and [local proxy](android-local-proxy.md). The following contract remains for an explicitly selected external compatibility service.

This document describes the executable companion, not a claim that every requested
Android interface or the custom container engine is finished. All runtime state is
rooted at one `WORKFLOW_WORKSPACE`, defaulting to Termux `$HOME/.workspace`.

## What is implemented

- A Python standard-library daemon bound only to `127.0.0.1:8765`. Every route,
  including health, requires a random 256-bit pairing token. Browser Origin
  requests are rejected. No external HTTP server or pip dependency is needed.
- Real controlling PTYs, resize, byte-preserving input/output and process exit
  events. These are **Termux host terminals** unless `guest: true` is selected.
- Workspace-relative file listing, binary reads/uploads, atomic writes, optional
  SHA-256 optimistic concurrency and explicit rejection of symlink traversal.
- Bidirectional Codex app-server JSONL transport. It forwards every method,
  response, error and notification without reducing the protocol to a small list.
- Mihomo REST control for rule/global/direct, proxy groups and group selection.
  The root kernel has its own explicit launcher and configuration; the daemon does
  not silently invoke root or rewrite the user's YAML.
- A Debian private image builder, checksummed installer, metadata and virtual
  UID/GID/mode storage using `user.workflow.*` xattrs, with an attribute index
  fallback. Explicit `proot` compatibility launch and declarative rebuild support.

## Termux installation

Copy `runtime/artifacts/workflow-runtime.tar.gz` to the device Downloads directory.
In Termux, grant shared-storage access once with `termux-setup-storage`, then run:

```bash
mkdir -p "$HOME/workflow-runtime-install"
tar -xzf "$HOME/storage/downloads/workflow-runtime.tar.gz" -C "$HOME/workflow-runtime-install"
bash "$HOME/workflow-runtime-install/install-termux.sh"
bash "$HOME/.workspace/runtime/start-daemon.sh"
```

The installer installs only Python and zstd. It prints the local pairing token.
Copy the token into Workflow's activation page. To show it again:

```bash
bash "$HOME/.workspace/runtime/start-daemon.sh" --pair
```

Disable battery optimization for Termux and keep its session active. A Termux:Boot
service is a deployment option, not installed by this script. Set
`WORKFLOW_WORKSPACE=/your/private/path/.workspace` **before installation** to use a
different root. The generated launchers preserve that root. Keep executable images
in Termux private storage; Android shared storage does not provide suitable Unix
execution/attribute semantics.

For local development:

```bash
PYTHONPATH=runtime python3 -m workflow_daemon --workspace /tmp/workflow-test/.workspace
PYTHONPATH=runtime python3 -m workflow_daemon --workspace /tmp/workflow-test/.workspace --pair
```

Android's private `filesDir/.workspace` is a distinct sandbox from Termux's root.
The current UI repository's session/config data must be synchronized deliberately;
this implementation does not pretend the two paths are the same. The daemon is
the authority for terminals, edited runtime files, Codex and container state. Use
its file API for attachment uploads and edits.

## HTTP and events

All responses are JSON. Pair using `Authorization: Bearer <token>`.

| Route | Contract |
| --- | --- |
| `GET /health` | `ready`, daemon `instance`, installed/running Codex, workspace path; native engine always explicitly false |
| `GET /capabilities` | Generated complete method inventory and actual runtime capabilities |
| `GET /events?after=0&timeout=5` | `events`, `cursor`, `latest`, `instance`; use returned cursor for next poll |
| `GET /files?path=...` | Relative-path directory entries |
| `GET /file?path=...` | `base64` bytes and `sha256` |
| `PUT /file` | `{path,base64,expectedSha256?}`; stale hash gives 409 |
| `POST /directory` | `{path}` |
| `GET /terminals` | Existing daemon-owned terminals |
| `POST /terminals` | `{rows,columns,cwd,guest?}`; returns terminal `id` |
| `POST /terminals/{id}/input` | `{base64}`; control characters are literal bytes |
| `POST /terminals/{id}/resize` | `{rows,columns}` |
| `DELETE /terminals/{id}` | Close the terminal |
| `POST /codex/start` | Launch or reuse process; `existing` reports process reuse, `initialized` confirms a successful initialize response |
| `POST /codex/message` | Any raw JSON-RPC request, notification, result or error |
| `DELETE /codex` | Stop app-server |
| `GET /container` | Installed/configured/applying/state plus truthful engine availability |
| `POST /container/reconcile` | Validate and asynchronously apply supported container config |
| `GET /proxy/status` | Controller connection and version/config |
| `GET /proxy/proxies` | Native Mihomo group/node response |
| `POST /proxy/mode` | `{mode: "rule" \| "global" \| "direct"}` |
| `PUT /proxy/proxies/{encodedGroup}` | `{name}` |

Events have `{sequence,time,source,payload}`. `source: "codex"` retains the original
protocol object. Terminal events are `{id,type:"output",base64}` or
`{id,type:"exit",exitCode}`. The ring retains at most 8,192 events / approximately
32 MiB, whichever is reached first. A missing history window or a cursor from a
restarted daemon gives 410, `resetRequired` and `resetCursor` (the oldest retained
event minus one). The client reports the gap and can reconnect using that cursor.
Reloading full thread history is still required to fill a missing window; the UI
does not yet hydrate that history automatically. PTY history itself is not durable across daemon restart. Events are not
silently dropped and presented as a complete transcript.

The Kotlin adapter is `WorkflowRuntimeClient`. It uses native HTTP, background
coroutines, timeouts and `Result<JSONObject>`, without logging tokens or messages.
Pairing authorizes shell/process control; the token is not a read-only file token.

## Codex protocol coverage

The checked-in schema comes from **codex-cli 0.157.0**, generated with
`--experimental`. It contains **167 client requests, 11 server requests, 83 server
notifications and 1 client notification**. `tools/update-protocol.py` regenerates
JSON Schema, TypeScript and `runtime/schemas/inventory.json` together. The HTTP
inventory is marked as a reference snapshot and reports the installed version
after Codex starts; `schemaMatchesInstalled` explicitly indicates version drift.

The implementation was checked against these official sources:

- [Codex App Server documentation](https://learn.chatgpt.com/docs/app-server)
- [Versioned protocol definitions](https://github.com/openai/codex/blob/rust-v0.157.0/codex-rs/app-server-protocol/src/protocol/common.rs)
- [Versioned message processor](https://github.com/openai/codex/blob/rust-v0.157.0/codex-rs/app-server/src/message_processor.rs)

Source snapshots and the Apache license are in `runtime/reference`. The protocol
uses JSON-RPC-shaped objects without requiring a `jsonrpc` wire field. Initialize
once, await the response, then send `initialized` before ordinary requests.
Experimental fields require `capabilities.experimentalApi: true`.

Approvals and client-side requests remain client-owned. This includes command/file
approvals, permissions, user questions, MCP elicitation, dynamic tools, auth token
refresh, attestation, clock and legacy approval requests. The bridge never grants
a request automatically. Unknown methods and payload fields survive unchanged.

A generic request/response surface is necessary for forward compatibility, but
**transport completeness is not the same as polished UI coverage of every
feature**. Specialized authentication, review, plugin, skill, permission, realtime
and all other interfaces must still use the schema and track their own UI coverage.

`tools/smoke-codex.py` starts an isolated real app-server, performs initialize,
initialized and model/list, then stops it. It never starts a model turn or reads
the host's Codex login state. Recorded result: handshake passed, model/list
returned 7 models. The fixture test also sends unknown future methods and verifies
that a server request and the client's decline response are forwarded unchanged.

The integration review added HTTP regressions for cursor recovery and process
reuse before/after initialization; all 12 Python tests passed on 2026-09-26. A
fresh real app-server handshake and model/list also passed without invoking a
model. `RuntimeConversationTest` adds Android-side protocol replay for background
conversation routing across Session changes and unknown server requests with
numeric IDs; its device execution is tracked in the implementation record.
Specialized command/file approval buttons use the schema's `accept` / `decline`
decision object and user-input responses preserve the question-ID-to-answer map.
Other server request families remain an explicit raw JSON response surface.
Terminal Session associations are currently in memory; reconnecting to an existing
PTY after Android process death and full missed-history recovery remain unverified.

Codex is optional during activation. An Android-compatible executable may be set
with `WORKFLOW_CODEX=/absolute/executable`. The official Linux CLI was verified on
the build host; native Termux compatibility is not asserted. With an installed
Debian image and explicit `engine: "proot"`, install Codex inside that guest and
start the daemon with `WORKFLOW_CODEX="$HOME/.workspace/runtime/codex-guest.sh"`.
That adapter maps Codex state to `/workspace/codex` and preserves stdio. Android
guest Codex execution still requires device validation.

## Image format and engines

`metadata.json` plus a `rootfs/` tree form one tar stream compressed as
`image.tar.zst`. Metadata uses `schemaVersion:1`, `type:"debian-trixie"`, target
architecture, user `work` (1000:1000) and runtime requirements. It does not claim OCI
runtime compatibility. OCI is only the build input.

```bash
# Default target is arm64; an arm64 builder or working QEMU/binfmt is required.
tools/build-image.sh arm64
# A native amd64 build is included and verified on this workstation.
tools/build-image.sh amd64
```

The included amd64 image is about 259 MiB. The builder installs the requested
Debian packages, nvm 0.40.8, uv 0.12.19, Node 24 and Python 3.13. During verification
these resolved to Node 24.21.0 and Python 3.13.15. `work` can run passwordless sudo
inside the built OCI image. The flattened installer preserves leading-dot and
Unicode filenames, virtual attributes and guest symlinks; it rejects path escapes,
duplicate entries and unsafe hardlinks, stages installation, and refuses to replace
an existing rootfs silently.

Install only an image for the device architecture and verify its trusted digest:

```bash
PYTHONPATH="$HOME/.workspace/runtime/daemon" python -m workflow_daemon.image \
  /path/image.tar.zst --workspace "$HOME/.workspace" --sha256 EXPECTED_SHA256
```

`container.json` matches the Android core schema: `schemaVersion`, `imageType`,
`image`, `user`, `pythonVersion`, `nodeVersion`, `engine`, `environment`, `packages`,
and `bindMounts` (`source`, `target`, `readOnly`). Changes are detected every two
seconds. In explicit proot mode, an installed guest applies packages and uv/nvm
versions, with status events and `.workspace/container/build.log`. Invalid configs
and unsupported read-only binds fail visibly; proot cannot enforce read-only mounts.

The **custom Android PackageManager loader and ptrace engine are not implemented**.
Default `engine:"native"` therefore refuses launch; it cannot fall through to a
host shell. The image installer storing xattrs does not implement syscall metadata
virtualization. Pending work includes the packaged ELF/linker trampoline, guest
exec/mmap behavior, path and fd translation, per-task ptrace state, fork/clone,
execve, signal delivery, Android/new-old-kernel syscall tables, stat/ownership/mode
emulation, binfmt, bind semantics and guest lifecycle cleanup. The proot option is
explicit compatibility work, not represented as the requested custom engine.

## Mihomo delivery

Latest release checked through the official GitHub API on 2026-09-26:
[Mihomo v1.19.31](https://github.com/MetaCubeX/mihomo/releases/tag/v1.19.31).
The actual Android arm64-v8 and amd64 archives are preloaded in
`runtime/vendor/mihomo`, with SHA-256, provenance, GPL-3.0 license and the matching
source archive. They are not URLs masquerading as downloaded resources.

`tools/package-proxy.sh` builds `runtime/artifacts/workflow-proxy.tar.gz`. Extract
it in Termux and run `bash runtime/proxy/install-proxy-termux.sh`. It copies the
kernel bundle to the same workspace, generates a local controller secret only for
a new config, and leaves existing config untouched. Edit
`.workspace/.workflow/proxy/config.yaml` with your own providers and groups; the
initial example routes directly. Then explicitly start root mode:

```bash
bash "$HOME/.workspace/runtime/start-proxy.sh" --root
```

The launcher verifies the bundled archive digest, installs the correct executable,
quotes paths for the actual su shell, and runs in the foreground. Ctrl-C stops it.
REST reads `.workspace/.workflow/proxy/controller.json`; its endpoint and secret
must match the user-managed YAML. No config-override feature is introduced.

The owned Android 9 / API 28 x86_64 emulator passed a real root-kernel test:
`-v`, config validation, TUN creation with auto-route, authenticated REST,
rule/global/direct mode changes, GLOBAL group selection, unauthenticated 401,
and SIGTERM cleanup. The interface disappeared and routes matched their pre-test
state. Device files and port forwarding were removed. Evidence is in
`runtime/reference/mihomo-emulator-result.json`; root was granted through **adb
root**, so this does not verify a particular phone's `su` manager. The arm64
executable and physical rooted-device TUN lifecycle still require hardware
validation. Host routes were never changed.

## Verification commands

```bash
PYTHONPATH=runtime python3 -m unittest discover -s runtime/tests -v
python3 tools/smoke-codex.py
bash -n tools/*.sh runtime/install-termux.sh runtime/proxy/install-proxy-termux.sh
tools/package-runtime.sh
tools/package-proxy.sh
```

Tests exercise authenticated HTTP, origin rejection, symlink/path confinement,
concurrent-edit protection, PTY controlling-terminal and resize behavior, event
loss detection, unknown bidirectional RPC frames, truthful native-engine failure,
image traversal rejection and Unicode PAX repacking. Full-image install and OCI
work/sudo/language checks are recorded separately from these unit tests.
