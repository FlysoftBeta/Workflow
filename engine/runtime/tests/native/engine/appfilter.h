/* appfilter.h — syscalls Android's x86_64 app seccomp filter traps (SIGSYS),
 * as measured by the device harness on the API 28 AVD (sysprobe, 2026-09-28),
 * plus the set*id family the app filter refuses (seen as sudo failures there).
 * Used only by tests: test/legacy_probe.c installs it as a RET_TRAP filter to
 * reproduce the app sandbox on the host, and the engine's
 * WORKFLOW_ENGINE_TEST_LEGACY=sigsys hook lets these reach seccomp untouched,
 * as a pre-4.8 kernel (seccomp before the ptrace entry stop) would. */
#ifndef WORKFLOW_ENGINE_APPFILTER_H
#define WORKFLOW_ENGINE_APPFILTER_H
#include <sys/syscall.h>
#if defined(__x86_64__)
#define ENG_APPFILTER_LEGACY \
    __NR_access, __NR_open, __NR_stat, __NR_lstat, __NR_creat, __NR_mkdir, __NR_rmdir, __NR_unlink, \
    __NR_rename, __NR_link, __NR_symlink, __NR_readlink, __NR_chmod, __NR_chown, __NR_lchown, \
    __NR_mknod, __NR_utimes, __NR_futimesat, __NR_dup2, __NR_pipe, __NR_poll, __NR_select, \
    __NR_getdents, __NR_getpgrp, __NR_epoll_create, __NR_epoll_wait, __NR_inotify_init, \
    __NR_eventfd, __NR_signalfd, __NR_alarm, __NR_time
#define ENG_APPFILTER_OTHER \
    __NR_uselib, __NR_set_robust_list, __NR_rseq, __NR_statx, __NR_faccessat2, __NR_clone3, __NR_openat2, \
    __NR_close_range, __NR_pidfd_open, __NR_pidfd_getfd, __NR_pidfd_send_signal, __NR_epoll_pwait2, \
    __NR_process_madvise, __NR_mount_setattr, __NR_landlock_create_ruleset, __NR_memfd_secret, \
    __NR_futex_waitv, __NR_cachestat, __NR_fchmodat2, __NR_map_shadow_stack, __NR_pkey_alloc, \
    __NR_pkey_mprotect, __NR_io_uring_setup, __NR_io_pgetevents, __NR_membarrier, __NR_userfaultfd, \
    __NR_kcmp, __NR_name_to_handle_at, __NR_open_by_handle_at, __NR_bpf, __NR_kexec_file_load, \
    __NR_add_key, __NR_keyctl, __NR_get_mempolicy, __NR_set_mempolicy, __NR_mbind, __NR_migrate_pages, \
    __NR_move_pages, __NR_fanotify_init, __NR_clock_adjtime, __NR_adjtimex, __NR_acct, __NR_swapon, \
    __NR_chroot, __NR_mount, __NR_umount2, __NR_open_tree, __NR_fsopen, \
    __NR_setuid, __NR_setgid, __NR_setreuid, __NR_setregid, __NR_setresuid, __NR_setresgid, \
    __NR_setfsuid, __NR_setfsgid, __NR_setgroups
#endif
#endif
