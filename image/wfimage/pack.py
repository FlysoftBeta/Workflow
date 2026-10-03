"""Packer: rootfs directory + ownership manifest -> image.tar.zst + image.json (§2 of environment.md).

Needs no root and no target-architecture execution: contents come from the directory, every
attribute the host cannot represent comes from the manifest.
"""
import calendar
import hashlib
import json
import os
import subprocess
import time
from dataclasses import dataclass, field

from . import tarstream
from .attributes import Row, check_rows, format_rows
from .common import (ALWAYS_REQUIRED, ATTRIBUTES_NAME, ATTRIBUTES_VERSION, FORMAT, FORMAT_VERSION,
                     METADATA_NAME, ImageError, check_guest_path, encode_path, member_name, type_of_mode)
from .manifest import Prune
from . import stores as store_rules

WORKSPACE_FIELDS = ("user", "environment", "toolchains", "defaults")


@dataclass
class Plan:
    rows: list
    sources: dict = field(default_factory=dict)   # guest path -> (host path, size, mtime, linkname)
    regular_bytes: int = 0
    warnings: list = field(default_factory=list)


def _host(rootfs, path):
    return rootfs if path == b"/" else rootfs.rstrip(b"/") + path


def _walk(rootfs, records, prune, warnings):
    """Every entry in the directory must be listed (unless pruned) with the same type."""
    seen = set()
    stack = [b"/"]
    while stack:
        directory = stack.pop()
        with os.scandir(_host(rootfs, directory)) as entries:
            for entry in entries:
                path = (directory.rstrip(b"/") + b"/" + entry.name) if directory != b"/" else b"/" + entry.name
                if prune(path):
                    continue
                kind = type_of_mode(entry.stat(follow_symlinks=False).st_mode)
                record = records.get(path)
                if record is None:
                    if kind == "s":
                        warnings.append("dropped socket " + encode_path(path))
                        continue
                    raise ImageError("unlisted", encode_path(path))
                if record.type != kind:
                    raise ImageError("type-mismatch", f"{encode_path(path)}: manifest {record.type}, tree {kind}")
                seen.add(path)
                if kind == "d":
                    stack.append(path)
    return seen


def build_plan(rootfs, records, prune=None, overrides=None):
    rootfs = os.fsencode(rootfs)
    prune = prune or Prune()
    overrides = {check_guest_path(os.fsencode(k)): os.fsencode(v) for k, v in (overrides or {}).items()}
    plan = Plan([])
    kept = {p: r for p, r in records.items() if p == b"/" or not prune(p)}
    if b"/" not in kept or kept[b"/"].type != "d":
        raise ImageError("attributes-root", "manifest has no root directory")
    seen = _walk(rootfs, kept, prune, plan.warnings) | {b"/"}
    group_sizes = {}
    for record in kept.values():
        if record.type == "f":
            group_sizes[record.key] = group_sizes.get(record.key, 0) + 1
    primaries = {}
    for path in sorted(kept):
        record = kept[path]
        check_guest_path(path)
        kind, perm = record.type, record.mode & 0o7777
        if kind == "s":
            plan.warnings.append("dropped socket " + encode_path(path))
            continue
        override = overrides.pop(path, None)
        if override is not None and (kind != "f" or group_sizes[record.key] > 1):
            raise ImageError("override", "only a single-link regular file can be overridden: " + encode_path(path))
        if kind in "dfl" and path not in seen and override is None:
            raise ImageError("missing", encode_path(path))
        if kind == "f" and record.key in primaries:
            primary = kept[primaries[record.key]]
            if (primary.uid, primary.gid, primary.mode) != (record.uid, record.gid, record.mode):
                raise ImageError("hardlink-attributes", encode_path(path))
            plan.rows.append(Row(path, "h", record.uid, record.gid, perm, target=primaries[record.key]))
            continue
        if kind == "f":
            primaries[record.key] = path
        row = Row(path, kind, record.uid, record.gid, 0o777 if kind == "l" else perm,
                  device=record.device if kind in "cb" else ())
        plan.rows.append(row)
        if kind not in "dfl":
            continue
        host = _host(rootfs, path)
        info = os.lstat(host) if path in seen else os.stat(override)
        size, linkname = 0, ""
        if kind == "f":
            host = override or host
            size = os.stat(host).st_size
            plan.regular_bytes += size
        elif kind == "l":
            try:
                linkname = os.readlink(host).decode("utf-8")
            except UnicodeDecodeError:
                raise ImageError("non-utf8-path", "symlink target of " + encode_path(path)) from None
        plan.sources[path] = (host, size, max(0, int(info.st_mtime)), linkname)
    if overrides:
        raise ImageError("override", "not a regular file in the image: " + encode_path(next(iter(overrides))))
    check_rows(plan.rows)
    return plan


