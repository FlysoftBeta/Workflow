#!/usr/bin/env bash
# Host test suite for the container engine (Fedora x86_64; no NDK/device/root).
#
#   engine/environment/runtime/test-host.sh            # all suites
#   engine/environment/runtime/test-host.sh m0 m2      # selected suites
#   ENGINE_ORACLE=1 engine/environment/runtime/test-host.sh m2   # also re-run the podman oracle
#
# Inputs: a pristine Debian trixie amd64 rootfs (ENGINE_ROOTFS, default
# artifacts/engine/rootfs-amd64, pinned digest in docs/archive/implementation-1.0.0/reports/initial/w2-engine.md).
# Every suite works on a fresh reflink copy in $build/rootfs; the pristine tree
# is never modified.  Output: $ENGINE_HOST_BUILD (default artifacts/engine/host-build).
set -euo pipefail

eng_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_dir=$(cd -- "$eng_dir/../../.." && pwd)
build=${ENGINE_HOST_BUILD:-"$repo_dir/artifacts/engine/host-build"}
pristine=${ENGINE_ROOTFS:-"$repo_dir/artifacts/engine/rootfs-amd64"}
rootfs="$build/rootfs"
cc=${CC:-cc}
mkdir -p -- "$build"

pass=0; fail=0; failed=()
ok()  { printf '  ok   %s\n' "$1"; pass=$((pass+1)); }
bad() { printf '  FAIL %s\n' "$1"; fail=$((fail+1)); failed+=("$1"); }
check() { if [ "$2" = "$3" ]; then ok "$1"; else bad "$1 (got '$2' want '$3')"; fi; }

ENGINE() { "$build/engine" "$@"; }
GUEST()  { "$build/engine" run --root "$rootfs" "$@"; }
# assert_exit <want> <label> <engine args...>
assert_exit() {
  local want=$1 label=$2; shift 2
  set +e; "$build/engine" "$@" >/dev/null 2>>"$build/stderr.log"; local got=$?; set -e
  check "$label" "$got" "$want"
}

build_engine() {
  printf '== build (host, %s)\n' "$("$cc" -dumpmachine)"
  "$repo_dir/engine/environment/runtime/build-host.sh" "$build"
  "$cc" -std=c17 -O1 -Wall "$eng_dir/tests/native/sigsys_helper.c" -o "$build/sigsys_helper"
  "$cc" -std=c17 -O1 -Wall -pthread "$eng_dir/tests/native/fork_stress.c" -o "$build/fork_stress"
  "$cc" -std=c17 -O1 -Wall -static -I "$eng_dir/tests/native" "$eng_dir/tests/native/legacy_probe.c" -o "$build/legacy_probe"
  "$cc" -std=c17 -O1 -Wall -static "$eng_dir/tests/native/meta_probe.c" -o "$build/meta_probe"
  # 4 KiB link page + separate-code: R, RX, R, RW segments share one 16 KiB page
  "$cc" -std=c17 -O1 -static -no-pie -nostdlib -ffreestanding -fno-stack-protector \
    -Wl,-z,max-page-size=4096 -Wl,-z,separate-code "$eng_dir/tests/native/guest_static.c" -o "$build/guest_static"
  export WORKFLOW_ENGINE_LOADER="$build/loader"
}

fresh_rootfs() {
  [ -x "$pristine/bin/true" ] || { echo "no pristine rootfs at $pristine" >&2; return 1; }
  rm -rf -- "$rootfs"
  cp -a --reflink=auto -- "$pristine" "$rootfs"
  cp -f "$build/guest_static" "$rootfs/guest_static"
  cp -f "$eng_dir/tests/native/guest/paths.sh" "$rootfs/wf-paths.sh"
  cp -f "$build/legacy_probe" "$rootfs/legacy_probe"
}

suite_m0() {
  printf '== M0 tracer core (host pass-through)\n'
  : > "$build/stderr.log"
  assert_exit 0   "exit code 0"                       run -- /bin/true
  assert_exit 1   "exit code 1"                       run -- /bin/false
  assert_exit 42  "exit code 42"                      run -- /bin/sh -c 'exit 42'
  check "stdout passthrough" "$(ENGINE run -- /bin/echo hello 2>/dev/null)" "hello"
  assert_exit 139 "signalled exit -> 128+SIGSEGV"     run -- /bin/sh -c 'kill -SEGV $$'
  assert_exit 0   "fork/exec/vfork/thread stress"     run -- "$build/fork_stress"
  assert_exit 0   "SIGSYS(statx) -> ENOSYS emulation" run -- "$build/sigsys_helper"
  set +e; "$build/sigsys_helper" >/dev/null 2>&1; local raw=$?; set -e
  check "control: bare helper killed by SIGSYS" "$raw" "159"
  set +e
  WORKFLOW_ENGINE_CHECK_TOGGLE=1 ENGINE run -- "$build/fork_stress" >/dev/null 2>"$build/toggle.log"; local t=$?
  set -e
  check "entry/exit toggle agrees with PTRACE_GET_SYSCALL_INFO (fork/exec stress)" "$t" "0"
}

