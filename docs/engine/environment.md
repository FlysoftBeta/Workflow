# Environment

An environment is the complete runtime in which terminals and coding agents work: its image, packages, language versions, variables, mounts, home, and process lifecycle. `.workspace/env.json` is its declaration, rather than the environment itself. `engine/environment` is package `workflow-environment`. It owns that declaration's application and publishes measured health; Android renders the result. The [protocol](protocol.md) defines the client boundary, and the [image format](image-format.md) defines the distributable archive.

Both Android ABIs ship a customized `workspace` image with the APK. These images contain the `work` user, shell, development tools, uv, nvm, Python, Node, and `envctl`, so using default toolchains does not require an initial network installation. Additional versions or packages can require downloads. A `base` image is only a build or runtime-test input and cannot serve as the application's environment or release fallback. The Engine tools distribution is a separate verified, architecture-specific payload. APK assets carry `assets/environment/tools/tools.json` and `tools.zip`; a standalone Server can receive the same pair through `--tools`. Codex is not packaged as an Android JNI executable.

The catalog records archive identity, architecture, each member's size, SHA-256 and executable flag, mandatory tool versions and optional Claude release pins. Extraction rejects unsafe paths, links, unexpected members, size mismatches and digest mismatches. The Engine extracts the content-addressed payload to `.workspace/agents/tools/payload/<sha256>/` and binds it at `/opt/workflow/tools`, including for existing generations, without rerunning user post-scripts or migrating private state. The archive cache and measured tool state remain private below `.workspace/environment/tools/`. The only mandatory tool is `codex/bin/codex` under that prefix, with the Codex and Claude Code notices; chat runs inside Server and needs no Java runtime or service artifact. The managed `/usr/local/bin/codex` entry, generated at `.workspace/agents/tools/launchers/codex`, adds `--no-daemon`; Chat launches `codex app-server` directly.

Missing or unusable payload components cause explicit failure; there is no host execution fallback. Tool artifact readiness and chat protocol readiness remain separate measurements.

Claude is a pinned optional Engine-owned installation at `/opt/workflow/tools/claude/bin/claude`, stored in `.workspace/agents/tools/claude/<version>/`. `environment.tools.install` starts an asynchronous job, and `environment.tools.status` exposes revision, version, architecture, progress, operation ID and measured outcome. Engine downloads and verifies the pinned binary, checks its version and records interrupted work as failed. Unchanged failed work requires an explicit retry. Android submits only the tool ID and retry intent; it does not download, probe executables or install software itself.

## Declaring an environment

The current declaration is version 1 and lives at `<workspace-root>/.workspace/env.json`. The Engine does not import `container.json`, old `.workflow` directories, or historical runtime state.

```json
{
  "version": 1,
  "python": ["3.14", "3.13"],
  "node": ["24", "22"],
  "packages": ["ripgrep"],
  "env": {"PIP_INDEX_URL": "https://mirrors.example/pypi/simple"},
  "post_scripts": [
    {"id": "prepare", "run": "mkdir -p \"$HOME/.config/example\"", "user": "work"}
  ]
}
```

Python specifications may be a major version, minor series, or exact patch, such as `3`, `3.14`, or `3.14.7`. Node follows the equivalent forms `24`, `24.1`, or `24.1.0`. Missing arrays use the image's declared defaults; empty arrays disable the managed language. For nonempty arrays, the first entry is the default and the other versions remain installed alongside it. Duplicates and invalid specifications are errors.

Matching uses dotted version prefixes, so an installed `3.14.7` satisfies `3.14` and does not require another download merely because upstream releases a patch. Disabling a language must not silently expose an unrelated system interpreter: profile entry points shadow those commands with an explicit error and exit code 127.

`packages` lists additional Debian packages and defaults to an empty array. Invalid or duplicate names fail validation. Packages supplied by the customized image remain an irremovable baseline. `env` contains valid environment-variable names and string values without NUL characters. These values override image defaults, and an individual process can override them again. They are applied inside the guest, not injected into the host runtime or loader.

