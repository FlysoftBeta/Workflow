#!/bin/bash
# Run a build command with AArch64 binfmt confined to a disposable rootless user/mount namespace.
# Requires Linux >=6.7 (per-user-namespace binfmt_misc). Never mounts/registers in the host namespace.
set -euo pipefail
image_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
project=$(dirname "$image_dir")
if [ "${1:-}" = --inside ]; then
    shift
    emulator=$1
    shift
    [ "$(id -u)" = 0 ] || { echo 'expected podman rootless user namespace' >&2; exit 2; }
    binfmt=$(mktemp -d "$project/artifacts/image/binfmt.XXXXXX")
    cleanup() { umount "$binfmt" 2>/dev/null || true; rmdir "$binfmt"; }
    trap cleanup EXIT
    mount -t binfmt_misc binfmt_misc "$binfmt"
    printf '%s%s%s\n' ':workflow-aarch64:M::\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\xb7\x00:\xff\xff\xff\xff\xff\xff\xff\x00\xff\xff\xff\xff\xff\xff\xff\xff\xfe\xff\xff\xff:' "$emulator" ':OCF' > "$binfmt/register"
    "$@"
    exit
fi
[ "$(uname -m)" = x86_64 ] && [ "${1:-}" = arm64 ] || { echo 'with-cross.sh supports amd64 host -> arm64 guest' >&2; exit 2; }
shift
manifest=$project/third_party/qemu-user/manifest.json
cache=$project/third_party/.cache/qemu-user
mkdir -p "$cache" "$project/artifacts/image"
readarray -t pin < <(python3 - "$manifest" <<'PY'
import json, sys
m = json.load(open(sys.argv[1]))
print(m['url']); print(m['sha256'])
PY
)
if ! [ -f "$cache/qemu-user.deb" ] || ! echo "${pin[1]}  $cache/qemu-user.deb" | sha256sum -c --status -; then
    curl -fsSL --retry 3 "${pin[0]}" -o "$cache/qemu-user.deb.download"
    echo "${pin[1]}  $cache/qemu-user.deb.download" | sha256sum -c --quiet -
    mv "$cache/qemu-user.deb.download" "$cache/qemu-user.deb"
fi
# Re-extract from the verified archive, never trust a separately cached executable.
mkdir -p "$cache/root"
dpkg-deb --fsys-tarfile "$cache/qemu-user.deb" | tar -x -C "$cache/root" ./usr/bin/qemu-aarch64 ./usr/share/doc/qemu-user/copyright
cmp "$cache/root/usr/share/doc/qemu-user/copyright" "$project/third_party/qemu-user/LICENSE"
exec podman unshare unshare --mount --propagation private "$0" --inside "$cache/root/usr/bin/qemu-aarch64" "$@"
