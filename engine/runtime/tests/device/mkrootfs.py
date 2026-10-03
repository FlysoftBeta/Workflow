#!/usr/bin/env python3
"""Build an app-extractable guest rootfs tarball from a pinned OCI image layout.

    mkrootfs.py OCI_LAYOUT_DIR OUT.tar [--extra HOSTFILE:GUESTPATH ...]

The Android app UID cannot link(2) arbitrary files into place reliably nor mknod(2), so the
tarball is normalised for extraction by toybox tar running as the app user:
  * hardlinks become regular copies (content duplicated),
  * device nodes and FIFOs are dropped (the engine binds /dev entries itself),
  * OCI whiteouts are applied (multi-layer images), then dropped,
  * owner gets rwx on every directory (a 0555 dir would block its own children and a later rm -rf),
  * uid/gid are recorded as 0 with no names (the app cannot chown anyway).
Writes OUT.tar and OUT.manifest.json (source digests, counts, dropped entries). The file is
rebuilt only when the manifest's inputs differ (layer digests + this script's sha256).
"""
import argparse
import hashlib
import io
import json
import os
import sys
import tarfile


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def blob(layout, digest):
    algo, hexd = digest.split(":", 1)
    return os.path.join(layout, "blobs", algo, hexd)


def norm(name):
    while name.startswith("./") or name.startswith("/"):
        name = name[2:] if name.startswith("./") else name[1:]
    return "" if name == "." else name.rstrip("/")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("layout")
    ap.add_argument("out")
    ap.add_argument("--extra", action="append", default=[], help="HOSTFILE:GUESTPATH added as mode 0755")
    ap.add_argument("--force", action="store_true")
    a = ap.parse_args()

    index = json.load(open(os.path.join(a.layout, "index.json")))
    mdigest = index["manifests"][0]["digest"]
    manifest = json.load(open(blob(a.layout, mdigest)))
    layers = [l["digest"] for l in manifest["layers"]]
    script_sha = sha256_file(os.path.abspath(__file__))
    extras = []
    for spec in a.extra:
        host, guest = spec.rsplit(":", 1)
        extras.append((host, norm(guest), sha256_file(host)))
    inputs = {"manifest": mdigest, "layers": layers, "script": script_sha,
              "extras": [[g, s] for _, g, s in extras]}

    man_path = a.out + ".manifest.json"
    if not a.force and os.path.exists(a.out) and os.path.exists(man_path):
        old = json.load(open(man_path))
        if old.get("inputs") == inputs:
            print(f"up to date: {a.out} ({old['counts']})")
            return 0

    # path -> (TarInfo, bytes|None); insertion order kept so parents precede children.
    entries = {}
    dropped = {"device": [], "fifo": [], "whiteout": 0}
    hardlinks = 0
    for d in layers:
        with tarfile.open(blob(a.layout, d), "r:*") as tf:
            for ti in tf:
                name = norm(ti.name)
                if not name:
                    continue
                base = os.path.basename(name)
                parent = os.path.dirname(name)
                if base == ".wh..wh..opq":
                    pre = parent + "/"
                    for k in [k for k in entries if k.startswith(pre)]:
                        del entries[k]
                    dropped["whiteout"] += 1
                    continue
                if base.startswith(".wh."):
                    victim = os.path.join(parent, base[4:])
                    for k in [k for k in entries if k == victim or k.startswith(victim + "/")]:
                        del entries[k]
                    dropped["whiteout"] += 1
                    continue
                if ti.ischr() or ti.isblk():
                    dropped["device"].append(name)
                    continue
                if ti.isfifo():
                    dropped["fifo"].append(name)
                    continue
                data = None
                if ti.islnk():
                    target = norm(ti.linkname)
                    if target not in entries or entries[target][1] is None:
                        print(f"warning: hardlink {name} -> {target} has no regular target; dropped", file=sys.stderr)
                        continue
                    src_ti, data = entries[target]
                    nti = tarfile.TarInfo(name)
                    nti.type = tarfile.REGTYPE
                    nti.mode, nti.mtime, nti.size = src_ti.mode, src_ti.mtime, len(data)
                    ti = nti
                    hardlinks += 1
                elif ti.isreg():
                    data = tf.extractfile(ti).read()
                nti = tarfile.TarInfo(name)
                nti.type, nti.mode, nti.mtime = ti.type, ti.mode, ti.mtime
                nti.linkname = ti.linkname if ti.issym() else ""
                nti.size = len(data) if data is not None else 0
                if ti.isdir():
                    nti.mode |= 0o700
                entries.pop(name, None)
                entries[name] = (nti, data)

    for host, guest, _ in extras:
        with open(host, "rb") as f:
            data = f.read()
        ti = tarfile.TarInfo(guest)
        ti.type, ti.mode, ti.size, ti.mtime = tarfile.REGTYPE, 0o755, len(data), int(os.path.getmtime(host))
        entries.pop(guest, None)
        entries[guest] = (ti, data)

    counts = {"files": 0, "dirs": 0, "symlinks": 0, "bytes": 0}
    tmp = a.out + ".part"
    with tarfile.open(tmp, "w", format=tarfile.GNU_FORMAT) as out:
        for name, (ti, data) in entries.items():
            ti.uid = ti.gid = 0
            ti.uname = ti.gname = ""
            if ti.isdir():
                counts["dirs"] += 1
            elif ti.issym():
                counts["symlinks"] += 1
            else:
                counts["files"] += 1
                counts["bytes"] += ti.size
            out.addfile(ti, io.BytesIO(data) if data is not None else None)
    os.replace(tmp, a.out)
    info = {"inputs": inputs, "source": os.path.abspath(a.layout), "tarSha256": sha256_file(a.out),
            "counts": counts, "hardlinksCopied": hardlinks,
            "dropped": {"device": dropped["device"], "fifo": dropped["fifo"], "whiteout": dropped["whiteout"]}}
    with open(man_path, "w") as f:
        json.dump(info, f, indent=1)
    print(f"wrote {a.out}: {counts}, hardlinks copied {hardlinks}, dropped dev={len(dropped['device'])} "
          f"fifo={len(dropped['fifo'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