`post_scripts` defaults to an empty array. Each entry has `id`, `run`, and `user`, where user is `work` or `root`. Scripts run in order while building a new configuration; `root` is virtual guest root and does not grant Android device root. A failed step reports its failure and prevents activation. Retrying unchanged failed input requires an explicit retry.

## Persistent stores and guest paths

The Engine owns generations, activation records, logs, home, and toolchains beneath `.workspace/environment/`. A generation contains its own root filesystem. `/home/work` is backed by `stores/home/work`, and `/opt/toolchains` by `stores/toolchains`, allowing home and installed toolchain caches to survive a generation rebuild. User files appear at `/workspace`.

Agents live in the visible `.workspace/agents/` tree:

| Workspace path | Guest path | Contents |
| --- | --- | --- |
| `.workspace/agents/codex/` | `/home/work/.codex` (`CODEX_HOME`) | Codex home: configuration, credentials, sessions and logs |
| `.workspace/agents/claude/` | `/home/work/.claude` (`CLAUDE_CONFIG_DIR`) | Claude Code configuration directory, including its credentials and `.claude.json` |
| `.workspace/agents/tools/payload/<sha256>/` | `/opt/workflow/tools` | Verified mandatory payload: Codex and notices |
| `.workspace/agents/tools/claude/<version>/` | `/opt/workflow/tools/claude` | Optional pinned Claude Code installation, also bound at `/usr/local/bin/claude` once present |
| `.workspace/agents/tools/launchers/codex` | `/usr/local/bin/codex` | Engine-generated Codex entry point |

Chat and terminal processes bind the homes, mode 0700, over the persistent home and receive `CODEX_HOME` and `CLAUDE_CONFIG_DIR`, so both use one login and configuration per agent. Provisioning commands and post-scripts see neither the workspace nor these homes, and the post-script home stage never copies them. Version 1.0.0 has no migration: homes left in `stores/home/work/.codex` or `.claude` and payloads in the former `environment/tools/payloads` or `optional` directories are not read, so an earlier login must be repeated.

