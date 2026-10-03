"""Convert a foreign tar (OCI layer, `podman export`) into a rootfs directory + ownership manifest.

Runs without root: file contents land on disk owned by the invoking user with host-safe modes,
while uid/gid/full mode/device numbers/hardlink groups are recorded in the manifest only. Device
nodes and FIFOs are never created on the host.
"""
import os
import stat
import tarfile

from .common import ImageError, check_guest_path, parent_of
from .manifest import Record, format_records


def _guest_path(name):
    raw = name.encode("utf-8", "surrogateescape")
    while raw.startswith(b"./"):
        raw = raw[2:]
    raw = raw.strip(b"/")
    if raw in (b"", b"."):
        return b"/"
    return check_guest_path(b"/" + raw)


def from_tar(source, out_dir, warnings=None):
    """Extract `source` (path or binary stream) into out_dir/rootfs; write out_dir/manifest.

    Returns the manifest records keyed by guest path."""
    warnings = [] if warnings is None else warnings
    rootfs = os.path.join(os.fsencode(out_dir), b"rootfs")
    os.makedirs(rootfs)
    records = {b"/": Record(stat.S_IFDIR | 0o755, 0, 0, "d:0", 1, (0, 0), b"/")}
    directories, files, mtimes = {b"/"}, {}, {}
    counter = 0

    def host(path):
        return rootfs + path if path != b"/" else rootfs

    def ensure_parent(path):
        parent = parent_of(path)
        if parent in directories:
            return
        if parent in records:
            raise ImageError("parent-not-dir", repr(path))
        ensure_parent(parent)
        os.mkdir(host(parent), 0o755)
        directories.add(parent)
        records[parent] = Record(stat.S_IFDIR | 0o755, 0, 0, "d:" + parent.hex(), 1, (0, 0), parent)
        warnings.append("implicit directory " + parent.decode("utf-8", "replace"))

    opener = tarfile.open(source, "r|*") if isinstance(source, (str, bytes, os.PathLike)) \
        else tarfile.open(fileobj=source, mode="r|*")
    with opener as archive:
        for member in archive:
            path = _guest_path(member.name)
            xattrs = [k for k in member.pax_headers if k.startswith("SCHILY.xattr.")]
            if xattrs:
                warnings.append("dropped xattrs %s on %s" % (",".join(xattrs), path.decode("utf-8", "replace")))
            mode = member.mode & 0o7777
            if path == b"/":
                if not member.isdir():
                    raise ImageError("attributes-root", "root is not a directory")
                records[path] = Record(stat.S_IFDIR | mode, member.uid, member.gid, "d:0", 1, (0, 0), path)
                mtimes[path] = member.mtime
                continue
            if path in records:
                if member.isdir() and records[path].type == "d":
                    records[path] = Record(stat.S_IFDIR | mode, member.uid, member.gid, records[path].key,
                                           1, (0, 0), path)
                    mtimes[path] = member.mtime
                    continue
                raise ImageError("manifest-duplicate", member.name)
            ensure_parent(path)
            target = host(path)
            if member.isdir():
                os.mkdir(target, 0o755)
                directories.add(path)
                records[path] = Record(stat.S_IFDIR | mode, member.uid, member.gid, "d:" + path.hex(), 1,
                                       (0, 0), path)
            elif member.isreg():
                counter += 1
                fd = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                             0o755 if mode & 0o111 else 0o644)
                with os.fdopen(fd, "wb") as writer, archive.extractfile(member) as reader:
                    while chunk := reader.read(1 << 20):
                        writer.write(chunk)
                key = "t:%d" % counter
                files[path] = key
                records[path] = Record(stat.S_IFREG | mode, member.uid, member.gid, key, 1, (0, 0), path)
            elif member.islnk():
                linked = _guest_path(member.linkname)
                if linked not in files:
                    raise ImageError("hardlink-primary", member.name + " -> " + member.linkname)
                os.link(host(linked), target, follow_symlinks=False)
                primary = records[linked]
                files[path] = primary.key
                records[path] = Record(primary.mode, primary.uid, primary.gid, primary.key, 1, (0, 0), path)
            elif member.issym():
                os.symlink(member.linkname.encode("utf-8", "surrogateescape"), target)
                records[path] = Record(stat.S_IFLNK | 0o777, member.uid, member.gid, "l:" + path.hex(), 1,
                                       (0, 0), path)
            elif member.ischr() or member.isblk() or member.isfifo():
                kind = stat.S_IFCHR if member.ischr() else stat.S_IFBLK if member.isblk() else stat.S_IFIFO
                records[path] = Record(kind | mode, member.uid, member.gid, "s:" + path.hex(), 1,
                                       (member.devmajor, member.devminor), path)
                continue
            else:
                raise ImageError("unknown-type", member.name)
            mtimes[path] = member.mtime
    links = {}
    for record in records.values():
        if record.type == "f":
            links[record.key] = links.get(record.key, 0) + 1
    for path, record in list(records.items()):
        if record.type == "f" and links[record.key] > 1:
            records[path] = Record(record.mode, record.uid, record.gid, record.key, links[record.key],
                                   record.device, path)
    for path in sorted(mtimes, reverse=True):
        os.utime(host(path), (int(mtimes[path]), int(mtimes[path])), follow_symlinks=False)
    with open(os.path.join(os.fsencode(out_dir), b"manifest"), "wb") as handle:
        handle.write(format_records(records))
    return records
