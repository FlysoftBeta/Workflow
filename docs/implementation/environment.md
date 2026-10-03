# Development environment

An environment is the complete runtime in which terminals and coding agents work: its image, packages, language versions, variables, mounts, home, and process lifecycle. `.workspace/env.json` is its declaration, rather than the environment itself. The Rust Workspace Engine owns that declaration's application and publishes measured health; Android renders the result. The [protocol](protocol.md) defines the client boundary, and the [image format](image-format.md) defines the distributable archive.

Both Android ABIs ship a customized `workspace` image with the APK. These images contain the `work` user, shell, development tools, uv, nvm, Python, Node, and `envctl`, so using default toolchains does not require an initial network installation. Additional versions or packages can require downloads. A `base` image is only a build or runtime-test input and cannot serve as the application's environment or release fallback. The APK separately supplies Codex.

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

The Engine owns generations, activation records, logs, home, and toolchains beneath `.workspace/environment/`. A generation contains its own root filesystem. `/home/work` is backed by `stores/home/work`, and `/opt/toolchains` by `stores/toolchains`, allowing home and installed toolchain caches to survive a generation rebuild. User files appear at `/workspace`, with private `.workspace` state masked by the Server.

The customized Debian image defines a `work` user with UID/GID 1000, home `/home/work`, and `/bin/bash`. Passwordless sudo provides virtual guest root for package and environment work. Toolchains live under `/opt/toolchains`: uv stores Python installations in `uv/python`, nvm stores Node installations in `nvm/versions/node`, and immutable profiles select their defaults and complete version sets. The `active` symlink points to a profile. Home seeds apply only when the store is absent; toolchain seeds merge missing entries without replacing existing contents or an existing active selection.

The runtime supplies its device allowlist and `/proc`; the Server binds the current reported DNS to guest `/etc/resolv.conf` without changing host DNS. `TZ` can be declared in `env.json`, otherwise the image's default applies. Bound host files do not have the generation's virtual attribute store: they appear owned by `work` with their host permission bits, excluding set-ID bits. Detailed image paths, default environment variables, and archive rules are specified in the [image format](image-format.md).

## Reconciliation and activation

First use explicitly asks the Server to reconcile. That enrolls the environment in automatic declaration monitoring. Afterwards saved or externally edited `env.json` changes are checked by the Server, including after reconnect. A connection used only for files or local proxy services does not extract an environment that has never been requested. The attempted declaration is remembered so malformed or failed input cannot cause an endless rebuild loop.

A build prepares a candidate generation and persistent toolchain stores, installs additional packages, resolves and installs requested language versions, verifies all requested tools and packages, and finally runs post-scripts. Scripts execute from `/` inside the candidate; the user's workspace is not mounted during this configuration step. The Engine, not `envctl`, controls ordering, generation isolation, and retry behavior.

Scripts receive a private mode-0700 copy of `work`'s home. Activation compares content versions and merges script changes, preserving user home changes that the script did not touch during the build. Conflicting changes prevent activation and leave the old generation in use. Failed scripts never publish their home changes. Staging is cleaned after success or failure and must not enter backups or delivery artifacts. Toolchain cache additions can be reused by later builds.

Installation and verification do not change `active`. Once every required step succeeds, the Server chooses the lifecycle boundary for activation. With no managed processes running it activates directly. Otherwise the existing generation remains usable and the verified candidate waits in `needs_restart`. The terminal and settings restart actions explain that processes will stop and ask the user to confirm. An explicit restart with no verified pending generation leaves processes alone; with a pending generation it stops Engine-owned groups, waits up to ten seconds, and activates only after they exit.

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
