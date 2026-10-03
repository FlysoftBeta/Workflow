#!/usr/bin/env python3
"""Generate the engine's syscall inventory (python3 stdlib only).

Inputs (all under native/engine/gen/):
  src/linux-<TAG>/...   verbatim kernel syscall tables, pinned by sha256 below
  syscalls.tsv          hand-maintained classification: name<TAB>class<TAB>note

Outputs (deterministic; no timestamps, stable ordering):
  include/engine/sysinv.h, src/sysinv.c, gen/coverage.md

Usage:
  gensysinv.py [--out DIR]            validate + generate (DIR defaults to native/engine)
  gensysinv.py --check-headers        also cross-check NDK r30 + host <asm/unistd_64.h>
  gensysinv.py --verify-dump ABI FILE compare a `sysinv_test --dump` listing with the tables
"""
import argparse
import hashlib
import os
import re
import sys

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

HERE = os.path.dirname(os.path.abspath(__file__))
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


def ranges(nums):
    out, start, prev = [], None, None
    for n in sorted(nums):
        if start is None:
            start = prev = n
        elif n == prev + 1:
            prev = n
        else:
            out.append((start, prev))
            start = prev = n
    if start is not None:
        out.append((start, prev))
    return out


def gaps(t):
    used = {nr for nr, _ in t.values()}
    return ranges(set(range(max(used) + 1)) - used)


def fmt_ranges(rs):
    return ", ".join(str(a) if a == b else "%d-%d" % (a, b) for a, b in rs) or "none"


def counts(t, rows):
    c = {k: 0 for k in PRECEDENCE}
    for name in t:
        c[rows[name][0]] += 1
    return c


BANNER = ("GENERATED by native/engine/gen/gensysinv.py from Linux %s (linux-stable %s)\n"
          " * and native/engine/gen/syscalls.tsv.  Do not edit; run native/engine/gen/check.sh."
          % (LINUX_TAG, LINUX_COMMIT[:12]))


def gen_header():
    enum = ",\n    ".join("ENG_SC_" + c + (" = 0" if c == "UNKNOWN" else "") for c in CLASSES)
    return """/* sysinv.h -- versioned syscall inventory for the engine's guest ABI.
 *
 * %s
 *
 * One ABI per build: x86_64 (syscall_64.tbl, common+64 rows; x32 excluded) or aarch64
 * (generic scripts/syscall.tbl, common+64 plus arm64's syscall_abis_64).  Numbers outside
 * the table -- gaps, negatives, the x32 bit 0x40000000 -- are ENG_SC_UNKNOWN, which the
 * engine must refuse (default deny).  Lookups are O(1) over a dense static array.
 */
#ifndef ENGINE_SYSINV_H
#define ENGINE_SYSINV_H

#define ENG_SYSINV_LINUX "%s"
#define ENG_SYSINV_ABI_X86_64 1
#define ENG_SYSINV_ABI_AARCH64 2
#define ENG_SC_COUNT %d   /* number of eng_sc_class values */

typedef enum {
    %s
} eng_sc_class;

const char  *eng_sysinv_name(long nr);   /* NULL if nr is not a syscall on this ABI */
eng_sc_class eng_sysinv_class(long nr);  /* ENG_SC_UNKNOWN for non-syscalls */
long         eng_sysinv_max(void);       /* highest assigned number on this ABI */
const char  *eng_sysinv_class_name(eng_sc_class c);   /* "PATH", ...; "INVALID" if out of range */
const char  *eng_sysinv_abi(void);       /* "x86_64" or "aarch64" */

#endif
""" % (BANNER, LINUX_TAG, len(CLASSES), enum)


