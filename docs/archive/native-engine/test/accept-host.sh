#!/usr/bin/env bash
# Host acceptance run (research-engine.md §4.6) with the real workspace image,
# laid out exactly as on the device (environment.md §4) and started the way the
# app's ProcessLauncher does (docs/engine.md "CLI"):
#
#   native/engine/test/accept-host.sh [phase...]     phases: install workload pty lifecycle soak
#
# Needs network (apt, uv, npm, git).  Output: artifacts/engine/accept/.
set -uo pipefail
eng_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
repo=$(cd -- "$eng_dir/../.." && pwd)
build=${ENGINE_HOST_BUILD:-$repo/artifacts/engine/host-build}
out=${ENGINE_ACCEPT_OUT:-$repo/artifacts/engine/accept}
ws=$out/.workspace
E=$ws/.workflow/engine
image=$repo/artifacts/image/amd64/image.tar.zst
engine=$build/engine
export WORKFLOW_ENGINE_LOADER=$build/loader
mkdir -p "$out"
pass=0; fail=0
ok()  { echo "ok   $1"; pass=$((pass+1)); }
bad() { echo "FAIL $1 ${2:-}"; fail=$((fail+1)); }

# ProcessLauncher equivalent: clean env from metadata.environment, runtime binds
guest_env() {
  python3 - "$image" <<'EOF'
import json, sys
m = json.load(open(sys.argv[1].replace(".tar.zst", ".json")))["metadata"]
env = dict(m["environment"])
env.update(HOME="/home/work", USER="work", LOGNAME="work", SHELL="/bin/bash", TERM="xterm-256color", TZ="Asia/Hong_Kong")
for k, v in env.items():
    print("%s=%s" % (k, v))
EOF
}
run_guest() {   # run_guest [engine run options...] -- CMD...
  local -a env; mapfile -t env < <(guest_env)
  env -i "${env[@]}" WORKFLOW_ENGINE_LOADER="$build/loader" "$engine" run \
    --root "$E/generations/1/rootfs" \
    --bind "$ws:/workspace" --hide /workspace/.workflow \
    --bind "$E/home/work:/home/work" \
    --bind "$E/toolchains:/opt/toolchains" \
    --bind "$E/tmp/resolv.conf:/etc/resolv.conf" \
    --socket-dir "/tmp/wfs-$(id -u)" \
    --cwd /workspace "$@"
}

phase_install() {
  echo "== install (engine installer, seeds merged like EngineStore)"
  rm -rf "$ws"; mkdir -p "$E/generations" "$E/tmp"
  if "$engine" install --quiet --image "$image" --index "${image%.tar.zst}.json" --target "$E/generations/1.partial" > "$out/install.json"; then
    mv "$E/generations/1.partial" "$E/generations/1" && ok "install $(cat "$out/install.json")"
  else bad install; return; fi
  mkdir -p "$E/home"
  mv "$E/generations/1/seeds/home/work" "$E/home/work"          # if-absent
  mv "$E/generations/1/seeds/toolchains" "$E/toolchains"        # merge into an absent store
  rmdir -p "$E/generations/1/seeds/home" 2>/dev/null || true
  # the app writes the device's current DNS servers here (environment.md §2.6)
  local dns; dns=$( (grep -hm1 '^nameserver' /etc/resolv.conf /run/systemd/resolve/resolv.conf 2>/dev/null ||
                     nmcli -g IP4.DNS dev show 2>/dev/null | grep -m1 . | sed 's/^/nameserver /') | head -1)
  echo "${dns:-nameserver 1.1.1.1}" > "$E/tmp/resolv.conf"
  if "$engine" verify --quiet --generation "$E/generations/1" >/dev/null; then ok "verify generation"; else bad verify; fi
}

phase_workload() {
  echo "== workload (guest, as work)"
  cp -f "$eng_dir/test/guest/accept.sh" "$ws/accept.sh"
  run_guest -- /bin/bash /workspace/accept.sh 1 2>&1 | tee "$out/workload-1.log" | grep -E '^(ok|FAIL|accept)'
  grep -q 'accept: .* 0 failed' "$out/workload-1.log" && ok "workload phase 1" || bad "workload phase 1"
  run_guest -- /bin/bash /workspace/accept.sh 2 2>&1 | tee "$out/workload-2.log" | grep -E '^(ok|FAIL|accept)'
  grep -q 'accept: .* 0 failed' "$out/workload-2.log" && ok "workload phase 2 (new instance)" || bad "workload phase 2"
  local u; u=$(run_guest -- /usr/bin/sudo -n su -c 'id -u; grep ^Uid: /proc/self/status | cut -f2' 2>/dev/null | tr '\n' ' ')
  [ "$u" = "0 $(id -u) " ] && ok "sudo su: guest uid 0, host uid $(id -u) unchanged" || bad "sudo su host uid" "$u"
  if "$engine" verify --quiet --generation "$E/generations/1" >/dev/null; then ok "verify after workload (apt, dpkg, hardlinks)"
  else bad "verify after workload"; fi
}

phase_pty() {
  echo "== PTY: job control, Ctrl-C, resize, pipelines, less"
  local -a env; mapfile -t env < <(guest_env)
  python3 "$eng_dir/test/pty_check.py" -- env -i "${env[@]}" WORKFLOW_ENGINE_LOADER="$build/loader" "$engine" run \
    --root "$E/generations/1/rootfs" --bind "$ws:/workspace" --hide /workspace/.workflow \
    --bind "$E/home/work:/home/work" --bind "$E/toolchains:/opt/toolchains" --cwd /workspace -- /bin/bash -i \
    > "$out/pty.log" 2>&1
  local rc=$?
  grep -E '^(ok|FAIL)' "$out/pty.log"
  [ $rc = 0 ] && ok "pty checks" || bad "pty checks (see $out/pty.log)"
}