def _created_at():
    epoch = os.environ.get("SOURCE_DATE_EPOCH")
    seconds = int(epoch) if epoch else int(time.time())
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(seconds))


def build_metadata(plan, attributes, *, image_type, type_version, profile, architecture, base, extra=None):
    counts = check_rows(plan.rows)
    extra = dict(extra or {})
    stores = store_rules.parse(extra)
    user = extra.get("user")
    store_rules.check_rows(plan.rows, stores, (user["uid"], user["gid"]) if user else None)
    requires = list(ALWAYS_REQUIRED)
    if counts["hardlinks"]:
        requires.append("hardlink-emulation")
    if counts["special"]:
        requires.append("virtual-special-files")
    if stores:
        requires.append("store-seeds")
    created = _created_at()
    metadata = {
        "format": FORMAT, "formatVersion": FORMAT_VERSION, "type": image_type, "typeVersion": type_version,
        "profile": profile, "architecture": architecture, "createdAt": created, "base": base,
        "rootfs": {"entries": counts["rows"], "members": counts["members"], "regularBytes": plan.regular_bytes},
        "attributes": {"path": ATTRIBUTES_NAME, "version": ATTRIBUTES_VERSION, "rows": counts["rows"],
                       "size": len(attributes), "sha256": hashlib.sha256(attributes).hexdigest()},
        "requires": requires,
    }
    if profile == "workspace":
        missing = [k for k in WORKSPACE_FIELDS if k not in extra]
        if missing:
            raise ImageError("metadata", "workspace image needs " + ", ".join(missing))
    for key, value in extra.items():
        if key in metadata:
            raise ImageError("metadata", "extra metadata may not override " + key)
        metadata[key] = value
    return metadata


def write_tar(plan, metadata, attributes, out):
    writer = tarstream.Writer(out)
    mtime = calendar.timegm(time.strptime(metadata["createdAt"], "%Y-%m-%dT%H:%M:%SZ"))
    content = json.dumps(metadata, indent=2).encode() + b"\n"
    writer.member(METADATA_NAME, tarstream.REG, content, mode=0o644, mtime=mtime)
    writer.member(ATTRIBUTES_NAME, tarstream.REG, attributes, mode=0o644, mtime=mtime)
    for row in plan.rows:
        if row.type not in "dfl":
            continue
        host, size, member_mtime, linkname = plan.sources[row.path]
        name = member_name(row.path)
        if row.type == "d":
            writer.member(name, tarstream.DIR, mode=row.mode, mtime=member_mtime)
        elif row.type == "l":
            writer.member(name, tarstream.SYM, mode=0o777, mtime=member_mtime, linkname=linkname)
        else:
            fd = os.open(host, os.O_RDONLY | os.O_NOFOLLOW)
            with os.fdopen(fd, "rb") as source:
                writer.member(name, tarstream.REG, size=size, mode=row.mode, mtime=member_mtime, source=source)
    writer.close()


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        while chunk := handle.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def index_path_for(image_path):
    """image.tar.zst -> image.json (same directory, same stem)."""
    stem = os.fspath(image_path)
    for suffix in (".zst", ".tar"):
        stem = stem[:-len(suffix)] if stem.endswith(suffix) else stem
    return stem + ".json"


def write_image(plan, metadata, attributes, out_path, *, compress=True, level=19, window_log=27):
    """Write image (optionally zstd) atomically and its image.json index next to it."""
    out_path = os.fspath(out_path)
    partial = out_path + ".partial"
    with open(partial, "wb") as handle:
        if compress:
            if not 1 <= level <= 19 or not 10 <= window_log <= 27:
                raise ImageError("zstd", "level must be 1-19 and windowLog 10-27")
            process = subprocess.Popen(["zstd", "-q", "-T2", f"-{level}", f"--long={window_log}", "-c"],
                                       stdin=subprocess.PIPE, stdout=handle)
            try:
                write_tar(plan, metadata, attributes, process.stdin)
            finally:
                process.stdin.close()
            if process.wait() != 0:
                raise ImageError("zstd", "compression failed")
        else:
            write_tar(plan, metadata, attributes, handle)
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(partial, out_path)
    index = {"sha256": sha256_file(out_path), "size": os.path.getsize(out_path), "metadata": metadata}
    index_path = index_path_for(out_path)
    with open(index_path + ".partial", "w", encoding="utf-8") as handle:
        json.dump(index, handle, indent=2)
        handle.write("\n")
    os.replace(index_path + ".partial", index_path)
    return index


def pack(rootfs, records, out_path, *, prune=None, overrides=None, compress=True, level=19, window_log=27,
         **metadata_args):
    plan = build_plan(rootfs, records, prune, overrides)
    attributes = format_rows(plan.rows)
    metadata = build_metadata(plan, attributes, **metadata_args)
    index = write_image(plan, metadata, attributes, out_path, compress=compress, level=level,
                        window_log=window_log)
    return index, plan.warnings
