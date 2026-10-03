#!/usr/bin/env bash
# Serialize heavy builds without leaking the lease into daemons or child processes.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
source "$repo/tools/lib/coordination.sh"
primary=$(workflow_primary_root "$repo")
mkdir -p "$primary/artifacts"
if (($# == 0)); then printf 'Usage: tools/with-build-lock.sh COMMAND [ARG...]\n' >&2; exit 2; fi
if [[ ${WORKFLOW_BUILD_LOCK_HELD:-0} == 1 ]]; then exec "$@"; fi
exec flock --close "$primary/artifacts/.gradle.lock" env WORKFLOW_BUILD_LOCK_HELD=1 WORKFLOW_COORDINATION_ROOT="$primary" "$@"
