# Implementation

Workflow 1.0.0 has an Android client and a Rust Workspace Engine. The Engine owns the workspace; Android connects to it, renders its state, and performs platform operations that require Android APIs. Start with the [architecture](architecture.md), then read the [protocol](protocol.md) before changing a boundary between components.

The [workspace model](workspace.md) explains sessions, layouts, and drafts. The [Workspace Server](workspace-engine.md) explains persistence, recovery, files, and process ownership. The [environment](environment.md) describes how complete development environments are built and activated, while the [image format](image-format.md) specifies their distributable archive. The [container runtime](container-runtime.md) documents the compatibility layer beneath those environments.

The [Android client](android-client.md), [agent adapters](agents.md), and [local proxy](proxy.md) cover the other production components. [Dependencies](dependencies.md) identifies the pinned libraries and offline assets. These documents describe implemented mechanisms and their contracts; proposed extensions are called out explicitly. [Current status](../status.md) records which implementations have supporting acceptance evidence and which device or account scenarios remain unverified.
