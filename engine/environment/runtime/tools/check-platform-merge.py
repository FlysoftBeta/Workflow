#!/usr/bin/env python3
"""Compare cfg-selected runtime items with the retained pre-extraction source.

Rustc performs cfg selection, include expansion and parsing for each actual target.
Only item order, whitespace, comments and foreign argument names are ignored.
This checks source equivalence, not execution or the correctness of the baseline.
It uses the pinned compiler's expansion mode; no linking, Cargo or device is used.
"""

import argparse
from collections import Counter
import difflib
import os
from pathlib import Path
import re
import subprocess
import tempfile

BASE = "a2cf9908e6d4f82315bf741d5698a6898281cd25"
RUNTIME = Path(__file__).resolve().parents[1]
REPO = RUNTIME.parents[2]
TARGETS = {
    "host": "x86_64-unknown-linux-gnu",
    "android_x86_64": "x86_64-linux-android",
    "android_aarch64": "aarch64-linux-android",
}
MODULES = "arch cli exec guest ident install json mem meta path scratch sha256 sys tracer".split()
TOKEN = re.compile(
    r'//[^\n]*|/\*.*?\*/|(?:b|c)?"(?:\\.|[^"\\])*"|'
    r"(?:b)?'(?:\\.|[^'\\])'|(?:r#)?[A-Za-z_][A-Za-z_0-9]*|"
    r'[0-9][A-Za-z_0-9]*|\s+|.', re.S
)


def tokens(source):
    return [m[0] for m in TOKEN.finditer(source)
            if not m[0].isspace() and not m[0].startswith(("//", "/*"))]


def items(ts):
    """Split Rust items at balanced delimiters; reject incomplete input."""
    start = 0
    stack = []
    for i, t in enumerate(ts):
        if t in ("(", "[", "{"):
            stack.append(t)
        elif t in (")", "]", "}"):
            assert stack.pop() == {")": "(", "]": "[", "}": "{"}[t]
        if not stack and (t == ";" or t == "}"):
            yield ts[start:i + 1]
            start = i + 1
    assert not stack and start == len(ts), ts[start:start + 20]


def canonical(source):
    ts = tokens(source)
    start = ts.index("runtime") + 2
    assert ts[start - 1] == "{"
    ts = ts[start:-1]  # the harness contains only this module after its preamble
    result = []
    for item in items(ts):
        if item[:4] == ["unsafe", "extern", '"C"', "{"]:
            for foreign in items(item[4:-1]):
                # Foreign parameter identifiers have no effect on calls or ABI.
                foreign = ["_" if i + 1 < len(foreign) and foreign[i + 1] == ":"
                           and (i + 2 == len(foreign) or foreign[i + 2] != ":")
                           and i and foreign[i - 1] in ("(", ",") else t
                           for i, t in enumerate(foreign)]
                result.append("extern " + " ".join(foreign))
        else:
            result.append(" ".join(item))
    return Counter(result)


def expand(path, target, scratch):
    header = (RUNTIME / "src/main.rs").read_text().split("#[cfg", 1)[0]
    header = header.replace("extern crate zstd_sys;", "extern crate self as libc;")
    harness = scratch / "expand.rs"
    harness.write_text(header + f'\n#[path = "{path}"] mod runtime;\n')
    run = subprocess.run(
        ["rustc", "--edition=2024", "--crate-type=lib", "--target", target,
         "-A", "warnings", "-Zunpretty=expanded", str(harness)],
        env={**os.environ, "RUSTC_BOOTSTRAP": "1"}, text=True, capture_output=True,
    )
    if run.returncode:
        raise RuntimeError(run.stderr)
    return run.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=BASE)
    parser.add_argument("--modules", nargs="+", choices=MODULES, default=MODULES)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    failed = 0
    with tempfile.TemporaryDirectory(prefix="runtime-equivalence-") as directory:
        scratch = Path(directory)
        for tree, target in TARGETS.items():
            for module in args.modules:
                rel = f"engine/environment/runtime/src/{tree}/{module}.rs"
                original = subprocess.check_output(
                    ["git", "show", f"{args.base}:{rel}"], cwd=REPO, text=True)
                baseline = scratch / "baseline.rs"
                baseline.write_text(original)
                current = RUNTIME / "src" / tree / f"{module}.rs"
                if not current.exists():
                    current = RUNTIME / "src" / f"{module}.rs"
                before = canonical(expand(baseline, target, scratch))
                after = canonical(expand(current, target, scratch))
                if before != after:
                    failed += 1
                    print(f"FAIL {tree}/{module}")
                    if args.output:
                        args.output.mkdir(parents=True, exist_ok=True)
                        (args.output / f"{tree}-{module}.diff").write_text("".join(
                            difflib.unified_diff(sorted(before.elements()),
                                                 sorted(after.elements()),
                                                 fromfile="baseline", tofile="current",
                                                 lineterm="\n")))
                else:
                    print(f"PASS {tree}/{module}")
    print(f"{len(TARGETS) * len(args.modules)} target/module comparisons; {failed} unexplained differences")
    return bool(failed)


if __name__ == "__main__":
    raise SystemExit(main())