suite_m1() {
  printf '== M1 loader (separate freestanding loader, static guest)\n'
  fresh_rootfs
  assert_exit 0 "static no-libc guest: argv/env/auxv/pagesz/AT_RANDOM" run --root "$rootfs" -- /guest_static
  check "static guest stdout" "$(GUEST -- /guest_static 2>/dev/null)" "guest-static-ok"
  assert_exit 0 "static guest, forced anonymous-copy mapping" run --root "$rootfs" --no-filemap -- /guest_static
  assert_exit 0 "16 KiB page simulation: segments sharing pages keep bytes+prot" \
    run --root "$rootfs" --test-page-size 16384 -- /guest_static
  local e16; e16=$(GUEST --test-page-size 16384 -- /bin/true 2>&1 >/dev/null || true)
  case $e16 in *"libc.so.6: ELF load command address/offset not page-aligned"*)
    ok "16 KiB simulation: 4 KiB-linked glibc library refused by ld.so itself (platform limit)" ;;
    *) bad "16 KiB simulation: unexpected dynamic-guest result: $e16" ;; esac
  check "dynamic glibc /bin/echo" "$(GUEST -- /bin/echo hello 2>/dev/null)" "hello"
  check "dynamic glibc, anonymous-copy mapping" "$(GUEST --no-filemap -- /bin/echo anon 2>/dev/null)" "anon"
  local maps; maps=$(GUEST -- /bin/cat /proc/self/maps 2>/dev/null | awk '{print $6}' | grep -c "$rootfs/usr/lib/x86_64-linux-gnu/libc.so.6" || true)
  [ "$maps" -ge 1 ] && ok "guest maps Debian's own libc.so.6 from the rootfs" || bad "guest libc mapping ($maps)"
  assert_exit 127 "missing command -> 127"            run --root "$rootfs" -- /no/such/cmd
  assert_exit 126 "non-executable -> 126"             run --root "$rootfs" -- /etc/debian_version
}

suite_m2() {
  printf '== M2 path translation (engine vs real-kernel oracle)\n'
  fresh_rootfs
  : > "$build/stderr.log"
  local golden="$eng_dir/tests/native/guest/paths.golden" out="$build/paths.engine"
  set +e
  WORKFLOW_ENGINE_CHECK_TOGGLE=1 timeout 300 "$build/engine" run --root "$rootfs" --user root -- /bin/bash /wf-paths.sh e \
      > "$out" 2>"$build/paths.stderr"
  local rc=$?
  set -e
  check "paths.sh completes under the engine (toggle cross-check on)" "$rc" "0"
  if [ "${ENGINE_ORACLE:-0}" = 1 ] && command -v podman >/dev/null; then
    timeout 300 podman run --rm --network=none --security-opt label=disable \
      -v "$eng_dir/tests/native/guest/paths.sh:/wf-paths.sh:ro" --rootfs "$pristine:O" /bin/bash /wf-paths.sh e > "$build/paths.podman" 2>&1 || true
    if diff -q "$build/paths.podman" "$golden" >/dev/null; then ok "podman oracle matches stored golden"
    else bad "podman oracle drifted from $golden (diff $build/paths.podman)"; fi
  fi
  local n; n=$(wc -l < "$golden")
  if diff -u "$golden" "$out" > "$build/paths.diff"; then ok "all $n path-semantics cases identical to the real kernel"
  else bad "path semantics differ from the kernel oracle ($(grep -c '^[-+][a-z]' "$build/paths.diff") lines, see $build/paths.diff)"; fi
  check "pwd at guest /" "$(GUEST -- /bin/sh -c pwd 2>/dev/null)" "/"
  check "--cwd sets guest cwd" "$(GUEST --cwd /usr/share -- /bin/sh -c pwd 2>/dev/null)" "/usr/share"
  mkdir -p "$build/bindsrc/.hidden" && echo bound > "$build/bindsrc/f"
  check "bind mount visible" "$(GUEST --bind "$build/bindsrc:/mnt/b" -- /bin/cat /mnt/b/f 2>/dev/null)" "bound"
  check "bind .. goes to guest parent" "$(GUEST --bind "$build/bindsrc:/mnt/b" --cwd /mnt/b -- /bin/sh -c 'cd .. && pwd -P' 2>/dev/null)" "/mnt"
  check "hidden path absent from lookups" \
    "$(GUEST --bind "$build/bindsrc:/mnt/b" --hide /mnt/b/.hidden -- /bin/sh -c 'test -e /mnt/b/.hidden && echo seen || echo absent' 2>/dev/null)" "absent"
  # Environment's visible .workspace layout: agent homes and tools are masked below /workspace and
  # mounted at their own guest paths. Hides match resolved guest paths, and a directory reached
  # through two binds maps back through its longest host prefix, so the whole alias is masked.
  local ws="$build/visible-workspace"
  rm -rf -- "$ws"
  mkdir -p "$ws/.workspace/agents/codex" "$ws/.workspace/agents/tools/payload/x" "$ws/.workspace/state"
  echo secret > "$ws/.workspace/agents/codex/auth.json"
  echo cfg > "$ws/.workspace/agents/codex/config.toml"
  echo tool > "$ws/.workspace/agents/tools/payload/x/t"
  echo '{}' > "$ws/.workspace/env.json"
  local WS=(--bind "$ws/.workspace/agents/codex:/home/work/.codex" --bind "$ws:/workspace"
            --bind "$ws/.workspace/agents/tools/payload/x:/opt/workflow/tools"
            --hide /workspace/.workspace/state --hide /workspace/.workspace/agents)
  check "agent credential readable at its guest home" "$(GUEST "${WS[@]}" -- /bin/cat /home/work/.codex/auth.json 2>/dev/null)" "secret"
  check "agent configuration editable at its guest home" \
    "$(GUEST "${WS[@]}" -- /bin/sh -c 'echo edited >> /home/work/.codex/config.toml && tail -n1 /home/work/.codex/config.toml' 2>/dev/null)" "edited"
  check "agent home alias masked below /workspace" \
    "$(GUEST "${WS[@]}" -- /bin/sh -c 'cd /workspace/.workspace/agents/codex 2>/dev/null && echo seen || echo absent' 2>/dev/null)" "absent"
  check "workspace walks never reach agent credentials" \
    "$(GUEST "${WS[@]}" -- /bin/sh -c 'grep -rl secret /workspace; cd /workspace && tar -cf - . 2>/dev/null | tar -tf - | grep -c auth' 2>/dev/null)" "0"
  check "only configuration remains in the guest .workspace listing" \
    "$(GUEST "${WS[@]}" -- /bin/ls -A /workspace/.workspace 2>/dev/null | tr '\n' ' ')" "env.json "
  check "masked tools remain mounted at their guest path" "$(GUEST "${WS[@]}" -- /bin/cat /opt/workflow/tools/t 2>/dev/null)" "tool"
}

