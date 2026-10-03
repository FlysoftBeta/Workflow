from __future__ import annotations

from pathlib import Path
import subprocess
import time

from .common import Repository, WorkflowError, atomic_json, changed_paths, git, identifier, lock, overlap, owns, read_json, scope_path, source_snapshot, run_id


def task_file(repo: Repository, name: str) -> Path:
    return repo.tasks / (identifier(name) + ".json")


def load_task(repo: Repository, name: str) -> dict:
    task = read_json(task_file(repo, name))
    if task.get("format") != 1 or task.get("name") != name or task.get("branch") != "codex/" + name:
        raise WorkflowError("Invalid task record")
    if Path(task["checkout"]).resolve() != (repo.state / "checkouts" / name).resolve():
        raise WorkflowError("Task checkout escaped the managed directory")
    return task


def all_tasks(repo: Repository) -> list[dict]:
    return [read_json(path) for path in sorted(repo.tasks.glob("*.json"))]


def setup_links(repo: Repository, checkout: Path):
    (checkout / "artifacts").mkdir(exist_ok=True)
    for name, target in [(".gradle.lock", repo.build_lock), (".device.lock", repo.device_lock)]:
        link = checkout / "artifacts" / name
        if link.exists() or link.is_symlink():
            if link.resolve() != target.resolve():
                raise WorkflowError(f"Existing lock does not refer to the shared resource: {link}")
        else:
            link.symlink_to(target)
    for relative in ["local.properties", "third_party/.cache"]:
        target, link = repo.primary / relative, checkout / relative
        if target.exists() and not link.exists() and not link.is_symlink():
            link.parent.mkdir(parents=True, exist_ok=True)
            link.symlink_to(target, target_is_directory=target.is_dir())


def packet(task: dict) -> str:
    paths = ", ".join(f"`{p}`" for p in task["owns"])
    checks = ", ".join(f"`{name}`" for name in task["checks"]) or "the checks agreed with the coordinator"
    return f'''# {task['name']}

{task['objective']}

Work in `{task['checkout']}` on `{task['branch']}`, starting at `{task['base']}`.
Read AGENTS.md, the relevant product and UX documents, and docs/engine/protocol.md before editing contracts.
Your exclusive source ownership is {paths}. Ask the coordinator to transfer ownership before changing another area.
Other agents have separate checkouts; do not copy their uncommitted files or edit their working trees.

Validate with {checks}, using `tools/workflow check --task {task['name']} SUITE`.
Heavy builds share one lock across checkouts. Android tests consume a frozen APK pair through the one emulator lease;
never start another emulator directly. Test results apply only to their recorded source fingerprint.

Commit your source changes locally, then use `tools/workflow handoff {task['name']} --summary-file PATH`.
The summary should explain the resulting behavior, important implementation choices, validation and known limits in natural English.
Deliver once when the bounded task is complete; progress messages are only needed for a concrete blocker or a changed contract.
The coordinator reviews the diff and integrates the recorded commit, then runs the checks required by the combined change.

When creating a new subagent, use fork_turns="none" and give it its own bounded task packet.
Do not read old Codex transcripts. User instructions and repository safety rules remain authoritative.
'''


def create_task(repo: Repository, name: str, objective: str, scopes: list[str], checks: list[str], base: str):
    name = identifier(name)
    if name == "coordinator":
        raise WorkflowError("coordinator is reserved for integration checks")
    scopes = sorted(set(map(scope_path, scopes)))
    if not checks:
        raise WorkflowError("Choose at least one required suite, such as documentation")
    if not scopes or not objective.strip():
        raise WorkflowError("A task needs an objective and at least one owned source path")
    repo.initialize()
    commit = git(repo.root, "rev-parse", "--verify", f"{base}^{{commit}}", check=False)
    if not commit or len(commit) not in (40, 64) or any(c not in "0123456789abcdef" for c in commit):
        raise WorkflowError(f"Base {base!r} is not a commit; commit the initial source before creating isolated tasks")
    branch = "codex/" + name
    checkout = repo.state / "checkouts" / name
    with lock(repo.state / "registry.lock"):
        if task_file(repo, name).exists() or checkout.exists():
            raise WorkflowError(f"Task {name!r} already exists; do not reuse its identity")
        for other in all_tasks(repo):
            if other["status"] in {"active", "ready", "integrating"} and overlap(scopes, other["owns"]):
                raise WorkflowError(f"Ownership overlaps active task {other['name']}: {other['owns']}")
        task = {"format": 1, "name": name, "objective": objective.strip(), "owns": scopes,
                "checks": checks, "base": commit, "branch": branch, "checkout": str(checkout),
                "status": "active", "createdAt": int(time.time())}
        git(repo.root, "worktree", "add", "-b", branch, str(checkout), commit)
        try:
            setup_links(repo, checkout)
            atomic_json(task_file(repo, name), task)
            (repo.tasks / (name + ".md")).write_text(packet(task), encoding="utf-8")
        except Exception:
            # Source checkout is retained if setup failed; never force-remove someone else's edits.
            task["status"] = "setup_failed"
            atomic_json(task_file(repo, name), task)
            raise
    return task


