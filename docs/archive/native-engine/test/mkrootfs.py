#!/usr/bin/env python3
"""Extract an OCI layer tar into an engine-format rootfs (test/reference tool).

Writes exactly what the engine's installer writes (docs/engine.md, "on-disk format"):
  * regular files / dirs / symlinks as host objects; host permission bits are the
    archive's permission bits | 0600 (files) or | 0700 (dirs) -- never set-id/sticky;
  * xattr  user.workflow.meta = "1 UID GID MODE NLINK MAJ,MIN"  (MODE octal incl. type)
    on every regular file, directory and virtual special file (not on symlinks);
  * hardlinks: the first name is moved into <root>/.workflow-engine/links/<id> and every
    name becomes a symlink whose text is "/.workflow-engine/links/<id>"; NLINK counts names;
  * character/block devices and FIFOs: empty regular host file ("placeholder"), type
    (and rdev) only in the xattr -- the kernel refuses user.* xattrs on FIFOs;
  * mtimes preserved.
Usage: mkrootfs.py LAYER_TAR DEST_DIR            OCI layer (uid/gid/mode from tar headers)
       mkrootfs.py --image IMAGE.tar.zst DEST_DIR  workflow image formatVersion 2
                                                    (docs/environment.md §2: attributes.tsv)
DEST_DIR must not exist.
"""
import os
import stat
import sys
import tarfile

XATTR = "user.workflow.meta"
STORE = ".workflow-engine/links"
LINK_PREFIX = "/" + STORE + "/"


def meta(uid, gid, mode, nlink=0, rdev=(0, 0)):
    return ("1 %d %d %o %d %d,%d" % (uid, gid, mode, nlink, rdev[0], rdev[1])).encode()


def norm(n):
    n = n.lstrip("/")
    while n.startswith("./"):
        n = n[2:]
    return "" if n == "." else n


def setmeta(path, value):
    os.setxattr(path, XATTR, value, follow_symlinks=False)


def main(layer, dest):
    if os.path.exists(dest):
        sys.exit("destination exists: %s" % dest)
    os.makedirs(dest)
    root = os.path.realpath(dest)
    store = os.path.join(root, STORE)
    os.makedirs(store, mode=0o700)
    mtimes = []
    next_id = 1
    obj_of = {}          # guest path -> object id (for hardlink groups)
    with tarfile.open(layer) as tar:
        for m in tar:
            name = norm(m.name)
            parts = [p for p in name.split("/") if p]
            if any(p == ".." for p in parts) or any(p.startswith(".wh.") for p in parts):
                continue
            host = os.path.join(root, *parts) if parts else root
            gpath = "/" + "/".join(parts)
            perm = m.mode & 0o7777
            if m.isdir():
                if parts:
                    os.makedirs(host, exist_ok=True)
                os.chmod(host, (perm & 0o777) | 0o700)
                setmeta(host, meta(m.uid, m.gid, stat.S_IFDIR | perm))
                mtimes.append((host, m.mtime))
            elif m.isreg():
                with tar.extractfile(m) as src, open(host, "wb") as out:
                    while True:
                        b = src.read(1 << 20)
                        if not b:
                            break
                        out.write(b)
                os.chmod(host, (perm & 0o777) | 0o600)
                setmeta(host, meta(m.uid, m.gid, stat.S_IFREG | perm))
                mtimes.append((host, m.mtime))
            elif m.issym():
                os.symlink(m.linkname, host)
            elif m.islnk():
                target = "/" + norm(m.linkname)
                thost = os.path.join(root, target.lstrip("/"))
                oid = obj_of.get(target)
                if oid is None:
                    oid = "%016x" % next_id
                    next_id += 1
                    obj = os.path.join(store, oid)
                    os.rename(thost, obj)
                    os.symlink(LINK_PREFIX + oid, thost)
                    obj_of[target] = oid
                    val = os.getxattr(obj, XATTR).decode().split()
                    val[4] = "1"
                    os.setxattr(obj, XATTR, " ".join(val).encode())
                obj = os.path.join(store, oid)
                os.symlink(LINK_PREFIX + oid, host)
                obj_of[gpath] = oid
                val = os.getxattr(obj, XATTR).decode().split()
                val[4] = str(int(val[4]) + 1)
                os.setxattr(obj, XATTR, " ".join(val).encode())
            elif m.ischr() or m.isblk():
                open(host, "wb").close()
                os.chmod(host, 0o600)
                kind = stat.S_IFCHR if m.ischr() else stat.S_IFBLK
                setmeta(host, meta(m.uid, m.gid, kind | perm, 0, (m.devmajor, m.devminor)))
            elif m.isfifo():
                # user.* xattrs are refused on FIFOs: a regular placeholder carries the type
                open(host, "wb").close()
                os.chmod(host, 0o600)
                setmeta(host, meta(m.uid, m.gid, stat.S_IFIFO | perm))
    for host, mt in reversed(mtimes):
        try:
            os.utime(host, (mt, mt), follow_symlinks=False)
        except OSError:
            pass
    print("installed %s (hardlink objects: %d)" % (root, next_id - 1))


