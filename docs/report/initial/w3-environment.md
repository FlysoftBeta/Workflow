# W3 — workspace image, image format, container.json (report)

Status: **done except the arm64 workspace image, which is waiting on W2's engine** (2026-09-28). This update includes the planner's cheap-generations follow-up (§9). Long-term spec: [docs/environment.md](../../environment.md). It is in Chinese; §2 is the installer contract for W2.

## 1. Contract (`docs/environment.md`)

| § | Content |
| --- | --- |
| 2 | **Image format v2 (stable).** `image.tar.zst` is a zstd stream (window ≤128 MiB) containing a POSIX tar with members `metadata.json`, `attributes.tsv`, then `rootfs`. Rootfs members may only be `0`/`5`/`2`. Allowed PAX keys are `path`, `linkpath`, `size`, `mtime`. uid/gid are 0 and mode is informational. |
| 2.3 | **Metadata.** `format`, `formatVersion 2`, `type`/`typeVersion`, `profile workspace|base`, `architecture`, pinned `base`, `rootfs` counts, and the attributes digest. `requires` (`virtual-ownership`, `virtual-mode`, `hardlink-emulation`, `virtual-special-files`) must be enforced before extraction. Workspace images also carry `user`, `environment`, `toolchains`, `defaults`, `provision`. |
| 2.4 | **Sidecar.** Rows are `path type uid gid mode extra` with percent-encoded paths. Types are `d f l h c b p`. Five invariants apply: sorted rows, parent is a `d`, 1:1 in-order with tar members, `h` rows point at an earlier `f` primary with equal attributes, and counts/sha256 match the metadata. |
| 2.5–2.6 | **Installer duties and debian-trixie v1.** The `work` user, toolchain layout (`/opt/uv`, `/opt/nvm/current`), PATH, envctl, and the runtime binds and files the engine must provide. |
| 3–6 | **container.json v1 and the rest.** Fields `version`, `python`, `node`, `packages`, `env`; unknown keys are errors; 0.2.x files are migrated. Also covers the data layout under `.workflow/engine/`, the reconcile algorithm, the envctl protocol, activation/rollback/recovery, and the UX states. |
| 7 | Build and APK packaging. |

## 2. `image/` tooling

The tools use only the Python stdlib plus the `zstd` binary. They run on the host or inside a guest.

- **`wfimage/`**
  - `tarstream`: strict reader and writer. It deliberately does not use `tarfile`.
  - `attributes`, `manifest` (including prune and canonical lists), `tree` (rootless tar → dir + manifest), `pack`.
  - `verify`: strict reader, reference installer, and `to-tar`.
  - `fixture`: produces `good.tar[.zst]`, 23 `bad-*.tar` files and `fixtures.json` with the expected error codes. These are for W2's C installer tests.
  - `meta`.
- **`guest/`**
  - `provision.sh`: idempotent. Downloads are pinned by sha256.
  - `envctl`: the reconcile steps.
  - `capture-manifest.sh`: find + stat.
  - `pack-in-guest.sh`: the arm64 path.
  - `prune.list`, `canonical.list`, `placeholders/`.
- **`build.sh`**:
  1. pulls by digest and checks it;
  2. runs provision twice and requires identical `envctl state`;
  3. cross-checks the guest manifest against the export;
  4. packs and verifies.

  `--profile base` converts the Debian base for any architecture without executing it.
- **`versions.env`** pins:
  - Debian index `a99cfc51…`, amd64 `7792b1f7…`, arm64 `da496358…`;
  - uv 0.12.19 (per-arch sha256 plus licenses);
  - nvm 0.40.8 (per-file sha256);
  - defaults Python 3.14 and Node 24.

## 3. Artifacts

All artifacts are git-ignored under `artifacts/image/`.

| File | Bytes | sha256 | Rows | Unpacked |
| --- | --- | --- | --- | --- |
| amd64/image.tar.zst | 189,187,146 | 1bb92ed5c7f8099e0b95299987c7b1d61305f4f9ecd1d79b90f1489148563fb4 | 28,444 (3 h) | 898.5 MB |
| amd64/base.tar.zst | 19,492,617 | 44d7a00a64b12e81a9adfb1d75a1c6662beec7ce0930f6b082699514497cd577 | 3,269 | 78.8 MB |
| arm64/base.tar.zst | 19,059,130 | 044cc47f9d94d8af2295d8a5f525b2d94ca82197180f54f951e3e7ec6beecede | 3,264 | 100.6 MB |

