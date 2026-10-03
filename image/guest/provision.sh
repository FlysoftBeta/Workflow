#!/bin/bash
# Provision a Debian trixie guest into the Workflow workspace (debian-trixie typeVersion 1,
# docs/environment.md §2.6). Runs inside the guest as (virtual) root with network access — under
# podman on a native-architecture host, or inside the Workflow engine on the device. Idempotent:
# a second run on a provisioned guest downloads nothing and changes no declared state.
set -euo pipefail
umask 022
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=../versions.env
. "$here/../versions.env"
export DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8 PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

step() { printf '==> %s\n' "$*" >&2; }
fail() { printf 'provision: %s\n' "$*" >&2; exit 2; }
[ "$(id -u)" = 0 ] || fail "must run as root"
grep -qx 'VERSION_CODENAME=trixie' /etc/os-release || fail "not a Debian trixie guest"
case $(dpkg --print-architecture) in
    amd64) uv_triple=x86_64-unknown-linux-gnu uv_sha=$UV_SHA256_AMD64 ;;
    arm64) uv_triple=aarch64-unknown-linux-gnu uv_sha=$UV_SHA256_ARM64 ;;
    *) fail "unsupported architecture $(dpkg --print-architecture)" ;;
esac

# fetch URL DEST SHA256 [MODE] — download only when DEST is missing or differs from the pin.
fetch() {
    if [ -f "$2" ] && echo "$3  $2" | sha256sum -c --status -; then return 0; fi
    curl -fsSL --retry 3 -o "$2.download" "$1"
    echo "$3  $2.download" | sha256sum -c --quiet - || fail "checksum mismatch for $1"
    chmod "${4:-0644}" "$2.download"
    mv -f "$2.download" "$2"
}

step "packages"
# shellcheck disable=SC2086
if dpkg-query -W -f '${db:Status-Status}\n' $PACKAGES 2>/dev/null | grep -qvx installed \
    || [ "$(dpkg-query -W -f '${db:Status-Status}\n' $PACKAGES 2>/dev/null | wc -l)" -ne "$(echo $PACKAGES | wc -w)" ]; then
    apt-get update
    # shellcheck disable=SC2086
    apt-get install -y --no-install-recommends $PACKAGES
fi

step "locale and time zone"
sed -i -E 's/^# *(en_US\.UTF-8 UTF-8)$/\1/' /etc/locale.gen
grep -qx 'en_US.UTF-8 UTF-8' /etc/locale.gen || fail "en_US.UTF-8 missing from /etc/locale.gen"
if ! locale -a 2>/dev/null | grep -qx 'en_US.utf8'; then locale-gen; fi
update-locale LANG=en_US.UTF-8
ln -sfn /usr/share/zoneinfo/Etc/UTC /etc/localtime
echo Etc/UTC > /etc/timezone

step "user work"
getent group work >/dev/null || groupadd --gid 1000 work
id -u work >/dev/null 2>&1 || useradd --uid 1000 --gid 1000 --create-home --shell /bin/bash work
[ "$(id -u work):$(id -g work)" = 1000:1000 ] || fail "user work is not 1000:1000"
passwd -d work >/dev/null
cat > /etc/sudoers.d/workflow.new <<'EOF'
Defaults secure_path="/opt/toolchains/active/python/bin:/opt/toolchains/active/node/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
work ALL=(ALL:ALL) NOPASSWD: ALL
EOF
chmod 0440 /etc/sudoers.d/workflow.new
visudo -cqf /etc/sudoers.d/workflow.new
mv -f /etc/sudoers.d/workflow.new /etc/sudoers.d/workflow

step "directories"
# /opt/toolchains and /home/work become persistent stores on the device (metadata.stores); the image
# content below them is only their seed.
install -d -m 0755 -o work -g work /workspace /opt/toolchains /opt/toolchains/uv /opt/toolchains/uv/python \
    /opt/toolchains/nvm /opt/toolchains/profiles
install -d -m 0755 /usr/local/libexec/workflow /usr/local/share/workflow /usr/local/share/doc/uv /usr/local/share/doc/nvm \
    /usr/local/lib/nvm

