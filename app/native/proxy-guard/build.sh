#!/usr/bin/env bash
set -euo pipefail
script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_directory=$(cd -- "$script_directory/../../.." && pwd)
output_directory="$repo_directory/artifacts/local-runtime/proxy-guard"
ndk_directory="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-${ANDROID_HOME:-$HOME/Android/Sdk}/ndk/30.0.15729638}}"
while (($#)); do
  case "$1" in
    --output-dir) output_directory="${2:?--output-dir needs a directory}"; shift 2 ;;
    --ndk) ndk_directory="${2:?--ndk needs a directory}"; shift 2 ;;
    *) printf 'Unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
done
toolchain_directory="$ndk_directory/toolchains/llvm/prebuilt/linux-x86_64/bin"
for abi in arm64-v8a x86_64; do
  case "$abi" in
    arm64-v8a) target=aarch64-linux-android28 ;;
    x86_64) target=x86_64-linux-android28 ;;
  esac
  mkdir -p -- "$output_directory/$abi"
  "$toolchain_directory/$target-clang" -std=c11 -O2 -Wall -Wextra -Werror \
    -fPIE -pie -fstack-protector-strong -D_FORTIFY_SOURCE=2 \
    -Wl,-z,relro,-z,now,-z,max-page-size=16384 \
    "$script_directory/proxy_guard.c" \
    -o "$output_directory/$abi/libworkflow_proxy_guard.so"
  "$toolchain_directory/llvm-strip" --strip-unneeded "$output_directory/$abi/libworkflow_proxy_guard.so"
done
