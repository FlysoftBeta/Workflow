from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from workflow_dev.common import Repository, WorkflowError, atomic_json, changed_paths, git, lock, overlap, scope_path, source_snapshot
from workflow_dev.tasks import archive_task, change_scope, create_task, finish_integration, handoff, integrate, load_task, prepare_inputs, revise_task, verify_scope
from workflow_dev.runner import apk_manifest, run_check
from workflow_dev.device_driver import instrumentation_result

CLI = Path(__file__).resolve().parents[1] / "workflow"


class WorktreeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="workflow-infra-")
        self.root = Path(self.temporary.name) / "repo with spaces"
        self.root.mkdir()
        subprocess.run(["git", "init", "-q", "-b", "main", str(self.root)], check=True)
        for key, value in [("user.name", "Infrastructure Test"), ("user.email", "test@example.invalid"), ("commit.gpgsign", "false")]:
            git(self.root, "config", key, value)
        (self.root / ".gitignore").write_text("/artifacts/\n/local.properties\n/third_party/.cache/\n**/__pycache__/\n")
        for name in ["a", "b", "tools"]:
            (self.root / name).mkdir()
        (self.root / "a/file").write_text("base a\n")
        (self.root / "b/file").write_text("base b\n")
        self.catalog = {"version": 1, "suites": {"tiny": {"command": [sys.executable, "-c", "print('verified')"], "heavy": False}}}
        self.write_catalog()
        git(self.root, "add", "."); git(self.root, "commit", "-qm", "base")
        self.repo = Repository(self.root)

    def tearDown(self):
        self.temporary.cleanup()

    def write_catalog(self):
        (self.root / "tools/workflow-suites.json").write_text(json.dumps(self.catalog))

    def task(self, name="alpha", owns=None, checks=None):
        return create_task(self.repo, name, "Improve one module.", owns or ["a"], checks if checks is not None else ["tiny"], "HEAD")

    def commit_change(self, task, value="changed\n"):
        root = Path(task["checkout"])
        (root / "a/file").write_text(value)
        git(root, "add", "a/file"); git(root, "commit", "-qm", "change")

    def summary(self):
        path = Path(self.temporary.name) / "handoff.md"
        path.write_text("The owned module now has the requested behavior. The tiny check passed.\n")
        return path

    def test_review_revision_preserves_the_previous_handoff(self):
        task = self.task(); self.commit_change(task)
        run_check(self.repo, "tiny", "alpha")
        first = handoff(self.repo, "alpha", self.summary())
        revise_task(self.repo, "alpha", "reopen", "Address review feedback")
        self.commit_change(task, "revised\n")
        run_check(self.repo, "tiny", "alpha")
        second = handoff(self.repo, "alpha", self.summary())
        self.assertEqual(2, len(second["handoffs"]))
        self.assertEqual(first["head"], git(self.root, "rev-parse", first["handoffs"][0]["reference"]))
        self.assertNotEqual(first["head"], second["head"])

    def test_cancel_releases_ownership_but_keeps_dirty_source(self):
        task = self.task(); (Path(task["checkout"]) / "a/file").write_text("unfinished")
        revise_task(self.repo, "alpha", "cancel", "Superseded by a different approach")
        self.task("beta", ["a"])
        with self.assertRaises(WorkflowError): revise_task(self.repo, "alpha", "reopen", "Resume")
        with self.assertRaises(WorkflowError): archive_task(self.repo, "alpha")
        self.assertEqual("unfinished", (Path(task["checkout"]) / "a/file").read_text())
        git(Path(task["checkout"]), "add", "."); git(Path(task["checkout"]), "commit", "-qm", "retain unfinished work")
        self.assertEqual("archived", archive_task(self.repo, "alpha")["status"])

    def test_path_ownership_rejects_overlap_and_traversal(self):
        for value in ["../a", "/a", ".git/config", "artifacts/a", "a/*", "a\nb", "."]:
            with self.assertRaises(WorkflowError): scope_path(value)
        self.task()
        with self.assertRaises(WorkflowError): self.task("beta", ["a/file"])
        self.task("beta", ["b"])
        self.assertFalse(overlap(["app"], ["app2"]))

    def test_scope_can_update_checks_without_releasing_claims(self):
        task = self.task()
        changed = change_scope(self.repo, "alpha", None, ["tiny", "android"])
        self.assertEqual(task["owns"], changed["owns"])
        self.assertEqual(["android", "tiny"], changed["checks"])
        with self.assertRaises(WorkflowError): change_scope(self.repo, "alpha", None, ["missing"])
        self.assertEqual(["android", "tiny"], load_task(self.repo, "alpha")["checks"])

    def test_shared_claim_requires_an_explicit_file_and_recorded_reason(self):
        self.task()
        self.task("beta", ["b"])
        with self.assertRaises(WorkflowError): change_scope(self.repo, "alpha", ["a", "b/file"])
        with self.assertRaises(WorkflowError): change_scope(self.repo, "alpha", ["a", "b"], share_paths=["b"], reason="coordinated")
        with self.assertRaises(WorkflowError): change_scope(self.repo, "alpha", ["a", "b/file"], share_paths=["b/file"])
        task = change_scope(self.repo, "alpha", ["a", "b/file"], share_paths=["b/file"], reason="Owner-approved one-line documentation fix")
        self.assertEqual(["b/file"], task["sharedPaths"])
        self.assertEqual("share-files", task["decisions"][-1]["action"])
        self.assertEqual(["b"], change_scope(self.repo, "beta", ["b"])["owns"])
        # A shared claim does not release the original owner's remaining exclusive directory.
        with self.assertRaises(WorkflowError): self.task("gamma", ["b/file"])
        with self.assertRaises(WorkflowError): change_scope(self.repo, "alpha", ["a", "b"])

    def test_worktrees_share_lock_identity_but_not_checkout_output(self):
        a = self.task(); b = self.task("beta", ["b"])
        left, right = Repository(Path(a["checkout"])), Repository(Path(b["checkout"]))
        self.assertEqual(left.primary, self.root)
        self.assertEqual(left.device_lock, right.device_lock)
        self.assertEqual(os.stat(Path(a["checkout"]) / "artifacts/.gradle.lock").st_ino, os.stat(right.build_lock).st_ino)
        with lock(left.device_lock):
            with self.assertRaises(WorkflowError):
                with lock(right.device_lock, blocking=False): pass
        (Path(a["checkout"]) / "artifacts/local-output").write_text("only alpha")
        self.assertFalse((Path(b["checkout"]) / "artifacts/local-output").exists())

    def test_handoff_requires_current_tests_and_scoped_clean_commit(self):
        task = self.task(); self.commit_change(task)
        with self.assertRaises(WorkflowError): handoff(self.repo, "alpha", self.summary())
        self.assertEqual("passed", run_check(self.repo, "tiny", "alpha")["status"])
        (Path(task["checkout"]) / "a/file").write_text("new source")
        git(Path(task["checkout"]), "add", "a/file"); git(Path(task["checkout"]), "commit", "-qm", "new")
        with self.assertRaises(WorkflowError): handoff(self.repo, "alpha", self.summary())
        run_check(self.repo, "tiny", "alpha")
        delivered = handoff(self.repo, "alpha", self.summary())
        self.assertEqual("ready", delivered["status"])
        self.assertEqual(["a/file"], delivered["changedPaths"])

    def test_source_edit_during_successful_command_taints_result(self):
        self.catalog["suites"]["mutate"] = {"command": [sys.executable, "-c", "from pathlib import Path; Path('a/file').write_text('changed')"], "heavy": False}
        self.write_catalog(); git(self.root, "add", "."); git(self.root, "commit", "-qm", "catalog")
        result = run_check(self.repo, "mutate")
        self.assertEqual(0, result["exitCode"])
        self.assertEqual("source_changed", result["status"])

    def test_rename_cannot_hide_an_unowned_deleted_path(self):
        task = self.task(); root = Path(task["checkout"])
        git(root, "mv", "b/file", "a/moved")
        self.assertIn("b/file", changed_paths(root, task["base"]))
        with self.assertRaises(WorkflowError): verify_scope(task)

    def test_scope_cannot_drop_changes_or_steal_another_task(self):
        task = self.task(); self.task("beta", ["b"])
        with self.assertRaises(WorkflowError): change_scope(self.repo, "alpha", ["b"])
        self.commit_change(task)
        with self.assertRaises(WorkflowError): change_scope(self.repo, "alpha", ["tools"])
        self.assertEqual(["a", "new-module"], change_scope(self.repo, "alpha", ["a", "new-module"])["owns"])

    def test_integration_and_archive_keep_commit_and_evidence(self):
        task = self.task(); self.commit_change(task)
        run_check(self.repo, "tiny", "alpha"); handoff(self.repo, "alpha", self.summary())
        merged = integrate(self.repo, "alpha")
        self.assertEqual("integrated", merged["status"])
        self.assertEqual("changed\n", (self.root / "a/file").read_text())
        archived = archive_task(self.repo, "alpha")
        self.assertEqual("archived", archived["status"])
        self.assertFalse(Path(task["checkout"]).exists())
        self.assertTrue(git(self.root, "rev-parse", "refs/heads/codex/alpha"))
        self.assertTrue(list(self.repo.runs.glob("alpha/*/run.json")))

    def test_conflicting_integration_preserves_resolution_and_can_be_aborted(self):
        task = self.task(); self.commit_change(task, "agent\n")
        run_check(self.repo, "tiny", "alpha"); handoff(self.repo, "alpha", self.summary())
        (self.root / "a/file").write_text("coordinator\n")
        git(self.root, "add", "."); git(self.root, "commit", "-qm", "concurrent")
        with self.assertRaises(WorkflowError): integrate(self.repo, "alpha")
        self.assertEqual("integrating", load_task(self.repo, "alpha")["status"])
        self.assertIn("<<<<<<<", (self.root / "a/file").read_text())
        git(self.root, "merge", "--abort")
        self.assertEqual("ready", finish_integration(self.repo, "alpha")["status"])
        self.assertEqual("coordinator\n", (self.root / "a/file").read_text())

    def test_apk_snapshot_is_frozen_and_tampering_is_rejected(self):
        app = "app/android/build/outputs/apk/x86_64/debug/android-x86_64-debug.apk"
        test = "app/android/build/outputs/apk/androidTest/x86_64/debug/android-x86_64-debug-androidTest.apk"
        (self.root / ".gitignore").write_text((self.root / ".gitignore").read_text() + "**/build/\n")
        code = f"from pathlib import Path; paths={[app,test]!r}; [(Path(p).parent.mkdir(parents=True, exist_ok=True),Path(p).write_bytes(b'APK')) for p in paths]"
        self.catalog["suites"]["apk"] = {"command": [sys.executable, "-c", code], "heavy": True, "apkSnapshot": True}
        self.write_catalog(); git(self.root, "add", "."); git(self.root, "commit", "-qm", "apk fixture")
        result = run_check(self.repo, "apk")
        folder = self.repo.runs / "coordinator" / result["id"]
        self.assertEqual("passed", apk_manifest(folder)["status"])
        frozen = Path(result["apks"]["app"]["file"])
        (self.root / app).write_bytes(b"new build")
        self.assertEqual(b"APK", frozen.read_bytes())
        frozen.chmod(0o644); frozen.write_bytes(b"tampered")
        with self.assertRaises(WorkflowError): apk_manifest(folder)

    def test_prepare_copies_image_inputs_instead_of_writable_aliases(self):
        task = self.task()
        for arch in ["arm64", "amd64"]:
            directory = self.root / "artifacts/image" / arch; directory.mkdir(parents=True)
            (directory / "image.json").write_text("{}")
            (directory / "image.tar.zst").write_bytes(b"snapshot")
        prepare_inputs(self.repo, task)
        target = Path(task["checkout"]) / "artifacts/image/arm64/image.tar.zst"
        self.assertFalse(target.is_symlink())
        target.write_bytes(b"owned change")
        self.assertEqual(b"snapshot", (self.root / "artifacts/image/arm64/image.tar.zst").read_bytes())

    def test_heavy_suites_serialize_across_worktrees(self):
        marker = Path(self.temporary.name) / "occupied"
        script = "import pathlib,time; p=pathlib.Path(" + repr(str(marker)) + "); f=p.open('x'); time.sleep(.25); f.close(); p.unlink()"
        self.catalog["suites"]["heavy"] = {"command": [sys.executable, "-c", script], "heavy": True}
        self.write_catalog(); git(self.root, "add", "."); git(self.root, "commit", "-qm", "suite")
        self.task(); self.task("beta", ["b"])
        with ThreadPoolExecutor(2) as pool:
            results = list(pool.map(lambda name: run_check(self.repo, "heavy", name), ["alpha", "beta"]))
        self.assertEqual(["passed", "passed"], [r["status"] for r in results])

    def test_cli_cancellation_reaps_its_child_and_records_failure(self):
        pidfile = Path(self.temporary.name) / "child.pid"
        code = "import os,pathlib,time; pathlib.Path(" + repr(str(pidfile)) + ").write_text(str(os.getpid())); time.sleep(60)"
        self.catalog["suites"]["wait"] = {"command": [sys.executable, "-c", code], "heavy": True}
        self.write_catalog(); git(self.root, "add", "."); git(self.root, "commit", "-qm", "suite")
        process = subprocess.Popen([str(CLI), "--repo", str(self.root), "check", "wait"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            deadline = time.monotonic() + 10
            while not pidfile.exists() and time.monotonic() < deadline: time.sleep(.02)
            self.assertTrue(pidfile.exists())
            pid = int(pidfile.read_text()); process.terminate(); self.assertEqual(130, process.wait(timeout=10))
            with self.assertRaises(ProcessLookupError): os.kill(pid, 0)
            result = json.loads(next(self.repo.runs.glob("coordinator/*/run.json")).read_text())
            self.assertEqual("cancelled", result["status"])
            with lock(self.repo.build_lock, blocking=False): pass
        finally:
            if process.poll() is None: process.kill(); process.wait()

    def test_emulator_lease_serializes_linked_worktrees_and_cleans_up(self):
        import shutil
        tools_root = CLI.parent
        for relative in ["with-emulator.sh", "lib/coordination.sh"]:
            target = self.root / "tools" / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(tools_root / relative, target)
            target.chmod(0o755)
        git(self.root, "add", "."); git(self.root, "commit", "-qm", "wrapper")
        alpha = self.task(); beta = self.task("beta", ["b"])
        sandbox = Path(self.temporary.name)
        sdk = sandbox / "sdk"; (sdk / "emulator").mkdir(parents=True)
        bin_dir = sandbox / "bin"; bin_dir.mkdir()
        emulator = sdk / "emulator/emulator"
        emulator.write_text("""#!/usr/bin/env python3
import os, pathlib, signal, time
p=pathlib.Path(os.environ['FAKE_DEVICE'])
def stop(*args): raise SystemExit
signal.signal(signal.SIGTERM, stop)
p.open('x').write(str(os.getpid()))
try:
 while True: time.sleep(.02)
finally: p.unlink(missing_ok=True)
""")
        adb = bin_dir / "adb"
        adb.write_text("""#!/usr/bin/env python3
import os, pathlib, signal, sys
p=pathlib.Path(os.environ['FAKE_DEVICE']); args=sys.argv[1:]
if args and args[0]=='-s': args=args[2:]
if args==['start-server']: pass
elif args==['devices']: print('List of devices attached')
elif args[:2]==['shell','getprop']: print('1' if p.exists() else '')
elif args==['emu','kill']:
 if p.exists():
  try: os.kill(int(p.read_text()), signal.SIGTERM)
  except ProcessLookupError: pass
else: sys.exit(1)
""")
        emulator.chmod(0o755); adb.chmod(0o755)
        env = os.environ.copy()
        env.update(ANDROID_HOME=str(sdk), PATH=str(bin_dir) + os.pathsep + env['PATH'],
                   FAKE_DEVICE=str(sandbox / 'device.pid'), WORKFLOW_EMULATOR_PORT='5988')
        env.pop('WORKFLOW_COORDINATION_ROOT', None)
        marker = sandbox / 'command-running'
        code = "import pathlib,time; p=pathlib.Path(" + repr(str(marker)) + "); p.open('x').close(); time.sleep(.3); p.unlink()"
        def run(task):
            root = Path(task['checkout'])
            return subprocess.run([str(root / 'tools/with-emulator.sh'), sys.executable, '-c', code], cwd=root,
                                  env=env, capture_output=True, timeout=20)
        with ThreadPoolExecutor(2) as pool: results = list(pool.map(run, [alpha, beta]))
        for result in results: self.assertEqual(0, result.returncode, result.stderr.decode())
        self.assertFalse((sandbox / 'device.pid').exists())
        with lock(self.repo.device_lock, blocking=False): pass
        failed = subprocess.run([str(Path(alpha['checkout']) / 'tools/with-emulator.sh'), sys.executable, '-c', 'raise SystemExit(7)'],
                                env=env, capture_output=True, timeout=15)
        self.assertEqual(7, failed.returncode)
        self.assertFalse((sandbox / 'device.pid').exists())


class InstrumentationTests(unittest.TestCase):
    def test_only_real_success_counts(self):
        for text in ["INSTRUMENTATION_CODE: -1", "OK (0 tests)", "FAILURES!!!\nOK (1 test)", "Process crashed"]:
            with self.assertRaises(RuntimeError): instrumentation_result(text)
        self.assertEqual({"tests": 2, "skipped": 0}, instrumentation_result("\nOK (2 tests)\n\nINSTRUMENTATION_CODE: -1"))

    def test_skips_require_explicit_opt_in(self):
        text = "INSTRUMENTATION_STATUS_CODE: -3\nOK (1 test)\n"
        with self.assertRaises(RuntimeError): instrumentation_result(text)
        self.assertEqual(1, instrumentation_result(text, True)["skipped"])


if __name__ == "__main__": unittest.main()
