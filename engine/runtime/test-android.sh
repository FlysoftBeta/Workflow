#!/usr/bin/env bash
# Run the frozen native acceptance cases using Rust binaries in an app sandbox.
# The harness retains its original filename, so only its staged test copy is aliased.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../.." && pwd)
cd "$repo"
abi=${ENGINE_TEST_ABI:-x86_64}
case "$abi" in x86_64) triple=x86_64-linux-android28 ;; arm64-v8a) triple=aarch64-linux-android28 ;; *) exit 2 ;; esac
export ENGINE_ANDROID_OUT=${ENGINE_ANDROID_OUT:-$repo/artifacts/engine/rust-android}
"$here/build-android.sh" "$abi"
stage=$repo/artifacts/engine/rust-harness
mkdir -p "$stage/$abi/test"
cp "$ENGINE_ANDROID_OUT/$abi/libworkflow-runtime.so" "$stage/$abi/libworkflow-engine.so"
cp "$ENGINE_ANDROID_OUT/$abi/libworkflow-loader.so" "$stage/$abi/libworkflow-loader.so"
ndk=${ANDROID_NDK:-$HOME/Android/Sdk/ndk/30.0.15729638}
cc=$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/clang
for helper in sigsys_helper fork_stress; do
  "$cc" --target="$triple" -std=c17 -O1 -fPIE -pie \
    "$repo/engine/runtime/tests/native/$helper.c" -o "$stage/$abi/test/$helper"
done
"$cc" --target="$triple" -std=c17 -O1 -static -nostdlib -ffreestanding -fno-stack-protector \
  -Wl,-z,max-page-size=4096 -Wl,-z,separate-code \
  "$repo/engine/runtime/tests/native/guest_static.c" -o "$stage/$abi/test/guest_static"
export ENGINE_TEST_OUTPUT=$stage ENGINE_BUILD=0
export WORKFLOW_EMULATOR_PORT=${WORKFLOW_EMULATOR_PORT:-5830}
"$repo/tools/with-emulator.sh" bash "$here/test-app-sandbox.sh" "$@"