def unpct(b):
    out = bytearray()
    i = 0
    while i < len(b):
        if b[i] == 0x25:
            out.append(int(b[i + 1:i + 3], 16))
            i += 3
        else:
            out.append(b[i])
            i += 1
    return bytes(out)


class Linker:
    def __init__(self, root):
        self.root = root
        self.store = os.path.join(root, STORE)
        os.makedirs(self.store, mode=0o700, exist_ok=True)
        self.next_id = 1
        self.obj_of = {}

    def link(self, target_host, new_host):
        oid = self.obj_of.get(target_host)
        obj = None
        if oid is None:
            oid = "%016x" % self.next_id
            self.next_id += 1
            obj = os.path.join(self.store, oid)
            os.rename(target_host, obj)
            os.symlink(LINK_PREFIX + oid, target_host)
            self.obj_of[target_host] = oid
            v = os.getxattr(obj, XATTR).decode().split()
            v[4] = "1"
            os.setxattr(obj, XATTR, " ".join(v).encode())
        obj = os.path.join(self.store, oid)
        os.symlink(LINK_PREFIX + oid, new_host)
        self.obj_of[new_host] = oid
        v = os.getxattr(obj, XATTR).decode().split()
        v[4] = str(int(v[4]) + 1)
        os.setxattr(obj, XATTR, " ".join(v).encode())


def install_image(image, dest):
    import json
    from compression import zstd
    if os.path.exists(dest):
        sys.exit("destination exists: %s" % dest)
    os.makedirs(dest)
    root = os.path.realpath(dest)
    with zstd.open(image, "rb") as raw, tarfile.open(fileobj=raw, mode="r|") as tar:
        it = iter(tar)
        m = next(it)
        assert m.name == "metadata.json", m.name
        metadata = json.loads(tar.extractfile(m).read())
        assert metadata["format"] == "workflow-image" and metadata["formatVersion"] == 2
        m = next(it)
        assert m.name == "attributes.tsv", m.name
        rows = tar.extractfile(m).read().split(b"\n")
        assert rows[0] == b"#workflow-attributes 1", rows[0]
        attrs = {}
        order = []
        for line in rows[1:]:
            if not line:
                continue
            path, typ, uid, gid, mode, extra = line.split(b"\t")
            path = unpct(path)
            attrs[path] = (typ.decode(), int(uid), int(gid), int(mode, 8), extra)
            order.append(path)
        mtimes = []
        for m in it:
            name = m.name.encode("utf-8", "surrogateescape")
            assert name == b"rootfs" or name.startswith(b"rootfs/"), name
            g = b"/" + name[len(b"rootfs/"):] if name != b"rootfs" else b"/"
            typ, uid, gid, mode, extra = attrs[g]
            host = root.encode() + (g if g != b"/" else b"")
            if m.isdir():
                assert typ == "d"
                os.makedirs(host, exist_ok=True)
                os.chmod(host, (mode & 0o777) | 0o700)
                setmeta(host, meta(uid, gid, stat.S_IFDIR | mode))
                mtimes.append((host, m.mtime))
            elif m.isreg():
                assert typ == "f"
                with tar.extractfile(m) as src, open(host, "wb") as out:
                    while True:
                        b = src.read(1 << 20)
                        if not b:
                            break
                        out.write(b)
                os.chmod(host, (mode & 0o777) | 0o600)
                setmeta(host, meta(uid, gid, stat.S_IFREG | mode))
                mtimes.append((host, m.mtime))
            elif m.issym():
                assert typ == "l"
                os.symlink(m.linkname.encode("utf-8", "surrogateescape"), host)
            else:
                sys.exit("unexpected member type in %r" % name)
    linker = Linker(root)
    for g in order:
        typ, uid, gid, mode, extra = attrs[g]
        host = root.encode() + g
        if typ == "h":
            target = root.encode() + unpct(extra)
            linker.link(target.decode("utf-8", "surrogateescape"), host.decode("utf-8", "surrogateescape"))
        elif typ in ("c", "b"):
            maj, mi = (int(x) for x in extra.split(b","))
            open(host, "wb").close()
            os.chmod(host, 0o600)
            setmeta(host, meta(uid, gid, (stat.S_IFCHR if typ == "c" else stat.S_IFBLK) | mode, 0, (maj, mi)))
        elif typ == "p":
            open(host, "wb").close()
            os.chmod(host, 0o600)
            setmeta(host, meta(uid, gid, stat.S_IFIFO | mode))
    for host, mt in reversed(mtimes):
        try:
            os.utime(host, (mt, mt), follow_symlinks=False)
        except OSError:
            pass
    print("installed %s: %d rows, hardlink objects %d" % (root, len(order), linker.next_id - 1))


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--image":
        install_image(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 3:
        main(sys.argv[1], sys.argv[2])
    else:
        sys.exit(__doc__)