def verify_scope(task: dict) -> list[str]:
    paths = changed_paths(Path(task["checkout"]), task["base"])
    outside = [path for path in paths if not owns(task["owns"], path)]
    if outside:
        raise WorkflowError("Changed paths outside task ownership: " + ", ".join(outside))
    return paths


def prepare_inputs(repo: Repository, task: dict):
    checkout = Path(task["checkout"])
    setup_links(repo, checkout)
    copied = []
    # Images are snapshots, not writable links to another agent's image output.
    with lock(repo.state / "task-locks" / (task["name"] + ".lock")), lock(repo.build_lock):
        for arch in ["arm64", "amd64"]:
            for name in ["image.tar.zst", "image.json"]:
                source = repo.primary / "artifacts/image" / arch / name
                if not source.is_file():
                    raise WorkflowError(f"Build the customized image first: {source}")
                target = checkout / "artifacts/image" / arch / name
                target.parent.mkdir(parents=True, exist_ok=True)
                if source.resolve() != target.resolve():
                    subprocess.run(["cp", "--reflink=auto", "--", str(source), str(target)], check=True)
                copied.append(str(target))
    return copied


def handoff(repo: Repository, name: str, summary: Path):
    with lock(repo.state / "registry.lock"), lock(repo.state / "task-locks" / (name + ".lock"), blocking=False):
        task = load_task(repo, name)
        if task["status"] != "active":
            raise WorkflowError("Only an active task can be handed off")
        root = Path(task["checkout"])
        paths = verify_scope(task)
        if git(root, "status", "--porcelain"):
            raise WorkflowError("Commit the owned source changes before handing off")
        head = git(root, "rev-parse", "HEAD")
        if head == task["base"] or not paths:
            raise WorkflowError("A handoff must contain a source commit")
        snapshot = source_snapshot(root)
        from .runner import inputs_match
        runs = [read_json(p) for p in sorted(repo.runs.glob(f"{name}/*/run.json"))]
        verified = {run["suite"]: run["id"] for run in runs if run["status"] == "passed" and run["source"]["fingerprint"] == snapshot["fingerprint"] and inputs_match(root, run.get("inputs", {}))}
        missing = set(task["checks"]) - set(verified)
        if missing:
            raise WorkflowError("Current source lacks successful checks: " + ", ".join(sorted(missing)))
        text = summary.read_text(encoding="utf-8").strip()
        if not text:
            raise WorkflowError("The handoff summary is empty")
        delivery_id = run_id()
        history_path = repo.tasks / name / "handoffs" / (delivery_id + ".md")
        history_path.parent.mkdir(parents=True, exist_ok=True)
        history_path.write_text(text + "\n", encoding="utf-8")
        reference = "refs/workflow/handoffs/" + name + "/" + delivery_id
        git(repo.root, "update-ref", reference, head)
        task.setdefault("handoffs", []).append({"head": head, "reference": reference, "summary": str(history_path), "evidence": verified})
        task.update(status="ready", head=head, source=snapshot["fingerprint"], evidence=verified,
                    changedPaths=paths, deliveredAt=int(time.time()))
        (repo.tasks / (name + "-handoff.md")).write_text(text + "\n", encoding="utf-8")
        atomic_json(task_file(repo, name), task)
        return task


def integrate(repo: Repository, name: str):
    with lock(repo.state / "registry.lock"), lock(repo.state / "integration.lock"):
        task = load_task(repo, name)
        if task["status"] != "ready":
            raise WorkflowError("Review a completed handoff before integrating it")
        if repo.root == Path(task["checkout"]):
            raise WorkflowError("Integrate from the coordinator checkout, not the task checkout")
        if git(repo.root, "status", "--porcelain"):
            raise WorkflowError("The coordinator checkout must be clean before integration")
        if git(Path(task["checkout"]), "rev-parse", "HEAD") != task["head"] or source_snapshot(Path(task["checkout"]))["fingerprint"] != task["source"]:
            raise WorkflowError("The delivered checkout changed after handoff")
        verify_scope(task)
        task["status"] = "integrating"
        atomic_json(task_file(repo, name), task)
        result = subprocess.run(["git", "-C", str(repo.root), "merge", "--no-ff", "--no-edit", task["head"]])
        if result.returncode:
            raise WorkflowError("Merge stopped. Resolve or abort it explicitly; no files were discarded. The task remains integrating.")
        task.update(status="integrated", integratedAt=int(time.time()), integrationCommit=git(repo.root, "rev-parse", "HEAD"))
        atomic_json(task_file(repo, name), task)
        return task