step "uv $UV_VERSION"
if [ "$(/usr/local/bin/uv --version 2>/dev/null | cut -d' ' -f2)" != "$UV_VERSION" ]; then
    scratch=$(mktemp -d)
    fetch "https://github.com/astral-sh/uv/releases/download/$UV_VERSION/uv-$uv_triple.tar.gz" "$scratch/uv.tar.gz" "$uv_sha"
    tar -xzf "$scratch/uv.tar.gz" -C "$scratch"
    install -m 0755 "$scratch/uv-$uv_triple/uv" "$scratch/uv-$uv_triple/uvx" /usr/local/bin/
    rm -rf "$scratch"
fi
fetch "https://raw.githubusercontent.com/astral-sh/uv/$UV_VERSION/LICENSE-MIT" /usr/local/share/doc/uv/LICENSE-MIT "$UV_LICENSE_MIT_SHA256"
fetch "https://raw.githubusercontent.com/astral-sh/uv/$UV_VERSION/LICENSE-APACHE" /usr/local/share/doc/uv/LICENSE-APACHE "$UV_LICENSE_APACHE_SHA256"

step "nvm $NVM_VERSION"
nvm_base="https://raw.githubusercontent.com/nvm-sh/nvm/v$NVM_VERSION"
fetch "$nvm_base/nvm.sh" /usr/local/lib/nvm/nvm.sh "$NVM_SH_SHA256"
fetch "$nvm_base/nvm-exec" /usr/local/lib/nvm/nvm-exec "$NVM_EXEC_SHA256" 0755
fetch "$nvm_base/bash_completion" /usr/local/lib/nvm/bash_completion "$NVM_BASH_COMPLETION_SHA256"
fetch "$nvm_base/LICENSE.md" /usr/local/share/doc/nvm/LICENSE.md "$NVM_LICENSE_SHA256"

step "bundled agent entry point"
install -d -m 0755 /opt/workflow/bundled
ln -sfn /opt/workflow/bundled/libcodex.so /usr/local/bin/codex

step "shell environment"
cat > /etc/profile.d/workflow.sh <<'EOF'
# Workflow environment (debian-trixie typeVersion 1). Managed by the image; see .workspace/env.json.
# /opt/toolchains/active is the toolchain profile of this environment instance.
export NVM_DIR=/opt/toolchains/nvm UV_PYTHON_INSTALL_DIR=/opt/toolchains/uv/python UV_PYTHON_PREFERENCE=system
case ":$PATH:" in
    *:/opt/toolchains/active/python/bin:*) ;;
    *) PATH=/opt/toolchains/active/python/bin:/opt/toolchains/active/node/bin:$PATH ;;
esac
export PATH
if [ -n "${BASH_VERSION:-}" ]; then
    # Loading nvm.sh is slow under ptrace; load it on first use only.
    nvm() { unset -f nvm; . /usr/local/lib/nvm/nvm.sh --no-use; nvm "$@"; }
fi
EOF
marker_begin='# >>> workflow >>>'
marker_end='# <<< workflow <<<'
sed -i "/^$marker_begin\$/,/^$marker_end\$/d" /etc/bash.bashrc
printf '%s\n[ -r /etc/profile.d/workflow.sh ] && . /etc/profile.d/workflow.sh\n%s\n' \
    "$marker_begin" "$marker_end" >> /etc/bash.bashrc
install -m 0755 "$here/envctl" /usr/local/libexec/workflow/envctl

step "Python $DEFAULT_PYTHON and Node.js $DEFAULT_NODE"
as_work() { runuser -u work -- env -i HOME=/home/work USER=work LOGNAME=work LANG=C.UTF-8 "$@"; }
envctl=/usr/local/libexec/workflow/envctl
as_work "$envctl" python "$DEFAULT_PYTHON" >/dev/null
as_work "$envctl" node "$DEFAULT_NODE" >/dev/null
profile=$(as_work "$envctl" verify "$DEFAULT_PYTHON" "$DEFAULT_NODE" | jq -r .profile)
as_work "$envctl" activate "$profile"

step "cleanup"
apt-get clean
rm -rf /var/lib/apt/lists/* /var/cache/debconf/*-old /var/lib/dpkg/*-old /root/.cache /home/work/.cache /opt/toolchains/.tmp \
    /root/.wget-hsts /home/work/.wget-hsts
find /var/log -type f \( -name '*.gz' -o -name '*.xz' -o -name '*.[0-9]' \) -delete
find /var/log -type f -name '*.log' -exec truncate -s 0 {} +
dpkg-query -W -f '${Package}\n' | sort > /usr/local/share/workflow/base-packages
step "done"
