"""Conformance fixtures for installers: one valid image covering every row type, plus invalid
variants that each break one rule. `fixtures.json` lists every file with the expected error code."""
import hashlib
import io
import json
import os
import stat
import tempfile

from . import tarstream
from .attributes import Row, check_rows, format_rows, parse_rows
from .common import ATTRIBUTES_NAME, METADATA_NAME
from .manifest import Record
from .pack import build_metadata, build_plan, pack, write_tar

BASE = {"reference": "fixture", "indexDigest": "sha256:" + "0" * 64, "manifestDigest": "sha256:" + "1" * 64,
        "platform": "linux/amd64"}
EXTRA = {"user": {"name": "work", "uid": 1000, "gid": 1000, "home": "/home/work", "shell": "/bin/bash"},
         "environment": {"PATH": "/usr/bin:/bin", "LANG": "C.UTF-8"},
         "toolchains": {}, "defaults": {"python": "3.14", "node": "24"},
         "stores": {"/home/work": {"store": "home/work", "seed": "if-absent"}},
         "provision": {"sha256": "0" * 64, "builder": "fixture"}}
LONG_DIR = "/usr/lib/" + "/".join(["deep-directory-name-%02d" % i for i in range(6)])
LONG_TARGET = "../" * 3 + "a-symlink-target-that-is-deliberately-longer-than-the-ustar-linkname-field-" + "x" * 40

# path, type, uid, gid, mode, content-or-target, extra
ENTRIES = [
    ("/", "d", 0, 0, 0o755, None),
    ("/bin", "l", 0, 0, 0o777, "usr/bin"),
    ("/dev", "d", 0, 0, 0o755, None),
    ("/dev/null", "c", 0, 0, 0o666, (1, 3)),
    ("/etc", "d", 0, 0, 0o755, None),
    ("/etc/shadow", "f", 0, 42, 0o640, b"root:*:20000:0:99999:7:::\n"),
    ("/etc/ssl", "d", 0, 0, 0o755, None),
    ("/etc/ssl/证书.pem", "f", 0, 0, 0o644, b"-----BEGIN CERTIFICATE-----\n"),
    ("/home", "d", 0, 0, 0o755, None),
    ("/home/work", "d", 1000, 1000, 0o750, None),
    ("/home/work/.profile", "f", 1000, 1000, 0o644, b"export EDITOR=nano\n"),
    ("/home/work/space %\ttab", "f", 1000, 1000, 0o600, b"odd name\n"),
    ("/run", "d", 0, 0, 0o755, None),
    ("/run/initctl", "p", 0, 0, 0o600, None),
    ("/tmp", "d", 0, 0, 0o1777, None),
    ("/usr", "d", 0, 0, 0o755, None),
    ("/usr/bin", "d", 0, 0, 0o755, None),
    ("/usr/bin/perl", "f", 0, 0, 0o755, b"\x7fELF perl\n"),
    ("/usr/bin/perl5.40", "h", 0, 0, 0o755, "/usr/bin/perl"),
    ("/usr/bin/sudo", "f", 0, 0, 0o4755, b"\x7fELF sudo\n"),
    ("/usr/lib", "d", 0, 0, 0o755, None),
    ("/usr/lib/perl-alias", "h", 0, 0, 0o755, "/usr/bin/perl"),
    ("/usr/lib/long-link", "l", 0, 0, 0o777, LONG_TARGET),
    ("/usr/share", "d", 0, 0, 0o755, None),
    ("/usr/share/empty", "f", 0, 0, 0o644, b""),
    ("/var", "d", 0, 0, 0o755, None),
    ("/var/absolute", "l", 0, 0, 0o777, "/etc/shadow"),
    ("/var/dangling", "l", 0, 0, 0o777, "/nonexistent"),
    ("/var/mail", "d", 0, 8, 0o2775, None),
]


def _entries():
    entries = list(ENTRIES)
    parts = LONG_DIR.split("/")
    for depth in range(4, len(parts) + 1):
        entries.append(("/".join(parts[:depth]), "d", 0, 0, 0o755, None))
    entries.append((LONG_DIR + "/file-with-a-long-path.txt", "f", 0, 0, 0o644, b"long\n"))
    return entries


MODE_BITS = {"d": stat.S_IFDIR, "f": stat.S_IFREG, "h": stat.S_IFREG, "l": stat.S_IFLNK, "c": stat.S_IFCHR,
             "p": stat.S_IFIFO}