phase_lifecycle() {
  echo "== lifecycle"
  # SIGTERM: forwarded, whole tree gone within the grace period
  run_guest -- /bin/bash -c 'sleep 300 & sleep 300 & (trap "" TERM; sleep 300) & wait' >/dev/null 2>&1 &
  local ep=$!; sleep 2
  local kids; kids=$(pgrep -P $ep | wc -l)
  kill -TERM $ep; local t0=$SECONDS; wait $ep; local st=$?
  sleep 0.5
  local left; left=$(ps -eo pid,args | grep -c '[s]leep 300')
  [ $left = 0 ] && [ $((SECONDS - t0)) -le 6 ] && ok "SIGTERM reaps the tree (exit $st, TERM-ignoring child killed after grace)" || bad "SIGTERM" "left=$left"
  # engine killed: PTRACE_O_EXITKILL takes the guests with it
  run_guest -- /bin/bash -c 'sleep 301 & sleep 301 & wait' >/dev/null 2>&1 &
  ep=$!; sleep 2
  local engpid; engpid=$(pgrep -f -n "engine run --root $E/generations/1/rootfs")
  kill -KILL "$engpid"; wait $ep 2>/dev/null; sleep 0.5
  left=$(ps -eo pid,args | grep -c '[s]leep 301')
  [ $left = 0 ] && ok "SIGKILL of the engine kills the guest tree (EXITKILL)" || bad "EXITKILL" "left=$left"
  # power loss during install: the target stays *.partial (never committed); remove cleans it
  rm -rf "$E/generations/2.partial"
  "$engine" install --quiet --image "$image" --index "${image%.tar.zst}.json" --target "$E/generations/2.partial" >/dev/null 2>&1 &
  local ip=$!; sleep 1; kill -KILL $ip; wait $ip 2>/dev/null
  if [ -d "$E/generations/2.partial" ] && [ ! -e "$E/generations/2" ] && "$engine" remove --generation "$E/generations/2.partial" && [ ! -e "$E/generations/2.partial" ]; then
    ok "install killed midway: only 2.partial left, removed"
  else bad "install kill"; fi
  # clone for an apt change, then remove it; remove refused while running
  if "$engine" clone --quiet --from "$E/generations/1" --to "$E/generations/3.partial" >/dev/null &&
     "$engine" verify --quiet --generation "$E/generations/3.partial" >/dev/null; then ok "clone + verify"; else bad clone; fi
  run_guest -- /bin/sleep 3 >/dev/null 2>&1 &
  sleep 1
  "$engine" remove --generation "$E/generations/1" >/dev/null 2>&1; local rrc=$?
  wait
  [ $rrc = 3 ] && [ -d "$E/generations/1/rootfs" ] && ok "remove refused while an instance runs" || bad "remove while running" "rc=$rrc"
  "$engine" remove --generation "$E/generations/3.partial" && [ ! -e "$E/generations/3.partial" ] && ok "remove clone" || bad "remove clone"
}

phase_soak() {
  local mins=${SOAK_MINUTES:-10}
  echo "== soak: ${mins} min of builds/servers in one instance"
  run_guest -- /bin/bash -c "
    end=\$((SECONDS + $mins * 60)); n=0
    python3 -m http.server 18765 --bind 127.0.0.1 >/dev/null 2>&1 & srv=\$!
    for i in \$(seq 100); do curl -s -o /dev/null http://127.0.0.1:18765/ && break; sleep 0.2; done
    while [ \$SECONDS -lt \$end ]; do
      cd /workspace/accept/mk && rm -f *.o && make -s -j4 >/dev/null || { echo make-failed; exit 1; }
      curl -s -o /dev/null http://127.0.0.1:18765/ || { echo curl-failed; exit 2; }
      n=\$((n + 1)); echo \$n > /workspace/accept/soak.rounds
    done
    kill \$srv; echo rounds=\$n" > "$out/soak.log" 2>&1 &
  local ep=$!
  : > "$out/soak-rss.log"
  while kill -0 $ep 2>/dev/null; do
    local p; p=$(pgrep -f -n "engine run --root $E/generations/1/rootfs")
    [ -n "$p" ] && echo "$SECONDS $(grep VmRSS /proc/$p/status | awk '{print $2}')" >> "$out/soak-rss.log"
    sleep 10
  done
  wait $ep; local rc=$?
  local first last; first=$(head -2 "$out/soak-rss.log" | tail -1 | cut -d' ' -f2); last=$(tail -1 "$out/soak-rss.log" | cut -d' ' -f2)
  if [ $rc = 0 ] && [ -n "$first" ] && [ "$last" -le $((first * 2 + 2048)) ]; then
    ok "soak $mins min: $(cat "$out/soak.log"), engine RSS ${first} -> ${last} kB"
  else bad "soak" "rc=$rc $(cat "$out/soak.log") rss $first -> $last"; fi
  sleep 1
  local orphans; orphans=$(ps -eo pid,ppid,args | grep '[p]ython3 -m http.server 18765' | grep -v 'engine run')
  [ -z "$orphans" ] && ok "no orphans after soak" || bad "orphans after soak" "$orphans"
}

phases=("$@"); ((${#phases[@]})) || phases=(install workload pty lifecycle)
for p in "${phases[@]}"; do "phase_$p"; done
echo "== accept-host: $pass ok, $fail failed"
[ $fail = 0 ]
