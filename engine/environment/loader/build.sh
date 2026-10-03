#!/usr/bin/env bash
# Caller holds artifacts/.gradle.lock when running as part of a heavy build.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
target=${1:-x86_64-unknown-linux-gnu}
output=${2:?output path required}
mkdir -p "$(dirname "$output")"
linker=cc
if [[ "$target" == *-linux-android ]]; then
  ndk=${ANDROID_NDK:-$HOME/Android/Sdk/ndk/30.0.15729638}
  linker="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/${target}28-clang"
fi
rustc --edition=2024 --target "$target" -C opt-level=2 -C panic=abort \
  -C lto=fat -C codegen-units=1 -C relocation-model=pie -C linker="$linker" -C link-arg=-nostdlib \
  -C link-arg=-static-pie -C link-arg=-Wl,-z,max-page-size=16384 \
  -C link-arg=-Wl,-z,noexecstack -C link-arg=-Wl,--no-dynamic-linker \
  "$here/src/main.rs" -o "$output"
if readelf -r "$output" | grep -q 'R_'; then
  echo "loader must have no runtime relocations: $output" >&2; exit 1
fi
if readelf -l "$output" | grep -q INTERP; then
  echo "loader must not require a dynamic interpreter: $output" >&2; exit 1
fi
