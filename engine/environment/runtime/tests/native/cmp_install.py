#!/usr/bin/env python3
"""Compare an engine-installed generation with the wfimage reference install.

    cmp_install.py ENGINE_GEN REFERENCE_GEN [--rootfs-only]

The reference keeps ownership in <gen>/attributes.tsv and real hardlinks; the
engine keeps it in user.workflow.meta xattrs and emulates hardlinks with stubs
into rootfs/.workflow-engine/links.  For every attributes row this checks type,
uid, gid, mode, NLINK (hardlinks), device numbers, content (sha256), symlink
text, mtime; seeds/ must match byte for byte including host permission bits.
Prints the first differences and a summary; exit 0 if identical."""
import hashlib
import os
import stat
import sys

XATTR = "user.workflow.meta"
LINKS = b"/.workflow-engine/links/"


def unpct(b):
    out, i = bytearray(), 0
    while i < len(b):
        if b[i] == 0x25:
            out.append(int(b[i + 1:i + 3], 16)); i += 3
        else:
            out.append(b[i]); i += 1
    return bytes(out)


def digest(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(1 << 20):
            h.update(chunk)
    return h.hexdigest()


def main(eng, ref, rootfs_only=False):
    eng, ref = os.fsencode(eng), os.fsencode(ref)
    rows = []
    with open(os.path.join(ref, b"attributes.tsv"), "rb") as f:
        lines = f.read().split(b"\n")
    for line in lines[1:]:
        if not line:
            continue
        p, t, u, g, m, x = line.split(b"\t")
        rows.append((unpct(p), t.decode(), int(u), int(g), int(m, 8), x))
    stores = []
    import json
    meta = json.load(open(os.path.join(ref, b"metadata.json")))
    for gp, spec in (meta.get("stores") or {}).items():
        stores.append((os.fsencode(gp), os.fsencode(spec["store"])))
    nlinks = {}
    for p, t, u, g, m, x in rows:
        if t == "h":
            prim = unpct(x)
            nlinks[prim] = nlinks.get(prim, 1) + 1
    diffs = []
    checked = 0

    def diff(msg):
        diffs.append(msg)

    for p, t, u, g, m, x in rows:
        seed = next(((gp, st) for gp, st in stores if p.startswith(gp + b"/")), None)
        if seed and rootfs_only:
            continue
        if seed:
            rel = seed[1] + p[len(seed[0]):]
            e = os.path.join(eng, b"seeds", rel)
            r = os.path.join(ref, b"seeds", rel)
            es, rs = os.lstat(e), os.lstat(r)
            if stat.S_IFMT(es.st_mode) != stat.S_IFMT(rs.st_mode) or (es.st_mode & 0o7777) != (rs.st_mode & 0o7777):
                diff("seed %s: mode %o vs %o" % (p, es.st_mode, rs.st_mode))
            if not stat.S_ISDIR(es.st_mode) and es.st_mtime_ns // 10**9 != rs.st_mtime_ns // 10**9:
                diff("seed %s: mtime" % p)
            if stat.S_ISREG(es.st_mode) and digest(e) != digest(r):
                diff("seed %s: content" % p)
            if stat.S_ISLNK(es.st_mode) and os.readlink(e) != os.readlink(r):
                diff("seed %s: link" % p)
            checked += 1
            continue
        e = os.path.join(eng, b"rootfs") + (p if p != b"/" else b"")
        r = os.path.join(ref, b"rootfs") + (p if p != b"/" else b"")
        es = os.lstat(e)
        obj = e
        if stat.S_ISLNK(es.st_mode) and os.readlink(e).startswith(LINKS):
            obj = os.path.join(eng, b"rootfs") + os.readlink(e)
            es = os.lstat(obj)
        if t == "l":
            if not stat.S_ISLNK(es.st_mode) or os.readlink(e) != os.readlink(r):
                diff("%s: symlink differs" % p)
            if os.lstat(e).st_mtime_ns // 10**9 != os.lstat(r).st_mtime_ns // 10**9:
                diff("%s: symlink mtime" % p)
            checked += 1
            continue
        try:
            v = os.getxattr(obj, XATTR, follow_symlinks=False).decode().split()
        except OSError:
            diff("%s: no metadata xattr" % p)
            continue
        ver, uid, gid, mode, nl, dev = v
        uid, gid, mode, nl = int(uid), int(gid), int(mode, 8), int(nl)
        kind = {"d": stat.S_IFDIR, "f": stat.S_IFREG, "h": stat.S_IFREG, "c": stat.S_IFCHR, "b": stat.S_IFBLK,
                "p": stat.S_IFIFO}[t]
        if (uid, gid, mode) != (u, g, kind | m):
            diff("%s: meta %d %d %o, want %d %d %o" % (p, uid, gid, mode, u, g, kind | m))
        want_nl = nlinks.get(p) or (nlinks.get(unpct(x)) if t == "h" else None) or 0
        if want_nl and nl != want_nl:
            diff("%s: nlink %d want %d" % (p, nl, want_nl))
        if t in "cb" and dev != x.decode():
            diff("%s: device %s want %s" % (p, dev, x.decode()))
        if t in "fh":
            if digest(obj) != digest(r):
                diff("%s: content" % p)
            if es.st_mtime_ns // 10**9 != os.lstat(r).st_mtime_ns // 10**9:
                diff("%s: mtime" % p)
            if es.st_mode & 0o7000:
                diff("%s: host set-id bits" % p)
        if t == "d" and es.st_mtime_ns // 10**9 != os.lstat(r).st_mtime_ns // 10**9:
            diff("%s: dir mtime" % p)
        checked += 1
    for d in diffs[:20]:
        print("DIFF", d)
    print("compared %d rows: %d difference(s)" % (checked, len(diffs)))
    return 1 if diffs else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2], "--rootfs-only" in sys.argv[3:]))
