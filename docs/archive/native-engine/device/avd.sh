#!/usr/bin/env bash
# Create / boot / stop the engine acceptance AVDs (headless). No avdmanager needed.
#   native/engine/device/avd.sh create engine-api28|engine-api37-16k
#   native/engine/device/avd.sh boot   NAME PORT      # prints the serial (emulator-PORT) once booted
#   native/engine/device/avd.sh stop   PORT           # only for emulators this script booted
#   native/engine/device/avd.sh register PORT         # re-announce a running emulator to a restarted adb
#                                                     # server (it only auto-scans ports 5555-5585)
# AVDs live in $ANDROID_AVD_HOME (default <repo>/artifacts/avd). Boot is cold, -no-snapshot.
set -euo pipefail
dev_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd -- "$dev_dir/../../.." && pwd)
sdk=${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}
export ANDROID_AVD_HOME=${ANDROID_AVD_HOME:-$repo/artifacts/avd}
export PATH=$sdk/platform-tools:$sdk/emulator:$PATH
logdir=$repo/artifacts/engine/device/emulator
mkdir -p "$logdir" "$ANDROID_AVD_HOME"

image_for() {
  case "$1" in
    engine-api28) echo "android-28 default x86_64 android-28" ;;
    engine-api37-16k) echo "android-37.0 google_apis_ps16k x86_64 android-37.0" ;;
    *) echo "unknown AVD $1" >&2; return 2 ;;
  esac
}

create() {
  local name=$1 api tag abi target
  read -r api tag abi target < <(image_for "$name")
  local sysdir=$sdk/system-images/$api/$tag/$abi
  [[ -f $sysdir/system.img ]] || { echo "missing system image $sysdir" >&2; exit 1; }
  local avd=$ANDROID_AVD_HOME/$name.avd
  if [[ -f $avd/config.ini ]]; then echo "exists: $avd"; return 0; fi
  mkdir -p "$avd"
  printf 'avd.ini.encoding=UTF-8\npath=%s\ntarget=%s\n' "$avd" "$target" > "$ANDROID_AVD_HOME/$name.ini"
  cat > "$avd/config.ini" <<EOF
AvdId = $name
avd.ini.encoding = UTF-8
abi.type = $abi
tag.id = ${tag%_ps16k}
image.sysdir.1 = $sysdir/
hw.cpu.arch = $abi
hw.cpu.ncore = 2
hw.ramSize = 2048
hw.lcd.width = 720
hw.lcd.height = 1280
hw.lcd.density = 320
hw.keyboard = yes
hw.gpu.enabled = yes
hw.gpu.mode = swiftshader_indirect
hw.mainKeys = no
hw.audioInput = no
disk.dataPartition.size = 6442450944
showDeviceFrame = no
fastboot.forceColdBoot = yes
EOF
  echo "created $avd"
}

boot() {
  local name=$1 port=$2 serial=emulator-$2
  [[ -f $ANDROID_AVD_HOME/$name.avd/config.ini ]] || create "$name"
  if adb devices | grep -q "^$serial\b"; then echo "$serial already present" >&2; exit 1; fi
  nohup emulator -avd "$name" -port "$port" -no-window -no-audio -no-boot-anim -no-snapshot \
    -gpu swiftshader_indirect > "$logdir/$name.log" 2>&1 &
  echo $! > "$logdir/$name.pid"
  echo "booting $name as $serial (pid $!, log $logdir/$name.log)" >&2
  adb -s "$serial" wait-for-device
  local i
  for i in $(seq 1 180); do
    [[ $(adb -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r') == 1 ]] && { echo "$serial"; return 0; }
    sleep 2
  done
  echo "$serial did not finish booting" >&2; exit 1
}

register() {
  python3 - "$(( $1 + 1 ))" <<'PY'
import socket, sys
msg = ("host:emulator:%s" % sys.argv[1]).encode()
s = socket.create_connection(("127.0.0.1", 5037))
s.sendall(b"%04x" % len(msg) + msg)
s.close()
PY
  sleep 1
  adb devices | grep -q "^emulator-$1\b"
}

stop() {
  local port=$1 serial=emulator-$1
  adb -s "$serial" emu kill || true
}

cmd=${1:-}; shift || true
case "$cmd" in
  create) create "$@" ;;
  boot) boot "$@" ;;
  stop) stop "$@" ;;
  register) register "$@" ;;
  *) sed -n '2,6p' "$0"; exit 2 ;;
esac