suite_legacy() {
  printf '== legacy x86_64 syscalls behind the Android app filter (API 28 set)\n'
  fresh_rootfs
  check "legacy probe, no filter (entry rewrite)" "$(GUEST --user root -- /legacy_probe 2>&1 | tail -1)" "legacy-probe passed"
  check "legacy probe behind filter, kernel >= 4.8 order (entry rewrite)" \
    "$(GUEST --user root -- /legacy_probe --filter 2>&1 | tail -1)" "legacy-probe passed"
  check "legacy probe behind filter, 4.4 order (SIGSYS re-issue)" \
    "$(WORKFLOW_ENGINE_TEST_LEGACY=sigsys GUEST --user root -- /legacy_probe --filter 2>&1 | tail -1)" "legacy-probe passed"
  local golden="$eng_dir/tests/native/guest/paths.golden" mode
  for mode in entry sigsys; do
    fresh_rootfs
    set +e
    WORKFLOW_ENGINE_TEST_LEGACY=$mode timeout 300 "$build/engine" run --root "$rootfs" --user root -- \
      /legacy_probe --exec /bin/bash /wf-paths.sh e > "$build/paths.$mode" 2>"$build/paths.$mode.stderr"
    set -e
    if diff -q "$golden" "$build/paths.$mode" >/dev/null; then ok "path oracle behind the app filter ($mode) identical"
    else bad "path oracle behind the app filter ($mode) differs ($build/paths.$mode)"; fi
  done
}

# Installed rootfs (metadata in xattrs, hardlinks emulated) from the pinned layer,
# built once by the reference tool test/mkrootfs.py; each case works on a reflink copy.
layer_path() { local l; l=$(cat "$repo_dir/artifacts/engine/layer-amd64.path"); echo "$repo_dir/artifacts/engine/$l"; }
m3_pristine="$build/m3-pristine"
m3_fresh() {
  if [ ! -f "$m3_pristine/.wf-ok" ]; then
    rm -rf -- "$m3_pristine"
    python3 "$eng_dir/tests/native/mkrootfs.py" "$(layer_path)" "$m3_pristine" >/dev/null
    touch "$m3_pristine/.wf-ok"
  fi
  rm -rf -- "$rootfs"
  cp -a --reflink=auto -- "$m3_pristine" "$rootfs"
  rm -f "$rootfs/.wf-ok"
  cp -f "$eng_dir/tests/native/guest/meta.sh" "$rootfs/wf-meta.sh"
  cp -f "$build/meta_probe" "$rootfs/meta_probe"
}
R() { "$build/engine" run --root "$rootfs" --user root "$@"; }
RQ() { R -- /bin/sh -c "$1" 2>&1; }

