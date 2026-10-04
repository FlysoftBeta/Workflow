# Terminal

`engine/terminal` is package `workflow-terminal`. It owns terminal identities, launch settings, metadata, runtime PTY attachment, output generations, working-directory tracking and cleanup. It uses `workflow-environment` for guest execution and typed persistence. Server maps its operations to [`terminal.*`](protocol.md#terminal-resources); Android owns only a connection-scoped xterm attachment.

## Resources and lifetime

A terminal has a stable ID, ordinal, title, optional custom title, guest-absolute cwd, rows, columns, process state and output generation. Metadata is persisted in `.workspace/state/terminals.json` through Environment's store. The process output ring is bounded and exists only for the Server lifetime; it is not a durable transcript.

Creating a terminal allocates its identity in Engine, whether or not the environment is usable. Without a usable environment the resource is `starting` with no process and keeps its requested directory and size; the first attach after the environment becomes usable starts it, and attach is refused with `environment_unavailable` before then. A client can therefore open the terminal's view while the environment prepares. Attaching does not restart an existing live or exited process; explicit restart does. A Server restart can leave metadata whose process is gone, and attachment can rehydrate that resource. Renaming and clearing are Engine operations. Clearing retained output advances generation so every client observes the same reset.

Closing a panel removes a view only. Engine examines references in live workspace sessions and retains an unreferenced resource for a fifteen-second grace period before cleanup. Cancelling an Android attachment never stops its Engine resource. A real environment activation restores previously running terminals using their retained cwd and size; restart with no verified pending generation leaves them alone.

Environment provides the PTY, controlling terminal, resize, signal and wait operations. Product execution requires a verified environment, with no Android-shell or host PTY fallback. Host/JNI launchers exist only as explicit test fixtures. The embedded Server stops its processes when its stdio transport closes.

## Output and working directory

A read returns metadata and a generation-tagged byte window together. The client applies reset, metadata and matching bytes as one frame. A generation mismatch resets reading to offset zero; clipping discloses the actual retained starting offset. Reading continues after EOF so another client's restart or rename can be observed.

Engine cursors describe bytes, not rendered screen rows. xterm owns wrapping, alternate-screen state and viewport geometry. Android resets its decoder, selection and disposable scrollback when a generation change or clipped window invalidates them. Large touch scrollbars therefore use xterm rows rather than a second Engine scrollback database.

Terminal tracks OSC 7 working-directory updates incrementally across output chunks, with the launch directory as fallback. Generation changes and lost scrollback must invalidate stale parser and navigation state. File navigation is authoritative on the Engine side: Terminal supplies identity, generation and cwd, FileWork checks safe workspace paths, and Server composes the reply. A client must not reconstruct the host workspace root or search guessed parent listings.

Candidate paths can include diagnostic line/column suffixes, but guest system/home paths, nonlocal file URIs, symlink traversal and Engine-private paths cannot be opened as workspace files. `terminal.resolvePaths {terminalId,generation,candidates}` returns the checked generation/cwd and one result per candidate. Resolved results carry the original text, workspace path, kind and optional line/column; rejected results carry only the text. Stale generations fail rather than using current metadata for an old link. Its wire shape is part of the [protocol](protocol.md#terminal-resources), and source tests must cover chunk boundaries, URI decoding, Unicode, traversal and stale generations.

## App interaction and acceptance

The [Workbench adapter](../app/workbench.md) owns link hit regions, tap-versus-scroll arbitration, visible selection handles, copy, and a large draggable scroll thumb. These four terminal repairs need joint API 28 touch acceptance while streaming, wrapping, rotating and using the keyboard. The presence of a callback or a passing JavaScript test does not prove that interaction works on Android.

`TerminalAttachmentTest` uses a managed fixture for generation/byte alignment, repeated EOF, remote restart observation and cancellation without process termination. Rust tests cover terminal lifecycle and resolution policy. Real guest exec, PTY and stop need isolated runtime/device tests with source-bound frozen artifacts. [Status](../status.md) records pending evidence without extending historical acceptance claims.
