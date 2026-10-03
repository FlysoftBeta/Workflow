#!/usr/bin/env bash
# Build the complete embedded Rust Engine (Server, runtime, loader) for PackageManager extraction.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(dirname "$here")
if [[ ${WORKFLOW_BUILD_LOCK_HELD:-0} != 1 ]]; then
  exec "$repo/tools/with-build-lock.sh" "$0" "$@"
fi
ndk=${ANDROID_NDK:-$HOME/Android/Sdk/ndk/30.0.15729638}
tc="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin"
out=${ENGINE_ANDROID_OUT:-$repo/artifacts/engine/android}
abis=("$@"); ((${#abis[@]})) || abis=(arm64-v8a x86_64)
ENGINE_ANDROID_OUT="$out" "$here/runtime/build-android.sh" "${abis[@]}"
for abi in "${abis[@]}"; do
  case "$abi" in
    arm64-v8a) target=aarch64-linux-android; cargo_target=AARCH64_LINUX_ANDROID ;;
    x86_64) target=x86_64-linux-android; cargo_target=X86_64_LINUX_ANDROID ;;
    *) echo "Unsupported ABI $abi" >&2; exit 2 ;;
  esac
  env "CARGO_TARGET_${cargo_target}_LINKER=$tc/${target}28-clang" CARGO_BUILD_JOBS=2 \
    RUSTFLAGS='-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,noexecstack' \
    cargo build --manifest-path "$here/Cargo.toml" --locked --release --target "$target" -p workflow-engine
  cp "$here/target/$target/release/workflow-engine" "$out/$abi/libworkflow-engine.so"
done