suite_m3() {
  printf '== M3 metadata store, hardlink emulation, crash injection\n'
  m3_fresh
  local golden="$eng_dir/tests/native/guest/meta.golden"
  # meta.golden is the podman (real kernel, real ownership) output except three
  # rows rootless podman cannot produce: /dev/null shows 65534 there (host node
  # in a user namespace) and mknod c fails (no CAP_MKNOD for devices); a real
  # root on Debian gets "666 0 0", "644 0 0 ... 1,3" and a 0-byte read.
  set +e; timeout 300 "$build/engine" run --root "$rootfs" --user root -- /bin/bash /wf-meta.sh > "$build/meta.engine" 2>&1; set -e
  if diff -u "$golden" "$build/meta.engine" > "$build/meta.diff"; then ok "metadata semantics: $(wc -l < "$golden") cases identical to the oracle"
  else bad "metadata semantics differ ($build/meta.diff)"; fi
  if [ "${ENGINE_ORACLE:-0}" = 1 ] && command -v podman >/dev/null; then
    podman run --rm --network=none -v "$eng_dir/tests/native/guest/meta.sh:/wf-meta.sh:ro" \
      "docker.io/library/debian@$(cut -d' ' -f1 < "$repo_dir/artifacts/engine/amd64.digest" 2>/dev/null || echo sha256:7792b1f7702a86946cd518db72b6a407302c3e9bc1635634368b878189e8221c)" \
      /bin/bash /wf-meta.sh > "$build/meta.podman" 2>&1 || true
    local nd; nd=$(diff "$golden" "$build/meta.podman" | grep -c '^>' || true)
    local od; od=$(diff "$golden" "$build/meta.podman" | grep '^>' | grep -vc -e '^> img_devnull' -e '^> mknod_c' -e '^> mknod_write' || true)
    check "podman oracle differs only in the 3 documented rows" "$nd/$od" "3/0"
  fi
  check "fsck after the metadata workload" "$("$build/engine" fsck --root "$rootfs" | tail -1 | sed 's/.*), //')" "0 problem(s)"
  check "engine store hidden from / listing" "$(RQ 'ls -a / | grep -c workflow-engine')" "0"
  check "engine xattrs hidden and protected" "$(RQ 'touch /tmp/x && /meta_probe /tmp/x')" "meta-probe passed"

  # persistence across engine instances
  RQ 'mkdir -p /srv/p && echo a > /srv/p/f && chown 1:2 /srv/p/f && chmod 4750 /srv/p/f && ln /srv/p/f /srv/p/g && mkfifo -m 600 /srv/p/q && mknod /srv/p/z c 1 5' >/dev/null
  check "metadata persists across engine restart" "$(RQ 'stat -c "%a %u %g %h %F" /srv/p/f /srv/p/g /srv/p/q; stat -c "%a %t,%T %F" /srv/p/z' | tr '\n' '|')" \
    "4750 1 2 2 regular file|4750 1 2 2 regular file|600 0 0 1 fifo|644 1,5 character special file|"
  check "FIFO placeholder carries data between processes (non-root)" \
    "$("$build/engine" run --root "$rootfs" --uid 1000 --gid 1000 -- /bin/sh -c 'mkfifo /tmp/wf.fifo && (echo through > /tmp/wf.fifo &) && cat /tmp/wf.fifo; stat -c "%F %u" /tmp/wf.fifo; rm /tmp/wf.fifo' 2>&1 | tr '\n' '|')" \
    "through|fifo 1000|"
  check "virtual /dev/zero node reads zeros" "$(RQ 'head -c 4 /srv/p/z | od -An -tx1 | tr -d " "')" "00000000"

  # dpkg: package with owners, set-id, hardlinks built and installed in the guest
  RQ 'set -e; d=/tmp/pkg; rm -rf $d; mkdir -p $d/DEBIAN $d/usr/local/wf/sub
      printf "Package: wftest\nVersion: 1.0\nArchitecture: all\nMaintainer: t <t@t>\nDescription: t\n" > $d/DEBIAN/control
      echo payload > $d/usr/local/wf/tool; chmod 4755 $d/usr/local/wf/tool
      echo data > $d/usr/local/wf/sub/owned; chown 1:1 $d/usr/local/wf/sub/owned; chmod 640 $d/usr/local/wf/sub/owned
      chown 0:50 $d/usr/local/wf/sub; chmod 2775 $d/usr/local/wf/sub
      ln $d/usr/local/wf/tool $d/usr/local/wf/tool-hl
      dpkg-deb --build $d /tmp/wftest.deb >/dev/null && dpkg -i /tmp/wftest.deb >/dev/null' > "$build/dpkg.log" 2>&1 || true
  check "dpkg -i: owners/modes/hardlinks persist into a new instance" \
    "$(RQ 'stat -c "%n %a %U %G %h" /usr/local/wf/tool /usr/local/wf/tool-hl /usr/local/wf/sub/owned; stat -c "%n %a %U %G" /usr/local/wf/sub; dpkg-query -W -f="\${Status}" wftest' | tr '\n' '|')" \
    "/usr/local/wf/tool 4755 root root 2|/usr/local/wf/tool-hl 4755 root root 2|/usr/local/wf/sub/owned 640 daemon daemon 1|/usr/local/wf/sub 2775 root staff|install ok installed"
  check "dpkg --verify clean" "$(RQ 'dpkg --verify wftest && echo clean')" "clean"
  check "dpkg -r removes everything, store consistent" \
    "$(RQ 'dpkg -r wftest >/dev/null 2>&1; test -e /usr/local/wf && echo left || echo gone'; "$build/engine" fsck --root "$rootfs" | tail -1 | sed 's/.*), //')" \
    "$(printf 'gone\n0 problem(s)')"

  # crash injection: the engine dies at each step of a hardlink operation
  # (first link of a file = conversion into the store; then adding a name; then a removal)
  local pt out
  for pt in link-journaled link-moved link-stubbed link-counted link-drop; do
    RQ 'rm -rf /srv/c && mkdir /srv/c && echo precious > /srv/c/f && chmod 604 /srv/c/f' >/dev/null || true
    set +e
    case $pt in
      link-counted) RQ 'ln /srv/c/f /srv/c/k' >/dev/null; WORKFLOW_ENGINE_CRASH_AT=$pt R -- /bin/ln /srv/c/k /srv/c/g >/dev/null 2>&1 ;;
      link-drop) RQ 'ln /srv/c/f /srv/c/k' >/dev/null; WORKFLOW_ENGINE_CRASH_AT=$pt R -- /bin/rm /srv/c/k >/dev/null 2>&1 ;;
      *) WORKFLOW_ENGINE_CRASH_AT=$pt R -- /bin/ln /srv/c/f /srv/c/k >/dev/null 2>&1 ;;
    esac
    local crc=$?
    set -e
    out=$(RQ 'cat /srv/c/f; stat -c "%a %h" /srv/c/f; ls /srv/c | tr "\n" " "')
    "$build/engine" fsck --root "$rootfs" > "$build/fsck.$pt" || true
    "$build/engine" fsck --root "$rootfs" --repair > /dev/null || true
    local after; after=$("$build/engine" fsck --root "$rootfs" | tail -1 | sed 's/.*), //')
    local names; names=$(RQ 'ls /srv/c | wc -l')
    local nl; nl=$(RQ 'stat -c %h /srv/c/f')
    if [ $crc = 99 ] && [ "${out%%$'\n'*}" = precious ] && [ "$after" = "0 problem(s)" ] && [ "$nl" = "$names" ]; then
      ok "crash at $pt: content intact, recovered ($(grep -c -e count -e orphan "$build/fsck.$pt" || true) finding(s) repaired), nlink=$nl names=$names"
    else bad "crash at $pt (exit $crc): $out / $after / nlink $nl names $names"; fi
  done

  # two engine instances on one rootfs
  RQ 'rm -rf /srv/m && mkdir /srv/m && echo x > /srv/m/x && ln /srv/m/x /srv/m/x0' >/dev/null
  R -- /bin/bash -c 'for i in $(seq 1 60); do ln /srv/m/x /srv/m/a$i; done' > /dev/null 2>&1 &
  local p1=$!
  R -- /bin/bash -c 'for i in $(seq 1 60); do ln /srv/m/x /srv/m/b$i; done' > /dev/null 2>&1 &
  local p2=$!
  R -- /bin/bash -c 'for i in $(seq 1 40); do chmod 6$((i % 8))0 /srv/m/x; done; chmod 640 /srv/m/x' > /dev/null 2>&1 &
  local p3=$!
  R -- /bin/bash -c 'for i in $(seq 1 40); do chown $((i % 5)):$((i % 7)) /srv/m/x; done; chown 1:1 /srv/m/x' > /dev/null 2>&1 &
  local p4=$!
  wait $p1 $p2 $p3 $p4 || true
  check "concurrent instances: 122 names counted, no lost chmod/chown update" "$(RQ 'stat -c "%h %a %u %g" /srv/m/x')" "122 640 1 1"
  R -- /bin/bash -c 'rm /srv/m/a*' >/dev/null 2>&1 & p1=$!
  R -- /bin/bash -c 'rm /srv/m/b*' >/dev/null 2>&1 & p2=$!
  wait $p1 $p2 || true
  check "concurrent removal: count back to 2, store consistent" \
    "$(RQ 'stat -c %h /srv/m/x'; "$build/engine" fsck --root "$rootfs" | tail -1 | sed 's/.*), //')" "$(printf '2\n0 problem(s)')"
  check "fsck --repair refused while an instance runs" \
    "$(R -- /bin/sleep 2 & sleep 0.5; set +e; "$build/engine" fsck --root "$rootfs" --repair >/dev/null 2>&1; echo $?; set -e; wait)" "3"

  # binds keep no metadata: owner = bind owner, mode = host bits
  rm -rf "$build/m3bind"; mkdir -p "$build/m3bind"
  local BD=(--bind "$build/m3bind:/mnt/w")
  check "bind: root-created file presented as work, chown accepted without effect" \
    "$(R "${BD[@]}" -- /bin/sh -c 'touch /mnt/w/n && chown 0:0 /mnt/w/n && chmod 640 /mnt/w/n && stat -c "%a %u %g" /mnt/w/n' 2>&1)" "640 1000 1000"
  check "bind: host file has the mode and no engine xattr" \
    "$(stat -c %a "$build/m3bind/n"; getfattr -d -m user.workflow "$build/m3bind/n" 2>/dev/null | wc -l)" "$(printf '640\n0')"
  check "bind: device nodes and hardlinks refused" \
    "$(R "${BD[@]}" -- /bin/sh -c 'mknod /mnt/w/d c 1 3 2>&1; ln /mnt/w/n /mnt/w/l 2>&1' | sed 's/.*: //' | tr '\n' '|')" \
    "Operation not permitted|Operation not permitted|"
  check "bind: moving a hardlinked file out copies it (EXDEV)" \
    "$(R "${BD[@]}" -- /bin/sh -c 'echo hl > /srv/h1 && ln /srv/h1 /srv/h2 && mv /srv/h2 /mnt/w/h2 && cat /mnt/w/h2 && stat -c %h /srv/h1' 2>&1 | tr '\n' '|'; [ -f "$build/m3bind/h2" ] && [ ! -L "$build/m3bind/h2" ] && echo host-regular)" \
    "hl|1|host-regular"
  check "bind: moving a file in keeps the owner the guest saw" \
    "$(R "${BD[@]}" --user work -- /bin/sh -c 'echo z > /mnt/w/in' 2>&1; R "${BD[@]}" -- /bin/sh -c 'mv /mnt/w/in /srv/in && stat -c "%u %g" /srv/in' 2>&1)" "1000 1000"
}

