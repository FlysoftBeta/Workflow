//! Virtual credentials, capabilities and Linux set-id transition rules.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
#[repr(C)]
pub struct eng_tracer {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct __sFILE {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    unsafe fn eng_arg(r: *const eng_regs, i: ::core::ffi::c_int) -> uint64_t;
    unsafe fn eng_set_arg(r: *mut eng_regs, i: ::core::ffi::c_int, v: uint64_t);
    unsafe fn eng_set_ret(r: *mut eng_regs, v: uint64_t);
    unsafe fn eng_task_find(tr: *mut eng_tracer, tid: pid_t) -> *mut eng_task;
    unsafe fn eng_task_void(
        t: *mut eng_task,
        r: *mut eng_regs,
        result: ::core::ffi::c_long,
    ) -> ::core::ffi::c_int;
    unsafe fn fclose(__fp: *mut FILE) -> ::core::ffi::c_int;
    unsafe fn fread(
        __buf: *mut ::core::ffi::c_void,
        __size: size_t,
        __count: size_t,
        __fp: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    unsafe fn fopen(
        __path: *const ::core::ffi::c_char,
        __mode: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    unsafe fn snprintf(
        __buf: *mut ::core::ffi::c_char,
        __size: size_t,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn malloc(__byte_count: size_t) -> *mut ::core::ffi::c_void;
    unsafe fn realloc(
        __ptr: *mut ::core::ffi::c_void,
        __byte_count: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn free(__ptr: *mut ::core::ffi::c_void);
    unsafe fn strtoul(
        __s: *const ::core::ffi::c_char,
        __end_ptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulong;
    unsafe fn memset(
        __dst: *mut ::core::ffi::c_void,
        __ch: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn strchr(
        __s: *const ::core::ffi::c_char,
        __ch: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strcmp(
        __lhs: *const ::core::ffi::c_char,
        __rhs: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn strtok_r(
        __s: *mut ::core::ffi::c_char,
        __delimiter: *const ::core::ffi::c_char,
        __pos_ptr: *mut *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn eng_guest_to_host(
        g: *const eng_guest,
        guest: *const ::core::ffi::c_char,
        host: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_mem_read(
        tid: pid_t,
        addr: uintptr_t,
        buf: *mut ::core::ffi::c_void,
        n: size_t,
    ) -> ssize_t;
    unsafe fn eng_mem_write(
        tid: pid_t,
        addr: uintptr_t,
        buf: *const ::core::ffi::c_void,
        n: size_t,
    ) -> ssize_t;
}
pub type size_t = usize;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type uint64_t = u64;
pub type uintptr_t = usize;
pub type __kernel_pid_t = ::core::ffi::c_int;
pub type __pid_t = __kernel_pid_t;
pub type pid_t = __pid_t;
pub type ssize_t = isize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct user_regs_struct {
    pub r15: ::core::ffi::c_ulong,
    pub r14: ::core::ffi::c_ulong,
    pub r13: ::core::ffi::c_ulong,
    pub r12: ::core::ffi::c_ulong,
    pub rbp: ::core::ffi::c_ulong,
    pub rbx: ::core::ffi::c_ulong,
    pub r11: ::core::ffi::c_ulong,
    pub r10: ::core::ffi::c_ulong,
    pub r9: ::core::ffi::c_ulong,
    pub r8: ::core::ffi::c_ulong,
    pub rax: ::core::ffi::c_ulong,
    pub rcx: ::core::ffi::c_ulong,
    pub rdx: ::core::ffi::c_ulong,
    pub rsi: ::core::ffi::c_ulong,
    pub rdi: ::core::ffi::c_ulong,
    pub orig_rax: ::core::ffi::c_ulong,
    pub rip: ::core::ffi::c_ulong,
    pub cs: ::core::ffi::c_ulong,
    pub eflags: ::core::ffi::c_ulong,
    pub rsp: ::core::ffi::c_ulong,
    pub ss: ::core::ffi::c_ulong,
    pub fs_base: ::core::ffi::c_ulong,
    pub gs_base: ::core::ffi::c_ulong,
    pub ds: ::core::ffi::c_ulong,
    pub es: ::core::ffi::c_ulong,
    pub fs: ::core::ffi::c_ulong,
    pub gs: ::core::ffi::c_ulong,
}
pub type eng_regs = user_regs_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_guest {
    pub root: [::core::ffi::c_char; 4096],
    pub rootlen: size_t,
    pub rootfd: ::core::ffi::c_int,
    pub binds: [eng_bind; 32],
    pub nbinds: ::core::ffi::c_int,
    pub hides: [[::core::ffi::c_char; 4096]; 16],
    pub nhides: ::core::ffi::c_int,
    pub binfmt: [eng_binfmt; 32],
    pub nbinfmt: ::core::ffi::c_int,
    pub loader: [::core::ffi::c_char; 4096],
    pub sockdir: [::core::ffi::c_char; 4096],
    pub no_filemap: ::core::ffi::c_int,
    pub test_pagesz: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_binfmt {
    pub name: [::core::ffi::c_char; 64],
    pub r#type: ::core::ffi::c_char,
    pub offset: ::core::ffi::c_uint,
    pub magic: [::core::ffi::c_uchar; 128],
    pub mask: [::core::ffi::c_uchar; 128],
    pub len: ::core::ffi::c_uint,
    pub ext: [::core::ffi::c_char; 64],
    pub interp: [::core::ffi::c_char; 4096],
    pub preserve_argv0: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_bind {
    pub guest: [::core::ffi::c_char; 4096],
    pub host: [::core::ffi::c_char; 4096],
    pub glen: size_t,
    pub hlen: size_t,
    pub uid: ::core::ffi::c_uint,
    pub gid: ::core::ffi::c_uint,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_phase(pub ::core::ffi::c_uint);
impl eng_phase {
    pub const ENG_PH_BOOT: Self = Self(0);
    pub const ENG_PH_LOADER: Self = Self(1);
    pub const ENG_PH_GUEST: Self = Self(2);
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_mm {
    pub refs: ::core::ffi::c_int,
    pub base: uint64_t,
    pub size: uint32_t,
    pub nslots: uint32_t,
    pub used: [uint8_t; 512],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_proc {
    pub refs: ::core::ffi::c_int,
    pub exe: [::core::ffi::c_char; 4096],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_cred {
    pub ruid: uint32_t,
    pub euid: uint32_t,
    pub suid: uint32_t,
    pub fsuid: uint32_t,
    pub rgid: uint32_t,
    pub egid: uint32_t,
    pub sgid: uint32_t,
    pub fsgid: uint32_t,
    pub ngroups: uint32_t,
    pub groups: [uint32_t; 32],
    pub cap_eff: uint64_t,
    pub cap_prm: uint64_t,
    pub cap_inh: uint64_t,
    pub cap_amb: uint64_t,
    pub cap_bnd: uint64_t,
    pub securebits: uint32_t,
    pub nnp: ::core::ffi::c_int,
    pub undumpable: ::core::ffi::c_int,
    pub umask: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_task {
    pub tid: pid_t,
    pub tgid: pid_t,
    pub in_use: ::core::ffi::c_int,
    pub linked: ::core::ffi::c_int,
    pub held: ::core::ffi::c_int,
    pub phase: eng_phase,
    pub at_syscall_entry: ::core::ffi::c_int,
    pub await_exit: ::core::ffi::c_int,
    pub sysno: ::core::ffi::c_long,
    pub void_pending: ::core::ffi::c_int,
    pub inject_result: ::core::ffi::c_long,
    pub regs_modified: ::core::ffi::c_int,
    pub entry_regs: eng_regs,
    pub mm: *mut eng_mm,
    pub slot: ::core::ffi::c_int,
    pub slot_off: uint32_t,
    pub stack_scratch: uint64_t,
    pub proc: *mut eng_proc,
    pub plan: *mut ::core::ffi::c_void,
    pub plan_len: uint32_t,
    pub plan_exe: [::core::ffi::c_char; 4096],
    pub fixup: ::core::ffi::c_int,
    pub fix_addr: uint64_t,
    pub fix_len: uint64_t,
    pub fix_aux: ::core::ffi::c_long,
    pub fix_nofollow: ::core::ffi::c_int,
    pub fix_mode: uint32_t,
    pub fix_id: [::core::ffi::c_char; 64],
    pub fix_path: [::core::ffi::c_char; 4096],
    pub plan_setid: ::core::ffi::c_int,
    pub plan_euid: uint32_t,
    pub plan_egid: uint32_t,
    pub lg_fix: ::core::ffi::c_int,
    pub lg_a: uint64_t,
    pub lg_b: uint64_t,
    pub lg_restart: ::core::ffi::c_int,
    pub lg_nr: ::core::ffi::c_long,
    pub lg_saved: eng_regs,
    pub cr: eng_cred,
    pub umask_old: ::core::ffi::c_int,
    pub tr: *mut eng_tracer,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_log_level(pub ::core::ffi::c_uint);
impl eng_log_level {
    pub const ENG_LOG_ERROR: Self = Self(0);
    pub const ENG_LOG_WARN: Self = Self(1);
    pub const ENG_LOG_INFO: Self = Self(2);
    pub const ENG_LOG_DEBUG: Self = Self(3);
    pub const ENG_LOG_TRACE: Self = Self(4);
}
pub type FILE = __sFILE;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ENG_CAP_SETGID: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const ENG_CAP_SETUID: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const ENG_CAP_SETPCAP: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const ENG_CAP_LAST: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const ENG_CAP_FULL: ::core::ffi::c_ulonglong = ((1 as ::core::ffi::c_ulonglong)
    << ENG_CAP_LAST + 1 as ::core::ffi::c_int)
    .wrapping_sub(1 as ::core::ffi::c_ulonglong);
#[inline]
unsafe extern "C" fn eng_capable(
    mut t: *const eng_task,
    mut cap: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*t).cr.cap_eff >> cap & 1 as uint64_t) as ::core::ffi::c_int;
}
pub const EPERM: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ESRCH: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const EFAULT: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const EINVAL: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const PR_GET_DUMPABLE: ::core::ffi::c_long = 3 as ::core::ffi::c_long;
pub const PR_SET_DUMPABLE: ::core::ffi::c_long = 4 as ::core::ffi::c_long;
pub const PR_GET_KEEPCAPS: ::core::ffi::c_long = 7 as ::core::ffi::c_long;
pub const PR_SET_KEEPCAPS: ::core::ffi::c_long = 8 as ::core::ffi::c_long;
pub const PR_CAPBSET_READ: ::core::ffi::c_long = 23 as ::core::ffi::c_long;
pub const PR_CAPBSET_DROP: ::core::ffi::c_long = 24 as ::core::ffi::c_long;
pub const PR_GET_SECUREBITS: ::core::ffi::c_long = 27 as ::core::ffi::c_long;
pub const PR_SET_SECUREBITS: ::core::ffi::c_long = 28 as ::core::ffi::c_long;
pub const PR_SET_NO_NEW_PRIVS: ::core::ffi::c_int = 38 as ::core::ffi::c_int;
pub const PR_GET_NO_NEW_PRIVS: ::core::ffi::c_long = 39 as ::core::ffi::c_long;
pub const PR_CAP_AMBIENT: ::core::ffi::c_long = 47 as ::core::ffi::c_long;
pub const PR_CAP_AMBIENT_IS_SET: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PR_CAP_AMBIENT_RAISE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PR_CAP_AMBIENT_LOWER: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const PR_CAP_AMBIENT_CLEAR_ALL: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const S_ISUID: ::core::ffi::c_int = 0o4000 as ::core::ffi::c_int;
pub const S_ISGID: ::core::ffi::c_int = 0o2000 as ::core::ffi::c_int;
pub const S_IXGRP: ::core::ffi::c_int = 0o10 as ::core::ffi::c_int;
pub const __NR_umask: ::core::ffi::c_int = 95 as ::core::ffi::c_int;
pub const __NR_getuid: ::core::ffi::c_long = 102 as ::core::ffi::c_long;
pub const __NR_getgid: ::core::ffi::c_long = 104 as ::core::ffi::c_long;
pub const __NR_setuid: ::core::ffi::c_long = 105 as ::core::ffi::c_long;
pub const __NR_setgid: ::core::ffi::c_long = 106 as ::core::ffi::c_long;
pub const __NR_geteuid: ::core::ffi::c_long = 107 as ::core::ffi::c_long;
pub const __NR_getegid: ::core::ffi::c_long = 108 as ::core::ffi::c_long;
pub const __NR_setreuid: ::core::ffi::c_long = 113 as ::core::ffi::c_long;
pub const __NR_setregid: ::core::ffi::c_long = 114 as ::core::ffi::c_long;
pub const __NR_getgroups: ::core::ffi::c_long = 115 as ::core::ffi::c_long;
pub const __NR_setgroups: ::core::ffi::c_long = 116 as ::core::ffi::c_long;
pub const __NR_setresuid: ::core::ffi::c_long = 117 as ::core::ffi::c_long;
pub const __NR_getresuid: ::core::ffi::c_long = 118 as ::core::ffi::c_long;
pub const __NR_setresgid: ::core::ffi::c_long = 119 as ::core::ffi::c_long;
pub const __NR_getresgid: ::core::ffi::c_long = 120 as ::core::ffi::c_long;
pub const __NR_setfsuid: ::core::ffi::c_long = 122 as ::core::ffi::c_long;
pub const __NR_setfsgid: ::core::ffi::c_long = 123 as ::core::ffi::c_long;
pub const __NR_capget: ::core::ffi::c_long = 125 as ::core::ffi::c_long;
pub const __NR_capset: ::core::ffi::c_long = 126 as ::core::ffi::c_long;
pub const __NR_prctl: ::core::ffi::c_long = 157 as ::core::ffi::c_long;
pub const NOCHG: uint32_t = -1 as ::core::ffi::c_int as uint32_t;
pub const SECBIT_NOROOT: ::core::ffi::c_uint =
    (1 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const SECBIT_NO_SETUID_FIXUP: ::core::ffi::c_uint =
    (1 as ::core::ffi::c_uint) << 2 as ::core::ffi::c_int;
pub const SECBIT_KEEP_CAPS: ::core::ffi::c_uint =
    (1 as ::core::ffi::c_uint) << 4 as ::core::ffi::c_int;
pub const SECBIT_NO_CAP_AMBIENT_RAISE: ::core::ffi::c_uint =
    (1 as ::core::ffi::c_uint) << 6 as ::core::ffi::c_int;
pub const SECURE_ALL_BITS: ::core::ffi::c_uint = 0x55 as ::core::ffi::c_uint;
pub const SECURE_ALL_LOCKS: ::core::ffi::c_uint = SECURE_ALL_BITS << 1 as ::core::ffi::c_int;
pub const CAP_FS_MASK: ::core::ffi::c_ulonglong = (1 as ::core::ffi::c_ulonglong)
    << 0 as ::core::ffi::c_int
    | (1 as ::core::ffi::c_ulonglong) << 1 as ::core::ffi::c_int
    | (1 as ::core::ffi::c_ulonglong) << 2 as ::core::ffi::c_int
    | (1 as ::core::ffi::c_ulonglong) << 3 as ::core::ffi::c_int
    | (1 as ::core::ffi::c_ulonglong) << 4 as ::core::ffi::c_int
    | (1 as ::core::ffi::c_ulonglong) << 9 as ::core::ffi::c_int
    | (1 as ::core::ffi::c_ulonglong) << 27 as ::core::ffi::c_int
    | (1 as ::core::ffi::c_ulonglong) << 32 as ::core::ffi::c_int;
unsafe extern "C" fn is_one_of(
    mut v: uint32_t,
    mut a: uint32_t,
    mut b: uint32_t,
    mut c: uint32_t,
) -> ::core::ffi::c_int {
    return (v == a || v == b || v == c) as ::core::ffi::c_int;
}
unsafe extern "C" fn put_u32s(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut first_arg: ::core::ffi::c_int,
    mut v: *const uint32_t,
    mut n: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < n {
        let mut a: uint64_t = eng_arg(r, first_arg + i);
        if a == 0
            || eng_mem_write(
                (*t).tid,
                a as uintptr_t,
                v.offset(i as isize) as *const ::core::ffi::c_void,
                4 as size_t,
            ) != 4 as ssize_t
        {
            return -EFAULT;
        }
        i += 1;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn fix_setxuid(mut c: *mut eng_cred, mut old: *const eng_cred) {
    if (*c).securebits & SECBIT_NO_SETUID_FIXUP as uint32_t != 0 {
        return;
    }
    if ((*old).ruid == 0 as uint32_t
        || (*old).euid == 0 as uint32_t
        || (*old).suid == 0 as uint32_t)
        && ((*c).ruid != 0 as uint32_t && (*c).euid != 0 as uint32_t && (*c).suid != 0 as uint32_t)
    {
        if (*c).securebits & SECBIT_KEEP_CAPS as uint32_t == 0 {
            (*c).cap_prm = 0 as uint64_t;
            (*c).cap_eff = 0 as uint64_t;
        }
        (*c).cap_amb = 0 as uint64_t;
    }
    if (*old).euid == 0 as uint32_t && (*c).euid != 0 as uint32_t {
        (*c).cap_eff = 0 as uint64_t;
    }
    if (*old).euid != 0 as uint32_t && (*c).euid == 0 as uint32_t {
        (*c).cap_eff = (*c).cap_prm;
    }
}
unsafe extern "C" fn fix_setfsuid(mut c: *mut eng_cred, mut old_fsuid: uint32_t) {
    if (*c).securebits & SECBIT_NO_SETUID_FIXUP as uint32_t != 0 {
        return;
    }
    if old_fsuid == 0 as uint32_t && (*c).fsuid != 0 as uint32_t {
        (*c).cap_eff = ((*c).cap_eff as ::core::ffi::c_ulonglong & !CAP_FS_MASK) as uint64_t;
    }
    if old_fsuid != 0 as uint32_t && (*c).fsuid == 0 as uint32_t {
        (*c).cap_eff = ((*c).cap_eff as ::core::ffi::c_ulonglong
            | (*c).cap_prm as ::core::ffi::c_ulonglong & CAP_FS_MASK)
            as uint64_t;
    }
}
unsafe extern "C" fn set_resuid(
    mut t: *mut eng_task,
    mut ru: uint32_t,
    mut eu: uint32_t,
    mut su: uint32_t,
) -> ::core::ffi::c_int {
    let mut old: eng_cred = (*t).cr;
    if eng_capable(t, ENG_CAP_SETUID) == 0 {
        if ru != NOCHG && is_one_of(ru, old.ruid, old.euid, old.suid) == 0 {
            return -EPERM;
        }
        if eu != NOCHG && is_one_of(eu, old.ruid, old.euid, old.suid) == 0 {
            return -EPERM;
        }
        if su != NOCHG && is_one_of(su, old.ruid, old.euid, old.suid) == 0 {
            return -EPERM;
        }
    }
    if ru != NOCHG {
        (*t).cr.ruid = ru;
    }
    if eu != NOCHG {
        (*t).cr.euid = eu;
    }
    if su != NOCHG {
        (*t).cr.suid = su;
    }
    (*t).cr.fsuid = (*t).cr.euid;
    fix_setxuid(&raw mut (*t).cr, &raw mut old);
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn set_resgid(
    mut t: *mut eng_task,
    mut rg: uint32_t,
    mut eg: uint32_t,
    mut sg: uint32_t,
) -> ::core::ffi::c_int {
    if eng_capable(t, ENG_CAP_SETGID) == 0 {
        if rg != NOCHG && is_one_of(rg, (*t).cr.rgid, (*t).cr.egid, (*t).cr.sgid) == 0 {
            return -EPERM;
        }
        if eg != NOCHG && is_one_of(eg, (*t).cr.rgid, (*t).cr.egid, (*t).cr.sgid) == 0 {
            return -EPERM;
        }
        if sg != NOCHG && is_one_of(sg, (*t).cr.rgid, (*t).cr.egid, (*t).cr.sgid) == 0 {
            return -EPERM;
        }
    }
    if rg != NOCHG {
        (*t).cr.rgid = rg;
    }
    if eg != NOCHG {
        (*t).cr.egid = eg;
    }
    if sg != NOCHG {
        (*t).cr.sgid = sg;
    }
    (*t).cr.fsgid = (*t).cr.egid;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_setuid(mut t: *mut eng_task, mut u: uint32_t) -> ::core::ffi::c_int {
    if u == NOCHG {
        return -EINVAL;
    }
    let mut old: eng_cred = (*t).cr;
    if eng_capable(t, ENG_CAP_SETUID) != 0 {
        (*t).cr.suid = u;
        (*t).cr.ruid = (*t).cr.suid;
    } else if u != (*t).cr.ruid && u != (*t).cr.suid {
        return -EPERM;
    }
    (*t).cr.fsuid = u;
    (*t).cr.euid = (*t).cr.fsuid;
    fix_setxuid(&raw mut (*t).cr, &raw mut old);
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_setgid(mut t: *mut eng_task, mut g: uint32_t) -> ::core::ffi::c_int {
    if g == NOCHG {
        return -EINVAL;
    }
    if eng_capable(t, ENG_CAP_SETGID) != 0 {
        (*t).cr.sgid = g;
        (*t).cr.rgid = (*t).cr.sgid;
    } else if g != (*t).cr.rgid && g != (*t).cr.sgid {
        return -EPERM;
    }
    (*t).cr.fsgid = g;
    (*t).cr.egid = (*t).cr.fsgid;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_setreuid(
    mut t: *mut eng_task,
    mut ru: uint32_t,
    mut eu: uint32_t,
) -> ::core::ffi::c_int {
    let mut old: eng_cred = (*t).cr;
    if eng_capable(t, ENG_CAP_SETUID) == 0 {
        if ru != NOCHG && ru != old.ruid && ru != old.euid {
            return -EPERM;
        }
        if eu != NOCHG && is_one_of(eu, old.ruid, old.euid, old.suid) == 0 {
            return -EPERM;
        }
    }
    if ru != NOCHG {
        (*t).cr.ruid = ru;
    }
    if eu != NOCHG {
        (*t).cr.euid = eu;
    }
    if ru != NOCHG || eu != NOCHG && eu != old.ruid {
        (*t).cr.suid = (*t).cr.euid;
    }
    (*t).cr.fsuid = (*t).cr.euid;
    fix_setxuid(&raw mut (*t).cr, &raw mut old);
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_setregid(
    mut t: *mut eng_task,
    mut rg: uint32_t,
    mut eg: uint32_t,
) -> ::core::ffi::c_int {
    let mut old_r: uint32_t = (*t).cr.rgid;
    if eng_capable(t, ENG_CAP_SETGID) == 0 {
        if rg != NOCHG && rg != (*t).cr.rgid && rg != (*t).cr.egid {
            return -EPERM;
        }
        if eg != NOCHG && is_one_of(eg, (*t).cr.rgid, (*t).cr.egid, (*t).cr.sgid) == 0 {
            return -EPERM;
        }
    }
    if rg != NOCHG {
        (*t).cr.rgid = rg;
    }
    if eg != NOCHG {
        (*t).cr.egid = eg;
    }
    if rg != NOCHG || eg != NOCHG && eg != old_r {
        (*t).cr.sgid = (*t).cr.egid;
    }
    (*t).cr.fsgid = (*t).cr.egid;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_setfsuid(mut t: *mut eng_task, mut f: uint32_t) -> ::core::ffi::c_long {
    let mut old: uint32_t = (*t).cr.fsuid;
    if f != NOCHG
        && (eng_capable(t, ENG_CAP_SETUID) != 0
            || f == (*t).cr.ruid
            || f == (*t).cr.euid
            || f == (*t).cr.suid
            || f == (*t).cr.fsuid)
    {
        (*t).cr.fsuid = f;
        fix_setfsuid(&raw mut (*t).cr, old);
    }
    return old as ::core::ffi::c_long;
}
unsafe extern "C" fn h_setfsgid(mut t: *mut eng_task, mut f: uint32_t) -> ::core::ffi::c_long {
    let mut old: ::core::ffi::c_long = (*t).cr.fsgid as ::core::ffi::c_long;
    if f != NOCHG
        && (eng_capable(t, ENG_CAP_SETGID) != 0
            || f == (*t).cr.rgid
            || f == (*t).cr.egid
            || f == (*t).cr.sgid
            || f == (*t).cr.fsgid)
    {
        (*t).cr.fsgid = f;
    }
    return old;
}
unsafe extern "C" fn h_getgroups(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
) -> ::core::ffi::c_long {
    let mut size: ::core::ffi::c_long = eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t
        as ::core::ffi::c_int as ::core::ffi::c_long;
    if size < 0 as ::core::ffi::c_long {
        return -(EINVAL as ::core::ffi::c_long);
    }
    if size == 0 as ::core::ffi::c_long {
        return (*t).cr.ngroups as ::core::ffi::c_long;
    }
    if (size as uint32_t) < (*t).cr.ngroups {
        return -(EINVAL as ::core::ffi::c_long);
    }
    let mut n: size_t =
        ((*t).cr.ngroups as size_t).wrapping_mul(::core::mem::size_of::<uint32_t>());
    if n != 0
        && eng_mem_write(
            (*t).tid,
            eng_arg(r, 1 as ::core::ffi::c_int) as uintptr_t,
            &raw mut (*t).cr.groups as *mut uint32_t as *const ::core::ffi::c_void,
            n,
        ) != n as ssize_t
    {
        return -(EFAULT as ::core::ffi::c_long);
    }
    return (*t).cr.ngroups as ::core::ffi::c_long;
}
unsafe extern "C" fn h_setgroups(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
) -> ::core::ffi::c_long {
    let mut size: ::core::ffi::c_long = eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t
        as ::core::ffi::c_int as ::core::ffi::c_long;
    if eng_capable(t, ENG_CAP_SETGID) == 0 {
        return -(EPERM as ::core::ffi::c_long);
    }
    if size < 0 as ::core::ffi::c_long || size > 65536 as ::core::ffi::c_long {
        return -(EINVAL as ::core::ffi::c_long);
    }
    let mut n: uint32_t = size as uint32_t;
    let cap: uint32_t = ::core::mem::size_of::<[uint32_t; 32]>()
        .wrapping_div(::core::mem::size_of::<uint32_t>()) as uint32_t;
    if n > cap {
        eng_logf!(
            eng_log_level::ENG_LOG_WARN,
            b"setgroups: %u groups, keeping the first %u\0".as_ptr() as *const ::core::ffi::c_char,
            n,
            cap,
        );
        n = cap;
    }
    if n != 0
        && eng_mem_read(
            (*t).tid,
            eng_arg(r, 1 as ::core::ffi::c_int) as uintptr_t,
            &raw mut (*t).cr.groups as *mut uint32_t as *mut ::core::ffi::c_void,
            n.wrapping_mul(4 as uint32_t) as size_t,
        ) != n.wrapping_mul(4 as uint32_t) as ssize_t
    {
        return -(EFAULT as ::core::ffi::c_long);
    }
    (*t).cr.ngroups = n;
    return 0 as ::core::ffi::c_long;
}
pub const CAP_V1: ::core::ffi::c_uint = 0x19980330 as ::core::ffi::c_uint;
pub const CAP_V2: ::core::ffi::c_uint = 0x20071026 as ::core::ffi::c_uint;
pub const CAP_V3: ::core::ffi::c_uint = 0x20080522 as ::core::ffi::c_uint;
unsafe extern "C" fn h_capget(mut t: *mut eng_task, mut r: *mut eng_regs) -> ::core::ffi::c_long {
    let mut hdr: [uint32_t; 2] = [0; 2];
    if eng_mem_read(
        (*t).tid,
        eng_arg(r, 0 as ::core::ffi::c_int) as uintptr_t,
        &raw mut hdr as *mut uint32_t as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[uint32_t; 2]>(),
    ) != ::core::mem::size_of::<[uint32_t; 2]>() as ssize_t
    {
        return -(EFAULT as ::core::ffi::c_long);
    }
    let mut words: ::core::ffi::c_int = 0;
    if hdr[0usize] == CAP_V1 as uint32_t {
        words = 1 as ::core::ffi::c_int;
    } else if hdr[0usize] == CAP_V2 as uint32_t || hdr[0usize] == CAP_V3 as uint32_t {
        words = 2 as ::core::ffi::c_int;
    } else {
        let mut v: uint32_t = CAP_V3 as uint32_t;
        eng_mem_write(
            (*t).tid,
            eng_arg(r, 0 as ::core::ffi::c_int) as uintptr_t,
            &raw mut v as *const ::core::ffi::c_void,
            4 as size_t,
        );
        return (if eng_arg(r, 1 as ::core::ffi::c_int) != 0 {
            -EINVAL
        } else {
            0 as ::core::ffi::c_int
        }) as ::core::ffi::c_long;
    }
    if hdr[1usize] != 0 as uint32_t
        && hdr[1usize] != (*t).tgid as uint32_t
        && hdr[1usize] != (*t).tid as uint32_t
    {
        let mut o: *mut eng_task = eng_task_find((*t).tr, hdr[1usize] as pid_t);
        if o.is_null() {
            return -(ESRCH as ::core::ffi::c_long);
        }
        t = if o.is_null() { t } else { o };
    }
    if eng_arg(r, 1 as ::core::ffi::c_int) == 0 {
        return 0 as ::core::ffi::c_long;
    }
    let mut data: [uint32_t; 6] = [0; 6];
    let mut w: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while w < 2 as ::core::ffi::c_int {
        data[(w * 3 as ::core::ffi::c_int + 0 as ::core::ffi::c_int) as usize] =
            ((*t).cr.cap_eff >> 32 as ::core::ffi::c_int * w) as uint32_t;
        data[(w * 3 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as usize] =
            ((*t).cr.cap_prm >> 32 as ::core::ffi::c_int * w) as uint32_t;
        data[(w * 3 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as usize] =
            ((*t).cr.cap_inh >> 32 as ::core::ffi::c_int * w) as uint32_t;
        w += 1;
    }
    let mut n: size_t = (words as size_t).wrapping_mul(12 as size_t);
    return (if eng_mem_write(
        (*t).tid,
        eng_arg(r, 1 as ::core::ffi::c_int) as uintptr_t,
        &raw mut data as *mut uint32_t as *const ::core::ffi::c_void,
        n,
    ) == n as ssize_t
    {
        0 as ::core::ffi::c_int
    } else {
        -EFAULT
    }) as ::core::ffi::c_long;
}
unsafe extern "C" fn h_capset(mut t: *mut eng_task, mut r: *mut eng_regs) -> ::core::ffi::c_long {
    let mut hdr: [uint32_t; 2] = [0; 2];
    if eng_mem_read(
        (*t).tid,
        eng_arg(r, 0 as ::core::ffi::c_int) as uintptr_t,
        &raw mut hdr as *mut uint32_t as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[uint32_t; 2]>(),
    ) != ::core::mem::size_of::<[uint32_t; 2]>() as ssize_t
    {
        return -(EFAULT as ::core::ffi::c_long);
    }
    let mut words: ::core::ffi::c_int = 0;
    if hdr[0usize] == CAP_V1 as uint32_t {
        words = 1 as ::core::ffi::c_int;
    } else if hdr[0usize] == CAP_V2 as uint32_t || hdr[0usize] == CAP_V3 as uint32_t {
        words = 2 as ::core::ffi::c_int;
    } else {
        let mut v: uint32_t = CAP_V3 as uint32_t;
        eng_mem_write(
            (*t).tid,
            eng_arg(r, 0 as ::core::ffi::c_int) as uintptr_t,
            &raw mut v as *const ::core::ffi::c_void,
            4 as size_t,
        );
        return -(EINVAL as ::core::ffi::c_long);
    }
    if hdr[1usize] != 0 as uint32_t
        && hdr[1usize] != (*t).tgid as uint32_t
        && hdr[1usize] != (*t).tid as uint32_t
    {
        return -(EPERM as ::core::ffi::c_long);
    }
    let mut data: [uint32_t; 6] = [0 as uint32_t, 0, 0, 0, 0, 0];
    let mut n: size_t = (words as size_t).wrapping_mul(12 as size_t);
    if eng_mem_read(
        (*t).tid,
        eng_arg(r, 1 as ::core::ffi::c_int) as uintptr_t,
        &raw mut data as *mut uint32_t as *mut ::core::ffi::c_void,
        n,
    ) != n as ssize_t
    {
        return -(EFAULT as ::core::ffi::c_long);
    }
    let mut eff: uint64_t = 0 as uint64_t;
    let mut prm: uint64_t = 0 as uint64_t;
    let mut inh: uint64_t = 0 as uint64_t;
    let mut w: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while w < words {
        eff |= (data[(w * 3 as ::core::ffi::c_int + 0 as ::core::ffi::c_int) as usize] as uint64_t)
            << 32 as ::core::ffi::c_int * w;
        prm |= (data[(w * 3 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as usize] as uint64_t)
            << 32 as ::core::ffi::c_int * w;
        inh |= (data[(w * 3 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as usize] as uint64_t)
            << 32 as ::core::ffi::c_int * w;
        w += 1;
    }
    eff = (eff as ::core::ffi::c_ulonglong & ENG_CAP_FULL) as uint64_t;
    prm = (prm as ::core::ffi::c_ulonglong & ENG_CAP_FULL) as uint64_t;
    inh = (inh as ::core::ffi::c_ulonglong & ENG_CAP_FULL) as uint64_t;
    if words == 1 as ::core::ffi::c_int {
        eff = (eff as ::core::ffi::c_ulonglong
            | (*t).cr.cap_eff as ::core::ffi::c_ulonglong
                & !(0xffffffff as ::core::ffi::c_ulonglong)) as uint64_t;
        prm = (prm as ::core::ffi::c_ulonglong
            | (*t).cr.cap_prm as ::core::ffi::c_ulonglong
                & !(0xffffffff as ::core::ffi::c_ulonglong)) as uint64_t;
        inh = (inh as ::core::ffi::c_ulonglong
            | (*t).cr.cap_inh as ::core::ffi::c_ulonglong
                & !(0xffffffff as ::core::ffi::c_ulonglong)) as uint64_t;
    }
    let mut allowed_inh: uint64_t = ((*t).cr.cap_inh as ::core::ffi::c_ulonglong
        | if eng_capable(t, ENG_CAP_SETPCAP) != 0 {
            ENG_CAP_FULL
        } else {
            (*t).cr.cap_prm as ::core::ffi::c_ulonglong
        }) as uint64_t;
    if inh & !allowed_inh != 0 {
        return -(EPERM as ::core::ffi::c_long);
    }
    if inh & !((*t).cr.cap_inh | (*t).cr.cap_bnd) != 0 {
        return -(EPERM as ::core::ffi::c_long);
    }
    if prm & !(*t).cr.cap_prm != 0 {
        return -(EPERM as ::core::ffi::c_long);
    }
    if eff & !prm != 0 {
        return -(EPERM as ::core::ffi::c_long);
    }
    (*t).cr.cap_eff = eff;
    (*t).cr.cap_prm = prm;
    (*t).cr.cap_inh = inh;
    (*t).cr.cap_amb &= prm & inh;
    return 0 as ::core::ffi::c_long;
}
unsafe extern "C" fn h_prctl(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut res: *mut ::core::ffi::c_long,
) -> ::core::ffi::c_int {
    let mut op: ::core::ffi::c_long = eng_arg(r, 0 as ::core::ffi::c_int) as ::core::ffi::c_long;
    let mut a2: uint64_t = eng_arg(r, 1 as ::core::ffi::c_int);
    match op {
        PR_GET_KEEPCAPS => {
            *res = (if (*t).cr.securebits & SECBIT_KEEP_CAPS as uint32_t != 0 {
                1 as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            }) as ::core::ffi::c_long;
            return 1 as ::core::ffi::c_int;
        }
        PR_SET_KEEPCAPS => {
            if a2 > 1 as uint64_t {
                *res = -(EINVAL as ::core::ffi::c_long);
                return 1 as ::core::ffi::c_int;
            }
            if (*t).cr.securebits & (SECBIT_KEEP_CAPS as uint32_t) << 1 as ::core::ffi::c_int != 0 {
                *res = -(EPERM as ::core::ffi::c_long);
                return 1 as ::core::ffi::c_int;
            }
            if a2 != 0 {
                (*t).cr.securebits =
                    ((*t).cr.securebits as ::core::ffi::c_uint | SECBIT_KEEP_CAPS) as uint32_t;
            } else {
                (*t).cr.securebits =
                    ((*t).cr.securebits as ::core::ffi::c_uint & !SECBIT_KEEP_CAPS) as uint32_t;
            }
            *res = 0 as ::core::ffi::c_long;
            return 1 as ::core::ffi::c_int;
        }
        PR_CAPBSET_READ => {
            *res = if a2 > ENG_CAP_LAST as uint64_t {
                -(EINVAL as ::core::ffi::c_long)
            } else {
                ((*t).cr.cap_bnd >> a2 & 1 as uint64_t) as ::core::ffi::c_long
            };
            return 1 as ::core::ffi::c_int;
        }
        PR_CAPBSET_DROP => {
            if a2 > ENG_CAP_LAST as uint64_t {
                *res = -(EINVAL as ::core::ffi::c_long);
                return 1 as ::core::ffi::c_int;
            }
            if eng_capable(t, ENG_CAP_SETPCAP) == 0 {
                *res = -(EPERM as ::core::ffi::c_long);
                return 1 as ::core::ffi::c_int;
            }
            (*t).cr.cap_bnd = ((*t).cr.cap_bnd as ::core::ffi::c_ulonglong
                & !((1 as ::core::ffi::c_ulonglong) << a2))
                as uint64_t;
            *res = 0 as ::core::ffi::c_long;
            return 1 as ::core::ffi::c_int;
        }
        PR_GET_SECUREBITS => {
            *res = (*t).cr.securebits as ::core::ffi::c_long;
            return 1 as ::core::ffi::c_int;
        }
        PR_SET_SECUREBITS => {
            let mut nb: uint32_t = a2 as uint32_t;
            let mut ob: uint32_t = (*t).cr.securebits;
            if (ob & SECURE_ALL_LOCKS as uint32_t) >> 1 as ::core::ffi::c_int & (ob ^ nb) != 0
                || ob & SECURE_ALL_LOCKS as uint32_t & !nb != 0
                || nb & !(SECURE_ALL_LOCKS as uint32_t | SECURE_ALL_BITS as uint32_t) != 0
                || eng_capable(t, ENG_CAP_SETPCAP) == 0
            {
                *res = -(EPERM as ::core::ffi::c_long);
                return 1 as ::core::ffi::c_int;
            }
            (*t).cr.securebits = nb;
            *res = 0 as ::core::ffi::c_long;
            return 1 as ::core::ffi::c_int;
        }
        PR_CAP_AMBIENT => {
            let mut cap: uint64_t = eng_arg(r, 2 as ::core::ffi::c_int);
            if a2 == PR_CAP_AMBIENT_CLEAR_ALL as uint64_t {
                (*t).cr.cap_amb = 0 as uint64_t;
                *res = 0 as ::core::ffi::c_long;
                return 1 as ::core::ffi::c_int;
            }
            if cap > ENG_CAP_LAST as uint64_t {
                *res = -(EINVAL as ::core::ffi::c_long);
                return 1 as ::core::ffi::c_int;
            }
            let mut bit: uint64_t = ((1 as ::core::ffi::c_ulonglong) << cap) as uint64_t;
            if a2 == PR_CAP_AMBIENT_IS_SET as uint64_t {
                *res = (if (*t).cr.cap_amb & bit != 0 {
                    1 as ::core::ffi::c_int
                } else {
                    0 as ::core::ffi::c_int
                }) as ::core::ffi::c_long;
                return 1 as ::core::ffi::c_int;
            }
            if a2 == PR_CAP_AMBIENT_LOWER as uint64_t {
                (*t).cr.cap_amb &= !bit;
                *res = 0 as ::core::ffi::c_long;
                return 1 as ::core::ffi::c_int;
            }
            if a2 == PR_CAP_AMBIENT_RAISE as uint64_t {
                if (*t).cr.cap_prm & bit == 0
                    || (*t).cr.cap_inh & bit == 0
                    || (*t).cr.securebits & SECBIT_NO_CAP_AMBIENT_RAISE as uint32_t != 0
                {
                    *res = -(EPERM as ::core::ffi::c_long);
                    return 1 as ::core::ffi::c_int;
                }
                (*t).cr.cap_amb |= bit;
                *res = 0 as ::core::ffi::c_long;
                return 1 as ::core::ffi::c_int;
            }
            *res = -(EINVAL as ::core::ffi::c_long);
            return 1 as ::core::ffi::c_int;
        }
        PR_GET_NO_NEW_PRIVS => {
            *res = (*t).cr.nnp as ::core::ffi::c_long;
            return 1 as ::core::ffi::c_int;
        }
        PR_GET_DUMPABLE => {
            *res = ((*t).cr.undumpable == 0) as ::core::ffi::c_int as ::core::ffi::c_long;
            return 1 as ::core::ffi::c_int;
        }
        PR_SET_DUMPABLE => {
            if a2 > 1 as uint64_t {
                *res = -(EINVAL as ::core::ffi::c_long);
                return 1 as ::core::ffi::c_int;
            }
            (*t).cr.undumpable = (a2 == 0 as uint64_t) as ::core::ffi::c_int;
            *res = 0 as ::core::ffi::c_long;
            return 1 as ::core::ffi::c_int;
        }
        _ => return 0 as ::core::ffi::c_int,
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ident_entry(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
) -> ::core::ffi::c_int {
    let mut res: ::core::ffi::c_long = 0;
    match (*t).sysno {
        __NR_getuid => {
            res = (*t).cr.ruid as ::core::ffi::c_long;
        }
        __NR_geteuid => {
            res = (*t).cr.euid as ::core::ffi::c_long;
        }
        __NR_getgid => {
            res = (*t).cr.rgid as ::core::ffi::c_long;
        }
        __NR_getegid => {
            res = (*t).cr.egid as ::core::ffi::c_long;
        }
        __NR_getresuid => {
            let mut v: [uint32_t; 3] = [(*t).cr.ruid, (*t).cr.euid, (*t).cr.suid];
            res = put_u32s(
                t,
                r,
                0 as ::core::ffi::c_int,
                &raw mut v as *mut uint32_t,
                3 as ::core::ffi::c_int,
            ) as ::core::ffi::c_long;
        }
        __NR_getresgid => {
            let mut v_0: [uint32_t; 3] = [(*t).cr.rgid, (*t).cr.egid, (*t).cr.sgid];
            res = put_u32s(
                t,
                r,
                0 as ::core::ffi::c_int,
                &raw mut v_0 as *mut uint32_t,
                3 as ::core::ffi::c_int,
            ) as ::core::ffi::c_long;
        }
        __NR_setuid => {
            res =
                h_setuid(t, eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t) as ::core::ffi::c_long;
        }
        __NR_setgid => {
            res =
                h_setgid(t, eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t) as ::core::ffi::c_long;
        }
        __NR_setreuid => {
            res = h_setreuid(
                t,
                eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t,
                eng_arg(r, 1 as ::core::ffi::c_int) as uint32_t,
            ) as ::core::ffi::c_long;
        }
        __NR_setregid => {
            res = h_setregid(
                t,
                eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t,
                eng_arg(r, 1 as ::core::ffi::c_int) as uint32_t,
            ) as ::core::ffi::c_long;
        }
        __NR_setresuid => {
            res = set_resuid(
                t,
                eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t,
                eng_arg(r, 1 as ::core::ffi::c_int) as uint32_t,
                eng_arg(r, 2 as ::core::ffi::c_int) as uint32_t,
            ) as ::core::ffi::c_long;
        }
        __NR_setresgid => {
            res = set_resgid(
                t,
                eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t,
                eng_arg(r, 1 as ::core::ffi::c_int) as uint32_t,
                eng_arg(r, 2 as ::core::ffi::c_int) as uint32_t,
            ) as ::core::ffi::c_long;
        }
        __NR_setfsuid => {
            res = h_setfsuid(t, eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t);
        }
        __NR_setfsgid => {
            res = h_setfsgid(t, eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t);
        }
        __NR_getgroups => {
            res = h_getgroups(t, r);
        }
        __NR_setgroups => {
            res = h_setgroups(t, r);
        }
        __NR_capget => {
            res = h_capget(t, r);
        }
        __NR_capset => {
            res = h_capset(t, r);
        }
        __NR_prctl => {
            if eng_arg(r, 0 as ::core::ffi::c_int) as ::core::ffi::c_long
                == PR_SET_NO_NEW_PRIVS as ::core::ffi::c_long
                && eng_arg(r, 1 as ::core::ffi::c_int) == 1 as uint64_t
            {
                (*t).cr.nnp = 1 as ::core::ffi::c_int;
                return 0 as ::core::ffi::c_int;
            }
            if h_prctl(t, r, &raw mut res) == 0 {
                return 0 as ::core::ffi::c_int;
            }
        }
        95 => {
            (*t).umask_old = (*t).cr.umask as ::core::ffi::c_int;
            (*t).cr.umask = eng_arg(r, 0 as ::core::ffi::c_int) as uint32_t & 0o777 as uint32_t;
            eng_set_arg(
                r,
                0 as ::core::ffi::c_int,
                ((*t).cr.umask & 0o77 as uint32_t) as uint64_t,
            );
            (*t).regs_modified = 1 as ::core::ffi::c_int;
            return 1 as ::core::ffi::c_int;
        }
        _ => return -1 as ::core::ffi::c_int,
    }
    return eng_task_void(t, r, res);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ident_exit(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
) -> ::core::ffi::c_int {
    if (*t).sysno == __NR_umask as ::core::ffi::c_long && (*t).umask_old >= 0 as ::core::ffi::c_int
    {
        eng_set_ret(r, (*t).umask_old as uint64_t);
        (*t).umask_old = -1 as ::core::ffi::c_int;
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ident_setid(
    mut t: *const eng_task,
    mut mode: uint32_t,
    mut uid: uint32_t,
    mut gid: uint32_t,
    mut neuid: *mut uint32_t,
    mut negid: *mut uint32_t,
) -> ::core::ffi::c_int {
    *neuid = (*t).cr.euid;
    *negid = (*t).cr.egid;
    if (*t).cr.nnp != 0 {
        return 0 as ::core::ffi::c_int;
    }
    let mut setid: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if mode & S_ISUID as uint32_t != 0 {
        *neuid = uid;
        setid = 1 as ::core::ffi::c_int;
    }
    if mode & S_ISGID as uint32_t != 0 && mode & S_IXGRP as uint32_t != 0 {
        *negid = gid;
        setid = 1 as ::core::ffi::c_int;
    }
    return setid;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ident_exec(mut t: *mut eng_task) {
    let mut c: *mut eng_cred = &raw mut (*t).cr;
    let mut setid: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if (*t).plan_setid != 0 {
        if (*t).plan_euid != NOCHG && (*t).plan_euid != (*c).euid {
            (*c).euid = (*t).plan_euid;
            setid = 1 as ::core::ffi::c_int;
        }
        if (*t).plan_egid != NOCHG && (*t).plan_egid != (*c).egid {
            (*c).egid = (*t).plan_egid;
            setid = 1 as ::core::ffi::c_int;
        }
    }
    if setid != 0 {
        (*c).cap_amb = 0 as uint64_t;
    }
    if (*c).securebits & SECBIT_NOROOT as uint32_t == 0
        && ((*c).euid == 0 as uint32_t || (*c).ruid == 0 as uint32_t)
    {
        (*c).cap_prm =
            (((*c).cap_bnd | (*c).cap_inh) as ::core::ffi::c_ulonglong & ENG_CAP_FULL) as uint64_t;
        (*c).cap_eff = if (*c).euid == 0 as uint32_t {
            (*c).cap_prm
        } else {
            (*c).cap_amb
        };
    } else {
        (*c).cap_prm = (*c).cap_amb;
        (*c).cap_eff = (*c).cap_amb;
    }
    (*c).fsuid = (*c).euid;
    (*c).suid = (*c).fsuid;
    (*c).fsgid = (*c).egid;
    (*c).sgid = (*c).fsgid;
    (*c).securebits = ((*c).securebits as ::core::ffi::c_uint & !SECBIT_KEEP_CAPS) as uint32_t;
    (*c).undumpable = setid;
    (*t).plan_setid = 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn read_guest_file(
    mut g: *mut eng_guest,
    mut path: *const ::core::ffi::c_char,
    mut out: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    if eng_guest_to_host(
        g,
        path,
        &raw mut host as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return -1 as ::core::ffi::c_int;
    }
    let mut f: *mut FILE = fopen(
        &raw mut host as *mut ::core::ffi::c_char,
        b"re\0".as_ptr() as *const ::core::ffi::c_char,
    );
    if f.is_null() {
        return -1 as ::core::ffi::c_int;
    }
    let mut cap: size_t = ((1 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int) as size_t;
    let mut len: size_t = 0 as size_t;
    let mut buf: *mut ::core::ffi::c_char = malloc(cap) as *mut ::core::ffi::c_char;
    let mut n: size_t = 0;
    while !buf.is_null() && {
        n = fread(
            buf.offset(len as isize) as *mut ::core::ffi::c_void,
            1 as size_t,
            cap.wrapping_sub(len).wrapping_sub(1 as size_t),
            f,
        ) as size_t;
        n > 0 as size_t
    } {
        len = len.wrapping_add(n);
        if len.wrapping_add(1 as size_t) >= cap {
            cap = cap.wrapping_mul(2 as size_t);
            buf = realloc(buf as *mut ::core::ffi::c_void, cap) as *mut ::core::ffi::c_char;
        }
    }
    fclose(f);
    if buf.is_null() {
        return -1 as ::core::ffi::c_int;
    }
    *buf.offset(len as isize) = 0 as ::core::ffi::c_char;
    *out = buf;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ident_init(
    mut g: *mut eng_guest,
    mut t: *mut eng_task,
    mut uid: uint32_t,
    mut gid: uint32_t,
) {
    let mut c: *mut eng_cred = &raw mut (*t).cr;
    memset(
        c as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<eng_cred>(),
    );
    (*c).fsuid = uid;
    (*c).suid = (*c).fsuid;
    (*c).euid = (*c).suid;
    (*c).ruid = (*c).euid;
    (*c).fsgid = gid;
    (*c).sgid = (*c).fsgid;
    (*c).egid = (*c).sgid;
    (*c).rgid = (*c).egid;
    (*c).cap_bnd = ENG_CAP_FULL as uint64_t;
    if uid == 0 as uint32_t {
        (*c).cap_eff = ENG_CAP_FULL as uint64_t;
        (*c).cap_prm = (*c).cap_eff;
    }
    (*c).umask = 0o22 as uint32_t;
    (*t).umask_old = -1 as ::core::ffi::c_int;
    (*c).ngroups = 0 as uint32_t;
    let c2rust_fresh0 = (*c).ngroups;
    (*c).ngroups = (*c).ngroups.wrapping_add(1);
    (*c).groups[c2rust_fresh0 as usize] = gid;
    if g.is_null() {
        return;
    }
    let mut pw: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut gr: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut name: [::core::ffi::c_char; 256] = ::core::mem::transmute::<
        [u8; 256],
        [::core::ffi::c_char; 256],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    );
    if read_guest_file(
        g,
        b"/etc/passwd\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut pw,
    ) == 0 as ::core::ffi::c_int
    {
        let mut save: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
        let mut l: *mut ::core::ffi::c_char = strtok_r(
            pw,
            b"\n\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut save,
        );
        while !l.is_null() {
            let mut f: [*mut ::core::ffi::c_char; 4] = [
                ::core::ptr::null_mut::<::core::ffi::c_char>(),
                ::core::ptr::null_mut::<::core::ffi::c_char>(),
                ::core::ptr::null_mut::<::core::ffi::c_char>(),
                ::core::ptr::null_mut::<::core::ffi::c_char>(),
            ];
            let mut p: *mut ::core::ffi::c_char = l;
            let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i < 4 as ::core::ffi::c_int && !p.is_null() {
                f[i as usize] = p;
                p = strchr(p, ':' as ::core::ffi::c_int);
                if !p.is_null() {
                    let c2rust_fresh1 = p;
                    p = p.offset(1);
                    *c2rust_fresh1 = 0 as ::core::ffi::c_char;
                }
                i += 1;
            }
            if !f[2usize].is_null()
                && strtoul(
                    f[2usize],
                    ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                    10 as ::core::ffi::c_int,
                ) as uint32_t
                    == uid
            {
                snprintf(
                    &raw mut name as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 256]>(),
                    b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                    f[0usize],
                );
                break;
            } else {
                l = strtok_r(
                    ::core::ptr::null_mut::<::core::ffi::c_char>(),
                    b"\n\0".as_ptr() as *const ::core::ffi::c_char,
                    &raw mut save,
                );
            }
        }
        free(pw as *mut ::core::ffi::c_void);
    }
    if name[0usize] == 0
        || read_guest_file(
            g,
            b"/etc/group\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut gr,
        ) != 0 as ::core::ffi::c_int
    {
        return;
    }
    let mut save_0: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut l_0: *mut ::core::ffi::c_char = strtok_r(
        gr,
        b"\n\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut save_0,
    );
    while !l_0.is_null() {
        let mut f_0: [*mut ::core::ffi::c_char; 4] = [
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
        ];
        let mut p_0: *mut ::core::ffi::c_char = l_0;
        let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i_0 < 4 as ::core::ffi::c_int && !p_0.is_null() {
            f_0[i_0 as usize] = p_0;
            p_0 = strchr(p_0, ':' as ::core::ffi::c_int);
            if !p_0.is_null() {
                let c2rust_fresh2 = p_0;
                p_0 = p_0.offset(1);
                *c2rust_fresh2 = 0 as ::core::ffi::c_char;
            }
            i_0 += 1;
        }
        if !(f_0[2usize].is_null() || f_0[3usize].is_null()) {
            let mut id: uint32_t = strtoul(
                f_0[2usize],
                ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                10 as ::core::ffi::c_int,
            ) as uint32_t;
            let mut s2: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
            let mut m: *mut ::core::ffi::c_char = strtok_r(
                f_0[3usize],
                b",\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut s2,
            );
            while !m.is_null() {
                if !(strcmp(m, &raw mut name as *mut ::core::ffi::c_char) != 0 || id == gid) {
                    if ((*c).ngroups as usize)
                        < ::core::mem::size_of::<[uint32_t; 32]>()
                            .wrapping_div(::core::mem::size_of::<uint32_t>())
                    {
                        let c2rust_fresh3 = (*c).ngroups;
                        (*c).ngroups = (*c).ngroups.wrapping_add(1);
                        (*c).groups[c2rust_fresh3 as usize] = id;
                    }
                }
                m = strtok_r(
                    ::core::ptr::null_mut::<::core::ffi::c_char>(),
                    b",\0".as_ptr() as *const ::core::ffi::c_char,
                    &raw mut s2,
                );
            }
        }
        l_0 = strtok_r(
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            b"\n\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut save_0,
        );
    }
    free(gr as *mut ::core::ffi::c_void);
}
