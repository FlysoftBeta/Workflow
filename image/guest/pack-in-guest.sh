#!/bin/bash
# Pack the running, provisioned guest into a workspace image from inside the guest. This is the
# arm64 build path (run in the Workflow engine on the device after provision.sh, see
# docs/engine/environment.md §7); image/tests/podman-roundtrip.sh exercises it under podman.
# Usage, as (virtual) root: pack-in-guest.sh OUT_DIR [BUILDER]   (OUT_DIR under /tmp or /workspace)
# Environment: LEVEL (zstd level, default 19), WINDOW_LOG (default 27; the app packs with 3 and 23 on the
# device to bound compressor memory).
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
out=${1:?usage: pack-in-guest.sh OUT_DIR [BUILDER]}
builder=${2:-engine}
[ "$(id -u)" = 0 ] || { echo "pack-in-guest.sh must run as root" >&2; exit 2; }
case $out in
    /tmp/* | /workspace/*) ;;
    *) echo "OUT_DIR must be under /tmp or /workspace, which the image excludes" >&2; exit 2 ;;
esac
# shellcheck source=../versions.env
. "$here/../versions.env"
arch=$(dpkg --print-architecture)
export PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$here/.."
wfimage() { /opt/toolchains/active/python/bin/python3 -m wfimage "$@"; }
mkdir -p "$out"
work=$(mktemp -d /tmp/pack-in-guest.XXXXXX)
trap 'rm -rf "$work"' EXIT

runuser -u work -- /usr/local/libexec/workflow/envctl state > "$work/state.json"
wfimage metadata --versions "$here/../versions.env" --arch "$arch" --base-out "$work/base.json" \
    --state "$work/state.json" --provision "$here/provision.sh" --builder "$builder" --extra-out "$work/extra.json"
overrides=()
for placeholder in hosts hostname resolv.conf; do
    if [ -f "/etc/$placeholder" ]; then overrides+=(--override "/etc/$placeholder=$here/placeholders/$placeholder"); fi
done
# Capture last, right before packing, so nothing changes in between.
bash "$here/capture-manifest.sh" / "$here/prune.list" > "$work/manifest"
wfimage pack --rootfs / --manifest "$work/manifest" --out "$out/image.tar.zst" --arch "$arch" --profile workspace \
    --type "$IMAGE_TYPE" --type-version "$IMAGE_TYPE_VERSION" --base-json "$work/base.json" \
    --extra-json "$work/extra.json" --prune-file "$here/prune.list" --canonical-file "$here/canonical.list" \
    --level "${LEVEL:-19}" --window-log "${WINDOW_LOG:-27}" "${overrides[@]}"
wfimage verify "$out/image.tar.zst" --index "$out/image.json" --arch "$arch" --profile workspace
