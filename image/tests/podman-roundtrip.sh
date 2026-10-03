#!/bin/bash
# End-to-end check of a customized workspace image; arm64 on x86_64 uses with-cross.sh.
#   1. rebuild a conventional tar from the image (ownership from attributes.tsv) and import it;
#   2. smoke-test the environment as `work` (sudo, toolchains, locale, compiler);
#   3. run the envctl reconcile steps: packages, Python/Node switches, idempotency;
#   4. pack the imported guest again from inside (the in-engine arm64 path) and require the same
#      attributes table as the original image.
# Usage: image/tests/podman-roundtrip.sh [IMAGE.tar.zst] [--skip-repack]
# --skip-repack omits only compression inside an emulated guest; host image verification still applies.
set -euo pipefail
image_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
project=$(dirname "$image_dir")
image=${1:-$project/artifacts/image/amd64/image.tar.zst}
index=${image%.tar.zst}.json
repack_enabled=1
if [ "${2:-}" = --skip-repack ]; then repack_enabled=0; elif [ $# -gt 1 ]; then echo "invalid option: $2" >&2; exit 2; fi
arch=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["metadata"]["architecture"])' "$index")
work=$project/artifacts/image/work/roundtrip-$arch
tag=localhost/workflow-roundtrip:$arch
steps=wf-roundtrip-steps-$$
repack=wf-roundtrip-pack-$$
failures=0
rm -rf "$work"
mkdir -p "$work"
cleanup() {
    podman rm -f "$steps" "$repack" >/dev/null 2>&1 || true
    podman rmi -f "$tag" >/dev/null 2>&1 || true
}
trap cleanup EXIT
wfimage() { PYTHONPATH=$image_dir python3 -m wfimage "$@"; }
check() {
    local name=$1
    shift
    if "$@"; then echo "ok    $name"; else echo "FAIL  $name"; failures=$((failures + 1)); fi
}
field() { grep -m1 "^$1=" "$work/smoke.txt" | cut -d= -f2-; }

echo "== import"
wfimage to-tar "$image" "$work/rootfs.tar" --index "$index" >/dev/null
podman import -q --arch "$arch" "$work/rootfs.tar" "$tag" >/dev/null
rm -f "$work/rootfs.tar"
python=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["metadata"]["toolchains"]["python"])' "$index")
node=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["metadata"]["toolchains"]["node"])' "$index")

echo "== smoke (python $python, node $node)"
podman run --rm -u work "$tag" bash -lc '
    echo "machine=$(uname -m)"
    echo "debian_arch=$(dpkg --print-architecture)"
    echo "id=$(id -u):$(id -g):$(id -un):$HOME"
    echo "root=$(sudo -n su -c "id -u")"
    echo "python=$(python3 --version)"
    echo "node=$(node --version)"
    echo "npm=$(npm --version)"
    echo "uv=$(uv --version)"
    echo "nvm=$(nvm --version)"
    echo "active=$(readlink /opt/toolchains/active)"
    echo "sudo_mode=$(stat -c %a:%U /usr/bin/sudo)"
    echo "locale_warnings=$(LC_ALL=en_US.UTF-8 locale 2>&1 >/dev/null | wc -l)"
    echo "sudo_node=$(sudo -n node --version)"
    printf "int main(void){return 7;}\n" > /tmp/t.c && gcc /tmp/t.c -o /tmp/t && { /tmp/t; echo "gcc_exit=$?"; }
    echo "cmake=$(cmake --version | head -1)"
    echo "venv=$(cd /tmp && uv venv -q v && v/bin/python -c "import sys; print(sys.version.split()[0])")"
    echo "git=$(git --version)"
' > "$work/smoke.txt" 2>&1 || true
cat "$work/smoke.txt"
check "guest Debian architecture" test "$(field debian_arch)" = "$arch"
case $arch in amd64) expected_machine=x86_64 ;; arm64) expected_machine=aarch64 ;; esac
check "guest machine architecture" test "$(field machine)" = "$expected_machine"
check "user work 1000:1000" test "$(field id)" = "1000:1000:work:/home/work"
check "passwordless sudo su" test "$(field root)" = 0
check "python3 is the image default" test "$(field python)" = "Python $python"
check "node is the image default" test "$(field node)" = "v$node"
check "npm present" test -n "$(field npm)"
check "nvm loads lazily" test "$(field nvm)" = "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["metadata"]["toolchains"]["nvm"])' "$index")"
check "active profile seeded" test "$(field active)" = "profiles/py$python-node$node"
check "setuid sudo preserved" test "$(field sudo_mode)" = "4755:root"
check "en_US.UTF-8 locale" test "$(field locale_warnings)" = 0
check "sudo keeps node on PATH" test "$(field sudo_node)" = "v$node"
check "gcc builds and runs" test "$(field gcc_exit)" = 7
check "uv venv uses the default python" test "$(field venv)" = "$python"

