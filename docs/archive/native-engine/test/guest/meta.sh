#!/bin/bash
# Metadata probe (M3).  Runs as root; uid-1000 checks use setpriv.  Executed by
# the engine on an installed rootfs (virtual metadata) and by podman on the real
# OCI image (real ownership, kernel DAC); outputs must match.  Numeric ids only.
set -u
T=/tmp/wf-meta
rm -rf "$T"; mkdir -p "$T"; chmod 1777 "$T"; cd "$T" || exit 90
umask 022
r() { local n=$1; shift; local o; o=$("$@" 2>&1); local rc=$?; o=${o//$'\n'/|}; printf '%s rc=%d %s\n' "$n" "$rc" "$o"; }
U() { setpriv --reuid=1000 --regid=1000 --clear-groups "$@"; }

r img_modes     stat -c '%a %u %g %F' /tmp /root /etc/shadow /var/mail /etc/passwd
r img_setid     bash -c '[ -u /usr/bin/su ] && [ -g /usr/bin/chage ] && [ -k /tmp ] && echo bits'
r img_hardlink  stat -c '%a %u %g %F %h' /usr/bin/perl /usr/bin/perl5.40.1
r img_same_ino  bash -c '[ "$(stat -c %i /usr/bin/perl)" = "$(stat -c %i /usr/bin/perl5.40.1)" ] && echo same'
r img_devnull   stat -c '%a %u %g %F' /dev/null
r ls_ln         bash -c 'ls -ln /etc/shadow | cut -d" " -f1,3,4'
r root_ids      bash -c 'id -u; id -g'

r create        bash -c 'touch f && mkdir d && stat -c "%a %u %g %F" f d'
r umask077      bash -c 'umask 077; touch u; mkdir ud; stat -c %a u ud'
r chown_file    bash -c 'chown 1000:8 f && stat -c "%u %g" f'
r chmod_file    bash -c 'chmod 640 f && stat -c %a f'
r chown_names   bash -c 'chown root:mail d && stat -c "%U %G" d'
r chmod_dir     bash -c 'chmod 2775 d && stat -c %a d'
r sgid_inherit  bash -c 'mkdir d/sub && touch d/g && stat -c "%g %a" d/sub d/g'
r symlink_chmod bash -c 'ln -s f sl && chmod 600 sl && stat -c %a f && stat -c %F sl'
r mknod_c       bash -c 'mknod nul c 1 3 && stat -c "%a %u %g %F %t,%T" nul'
r mknod_write   bash -c 'echo gone > nul && wc -c < nul'
r mkfifo        bash -c 'mkfifo fi && stat -c "%F %a" fi'

r ln_count      bash -c 'echo one > h1 && ln h1 h2 && stat -c %h h1 h2'
r ln_same_ino   bash -c '[ "$(stat -c %i h1)" = "$(stat -c %i h2)" ] && echo same'
r ln_shared     bash -c 'echo two >> h2 && cat h1'
r ln_meta       bash -c 'chmod 604 h2 && stat -c %a h1'
r ln_third      bash -c 'ln h2 h3 && stat -c %h h1 h2 h3'
r ln_rm         bash -c 'rm h1 && stat -c %h h2 h3 && cat h3'
r ln_mv         bash -c 'mv h3 d/h4 && stat -c %h h2 d/h4 && cat d/h4'
r ln_replace    bash -c 'echo new > n && mv n h2 && stat -c %h h2 d/h4 && cat d/h4'
r ln_lstat      stat -c %F d/h4
r ln_readlink   readlink d/h4
r ln_dir        ln d d2
r ln_find       bash -c 'find . -type l | sort'
r ls_types      bash -c 'ls -l d | cut -c1'

r u_ids         U bash -c 'id -u; id -g; id -G'
r u_create      U bash -c 'touch uf && stat -c "%a %u %g" uf'
r u_etc         U touch /etc/wf-x
r u_shadow      U cat /etc/shadow
r u_root        U ls /root
r u_chown       U chown 0 uf
r u_chgrp       U chgrp 0 uf
r u_chmod_other U chmod 777 f
r u_mknod       U mknod n2 c 1 3
r u_rm_sticky   U rm f
r u_access      U bash -c 'test -r /etc/shadow && echo r || echo no-r; test -w /etc/passwd && echo w || echo no-w; test -x /usr/bin/su && echo x'
r u_readonly    U bash -c 'echo x > uro; chmod 444 uro; echo y > uro'
r cleanup       bash -c 'cd / && rm -rf /tmp/wf-meta && echo gone'
