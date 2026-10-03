#!/usr/bin/env python3
"""Drive an interactive guest shell through a real PTY (like the app's terminal).

    pty_check.py -- COMMAND...      COMMAND must start an interactive bash

Checks window size + SIGWINCH, background jobs, Ctrl-C, Ctrl-Z/fg, a pipeline,
`less` and exit.  Prints ok/FAIL lines; exit 0 if all pass."""
import fcntl
import os
import pty
import select
import signal
import struct
import sys
import termios
import time

cmd = sys.argv[sys.argv.index("--") + 1:]
results = []


def setsize(fd, rows, cols):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


pid, fd = pty.fork()
if pid == 0:
    os.execvp(cmd[0], cmd)
setsize(fd, 30, 100)
buf = b""


def read_until(token, timeout=20.0):
    global buf
    end = time.time() + timeout
    while token not in buf:
        left = end - time.time()
        if left <= 0:
            return None
        r, _, _ = select.select([fd], [], [], left)
        if r:
            try:
                data = os.read(fd, 65536)
            except OSError:
                return None
            if not data:
                return None
            buf += data
    i = buf.index(token) + len(token)
    out, buf = buf[:i], buf[i:]
    return out


def send(s):
    os.write(fd, s.encode() if isinstance(s, str) else s)


def check(name, cond, detail=""):
    results.append(bool(cond))
    print(("ok   " if cond else "FAIL ") + name + ("" if cond else " " + repr(detail)[:300]))


P = "WFP> "
send("PS1='WF''P> '; stty -echo; bind 'set enable-bracketed-paste off' 2>/dev/null\n")
check("prompt", read_until(P.encode()) is not None, buf)


def run(line, timeout=20.0):
    send(line + "\n")
    return read_until(P.encode(), timeout)


out = run("stty size")
check("window size 30x100", out and b"30 100" in out, out)
setsize(fd, 40, 120)            # the kernel signals SIGWINCH to the foreground group
time.sleep(0.3)
out = run("stty size; echo cols=$COLUMNS")
check("resize to 40x120 (+SIGWINCH updates $COLUMNS)", out and b"40 120" in out and b"cols=120" in out, out)

out = run("sleep 100 & jobs")
check("background job listed", out and b"Running" in out and b"sleep 100" in out, out)
out = run("kill %1; wait; jobs; echo done-bg")
check("background job killed", out and b"done-bg" in out, out)

send("sleep 100\n")
time.sleep(0.8)
t0 = time.time()
send(b"\x03")
out = read_until(P.encode(), 10)
check("Ctrl-C interrupts the foreground job", out is not None and time.time() - t0 < 5, out)
out = run("echo rc=$?")
check("exit status 130 after Ctrl-C", out and b"rc=130" in out, out)

send("sleep 100\n")
time.sleep(0.8)
send(b"\x1a")
out = read_until(P.encode(), 10)
check("Ctrl-Z stops the job", out and b"Stopped" in out, out)
send("fg\n")
time.sleep(0.8)
send(b"\x03")
out = read_until(P.encode(), 10)
out2 = run("echo rc=$?; jobs | wc -l")
check("fg resumes, Ctrl-C ends it", out is not None and out2 and b"rc=130" in out2, (out, out2))

want = sum(1 for i in range(1, 1001) if "5" in str(i))
out = run("seq 1 1000 | grep 5 | wc -l")
check("pipeline", out and (b"%d" % want) in out, out)

send("seq 1 200 | more\n")
out = read_until(b"--More--", 10)
send(" ")
time.sleep(0.5)
send("q")
out2 = read_until(P.encode(), 10)
check("more pages (space) and quits (q)", out is not None and out2 is not None, (out, out2))

send("top -b -n 1 | head -3; watch -n 0.2 -t -g date +%N >/dev/null; echo watch-rc=$?\n")
out = read_until(P.encode(), 20)
check("top/watch (procps) run", out and b"watch-rc=0" in out and b"load average" in out, out)

send("nano -R /etc/hostname\n")
time.sleep(1.0)
send(b"\x18")              # Ctrl-X
out = read_until(P.encode(), 10)
check("nano full-screen editor opens and exits", out is not None, out)

send("exit 0\n")
end = time.time() + 20
status = None
while time.time() < end:
    wpid, st = os.waitpid(pid, os.WNOHANG)
    if wpid:
        status = st
        break
    try:
        r, _, _ = select.select([fd], [], [], 0.2)
        if r:
            os.read(fd, 65536)
    except OSError:
        pass
check("shell exits 0", status is not None and os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0, status)
if status is None:
    os.kill(pid, signal.SIGKILL)
sys.exit(0 if all(results) else 1)
