"""Strict reader and reference installer for the image format (docs/environment.md §2.5).

This is the executable form of the contract: the engine's installer must accept exactly what this
accepts. The reference installer materialises hardlinks as host hardlinks and never creates
special files; the engine instead feeds attributes into its own store and hardlink emulation.
"""
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import tarfile

from . import tarstream
from .attributes import check_rows, parse_rows
from .common import (ALWAYS_REQUIRED, ARCHITECTURES, ATTRIBUTES_LIMIT, ATTRIBUTES_NAME, ATTRIBUTES_VERSION,
                     FORMAT, FORMAT_VERSION, KNOWN_TYPES, METADATA_LIMIT, METADATA_NAME, PROFILES, REQUIRES,
                     ImageError, encode_path, guest_from_member)
from .pack import sha256_file
from . import stores as store_rules

DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")
KIND = {"d": tarstream.DIR, "f": tarstream.REG, "l": tarstream.SYM}


def _require(condition, code, message):
    if not condition:
        raise ImageError(code, message)


def _is_int(value):
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def check_metadata(metadata, architecture=None, profile=None, capabilities=REQUIRES):
    _require(isinstance(metadata, dict), "metadata", "metadata is not an object")
    _require(metadata.get("format") == FORMAT, "format", "not a workflow-image")
    _require(metadata.get("formatVersion") == FORMAT_VERSION, "format-version",
             f"formatVersion {metadata.get('formatVersion')!r} is not supported")
    kind = metadata.get("type")
    _require(kind in KNOWN_TYPES, "unknown-type", f"type {kind!r}")
    _require(metadata.get("typeVersion") in KNOWN_TYPES[kind], "type-version",
             f"typeVersion {metadata.get('typeVersion')!r}")
    _require(metadata.get("profile") in PROFILES and profile in (None, metadata.get("profile")), "profile",
             f"profile {metadata.get('profile')!r}")
    _require(metadata.get("architecture") in ARCHITECTURES and architecture in (None, metadata["architecture"]),
             "architecture", f"image is {metadata.get('architecture')!r}, device is {architecture!r}")
    requires = metadata.get("requires")
    _require(isinstance(requires, list) and all(isinstance(x, str) for x in requires), "metadata", "requires")
    unknown = [x for x in requires if x not in capabilities]
    _require(not unknown, "unsupported-requirement", ", ".join(unknown))
    _require(all(x in requires for x in ALWAYS_REQUIRED), "requires-missing", "virtual ownership/mode")
    _require(isinstance(metadata.get("createdAt"), str), "metadata", "createdAt")
    base = metadata.get("base")
    _require(isinstance(base, dict) and isinstance(base.get("reference"), str)
             and all(isinstance(base.get(k), str) and DIGEST.match(base[k]) for k in ("indexDigest", "manifestDigest"))
             and isinstance(base.get("platform"), str), "metadata", "base")
    rootfs = metadata.get("rootfs")
    _require(isinstance(rootfs, dict) and all(_is_int(rootfs.get(k)) for k in ("entries", "members", "regularBytes")),
             "metadata", "rootfs")
    attributes = metadata.get("attributes")
    _require(isinstance(attributes, dict) and attributes.get("path") == ATTRIBUTES_NAME
             and attributes.get("version") == ATTRIBUTES_VERSION and _is_int(attributes.get("rows"))
             and _is_int(attributes.get("size")) and attributes["size"] <= ATTRIBUTES_LIMIT
             and isinstance(attributes.get("sha256"), str) and re.match(r"^[0-9a-f]{64}$", attributes["sha256"]),
             "metadata", "attributes")
    if metadata["profile"] == "workspace":
        user = metadata.get("user")
        _require(isinstance(user, dict) and isinstance(user.get("name"), str) and _is_int(user.get("uid"))
                 and _is_int(user.get("gid")) and isinstance(user.get("home"), str)
                 and isinstance(user.get("shell"), str), "metadata", "user")
        environment = metadata.get("environment")
        _require(isinstance(environment, dict) and all(isinstance(v, str) for v in environment.values()),
                 "metadata", "environment")
        defaults = metadata.get("defaults")
        _require(isinstance(defaults, dict) and all(isinstance(defaults.get(k), str) for k in ("python", "node")),
                 "metadata", "defaults")
        _require(isinstance(metadata.get("toolchains"), dict), "metadata", "toolchains")
    return metadata


