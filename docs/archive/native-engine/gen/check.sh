#!/usr/bin/env bash
# Regenerate the engine syscall inventory and run its checks.  No device, no Gradle.
#
#   native/engine/gen/check.sh
#
# 1. verifies the pinned kernel tables (sha256), validates syscalls.tsv, generates twice into
#    scratch and requires identical output (determinism), cross-checks NDK r30 + host headers;
# 2. compiles the scratch sysinv.c with the engine's host flags and NDK clang (aarch64, x86_64)
#    before installing it, so the tree never holds a sysinv.c that breaks test-host.sh;
# 3. installs include/engine/sysinv.h, src/sysinv.c, gen/coverage.md (reports what changed);
# 4. builds test/sysinv_test.c natively (x86_64 host) and with ENG_SYSINV_FORCE_ABI=1/2, runs
#    the checks, and compares each --dump with the kernel tables in python;
# 5. compiles + links the test for aarch64-linux-android28 (not run: no arm64 host).
# Env: CC (host compiler, default cc), ANDROID_NDK_ROOT (default ~/Android/Sdk/ndk/30.0.15729638),
#      SYSINV_BUILD (default <repo>/artifacts/engine/sysinv-check).
set -euo pipefail

gen_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
eng_dir=$(cd -- "$gen_dir/.." && pwd)
repo_dir=$(cd -- "$eng_dir/../.." && pwd)
build=${SYSINV_BUILD:-"$repo_dir/artifacts/engine/sysinv-check"}
ndk=${ANDROID_NDK_ROOT:-"$HOME/Android/Sdk/ndk/30.0.15729638"}
cc=${CC:-cc}
ndkcc="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/clang"
gen=(python3 "$gen_dir/gensysinv.py")

# Same flags as native/engine/test-host.sh, plus -Wpedantic for the generated file.
CFLAGS=(-std=c17 -O1 -g -Wall -Wextra -Werror -Wno-unused-parameter -Wno-format-truncation -fPIE)

[ -x "$ndkcc" ] || { echo "NDK clang not found at $ndkcc (set ANDROID_NDK_ROOT)" >&2; exit 1; }
rm -rf -- "$build"
mkdir -p -- "$build"

echo "== generate (twice, into scratch) + header cross-check"
"${gen[@]}" --out "$build/a" --check-headers
"${gen[@]}" --out "$build/b" > /dev/null
diff -r "$build/a" "$build/b" || { echo "generation is not deterministic" >&2; exit 1; }
echo "deterministic: yes"

echo "== compile generated sysinv.c before installing it"
"$cc" "${CFLAGS[@]}" -Wpedantic -I "$build/a/include" -c "$build/a/src/sysinv.c" -o "$build/pre.o"
for t in aarch64-linux-android28 x86_64-linux-android28; do
  "$ndkcc" --target="$t" "${CFLAGS[@]}" -Wpedantic -I "$build/a/include" \
    -c "$build/a/src/sysinv.c" -o "$build/pre-$t.o"
done

echo "== install"
for f in include/engine/sysinv.h src/sysinv.c gen/coverage.md; do
  if cmp -s "$build/a/$f" "$eng_dir/$f"; then echo "unchanged $f"
  else cp -f -- "$build/a/$f" "$eng_dir/$f"; echo "UPDATED   $f"; fi
done

pass=0
run_variant() {  # <label> <expected abi> [extra cflags...]
  local label=$1 abi=$2; shift 2
  local exe="$build/sysinv_test-$label"
  "$cc" "${CFLAGS[@]}" -Wpedantic "$@" -I "$eng_dir/include" \
    "$eng_dir/src/sysinv.c" "$eng_dir/test/sysinv_test.c" -pie -o "$exe"
  "$exe"
  "$exe" --dump > "$exe.dump"
  "${gen[@]}" --verify-dump "$abi" "$exe.dump"
  pass=$((pass + 1))
}

echo "== host tests ($("$cc" -dumpmachine))"
case "$("$cc" -dumpmachine)" in
  x86_64*)  native=x86_64 ;;
  aarch64*) native=aarch64 ;;
  *) echo "unsupported host" >&2; exit 1 ;;
esac
run_variant native "$native"
san=(-fsanitize=address,undefined -fno-sanitize-recover=all)
if echo 'int main(void){return 0;}' | "$cc" "${san[@]}" -x c - -o "$build/san-probe" 2>/dev/null; then
  run_variant native-asan "$native" "${san[@]}"
else
  echo "(ASan/UBSan runtime not available: sanitizer variant skipped)"
fi
run_variant force-x86_64 x86_64 -DENG_SYSINV_FORCE_ABI=1
run_variant force-aarch64 aarch64 -DENG_SYSINV_FORCE_ABI=2

echo "== NDK clang aarch64-linux-android28 (compile + link; not run)"
"$ndkcc" --target=aarch64-linux-android28 "${CFLAGS[@]}" -Wpedantic -I "$eng_dir/include" \
  "$eng_dir/src/sysinv.c" "$eng_dir/test/sysinv_test.c" -pie -o "$build/sysinv_test-android-aarch64"
file -b "$build/sysinv_test-android-aarch64" 2>/dev/null || true

echo "== sysinv check: OK ($pass host variants; generated files in sync)"
