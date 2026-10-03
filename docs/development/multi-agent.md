# Working with multiple agents

The coordinator owns the shared contract and the final integration. Each contributor receives a bounded objective, exclusive source paths, an isolated Git worktree, and an agreed set of checks. The repository tools are model-independent: they prepare work and record evidence, but do not launch a model, message another chat, approve an agent request, or publish code.

Create new subagents with `fork_turns="none"`. Supply the generated task packet and the few relevant documents instead of copying a long conversation. Contributors should work independently and deliver once; a concrete blocker, ownership conflict, or contract change is a reason to contact the coordinator earlier.

## Isolate writes before parallel work

The repository must have an initial commit before a task worktree can be created. From the coordinator checkout, initialize the local records and allocate a task:

```sh
tools/workflow init
tools/workflow task create editor-reading-position \
  --objective "Preserve the reading position when the editor is resized." \
  --owns app/android/src/main/java/top/flysoftbeta/workflow/feature/editor \
  --owns app/android/src/androidTest/java/top/flysoftbeta/workflow/feature/editor \
  --checks app-unit --checks android
```

The command creates `codex/editor-reading-position` and prints its checkout path. Its Markdown packet is stored under `artifacts/workflow/tasks/`. Start the agent in that checkout. Claims are plain repository-relative files or directories; a parent directory overlaps its descendants. A coordinator-authorized shared-file exception uses `task scope --share-path PATH --reason TEXT` together with an explicit file claim. It records the reason and applies only to that existing file, never a directory; ordinary overlapping claims remain rejected. Use this narrowly for agreed integration edits and review the merge deliberately. They are deliberately exclusive, so two contributors cannot accidentally claim the same implementation. `tools/workflow task scope NAME --owns PATH ...` replaces an active claim after checking other tasks and the paths already changed.

Worktrees live under the primary checkout's ignored `artifacts/workflow/checkouts/`. Their source and build directories are independent. Machine-local `local.properties` and the pinned prebuilt cache may be linked from the primary checkout. `tools/workflow prepare NAME` takes independent, copy-on-write image snapshots where supported; an agent cannot overwrite another task's image through a shared writable image link. Signing material is not copied or linked into task checkouts. Release signing belongs to the coordinator checkout.

## Run checks independently

```sh
tools/workflow prepare editor-reading-position
tools/workflow check app-unit --task editor-reading-position
tools/workflow check android-apk --task editor-reading-position
```

Every run receives a unique directory under the primary checkout's `artifacts/workflow/runs/OWNER/`. `run.json` records the command, source fingerprint, result, timestamps and artifact digests. Text output and selected test reports accompany it. Source changes during a command invalidate its result even if the command exits successfully. There is no automatic retry that could hide an intermittent failure.

The suite catalog in `tools/workflow-suites.json` is the executable mapping from a check name to an argv array. Lightweight checks can run concurrently in separate worktrees. Heavy Cargo and Gradle work uses the primary checkout's existing `artifacts/.gradle.lock`, with Cargo restricted to two jobs. The per-task lease also prevents two commands from corrupting the same checkout's build output. Direct heavy commands should use `tools/with-build-lock.sh`; the wrapper and module build scripts use the same global resource. Do not wrap `tools/workflow check` in another build lock.

`WORKFLOW_BUILD_LOCK_HELD=1` is an internal nesting signal passed only after a caller owns the build lease. It is not a way to bypass serialization. The lock wrapper closes the lock descriptor in child processes so a daemon cannot retain it after the caller exits.

## Share one emulator

An `android-apk` run copies both APKs under the build lease and records their digests. Pass that run directory to the device command:

```sh
tools/workflow device --task editor-reading-position \
  --apk-run /absolute/path/to/the/android-apk/run \
  --class top.flysoftbeta.workflow.feature.editor.EditorEngineAcceptanceTest
```

The device runner rejects a changed APK or a checkout that differs from its build snapshot. It queues through `tools/with-emulator.sh`, whose device lock and AVD catalog belong to the primary checkout. Every worktree therefore competes for the same physical resource. The emulator uses 1536 MiB, two cores, disk-backed temporary overlays, and no snapshots. The wrapper refuses to take over an occupied port and removes only the emulator it started.

Root is an explicit device-run option. A root proxy run also supplies the instrumentation gates:

```sh
tools/workflow device --apk-run /absolute/path/to/the/android-apk/run \
  --class top.flysoftbeta.workflow.platform.proxy.ProxyEmulatorRootTest \
  --arg proxyEmulator=true --arg proxyEmulatorRoot=true --root
```

All adb calls select the wrapper's emulator serial and verify emulator hardware. The driver clears only this disposable emulator's Workflow data before the test. Instrumentation must report a non-empty successful suite; adb's exit status alone is insufficient. Skipped tests fail acceptance unless the caller explicitly chooses `--allow-skips`. The log records that choice and the actual skipped count. Configure `ANDROID_HOME`, `WORKFLOW_AVD`, or `WORKFLOW_EMULATOR_PORT` when the machine needs a different installed SDK or disposable AVD.

## Handoff, conflicts and recovery

After committing the task's changes, run `tools/workflow handoff NAME --summary-file PATH`. The tool checks ownership, cleanliness, required results and source identity, then records the exact commit for review. A later source change invalidates integration. The coordinator invokes `tools/workflow integrate NAME` from a clean checkout after reviewing the diff; no agent should integrate another agent's work itself.

If review needs another revision, `tools/workflow task reopen NAME --reason TEXT` returns the checkout to active work while preserving its earlier handoff, evidence and a Git reference to the reviewed commit. A superseded task can be cancelled with `task cancel`; its claim is released, but dirty source is retained and archival still requires a clean committed checkout. If a task needs a substantially newer base, create a successor from that base and transfer the owned commits deliberately instead of silently changing the recorded base.

A conflicting merge remains in Git for deliberate resolution. `tools/workflow finish NAME` recognizes either a committed resolution or an aborted merge, returning an aborted task to `ready`. Once the result has been integrated, or a cancelled task has a clean retained commit, `tools/workflow archive NAME` removes the clean worktree. The branch, packet, handoff, and immutable run evidence remain available.

Cancellation stops the owned validation process group before releasing its resource lease. Interrupted task setup retains the checkout rather than force-removing source. A machine crash can leave a run marked `running`; inspect its PID-independent logs and Git state before retrying, and create a new run rather than rewriting old evidence. Ownership records coordinate trusted contributors; they are not a filesystem security sandbox.
