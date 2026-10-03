#!/usr/bin/env python3
"""Print a pass/fail summary of one harness run directory (results.json + out/*.stderr).

    summarize.py RUN_DIR [--engine-dir artifacts/engine/android/<abi>] [--json]

Also extracts the SIGSYS-emulated syscalls from every case's debug stderr (lines
`SIGSYS tid=N syscall=NR(name) -> ENOSYS`): those are the syscalls the app seccomp filter blocks.
"""
import argparse
import hashlib
import json
import os
import re
import sys

SIGSYS = re.compile(r"SIGSYS tid=\d+ syscall=(-?\d+)\(([^)]*)\)")
PACKAGED = {
    "libworkflow-engine.so": "libworkflow-engine.so",
    "libworkflow-loader.so": "libworkflow-loader.so",
    "libwftest-sigsys.so": "test/sigsys_helper",
    "libwftest-forkstress.so": "test/fork_stress",
    "libwftest-gueststatic.so": "test/guest_static",
}


def syscall_names(abi):
    """nr -> name from the NDK's uapi headers (the engine's own table is partial)."""
    ndk = os.environ.get("ANDROID_NDK") or os.path.expanduser("~/Android/Sdk/ndk/30.0.15729638")
    arch = {"x86_64": "x86_64", "arm64-v8a": "aarch64"}.get(abi)
    hdr = os.path.join(ndk, "toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/include",
                       f"{arch}-linux-android/asm/unistd_64.h")
    names = {}
    if arch and os.path.exists(hdr):
        for m in re.finditer(r"#define __NR_(\w+) (\d+)\n", open(hdr).read()):
            names.setdefault(int(m.group(2)), m.group(1))
    return names


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run_dir")
    ap.add_argument("--engine-dir")
    ap.add_argument("--json", action="store_true", help="also write RUN_DIR/sigsys.json")
    a = ap.parse_args()
    res = json.load(open(os.path.join(a.run_dir, "results.json")))
    g = res["g0"]
    u = g.get("uname", {})
    print(f"# {g.get('model')} api={g.get('sdkInt')} ({g.get('release')}) abi={g.get('supportedAbis', ['?'])[0]} "
          f"kernel={u.get('release')} pageSize={g.get('pageSize')}")
    print(f"  selinux={g.get('selinux')}  Seccomp={g.get('seccomp')}  NoNewPrivs={g.get('noNewPrivs')}  "
          f"filters={g.get('status', {}).get('Seccomp_filters', 'n/a')}  parent={g.get('parentCmdline')}")
    print(f"  targetSdk={g.get('targetSdk')} debuggable={g.get('debuggable')} extractNativeLibs={g.get('extractNativeLibs')} "
          f"yama={g.get('yamaPtraceScope') or 'n/a'}")
    print(f"  nativeLibraryDir={g.get('nativeLibraryDir')} ({g.get('nativeLibraryDirLabel')})")
    ps_path = os.path.join(a.run_dir, "ps.txt")
    if os.path.exists(ps_path):
        procs = {}
        for line in open(ps_path):
            f = line.split()
            if len(f) >= 3 and f[0].isdigit():
                procs[int(f[0])] = f[2]
        print(f"  harness pid={g.get('pid')} ppid={g.get('ppid')} -> parent process name: {procs.get(g.get('ppid'), '?')}")
    listing = {e["name"]: e for e in g.get("nativeLibraryDirListing", [])}
    problems = []
    for name, src in PACKAGED.items():
        e = listing.get(name)
        if not e:
            problems.append(f"{name} missing")
            continue
        if not e.get("canExecute"):
            problems.append(f"{name} not executable")
        if a.engine_dir and os.path.exists(os.path.join(a.engine_dir, src)):
            if sha256(os.path.join(a.engine_dir, src)) != e.get("sha256"):
                problems.append(f"{name} sha256 differs from {src}")
    print(f"  nativeLibraryDir check: {'OK (' + str(len(PACKAGED)) + ' files, executable, sha256 match)' if not problems else '; '.join(problems)}")
    rootfs = g.get("rootfs", {})
    print(f"  rootfs exists={rootfs.get('exists')} stamp={rootfs.get('stamp', '')[:16]} label={rootfs.get('label')}")
    for s in res.get("setup", []):
        print(f"  setup copy {os.path.basename(s.get('copy', ''))} -> {s.get('to')}: {s.get('result')}")
    print(f"  run complete={res.get('complete')} summary={res.get('summary')}")
    print()

    abi = (g.get("supportedAbis") or ["?"])[0]
    names = syscall_names(abi)
    all_sigsys = {}
    probe = {}
    for c in res["cases"]:
        verdict = c["verdict"]
        line = f"{verdict:4} {c['id']:24} exit={str(c.get('exit')):5} {c.get('durationMs', 0):6}ms"
        if c.get("reasons"):
            line += "  " + "; ".join(c["reasons"])
        print(line)
        err_path = os.path.join(a.run_dir, "out", c["id"] + ".stderr")
        stderr = open(err_path, errors="replace").read() if os.path.exists(err_path) else c.get("stderrHead", "")
        found = {}
        for nr, nm in SIGSYS.findall(stderr):
            key = (int(nr), names.get(int(nr), nm))
            found[key] = found.get(key, 0) + 1
        if "filter-probe" in c.get("tags", []):
            probe[c["id"]] = (c, stderr, found)
        if found and "self-sigsys" not in c.get("tags", []) and "filter-probe" not in c.get("tags", []):
            all_sigsys[c["id"]] = found
        if verdict == "FAIL":
            shown = [l for l in stderr.splitlines() if l.strip()]
            important = [l for l in shown if not l.startswith("[engine:D") and not l.startswith("[engine:T")]
            tail = (important or shown)[-8:]
            for l in tail:
                print(f"       | {l[:200]}")
            if c.get("stdoutHead"):
                print(f"       stdout: {c['stdoutHead'][:200]!r}")
    print()
    union = {}
    for cid, found in all_sigsys.items():
        for k, n in found.items():
            union.setdefault(k, set()).add(cid)
    if union:
        print(f"SIGSYS-emulated syscalls seen in guest/pass-through workloads on {abi} (app filter trapped, engine answered ENOSYS):")
        for (nr, nm), cids in sorted(union.items()):
            print(f"  {nm}({nr})  in: {', '.join(sorted(cids))}")
        print("  = " + " ".join(nm for (nr, nm) in sorted(union)))
    else:
        print("SIGSYS-emulated syscalls: none observed (only cases run with WORKFLOW_ENGINE_LOG>=3 log them)")
    bare = probe.get("sysprobe-bare")
    blocked = None
    if bare:
        out_path = os.path.join(a.run_dir, "out", "sysprobe-bare.stdout")
        text = open(out_path).read() if os.path.exists(out_path) else bare[0].get("stdoutHead", "")
        for line in text.splitlines():
            if line.startswith("blocked:"):
                blocked = line[len("blocked:"):].split()
            if line.startswith("enosys:"):
                print(f"Filter probe: kernel lacks (ENOSYS without SIGSYS):{line[len('enosys:'):] or ' none'}")
        if blocked is not None:
            print(f"Filter probe (no engine, {abi}): BLOCKED by the app seccomp filter = {' '.join(blocked) or 'none'}")
    eng = probe.get("sysprobe-engine")
    if eng and blocked is not None:
        seen = sorted({nm for (_, nm) in eng[2]})
        verdict = "match" if seen == sorted(blocked) else f"MISMATCH (engine SIGSYS lines: {' '.join(seen)})"
        print(f"Engine SIGSYS->ENOSYS for the same probe: {verdict}")
    if a.json:
        with open(os.path.join(a.run_dir, "sigsys.json"), "w") as f:
            json.dump({cid: [{"nr": nr, "name": nm, "count": n} for (nr, nm), n in sorted(found.items())]
                       for cid, found in all_sigsys.items()}, f, indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