# Workspace image (artifacts/image/amd64, docs/environment.md §2) installed by the
# reference tool; the podman oracle is the same image imported as a plain rootfs.
ws_image="$repo_dir/artifacts/image/amd64/image.tar.zst"
ws_pristine="$build/ws-pristine"
WS_ENV=(PATH=/opt/toolchains/active/python/bin:/opt/toolchains/active/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
        HOME=/home/work USER=work LOGNAME=work LANG=en_US.UTF-8 SHELL=/bin/bash)
ws_fresh() {
  [ -f "$ws_image" ] || { echo "no workspace image at $ws_image" >&2; return 1; }
  local stamp; stamp=$(cut -c1-64 "$ws_image.sha256" 2>/dev/null || sha256sum "$ws_image" | cut -c1-64)
  if [ "$(cat "$ws_pristine/.wf-ok" 2>/dev/null)" != "$stamp" ]; then
    rm -rf -- "$ws_pristine"
    python3 "$eng_dir/tests/native/mkrootfs.py" --image "$ws_image" "$ws_pristine" >/dev/null
    echo "$stamp" > "$ws_pristine/.wf-ok"
  fi
  rm -rf -- "$rootfs"
  cp -a --reflink=auto -- "$ws_pristine" "$rootfs"
  rm -f "$rootfs/.wf-ok"
}
WS() { env -i "${WS_ENV[@]}" WORKFLOW_ENGINE_LOADER="$build/loader" "$build/engine" run --root "$rootfs" --cwd /home/work "$@"; }