def archive_task(repo: Repository, name: str):
    with lock(repo.state / "registry.lock"), lock(repo.state / "task-locks" / (name + ".lock"), blocking=False):
        task = load_task(repo, name)
        if task["status"] not in {"integrated", "cancelled"}:
            raise WorkflowError("Only an integrated or cancelled clean task can be archived; its branch and evidence are retained")
        root = Path(task["checkout"])
        if root.exists():
            if git(root, "status", "--porcelain"):
                raise WorkflowError("The checkout has uncommitted source; it was not removed")
            git(repo.root, "worktree", "remove", str(root))
        task.update(status="archived", archivedAt=int(time.time()))
        atomic_json(task_file(repo, name), task)
        return task


def change_scope(repo: Repository, name: str, scopes: list[str] | None, checks: list[str] | None = None,
                 share_paths: list[str] | None = None, reason: str | None = None):
    with lock(repo.state / "registry.lock"):
        task = load_task(repo, name)
        scopes = sorted(set(map(scope_path, scopes))) if scopes is not None else task["owns"]
        if not scopes:
            raise WorkflowError("A task needs at least one owned path")
        if task["status"] != "active":
            raise WorkflowError("Only active tasks can change ownership")
        shared = set(task.get("sharedPaths", []))
        if share_paths:
            if not reason or not reason.strip():
                raise WorkflowError("Explicit shared-file coordination needs a recorded reason")
            for path in map(scope_path, share_paths):
                checkout = Path(task["checkout"])
                is_file = (checkout / path).is_file() or git(checkout, "cat-file", "-t", task["base"] + ":" + path, check=False) == "blob"
                if path not in scopes or not is_file:
                    raise WorkflowError("Only an explicitly claimed source file can be shared")
                shared.add(path)
        shared &= set(scopes)
        for other in all_tasks(repo):
            if other["name"] != name and other["status"] in {"active", "ready", "integrating"}:
                conflicts = [path for path in scopes if overlap([path], other["owns"]) and path not in shared]
                if conflicts:
                    raise WorkflowError(f"Ownership overlaps {other['name']}: {conflicts}")
        outside = [p for p in changed_paths(Path(task["checkout"]), task["base"]) if not owns(scopes, p)]
        if outside:
            raise WorkflowError("Cannot release paths that already have changes: " + ", ".join(outside))
        if checks is not None:
            catalog = read_json(Path(task["checkout"]) / "tools/workflow-suites.json")["suites"]
            if not checks or set(checks) - (set(catalog) | {"android"}):
                raise WorkflowError("Required checks must name existing suites")
            task["checks"] = sorted(set(checks))
        task["owns"] = scopes
        if shared:
            task["sharedPaths"] = sorted(shared)
        else:
            task.pop("sharedPaths", None)
        if share_paths:
            task.setdefault("decisions", []).append({"action": "share-files", "paths": sorted(set(share_paths)),
                                                   "reason": reason.strip(), "at": int(time.time())})
        atomic_json(task_file(repo, name), task)
        (repo.tasks / (name + ".md")).write_text(packet(task), encoding="utf-8")
        return task


def finish_integration(repo: Repository, name: str):
    with lock(repo.state / "registry.lock"), lock(repo.state / "integration.lock"):
        task = load_task(repo, name)
        if task["status"] != "integrating":
            raise WorkflowError("This task has no interrupted integration")
        if git(repo.root, "rev-parse", "--verify", "MERGE_HEAD", check=False):
            raise WorkflowError("Commit the resolved merge or abort it before finishing")
        if git(repo.root, "status", "--porcelain"):
            raise WorkflowError("The coordinator checkout must be clean")
        included = subprocess.run(["git", "-C", str(repo.root), "merge-base", "--is-ancestor", task["head"], "HEAD"]).returncode == 0
        if included:
            task.update(status="integrated", integrationCommit=git(repo.root, "rev-parse", "HEAD"), integratedAt=int(time.time()))
        else:
            task["status"] = "ready"
        atomic_json(task_file(repo, name), task)
        return task


def revise_task(repo: Repository, name: str, action: str, reason: str):
    if not reason.strip(): raise WorkflowError("Record the reason for this lifecycle change")
    with lock(repo.state / "registry.lock"), lock(repo.state / "task-locks" / (name + ".lock"), blocking=False):
        task = load_task(repo, name)
        if action == "cancel":
            if task["status"] not in {"active", "ready", "setup_failed"}:
                raise WorkflowError("Finish or abort any integration before cancelling a task")
            task["status"] = "cancelled"
        else:
            if task["status"] not in {"ready", "cancelled"}:
                raise WorkflowError("Only a ready or cancelled checkout can be reopened")
            for other in all_tasks(repo):
                if other["name"] != name and other["status"] in {"active", "ready", "integrating"} and overlap(task["owns"], other["owns"]):
                    raise WorkflowError(f"Ownership is now held by {other['name']}")
            verify_scope(task)
            task["status"] = "active"
        task.setdefault("decisions", []).append({"action": action, "reason": reason.strip(), "at": int(time.time())})
        atomic_json(task_file(repo, name), task)
        return task
