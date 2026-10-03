#!/usr/bin/env python3
"""Targeted CLI invariants beyond the frozen host suite (no changes to its goldens)."""
import os, pathlib, subprocess, sys, time
runtime, loader, root = map(lambda x: str(pathlib.Path(x).resolve()), sys.argv[1:4])
env = {**os.environ, 'WORKFLOW_ENGINE_LOADER': loader}
def run(args, **kw):
    return subprocess.run([runtime, 'run', '--root', root, *args], env=env, capture_output=True, timeout=8, **kw)
p = run(['--', '/bin/echo', '工作区 café 🦀'])
assert p.returncode == 0 and p.stdout == '工作区 café 🦀\n'.encode(), (p.returncode, p.stdout, p.stderr)
p = run(['--', '/bin/sh', '-c', 'f=/tmp/工作区-café-🦀; printf data > "$f"; cat "$f"; rm "$f"'])
assert p.returncode == 0 and p.stdout == b'data', (p.returncode, p.stdout, p.stderr)
p = run(['--', '/bin/echo', b'raw-\xff-byte'])
assert p.returncode == 0 and p.stdout == b'raw-\xff-byte\n', (p.returncode, p.stdout, p.stderr)
p = run(['--no-default-binds', '--bind', '/dev/null:/dev/null', '--', '/bin/sh', '-c',
         'test ! -e /proc/self/stat && test -c /dev/null && echo pack-only'])
assert p.returncode == 0 and p.stdout == b'pack-only\n', (p.returncode, p.stdout, p.stderr)
p = run(['--', '/bin/sh', '-c', 'test -e /proc/self/stat && echo defaults'])
assert p.returncode == 0 and p.stdout == b'defaults\n', (p.returncode, p.stdout, p.stderr)
t = time.monotonic()
p = run(['--', '/bin/sh', '-c', 'sleep 60 & echo $!; exit 7'])
assert p.returncode == 7 and time.monotonic()-t < 5, (p.returncode, p.stdout, p.stderr)
child = int(p.stdout.strip())
assert not pathlib.Path('/proc', str(child)).exists(), 'main exit left a child alive'
t = time.monotonic()
p = run(['--wait-all', '--', '/bin/sh', '-c', 'sleep 0.3 & exit 7'])
assert p.returncode == 7 and time.monotonic()-t >= 0.25, (p.returncode, p.stdout, p.stderr)
print('CLI contract: UTF-8, pack-only binds, default binds, main-exit tree cleanup, wait-all passed')
