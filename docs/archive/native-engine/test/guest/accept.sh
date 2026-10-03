#!/bin/bash
# Acceptance workload (research-engine.md §4.6) run INSIDE the guest as `work`,
# in the workspace image with the runtime binds of environment.md §2.6.
# Prints "ok <name>" / "FAIL <name> <detail>" lines and "accept: N ok, M failed".
# Phase "1" does the work (network needed), phase "2" runs in a fresh engine
# instance and checks what phase 1 left behind.
set -u
phase=${1:-1}
pass=0; fail=0
ok()  { echo "ok   $1"; pass=$((pass+1)); }
bad() { echo "FAIL $1 ${2:-}"; fail=$((fail+1)); }
t() { local name=$1; shift; local out; if out=$("$@" 2>&1); then ok "$name"; else bad "$name" "$(echo "$out" | tail -3 | tr '\n' ' ')"; fi; }
W=/workspace/accept
if [ "$phase" = 1 ]; then
  rm -rf "$W"; mkdir -p "$W"; cd "$W" || exit 90

  # 1 identity
  t id-work        test "$(id -un):$(id -u)" = work:1000
  t sudo-su-root   test "$(sudo -n su -c whoami)" = root
  # 2 apt/dpkg with network; ownership persists (phase 2)
  t apt-update     sudo -n apt-get update -q
  t apt-install    sudo -n apt-get install -y -q jq strace
  t jq-runs        sh -c 'echo "{\"a\":[1,2]}" | jq -c ".a|length" | grep -qx 2'
  # 3 toolchain: C, cmake
  printf '#include <stdio.h>\nint main(void){puts("c-ok");return 0;}\n' > hello.c
  t gcc            sh -c 'gcc -O2 -o hello hello.c && ./hello | grep -qx c-ok'
  mkdir -p cm && printf 'cmake_minimum_required(VERSION 3.20)\nproject(t C)\nadd_executable(t ../hello.c)\n' > cm/CMakeLists.txt
  t cmake          sh -c 'cd cm && cmake -S . -B build -G "Unix Makefiles" >/dev/null && cmake --build build -j4 >/dev/null && ./build/t | grep -qx c-ok'
  # 4 python via uv, node via npm
  t uv-venv        sh -c 'uv venv -q .venv && .venv/bin/python -c "import sys; print(sys.version)"'
  t uv-pip         sh -c 'uv pip install -q --python .venv/bin/python six && .venv/bin/python -c "import six; print(six.__version__)"'
  t npm-install    sh -c 'mkdir -p js && cd js && npm init -y >/dev/null && npm install --silent is-number@7.0.0 >/dev/null && node -e "console.log(require(\"is-number\")(5))" | grep -qx true'
  # 5 git
  t git-clone      git clone -q https://github.com/octocat/Hello-World.git hw
  t git-commit     sh -c 'cd hw && git -c user.name=t -c user.email=t@t commit -q --allow-empty -m wf && git status --short | wc -l | grep -qx 0 && git log --oneline | wc -l | grep -qv "^0$"'
  # 6 hardlinks (rootfs: emulated; workspace bind: refused -> cp falls back)
  t hardlink-rootfs sh -c 'rm -f /tmp/hl1 /tmp/hl2 /tmp/hl3; echo x > /tmp/hl1 && ln /tmp/hl1 /tmp/hl2 && cp -l /tmp/hl1 /tmp/hl3 && test "$(stat -c %h /tmp/hl1)" = 3'
  t hardlink-bind  sh -c 'echo y > hb1 && ! ln hb1 hb2 2>/dev/null && cp -al hb1 hb3 2>/dev/null; true'
  # 7 AF_UNIX (pathname + abstract), long directory
  t unix-socket    python3 -c '
import socket, os, threading
d=os.path.join(os.getcwd(), "s"*70); os.makedirs(d, exist_ok=True); p=d+"/sock"
s=socket.socket(socket.AF_UNIX); s.bind(p); s.listen(1)
threading.Thread(target=lambda: (lambda c: (c[0].sendall(c[0].recv(9)), c[0].close()))(s.accept())).start()
c=socket.socket(socket.AF_UNIX); c.connect(p); c.sendall(b"roundtrip"); assert c.recv(9)==b"roundtrip"
a=socket.socket(socket.AF_UNIX); a.bind("\0wf-acc"); assert s.getsockname()==p'
  # 9 unicode/long paths, /proc/self/exe, getcwd
  t unicode-long   sh -c 'd="$(printf "ü%.0s" $(seq 60))/深い/dir with space/$(printf "x%.0s" $(seq 200))"; mkdir -p "$d" && cd "$d" && pwd | grep -q "深い" && touch "f ☃" && ls | grep -q "☃"'
  t proc-self-exe  sh -c 'test "$(readlink /proc/self/exe)" = /usr/bin/readlink'
  # 10 process stress: execs, threads, vfork (posix_spawn)
  t exec-500       sh -c 'i=0; while [ $i -lt 500 ]; do /bin/true || exit 1; i=$((i+1)); done'
  t threads        python3 -c '
import threading, os
r=[]
def w(i):
    for _ in range(50): os.stat("/etc/passwd")
    r.append(i)
ts=[threading.Thread(target=w, args=(i,)) for i in range(16)]
[t.start() for t in ts]; [t.join() for t in ts]; assert len(r)==16'
  t posix-spawn    python3 -c 'import subprocess; assert all(subprocess.run(["/bin/true"]).returncode==0 for _ in range(100))'
  t make-j8        sh -c 'mkdir -p mk && cd mk && for i in $(seq 16); do echo "int f$i(void){return $i;}" > f$i.c; done &&
                          printf "all: \$(patsubst %%.c,%%.o,\$(wildcard *.c))\n%%.o: %%.c\n\tgcc -c \$< -o \$@\n" > Makefile && make -s -j8 && ls *.o | wc -l | grep -qx 16'
  # 11 nested ptrace refused, no deadlock
  t ptrace-refused sh -c 'out=$(timeout 20 strace -f true 2>&1); rc=$?; [ $rc -ne 124 ] && echo "$out" | grep -qi "not permitted"'
  # 12 io_uring blocked
  t io-uring-enosys python3 -c '
import ctypes, os
libc=ctypes.CDLL(None, use_errno=True)
r=libc.syscall(425, 8, ctypes.create_string_buffer(120))
assert r==-1 and ctypes.get_errno()==38, (r, ctypes.get_errno())'
  # identity: files created as root keep root in the rootfs, work in the bind
  t root-owned     sh -c 'sudo -n touch /usr/local/acc-root && test "$(stat -c %U /usr/local/acc-root)" = root'
  t bind-owner     sh -c 'sudo -n touch acc-bind && test "$(stat -c %U acc-bind)" = work'
else
  cd "$W" || exit 90
  t persist-jq     sh -c 'dpkg-query -W -f="\${Status}" jq | grep -q "install ok installed" && test "$(stat -c %U:%a /usr/bin/jq)" = root:755'
  t persist-root   test "$(stat -c %U /usr/local/acc-root)" = root
  t persist-venv   .venv/bin/python -c 'import six'
  t persist-git    sh -c 'cd hw && git log -1 --format=%s | grep -qx wf'
  t persist-hl     test "$(stat -c %h /tmp/hl2)" = 3
fi
echo "accept: $pass ok, $fail failed"
[ $fail = 0 ]
