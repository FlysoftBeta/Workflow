#!/usr/bin/env bash
# A source-bound client suite includes the real Rust peer, not an optional stale external binary.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if [[ ${WORKFLOW_BUILD_LOCK_HELD:-0} != 1 ]]; then
  exec "$repo/tools/with-build-lock.sh" "$0" "$@"
fi
cd -- "$repo"
cargo build --manifest-path engine/Cargo.toml --locked -p workflow-server --bin workflow-engine -j 2
export WORKFLOW_ENGINE="$repo/engine/target/debug/workflow-engine"
exec ./gradlew :app:client:test :app:proxy:test :app:android:testX86_64DebugUnitTest
