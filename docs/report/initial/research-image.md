# Workspace image & declarative environment — research report

2026-09-26. Scope: `tools/build-image.sh`, `tools/pack-image.py`, `runtime/image/`,
`runtime/workflow_daemon/image.py`, `runtime/workflow_daemon/engine.py`,
`runtime/tests/test_image.py`, `app/src/main/java/.../core/{WorkspaceModels,WorkspaceCodec,WorkspaceRepository}.kt`,
`runtime/artifacts/amd64/`. Host: Fedora 44 x86_64, `podman 5.8.7`, no passwordless
`sudo`, no `qemu-user-static`/binfmt registered. No Gradle run, no tablet touched.

## 1. What exists today and what's wrong with it

**`tools/build-image.sh`** — builds via `podman build --platform linux/$arch`,
exports the container filesystem, hands it to the packer, then `zstd -T0 -15`s it
with a `.sha256` sidecar. Sound shape (staging dir, trap cleanup, digest sidecar),
but: (a) the Dockerfile's `FROM docker.io/library/debian:13-slim` is a **floating
tag**, not a pinned digest — two builds months apart silently get different bases;
(b) `arm64`/`arm/v7` builds require the *target arch's binaries to execute during
`RUN`* (apt postinsts, nvm/uv installers) — this needs real emulation, which is
exactly the thing missing on this host (§2); (c) nothing records the digest actually
pulled, or the resolved Node/Python patch versions, into the shipped metadata — the
existing `build.log` shows Node resolved to `24.21.0` and Python to `3.13.15` from
the `24`/`3.13` ARGs, but that provenance is lost after packing.

**`tools/pack-image.py`** — flattens the container export into
`metadata.json + rootfs/…` inside one tar. Good details worth keeping: it strips
PAX `path`/`linkpath` override headers before renaming (tested, handles Unicode
paths like `证书.pem`), and it pre-validates `..`/absolute members even though the
installer re-validates independently (defense in depth). Gaps: `metadata.json`'s
`sourceImage` is a bare tag string, not a pinned digest; `requires: [...]` is
written but **nothing ever reads it** — an engine that can't satisfy a declared
requirement would still install the image; there's no build timestamp or builder
identity for auditing a `runtime/artifacts/*` blob later.

