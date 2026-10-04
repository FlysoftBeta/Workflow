# Engine tools distribution

`package.py` builds one verified Linux payload per architecture from the pinned Codex release: `codex/bin/codex` and the `codex/bin/codex-code-mode-host` it starts for Code Mode, both taken from the same release's MUSL archives. The APK carries `assets/environment/tools/tools.json` and `tools.zip`. A standalone Server accepts the same pair through its tools directory option. Codex is an ordinary guest executable, not an Android native library.

The catalog records every file's size, SHA-256 and executable mode, the payload archive identity, architecture, tool versions and the pinned optional Claude download. Safe extraction rejects links, traversal, extra members and mismatched content. The Codex and Claude Code legal notices are retained in the payload. All downloads are cached under ignored `third_party/.cache/engine`; an invalid existing cache fails instead of being silently replaced.

The Server verifies and binds a content-addressed payload into every generation at `/opt/workflow/tools`. Existing generations require no migration or post-script replay. Codex's managed CLI launcher supplies `--no-daemon`; chat uses its app-server entry directly. Engine owns the optional Claude installation, including download, hash and size checks, measured version, interruption state and retry. The App only projects the tool catalog and requests an installation.

Chat runs inside the Server binary, so the payload carries no Java runtime or chat service. Host runtime proof does not establish Android device acceptance.
