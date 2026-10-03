from __future__ import annotations

import contextlib
import os
from pathlib import Path
import shutil
import signal
import subprocess
import time

from .common import Repository, WorkflowError, atomic_json, digest, lock, read_json, run_id, source_snapshot
from .tasks import load_task, verify_scope


def suites(root: Path) -> dict:
    document = read_json(root / "tools/workflow-suites.json")
    if document.get("version") != 1 or not isinstance(document.get("suites"), dict):
        raise WorkflowError("Unsupported suite catalog")
    return document["suites"]


def execute(command: list[str], cwd: Path, environment: dict, log: Path) -> int:
    with log.open("wb") as output:
        process = subprocess.Popen(command, cwd=cwd, env=environment, stdin=subprocess.DEVNULL,
                                   stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            return process.wait()
        except BaseException:
            with contextlib.suppress(ProcessLookupError):
                os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                with contextlib.suppress(ProcessLookupError):
                    os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            raise


def task_context(repo: Repository, task_name: str | None):
    if task_name:
        task = load_task(repo, task_name)
        if task["status"] != "active":
            raise WorkflowError("Validation requires an active task")
        verify_scope(task)
        return Path(task["checkout"]), task_name
    return repo.root, "coordinator"


def copy_apks(root: Path, output: Path) -> dict:
    paths = {"app": "app/android/build/outputs/apk/x86_64/debug/android-x86_64-debug.apk",
             "test": "app/android/build/outputs/apk/androidTest/x86_64/debug/android-x86_64-debug-androidTest.apk"}
    output.mkdir(parents=True)
    result = {}
    for name, relative in paths.items():
        source = root / relative
        if not source.is_file():
            raise WorkflowError(f"Build did not produce {source}")
        target = output / (name + ".apk")
        shutil.copyfile(source, target)
        target.chmod(0o444)
        result[name] = {"file": str(target), "sha256": digest(target), "bytes": target.stat().st_size}
    return result


def build_inputs(root: Path, apk: bool) -> dict:
    if not apk: return {}
    records = {}
    for arch in ("amd64", "arm64"):
        for name in ("image.json", "image.tar.zst"):
            relative = "artifacts/image/" + arch + "/" + name
            path = root / relative
            if path.is_file(): records[relative] = {"sha256": digest(path), "bytes": path.stat().st_size}
    return records


def inputs_match(root: Path, inputs: dict) -> bool:
    for relative, expected in inputs.items():
        path = root / relative
        if not path.is_file() or path.stat().st_size != expected["bytes"] or digest(path) != expected["sha256"]:
            return False
    return True


def run_check(repo: Repository, suite_name: str, task_name: str | None = None) -> dict:
    repo.initialize()
    root, owner = task_context(repo, task_name)
    catalog = suites(root)
    if suite_name not in catalog:
        raise WorkflowError(f"Unknown suite {suite_name!r}; choose from {', '.join(catalog)}")
    suite = catalog[suite_name]
    command = suite["command"]
    if not isinstance(command, list) or not command or not all(isinstance(arg, str) for arg in command):
        raise WorkflowError("Suite commands must be argv arrays")
    cwd = (root / suite.get("cwd", ".")).resolve()
    if not cwd.is_relative_to(root):
        raise WorkflowError("Suite working directory must be within its checkout")
    folder = repo.runs / owner / run_id()
    folder.mkdir(parents=True)
    result = {"format": 1, "id": folder.name, "owner": owner, "suite": suite_name,
              "checkout": str(root), "command": command, "status": "waiting", "startedAt": int(time.time())}
    atomic_json(folder / "run.json", result)
    print(f"Run {folder}; waiting for its checkout and resource lease", flush=True)
    try:
        with lock(repo.state / "task-locks" / (owner + ".lock")):
            resource = lock(repo.build_lock) if suite.get("heavy") else contextlib.nullcontext()
            with resource:
                if task_name:
                    verify_scope(load_task(repo, task_name))
                before = source_snapshot(root)
                result.update(status="running", source=before, inputs=build_inputs(root, suite.get("apkSnapshot", False)))
                atomic_json(folder / "run.json", result)
                environment = os.environ.copy()
                environment.update(suite.get("env", {}))
                environment.update({"CARGO_BUILD_JOBS": "2", "WORKFLOW_COORDINATION_ROOT": str(repo.primary)})
                if suite.get("heavy"):
                    environment["WORKFLOW_BUILD_LOCK_HELD"] = "1"
                else:
                    environment.pop("WORKFLOW_BUILD_LOCK_HELD", None)
                # The host runtime oracle takes its own disposable copy of this immutable fixture.
                if suite_name == "runtime-host":
                    environment.setdefault("ENGINE_ROOTFS", str(repo.primary / "artifacts/engine/rootfs-amd64"))
                result["exitCode"] = execute(command, cwd, environment, folder / "output.log")
                after = source_snapshot(root)
                result["finishedSource"] = after["fingerprint"]
                result["status"] = "passed" if result["exitCode"] == 0 else "failed"
                if before["fingerprint"] != after["fingerprint"] or not inputs_match(root, result["inputs"]):
                    result["status"] = "source_changed"
                if result["status"] == "passed" and suite.get("apkSnapshot"):
                    result["apks"] = copy_apks(root, folder / "apk")
                for relative in suite.get("reports", []):
                    source = (root / relative).resolve()
                    if not source.is_relative_to(root):
                        raise WorkflowError("Report path escaped its checkout")
                    if source.is_dir():
                        shutil.copytree(source, folder / "reports" / relative, dirs_exist_ok=True)
                    elif source.is_file():
                        target = folder / "reports" / relative
                        target.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(source, target)
    except (KeyboardInterrupt, SystemExit):
        result["status"] = "cancelled"
        raise
    except Exception as error:
        result.update(status="failed", error=str(error))
    finally:
        result["finishedAt"] = int(time.time())
        atomic_json(folder / "run.json", result)
    print(f"{suite_name}: {result['status']} ({folder / 'output.log'})", flush=True)
    return result


def apk_manifest(path: Path) -> dict:
    run = read_json(path / "run.json" if path.is_dir() else path)
    if run.get("status") != "passed" or set(run.get("apks", {})) != {"app", "test"}:
        raise WorkflowError("Device acceptance requires a successful frozen app/test APK build")
    for value in run["apks"].values():
        file = Path(value["file"])
        if not file.is_file() or file.stat().st_size != value["bytes"] or digest(file) != value["sha256"]:
            raise WorkflowError(f"Frozen APK is missing or changed: {file}")
    return run


def run_device(repo: Repository, apk_run: Path, task_name: str | None, classes: list[str], arguments: list[str], rooted: bool, allow_skips: bool = False) -> dict:
    repo.initialize()
    root, owner = task_context(repo, task_name)
    build = apk_manifest(apk_run)
    folder = repo.runs / owner / run_id()
    folder.mkdir(parents=True)
    # Copy the driver before queuing so source edits cannot change a running/queued script.
    shutil.copyfile(Path(__file__).with_name("device_driver.py"), folder / "device_driver.py")
    for value in arguments:
        if "=" not in value or not value.split("=", 1)[0]:
            raise WorkflowError("Instrumentation arguments use KEY=VALUE")
    result = {"format": 1, "id": folder.name, "owner": owner, "suite": "android", "status": "waiting",
              "startedAt": int(time.time()), "checkout": str(root), "apkBuild": build["id"],
              "apks": build["apks"], "classes": classes, "arguments": arguments, "rooted": rooted,
              "allowSkips": allow_skips}
    atomic_json(folder / "run.json", result)
    print(f"Device run {folder}; waiting for the shared emulator", flush=True)
    try:
        with lock(repo.state / "task-locks" / (owner + ".lock")):
            before = source_snapshot(root)
            result["source"] = before
            result["inputs"] = build.get("inputs", {})
            if not inputs_match(root, result["inputs"]):
                raise WorkflowError("Image inputs changed after the APK snapshot; build a new pair")
            if before["fingerprint"] != build["source"]["fingerprint"]:
                raise WorkflowError("The checkout differs from the APK source snapshot; build a new pair")
            result["status"] = "running"
            atomic_json(folder / "run.json", result)
            environment = os.environ.copy()
            environment["WORKFLOW_COORDINATION_ROOT"] = str(repo.primary)
            command = [str(root / "tools/with-emulator.sh"), "python3", str(folder / "device_driver.py"), str(folder / "run.json")]
            result["exitCode"] = execute(command, root, environment, folder / "output.log")
            result["status"] = "passed" if result["exitCode"] == 0 else "failed"
            if source_snapshot(root)["fingerprint"] != before["fingerprint"] or not inputs_match(root, result["inputs"]):
                result["status"] = "source_changed"
            apk_manifest(apk_run)
            if (folder / "device-result.json").is_file():
                result["device"] = read_json(folder / "device-result.json")
    except (KeyboardInterrupt, SystemExit):
        result["status"] = "cancelled"
        raise
    except Exception as error:
        result.update(status="failed", error=str(error))
    finally:
        result["finishedAt"] = int(time.time())
        atomic_json(folder / "run.json", result)
    print(f"android: {result['status']} ({folder / 'output.log'})", flush=True)
    return result
