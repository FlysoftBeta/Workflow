from __future__ import annotations

import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import time
import uuid


class WorkflowError(RuntimeError):
    pass


def git(root: Path, *args: str, check: bool = True) -> str:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    if check and result.returncode:
        raise WorkflowError(result.stderr.strip() or result.stdout.strip() or "Git command failed")
    return result.stdout.strip()


class Repository:
    def __init__(self, path: Path):
        self.root = Path(git(path.resolve(), "rev-parse", "--show-toplevel"))
        listing = git(self.root, "worktree", "list", "--porcelain")
        self.primary = Path(next(line[9:] for line in listing.splitlines() if line.startswith("worktree "))).resolve()
        self.state = self.primary / "artifacts/workflow"
        self.tasks = self.state / "tasks"
        self.runs = self.state / "runs"
        self.build_lock = self.primary / "artifacts/.gradle.lock"
        self.device_lock = self.primary / "artifacts/.device.lock"

    def initialize(self):
        self.tasks.mkdir(parents=True, exist_ok=True)
        self.runs.mkdir(parents=True, exist_ok=True)
        self.build_lock.touch(exist_ok=True)
        self.device_lock.touch(exist_ok=True)


@contextlib.contextmanager
def lock(path: Path, blocking: bool = True):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+") as stream:
        try:
            fcntl.flock(stream, fcntl.LOCK_EX | (0 if blocking else fcntl.LOCK_NB))
        except BlockingIOError as error:
            raise WorkflowError(f"Resource is in use: {path}") from error
        try:
            yield stream
        finally:
            fcntl.flock(stream, fcntl.LOCK_UN)


def atomic_json(path: Path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    try:
        with temporary.open("w", encoding="utf-8") as stream:
            json.dump(value, stream, ensure_ascii=False, indent=2)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def read_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise WorkflowError(f"Cannot read {path}: {error}") from error


def identifier(value: str) -> str:
    if not re.fullmatch(r"[a-z][a-z0-9-]{0,47}", value):
        raise WorkflowError("Task names must start with a lowercase letter and contain lowercase letters, digits or hyphens (48 characters maximum)")
    return value


def scope_path(value: str) -> str:
    path = PurePosixPath(value.rstrip("/"))
    if not value or path.is_absolute() or ".." in path.parts or "\\" in value or any(c in value for c in "\n\r\0*?[]"):
        raise WorkflowError(f"Use a plain repository-relative file or directory for ownership: {value!r}")
    result = str(path)
    if result == "." or path.parts[0] in {".git", "artifacts", ".claude", ".codex"}:
        raise WorkflowError(f"Ownership must name a source area, not {value!r}")
    return result


def owns(scopes: list[str], path: str) -> bool:
    return any(path == scope or path.startswith(scope + "/") for scope in scopes)


def overlap(left: list[str], right: list[str]) -> bool:
    return any(owns([a], b) or owns([b], a) for a in left for b in right)


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def source_snapshot(root: Path) -> dict:
    result = subprocess.run(["git", "-C", str(root), "ls-files", "-z", "--cached", "--others", "--exclude-standard"], capture_output=True, check=True)
    paths = sorted(set(result.stdout.decode().split("\0")) - {""})
    rows = []
    for relative in paths:
        path = root / relative
        if path.is_symlink():
            value = "link:" + os.readlink(path)
        elif path.is_file():
            value = digest(path)
        elif not path.exists():
            value = "deleted"
        else:
            raise WorkflowError(f"Unsupported source entry: {relative}")
        rows.append([relative, value, bool(path.stat().st_mode & 0o111) if path.exists() else False])
    fingerprint = hashlib.sha256(json.dumps(rows, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
    return {"fingerprint": fingerprint, "head": git(root, "rev-parse", "--verify", "HEAD", check=False) or None, "files": rows}


def changed_paths(root: Path, base: str) -> list[str]:
    changed = subprocess.run(["git", "-C", str(root), "diff", "--no-renames", "--name-only", "-z", base], capture_output=True, check=True).stdout
    untracked = subprocess.run(["git", "-C", str(root), "ls-files", "--others", "--exclude-standard", "-z"], capture_output=True, check=True).stdout
    return sorted(set((changed + untracked).decode().split("\0")) - {""})


def run_id() -> str:
    return time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()) + "-" + uuid.uuid4().hex[:8]