class _Stream:
    def __init__(self, path):
        self.process = None
        if os.fspath(path).endswith(".zst"):
            self.process = subprocess.Popen(["zstd", "-dcq", "--memory=128MB", os.fspath(path)],
                                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            self.file = self.process.stdout
        else:
            self.file = open(path, "rb")

    def close(self, check):
        if self.process is None:
            self.file.close()
            return
        if check:
            while self.file.read(1 << 20):
                pass
        self.file.close()
        stderr = self.process.stderr.read()
        self.process.stderr.close()
        code = self.process.wait()
        if check and code != 0:
            raise ImageError("zstd", stderr.decode("utf-8", "replace").strip() or f"exit {code}")


def verify(path, index=None, architecture=None, profile=None, capabilities=REQUIRES, extract_to=None, sink=None):
    """Validate an image completely; optionally extract its rootfs into extract_to/rootfs.

    `sink` (member(row, member, reader) / finish(rows, metadata, table)) receives the content in order;
    it only commits in finish(), which runs after the whole stream has been validated."""
    if index is not None:
        _require(os.path.getsize(path) == index.get("size"), "image-digest", "size differs from image.json")
        _require(sha256_file(path) == index.get("sha256"), "image-digest", "sha256 differs from image.json")
    stream = _Stream(path)
    ok = False
    try:
        reader = tarstream.Reader(stream.file)
        member = reader.next()
        _require(member is not None and member.name == METADATA_NAME and member.kind == tarstream.REG,
                 "member-order", "first member must be metadata.json")
        try:
            metadata = json.loads(reader.read_data(METADATA_LIMIT))
        except (ValueError, UnicodeDecodeError):
            raise ImageError("metadata", "metadata.json is not JSON") from None
        check_metadata(metadata, architecture, profile, capabilities)
        if index is not None:
            _require(index.get("metadata") == metadata, "index-mismatch", "image.json metadata differs")
        member = reader.next()
        _require(member is not None and member.name == ATTRIBUTES_NAME and member.kind == tarstream.REG,
                 "member-order", "second member must be attributes.tsv")
        table = reader.read_data(ATTRIBUTES_LIMIT)
        described = metadata["attributes"]
        _require(len(table) == described["size"] and hashlib.sha256(table).hexdigest() == described["sha256"],
                 "attributes-digest", "attributes.tsv differs from metadata")
        rows = parse_rows(table)
        counts = check_rows(rows)
        _require(counts["rows"] == described["rows"] == metadata["rootfs"]["entries"]
                 and counts["members"] == metadata["rootfs"]["members"], "attributes-count", "row counts differ")
        _require(not counts["hardlinks"] or "hardlink-emulation" in metadata["requires"], "requires-missing",
                 "hardlink rows without hardlink-emulation")
        _require(not counts["special"] or "virtual-special-files" in metadata["requires"], "requires-missing",
                 "special rows without virtual-special-files")
        stores = store_rules.parse(metadata)
        user = metadata.get("user")
        store_rules.check_rows(rows, stores, (user["uid"], user["gid"]) if isinstance(user, dict) else None)
        _require(not stores or "store-seeds" in metadata["requires"], "requires-missing", "stores without store-seeds")
        installer = _Installer(extract_to, stores) if extract_to is not None else sink
        regular = 0
        for row in rows:
            if row.type not in KIND:
                continue
            member = reader.next()
            _require(member is not None, "missing-member", encode_path(row.path))
            guest = guest_from_member(member.name)
            _require(guest == row.path, "member-order", f"expected {encode_path(row.path)}, found {member.name}")
            _require(member.kind == KIND[row.type], "type-mismatch", member.name)
            regular += member.size
            if installer:
                installer.member(row, member, reader)
        _require(reader.next() is None, "unexpected-member", "archive continues after the last row")
        _require(regular == metadata["rootfs"]["regularBytes"], "attributes-count", "regularBytes differs")
        stream.close(check=True)
        if installer:
            installer.finish(rows, metadata, table)
        ok = True
        return {"metadata": metadata, **counts, "regularBytes": regular}
    finally:
        if not ok:
            stream.close(check=False)


class _Installer:
    """Generation layout: rootfs/ (attributes live in attributes.tsv here, in the engine's store on
    the device) and seeds/<store>/ (plain files with their real permission bits, §2.5)."""

    def __init__(self, destination, stores=None):
        self.destination = os.fsencode(destination)
        self.rootfs = os.path.join(self.destination, b"rootfs")
        self.stores = stores or {}
        os.makedirs(self.destination, exist_ok=True)
        os.mkdir(self.rootfs, 0o700)       # fails if a rootfs already exists
        self.mtimes = []

    def host(self, path):
        seed = store_rules.owner_of(path, self.stores)
        if seed is not None:
            _, store, relative = seed
            return os.path.join(self.destination, b"seeds", store.encode(), relative)
        return self.rootfs if path == b"/" else self.rootfs + path

    def member(self, row, member, reader):
        # Invariant 2 + strict order guarantee every parent is a directory created here, so no
        # archive-created symlink is ever traversed. O_NOFOLLOW|O_EXCL is defence in depth.
        target = self.host(row.path)
        seeded = store_rules.owner_of(row.path, self.stores) is not None
        if row.path in self.stores:
            os.makedirs(os.path.join(self.destination, b"seeds", self.stores[row.path][0].encode()), 0o700)
        if row.type == "d":
            if row.path != b"/":
                os.mkdir(target, (row.mode & 0o777) | 0o700 if seeded else 0o700)
        elif row.type == "f":
            fd = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                         row.mode & 0o777 if seeded else 0o600)
            with os.fdopen(fd, "wb") as out:
                reader.copy_data(out)
        else:
            os.symlink(member.linkname.encode("utf-8"), target)
        self.mtimes.append((target, member.mtime))

    def finish(self, rows, metadata, table):
        for row in rows:
            if row.type == "h":
                os.link(self.host(row.target), self.host(row.path), follow_symlinks=False)
        for target, mtime in reversed(self.mtimes):
            os.utime(target, (mtime, mtime), follow_symlinks=False)
        with open(os.path.join(self.destination, b"metadata.json"), "w", encoding="utf-8") as out:
            json.dump(metadata, out, indent=2)
        with open(os.path.join(self.destination, b"attributes.tsv"), "wb") as out:
            out.write(table)


