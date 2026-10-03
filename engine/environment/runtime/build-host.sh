#!/usr/bin/env bash
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../.." && pwd)
out=${1:-$repo/artifacts/engine/rust-host}
mkdir -p "$out" "$repo/artifacts"
if [[ ${WORKFLOW_BUILD_LOCK_HELD:-0} != 1 ]]; then
  exec "$repo/tools/with-build-lock.sh" "$0" "$out"
fi
profile=${ENGINE_RUST_PROFILE:-release}
case "$profile" in release) cargo_args=(--release) ;; debug) cargo_args=() ;; *) echo "invalid profile $profile" >&2; exit 2 ;; esac
CARGO_BUILD_JOBS=2 cargo build --manifest-path "$here/../../Cargo.toml" "${cargo_args[@]}" -p workflow-runtime
cp "$here/../../target/$profile/workflow-runtime" "$out/engine"
"$here/../loader/build.sh" x86_64-unknown-linux-gnu "$out/loader"