def build_tree(directory):
    """Write the fixture rootfs (contents only) and return its manifest records."""
    rootfs = os.path.join(os.fsencode(directory), b"rootfs")
    records, keys = {}, {}
    for index, (path, kind, uid, gid, mode, value) in enumerate(sorted(_entries(), key=lambda e: e[0].encode())):
        raw = path.encode("utf-8")
        host = rootfs + (b"" if raw == b"/" else raw)
        key = "k%d" % index
        device = (0, 0)
        if kind == "d":
            os.makedirs(host, exist_ok=True)
        elif kind == "f":
            with open(host, "wb") as out:
                out.write(value)
            keys[raw] = key
        elif kind == "h":
            os.link(rootfs + value.encode(), host)
            key = keys[value.encode()]
        elif kind == "l":
            os.symlink(value, host)
        elif kind == "c":
            device = value
        records[raw] = Record(MODE_BITS[kind] | mode, uid, gid, key, 1, device, raw)
    for path, record in sorted(records.items(), reverse=True):
        if record.type in "cbp":
            continue
        os.utime(rootfs + (b"" if path == b"/" else path), (1700000000, 1700000000), follow_symlinks=False)
    return rootfs, records


def _decompose(tar_bytes):
    reader = tarstream.Reader(io.BytesIO(tar_bytes))
    members = []
    while (member := reader.next()) is not None:
        members.append([member.name, member.kind, reader.read_data(), member.mode, member.mtime, member.linkname])
    return members


def _raw(out, name, kind, data=b"", linkname=b""):
    out.write(tarstream.header(name, kind, len(data), 0o644, 0, linkname) + data + tarstream.padding(len(data)))


def _serialize(members, trailer=b""):
    out = io.BytesIO()
    writer = tarstream.Writer(out)
    for name, kind, data, mode, mtime, linkname in members:
        if kind in (b"1", b"3", b"g", b"L"):
            _raw(out, name.encode(), kind, data, linkname.encode())
        elif kind == b"x":
            _raw(out, b"@PaxHeader", kind, data)
        else:
            writer.member(name, kind, data, mode=mode, mtime=mtime, linkname=linkname)
    writer.close()
    return out.getvalue() + trailer


def _with_tables(members, mutate_meta=None, mutate_rows=None, reseal=True):
    """Rewrite metadata/attributes (members 0 and 1), optionally keeping digests consistent."""
    metadata = json.loads(members[0][2])
    rows = parse_rows(members[1][2])
    if mutate_rows:
        rows = mutate_rows(rows)
    table = format_rows(rows)
    if reseal:
        metadata["attributes"].update(size=len(table), rows=len(rows))
        metadata["attributes"]["sha256"] = hashlib.sha256(table).hexdigest()
        metadata["rootfs"]["entries"] = len(rows)
        metadata["rootfs"]["members"] = sum(r.type in "dfl" for r in rows)
    if mutate_meta:
        mutate_meta(metadata)
    members = [list(m) for m in members]
    members[0][2] = json.dumps(metadata, indent=2).encode()
    members[1][2] = table if reseal else members[1][2]
    return members


