# Terminal resources

`workflow-terminal` owns stable terminal identities, metadata, generation resets,
OSC title and working-directory observations, and reference-based resource cleanup.
It depends on `workflow-environment` for typed private storage and PTY operations.
The Server supplies typed arguments and converts raw output bytes to its transport
encoding; this crate has no protocol method dispatcher.

Terminal metadata survives an Engine restart. A persisted process does not: loaded
resources start ended and can be rehydrated through attach. Existing live and exited
processes retain their generation until an explicit restart or clear. Output remains
the runtime's bounded process-lifetime ring, and a mismatched generation resets the
read offset. OSC observations are scanned without requiring a client attachment.

Workspace supplies the set of terminal IDs referenced by live sessions. A resource
without a reference receives a fifteen-second grace period before its process is
stopped. Closing a view does not stop the terminal directly. Environment activation
can restore the set of resources that were running before the old runtime stopped.

`working_directory` and `resolve_workspace_path` perform pure guest-path operations.
The latter yields only a candidate: opening it must still pass FileWork's filesystem,
private-state and symlink checks. UI selection, terminal link parsing, rendering and
attachment behavior remain separate follow-up work.
