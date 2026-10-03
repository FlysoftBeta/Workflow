#!/usr/bin/env python3
"""Check actual shipped ELF architecture and entry points, beyond metadata declarations.

Usage: python3 image/tests/inspect-artifacts.py artifacts/image/ARCH/image.tar.zst
Run wfimage verify separately for the full digest and format/attributes contract.
"""
import hashlib
import json
import subprocess
import sys
import tarfile
from pathlib import Path

path = Path(sys.argv[1])
index = json.loads(path.with_suffix("").with_suffix(".json").read_text())
metadata = index["metadata"]
expected = {"amd64": 62, "arm64": 183}[metadata["architecture"]]
assert metadata["profile"] == "workspace"
elfs = {}
entry_points = {}
envctl_sha = None
with subprocess.Popen(["zstd", "-dcq", str(path)], stdout=subprocess.PIPE) as decompressor:
    with tarfile.open(fileobj=decompressor.stdout, mode="r|") as archive:
        for member in archive:
            if member.name == "rootfs/usr/local/bin/codex":
                assert member.issym() and member.linkname == "/opt/workflow/tools/codex/bin/codex"
                entry_points["codex"] = member.linkname
            assert member.name != "rootfs/usr/bin/qemu-aarch64", "build-only emulator leaked into image"
            if not member.isfile():
                continue
            file = archive.extractfile(member)
            prefix = file.read(20)
            if prefix[:4] == b"\x7fELF":
                machine = int.from_bytes(prefix[18:20], "little" if prefix[5] == 1 else "big")
                assert machine == expected, (member.name, machine, expected)
                elfs[member.name] = machine
            if member.name == "rootfs/usr/local/libexec/workflow/envctl":
                envctl_sha = hashlib.sha256(prefix + file.read()).hexdigest()
    assert decompressor.wait() == 0
assert "rootfs/usr/bin/bash" in elfs
assert "rootfs/usr/local/bin/uv" in elfs
assert any(p.startswith("rootfs/opt/toolchains/uv/python/cpython-") and "/bin/python" in p for p in elfs)
assert any(p.startswith("rootfs/opt/toolchains/nvm/versions/node/v") and p.endswith("/bin/node") for p in elfs)
assert envctl_sha == hashlib.sha256((Path(__file__).resolve().parents[1] / "guest/envctl").read_bytes()).hexdigest()
assert entry_points.get("codex")
print(json.dumps({"architecture": metadata["architecture"], "elfMachine": expected,
                  "verifiedElfFiles": len(elfs), "imageSha256": index["sha256"],
                  "entryPoints": entry_points, "envctlSha256": envctl_sha,
                  "buildEmulatorAbsent": True}, indent=2))
