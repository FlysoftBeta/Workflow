#!/bin/bash
# Path-semantics probe.  Runs unchanged inside the engine and inside a real
# kernel container (podman --rootfs) over the same Debian rootfs; the host test
# diffs the two outputs.  Output must not depend on uid, pid, time or inode.
# Each line: "<case> <result>"; errors print the errno text from coreutils.
set -u; export LC_ALL=C # Stable diagnostic punctuation regardless of the host locale.
T=/tmp/wf-paths.$1
rm -rf "$T" 2>/dev/null
mkdir -p "$T" && cd "$T" || exit 90

r() { # r <case> <cmd...> : run, print stdout (one line) or the error kind
  local name=$1; shift
  local out
  out=$("$@" 2>&1) ; local rc=$?
  out=${out//$'\n'/|}
  out=${out//"$T"/T}
  printf '%s rc=%d %s\n' "$name" "$rc" "$out"
}

mkdir -p a/b/c d "sp ace/多字节" long
echo data > a/b/c/file
echo root > /tmp/wf-paths-rootfile.$1
ln -s a/b rel_dir               # relative dir link
ln -s "$T/a/b/c/file" abs_file  # absolute file link
ln -s /etc abs_etc              # absolute link into the guest root
ln -s ../../../../../../../etc up_etc   # climbs above "/"
ln -s dangling_target dangling
ln -s loop2 loop1; ln -s loop1 loop2
ln -s rel_dir chain1; ln -s chain1 chain2; ln -s chain2 chain3
ln -s c a/b/cdir_link
echo spaced > "sp ace/多字节/f i le"

r cat_rel        cat rel_dir/c/file
r cat_abs        cat abs_file
r cat_chain      cat chain3/c/file
r etc_via_abs    cat abs_etc/debian_version
r etc_via_up     cat up_etc/debian_version
r dotdot_root    cat /../../../etc/debian_version
r dotdot_link    cat rel_dir/../b/c/file
r dotdot_after_l ls -d chain3/..
r readlink_rel   readlink rel_dir
r readlink_abs   readlink abs_file
r readlink_f     readlink -f chain3/c/file
r realpath_up    realpath up_etc
r realpath_dotdot realpath a/b/../../rel_dir/c
r dangling_cat   cat dangling
r dangling_rl    readlink dangling
r dangling_stat  stat -c %F dangling
r dangling_lstat stat -L -c %F dangling
r loop           cat loop1
r loop_lstat     stat -c %F loop1
r trail_file     cat a/b/c/file/
r trail_link     stat -c %F rel_dir/
r trail_ln_l     stat -c %F rel_dir
r notdir         cat a/b/c/file/x
r enoent_mid     cat a/nope/file
r unicode        cat "sp ace/多字节/f i le"
r ls_unicode     ls "sp ace/多字节"
r cd_link_pwd    bash -c 'cd chain3 && pwd && pwd -P'
r cd_up_link     bash -c 'cd chain3/.. && pwd -P'
r cd_root_up     bash -c 'cd / && cd .. && pwd'
r proc_cwd       bash -c 'cd a/b && readlink /proc/self/cwd'
r proc_cwd_ls    bash -c 'cd a/b && ls /proc/self/cwd/'
r proc_root      readlink /proc/self/root
r proc_root_etc  cat /proc/self/root/etc/debian_version
r proc_exe       readlink /proc/self/exe
r proc_fd        bash -c 'exec 3<a/b/c/file; readlink /proc/self/fd/3'
r proc_fd_cat    bash -c 'exec 3<a/b/c/file; cat /proc/self/fd/3'
r dev_fd         bash -c 'exec 4<a/b/c/file; cat /dev/fd/4'
r dev_stdin      bash -c 'echo piped | cat /dev/stdin'
r dev_null       bash -c 'echo x > /dev/null && echo written'
r pipe_fd        bash -c 'echo hi | readlink /proc/self/fd/0 | cut -c1-5'
r mkdir_p        mkdir -p x/y/z
r mkdir_exist    mkdir x
r touch_new      touch x/y/z/new
r mv_file        mv x/y/z/new x/moved
r mv_dir         mv x/y d/y2
r ls_after_mv    ls -R x d
r rm_file        rm x/moved
r rmdir_ne       rmdir d
r rm_r           rm -r d
r ln_s_create    ln -s /usr/bin newlink
r ls_newlink     ls newlink/true
r symlink_exist  ln -s /etc abs_file
r unlink_link    rm newlink
r find_tree      bash -c 'find . -name "*file*" | sort'
r glob           bash -c 'echo a/*/c/*'
long=$(printf 'd%.0s' {1..200})
r long_mkdir     mkdir -p "long/$long/$long/$long/$long"
r long_touch     touch "long/$long/$long/$long/$long/f"
r long_ls        bash -c "ls long/$long/$long/$long/$long | wc -l"
r cp_r           cp -r a acopy
r cp_link        cp -P abs_etc acopy/l
r readlink_cp    readlink acopy/l
r cmp            cmp a/b/c/file acopy/b/c/file
r stat_dir       stat -c %F a
r stat_link_dir  stat -c %F rel_dir
r test_ops       bash -c '[ -d rel_dir ] && [ -L rel_dir ] && [ -f abs_file ] && [ ! -e dangling ] && [ -h dangling ] && echo ok'
r script_exec    bash -c 'printf "#!/bin/sh\necho script \$0 \$1\n" > s.sh; chmod +x s.sh; ./s.sh arg'
r script_arg     bash -c 'printf "#!/bin/sh -e\necho e\n" > s2.sh; chmod +x s2.sh; ./s2.sh'
r script_env     bash -c 'printf "#!/usr/bin/env bash\necho env-ok\n" > s3.sh; chmod +x s3.sh; ./s3.sh'
r script_noexec  bash -c 'printf "#!/nonexistent/interp\n" > s4.sh; chmod +x s4.sh; ./s4.sh'
r exec_noperm    bash -c 'printf "echo hi\n" > s5.sh; chmod -x s5.sh; ./s5.sh'
r exec_dir       bash -c './a'
r exec_noshebang bash -c 'printf "echo plain\n" > s6.sh; chmod +x s6.sh; ./s6.sh'
r exec_via_link  bash -c 'ln -s /usr/bin/echo myecho; ./myecho linked'
r exec_proc_self bash -c 'exec /proc/self/exe -c "echo reexec"'
r exec_fexecve   python3 -c 'print(1)'
r env_path       bash -c 'command -v ls'
r cleanup        bash -c "cd / && rm -rf $T /tmp/wf-paths-rootfile.$1 && echo gone"
