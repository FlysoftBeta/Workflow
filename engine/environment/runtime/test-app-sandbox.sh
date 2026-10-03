#!/usr/bin/env bash
# Invoked only inside tools/with-emulator.sh, which owns emulator/device lifecycle.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../.." && pwd)
serial=${ANDROID_SERIAL:?must run via tools/with-emulator.sh}
[[ "$serial" == emulator-* ]] || { echo 'emulator required' >&2; exit 2; }
RESULT_SUFFIX=-rust "$repo/engine/environment/runtime/tests/device/run.sh" "$serial" "$repo/engine/environment/runtime/tests/device/cases/default.json"
RESULT_SUFFIX=-rust-m5 WORKSPACE_IMAGE=1 HARNESS_BUILD=0 \
  "$repo/engine/environment/runtime/tests/device/run.sh" "$serial" "$repo/engine/environment/runtime/tests/device/cases/m5.json"
