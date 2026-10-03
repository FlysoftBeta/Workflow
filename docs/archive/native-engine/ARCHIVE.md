# Archived C container engine

This is the frozen implementation formerly at `native/engine/`, including its loader, generator,
upstream licenses and original test tooling. It is not part of any production build or current
acceptance entry point. Relative paths in these frozen files describe their original location.
For historical reproduction, restore this tree to `native/engine/` in an isolated checkout.

Production replacements: `engine/runtime/`, `engine/loader/`, and `engine/server/`.
The retained, actively maintained acceptance oracles and app harness are now under
`engine/runtime/tests/`; run `engine/runtime/test-host.sh` or `engine/runtime/test-android.sh`.
The Rust syscall generator is `engine/runtime/tools/generate-syscalls.py --check`.
