# Reference-store filesystem fixtures

`FileSystem`, `FileStat`, `MemoryFileSystem`, `JvmFileSystem` and `FileWatcher` support the frozen
Kotlin writer tests only. The port, implementations and writer extensions are exported through
Gradle test fixtures and are absent from the production core JAR. Shared `FileEntry`, path helpers
and hashes remain in the production module because the RPC client uses them.
Production workspace files and observation belong to the Rust Server; Android uses Engine RPC.
The Android fsync adapter needed by instrumented reference tests lives in `app/src/androidTest/`.
