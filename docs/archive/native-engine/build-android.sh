#!/usr/bin/env bash
# Cross-build the engine for Android (NDK, API 28 sysroot, 16 KiB max page size).
#   native/engine/build-android.sh [arm64-v8a|x86_64 ...]
# Output: $ENGINE_ANDROID_OUT/<abi>/ (default artifacts/engine/android):
#   libworkflow-engine.so   tracer/CLI (PIE executable, bionic)
#   libworkflow-loader.so   freestanding guest loader (static-pie, no libc, no relocations)
#   test/sigsys_helper, test/fork_stress, test/guest_static   device test helpers
set -euo pipefail
eng_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_dir=$(cd -- "$eng_dir/../.." && pwd)
ndk=${ANDROID_NDK:-$HOME/Android/Sdk/ndk/30.0.15729638}
tc=$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin
out_root=${ENGINE_ANDROID_OUT:-$repo_dir/artifacts/engine/android}
api=28
abis=("$@"); ((${#abis[@]})) || abis=(arm64-v8a x86_64)

for abi in "${abis[@]}"; do
  case "$abi" in
    arm64-v8a) triple=aarch64-linux-android ;;
    x86_64) triple=x86_64-linux-android ;;
    *) echo "unsupported ABI $abi (guest arch == host arch; 64-bit only)" >&2; exit 2 ;;
  esac
  cc="$tc/clang --target=${triple}${api}"
  out="$out_root/$abi"
  mkdir -p "$out/test"
  common=(-std=c17 -Wall -Wextra -Werror -Wno-unused-parameter -I "$eng_dir/include")
  $cc -std=c17 -O2 -fPIE -c "$eng_dir/third_party/zstd-1.5.7/zstddeclib.c" -o "$out/zstddeclib.o"
  $cc "${common[@]}" -O2 -fPIE -pie -Wl,-z,max-page-size=16384 -Wl,-z,noexecstack \
      "$eng_dir"/src/*.c "$out/zstddeclib.o" -o "$out/libworkflow-engine.so"
  rm -f "$out/zstddeclib.o"
  $cc "${common[@]}" -O2 -fPIE -ffreestanding -fno-builtin -fno-stack-protector -fno-jump-tables \
      -fno-asynchronous-unwind-tables -nostdlib -static-pie -Wl,-z,max-page-size=16384 \
      -Wl,-z,noexecstack "$eng_dir/loader/loader.c" -o "$out/libworkflow-loader.so"
  if "$tc/llvm-readelf" -r "$out/libworkflow-loader.so" | grep -q 'R_'; then
    echo "$abi: loader has relocations" >&2; exit 1
  fi
  $cc -std=c17 -O1 -fPIE -pie "$eng_dir/test/sigsys_helper.c" -o "$out/test/sigsys_helper"
  $cc -std=c17 -O1 -fPIE -pie "$eng_dir/test/fork_stress.c" -o "$out/test/fork_stress"
  # 4 KiB link page on purpose: on a 16 KiB kernel its segments share pages (loader test)
  $cc -std=c17 -O1 -static -nostdlib -ffreestanding -fno-stack-protector \
      -Wl,-z,max-page-size=4096 -Wl,-z,separate-code \
      "$eng_dir/test/guest_static.c" -o "$out/test/guest_static"
  printf '%s: %s\n' "$abi" "$(cd "$out" && ls -1 libworkflow-*.so test/* | tr '\n' ' ')"
done
