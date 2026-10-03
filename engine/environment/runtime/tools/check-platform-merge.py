#!/usr/bin/env python3
"""Compare cfg-selected runtime items with the retained pre-extraction source.

Rustc performs cfg selection, include expansion and parsing for each actual target.
See MERGE_AUDIT.md for the limited ABI-preserving spelling normalizations.
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


PRIMITIVES = {
    "c_schar": "i8", "c_uchar": "u8", "c_short": "i16", "c_ushort": "u16",
    "c_int": "i32", "c_uint": "u32", "c_long": "i64", "c_ulong": "u64",
    "c_longlong": "i64", "c_ulonglong": "u64", "c_float": "f32", "c_double": "f64",
}
MODE_NAMES = {"S_IF" + suffix for suffix in ("MT", "REG", "LNK", "DIR", "CHR", "BLK", "IFO", "SOCK")}
MODE_NAMES |= {"__" + name for name in MODE_NAMES}


def closing(ts, start):
    stack = []
    for i in range(start, len(ts)):
        if ts[i] in ("(", "[", "{"):
            stack.append(ts[i])
        elif ts[i] in (")", "]", "}"):
            assert stack.pop() == {")": "(", "]": "[", "}": "{"}[ts[i]]
            if not stack:
                return i
    raise ValueError("Unbalanced Rust tokens")


def strip_docs(ts):
    out = []
    i = 0
    while i < len(ts):
        bracket = i + (2 if ts[i:i + 2] == ["#", "!"] else 1)
        if ts[i] == "#" and ts[bracket:bracket + 2] == ["[", "doc"]:
            i = closing(ts, bracket) + 1
        else:
            out.append(ts[i])
            i += 1
    return out


def scalar_types(ts, aliases):
    out = []
    i = 0
    prefix = [":", ":", "core", ":", ":", "ffi", ":", ":"]
    while i < len(ts):
        if ts[i:i + 8] == prefix and i + 8 < len(ts) and ts[i + 8] in PRIMITIVES:
            out.append(PRIMITIVES[ts[i + 8]])
            i += 9
        else:
            out.extend(aliases.get(ts[i], [ts[i]]))
            i += 1
    return out


def canonical(source):
    """Compare complete selected items after the narrowly audited spelling changes.

    LP64 libc scalar aliases and header-only transparent enums are expanded, but
    c_char, aggregate layouts, function signatures, values and executable control
    flow are retained. Empty ABI constructors are substituted at their call sites.
    The original and current expressions are still compared, field for field.
    """
    ts = tokens(source)
    start = ts.index("runtime") + 2
    assert ts[start - 1] == "{"
    entries = []
    for item in items(strip_docs(ts[start:-1])):
        if item[:4] == ["unsafe", "extern", '"C"', "{"]:
            entries.extend((True, f) for f in items(item[4:-1]))
        else:
            entries.append((False, item))

    aliases = {}
    for _, item in entries:
        if item[:2] == ["pub", "type"]:
            aliases[item[2]] = scalar_types(item[4:-1], {})
    for _ in range(len(aliases)):
        aliases = {k: scalar_types(v, aliases) for k, v in aliases.items()}
    aliases = {k: v for k, v in aliases.items()
               if len(v) == 1 and re.fullmatch(r"[ui](?:8|16|32|64|128|size)|f(?:32|64)", v[0])}
    entries = [(foreign, scalar_types(item, aliases)) for foreign, item in entries
               if not (item[:2] == ["pub", "type"] and item[2] in aliases)]

    wrappers = {}
    for _, item in entries:
        for i, token in enumerate(item):
            if token == "struct" and i + 5 < len(item):
                name = item[i + 1]
                if (name.startswith("C2Rust_Unnamed") or name == "__ptrace_request") and item[i + 2:i + 4] == ["(", "pub"]:
                    assert item[i + 4] in ("u32", "i32") and "transparent" in item
                    wrappers[name] = item[i + 4]
    expanded = []
    for foreign, item in entries:
        if "automatically_derived" in item:
            continue  # layout/fields are compared; derive implementations follow them
        if any(item[i:i + 2] == ["struct", name] for name in wrappers for i in range(len(item))):
            continue
        if item[:1] == ["impl"] and item[1] in wrappers:
            for const in items(item[3:-1]):
                assert const[:2] == ["pub", "const"]
                assert const[4:8] == ["Self", "=", "Self", "("]
                expanded.append((False, ["pub", "const", const[2], ":", wrappers[item[1]], "="] + const[8:-2] + [";"]))
            continue
        out = []
        i = 0
        while i < len(item):
            if item[i] in wrappers and item[i + 1:i + 3] == [":", ":"]:
                out.append(item[i + 3])
                i += 4
                if item[i:i + 2] == [".", "0"]:
                    i += 2
            else:
                out.append(wrappers.get(item[i], item[i]))
                i += 1
        expanded.append((foreign, out))
    entries = expanded

    errno_symbol = None
    helpers = {}
    modes = {}
    for foreign, item in entries:
        if foreign and "link_name" in item and "errno" in item:
            errno_symbol = item[item.index("link_name") + 2].strip('"')
            assert errno_symbol in ("__errno", "__errno_location")
        if "fn" in item:
            name = item[item.index("fn") + 1]
            if name.startswith("platform_empty_"):
                body = item[item.index("{") + 1:-1]
                assert "return" not in body and "unsafe" not in body and "fn" not in body
                assert "(" not in body, "Empty aggregate helper must have no calls"
                helpers[name] = body
        if item[:2] == ["pub", "const"] and item[2] in MODE_NAMES:
            assert item[4] == "i32"
            modes[item[2]] = item[6:-1]

    def mode_value(name, seen=()):
        assert name not in seen
        rhs = modes[name]
        if len(rhs) == 1 and rhs[0] in modes:
            return mode_value(rhs[0], (*seen, name))
        assert len(rhs) == 1 or rhs[1:] == ["as", "i32"], rhs
        return str(int(rhs[0], 0))

    result = []
    for foreign, item in entries:
        if item[:2] == ["pub", "const"] and item[2] in modes:
            continue
        if "fn" in item and item[item.index("fn") + 1] in helpers:
            continue
        if foreign and "link_name" in item and "errno" in item:
            a = item.index("link_name") - 2
            b = closing(item, a + 1)
            item = item[:a] + item[b + 1:]
        out = []
        i = 0
        while i < len(item):
            t = item[i]
            if t in helpers and item[i + 1:i + 3] == ["(", ")"]:
                out.extend(helpers[t])
                i += 3
                continue
            if t in modes:
                t = mode_value(t)
            if t == "errno" and errno_symbol:
                t = errno_symbol
            if re.fullmatch(r"0[xob][0-9a-fA-F_]+|[0-9]+", t):
                t = str(int(t, 0 if t.startswith(("0x", "0o", "0b")) else 10))
            out.append(t)
            i += 1
        # A typed const-pointer binding and is_null need no explicit const cast
        # after these C search calls. Keep memchr's distinct void-to-char cast.
        i = 0
        while i < len(out) - 1:
            if out[i] in ("strchr", "strrchr", "strstr", "memchr") and out[i + 1] == "(":
                end = closing(out, i + 1)
                kind = "c_void" if out[i] == "memchr" else "c_char"
                cast = ["as", "*", "const", ":", ":", "core", ":", ":", "ffi", ":", ":", kind]
                if out[end + 1:end + 1 + len(cast)] == cast:
                    del out[end + 1:end + 1 + len(cast)]
                if i and out[i - 1] == "(" and out[end + 1:end + 2] == [")"]:
                    del out[end + 1]
                    del out[i - 1]
                    i -= 1
            i += 1
        if foreign:
            out = ["_" if i + 1 < len(out) and out[i + 1] == ":"
                   and (i + 2 == len(out) or out[i + 2] != ":")
                   and i and out[i - 1] in ("(", ",") else t for i, t in enumerate(out)]
        out = [t for i, t in enumerate(out) if not (t == "," and out[i + 1:i + 2] in (["}"], [">"]))]
        result.append(("extern " if foreign else "") + " ".join(out))
    return Counter(result)

def expand(path, target, scratch):
    header = (RUNTIME / "src/main.rs").read_text().split("#[cfg", 1)[0]
    header = header.replace("extern crate zstd_sys;", "extern crate self as libc;")
    harness = scratch / "expand.rs"
    harness.write_text(header + '\nfn getpid() {}\nmod logging { pub fn enabled() {} pub fn dprintf() {} }\n'
                       + f'#[path = "{path}"] mod runtime;\n')
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
                            difflib.unified_diff([s + "\n" for s in sorted(before.elements())],
                                                 [s + "\n" for s in sorted(after.elements())],
                                                 fromfile="baseline", tofile="current",
                                                 lineterm="\n")))
                else:
                    print(f"PASS {tree}/{module}")
    print(f"{len(TARGETS) * len(args.modules)} target/module comparisons; {failed} unexplained differences")
    return bool(failed)


if __name__ == "__main__":
    raise SystemExit(main())