class TarExporter:
    """Sink that rebuilds a conventional tar with ownership, modes and hardlinks (for `podman import`).
    Device nodes are skipped unless `devices` is set, because rootless importers cannot create them."""

    def __init__(self, out_path, devices=False):
        self.devices, self.out_path = devices, os.fspath(out_path)
        self.archive = tarfile.open(self.out_path + ".partial", "w", format=tarfile.PAX_FORMAT)

    def _info(self, row, name=None):
        info = tarfile.TarInfo(name or ("." + row.path.decode("utf-8") if row.path != b"/" else "."))
        info.uid, info.gid, info.mode = row.uid, row.gid, row.mode
        return info

    def member(self, row, member, reader):
        info = self._info(row)
        info.mtime = member.mtime
        if row.type == "d":
            info.type = tarfile.DIRTYPE
            self.archive.addfile(info)
        elif row.type == "l":
            info.type, info.linkname = tarfile.SYMTYPE, member.linkname
            self.archive.addfile(info)
        else:
            data = reader.read_data()
            info.size = len(data)
            self.archive.addfile(info, io.BytesIO(data))

    def finish(self, rows, metadata, table):
        kinds = {"c": tarfile.CHRTYPE, "b": tarfile.BLKTYPE, "p": tarfile.FIFOTYPE}
        for row in rows:
            if row.type == "h":
                info = self._info(row)
                info.type, info.linkname = tarfile.LNKTYPE, "." + row.target.decode("utf-8")
                self.archive.addfile(info)
            elif row.type in kinds and (row.type == "p" or self.devices):
                info = self._info(row)
                info.type = kinds[row.type]
                if row.type in "cb":
                    info.devmajor, info.devminor = row.device
                self.archive.addfile(info)
        self.archive.close()
        os.replace(self.out_path + ".partial", self.out_path)


def to_tar(path, out_path, index=None, devices=False):
    exporter = TarExporter(out_path, devices)
    try:
        return verify(path, index=index, sink=exporter)
    finally:
        exporter.archive.close()
        if os.path.exists(exporter.out_path + ".partial"):
            os.remove(exporter.out_path + ".partial")


def install(path, destination, index, architecture=None, profile="workspace"):
    """Reference install into a fresh generation directory; atomic via <destination>.partial."""
    destination = os.fspath(destination)
    if os.path.lexists(destination):
        raise ImageError("exists", destination)
    staging = destination + ".partial"
    shutil.rmtree(staging, ignore_errors=True)
    try:
        report = verify(path, index=index, architecture=architecture, profile=profile, extract_to=staging)
        os.replace(staging, destination)
        return report
    finally:
        shutil.rmtree(staging, ignore_errors=True)