def _bad_variants(members):
    first = next(i for i, m in enumerate(members) if m[0].startswith("rootfs/"))
    file_index = next(i for i, m in enumerate(members) if m[0] == "rootfs/usr/share/empty")

    def swap(ms):
        ms = [list(m) for m in ms]
        ms[first + 1], ms[first + 2] = ms[first + 2], ms[first + 1]
        return ms

    def tamper_uid(ms):
        ms = [list(m) for m in ms]
        ms[1][2] = ms[1][2].replace(b"\t1000\t1000\t0750\t", b"\t0\t0\t0750\t")
        return ms

    def change(index, **fields):
        def apply(ms):
            ms = [list(m) for m in ms]
            for key, value in fields.items():
                ms[index][["name", "kind", "data", "mode", "mtime", "linkname"].index(key)] = value
            return ms
        return apply

    def insert(index, entry):
        return lambda ms: ms[:index] + [entry] + ms[index:]

    def meta(fn):
        return lambda ms: _with_tables(ms, mutate_meta=fn)

    def rows(fn):
        return lambda ms: _with_tables(ms, mutate_rows=fn)

    def hardlink_to_directory(rs):
        return [Row(r.path, r.type, r.uid, r.gid, r.mode, target=b"/usr/lib") if r.type == "h" else r for r in rs]

    def unsorted(rs):
        rs = list(rs)
        rs[1], rs[2] = rs[2], rs[1]
        return rs

    def root_owned_seed(rs):
        return [Row(r.path, r.type, 0, 0, r.mode) if r.path == b"/home/work/.profile" else r for r in rs]

    def under_symlink(rs):
        return sorted(rs + [Row(b"/bin/evil", "f", 0, 0, 0o644)], key=lambda r: r.path)

    return {
        "bad-member-order.tar": ("member-order", swap(members), b""),
        "bad-unexpected-member.tar": ("unexpected-member",
                                      members + [["rootfs/zzz", tarstream.REG, b"x", 0o644, 0, ""]], b""),
        "bad-missing-member.tar": ("missing-member", members[:-1], b""),
        "bad-type-mismatch.tar": ("type-mismatch", change(file_index, kind=tarstream.DIR)(members), b""),
        "bad-unsafe-path.tar": ("unsafe-path", change(first + 1, name="rootfs/../escape")(members), b""),
        "bad-tar-hardlink.tar": ("forbidden-type",
                                 insert(file_index, ["rootfs/usr/share/link", b"1", b"", 0, 0,
                                                     "rootfs/etc/shadow"])(members), b""),
        "bad-tar-device.tar": ("forbidden-type",
                               insert(first + 1, ["rootfs/dev/zero", b"3", b"", 0, 0, ""])(members), b""),
        "bad-global-pax.tar": ("forbidden-type", insert(first, ["", b"g", b"13 comment=\n", 0, 0, ""])(members), b""),
        "bad-gnu-longname.tar": ("forbidden-type",
                                 insert(first, ["././@LongLink", b"L", b"rootfs\0", 0, 0, ""])(members), b""),
        "bad-pax-keyword.tar": ("tar-pax", insert(first, ["", b"x", b"14 uname=root\n", 0, 0, ""])(members), b""),
        "bad-trailing-data.tar": ("tar-end", members, b"garbage" + b"\0" * 505),
        "bad-metadata-json.tar": ("metadata", change(0, data=b"{not json")(members), b""),
        "bad-format-version.tar": ("format-version", meta(lambda m: m.update(formatVersion=1))(members), b""),
        "bad-unknown-type.tar": ("unknown-type", meta(lambda m: m.update(type="alpine-edge"))(members), b""),
        "bad-architecture.tar": ("architecture", meta(lambda m: m.update(architecture="riscv64"))(members), b""),
        "bad-unsupported-requirement.tar": ("unsupported-requirement",
                                            meta(lambda m: m["requires"].append("selinux-labels"))(members), b""),
        "bad-requires-missing.tar": ("requires-missing",
                                     meta(lambda m: m["requires"].remove("hardlink-emulation"))(members), b""),
        "bad-attributes-digest.tar": ("attributes-digest", tamper_uid(members), b""),
        "bad-hardlink-primary.tar": ("hardlink-primary", rows(hardlink_to_directory)(members), b""),
        "bad-row-order.tar": ("row-order", rows(unsorted)(members), b""),
        "bad-parent-not-dir.tar": ("parent-not-dir", rows(under_symlink)(members), b""),
        "bad-store-seed-owner.tar": ("store-seed", rows(root_owned_seed)(members), b""),
        "bad-requires-store-seeds.tar": ("requires-missing",
                                         meta(lambda m: m["requires"].remove("store-seeds"))(members), b""),
    }


def generate(out_dir):
    out_dir = os.fspath(out_dir)
    os.makedirs(out_dir, exist_ok=True)
    os.environ.setdefault("SOURCE_DATE_EPOCH", "1700000000")
    args = dict(image_type="debian-trixie", type_version=1, profile="workspace", architecture="amd64",
                base=BASE, extra=EXTRA)
    listing = []
    with tempfile.TemporaryDirectory() as scratch:
        rootfs, records = build_tree(scratch)
        pack(rootfs, records, os.path.join(out_dir, "good.tar.zst"), level=3, **args)
        listing.append({"file": "good.tar.zst", "index": "good.json", "expect": None})
        plan = build_plan(rootfs, records)
        table = format_rows(plan.rows)
        check_rows(plan.rows)
        metadata = build_metadata(plan, table, **args)
        raw = io.BytesIO()
        write_tar(plan, metadata, table, raw)
    good = raw.getvalue()
    with open(os.path.join(out_dir, "good.tar"), "wb") as out:
        out.write(good)
    listing.append({"file": "good.tar", "expect": None})
    members = _decompose(good)
    assert members[0][0] == METADATA_NAME and members[1][0] == ATTRIBUTES_NAME
    for name, (code, variant, trailer) in sorted(_bad_variants(members).items()):
        with open(os.path.join(out_dir, name), "wb") as out:
            out.write(_serialize(variant, trailer))
        listing.append({"file": name, "expect": code})
    with open(os.path.join(out_dir, "fixtures.json"), "w", encoding="utf-8") as out:
        json.dump(listing, out, indent=2)
        out.write("\n")
    return listing