suite_m4() {
  printf '== M4 exec (shebang, execveat, binfmt), fake identity, sudo su\n'
  ws_fresh
  check "sudo su: guest root, host uid unchanged" \
    "$(printf 'id -u; whoami; grep ^Uid: /proc/self/status | cut -f2\n' | WS -- /usr/bin/sudo -n su 2>&1 | tr '\n' ' ')" "0 root $(id -u) "
  cp -f "$eng_dir/tests/native/guest/ident.sh" "$rootfs/wf-ident.sh"
  local golden="$eng_dir/tests/native/guest/ident.golden"
  set +e; WS -- /bin/bash /wf-ident.sh > "$build/ident.engine" 2>&1; set -e
  if diff -u "$golden" "$build/ident.engine" > "$build/ident.diff"; then
    ok "identity/exec semantics: $(wc -l < "$golden") cases identical to the oracle (sudo, su, chage, set-id, AT_SECURE, comm, execveat, shebang depth)"
  else bad "identity/exec semantics differ ($build/ident.diff)"; fi
  if [ "${ENGINE_ORACLE:-0}" = 1 ] && command -v podman >/dev/null; then
    local tag; tag=localhost/wf-engine-oracle:$(cut -c1-12 "$ws_image.sha256")
    if ! podman image exists "$tag"; then
      PYTHONPATH="$repo_dir/image" python3 -m wfimage to-tar "$ws_image" "$build/oracle.tar" --index "${ws_image%.tar.zst}.json" >/dev/null
      podman import -q "$build/oracle.tar" "$tag" >/dev/null; rm -f "$build/oracle.tar"
    fi
    podman run --rm --network=none -u work -w /home/work $(printf -- '-e %s ' "${WS_ENV[@]}") \
      -v "$eng_dir/tests/native/guest/ident.sh:/wf-ident.sh:ro" "$tag" /bin/bash /wf-ident.sh > "$build/ident.podman" 2>&1 || true
    if diff -q "$golden" "$build/ident.podman" >/dev/null; then ok "podman oracle matches ident.golden"
    else bad "podman oracle drifted from ident.golden ($build/ident.podman)"; fi
  fi
  check "root shell via --user root" "$(WS --user root -- /usr/bin/id -un 2>&1)" "root"

  # binfmt: command-line rules and the guest's /etc/binfmt.d (fake interpreters, no QEMU)
  printf '#!/bin/sh\necho "interp[$#]: $*"\n' > "$rootfs/usr/local/bin/wfbin-interp"
  printf '#!/bin/sh\necho "qemu[$#]: $*"\n' > "$rootfs/usr/local/bin/fake-qemu-aarch64"
  chmod 755 "$rootfs/usr/local/bin/wfbin-interp" "$rootfs/usr/local/bin/fake-qemu-aarch64"
  mkdir -p "$rootfs/etc/binfmt.d"
  printf '%s\n' ':wfprs:M::WFPRS::/usr/local/bin/wfbin-interp:P' \
    ':qemu-aarch64:M::\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\xb7\x00:\xff\xff\xff\xff\xff\xff\xff\x00\xff\xff\xff\xff\xff\xff\xff\xff\xfe\xff\xff\xff:/usr/local/bin/fake-qemu-aarch64:POCF' \
    > "$rootfs/etc/binfmt.d/wf-test.conf"
  local a64="$repo_dir/artifacts/image/arm64/base.tar.zst"
  if [ -f "$a64" ]; then
    zstd -dc "$a64" | tar -xO rootfs/usr/bin/true > "$rootfs/tmp/arm64-true" 2>/dev/null || true
    chmod 755 "$rootfs/tmp/arm64-true"
  fi
  cp -f "$eng_dir/tests/native/guest/binfmt.sh" "$rootfs/wf-binfmt.sh"
  set +e
  WS --user root --binfmt ':wfbin:M::WFBIN::/usr/local/bin/wfbin-interp:' \
     --binfmt ':wfext:E::wfx::/usr/local/bin/wfbin-interp:' -- /bin/bash /wf-binfmt.sh > "$build/binfmt.engine" 2>&1
  set -e
  if diff -u "$eng_dir/tests/native/guest/binfmt.golden" "$build/binfmt.engine" > "$build/binfmt.diff"; then
    ok "binfmt: magic, extension, P flag, PATH search, foreign ELF -> interpreter, no rule"
  else bad "binfmt results differ ($build/binfmt.diff)"; fi
  check "foreign ELF without a rule -> ENOEXEC (126)" \
    "$(WS --user root --no-guest-binfmt -- /bin/bash -c '/tmp/arm64-true 2>/dev/null; echo $?' 2>&1)" "126"
}

