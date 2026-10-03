#!/usr/bin/env python3
"""Host-only ptrace/ELF/xattr evidence; writes only under ignored artifacts/."""
import datetime
import errno
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import struct
import subprocess
import tempfile

PROJECT = Path(__file__).resolve().parents[2]
OUTPUT = PROJECT / "artifacts" / "engine-probes"
OUTPUT.mkdir(parents=True, exist_ok=True)


def elf_summary(path):
    data = path.read_bytes()
    if data[:4] != b"\x7fELF" or data[4] != 2 or data[5] not in (1, 2):
        raise ValueError("Probe expects a native ELF64 executable")
    endian = "<" if data[5] == 1 else ">"
    header = struct.unpack_from(endian + "HHIQQQIHHHHHH", data, 16)
    elf_type, machine, _, entry, phoff, _, _, _, phentsize, phnum, *_ = header
    interpreter, segments = None, []
    if phentsize < 56 or phnum > 4096 or phoff + phnum * phentsize > len(data):
        raise ValueError("Invalid program headers")
    for i in range(phnum):
        kind, flags, offset, vaddr, _, filesz, memsz, align = struct.unpack_from(endian + "IIQQQQQQ", data, phoff + i * phentsize)
        if kind in (1, 3) and offset + filesz > len(data):
            raise ValueError("ELF segment exceeds file")
        if kind == 3:
            interpreter = data[offset:offset + filesz].rstrip(b"\0").decode()
        if kind == 1:
            segments.append({"flags": flags, "fileSize": filesz, "memorySize": memsz, "alignment": align})
    return {"path": str(path), "type": elf_type, "machine": machine, "entry": entry, "interpreter": interpreter, "loadSegments": segments, "executedByCustomLoader": False}


def attributes(directory):
    file = directory / "file"
    file.write_text("metadata probe\n")
    file.chmod(0o600)
    before = file.stat()
    result = {"hostModeBefore": oct(stat.S_IMODE(before.st_mode)), "hostUidBefore": before.st_uid}
    try:
        for name, value in {"uid": "12345", "gid": "23456", "mode": str(0o755)}.items():
            os.setxattr(file, "user.workflow." + name, value.encode())
        link = directory / "hardlink"
        os.link(file, link)
        renamed = directory / "renamed"
        file.rename(renamed)
        result.update({"regularFileXattr": True, "hardlinkRenamePreserved": os.getxattr(link, "user.workflow.uid") == b"12345" and link.stat().st_ino == renamed.stat().st_ino,
                       "hostModeUnaffected": stat.S_IMODE(renamed.stat().st_mode) == 0o600, "hostUidUnaffected": renamed.stat().st_uid == before.st_uid})
        symlink = directory / "symlink"
        symlink.symlink_to("renamed")
        try:
            os.setxattr(symlink, "user.workflow.uid", b"12345", follow_symlinks=False)
            result["symlinkXattr"] = True
        except OSError as error:
            result.update({"symlinkXattr": False, "symlinkXattrErrno": error.errno})
    except OSError as error:
        result.update({"regularFileXattr": False, "regularFileXattrErrno": error.errno})
    return result


compiler = shutil.which("cc")
if not compiler:
    raise SystemExit("A host C compiler is required")
binary = OUTPUT / "ptrace-probe"
subprocess.run([compiler, "-std=c11", "-Wall", "-Wextra", "-Werror", "-O2", str(Path(__file__).with_name("ptrace_probe.c")), "-o", str(binary)], check=True)
completed = subprocess.run([str(binary)], text=True, capture_output=True, timeout=10)
if completed.returncode:
    raise SystemExit("ptrace probe failed: " + completed.stdout + completed.stderr)
with tempfile.TemporaryDirectory(prefix="attributes-", dir=OUTPUT) as temporary:
    report = {"recordedAt": datetime.datetime.now(datetime.timezone.utc).isoformat(), "scope": "host-only; no Android, guest linker, syscall rewriting or sandbox acceptance", "host": {"system": platform.system(), "kernel": platform.release(), "machine": platform.machine(), "uid": os.getuid()}, "ptrace": json.loads(completed.stdout), "elf": elf_summary(binary), "attributes": attributes(Path(temporary))}
path = OUTPUT / "result.json"
path.write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
print("Evidence:", path)
