#!/usr/bin/env python3
"""Generate cases/m5.json: M3-M5 device cases on the real workspace image.

The image (artifacts/image/<arch>/image.tar.zst + image.json) is uploaded by
run.sh when WORKSPACE_IMAGE=1 into files/image.tar.zst / files/image.json.
Guest scripts and their oracle outputs are embedded from native/engine/test/guest
(the goldens are the podman/real-kernel outputs used by the host suites).

    python3 native/engine/device/cases/gen_m5.py > native/engine/device/cases/m5.json
"""
import json
import os
import re

here = os.path.dirname(os.path.abspath(__file__))
guest = os.path.join(here, "..", "..", "test", "guest")


def read(name):
    with open(os.path.join(guest, name), encoding="utf-8") as f:
        return f.read()


def exact(text):
    """Java regex matching exactly `text` (the whole stdout)."""
    return r"\A" + re.escape(text) + r"\z"


GEN = "${FILES}/gen/1"
ROOT = GEN + "/rootfs"
ENV = {
    "PATH": "/opt/toolchains/active/python/bin:/opt/toolchains/active/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    "HOME": "/home/work", "USER": "work", "LOGNAME": "work", "SHELL": "/bin/bash", "LANG": "en_US.UTF-8",
    "NVM_DIR": "/opt/toolchains/nvm", "UV_PYTHON_INSTALL_DIR": "/opt/toolchains/uv/python",
    "UV_PYTHON_PREFERENCE": "system", "TERM": "dumb",
}
BINDS = ["--bind", "${FILES}/ws/home/work:/home/work", "--bind", "${FILES}/ws/toolchains:/opt/toolchains",
         "--bind", "${FILES}/ws/workspace:/workspace", "--socket-dir", "${CACHE}/s"]


def guest_case(cid, cmd, user=None, extra=None, **kw):
    argv = ["run", "--root", ROOT] + BINDS + (["--user", user] if user else []) + (extra or []) + \
        ["--cwd", "/home/work", "--"] + cmd
    c = {"id": cid, "argv": argv, "clearEnv": True, "env": dict(ENV)}
    c.update(kw)
    return c


def sh(cid, script, **kw):
    return dict({"id": cid, "exe": "/system/bin/sh", "argv": ["-c", script]}, **kw)


