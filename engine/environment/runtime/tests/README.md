# Container runtime acceptance

`native/` contains C probes and frozen guest/oracle workloads, not a container implementation.
`device/` drives the app-sandbox acceptance cases, and `android-harness/` packages the staged Rust
runtime and loader with those probes. The harness keeps `libworkflow-engine.so` as its test-local
filename; the production APK calls the runtime `libworkflow-runtime.so` and reserves the engine name
for the Workspace Server.

Use `../test-host.sh` for host acceptance, and `../test-android.sh` for Android acceptance. The Android
entry point stages Rust binaries, then invokes `tools/with-emulator.sh`; direct device execution and
alternate AVD boot scripts are not supported. Historical C sources are in `docs/archive/native-engine/`.
