# Making a change

Workflow is a local-first Android client backed by an authoritative Rust Workspace Engine. Before editing, read the product behavior and UX relevant to the change, then the implementation contract it crosses. Changes to sessions, files, environment, and services must preserve the ownership rules in [architecture](../implementation/architecture.md). The JSONL boundary is specified in [protocol](../implementation/protocol.md).

## Choose a bounded result

Describe the behavior that should change and the acceptance evidence that would demonstrate it. An interface change usually needs a short proposal so callers and owners agree on the new contract before working in parallel. A small bug fix can use its task packet directly. Avoid introducing a second copy of state to make one screen easier to implement.

Use an isolated task checkout when more than one contributor is active. `tools/workflow task create` reserves explicit source paths and writes a task packet. The contributor owns those paths until the coordinator integrates the handoff. If implementation reveals a missing ownership area, revise the task's scope through the tool before editing it; existing overlapping claims are rejected.

## Implement and verify

Keep implementation details close to their module and keep reader-facing explanations in the appropriate documentation layer. Product prose explains what the user can do; UX prose explains the interaction; implementation prose explains how the code delivers it. Write maintained documentation in clear English, using paragraphs for reasoning and tables or lists only when the information benefits from comparison.

Run the narrowest meaningful checks, then the integration checks required by the changed boundary. Prefer `tools/workflow check` so logs, source fingerprints and test reports stay together. Android acceptance consumes a frozen app/test APK pair through `tools/workflow device`; a successful compile does not demonstrate device behavior. Root and destructive tests are restricted to the disposable emulator, never the user's daily tablet.

A check whose source changes while it runs is recorded as `source_changed`. Fix the source or finish editing, then run the relevant check again. Do not edit scripts that are already executing or waiting for a resource lease. Commands used by validation must stay in the foreground so cancellation can stop their process group.

## Deliver and integrate

Commit only owned source files. A handoff requires a clean task checkout, an in-scope diff, and successful required checks matching the current source fingerprint. Write one concise English summary explaining the result, the decisions a reviewer needs to understand, the evidence, and any limits. The [handoff template](templates/handoff.md) provides prompts without prescribing a long checklist.

The coordinator reviews the delivered commit before invoking `tools/workflow integrate`. If Git reports a conflict, the tool leaves that merge intact. Resolve and commit it, or abort it, then use `tools/workflow finish` to reconcile the task record with Git. Run the checks appropriate to the combined change before declaring the integration complete. `tools/workflow archive` removes a clean integrated task checkout while retaining its branch, handoff and evidence.

Generated output, local workspace data, credentials and signing keys are not source. The initial repository import and subsequent commits must honor `.gitignore`; inspect the staged diff and file inventory before committing. Publishing or deployment is a separate action from a local Git commit.
