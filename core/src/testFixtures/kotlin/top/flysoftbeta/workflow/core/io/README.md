# Reference-store filesystem fixtures

`JvmFileSystem` and `FileWatcher` support the frozen Kotlin writer tests only.
Production workspace files and observation belong to the Rust Server; Android uses Engine RPC.
The Android fsync adapter needed by instrumented reference tests lives in `app/src/androidTest/`.