**`runtime/image/Dockerfile`** — matches the brief's preinstalled list (`sudo curl
wget git nano unzip zip rsync ca-certificates gnupg locales tzdata build-essential
cmake`, plus `xz-utils zstd` for the guest's own tooling), creates `work`
(uid 1000, passwordless sudo, no password), installs nvm 0.40.8 / uv 0.12.19 and
bakes a default Node 24 / Python 3.13 via them. This is consistent with
"Python/Node managed via uv/nvm" — the image ships one resolved default, and
`container.json` reconcile (already implemented, see §4) is what changes versions
per-workspace afterward.

**`runtime/workflow_daemon/image.py`** — the installer is careful: rejects
absolute paths, `..`, duplicate members, symlink-through-hardlink tricks, refuses
to clobber an existing `rootfs/`, and — importantly, and *not yet documented
anywhere* — it masks the real host `chmod()` to `mode & 0o777` while writing the
**full** mode (including setuid/setgid) only into `attributes.json`/xattr. That
correctly prevents a guest-declared `setuid root` binary (Debian ships several,
e.g. `sudo` itself) from becoming a real host-setuid file. This should be called
out explicitly in the metadata/attribute spec (§3) so it isn't accidentally
"simplified away" later. Real gaps: symlink `user.*` xattrs return `EPERM` on this
host filesystem (confirmed by the existing `runtime/artifacts/engine-probes`
result cited in `docs/native-engine-design.md`), so `attributes.json` is the only
durable record for symlinks — fine for v1 (install-only), but the file is
**path-keyed**, which — as `docs/native-engine-design.md` already flags — breaks
under rename/hardlink/unlink-reopen once a live engine starts mutating the guest.
Also: if `unpack_image()` is killed mid-extraction (process death, not a caught
exception), the `.install-<random>` staging directory under `.workspace/container/`
is never swept — no startup reconciliation for orphaned staging dirs exists.

**`runtime/tests/test_image.py`** — 3 tests: PAX/Unicode rewrite, a full
install-preserves-attributes-and-refuses-reinstall round trip, and a `..`-escape
rejection. Good baseline, but doesn't cover: oversized/absent `metadata.json`,
wrong `schemaVersion`/`type` independently, duplicate-member rejection, device/FIFO
member handling, the setuid/setgid host-mask behavior above, or crash recovery.
See §5.

**`runtime/artifacts/amd64/image.tar.zst`** — 271,322,159 bytes (259 MiB) from an
896 MiB tar at `zstd -15` (28.89% ratio), built entirely from podman's build cache
(every Dockerfile step says `Using cache` in `build.log`) — so this artifact does
not by itself prove build reproducibility from a cold cache. There is no arm64
artifact yet; that's the actual target per the brief and is covered next.

## 2. Host capability probe (this Fedora x86_64 host)

| Capability | Result |
| --- | --- |
| `podman`/`docker` | podman 5.8.7 present; `docker` is podman's CLI shim |
| `podman build --platform linux/arm64` (needs RUN-time emulation) | **Not viable**: confirmed by running an arm64 image under podman → `exec container process: Exec format error` |
| `sudo -n true` | Fails ("需要密码" / password required) — no passwordless sudo, nothing was installed |
| `qemu-user-static` / binfmt | Not installed; `/proc/sys/fs/binfmt_misc/` has no arch interpreters registered, and registering one needs root we don't have |
| `skopeo` | **Present** (1.22.3) — can inspect/copy multi-arch manifests by digest without executing anything |
| `dpkg-deb`, `ar`, `zstd`, `python3` | All present |
| `mmdebstrap`, `debootstrap` | Not installed |
| Network egress | `registry-1.docker.io` and `deb.debian.org` both reachable |

**Conclusion: cross-arch emulation is unavailable and shouldn't be assumed.** The
brief's suggested alternative — pull arm64 layers directly and add packages via
`.deb` + `dpkg-deb -x`, deferring maintainer scripts — is the right cheap path, and
I prototyped it end-to-end in `artifacts/image-research/` (git-ignored, ~30 MiB
kept as evidence):

1. `skopeo inspect --raw docker://docker.io/library/debian:13-slim` → resolved the
   manifest list and **pinned the arm64/v8 digest**:
   `docker.io/library/debian@sha256:da496358bd6934d2bd6a563a33176a2e50eff5490c54b4ac6fb051b69fef4071`.
2. `skopeo copy docker://...@sha256:da49… oci:debian-13-slim-arm64:latest` — pure
   download, 8s, no execution, no emulation needed. Verified by trying to run the
   pulled image under podman: fails with `Exec format error`, proving no binfmt
   fallback silently emulated it.
3. Extracted the single rootfs layer with `tar`; `file rootfs/bin/dash` confirms a
   real `ELF … ARM aarch64` binary; running it directly on this host correctly
   fails ("可执行文件格式错误" / exec format error) — the rootfs is genuinely arm64,
   not accidentally amd64.
4. Downloaded `nano_8.4-1+deb13u1_arm64.deb` from `deb.debian.org` (trixie/main),
   `dpkg-deb -x` (data-only extract, **no dpkg invocation, no maintainer scripts
   run**) into the rootfs. `file` confirms `rootfs/usr/bin/nano` is
   `ELF … ARM aarch64 … dynamically linked`. `dpkg-deb -e` shows the package *does*
   carry a `postinst`/`prerm` — captured but deliberately not executed, per the
   brief's "defer maintainer scripts to a first-boot configure step."

This proves the whole no-emulation pipeline (pin digest → pull → extract → add
`.deb`s → defer scripts) works on this host today, for zero emulation cost. The
deferred piece — actually *running* those postinst scripts — has nowhere safe to
execute target-arch code on this host either; it must run on-device, inside the
existing proot compatibility backend (which already works, per
`docs/native-engine-design.md`/`runtime/workflow_daemon/engine.py`) or, later,
inside the native engine once G0–G2 pass. `mmdebstrap --foreign`/`--variant` is a
plausible alternative front-end for step 1–4 if more of the base system needs
assembling this way, but wasn't installable to test here (not present, no sudo);
the skopeo+dpkg-deb path needs nothing beyond what's already on this host.
Evidence: `artifacts/image-research/EVIDENCE.md`, `rootfs-listing.txt`,
`debian-13-slim-arm64/` (pinned OCI layout), `nano_arm64.deb`, `control-extract/`.

