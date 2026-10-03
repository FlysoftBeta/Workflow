#!/usr/bin/env python3
"""Real Linux child-process acceptance tests; never accesses a device or root su."""
import ctypes
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import tempfile
import time
import unittest
import uuid


BUILD = Path(sys.argv.pop(1)).resolve()
LIBC = ctypes.CDLL(None, use_errno=True)
if LIBC.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER, for guardian-death test
    raise OSError(ctypes.get_errno(), "prctl child subreaper failed")


def start_time(pid):
    return int(Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[19])


class Guard:
    def __init__(self, directory, mode="normal", kernel=None):
        self.run_id = str(uuid.uuid4())
        self.directory = Path(directory).resolve()
        config = self.directory / (self.run_id + ".config")
        config.write_text(mode)
        self.process = subprocess.Popen(
            [str(BUILD / "guard-test"), "supervise", str(kernel or BUILD / "fake-kernel"),
             str(self.directory), str(config), self.run_id],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            bufsize=0,
        )
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ, "control")
        self.selector.register(self.process.stderr, selectors.EVENT_READ, "log")
        self.control = b""
        self.log = b""
        self.events = []
        self.started = None

    def pump(self, timeout):
        for selected, _ in self.selector.select(timeout):
            data = os.read(selected.fileobj.fileno(), 65536)
            if not data:
                self.selector.unregister(selected.fileobj)
            elif selected.data == "log":
                self.log += data
            else:
                self.control += data
                while b"\n" in self.control:
                    line, self.control = self.control.split(b"\n", 1)
                    self.events.append(json.loads(line))

    def event(self, timeout=5):
        deadline = time.monotonic() + timeout
        while not self.events and time.monotonic() < deadline:
            self.pump(max(0, deadline - time.monotonic()))
        if not self.events:
            raise AssertionError(f"No control event; status={self.process.poll()} log={self.log!r}")
        return self.events.pop(0)

    def ready(self):
        self.started = self.event()
        assert self.started["event"] == "started", self.started
        deadline = time.monotonic() + 5
        while b"FAKE_READY\n" not in self.log and time.monotonic() < deadline:
            self.pump(max(0, deadline - time.monotonic()))
        assert b"FAKE_READY\n" in self.log, self.log
        assert b"fake stdout on log channel\n" in self.log, self.log
        return self.started

    def send(self, command):
        self.process.stdin.write(command)

    def stop(self):
        item = self.started
        self.send(f"stop {self.run_id} {item['pid']} {item['startTime']}\n".encode())
        return self.event()

    def close(self):
        if self.process.stdin and not self.process.stdin.closed:
            self.process.stdin.close()
        try:
            self.process.wait(timeout=4)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=2)
            raise
        self.selector.close()
        self.process.stdout.close()
        self.process.stderr.close()


class ProxyGuardTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="workflow proxy guard ")
        self.guards = []

    def tearDown(self):
        for guard in self.guards:
            guard.close()
        self.directory.cleanup()

    def guard(self, mode="normal", kernel=None):
        guard = Guard(self.directory.name, mode, kernel)
        self.guards.append(guard)
        return guard

    def assert_reaped(self, guard, event, code):
        self.assertEqual(event["event"], "exit")
        self.assertEqual(event["exitCode"], code)
        self.assertEqual(guard.process.wait(timeout=3), code)
        self.assertFalse(Path(f"/proc/{event['pid']}").exists())

    def test_actual_exec_identity_and_stop(self):
        guard = self.guard()
        started = guard.ready()
        self.assertEqual(started["uid"], os.geteuid())
        self.assertEqual(started["guardPid"], guard.process.pid)
        self.assertEqual(started["guardStartTime"], start_time(guard.process.pid))
        self.assertEqual(started["startTime"], start_time(started["pid"]))
        self.assertEqual(started["runId"], guard.run_id)
        event = guard.stop()
        self.assertFalse(event["forced"])
        self.assert_reaped(guard, event, 143)

    def test_stale_invalid_and_unbounded_control_rejected(self):
        guard = self.guard()
        started = guard.ready()
        commands = [
            f"stop {uuid.uuid4()} {started['pid']} {started['startTime']}\n".encode(),
            f"stop {guard.run_id} {started['pid'] + 1} {started['startTime']}\n".encode(),
            f"stop {guard.run_id} {started['pid']} {started['startTime'] + 1}\n".encode(),
            f"stop {guard.run_id} {started['pid']} {started['startTime']} extra\n".encode(),
            b"x" * 2048 + b"\n", b"stop\x00anything\n",
        ]
        for command in commands:
            guard.send(command)
            event = guard.event()
            self.assertEqual(event["event"], "error")
            self.assertFalse(event["fatal"])
            self.assertEqual(start_time(started["pid"]), started["startTime"])
        self.assert_reaped(guard, guard.stop(), 143)

    def test_fragmented_control(self):
        guard = self.guard()
        started = guard.ready()
        command = f"stop {guard.run_id} {started['pid']} {started['startTime']}\n".encode()
        for index in range(0, len(command), 3):
            guard.send(command[index:index + 3])
        self.assert_reaped(guard, guard.event(), 143)

    def test_real_comm_with_spaces_parentheses_newline(self):
        guard = self.guard("comm")
        guard.ready()
        self.assert_reaped(guard, guard.stop(), 143)

    def test_eof_stops_and_reaps_child(self):
        guard = self.guard()
        guard.ready()
        guard.process.stdin.close()
        event = guard.event()
        self.assertFalse(event["forced"])
        self.assert_reaped(guard, event, 143)

    def test_eof_escalates_after_two_second_grace(self):
        guard = self.guard("ignore")
        guard.ready()
        before = time.monotonic()
        guard.process.stdin.close()
        event = guard.event()
        elapsed = time.monotonic() - before
        self.assertTrue(event["forced"])
        self.assertGreaterEqual(elapsed, 1.8)
        self.assertLess(elapsed, 4.5)
        self.assert_reaped(guard, event, 137)

    def test_guardian_signals_stop_child(self):
        for signum in (signal.SIGTERM, signal.SIGHUP, signal.SIGINT):
            guard = self.guard()
            guard.ready()
            guard.process.send_signal(signum)
            self.assert_reaped(guard, guard.event(), 143)

    def test_child_dies_if_guardian_is_killed(self):
        guard = self.guard()
        started = guard.ready()
        guard.process.kill()
        self.assertEqual(guard.process.wait(timeout=2), -signal.SIGKILL)
        deadline = time.monotonic() + 3
        result = (0, 0)
        while time.monotonic() < deadline:
            result = os.waitpid(started["pid"], os.WNOHANG)
            if result[0]:
                break
            time.sleep(0.01)
        self.assertEqual(result[0], started["pid"])
        self.assertTrue(os.WIFSIGNALED(result[1]))
        self.assertEqual(os.WTERMSIG(result[1]), signal.SIGKILL)

    def test_executable_replacement_refuses_control_but_eof_cleans_owned_child(self):
        guard = self.guard("replace")
        started = guard.ready()
        os.kill(started["pid"], signal.SIGUSR1)
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            if os.readlink(f"/proc/{started['pid']}/exe") != str(BUILD / "fake-kernel"):
                break
            time.sleep(0.01)
        event = guard.stop()
        self.assertEqual(event["event"], "error")
        self.assertFalse(event["fatal"])
        guard.process.stdin.close()
        self.assert_reaped(guard, guard.event(), 143)

    def test_natural_exit_is_reported_and_reaped(self):
        guard = self.guard("exit")
        guard.ready()
        event = guard.event()
        self.assertFalse(event["forced"])
        self.assert_reaped(guard, event, 37)

    def test_exec_failure_never_announces_started(self):
        kernel = Path(self.directory.name) / "invalid kernel"
        kernel.write_bytes(b"not an ELF file\n")
        kernel.chmod(0o700)
        guard = self.guard(kernel=kernel)
        event = guard.event()
        self.assertEqual(event["event"], "error")
        self.assertTrue(event["fatal"])
        self.assertEqual(guard.process.wait(timeout=3), 126)
        guard.pump(0)
        self.assertFalse(any(item["event"] == "started" for item in guard.events))

    def test_closed_control_reader_triggers_cleanup(self):
        guard = self.guard()
        started = guard.ready()
        guard.selector.unregister(guard.process.stdout)
        guard.process.stdout.close()
        guard.send(b"invalid\n")
        self.assertEqual(guard.process.wait(timeout=3), 143)
        self.assertFalse(Path(f"/proc/{started['pid']}").exists())

    @unittest.skipIf(os.geteuid() == 0, "host already root")
    def test_production_build_requires_root(self):
        result = subprocess.run([str(BUILD / "guard-production")], capture_output=True, check=False)
        event = json.loads(result.stdout)
        self.assertEqual(event["event"], "error")
        self.assertTrue(event["fatal"])
        self.assertIn("Root", event["message"])
        self.assertEqual(result.returncode, 126)


if __name__ == "__main__":
    unittest.main(verbosity=2)