cases = [
    sh("m5-clean", "rm -rf ${FILES}/gen ${FILES}/ws ${CACHE}/s && mkdir -p ${FILES}/gen ${FILES}/ws/workspace && echo clean",
       exit=0, stdout="^clean$"),
    {"id": "m5-install", "argv": ["install", "--image", "${FILES}/image.tar.zst", "--index", "${FILES}/image.json",
                                  "--target", GEN + ".partial"],
     "timeout": 600, "exit": 0, "stdout": r"\"rows\":28444", "note": "C installer on the device filesystem"},
    sh("m5-commit", "mv " + GEN + ".partial " + GEN + " && mkdir -p ${FILES}/ws/home && mv " + GEN +
       "/seeds/home/work ${FILES}/ws/home/work && mv " + GEN + "/seeds/toolchains ${FILES}/ws/toolchains && echo committed",
       exit=0, stdout="^committed$", note="what EngineStore does: rename .partial, move seeds into the stores"),
    {"id": "m5-verify", "argv": ["verify", "--generation", GEN], "exit": 0,
     "stdout": r"\"withoutMetadata\":0,\"hardlinkProblems\":0"},
    guest_case("m5-id", ["/usr/bin/id"], exit=0, stdout=r"^uid=1000\(work\) gid=1000\(work\)"),
    guest_case("m5-sudo-su", ["/usr/bin/sudo", "-n", "su", "-c", "id -u; whoami; grep ^Uid: /proc/self/status"],
               exit=0, stdout=r"\A0\nroot\nUid:\t(1\d{4})\t\1\t\1\t\1\n\z",
               note="guest root while the host uid stays the app uid"),
    guest_case("m5-put-scripts", ["/bin/bash", "-c", 'printf "%s" "$1" > /wf-meta.sh && printf "%s" "$2" > /wf-ident.sh && '
               'chmod 644 /wf-meta.sh /wf-ident.sh && echo put', "_", read("meta.sh"), read("ident.sh")],
               user="root", exit=0, stdout="^put$", note="oracle scripts as files (their names appear in bash messages)"),
    guest_case("m5-meta-oracle", ["/bin/bash", "/wf-meta.sh"], user="root", timeout=180,
               exit=0, stdout=exact(read("meta.golden")),
               note="44 metadata cases, byte-identical to the real-kernel oracle (meta.golden)"),
    guest_case("m5-ident-oracle", ["/bin/bash", "/wf-ident.sh"], timeout=300,
               exit=0, stdout=exact(read("ident.golden")),
               note="21 identity/exec cases incl. sudo/su/chage/set-id/AT_SECURE/execveat, byte-identical to podman"),
    guest_case("m5-sudoers-restore", ["/bin/sh", "-c", "chmod 0440 /etc/sudoers && sudo -n true && echo restored"],
               user="root", exit=0, stdout="^restored$", note="the ident oracle ends with a world-writable sudoers"),
    guest_case("m5-persist-setup", ["/bin/sh", "-c",
               "rm -rf /srv/p && mkdir -p /srv/p && echo a > /srv/p/f && chown 1:2 /srv/p/f && chmod 4750 /srv/p/f && "
               "ln /srv/p/f /srv/p/g && mkfifo -m 600 /srv/p/q && mknod /srv/p/z c 1 5 && echo set"],
               user="root", exit=0, stdout="^set$"),
    guest_case("m5-persist-check", ["/bin/sh", "-c", 'stat -c "%a %u %g %h %F" /srv/p/f /srv/p/g /srv/p/q; '
               'stat -c "%a %t,%T %F" /srv/p/z'], user="root", exit=0,
               stdout=exact("4750 1 2 2 regular file\n4750 1 2 2 regular file\n600 0 0 1 fifo\n644 1,5 character special file\n"),
               note="new engine instance reads what the previous one wrote"),
    guest_case("m5-dpkg", ["/bin/sh", "-c",
               "set -e; d=/tmp/pkg; rm -rf $d; mkdir -p $d/DEBIAN $d/usr/local/wf/sub; "
               "printf 'Package: wftest\\nVersion: 1.0\\nArchitecture: all\\nMaintainer: t <t@t>\\nDescription: t\\n' > $d/DEBIAN/control; "
               "echo payload > $d/usr/local/wf/tool; chmod 4755 $d/usr/local/wf/tool; "
               "echo data > $d/usr/local/wf/sub/owned; chown 1:1 $d/usr/local/wf/sub/owned; chmod 640 $d/usr/local/wf/sub/owned; "
               "chown 0:50 $d/usr/local/wf/sub; chmod 2775 $d/usr/local/wf/sub; ln $d/usr/local/wf/tool $d/usr/local/wf/tool-hl; "
               "dpkg-deb --build $d /tmp/wftest.deb >/dev/null; dpkg -i /tmp/wftest.deb >/dev/null; echo installed"],
               user="root", timeout=180, exit=0, stdout="^installed$"),
    guest_case("m5-dpkg-check", ["/bin/sh", "-c", 'stat -c "%n %a %U %G %h" /usr/local/wf/tool /usr/local/wf/tool-hl '
               '/usr/local/wf/sub/owned; stat -c "%n %a %U %G" /usr/local/wf/sub; dpkg --verify wftest && echo verified'],
               user="root", exit=0,
               stdout=exact("/usr/local/wf/tool 4755 root root 2\n/usr/local/wf/tool-hl 4755 root root 2\n"
                            "/usr/local/wf/sub/owned 640 daemon daemon 1\n/usr/local/wf/sub 2775 root staff\nverified\n")),
    guest_case("m5-crash-setup", ["/bin/sh", "-c", "rm -rf /srv/c; mkdir /srv/c; echo precious > /srv/c/f; ln /srv/c/f /srv/c/k && echo linked"],
               user="root", exit=0, stdout="^linked$"),
    guest_case("m5-crash-link", ["/bin/ln", "/srv/c/k", "/srv/c/g"],
               user="root", env=dict(ENV, WORKFLOW_ENGINE_CRASH_AT="link-counted"), exit=99,
               note="engine dies after counting a new name, before creating it (crash injection)"),
    {"id": "m5-fsck-finds", "argv": ["fsck", "--root", ROOT], "exit": 1, "stdout": r"count: object .* nlink 3, names 2"},
    {"id": "m5-fsck-repair", "argv": ["fsck", "--root", ROOT, "--repair"], "exit": 0},
    guest_case("m5-crash-after", ["/bin/sh", "-c", "cat /srv/c/f; stat -c %h /srv/c/f; ls /srv/c | wc -l"], user="root",
               exit=0, stdout=exact("precious\n2\n2\n")),
    guest_case("m5-gcc", ["/bin/sh", "-c", "cd /tmp && printf '#include <stdio.h>\\nint main(void){puts(\"c-ok\");return 0;}\\n' > h.c && gcc -O2 -o h h.c && ./h"],
               timeout=180, exit=0, stdout="^c-ok$"),
    guest_case("m5-python", ["/bin/sh", "-c", "python3 -c 'import sys, ssl, sqlite3; print(sys.version_info[:2] >= (3, 13))'"],
               exit=0, stdout="^True$", note="uv-installed CPython from the toolchain store bind"),
    guest_case("m5-node", ["/bin/sh", "-c", "node -e 'console.log(process.versions.node.split(\".\")[0] >= 22)'"],
               exit=0, stdout="^true$"),
    guest_case("m5-git", ["/bin/sh", "-c", "cd /workspace && rm -rf r && git init -q r && cd r && echo x > x && git add x && "
               "git -c user.name=t -c user.email=t@t commit -qm m && git log --oneline | wc -l"], exit=0, stdout="^1$"),
    guest_case("m5-unix-socket", ["/bin/sh", "-c", "cd /workspace && python3 -c '\n"
               "import socket, os, threading\n"
               "d=os.path.join(os.getcwd(), \"s\"*60); os.makedirs(d, exist_ok=True); p=d+\"/sock\"\n"
               "os.path.exists(p) and os.unlink(p)\n"
               "s=socket.socket(socket.AF_UNIX); s.bind(p); s.listen(1)\n"
               "threading.Thread(target=lambda: (lambda c: (c[0].sendall(c[0].recv(4)), c[0].close()))(s.accept())).start()\n"
               "c=socket.socket(socket.AF_UNIX); c.connect(p); c.sendall(b\"ping\"); print(c.recv(4).decode(), s.getsockname()==p)'"],
               exit=0, stdout="^ping True$"),
    guest_case("m5-pipeline-stats", ["/bin/bash", "-c", "for i in $(seq 50); do cat /etc/passwd; done | wc -l"],
               env=dict(ENV, WORKFLOW_ENGINE_STATS="1"), exit=0, stdout=r"^\d+$", stderr=r"stats syscall_stops=\d+ .*fast=\d",
               note="fast=1 where the kernel is >= 4.8 (seccomp RET_TRACE fast path), fast=0 on 4.4"),
    guest_case("m5-bionic-in-guest", ["${LIB}/libwftest-forkstress.so"], extra=["--bind", "${LIB}:${LIB}"],
               exit=0, note="Codex model: a bionic binary from nativeLibraryDir runs inside the guest "
                            "(PT_INTERP /system/bin/linker64 via the Android system binds)"),
    guest_case("m5-apt-network", ["/usr/bin/sudo", "-n", "sh", "-c", "apt-get update -q >/dev/null && apt-get install -y -q jq >/dev/null && jq --version"],
               extra=["--bind", "${FILES}/resolv.conf:/etc/resolv.conf"], timeout=600, exit=0, stdout="^jq-",
               note="needs emulator network; resolv.conf written by the setup case"),
    {"id": "m5-verify-after", "argv": ["verify", "--generation", GEN], "exit": 0,
     "stdout": r"\"withoutMetadata\":0,\"hardlinkProblems\":0"},
]
# resolv.conf for the apt case: the device's DNS as the app would write it -- world-readable, since
# apt resolves as _apt and bind files present their host bits (apps run with umask 077) (emulator: 10.0.2.3,
# which depends on the host's resolver configuration; public resolvers as fallback)
cases.insert(3, sh("m5-resolv", "printf 'nameserver 10.0.2.3\\nnameserver 8.8.8.8\\nnameserver 1.1.1.1\\noptions timeout:2 attempts:1\\n' > ${FILES}/resolv.conf && chmod 644 ${FILES}/resolv.conf && echo ok",
                   exit=0, stdout="^ok$"))

doc = {
    "defaults": {"exe": "${LIB}/libworkflow-engine.so", "timeout": 120, "cwd": "${FILES}"},
    "setup": [],
    "cases": cases,
}
print(json.dumps(doc, indent=1, ensure_ascii=False))