## 3. Image metadata schema, sidecar attributes, compression, integrity, shipping

**Metadata v2 (envelope/variant split, additive over the current v1 fields):**

```jsonc
{
  "schemaVersion": 2,                 // envelope format; independent of variant
  "type": "debian-trixie",            // opaque variant id (was conflated with schemaVersion)
  "variantSchema": 1,                 // this variant's own field set/semantics version
  "architecture": "arm64",
  "sourceImage": {                    // was a bare string; now pinned + auditable
    "reference": "docker.io/library/debian:13-slim",
    "digest": "sha256:da496358bd6934d2bd6a563a33176a2e50eff5490c54b4ac6fb051b69fef4071",
    "platform": "linux/arm64/v8"
  },
  "defaultUser": {"name": "work", "uid": 1000, "gid": 1000},
  "rootfs": "rootfs",
  "attributes": "attributes.json",
  "provenance": {
    "builtAt": "2026-09-26T11:41:00Z",
    "builderVersion": "tools/build-image.sh",
    "runtimeManagers": {"nvm": "0.40.8", "uv": "0.12.19"},
    "preinstalled": {"node": "24.21.0", "python": "3.13.15"}
  },
  "requires": ["linux-elf", "virtual-uid-gid", "virtual-mode", "bind-mounts"]
}
```