# The seccomp RET_TRACE fast path is the default (kernel >= 4.8).  This suite
# re-runs the oracles with it off (WORKFLOW_ENGINE_SECCOMP=0: every syscall
# stops, the mode used on 4.x < 4.8 kernels) and checks the fast path's effect.
suite_plain() {
  printf '== plain PTRACE_SYSCALL mode (fast path off) + fast-path effect\n'
  fresh_rootfs
  local st_fast st_plain
  st_fast=$(WORKFLOW_ENGINE_STATS=1 GUEST -- /bin/bash -c 'for i in $(seq 100); do cat /etc/passwd >/dev/null; done' 2>&1 | grep -o 'syscall_stops=[0-9]*' | cut -d= -f2)
  st_plain=$(WORKFLOW_ENGINE_SECCOMP=0 WORKFLOW_ENGINE_STATS=1 GUEST -- /bin/bash -c 'for i in $(seq 100); do cat /etc/passwd >/dev/null; done' 2>&1 | grep -o 'syscall_stops=[0-9]*' | cut -d= -f2)
  if [ -n "$st_fast" ] && [ -n "$st_plain" ] && [ $((st_fast * 3)) -lt "$st_plain" ]; then
    ok "fast path: $st_fast syscall stops vs $st_plain without it (100x cat)"
  else bad "fast path did not reduce stops ($st_fast vs $st_plain)"; fi
  export WORKFLOW_ENGINE_SECCOMP=0
  set +e
  WORKFLOW_ENGINE_CHECK_TOGGLE=1 timeout 300 "$build/engine" run --root "$rootfs" --user root -- /bin/bash /wf-paths.sh e > "$build/paths.plain" 2>/dev/null
  set -e
  if diff -q "$eng_dir/tests/native/guest/paths.golden" "$build/paths.plain" >/dev/null; then ok "plain: 77 path cases identical"
  else bad "plain: path cases differ ($build/paths.plain)"; fi
  if [ "$(uname -m)" = x86_64 ]; then
    check "plain: legacy probe behind filter" "$(GUEST --user root -- /legacy_probe --filter 2>&1 | tail -1)" "legacy-probe passed"
  fi
  m3_fresh
  set +e; timeout 300 "$build/engine" run --root "$rootfs" --user root -- /bin/bash /wf-meta.sh > "$build/meta.plain" 2>&1; set -e
  if diff -q "$eng_dir/tests/native/guest/meta.golden" "$build/meta.plain" >/dev/null; then ok "plain: metadata oracle identical"
  else bad "plain: metadata oracle differs ($build/meta.plain)"; fi
  if [ -f "$ws_image" ]; then
    ws_fresh
    cp -f "$eng_dir/tests/native/guest/ident.sh" "$rootfs/wf-ident.sh"
    set +e; WS -- /bin/bash /wf-ident.sh > "$build/ident.plain" 2>&1; set -e
    if diff -q "$eng_dir/tests/native/guest/ident.golden" "$build/ident.plain" >/dev/null; then ok "plain: identity oracle identical"
    else bad "plain: identity oracle differs ($build/ident.plain)"; fi
  fi
  unset WORKFLOW_ENGINE_SECCOMP
}

