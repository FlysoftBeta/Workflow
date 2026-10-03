#!/bin/bash
# binfmt table probe (M4): rules come from the engine command line (--binfmt)
# and from the guest's /etc/binfmt.d; the host's binfmt_misc is never used.
set -u
cd /tmp || exit 90
r() { local n=$1; shift; local o; o=$("$@" 2>&1); local rc=$?; o=${o//$'\n'/|}; printf '%s rc=%d %s\n' "$n" "$rc" "$o"; }
printf 'WFBIN payload\n' > /tmp/prog.wfb; chmod +x /tmp/prog.wfb
printf 'plain\n' > /tmp/prog.wfx; chmod +x /tmp/prog.wfx
printf 'WFPRS\n' > /tmp/prog.prs; chmod +x /tmp/prog.prs
r magic       /tmp/prog.wfb one two
r extension   /tmp/prog.wfx three
r preserve    bash -c 'exec -a custom-argv0 /tmp/prog.prs four'
r path-search bash -c 'cd /tmp && PATH=/tmp:$PATH prog.wfb five'
r foreign-elf /tmp/arm64-true
r no-rule     bash -c 'printf "ZZZZ\n" > /tmp/none; chmod +x /tmp/none; /tmp/none'
