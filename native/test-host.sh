#!/usr/bin/env bash
# Build and run the native host tests with the system C compiler (no NDK, device or root needed).
#   native/test-host.sh              # all suites
#   native/test-host.sh pty          # only the PTY suite
#   native/test-host.sh proxy-guard  # only the guardian suites (stat parser + supervisor harness)
# Build output goes to $NATIVE_HOST_BUILD_DIR (default: artifacts/native-host, git-ignored).
set -euo pipefail
native_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_directory=$(cd -- "$native_directory/.." && pwd)
build_directory=${NATIVE_HOST_BUILD_DIR:-"$repo_directory/artifacts/native-host"}
cc=${CC:-cc}
suites=("$@")
((${#suites[@]})) || suites=(pty proxy-guard)

run_pty() {
  local out="$build_directory/pty"
  mkdir -p -- "$out"
  "$cc" -std=c11 -Wall -Wextra -Werror -pthread \
    -I "$native_directory/pty" \
    "$native_directory/pty/pty_process.c" "$native_directory/pty/tests/pty_process_test.c" \
    -o "$out/pty-process-test"
  "$out/pty-process-test"
}

run_proxy_guard() {
  local out="$build_directory/proxy-guard" source="$native_directory/proxy-guard/proxy_guard.c"
  local tests="$native_directory/proxy-guard/tests"
  mkdir -p -- "$out"
  "$cc" -std=c11 -O2 -Wall -Wextra -Werror -DWORKFLOW_PROXY_GUARD_HOST_TEST "$source" -o "$out/guard-test"
  "$cc" -std=c11 -O2 -Wall -Wextra -Werror "$source" -o "$out/guard-production"
  "$cc" -std=c11 -O2 -Wall -Wextra -Werror "$tests/proxy_guard_fake_kernel.c" -o "$out/fake-kernel"
  "$cc" -std=c11 -O2 -Wall -Wextra -Werror -DWORKFLOW_PROXY_GUARD_HOST_TEST \
    "$tests/proxy_guard_parser_test.c" -o "$out/parser-test"
  "$out/parser-test"
  python3 "$tests/proxy_guard_test.py" "$out"
}

for suite in "${suites[@]}"; do
  case "$suite" in
    pty) printf '== native/pty host tests\n'; run_pty ;;
    proxy-guard) printf '== native/proxy-guard host tests\n'; run_proxy_guard ;;
    *) printf 'Unknown suite: %s (expected pty or proxy-guard)\n' "$suite" >&2; exit 2 ;;
  esac
done
printf '== native host tests passed: %s\n' "${suites[*]}"
