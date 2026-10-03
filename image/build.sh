#!/bin/bash
# Build a workspace image using podman, native or isolated build-only QEMU emulation.
# Debian base conversion remains available only for build and runtime test fixtures.
# Format and paths: docs/environment.md §2 and §7.
#
#   image/build.sh [--profile workspace|base] [--arch amd64|arm64] [--out DIR] [--level N] [--keep]
#
# Output: DIR/{image,base}.tar.zst, .json (index), .tar.zst.sha256, .build.log, .packages.tsv
set -euo pipefail
image_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
project=$(dirname "$image_dir")
# shellcheck source=versions.env
. "$image_dir/versions.env"

original_args=("$@")
profile=workspace arch='' out='' level=19 keep=0
while [ $# -gt 0 ]; do
    case $1 in
        --profile) profile=$2; shift 2 ;;
        --arch) arch=$2; shift 2 ;;
        --out) out=$2; shift 2 ;;
        --level) level=$2; shift 2 ;;
        --keep) keep=1; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
case $(uname -m) in x86_64) host_arch=amd64 ;; aarch64) host_arch=arm64 ;; *) host_arch=unknown ;; esac
arch=${arch:-$host_arch}
case $arch in amd64) platform=linux/amd64 ;; arm64) platform=linux/arm64/v8 ;; *) echo "arch must be amd64 or arm64" >&2; exit 2 ;; esac
case $profile in workspace) name=image ;; base) name=base ;; *) echo "profile must be workspace or base" >&2; exit 2 ;; esac
if [ "$profile" = workspace ] && [ "$arch" != "$host_arch" ] && [ "${WORKFLOW_IMAGE_CROSS:-0}" != 1 ]; then
    exec "$image_dir/with-cross.sh" "$arch" env WORKFLOW_IMAGE_CROSS=1 "$0" "${original_args[@]}"
fi
for tool in podman python3 zstd sha256sum; do
    command -v "$tool" >/dev/null || { echo "missing tool: $tool" >&2; exit 2; }
done

digest_var=DEBIAN_DIGEST_${arch^^}
digest=${!digest_var}
reference="${DEBIAN_REFERENCE%:*}@$digest"
out=${out:-$project/artifacts/image/$arch}
work=$project/artifacts/image/work/$arch-$profile
container=wf-image-$arch-$profile-$$
mkdir -p "$out"
rm -rf "$work"
mkdir -p "$work"
log=$out/$name.build.log
: > "$log"
cleanup() {
    podman rm -f "$container" >/dev/null 2>&1 || true
    [ "$keep" = 1 ] || rm -rf "$work"
}
trap cleanup EXIT
wfimage() { PYTHONPATH=$image_dir python3 -m wfimage "$@"; }
say() { printf '[build %s] %s\n' "$(date -u +%H:%M:%S)" "$*" | tee -a "$log" >&2; }

say "pull $reference ($platform)"
podman pull -q --platform "$platform" "$reference" >>"$log"
pulled=$(podman image inspect --format '{{.Digest}}' "$reference")
[ "$pulled" = "$digest" ] || { say "digest mismatch: $pulled"; exit 1; }

if [ "$profile" = workspace ]; then
    podman create --name "$container" --platform "$platform" "$reference" sleep infinity >>"$log"
    podman start "$container" >>"$log"
    podman exec "$container" mkdir -p /tmp/workflow-image
    podman cp "$image_dir/guest" "$container:/tmp/workflow-image/guest"
    podman cp "$image_dir/versions.env" "$container:/tmp/workflow-image/versions.env"
    say "provision (first run)"
    start=$(date +%s)
    podman exec "$container" bash /tmp/workflow-image/guest/provision.sh >>"$log" 2>&1
    say "provision took $(( $(date +%s) - start ))s"
    podman exec -u work "$container" /usr/local/libexec/workflow/envctl state $PACKAGES > "$work/state.1.json"
    say "provision (second run, must be a no-op)"
    start=$(date +%s)
    podman exec "$container" bash /tmp/workflow-image/guest/provision.sh >>"$log" 2>&1
    say "second run took $(( $(date +%s) - start ))s"
    podman exec -u work "$container" /usr/local/libexec/workflow/envctl state $PACKAGES > "$work/state.2.json"
    cmp -s "$work/state.1.json" "$work/state.2.json" || { say "second provision changed state"; exit 1; }
    podman exec "$container" dpkg-query -W -f '${Package}\t${Version}\t${Architecture}\n' > "$out/$name.packages.tsv"
    cp "$work/state.2.json" "$out/$name.state.json"
    say "capture manifest in the guest"
    podman exec "$container" bash /tmp/workflow-image/guest/capture-manifest.sh / \
        /tmp/workflow-image/guest/prune.list > "$work/manifest.capture"
    podman stop -t 0 "$container" >>"$log"
else
    podman create --name "$container" --platform "$platform" "$reference" /bin/true >>"$log"
fi

say "export and convert"
podman export -o "$work/rootfs.tar" "$container"
wfimage tree-from-tar "$work/rootfs.tar" "$work/tree" 2>>"$log" >>"$log"
rm -f "$work/rootfs.tar"
manifest=$work/tree/manifest
if [ "$profile" = workspace ]; then
    say "cross-check guest manifest against the export"
    if ! wfimage compare "$work/tree/manifest" "$work/manifest.capture" --prune-file "$image_dir/guest/prune.list" \
        --canonical-file "$image_dir/guest/canonical.list" > "$work/compare.txt"; then
        cat "$work/compare.txt" | tee -a "$log" >&2
        say "manifests differ"
        exit 1
    fi
    manifest=$work/manifest.capture
fi

overrides=()
for placeholder in hosts hostname resolv.conf; do
    # grep without -q: an early exit would SIGPIPE tr and fail the pipeline under pipefail.
    if tr '\0' '\n' < "$manifest" | grep -E "^8[0-9a-f]{3} .* \./etc/$placeholder\$" >/dev/null; then
        overrides+=(--override "/etc/$placeholder=$image_dir/guest/placeholders/$placeholder")
    fi
done

metadata_args=(--versions "$image_dir/versions.env" --arch "$arch" --base-out "$work/base.json")
extra=()
if [ "$profile" = workspace ]; then
    metadata_args+=(--state "$work/state.1.json" --provision "$image_dir/guest/provision.sh" --builder podman
                    --extra-out "$work/extra.json")
    extra=(--extra-json "$work/extra.json")
fi
wfimage metadata "${metadata_args[@]}"

say "pack (zstd -$level)"
start=$(date +%s)
flock "$project/artifacts/.gradle.lock" env PYTHONPATH="$image_dir" python3 -m wfimage pack --rootfs "$work/tree/rootfs" --manifest "$manifest" --out "$out/$name.tar.zst" --arch "$arch" \
    --profile "$profile" --type "$IMAGE_TYPE" --type-version "$IMAGE_TYPE_VERSION" --base-json "$work/base.json" \
    --prune-file "$image_dir/guest/prune.list" --canonical-file "$image_dir/guest/canonical.list" --level "$level" "${overrides[@]}" "${extra[@]}" 2>>"$log" | tee -a "$log"
say "pack took $(( $(date +%s) - start ))s"
say "verify"
wfimage verify "$out/$name.tar.zst" --index "$out/$name.json" --arch "$arch" --profile "$profile" | tee -a "$log"
(cd "$out" && sha256sum "$name.tar.zst" > "$name.tar.zst.sha256")
say "done: $out/$name.tar.zst ($(stat -c %s "$out/$name.tar.zst") bytes)"
