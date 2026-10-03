#!/usr/bin/env bash
# Own one low-memory, disposable emulator for the duration of a command. Serial across all workstreams.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$repo"
port=${WORKFLOW_EMULATOR_PORT:-5860}
avd=${WORKFLOW_AVD:-workflow-tablet-api28}
sdk=${ANDROID_HOME:-${ANDROID_SDK_ROOT:-"$HOME/Android/Sdk"}}
mkdir -p artifacts/emulator-tmp
# A daemon must never inherit the device lease. Start adb before opening the lock.
adb start-server 9>&- >/dev/null
exec 9> artifacts/.device.lock
flock 9
if adb -s "emulator-$port" get-state 9>&- >/dev/null 2>&1; then
  printf 'Port %s is already in use; refusing to take over an emulator.\n' "$port" >&2
  exit 1
fi
temporary=$(mktemp -d "$repo/artifacts/emulator-tmp/run.XXXXXX")
export TMPDIR="$temporary"
# The emulator uses ANDROID_TMP for read-only qcow overlays; TMPDIR alone is ignored for those.
export ANDROID_TMP="$temporary"
export ANDROID_AVD_HOME="$repo/artifacts/avd"
export ANDROID_SERIAL="emulator-$port"
"$sdk/emulator/emulator" -avd "$avd" -port "$port" -read-only -no-snapshot \
  -no-window -no-audio -no-boot-anim -gpu swiftshader_indirect -memory 1536 -cores 2 \
  9>&- > "$temporary/emulator.log" 2>&1 &
emulator_pid=$!
finish() {
  timeout 10 adb -s "$ANDROID_SERIAL" emu kill 9>&- >/dev/null 2>&1 || true
  kill "$emulator_pid" 2>/dev/null || true
  for ((attempt=0; attempt<10; attempt++)); do
    kill -0 "$emulator_pid" 2>/dev/null || break
    sleep 0.5
  done
  kill -KILL "$emulator_pid" 2>/dev/null || true
  wait "$emulator_pid" 2>/dev/null || true
  # Keep diagnostics; never leave large read-only overlays in tmpfs or on disk.
  find "$temporary" -type f -name '*.qcow2' -delete
}
trap finish EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP
for ((attempt=0; attempt<150; attempt++)); do
  if [[ $(adb -s "$ANDROID_SERIAL" shell getprop sys.boot_completed 9>&- 2>/dev/null | tr -d '\r') == 1 ]]; then
    "$@" 9>&-
    exit $?
  fi
  kill -0 "$emulator_pid" 2>/dev/null || { printf 'Emulator exited; see %s/emulator.log\n' "$temporary" >&2; exit 1; }
  sleep 2
done
printf 'Emulator did not boot; see %s/emulator.log\n' "$temporary" >&2
exit 1
