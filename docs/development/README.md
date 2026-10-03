# Developing Workflow

This section explains how to change the project. It complements the [product definition](../product/README.md), [UX design](../ux/README.md), and [implementation reference](../implementation/README.md): a feature proposal describes work that may happen, while an implementation document describes the system that exists.

Start with the [build guide](building.md) to prepare a development host and produce Debug or signed Release APKs, then use the [contribution workflow](contributing.md) for a normal change. The [multi-agent workflow](multi-agent.md) explains how to divide a change into independently owned worktrees, test each result, and integrate the recorded commits. [Testing](testing.md) identifies the useful checks and the actual Android acceptance boundary. [Repository layout](repository-layout.md) distinguishes source, local inputs, generated output, and historical evidence.

A new feature or module begins as a [proposal](proposals/README.md) when it changes a user-facing contract or an architectural boundary. Keep the proposal short enough to review before implementation. Once the change ships, move durable decisions into the product, UX, or implementation documentation and leave execution evidence in [reports](../report/README.md). A task packet is an assignment, not a second product specification.
