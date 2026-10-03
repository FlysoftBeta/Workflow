#!/usr/bin/env python3
"""Generate Rust syscall tables from pinned Linux ABI tables and explicit policy.
Run with --check in CI; any unclassified or stale syscall is an error.
"""
import hashlib, os, re, sys
from pathlib import Path
LINUX_TAG = "v7.2.8"
LINUX_COMMIT = "9a66fdc0d7fd55f54235524a73435af99051e46f"   # v7.2.8^{} in linux-stable
SRC_DIR = "src/linux-" + LINUX_TAG
SOURCES = {  # path inside SRC_DIR -> sha256 of the verbatim upstream file
    "arch/x86/entry/syscalls/syscall_64.tbl":
        "93e42d351de2002418bf499b9a538f189fd683e8ecd52b4adc8a0bbe59130455",
    "scripts/syscall.tbl":
        "222c40f91975eb2860bf4d334863005ef084fee39dd031d252ad2f2f2342d485",
    "arch/arm64/kernel/Makefile.syscalls":
        "27e95d7bd10c3ab67c8a3f16c7dfe5da30c586c2fda3993c42025109a22098c5",
}

# Order == enum order in sysinv.h (brief API); UNKNOWN is never a valid TSV class.
CLASSES = ["UNKNOWN", "PASS", "PATH", "FD", "ID", "META", "EXEC", "PROC", "SOCK", "ENOSYS", "EPERM"]
# Rule precedence (first match wins) -- documented, and used to order coverage.md summaries.
PRECEDENCE = ["EXEC", "ENOSYS", "EPERM", "PATH", "ID", "META", "SOCK", "PROC", "FD", "PASS"]

ABIS = ("x86_64", "aarch64")
ABI_ID = {"x86_64": 1, "aarch64": 2}

# Table name -> name used by uapi headers, when they differ.  Empty for v7.2.8: the generic
# table's 64-bit rows already use x86_64's names (79 newfstatat, 80 fstat); the "stat64" rows
# (fstatat64/fstat64) are 32-bit only and not selected for arm64.
HEADER_ALIASES = {"aarch64": {}, "x86_64": {}}
# Macros in unistd headers that are not syscalls.
HEADER_NON_SYSCALLS = {"syscalls", "arch_specific_syscall"}

HERE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "inventory")
ENGINE = os.path.dirname(HERE)
NDK_DEFAULT = os.path.expanduser("~/Android/Sdk/ndk/30.0.15729638")


def die(msg):
    sys.stderr.write("gensysinv: error: " + msg + "\n")
    sys.exit(1)


def read_source(rel):
    path = os.path.join(HERE, SRC_DIR, rel)
    with open(path, "rb") as f:
        data = f.read()
    got = hashlib.sha256(data).hexdigest()
    if got != SOURCES[rel]:
        die("%s: sha256 %s does not match the pinned %s" % (path, got, SOURCES[rel]))
    return data.decode("utf-8")


def parse_tbl(text, abis, label):
    """Return {name: (nr, has_entry)} for rows whose ABI column is in `abis`."""
    by_name, by_nr = {}, {}
    for lineno, line in enumerate(text.splitlines(), 1):
        f = line.split()
        if not f or f[0].startswith("#"):
            continue
        if len(f) < 3:
            die("%s:%d: malformed row" % (label, lineno))
        nr, abi, name = int(f[0]), f[1], f[2]
        if abi not in abis:
            continue
        entry = f[3] if len(f) > 3 else ""
        has_entry = bool(entry) and entry != "sys_ni_syscall"
        if name in by_name:
            die("%s:%d: duplicate name %s" % (label, lineno, name))
        if nr in by_nr:
            die("%s:%d: nr %d assigned to both %s and %s" % (label, lineno, nr, by_nr[nr], name))
        by_name[name] = (nr, has_entry)
        by_nr[nr] = name
    return by_name


def load_tables():
    x86 = parse_tbl(read_source("arch/x86/entry/syscalls/syscall_64.tbl"),
                    {"common", "64"}, "syscall_64.tbl")          # x32 rows excluded
    mk = read_source("arch/arm64/kernel/Makefile.syscalls")
    extra = []
    for line in mk.splitlines():
        m = re.match(r"\s*syscall_abis_64\s*\+=\s*(.*)$", line)
        if m:
            extra += m.group(1).split()
    if not extra:
        die("arm64 Makefile.syscalls: no syscall_abis_64 found")
    a64 = parse_tbl(read_source("scripts/syscall.tbl"),
                    {"common", "64"} | set(extra), "scripts/syscall.tbl")
    return {"x86_64": x86, "aarch64": a64}, sorted(extra)


def load_tsv():
    path = os.path.join(HERE, "syscalls.tsv")
    rows = {}
    with open(path, encoding="utf-8") as f:
        for lineno, line in enumerate(f, 1):
            line = line.rstrip("\n")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            f3 = line.split("\t")
            if len(f3) != 3:
                die("syscalls.tsv:%d: want 3 TAB-separated columns, got %d" % (lineno, len(f3)))
            name, cls, note = (x.strip() for x in f3)
            if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
                die("syscalls.tsv:%d: bad name %r" % (lineno, name))
            if cls not in CLASSES or cls == "UNKNOWN":
                die("syscalls.tsv:%d: bad class %r" % (lineno, cls))
            if cls != "PASS" and not note:
                die("syscalls.tsv:%d: %s needs a note (class %s)" % (lineno, name, cls))
            if "|" in note:
                die("syscalls.tsv:%d: note must not contain '|' (markdown table)" % lineno)
            if name in rows:
                die("syscalls.tsv:%d: duplicate row for %s" % (lineno, name))
            rows[name] = (cls, note)
    return rows


def validate(tables, rows):
    names = set()
    for t in tables.values():
        names |= set(t)
    missing = sorted(names - set(rows))
    stale = sorted(set(rows) - names)
    if missing:
        die("syscalls.tsv lacks rows for: " + " ".join(missing))
    if stale:
        die("syscalls.tsv has rows for syscalls absent from both tables: " + " ".join(stale))
    # A syscall with no kernel entry point on some ABI can only be ENOSYS.
    for abi, t in tables.items():
        for name, (nr, has_entry) in t.items():
            if not has_entry and rows[name][0] != "ENOSYS":
                die("%s %s (nr %d) has no kernel entry point but is classed %s"
                    % (abi, name, nr, rows[name][0]))



def main():
    tables, _ = load_tables()
    rows = load_tsv()
    validate(tables, rows)
    out = Path(__file__).resolve().parents[1] / "src" / "syscalls.rs"
    text = "// Generated by engine/environment/runtime/tools/generate-syscalls.py; Linux " + LINUX_TAG + ".\n"
    for abi in ABIS:
        by_nr = {nr: (name, CLASSES.index(rows[name][0])) for name, (nr, _) in tables[abi].items()}
        text += f"const {abi.upper()}: [Entry; {max(by_nr)+1}] = [\n"
        for nr in range(max(by_nr)+1):
            if nr in by_nr:
                name, cls = by_nr[nr]
                text += f'    Entry {{ name: Some(c"{name}"), class: {cls} }}, // {nr}\n'
            else:
                text += f'    Entry {{ name: None, class: 0 }}, // {nr}\n'
        text += "];\n"
    if "--check" in sys.argv:
        if out.read_text() != text:
            die("Rust syscall tables are stale; run tools/generate-syscalls.py")
    else:
        out.write_text(text)
    print("syscall inventory: pinned hashes and exhaustive classification verified")

if __name__ == "__main__":
    main()
