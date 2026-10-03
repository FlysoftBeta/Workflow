# Archived WebView transcript

Frozen pre-rewrite chat/Markdown renderer source, generated assets, npm lockfile and upstream licenses.
Production chat uses Compose with CommonMark and JLaTeXMath. Only the offline xterm terminal remains
in `web/` and `app/src/main/assets/web/`. These archived files are not inputs to the APK or npm tests.
Relative paths in the frozen build/tests describe their original `web/` location; restore there in an
isolated checkout to reproduce the old renderer. Current validation is documented in `docs/testing.md`.