- **Workspace image contents:** Python 3.14.7, Node 24.21.0, npm 11.19.0, uv 0.12.19, nvm 0.40.8 and 16 apt packages. The full package list is in `amd64/image.packages.tsv`.
- **Build time:** provision 57 s; the second provision run 1 s; pack 79 s.
- **arm64 base:** it is real aarch64 ELF, converted without emulation. It can serve now as W2's guest for M2–M5.
- **Compression trade-off (amd64 workspace image):**

  | Setting | Size | Decoder memory | Decode time |
  | --- | --- | --- | --- |
  | `-19 --long=27` (current) | 189.3 MB | 136 MB | ~2 s |
  | `-19` | 218.9 MB | 13 MB | ~2 s |

  I kept the first; W2 can ask for the second.

## 4. `:core` `top.flysoftbeta.workflow.core.environment`

| File | Contents |
| --- | --- |
| `ContainerSpec.kt` | Model, strict codec, 0.2.x migration, canonical fingerprint, dotted-prefix `VersionSpec`, explicit `format()` |
| `ImageInfo.kt` | `image.json` parsing, `ImageVariant`, `DebianTrixie` step → envctl argv |
| `ReconcilePlanner.kt` | Pure plan function. Outcomes: `UpToDate`, `ActivateOnly` (env-only change), or `Build` from the image or from a clone. Also checks the Verify output. |
| `Records.kt` | `generation.json`, `current`/`previous` activations (self-verifying fingerprint), `FailureRecord` |
| `EngineStore.kt` | Atomic pointers, `.partial` → commit, rollback, startup recovery, garbage collection, log pruning |
| `EnvironmentReconciler.kt` | `EnvironmentEngine` port (`install`/`clone`/`run`, to be implemented by `:app`). Emits `StateFlow<EnvironmentStatus>` with Applying, UpToDate, PendingRestart(`prompt` false for image updates), and Failed. A newer container.json supersedes an in-flight build via `collectLatest`. Failures are not retried automatically. |

- **One container model.** The old `ContainerConfig`/`BindMount` and `WorkspaceCodec.container` were removed. `WorkspaceState.container` is now a `ContainerSpec`. `WorkspaceRepository` parses, validates editor saves, writes defaults when the file is missing, and rewrites 0.2.x files once, keeping the original as `.bak`.
- **Note for W1b:** the new config store should use `ContainerSpecCodec` for container.json; the `core/config` package is still empty.

## 5. Tests

- **`python3 -m unittest discover -s image/tests`:** 27 OK, with ResourceWarnings treated as errors.
- **`image/tests/podman-roundtrip.sh`:** 31/31 ok (after the §9 redesign).
  - The image is converted with `to-tar` and imported, then smoke-tested:
    - identity and sudo: work 1000:1000, passwordless `sudo su`, setuid sudo 4755 root;
    - toolchains: Python/Node/npm/uv/nvm defaults, `sudo node`;
    - build tools and environment: locale, gcc, cmake, `uv venv`.
  - envctl steps:
    - `apt-install` is idempotent;
    - `apt-remove` keeps image packages;
    - switching Python 3.14→3.13 and Node 24→22 uninstalls the old versions;
    - login shell PATH is correct.
  - The in-guest repack produces a byte-identical `attributes.tsv` and equal metadata.
- **`flock … ./gradlew :core:test` (live tree, 2026-09-28):** compiles. 218 tests, 3 failed. All 3 failures are W1b's: `SessionPolicyTest` and `ArchiveAndComposerTest`, which look clock- or date-dependent.
  - All 40 environment tests pass: ContainerSpec 8, ReconcilePlanner 11, EngineStore 9, EnvironmentReconciler 12. The real `image.json` is accepted.
  - The workspace tests pass as well, including ConfigurationRecovery and WorkspaceRepository.
  - JUnit XML is in `artifacts/image/core-test-results/`.

## 6. Legacy removal

- **Deleted:** `runtime/workflow_daemon/`, `runtime/tests/`, `runtime/pyproject.toml`, `runtime/image/`, `tools/build-image.sh`, `tools/pack-image.py`, and `tools/smoke-codex.py`.
  - `smoke-codex.py` imported the daemon's `Runtime`. The audit recommended rewriting it as a standalone smoke test; that belongs to W4.
  - A tarball copy of everything deleted is at `artifacts/legacy-0.2/python-daemon-and-image-tools.tar.gz`, and the baseline shadow repo also has it.
