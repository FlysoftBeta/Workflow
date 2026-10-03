#!/usr/bin/env bash
# Engine device/AVD acceptance: run libworkflow-engine.so inside a real (zygote-forked) app process.
#   engine/runtime/tests/device/run.sh SERIAL [CASE_FILE]
# CASE_FILE defaults to engine/runtime/tests/device/cases/default.json. Environment switches:
#   ENGINE_BUILD=0     do not run engine/runtime/test-android.sh (use artifacts/engine/rust-harness as is)
#   HARNESS_BUILD=0    do not run Gradle (reuse the last harness APKs)
#   ONLY=id1,id2       run only these case ids
#   REPROVISION=1      re-extract the rootfs even if the device stamp matches
#   UNINSTALL=1        uninstall the harness (and only the harness) at the end
#   WORKSPACE_IMAGE=1  upload artifacts/image/<arch>/image.tar.zst + image.json (for cases/m5.json)
# Results: artifacts/engine/device/<avd-name|serial>/{results.json,g0.json,out/,instrument.log,summary.txt}
# Only touches the harness package top.flysoftbeta.workflow.engineharness(.test) on the device.
set -euo pipefail
dev_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd -- "$dev_dir/../../../.." && pwd)
sdk=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-$HOME/Android/Sdk}}
export ANDROID_HOME=$sdk
export PATH=$sdk/platform-tools:$PATH
pkg=top.flysoftbeta.workflow.engineharness
runner=$pkg.test/androidx.test.runner.AndroidJUnitRunner
harness=$repo/engine/runtime/tests/android-harness
test_engine_out=${ENGINE_TEST_OUTPUT:-$repo/artifacts/engine/rust-harness}

serial=${1:?usage: run.sh SERIAL [CASE_FILE]}
[[ ${ANDROID_SERIAL:-} == "$serial" && "$serial" == emulator-* ]] || { echo "run via tools/with-emulator.sh" >&2; exit 2; }
cases=${2:-$dev_dir/cases/default.json}
[[ -f $cases ]] || { echo "case file $cases not found" >&2; exit 2; }
python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$cases" || { echo "case file is not valid JSON" >&2; exit 2; }

adb() { command adb -s "$serial" "$@"; }
# Run a shell snippet as the harness app user (cwd = app data dir). The snippet must not contain '.
ras() { adb shell "run-as $pkg sh -c '$1'"; }
log() { printf '== %s\n' "$*" >&2; }

adb get-state >/dev/null 2>&1 || { echo "device $serial not available" >&2; exit 1; }
abi=$(adb shell getprop ro.product.cpu.abi | tr -d '\r')
api=$(adb shell getprop ro.build.version.sdk | tr -d '\r')
name=$serial
if [[ $serial == emulator-* ]]; then
  avd=$(adb emu avd name 2>/dev/null | head -1 | tr -d '\r' || true)
  [[ -n $avd && $avd != OK ]] && name=$avd
fi
out=$repo/artifacts/engine/device/$name${RESULT_SUFFIX:-}
mkdir -p "$out"
log "target $serial ($name) abi=$abi api=$api -> $out"
case "$abi" in
  arm64-v8a) layout=$repo/artifacts/engine/fixtures/debian-13-slim-arm64 ;;
  x86_64) layout=$repo/artifacts/engine/debian-13-slim-amd64 ;;
  *) echo "unsupported ABI $abi" >&2; exit 1 ;;
esac

# 1. engine + harness build
[[ ${ENGINE_BUILD:-1} == 0 ]] || { echo "run engine/runtime/test-android.sh to build and stage Rust artifacts" >&2; exit 2; }
# seccomp survey probe (independent of glibc loading), packaged as libwftest-sysprobe.so
ndk=${ANDROID_NDK:-$sdk/ndk/30.0.15729638}
case "$abi" in arm64-v8a) triple=aarch64-linux-android28 ;; *) triple=x86_64-linux-android28 ;; esac
mkdir -p "$repo/artifacts/engine/device/probe/$abi"
"$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/clang" --target=$triple -std=c17 -O1 -Wall -Wextra -Werror \
  -fPIE -pie -Wl,-z,max-page-size=16384 "$dev_dir/sysprobe.c" -o "$repo/artifacts/engine/device/probe/$abi/sysprobe"
apk=$harness/build/outputs/apk/debug/EngineHarness-debug.apk
test_apk=$harness/build/outputs/apk/androidTest/debug/EngineHarness-debug-androidTest.apk
if [[ ${HARNESS_BUILD:-1} != 0 ]]; then
  log "build harness APKs ($abi) under the Gradle lock"
  "$repo/tools/with-build-lock.sh" "$repo/gradlew" -p "$harness" --console=plain -q \
    -PengineAbis="$abi" -PengineOut="$test_engine_out" assembleDebug assembleDebugAndroidTest
fi
[[ -f $apk && -f $test_apk ]] || { echo "harness APKs missing (run without HARNESS_BUILD=0)" >&2; exit 1; }

# 2. install (never uninstalls anything)
log "install harness"
adb install -r -t "$apk" >/dev/null
adb install -r -t "$test_apk" >/dev/null
ras 'mkdir -p files && chmod 771 files'