Splitting `schemaVersion` (envelope) from `type` (variant id) from `variantSchema`
(that variant's own version) is the "schema so the engine can handle variants and
future upgrades" the brief asks for: a future `debian-forky` variant, or a
`debian-trixie` v2 with different fields, negotiates independently of the outer
tar format. **The installer must start enforcing `requires[]`** against its own
declared capability set before extracting anything — today it's written by the
packer and never read, which defeats its purpose.

**Sidecar attribute table** (`attributes.json`, produced by the installer at
install time, one entry per rootfs member): keep the current path-keyed JSON shape
for v1 — it's install-time-only (nothing mutates the guest yet) — but document its
fields explicitly and its known limitation up front rather than silently:
`path`, `type` (`file|dir|symlink|hardlink|fifo|device|socket`), `uid`, `gid`,
`mode` (full, including setuid/setgid/sticky — the *virtual* value; the real host
mode is always masked to `mode & 0o777`, which is already correctly implemented
and must stay that way), `linkTarget` (symlinks), `specialType` (device/FIFO/
socket, virtual only — never `mknod`'d on the host). Path-keying is known to break
under rename/hardlink/unlink-reopen; `docs/native-engine-design.md` already
specs the fix (a generation-scoped, object-identity journal) for when the native
engine starts mutating a running guest. This report doesn't re-litigate that — it
just says: don't let a v1 "sidecar table" ship without this caveat written down
next to it, since it will otherwise look finished.

**Compression** — measured on the existing amd64 tar (896 MiB raw):

| Setting | Size | Ratio | Time (16 cores) |
| --- | --- | --- | --- |
| `zstd -T0 -15` (current) | 259 MiB | 28.9% | 29s |
| `zstd -T0 -19 --long=27` | **206 MiB** | 23.0% | 91s |

Recommend `-19 --long=27` for the artifact that actually ships (compression is a
one-time build cost; decompression cost on-device is what matters, and zstd
decompress speed is ~level-independent). Keep a fast `-3`/`-9` profile for local
dev-loop rebuilds behind a `--fast` flag in `tools/build-image.sh`. Not
recommending content-defined chunking/dedup for v1 — real complexity for a
one-shot bundled asset; resumable downloads (if ever needed) belong at the
transport layer (HTTP Range), not the archive format.

**Integrity** — keep the existing top-level `sha256sum` sidecar (the installer
already requires a caller-supplied trusted digest and refuses on mismatch); zstd
frames also carry their own content checksum, so outer corruption is already
caught by `zstd -d` failing before the tar is even parsed. Per-file digests in the
attribute sidecar are a nice-to-have for future delta-updates, not a v1
requirement — don't build it until there's a consumer.

**Size budget**: brief wants "out of the box." Current single-arch payload is
206–259 MiB depending on compression level; recommend a **≤300 MiB per
architecture, compressed** budget, comfortably met today. `build-essential`+`cmake`
are non-negotiable per the brief and are the dominant size cost already baked in
— don't look there for savings; look at compression level instead (above).

**Shipping location**: bundle the **arm64** image as an in-APK/AAB asset for
release builds, matching the app's existing precedent of bundling the ARM64
Codex/Mihomo executables and offline WebView assets, and AGENTS.md's explicit "do
not introduce a CDN requirement." A separate downloadable asset would trade a
larger initial APK for a first-run network dependency — directly against "must
work out of the box" and against the no-CDN constraint. On-device *base-image*
building (full debootstrap-equivalent on first boot) is strictly worse: needs
network, needs significant on-device CPU/time, and produces a device-specific,
harder-to-verify rootfs. The existing `container.json` reconcile mechanism (§4)
already *is* the on-device-build step, but only for the delta on top of a bundled
base — that split is correct and should stay. Keep amd64 as a dev/test-only
artifact in git-ignored `runtime/artifacts/amd64/`, never bundled, as today.

## 4. `container.json` schema, reconcile, UX states, directory layout

**Current v1 schema already implemented** (`WorkspaceModels.ContainerConfig`,
`WorkspaceCodec`, `ContainerEngine.configuration()` in `runtime/workflow_daemon/engine.py`,
all three kept in sync): `schemaVersion`, `imageType` (must be `debian-trixie`),
`image`, `user` (must be `work`), `pythonVersion`/`nodeVersion` (numeric, validated
by regex on both the Kotlin and Python side), `engine` (`native`|`proot`),
`environment` (map, key validated as a shell identifier), `packages` (apt names,
validated), `bindMounts` (`source`/`target`/`readOnly`; **`readOnly` is currently
rejected outright** because proot can't enforce it — correct and should stay
explicit rather than silently downgrading to read-write). This already covers
everything the brief's item 4 asks for except **pip/npm global installs**, which
don't exist yet — proposed additive v1.1 fields `pythonPackages: string[]` (→ `uv
tool install`) and `nodePackages: string[]` (→ `npm install -g`), same validation
posture as `packages`.

**What's missing is the reconcile algorithm's shape**, not the schema. Today
`ContainerEngine._apply()` unconditionally reinstalls all packages and reruns
nvm/uv every time reconcile fires — idempotent in *result*, but not a "diff
desired vs applied, produce a minimal step plan" as the brief asks, and a
container.json edit that lands *while* a build is already running is silently
dropped (`reconcile()` just returns `{"existing": True}` and never re-reads the
new file). Proposed reconcile algorithm:

1. Compute `desiredFingerprint = sha256(canonical JSON of container.json)`
   (fingerprinting already exists; canonicalize key order, which it doesn't
   explicitly do today via `sort_keys=True` — keep that).
2. If `desiredFingerprint == currentGeneration.appliedFingerprint`: report
   `up-to-date`, no-op. This makes the 2-second file-watch poll cheap and
   idempotent by construction.
3. Otherwise diff desired vs. the last-applied config for the generation lineage:
   package set difference (`toInstall`/`toRemove`), version changes
   (`pythonVersion`/`nodeVersion` → targeted `uv`/`nvm` commands only, not always
   both), environment (fully regenerated each time — cheap, no diff needed),
   pip/npm globals similarly diffed. Produce an ordered step list with a stage
   name per step, for reporting.
4. Clone the current generation's rootfs (hardlink clone where the filesystem
   supports it, else copy) into a new `generations/<id>/` directory; run the step
   plan against *that* clone via the existing proot-backed `command()` — never
   touch the live generation mid-build.
5. On success: `fsync`, then atomically flip `.workflow/engine/current.json` to
   the new generation id using the same temp-file + `ATOMIC_MOVE` pattern
   `WorkspaceRepository.atomicWrite()` already uses for `config.json`/
   `container.json` — no new primitive needed, just reuse it from the daemon side
   too. If any session already has a live shell/process pinned to the old
   generation, report `pendingRestart` instead of silently swapping under it.
6. On failure: leave the old generation active and untouched; keep the failed
   generation's directory (with `build.log`) for inspection; report `failed` with
   the failing stage, exit code, and a log tail.
7. If container.json changes again while a build is in flight: cancel/supersede
   the in-flight build (kill its process group, discard its half-built
   generation) and start a new one for the latest desired state, rather than
   dropping the edit (today's behavior).
8. Keep the last N generations for explicit rollback (`POST` to reactivate an old
   generation id by flipping `current.json` back); prune older ones only on
   explicit user action, respecting the size budget from §3.

**UX states** (matches the brief's "up-to-date / pending restart / building /
failed with log", and is consistent with the Chinese wording already used in
`docs/resource-store-next.md`: 已保存/正在应用/应用失败/当前生效版本):
`unconfigured` (no container.json yet) → `saved` (file written, not yet
validated) → `validating` → `building` (step plan executing against the new
generation) → `ready`/`up-to-date` (fingerprint matches, generation active) or
`pendingRestart` (new generation ready, but the live guest is still on the old
one — requires an explicit user "restart environment" action, never automatic,
consistent with the project's "never silently discard/override running work"
posture) or `failed` (stage, error, log path/tail). Saving container.json always
auto-triggers validate+build in the background (per the brief); only *activating*
a finished, different generation over a live one asks the user.

**Directory layout** — keep `container.json` at the workspace root (already
implemented, analogous to `config.json`), but move the *engine's own* derived
state from the current flat `.workspace/container/` to
`.workspace/.workflow/engine/`, matching the existing `.workspace/.workflow/proxy/`
convention documented in `docs/architecture.md` (today's container engine is the
one piece of runtime state that doesn't follow that convention):

```
.workspace/
  config.json
  container.json                      # desired state, unchanged location
  .workflow/
    engine/
      current.json                    # {generationId, activatedAt} — atomic pointer
      generations/<id>/{rootfs/, metadata.json, attributes.json, config.json, build.log, status.json}
      downloads/                      # staged image.tar.zst + .sha256 before install
    proxy/                            # existing, unchanged
```

## 5. Test plan

**Packer** (`tools/pack-image.py`): keep the existing PAX/Unicode test; add
metadata-envelope validation (all v2 fields present/typed), a symlink+hardlink+
setuid-bit round trip asserting the *packed tar header* still carries the full
mode (host masking is the installer's job, not the packer's), and a
build-determinism smoke test (pack the same source tar twice, diff the output
byte-for-byte; if it isn't identical, isolate whether the nondeterminism comes
from `podman export` or from the packer itself).

**Installer** (`runtime/workflow_daemon/image.py`): keep the existing 3 tests; add
oversized/absent `metadata.json`, wrong `schemaVersion` vs. wrong `type` as
independent assertions, duplicate-member rejection, a device/FIFO member
asserting no host file is created and `specialType` is recorded, an explicit
regression test for the setuid/setgid host-chmod masking (`os.stat(target).st_mode`
must never carry `S_ISUID`/`S_ISGID` even when `attributes.json` does), a
hardlink-to-symlink rejection test (currently only the happy path is tested), a
crash-recovery test (kill mid-extraction, assert the next `unpack_image()` call
either resumes cleanly or a startup sweep removes the orphaned `.install-*`
directory — the sweep doesn't exist yet and should be added), and, once `requires[]`
is enforced, a fail-fast test that rejects an unsatisfiable requirement before any
extraction.

**Reconciler** (`runtime/workflow_daemon/engine.py`): keep the existing HTTP-level
503/`nativeEngineAvailable` test; add a `command_runner` seam so unit tests don't
need a real `proot`/`apt-get`, then test: idempotency (unchanged config → zero
new commands), the diff-based step plan (package add/remove sets, version-change
triggers, environment regeneration) as isolated unit tests once extracted from the
monolithic `_apply()`, atomic activation (`current.json` never points at a
partial/nonexistent generation, verified via crash-injection at each commit
point), rollback to a prior generation, concurrent-edit supersession (a
container.json edit mid-build produces a build of the *latest* desired state, not
a dropped edit), and the `failed` state exposing stage/error/log-tail.

**Host/build-capability tests** (new, given §2): a preflight check in
`tools/build-image.sh` that fails fast with an actionable message when
`--platform linux/arm64` can't actually execute anything (today it fails deep
inside a `RUN` step with a bare exec-format error); and, once the skopeo+
`dpkg-deb` no-emulation path is implemented, tests that the pulled base matches
the pinned digest, the extracted rootfs is genuinely target-arch (checked via
`file`/ELF header, never by executing it on the build host), added `.deb` files
land without invoking `dpkg` or their maintainer scripts, and the deferred
postinst/prerm scripts are recorded in a manifest for the on-device first-boot
configure step to execute later inside an environment that can actually run
target-arch code (proot today, native engine once it passes G0–G2).
