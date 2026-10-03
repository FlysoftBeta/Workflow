"""Ownership manifest: the packer's metadata input.

One NUL-terminated record per filesystem entry, as printed by guest/capture-manifest.sh:

    <raw st_mode hex> <uid> <gid> <inode key> <nlink> <major hex>:<minor hex> <path>

`path` is relative to the captured root ('.' or './usr/bin'). The inode key identifies hardlink
groups; it only has to be unique within one manifest.
"""
import fnmatch
from dataclasses import dataclass

from .common import ImageError, check_guest_path, type_of_mode


@dataclass(frozen=True)
class Record:
    mode: int          # raw st_mode including type bits
    uid: int
    gid: int
    key: str
    nlink: int
    device: tuple      # (major, minor)
    path: bytes        # guest absolute

    @property
    def type(self):
        return type_of_mode(self.mode)


def relative(path):
    return b"." if path == b"/" else b"." + path


def absolute(path):
    if path == b".":
        return b"/"
    if path.startswith(b"./"):
        path = path[1:]
    return check_guest_path(path)


def parse(data):
    if data and not data.endswith(b"\0"):
        raise ImageError("manifest-syntax", "manifest must end with NUL")
    records = {}
    for chunk in data.split(b"\0")[:-1] if data else []:
        fields = chunk.split(b" ", 6)
        if len(fields) != 7:
            raise ImageError("manifest-syntax", repr(chunk[:80]))
        try:
            mode = int(fields[0], 16)
            uid, gid, nlink = int(fields[1]), int(fields[2]), int(fields[4])
            major, minor = (int(x, 16) for x in fields[5].split(b":"))
        except ValueError:
            raise ImageError("manifest-syntax", repr(chunk[:80])) from None
        record = Record(mode, uid, gid, fields[3].decode("ascii"), nlink, (major, minor), absolute(fields[6]))
        if record.path in records:
            raise ImageError("manifest-duplicate", repr(record.path))
        records[record.path] = record
    return records


def format_records(records):
    out = []
    for path in sorted(records):
        r = records[path]
        out.append(b"%x %d %d %s %d %x:%x %s\0" % (r.mode, r.uid, r.gid, r.key.encode("ascii"), r.nlink,
                                                  r.device[0], r.device[1], relative(path)))
    return b"".join(out)


class Prune:
    """Patterns from guest/prune.list, matched like find(1) -path against './relative' paths.

    A line starting with '!' is an exception to the preceding patterns."""

    def __init__(self, lines=()):
        self.patterns, self.exceptions = [], []
        for line in lines:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            (self.exceptions if line.startswith("!") else self.patterns).append(line.lstrip("!"))

    @classmethod
    def load(cls, path):
        if path is None:
            return cls()
        with open(path, encoding="utf-8") as handle:
            return cls(handle)

    def __call__(self, guest_path):
        text = relative(guest_path).decode("utf-8", "surrogateescape")
        if any(fnmatch.fnmatchcase(text, p) for p in self.exceptions):
            return False
        return any(fnmatch.fnmatchcase(text, p) for p in self.patterns)


def load_canonical(path):
    """guest/canonical.list -> {guest path: (type, uid, gid, mode)}."""
    canonical = {}
    if path is None:
        return canonical
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            guest, kind, uid, gid, mode = line.split()
            if kind not in ("d", "f"):
                raise ImageError("canonical", line)
            canonical[check_guest_path(guest.encode("utf-8"))] = (kind, int(uid), int(gid), int(mode, 8))
    return canonical


def apply_canonical(records, canonical):
    """Replace ownership and mode of listed paths that exist in the manifest (types must agree)."""
    result = dict(records)
    for path, (kind, uid, gid, mode) in canonical.items():
        record = result.get(path)
        if record is None:
            continue
        if record.type != kind:
            raise ImageError("canonical", f"{path!r} is {record.type}, canonical.list says {kind}")
        result[path] = Record((record.mode & ~0o7777) | mode, uid, gid, record.key, record.nlink, record.device, path)
    return result


def hardlink_groups(records):
    groups = {}
    for path, record in records.items():
        if record.type == "f":
            groups.setdefault(record.key, []).append(path)
    return sorted(sorted(paths) for paths in groups.values() if len(paths) > 1)


def compare(left, right, prune=None, ignore=()):
    """Differences between two manifests (paths, types, ownership, mode, devices, hardlink groups)."""
    prune = prune or Prune()
    ignore = set(ignore)

    def keep(records):
        return {p: r for p, r in records.items() if not prune(p) and p not in ignore}

    left, right = keep(left), keep(right)
    problems = []
    for path in sorted(set(left) | set(right)):
        a, b = left.get(path), right.get(path)
        if a is None or b is None:
            problems.append(("only-" + ("right" if a is None else "left"), path))
            continue
        for field in ("type", "uid", "gid"):
            if getattr(a, field) != getattr(b, field):
                problems.append((field, path, getattr(a, field), getattr(b, field)))
        if a.mode & 0o7777 != b.mode & 0o7777 and a.type != "l":
            problems.append(("mode", path, oct(a.mode & 0o7777), oct(b.mode & 0o7777)))
        if a.type in "cb" and a.device != b.device:
            problems.append(("device", path, a.device, b.device))
    if hardlink_groups(left) != hardlink_groups(right):
        problems.append(("hardlinks", b"", len(hardlink_groups(left)), len(hardlink_groups(right))))
    return problems