# 3. rootfs provisioning into files/rootfs (stamp = sha256 of the normalised tarball)
tar_file=$repo/artifacts/engine/device/rootfs/debian-13-slim-$abi.tar
python3 "$dev_dir/mkrootfs.py" "$layout" "$tar_file" >&2
read -r tar_sha n_files n_dirs n_links < <(python3 -c '
import json,sys; m=json.load(open(sys.argv[1])); c=m["counts"]
print(m["tarSha256"], c["files"], c["dirs"] + 1, c["symlinks"])' "$tar_file.manifest.json")
stamp=$(ras 'cat files/rootfs.stamp 2>/dev/null' | tr -d '\r' || true)
if [[ ${REPROVISION:-0} != 0 || $stamp != "$tar_sha" ]]; then
  log "provision rootfs ($(du -h "$tar_file" | cut -f1), device stamp '${stamp:-none}')"
  ras 'rm -rf files/rootfs.tar files/rootfs.new files/rootfs.stamp'
  # shell protocol v2 (-T, no pty) passes binary stdin intact; `adb exec-in` truncates on API 28.
  adb shell -T "run-as $pkg sh -c 'cat > files/rootfs.tar'" < "$tar_file"
  dev_sha=$(ras 'sha256sum files/rootfs.tar' | cut -d' ' -f1 | tr -d '\r')
  [[ $dev_sha == "$tar_sha" ]] || { echo "rootfs upload corrupted: $dev_sha != $tar_sha" >&2; exit 1; }
  # toybox tar runs as the app uid: every chown fails (expected) and makes rc=1, so success is judged by
  # the absence of other errors and by exact file/dir/symlink counts.
  ras 'umask 0; cd files && mkdir rootfs.new && tar xf rootfs.tar -C rootfs.new 2>rootfs.tar.err; grep -v "chown 0:0" rootfs.tar.err; true' \
    | tee "$out/provision.err" >&2
  [[ -s $out/provision.err ]] && { echo "rootfs extraction reported errors (see $out/provision.err)" >&2; exit 1; }
  counts=$(ras 'cd files/rootfs.new && echo $(find . -type f | wc -l) $(find . -type d | wc -l) $(find . -type l | wc -l)' | tr -d '\r')
  [[ $counts == "$n_files $n_dirs $n_links" ]] || { echo "rootfs counts $counts != expected $n_files $n_dirs $n_links" >&2; exit 1; }
  ras "chmod 755 files/rootfs.new && rm -rf files/rootfs files/rootfs.tar files/rootfs.tar.err && mv files/rootfs.new files/rootfs && echo $tar_sha > files/rootfs.stamp"
  log "rootfs ok: files/dirs/symlinks = $counts"
else
  log "rootfs up to date ($tar_sha)"
fi

# 3b. optional: the real workspace image for the installer cases (cases/m5.json)
if [[ ${WORKSPACE_IMAGE:-0} != 0 ]]; then
  case "$abi" in arm64-v8a) iarch=arm64 ;; *) iarch=amd64 ;; esac
  img=$repo/artifacts/image/$iarch/image.tar.zst
  [[ -f $img ]] || { echo "no workspace image for $iarch at $img" >&2; exit 1; }
  isha=$(cut -c1-64 < "$img.sha256")
  dsha=$(ras 'cat files/image.stamp 2>/dev/null' | tr -d '\r' || true)
  if [[ $dsha != "$isha" ]]; then
    log "upload workspace image ($(du -h "$img" | cut -f1))"
    ras 'rm -f files/image.tar.zst files/image.stamp'
    adb shell -T "run-as $pkg sh -c 'cat > files/image.tar.zst'" < "$img"
    got=$(ras 'sha256sum files/image.tar.zst' | cut -d' ' -f1 | tr -d '\r')
    [[ $got == "$isha" ]] || { echo "image upload corrupted: $got" >&2; exit 1; }
    ras "echo $isha > files/image.stamp"
  fi
  adb shell -T "run-as $pkg sh -c 'cat > files/image.json'" < "${img%.tar.zst}.json"
fi

# 4. case file + instrumentation run in the app process
adb shell -T "run-as $pkg sh -c 'cat > files/cases.json'" < "$cases"
extra=()
[[ -n ${ONLY:-} ]] && extra+=(-e only "$ONLY")
# Freshly booted images (esp. the 2 GiB 16K google_apis AVD) run lmkd hard for ~1 min and kill even the
# instrumented harness (observed: killinfo for the harness pid, "Process crashed"). Let the device settle.
up=$(adb shell cat /proc/uptime | cut -d. -f1 | tr -d '\r')
if (( ${up:-999} < ${SETTLE_SECONDS:-120} )); then
  log "device uptime ${up}s: waiting until ${SETTLE_SECONDS:-120}s before instrumenting"
  sleep $(( ${SETTLE_SECONDS:-120} - up ))
fi
log "am instrument (case file $(basename "$cases"))"
set +e
adb shell am instrument -w -e cases cases.json "${extra[@]}" "$runner" 2>&1 | tr -d '\r' | tee "$out/instrument.log"
set -e

# 5. collect (ps: lets the summary name the harness process's parent, i.e. zygote/zygote64)
adb shell ps -A -o PID,PPID,NAME | tr -d '\r' > "$out/ps.txt" || true
rm -rf "$out/out" "$out/results.json" "$out/g0.json"
adb exec-out "run-as $pkg sh -c 'cd files/results && tar cf - .'" | tar xf - -C "$out"
cp "$cases" "$out/cases.json"
if grep -q "Process crashed" "$out/instrument.log"; then
  echo "WARNING: the harness process died during the run (lmkd kill or crash); results are partial." >&2
  adb logcat -d -b main,system,events 2>/dev/null | grep -E "killinfo|am_proc_died|engineharness" | tail -20 > "$out/crash-context.txt" || true
fi
[[ -f $out/results.json ]] || { echo "no results.json pulled (see $out/instrument.log)" >&2; exit 1; }
python3 "$dev_dir/summarize.py" "$out" --json --engine-dir "$test_engine_out/$abi" | tee "$out/summary.txt"

if [[ ${UNINSTALL:-0} != 0 ]]; then
  log "uninstall harness"
  adb uninstall "$pkg.test" >/dev/null || true
  adb uninstall "$pkg" >/dev/null || true
fi