def gen_c(tables, rows):
    parts = ["""/* sysinv.c -- syscall inventory tables (see engine/sysinv.h).
 *
 * %s
 *
 * ENG_SYSINV_FORCE_ABI (1 = x86_64, 2 = aarch64) is a test-only hook that selects a table
 * on any host so gen/check.sh can dump and verify the other ABI's data; production builds
 * never define it and select by __x86_64__ / __aarch64__.
 */
#include "engine/sysinv.h"

#include <stddef.h>

#if defined(ENG_SYSINV_FORCE_ABI)
#define SYSINV_ABI ENG_SYSINV_FORCE_ABI
#elif defined(__x86_64__)
#define SYSINV_ABI ENG_SYSINV_ABI_X86_64
#elif defined(__aarch64__)
#define SYSINV_ABI ENG_SYSINV_ABI_AARCH64
#else
#error "sysinv: the engine supports x86_64 and aarch64 guests only"
#endif

typedef struct {
    const char *name;
    unsigned char cls;   /* eng_sc_class */
} sysinv_ent;
""" % BANNER]
    first = True
    for abi in ABIS:
        t = tables[abi]
        mx = max(nr for nr, _ in t.values())
        c = counts(t, rows)
        parts.append("\n#%s SYSINV_ABI == ENG_SYSINV_ABI_%s\n" % ("if" if first else "elif", abi.upper()))
        first = False
        parts.append("/* %s: %d syscalls, max %d, unassigned: %s.\n * %s */\n" % (
            abi, len(t), mx, fmt_ranges(gaps(t)),
            " ".join("%s %d" % (k, c[k]) for k in PRECEDENCE)))
        parts.append('#define SYSINV_ABI_NAME "%s"\n#define SYSINV_MAX %d\n' % (abi, mx))
        parts.append("static const sysinv_ent TABLE[SYSINV_MAX + 1] = {\n")
        for name, (nr, _) in sorted(t.items(), key=lambda kv: kv[1][0]):
            parts.append('    [%d] = {"%s", ENG_SC_%s},\n' % (nr, name, rows[name][0]))
        parts.append("};\n")
    parts.append('#else\n#error "sysinv: ENG_SYSINV_FORCE_ABI must be 1 (x86_64) or 2 (aarch64)"\n#endif\n')
    names = "\n    " + ",\n    ".join('"%s"' % c for c in CLASSES) + ",\n"
    parts.append("""
static const char *const CLASS_NAMES[ENG_SC_COUNT] = {%s};

_Static_assert(sizeof CLASS_NAMES / sizeof CLASS_NAMES[0] == ENG_SC_EPERM + 1, "class names");

const char *eng_sysinv_name(long nr) {
    if (nr < 0 || nr > SYSINV_MAX) return NULL;
    return TABLE[nr].name;
}

eng_sc_class eng_sysinv_class(long nr) {
    if (nr < 0 || nr > SYSINV_MAX) return ENG_SC_UNKNOWN;
    return (eng_sc_class)TABLE[nr].cls;
}

long eng_sysinv_max(void) { return SYSINV_MAX; }

const char *eng_sysinv_class_name(eng_sc_class c) {
    if ((unsigned)c >= ENG_SC_COUNT) return "INVALID";
    return CLASS_NAMES[c];
}

const char *eng_sysinv_abi(void) { return SYSINV_ABI_NAME; }
""" % names)
    return "".join(parts)


def gen_coverage(tables, rows, arm64_abis):
    out = ["# Engine syscall coverage\n\n",
           "Generated by `gen/gensysinv.py` from Linux %s (linux-stable `%s`) and `gen/syscalls.tsv`.\n"
           % (LINUX_TAG, LINUX_COMMIT),
           "Do not edit; run `native/engine/gen/check.sh`.\n\n",
           "- x86_64: `arch/x86/entry/syscalls/syscall_64.tbl`, ABI `common` + `64` (x32 excluded).\n",
           "- aarch64: `scripts/syscall.tbl`, ABI `common` + `64` + arm64 `syscall_abis_64` (%s).\n"
           % ", ".join("`%s`" % a for a in arm64_abis),
           "- Class precedence (first match wins): %s. `UNKNOWN` (not in the table) is refused.\n\n"
           % " > ".join(PRECEDENCE),
           "| class | x86_64 | aarch64 |\n| --- | ---: | ---: |\n"]
    cs = {abi: counts(tables[abi], rows) for abi in ABIS}
    for k in PRECEDENCE:
        out.append("| %s | %d | %d |\n" % (k, cs["x86_64"][k], cs["aarch64"][k]))
    out.append("| **total** | %d | %d |\n\n" % (len(tables["x86_64"]), len(tables["aarch64"])))
    for abi in ABIS:
        t = tables[abi]
        out.append("- %s: max nr %d; unassigned: %s.\n"
                   % (abi, max(nr for nr, _ in t.values()), fmt_ranges(gaps(t))))
    out.append("- `—` = not a syscall on that ABI; `*` = table row without a kernel entry point "
               "(sys_ni_syscall).\n\n")
    out.append("| name | x86_64 | aarch64 | class | note |\n| --- | ---: | ---: | --- | --- |\n")
    for name in sorted(rows):
        cells = []
        for abi in ABIS:
            e = tables[abi].get(name)
            cells.append("—" if e is None else "%d%s" % (e[0], "" if e[1] else "*"))
        cls, note = rows[name]
        out.append("| `%s` | %s | %s | %s | %s |\n" % (name, cells[0], cells[1], cls, note))
    return "".join(out)


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    os.replace(tmp, path)


