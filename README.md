# Workflow

Workflow 1.0.0 is an Android workbench backed by a Rust Workspace Engine. The Workbench presents files, native Compose conversations, Sora editors, and xterm terminals as panels. A Workspace owns the user's files, sessions, drafts, services and complete execution environment. Codex, Claude Code and terminal processes run only inside that environment.

The Android client connects to the embedded Engine and keeps only connection profiles and workspace-delivered local preferences. Workspace configuration lives at `<workspace-root>/.workspace/`. Both Android ABIs ship customized images with their default development tools. Remote and SSH bootstrapping have an extension interface, but no remote connection is implemented in this release.

## Find the right document

[Product design](docs/product/README.md) explains the work model and supported behavior. [UX design](docs/ux/README.md) describes visual rules and interaction. [Implementation](docs/implementation/README.md) explains the modules and their contracts. [Development](docs/development/README.md) describes how to propose, implement, verify and integrate a change. The [status record](docs/status.md) distinguishes measured acceptance from known limits.

Historical reports, retired implementations and their original licenses are indexed in [the archive](docs/archive/README.md). Historical originals retain their original language; maintained documentation is written in English. New change-specific evidence belongs in [reports](docs/report/README.md), while large local logs and artifacts remain ignored.

## Build and test

Use JDK 25, Rust 1.93.1, Python 3.11 or newer, and the Android toolchain versions listed in [dependencies](docs/implementation/dependencies.md). Configure the SDK locally and build the two customized images as described in [environment construction](docs/implementation/environment.md). Prebuilt executables are verified against checked-in manifests.

```sh
tools/workflow init
tools/workflow check core
tools/workflow check android-apk
```

The APK check records a frozen app/test pair. Use its run directory with `tools/workflow device` for acceptance on the one shared disposable emulator. The [testing guide](docs/development/testing.md) explains suite selection, host fixtures, root gating and release verification. `device` packages target ARM64 and `emulator` packages target x86_64; minSdk remains 28.

## Work in parallel

`tools/workflow task create` creates an isolated Git worktree, reserves explicit source ownership, and writes a task packet. Checks run independently, while heavy builds and device use share separate global locks. A handoff records a clean scoped commit and matching evidence; the coordinator reviews and integrates it. Read the [multi-agent workflow](docs/development/multi-agent.md) before starting concurrent work.

Source modules have their own READMEs. Generated output, local workspace data and signing material live outside the tracked source. Release packages are written to `artifacts/delivery/1.0.0/`; release commands and the documented device restrictions remain part of the working agreement in [AGENTS.md](AGENTS.md).