- **Moved:**
  - `runtime/reference/*.json` → `docs/report/initial/legacy-0.2/runtime-reference/`;
  - the Codex `.rs` snapshots and their license → `artifacts/reference/codex/legacy-0.2-snapshot/`.
- **README:** the test command now points at `image/tests`, and an `image/` entry was added.
- **Left alone:**
  - `runtime/proxy/` (W9). `controller.example.json` is now unreferenced; the audit says to delete it.
  - The git-ignored `runtime/artifacts/`. Its `amd64/image.tar.zst` is the obsolete format-1 image.
  - Legacy docs (`docs/runtime.md`, `implementation-status.md`) that still mention the daemon.

## 7. Decisions worth review

- **Defaults are baked into the image.** This gives an offline, zero-config first run, and Claude Code needs Node. It accounts for most of the 189 MB.
- **Cheap generations (§9).** A rootfs copy happens only when the apt package set changes (Android refuses `link()`, so a copy is a real copy of about 580 MB).
- **Per-ABI images: accepted.** Flavors are `device` (arm64-v8a, arm64 image) and `emulator` (x86_64, amd64 image), documented in environment.md §7.1. The release workstream will change `app/build.gradle.kts`; W3 did not.

## 8. Waiting on others

- **W2 engine:** implement the installer against §2 using the fixtures, and implement `EnvironmentEngine`. Engine runtime duties: binds, `/etc/resolv.conf`, `TZ`, the home seed.
- **arm64 workspace image:** after M4/M5, install `arm64/base.tar.zst`, run `guest/provision.sh`, then `guest/pack-in-guest.sh /workspace/out`.
- **Integration:** wire the reconciler into `RuntimeService`, the file watcher and the Settings/Workbench UI, and do APK flavor packaging.

## 9. Follow-up: cheap generations (planner decision, 2026-09-28)

**Toolchain store.** `/opt/toolchains` is a persistent bind of `engine/toolchains/`. It is outside every rootfs generation.
- It holds uv Pythons (`uv/python/cpython-X.Y.Z-*`) and nvm Nodes (`nvm/versions/node/vX.Y.Z`) side by side.
- `profiles/pyX-nodeY/{python,node}` are relative links to those version dirs.
- `active` → `profiles/<name>`.
- PATH is `/opt/toolchains/active/{python,node}/bin`.
- `UV_PYTHON_PREFERENCE=system` makes `uv venv`/`uv run` default to the active Python while project pins still win. This was verified on the host and in podman.

**What each change costs.**

| Change | Mechanism | Cost |
| --- | --- | --- |
| `env` | New activation only | Metadata only |
| Python/Node | `envctl python/node` installs side by side (temp files and HOME stay in the store, so this is safe next to a running instance). `envctl verify` creates the profile, and a new activation records it. | The new versions only |
| apt package set | Rootfs clone | One rootfs copy (~580 MB). Toolchains (322 MB of the image) are no longer copied. |

**Instance lifecycle and rollback.**
- The runtime flips `active` atomically on the host in `prepareStart()`, just before an instance starts, so running instances keep their profile until restart.
- Rollback is an activation switch back.
- `previous` is the activation actually running when it differs from the old current, so at most one previous rootfs exists.
- After a successful restart, a previous activation on another rootfs is dropped and that rootfs is pruned. A toolchain-only previous is kept, since rolling it back costs nothing.
- GC removes unreferenced profiles and managed versions, which are tracked in `engine/toolchains.json`. User-installed versions are never removed.

**Image format (additive).**
- New metadata field `stores`: `/home/work` → `home/work` (if-absent) and `/opt/toolchains` → `toolchains` (merge).
- New requires token `store-seeds`, plus invariant 6: rows under a store are d/f/l, owned by the user, with no set-id bits and no hardlinks.
- **W2's installer:** extract those rows as plain files into `<generation>/seeds/<store>/`. Reconcile merges them into the stores with renames only.
- The reference installer, fixtures (2 new bad variants) and the round-trip all cover this.

**Engine port.** `run()` now always binds the toolchain store.
- `EnvironmentReconciler` lifecycle calls are `prepareStart()`, `instanceStarted(activation)` and `instanceStopped()`.
- GC never races a build.