# The M3/M4 oracles behind the whole measured Android app filter (appfilter.h:
# legacy + newer calls + set*id): the calls the app filter traps must still
# behave exactly like the real kernel, with the fast path on (TRAP beats
# TRACE -> SIGSYS emulation) and in the 4.4 order (trap before the entry stop).
suite_appfilter() {
  printf '== M3/M4 oracles behind the Android app filter\n'
  local mode
  for mode in entry sigsys; do
    m3_fresh
    cp -f "$build/legacy_probe" "$rootfs/legacy_probe"
    set +e
    WORKFLOW_ENGINE_TEST_LEGACY=$mode timeout 300 "$build/engine" run --root "$rootfs" --user root --test-app-filter -- \
      /bin/bash /wf-meta.sh > "$build/meta.app.$mode" 2>&1
    set -e
    if diff -q "$eng_dir/tests/native/guest/meta.golden" "$build/meta.app.$mode" >/dev/null; then ok "metadata oracle behind the app filter ($mode)"
    else bad "metadata oracle behind the app filter ($mode) differs ($build/meta.app.$mode)"; fi
    if [ -f "$ws_image" ]; then
      ws_fresh
      cp -f "$eng_dir/tests/native/guest/ident.sh" "$rootfs/wf-ident.sh"
      cp -f "$build/legacy_probe" "$rootfs/legacy_probe"
      set +e
      WORKFLOW_ENGINE_TEST_LEGACY=$mode WS --test-app-filter -- /bin/bash /wf-ident.sh > "$build/ident.app.$mode" 2>&1
      set -e
      if diff -q "$eng_dir/tests/native/guest/ident.golden" "$build/ident.app.$mode" >/dev/null; then ok "identity oracle behind the app filter ($mode)"
      else bad "identity oracle behind the app filter ($mode) differs ($build/ident.app.$mode)"; fi
    fi
  done
}

declare -A suites=([appfilter]=suite_appfilter [m0]=suite_m0 [m1]=suite_m1 [m2]=suite_m2 [legacy]=suite_legacy [m3]=suite_m3 [m4]=suite_m4
                   [plain]=suite_plain)
order=(m0 m1 m2 m3 m4 plain)
[ "$(uname -m)" = x86_64 ] && order+=(legacy appfilter)

build_engine
sel=("$@"); ((${#sel[@]})) || sel=("${order[@]}")
for s in "${sel[@]}"; do
  fn=${suites[$s]:-}; [ -n "$fn" ] || { echo "unknown suite: $s" >&2; exit 2; }
  $fn
done
printf '\n== engine host tests: %d passed, %d failed\n' "$pass" "$fail"
for f in "${failed[@]}"; do printf '   - %s\n' "$f"; done
[ "$fail" = 0 ]
