#!/usr/bin/env python3
"""Frozen per-run driver, called only inside tools/with-emulator.sh's device lease."""
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys


def instrumentation_result(text: str, allow_skips: bool = False) -> dict:
    match = re.search(r"^OK \((\d+) tests?\)\s*$", text, re.MULTILINE)
    skipped = len(re.findall(r"^INSTRUMENTATION_STATUS_CODE: -[34]\s*$", text, re.MULTILINE))
    if not match or "FAILURES!!!" in text or "Process crashed" in text or int(match[1]) == 0:
        raise RuntimeError("Instrumentation did not report a successful non-empty suite")
    if skipped and not allow_skips:
        raise RuntimeError(f"Instrumentation skipped {skipped} tests; explicit opt-in is required")
    return {"tests": int(match[1]), "skipped": skipped}


def main():
    manifest = Path(sys.argv[1]); run = json.loads(manifest.read_text()); output = manifest.parent
    serial = os.environ.get("ANDROID_SERIAL", "")
    if not re.fullmatch(r"emulator-\d+", serial):
        raise RuntimeError("Only the wrapper-owned disposable emulator is admitted")
    def adb(*args, timeout=60, check=True):
        return subprocess.run(["adb", "-s", serial, *args], stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, timeout=timeout, check=check).stdout
    hardware = adb("shell", "getprop", "ro.hardware").decode().strip()
    if hardware not in {"ranchu", "goldfish"}:
        raise RuntimeError("Refusing non-emulator hardware")
    if run["rooted"]:
        adb("root"); adb("wait-for-device")
        if adb("shell", "id", "-u").decode().strip() != "0":
            raise RuntimeError("The requested emulator root channel is unavailable")
    for name in ["app", "test"]:
        (output / f"install-{name}.log").write_bytes(adb("install", "-r", run["apks"][name]["file"], timeout=180))
    # This test run owns the read-only emulator overlay and its test application's data.
    adb("shell", "pm", "clear", "top.flysoftbeta.workflow")
    adb("logcat", "-c")
    args = ["am", "instrument", "-w", "-r"]
    if run["classes"]:
        args += ["-e", "class", ",".join(run["classes"])]
    for value in run["arguments"]:
        key, val = value.split("=", 1); args += ["-e", key, val]
    args += ["top.flysoftbeta.workflow.test/androidx.test.runner.AndroidJUnitRunner"]
    try:
        text = adb("shell", shlex.join(args), timeout=1800).decode(errors="replace")
        (output / "instrumentation.log").write_text(text)
        print(text)
        result = instrumentation_result(text, run["allowSkips"])
        result.update(serial=serial, hardware=hardware, api=adb("shell", "getprop", "ro.build.version.sdk").decode().strip())
        (output / "device-result.json").write_text(json.dumps(result, indent=2) + "\n")
    finally:
        (output / "logcat.txt").write_bytes(adb("logcat", "-d", timeout=30, check=False))


if __name__ == "__main__":
    main()