The typed `access` module classifies every `.workspace/` entry as a fixed folder, editable configuration, read-only content or private. FileWork enforces that allowlist, which the [FileWork reference](filework.md#paths-and-explicit-configuration) tabulates. Agent credentials (`auth.json`, `.credentials.json` and `.claude.json`) are private: they are never listed, read or written through file APIs and never logged. The guest mask is derived from the same tables. Processes that mount the workspace hide `state`, `environment`, `documents`, `uploads`, `corrupt`, `trash`, `engine.lock` and the whole `agents` tree below `/workspace/.workspace`. `config.json`, `env.json`, `proxy/` and `services/` remain readable and writable there. Agent configuration is edited in the guest at `~/.codex` and `~/.claude`, and tools are used at their mount points.

The runtime matches hides against resolved guest paths and maps a directory back to the guest through its longest host prefix. Masking individual credential files below `/workspace` would still let a recursive walk reach them through the home mount, so the alias is masked as a whole. The guest user can still read its own credentials at the agent home, where the agents need them; the mask keeps workspace walks, archives and searches of `/workspace` from reaching them.

The customized Debian image defines a `work` user with UID/GID 1000, home `/home/work`, and `/bin/bash`. Passwordless sudo provides virtual guest root for package and environment work. Toolchains live under `/opt/toolchains`: uv stores Python installations in `uv/python`, nvm stores Node installations in `nvm/versions/node`, and immutable profiles select their defaults and complete version sets. The `active` symlink points to a profile. Home seeds apply only when the store is absent; toolchain seeds merge missing entries without replacing existing contents or an existing active selection.

The runtime supplies its device allowlist and `/proc`; Environment binds the current reported DNS to guest `/etc/resolv.conf` without changing host DNS. `TZ` can be declared in `env.json`, otherwise the image's default applies. Bound host files do not have the generation's virtual attribute store: they appear owned by `work` with their host permission bits, excluding set-ID bits. Detailed image paths, default environment variables, and archive rules are specified in the [image format](image-format.md).

## Reconciliation and activation

First use explicitly asks the Server to reconcile. That enrolls the environment in automatic declaration monitoring. Afterwards saved or externally edited `env.json` changes are checked by the Engine, including after reconnect. A connection used only for files or local proxy services does not extract an environment that has never been requested. The attempted declaration is remembered so malformed or failed input cannot cause an endless rebuild loop.

A build prepares a candidate generation and persistent toolchain stores, installs additional packages, resolves and installs requested language versions, verifies all requested tools and packages, and finally runs post-scripts. Scripts execute from `/` inside the candidate; the user's workspace is not mounted during this configuration step. The Engine, not `envctl`, controls ordering, generation isolation, and retry behavior.

Scripts receive a private mode-0700 copy of `work`'s home. Each copied entry is checked against its source version while it is copied. Later writes to other, already copied files do not abort preparation; activation still checks every path changed by the scripts against that path's captured baseline. Activation compares content versions and merges script changes, preserving user home changes that the script did not touch during the build. Conflicting changes prevent activation and leave the old generation in use. Failed scripts never publish their home changes. Staging is cleaned after success or failure and must not enter backups or delivery artifacts. Toolchain cache additions can be reused by later builds.

Installation and verification do not change `active`. Once every required step succeeds, Environment chooses the lifecycle boundary for activation, with Server composing the affected domains. With no managed processes running it activates directly. Otherwise the existing generation remains usable and the verified candidate waits in `needs_restart`. The terminal and settings restart actions explain that processes will stop and ask the user to confirm. An explicit restart with no verified pending generation leaves processes alone; with a pending generation it stops the chat service and Engine-owned groups, waits up to ten seconds, and activates only after they exit. Engine then restores the previously running terminal resources using their retained cwd and dimensions. Android only observes the resulting generations and metadata.

Status distinguishes installation, building, readiness, pending restart, and failure, and exposes stage and progress for long work. A failed build can coexist with `usable:true` when an older verified generation remains available. This distinction keeps a failed requested configuration from being mistaken for a broken current terminal environment. File and session commands remain independent of long builds.

Current code conservatively retains old generations. Automatic garbage collection is not implemented. A future collector would need to respect all active, pending, and running-instance references before reclaiming generations or profiles; that constraint is not a claim that collection already exists. Remote/SSH connections are likewise only reserved interfaces in 1.0.0, and there is no host terminal or agent fallback.

## The envctl step interface

`/usr/local/libexec/workflow/envctl` performs individual guest-side steps. Progress goes to stderr; stdout contains only machine JSON, or is empty for steps without a result. It does not run post-scripts or decide when an environment should activate.

| Command | Guest user | Contract |
| --- | --- | --- |
| `apt-install PKG…` | root | Install missing packages, updating indexes only when needed; mark packages manual and clear apt caches and indexes |
| `apt-remove PKG…` | root | Remove extra packages and autoremove while preserving `/usr/local/share/workflow/base-packages` |
| `python SPEC` | work | Install alongside existing Python versions through uv; reuse a matching installation and return `{python,installed}` without activation |
| `node SPEC` | work | Load `nvm.sh --no-use` in an isolated subshell, install or reuse the version, and return `{node,installed}` without activation |
| `verify-many PY_JSON NODE_JSON [PKG…]` | work | Resolve the latest installed match for every specification, verify all interpreters and packages, create or verify an immutable profile, and return `{profile,python:[...],node:[...],packages:{name:version}}` |
| `verify PY NODE [PKG…]` | work | Image-build convenience form for two singleton lists, returning scalar Python/Node values; the Server uses `verify-many` |
| `activate PROFILE` | work | Atomically switch `active` after a Server lifecycle decision or during image provisioning |
| `state [PKG…]` | any | Return `{profile,python,node,installedPython:[...],installedNode:[...],packages:{...}}`; defaults can be null, and installed arrays list all stored versions |

Profile identity depends on the complete ordered arrays of resolved versions. A single Python/Node pair uses `pyX.Y.Z-nodeA.B.C`; multi-version and empty variants use `set-<canonical-array-SHA256>`. Changing a secondary version, ordering, or disabled language therefore changes profile identity. `toolchains.json` records those arrays. `python` and `node` select defaults, while `pythons/X.Y.Z/bin/python3` and `nodes/A.B.C/bin/node` expose every selected version. Verification must fail when a package is absent or a requested interpreter cannot execute.

## Building and checking images

`image/versions.env` pins the Debian OCI digests, uv/nvm releases and download hashes, default language specifications, and base packages. `image/guest/provision.sh` configures the guest twice and compares state to check idempotence. `image/guest/envctl` becomes the runtime step interface.

`image/build.sh --arch amd64` and `image/build.sh --arch arm64` each build a complete customized image. `--profile base` explicitly selects the test/build base. Outputs under `artifacts/image/<arch>/` include the archive, index, SHA256, package manifest, state, and logs. Export checks compare the guest manifest, tar members, and attribute table. Compression uses at most two zstd threads and the shared `artifacts/.gradle.lock`.

On x86_64 hosts, `image/with-cross.sh arm64 COMMAND…` uses the digest-pinned static QEMU from `third_party/qemu-user/manifest.json`. It enters `podman unshare` and a private mount namespace, then mounts binfmt_misc only for that user namespace on Linux 6.7 or newer. It does not alter global host binfmt or network settings and does not use the user's tablet. QEMU and its licenses are build-tool artifacts, not image or APK contents.

Format tests run with `PYTHONPATH=image python3 -m unittest discover -s image/tests -v`. `image/tests/podman-roundtrip.sh` exercises the actual user, sudo, shell, interpreters, compiler, and repacking. `image/tests/toolchain-profiles.sh CONTAINER` covers multi-version and empty selections on a disposable instance. `python3 image/tests/inspect-artifacts.py IMAGE` checks ELF architecture, envctl source identity, the Codex symlink, and absence of build-time QEMU.

APK packaging checks the workspace profile, architecture, archive size, and SHA256 for its ABI. It does not accept vanilla Debian as a fallback. The [custom-image report](../archive/implementation-1.0.0/reports/rewrite/custom-images.md) and [status](../status.md) distinguish successful image construction from actual Android acceptance.

## Storage, configuration and service ownership

Environment is the foundation used by Workspace, FileWork, Terminal and Chat. Its typed store centralizes `.workspace/` keys, strict bounded reads, atomic publication, backup, quarantine, upload staging and the workspace lock. Each domain owns its document meaning and recovery policy. Known records use typed serde models with explicit preserved unknown fields; opaque documents remain a named boundary, not permission to index known JSON structures throughout the Engine.

Environment also owns workspace-delivered appearance, palette, fonts, overlay/client configuration and local service intent, executor epochs and measured receipts. These belong in its `config` and `services` modules. Chat owns agent/backend defaults and Terminal owns terminal settings; Server composes them into the existing revisioned configuration and wire contract. Ordinary workspace-file IO belongs to FileWork and uses Environment's configuration validation when the target is explicitly editable.

Canonical proxy YAML, providers and redacted logs live in `.workspace/proxy/`. The `services.proxy` document namespace still addresses those original files, while private content-hash/revision sidecars remain under `.workspace/documents/`. Other service files retain their classified service directories. Version 1.0.0 does not migrate the former `.workspace/services/proxy/` location. The [App configuration](../app/configuration.md) and [proxy](../app/proxy.md) references describe revision checks and measured local execution.

The `runtime` API owns guest commands, pipes, PTYs, process groups and stop/wait; [Runtime](runtime.md) implements the isolation executable and [Loader](loader.md) the guest transfer. Domains do not invoke the concrete guest directly. Public declaration remains `.workspace/env.json`; `.workspace/environment/environment.json` is the private lifecycle record. Their field names and existing declaration/activation fingerprints remain unchanged.