echo "== envctl reconcile steps"
podman run -d --name "$steps" "$tag" sleep infinity >/dev/null
envctl() { podman exec "$@"; }
state() { podman exec -u work "$steps" /usr/local/libexec/workflow/envctl state "$@"; }
json() { python3 -c 'import json,sys; d=json.load(sys.stdin); print(eval(sys.argv[1], {}, {"d": d}))' "$1"; }
E=/usr/local/libexec/workflow/envctl
envctl "$steps" $E apt-install ripgrep >"$work/steps.log" 2>&1
check "apt-install ripgrep" test "$(state ripgrep | json 'd["packages"]["ripgrep"] is not None')" = True
check "ripgrep runs" podman exec "$steps" rg --version
check "apt-install is idempotent" bash -c "podman exec $steps $E apt-install ripgrep 2>&1 | grep -q 'already installed'"
envctl "$steps" $E apt-remove ripgrep git >>"$work/steps.log" 2>&1
check "apt-remove ripgrep, keep image package git" test "$(state ripgrep git | json '(d["packages"]["ripgrep"], d["packages"]["git"] is not None)')" = "(None, True)"
W() { podman exec -u work "$steps" "$@"; }
login() { podman exec -u work "$steps" bash -lc "$1"; }
check "python 3.13 installs side by side" test "$(W $E python 3.13 2>>"$work/steps.log" | json 'd["installed"]')" = True
check "python step is idempotent" test "$(W $E python 3.13 2>>"$work/steps.log" | json 'd["installed"]')" = False
check "node 22 installs side by side" test "$(W $E node 22 2>>"$work/steps.log" | json 'd["installed"]')" = True
profile=$(W $E verify 3.13 22 git 2>>"$work/steps.log" | json 'd["profile"]')
check "verify creates a profile" bash -c "[[ $profile == py3.13.*-node22.* ]]"
check "image profile untouched until activation" test "$(login 'python3 --version')" = "Python $python"
W $E activate "$profile"
check "activation switches python" bash -c "[[ \$(podman exec -u work $steps bash -lc 'python3 --version') == 'Python 3.13.'* ]]"
check "activation switches node" bash -c "[[ \$(podman exec -u work $steps bash -lc 'node --version') == v22.* ]]"
check "uv venv follows the active profile" bash -c "[[ \$(podman exec -u work $steps bash -lc 'cd /tmp && rm -rf v && uv venv -q v && v/bin/python --version') == 'Python 3.13.'* ]]"
check "project pin still wins" test "$(login "mkdir -p /tmp/p && cd /tmp/p && echo 3.14 > .python-version && rm -rf v && uv venv -q v && v/bin/python --version")" = "Python $python"
check "state reports the active profile" test "$(W $E state | json 'd["profile"]')" = "$profile"
W $E activate "py$python-node$node"
check "rollback is a switch back" test "$(login 'python3 --version; node --version' | tr '\n' ' ')" = "Python $python v$node "
check "previous versions stay installed" podman exec "$steps" test -d "/opt/toolchains/nvm/versions/node/v$node"
check "no store temp files left" bash -c "! podman exec $steps sh -c 'ls -A /opt/toolchains/.tmp 2>/dev/null | grep -q .'"
echo "   (step log: $work/steps.log)"

echo "== multi-toolchain profiles"
"$image_dir/tests/toolchain-profiles.sh" "$steps"

if [ "$repack_enabled" = 1 ]; then
echo "== pack from inside the guest"
podman run -d --name "$repack" "$tag" sleep infinity >/dev/null
podman exec "$repack" mkdir -p /tmp/workflow-image
for part in guest wfimage versions.env; do podman cp "$image_dir/$part" "$repack:/tmp/workflow-image/$part"; done
flock "$project/artifacts/.gradle.lock" podman exec "$repack" bash /tmp/workflow-image/guest/pack-in-guest.sh /tmp/out podman-roundtrip >"$work/repack.log" 2>&1 \
    || { tail -20 "$work/repack.log"; failures=$((failures + 1)); }
mkdir -p "$work/repacked"
podman cp "$repack:/tmp/out/image.tar.zst" "$work/repacked/" 2>/dev/null || true
podman cp "$repack:/tmp/out/image.json" "$work/repacked/" 2>/dev/null || true
if [ -f "$work/repacked/image.tar.zst" ]; then
    zstd -dcq "$image" | tar -xO attributes.tsv > "$work/original.tsv"
    zstd -dcq "$work/repacked/image.tar.zst" | tar -xO attributes.tsv > "$work/repacked.tsv"
    check "in-guest repack reproduces attributes.tsv" cmp -s "$work/original.tsv" "$work/repacked.tsv"
    check "in-guest repack reproduces metadata" python3 - "$index" "$work/repacked/image.json" <<'EOF'
import json, sys
a, b = (json.load(open(p))["metadata"] for p in sys.argv[1:])
for m in (a, b):
    m.pop("createdAt"); m["provision"].pop("builder")
sys.exit(0 if a == b else 1)
EOF
    cmp -s "$work/original.tsv" "$work/repacked.tsv" || diff "$work/original.tsv" "$work/repacked.tsv" | head -20
fi

else
    echo "== in-guest recompression explicitly skipped; image was packed and verified on the host"
fi

echo "== $failures failure(s)"
[ "$failures" = 0 ]
