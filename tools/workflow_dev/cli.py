from __future__ import annotations

import argparse
import json
from pathlib import Path
import signal
import sys

from .common import Repository, WorkflowError
from .runner import run_check, run_device, suites
from .tasks import all_tasks, archive_task, change_scope, create_task, finish_integration, handoff, integrate, load_task, prepare_inputs, revise_task, setup_links, verify_scope


def parser():
    root = argparse.ArgumentParser(description="Isolated work, verifiable handoffs and one shared Android emulator")
    root.add_argument("--repo", type=Path, default=Path.cwd(), help="a checkout in this repository")
    sub = root.add_subparsers(dest="command", required=True)
    sub.add_parser("init", help="initialize local coordination records")
    sub.add_parser("status", help="show task ownership and run locations")
    tasks = sub.add_parser("task").add_subparsers(dest="task_command", required=True)
    create = tasks.add_parser("create", help="reserve paths and create an isolated worktree")
    create.add_argument("name"); create.add_argument("--objective", required=True)
    create.add_argument("--owns", action="append", required=True)
    create.add_argument("--checks", action="append", default=[])
    create.add_argument("--base", default="HEAD")
    for action in ["reopen", "cancel"]:
        change = tasks.add_parser(action)
        change.add_argument("name"); change.add_argument("--reason", required=True)
    tasks.add_parser("list")
    show = tasks.add_parser("show"); show.add_argument("name")
    scope = tasks.add_parser("scope"); scope.add_argument("name"); scope.add_argument("--owns", action="append"); scope.add_argument("--checks", action="append"); scope.add_argument("--share-path", action="append", default=[]); scope.add_argument("--reason")
    verify = tasks.add_parser("verify"); verify.add_argument("name")
    prepare = sub.add_parser("prepare", help="snapshot image inputs and link machine-local caches")
    prepare.add_argument("name")
    check = sub.add_parser("check", help="run a named suite and bind its result to the source snapshot")
    check.add_argument("suite"); check.add_argument("--task")
    device = sub.add_parser("device", help="test an immutable APK pair on the shared disposable emulator")
    device.add_argument("--apk-run", type=Path, required=True)
    device.add_argument("--task"); device.add_argument("--class", dest="classes", action="append", default=[])
    device.add_argument("--arg", dest="arguments", action="append", default=[])
    device.add_argument("--root", action="store_true", help="request adb root on the disposable emulator only")
    device.add_argument("--allow-skips", action="store_true")
    delivery = sub.add_parser("handoff", help="record a clean, scoped, checked commit for review")
    delivery.add_argument("name"); delivery.add_argument("--summary-file", type=Path, required=True)
    for command in ["integrate", "finish", "archive"]:
        sub.add_parser(command).add_argument("name")
    return root


def main(argv=None):
    def interrupt(_signal, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, interrupt)
    signal.signal(signal.SIGHUP, interrupt)
    args = parser().parse_args(argv)
    try:
        repo = Repository(args.repo)
        if args.command == "init":
            repo.initialize()
            setup_links(repo, repo.root)
            result = {"primary": str(repo.primary), "records": str(repo.state), "buildLock": str(repo.build_lock), "deviceLock": str(repo.device_lock)}
        elif args.command == "status":
            result = {"tasks": all_tasks(repo), "runs": str(repo.runs), "buildLock": str(repo.build_lock), "deviceLock": str(repo.device_lock)}
        elif args.command == "task":
            if args.task_command == "create":
                unknown = set(args.checks) - (set(suites(repo.root)) | {"android"})
                if unknown:
                    raise WorkflowError("Unknown required suites: " + ", ".join(sorted(unknown)))
                result = create_task(repo, args.name, args.objective, args.owns, args.checks, args.base)
            elif args.task_command == "list": result = all_tasks(repo)
            elif args.task_command == "show": result = load_task(repo, args.name)
            elif args.task_command == "scope": result = change_scope(repo, args.name, args.owns, args.checks, args.share_path, args.reason)
            elif args.task_command in {"reopen", "cancel"}: result = revise_task(repo, args.name, args.task_command, args.reason)
            else: result = {"changedPaths": verify_scope(load_task(repo, args.name))}
        elif args.command == "prepare": result = {"inputs": prepare_inputs(repo, load_task(repo, args.name))}
        elif args.command == "check":
            result = run_check(repo, args.suite, args.task)
            return 0 if result["status"] == "passed" else 1
        elif args.command == "device":
            result = run_device(repo, args.apk_run, args.task, args.classes, args.arguments, args.root, args.allow_skips)
            return 0 if result["status"] == "passed" else 1
        elif args.command == "handoff": result = handoff(repo, args.name, args.summary_file)
        elif args.command == "integrate": result = integrate(repo, args.name)
        elif args.command == "finish": result = finish_integration(repo, args.name)
        else: result = archive_task(repo, args.name)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0
    except KeyboardInterrupt:
        print("Cancelled; owned test processes were stopped and evidence retained.", file=sys.stderr)
        return 130
    except (WorkflowError, OSError) as error:
        print(f"workflow: {error}", file=sys.stderr)
        return 1
