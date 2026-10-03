#!/usr/bin/env bash
# Rust runtime and freestanding Rust loader, both Android ABIs, API 28.
# ENGINE_ANDROID_OUT/<abi>/{libworkflow-runtime.so,libworkflow-loader.so}
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../.." && pwd)
ndk=${ANDROID_NDK:-$HOME/Android/Sdk/ndk/30.0.15729638}
tc=$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin
out_root=${ENGINE_ANDROID_OUT:-$repo/artifacts/engine/rust-android}
mkdir -p "$repo/artifacts"
if [[ ${WORKFLOW_BUILD_LOCK_HELD:-0} != 1 ]]; then
  exec "$repo/tools/with-build-lock.sh" "$0" "$@"
fi
abis=("$@"); ((${#abis[@]})) || abis=(arm64-v8a x86_64)
for abi in "${abis[@]}"; do
  case "$abi" in
    arm64-v8a) target=aarch64-linux-android; cargo_target=AARCH64_LINUX_ANDROID; cc_target=aarch64_linux_android ;;
    x86_64) target=x86_64-linux-android; cargo_target=X86_64_LINUX_ANDROID; cc_target=x86_64_linux_android ;;
    *) echo "unsupported ABI $abi" >&2; exit 2 ;;
  esac
  cc="$tc/${target}28-clang"
  out="$out_root/$abi"
  mkdir -p "$out"
  env "CARGO_TARGET_${cargo_target}_LINKER=$cc" "CC_${cc_target}=$cc" \
      "AR_${cc_target}=$tc/llvm-ar" CARGO_BUILD_JOBS=2 \
      RUSTFLAGS='-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,noexecstack' \
      cargo build --manifest-path "$here/../../Cargo.toml" --release --target "$target" -p workflow-runtime
  cp "$here/../../target/$target/release/workflow-runtime" "$out/libworkflow-runtime.so"
  "$here/../loader/build.sh" "$target" "$out/libworkflow-loader.so"
  printf '%s: %s\n' "$abi" "$out"
done