def parse_unistd(path):
    """#define __NR_name <int> lines -> {name: nr} (flat generated uapi headers)."""
    res = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            m = re.match(r"\s*#\s*define\s+__NR_(\w+)\s+(\d+)\b", line)
            if m:
                res[m.group(1)] = int(m.group(2))
    return res


def check_headers(tables):
    ndk = os.environ.get("ANDROID_NDK_ROOT") or os.environ.get("NDK") or NDK_DEFAULT
    inc = os.path.join(ndk, "toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/include")
    cands = [
        ("NDK aarch64", "aarch64", os.path.join(inc, "aarch64-linux-android/asm/unistd_64.h")),
        ("NDK x86_64", "x86_64", os.path.join(inc, "x86_64-linux-android/asm/unistd_64.h")),
        ("host x86_64", "x86_64", "/usr/include/asm/unistd_64.h"),
    ]
    bad = 0
    for label, abi, path in cands:
        if not os.path.exists(path):
            print("headers: %-11s skipped (no %s)" % (label, path))
            continue
        hdr = parse_unistd(path)
        alias = HEADER_ALIASES[abi]
        t = {alias.get(n, n): nr for n, (nr, _) in tables[abi].items()}
        mism = sorted((n, hdr[n], t[n]) for n in hdr if n in t and hdr[n] != t[n])
        hdr_only = sorted(n for n in hdr if n not in t and n not in HEADER_NON_SYSCALLS)
        newer = sorted((t[n], n) for n in t if n not in hdr)
        print("headers: %-11s %s: %d names; mismatched numbers %d, header-only %d, newer than header %d"
              % (label, path, len(hdr), len(mism), len(hdr_only), len(newer)))
        for n, h, k in mism:
            print("  MISMATCH %s: header %d, kernel %s table %d" % (n, h, LINUX_TAG, k))
        if hdr_only:
            print("  HEADER-ONLY " + " ".join(hdr_only))
        if newer:
            print("  newer (ok): " + " ".join("%s=%d" % (n, nr) for nr, n in newer))
        bad += len(mism) + len(hdr_only)
    return bad


def verify_dump(tables, rows, abi, path):
    t = tables[abi]
    by_nr = {nr: name for name, (nr, _) in t.items()}
    mx = max(by_nr)
    want = ["abi\t%s" % abi, "max\t%d" % mx]
    for nr in range(mx + 1):
        name = by_nr.get(nr)
        want.append("%d\t%s\t%s" % (nr, name or "-", rows[name][0] if name else "UNKNOWN"))
    with open(path, encoding="utf-8") as f:
        got = [line.rstrip("\n") for line in f]
    if got != want:
        for i, (g, w) in enumerate(zip(got + [""] * len(want), want + [""] * len(got))):
            if g != w:
                die("dump %s line %d: got %r, want %r" % (path, i + 1, g, w))
    print("dump %s: %d entries match %s table + syscalls.tsv" % (abi, mx + 1, LINUX_TAG))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", default=ENGINE, help="engine root to write into")
    ap.add_argument("--check-headers", action="store_true")
    ap.add_argument("--verify-dump", nargs=2, metavar=("ABI", "FILE"))
    a = ap.parse_args()
    tables, arm64_abis = load_tables()
    rows = load_tsv()
    validate(tables, rows)
    if a.verify_dump:
        abi, path = a.verify_dump
        if abi not in tables:
            die("unknown ABI " + abi)
        verify_dump(tables, rows, abi, path)
        return 0
    write(os.path.join(a.out, "include/engine/sysinv.h"), gen_header())
    write(os.path.join(a.out, "src/sysinv.c"), gen_c(tables, rows))
    write(os.path.join(a.out, "gen/coverage.md"), gen_coverage(tables, rows, arm64_abis))
    for abi in ABIS:
        c = counts(tables[abi], rows)
        print("%-7s %3d syscalls: %s" % (abi, len(tables[abi]),
                                         " ".join("%s=%d" % (k, c[k]) for k in PRECEDENCE)))
    if a.check_headers and check_headers(tables):
        die("header cross-check found disagreements (see above)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
