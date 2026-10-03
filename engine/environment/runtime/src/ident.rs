//! Virtual credentials, capabilities and Linux set-id transition rules.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    unsafe fn eng_arg(_: *const eng_regs, _: i32) -> u64;
    unsafe fn eng_set_arg(_: *mut eng_regs, _: i32, _: u64);
    unsafe fn eng_set_ret(_: *mut eng_regs, _: u64);
    unsafe fn eng_task_find(_: *mut eng_tracer, _: i32) -> *mut eng_task;
    unsafe fn eng_task_void(_: *mut eng_task, _: *mut eng_regs, _: i64) -> i32;
    unsafe fn fclose(_: *mut FILE) -> i32;
    unsafe fn fopen(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> *mut FILE;
    unsafe fn snprintf(
        _: *mut ::core::ffi::c_char,
        _: usize,
        _: *const ::core::ffi::c_char,
        ...
    ) -> i32;
    unsafe fn fread(_: *mut ::core::ffi::c_void, _: usize, _: usize, _: *mut FILE) -> u64;
    unsafe fn strtoul(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
        _: i32,
    ) -> u64;
    unsafe fn malloc(_: usize) -> *mut ::core::ffi::c_void;
    #[cfg(target_os = "linux")]
    unsafe fn realloc(_: *mut ::core::ffi::c_void, _: usize) -> *mut ::core::ffi::c_void;
    #[cfg(target_os = "android")]
    unsafe fn realloc(_: *mut ::core::ffi::c_void, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn free(_: *mut ::core::ffi::c_void);
    unsafe fn memset(_: *mut ::core::ffi::c_void, _: i32, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn strcmp(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn strchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strtok_r(
        _: *mut ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn eng_guest_to_host(
        _: *const eng_guest,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn eng_mem_read(_: i32, _: usize, _: *mut ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn eng_mem_write(_: i32, _: usize, _: *const ::core::ffi::c_void, _: usize) -> isize;
}
#[repr(C)]
pub struct eng_tracer {
    _opaque: [u8; 0],
}
#[cfg(target_os = "linux")]
#[repr(C)]
pub struct _IO_wide_data {
    _opaque: [u8; 0],
}
#[cfg(target_os = "linux")]
#[repr(C)]
pub struct _IO_codecvt {
    _opaque: [u8; 0],
}
#[cfg(target_os = "linux")]
#[repr(C)]
pub struct _IO_marker {
    _opaque: [u8; 0],
}
#[cfg(target_arch = "x86_64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct user_regs_struct {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub orig_rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub eflags: u64,
    pub rsp: u64,
    pub ss: u64,
    pub fs_base: u64,
    pub gs_base: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct user_regs_struct {
    pub regs: [u64; 31],
    pub sp: u64,
    pub pc: u64,
    pub pstate: u64,
}
pub type eng_regs = user_regs_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_guest {
    pub root: [::core::ffi::c_char; 4096],
    pub rootlen: usize,
    pub rootfd: i32,
    pub binds: [eng_bind; 32],
    pub nbinds: i32,
    pub hides: [[::core::ffi::c_char; 4096]; 16],
    pub nhides: i32,
    pub binfmt: [eng_binfmt; 32],
    pub nbinfmt: i32,
    pub loader: [::core::ffi::c_char; 4096],
    pub sockdir: [::core::ffi::c_char; 4096],
    pub no_filemap: i32,
    pub test_pagesz: u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_binfmt {
    pub name: [::core::ffi::c_char; 64],
    pub r#type: ::core::ffi::c_char,
    pub offset: u32,
    pub magic: [u8; 128],
    pub mask: [u8; 128],
    pub len: u32,
    pub ext: [::core::ffi::c_char; 64],
    pub interp: [::core::ffi::c_char; 4096],
    pub preserve_argv0: i32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_bind {
    pub guest: [::core::ffi::c_char; 4096],
    pub host: [::core::ffi::c_char; 4096],
    pub glen: usize,
    pub hlen: usize,
    pub uid: u32,
    pub gid: u32,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_phase(pub u32);
impl eng_phase {
    pub const ENG_PH_BOOT: Self = Self(0);
    pub const ENG_PH_LOADER: Self = Self(1);
    pub const ENG_PH_GUEST: Self = Self(2);
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_mm {
    pub refs: i32,
    pub base: u64,
    pub size: u32,
    pub nslots: u32,
    pub used: [u8; 512],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_proc {
    pub refs: i32,
    pub exe: [::core::ffi::c_char; 4096],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_cred {
    pub ruid: u32,
    pub euid: u32,
    pub suid: u32,
    pub fsuid: u32,
    pub rgid: u32,
    pub egid: u32,
    pub sgid: u32,
    pub fsgid: u32,
    pub ngroups: u32,
    pub groups: [u32; 32],
    pub cap_eff: u64,
    pub cap_prm: u64,
    pub cap_inh: u64,
    pub cap_amb: u64,
    pub cap_bnd: u64,
    pub securebits: u32,
    pub nnp: i32,
    pub undumpable: i32,
    pub umask: u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_task {
    pub tid: i32,
    pub tgid: i32,
    pub in_use: i32,
    pub linked: i32,
    pub held: i32,
    pub phase: eng_phase,
    pub at_syscall_entry: i32,
    pub await_exit: i32,
    pub sysno: i64,
    pub void_pending: i32,
    pub inject_result: i64,
    pub regs_modified: i32,
    pub entry_regs: eng_regs,
    pub mm: *mut eng_mm,
    pub slot: i32,
    pub slot_off: u32,
    pub stack_scratch: u64,
    pub proc: *mut eng_proc,
    pub plan: *mut ::core::ffi::c_void,
    pub plan_len: u32,
    pub plan_exe: [::core::ffi::c_char; 4096],
    pub fixup: i32,
    pub fix_addr: u64,
    pub fix_len: u64,
    pub fix_aux: i64,
    pub fix_nofollow: i32,
    pub fix_mode: u32,
    pub fix_id: [::core::ffi::c_char; 64],
    pub fix_path: [::core::ffi::c_char; 4096],
    pub plan_setid: i32,
    pub plan_euid: u32,
    pub plan_egid: u32,
    pub lg_fix: i32,
    pub lg_a: u64,
    pub lg_b: u64,
    pub lg_restart: i32,
    pub lg_nr: i64,
    pub lg_saved: eng_regs,
    pub cr: eng_cred,
    pub umask_old: i32,
    pub tr: *mut eng_tracer,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_log_level(pub u32);
impl eng_log_level {
    pub const ENG_LOG_ERROR: Self = Self(0);
    pub const ENG_LOG_WARN: Self = Self(1);
    pub const ENG_LOG_INFO: Self = Self(2);
    pub const ENG_LOG_DEBUG: Self = Self(3);
    pub const ENG_LOG_TRACE: Self = Self(4);
}
#[cfg(target_os = "linux")]
pub type FILE = _IO_FILE;
#[cfg(target_os = "android")]
pub type FILE = __sFILE;
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: i32,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: i32,
    pub _flags2: [u8; 3],
    pub _short_backupbuf: [::core::ffi::c_char; 1],
    pub _old_offset: i64,
    pub _cur_column: u16,
    pub _vtable_offset: i8,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: i64,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut ::core::ffi::c_void,
    pub _prevchain: *mut *mut _IO_FILE,
    pub _mode: i32,
    pub _unused3: i32,
    pub _total_written: u64,
    pub _unused2: [::core::ffi::c_char; 8],
}
#[cfg(target_os = "linux")]
pub type _IO_lock_t = ();
pub const ENG_CAP_SETGID: i32 = 6 as i32;
pub const ENG_CAP_SETUID: i32 = 7 as i32;
pub const ENG_CAP_SETPCAP: i32 = 8 as i32;
pub const ENG_CAP_LAST: i32 = 40 as i32;
pub const ENG_CAP_FULL: u64 = ((1 as u64) << ENG_CAP_LAST + 1 as i32).wrapping_sub(1 as u64);
#[inline]
unsafe extern "C" fn eng_capable(mut t: *const eng_task, mut cap: i32) -> i32 {
    return ((*t).cr.cap_eff >> cap & 1 as u64) as i32;
}
pub const EPERM: i32 = 1 as i32;
pub const ESRCH: i32 = 3 as i32;
pub const EFAULT: i32 = 14 as i32;
pub const EINVAL: i32 = 22 as i32;
pub const PR_GET_DUMPABLE: i64 = 3 as i64;
pub const PR_SET_DUMPABLE: i64 = 4 as i64;
pub const PR_GET_KEEPCAPS: i64 = 7 as i64;
pub const PR_SET_KEEPCAPS: i64 = 8 as i64;
pub const PR_CAPBSET_READ: i64 = 23 as i64;
pub const PR_CAPBSET_DROP: i64 = 24 as i64;
pub const PR_GET_SECUREBITS: i64 = 27 as i64;
pub const PR_SET_SECUREBITS: i64 = 28 as i64;
pub const PR_SET_NO_NEW_PRIVS: i32 = 38 as i32;
pub const PR_GET_NO_NEW_PRIVS: i64 = 39 as i64;
pub const PR_CAP_AMBIENT: i64 = 47 as i64;
pub const PR_CAP_AMBIENT_IS_SET: i32 = 1 as i32;
pub const PR_CAP_AMBIENT_RAISE: i32 = 2 as i32;
pub const PR_CAP_AMBIENT_LOWER: i32 = 3 as i32;
pub const PR_CAP_AMBIENT_CLEAR_ALL: i32 = 4 as i32;
#[cfg(target_os = "linux")]
pub const __S_ISUID: i32 = 0o4000 as i32;
#[cfg(target_os = "linux")]
pub const __S_ISGID: i32 = 0o2000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IEXEC: i32 = 0o100 as i32;
#[cfg(target_os = "linux")]
pub const S_ISUID: i32 = __S_ISUID;
#[cfg(target_os = "android")]
pub const S_ISUID: i32 = 0o4000 as i32;
#[cfg(target_os = "linux")]
pub const S_ISGID: i32 = __S_ISGID;
#[cfg(target_os = "android")]
pub const S_ISGID: i32 = 0o2000 as i32;
#[cfg(target_os = "linux")]
pub const S_IXUSR: i32 = __S_IEXEC;
#[cfg(target_os = "linux")]
pub const S_IXGRP: i32 = S_IXUSR >> 3 as i32;
#[cfg(target_os = "android")]
pub const S_IXGRP: i32 = 0o10 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_umask: i32 = 95 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_umask: i32 = 166 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_getuid: i64 = 102 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_getuid: i64 = 174 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_getgid: i64 = 104 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_getgid: i64 = 176 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_setuid: i64 = 105 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_setuid: i64 = 146 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_setgid: i64 = 106 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_setgid: i64 = 144 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_geteuid: i64 = 107 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_geteuid: i64 = 175 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_getegid: i64 = 108 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_getegid: i64 = 177 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_setreuid: i64 = 113 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_setreuid: i64 = 145 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_setregid: i64 = 114 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_setregid: i64 = 143 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_getgroups: i64 = 115 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_getgroups: i64 = 158 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_setgroups: i64 = 116 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_setgroups: i64 = 159 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_setresuid: i64 = 117 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_setresuid: i64 = 147 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_getresuid: i64 = 118 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_getresuid: i64 = 148 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_setresgid: i64 = 119 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_setresgid: i64 = 149 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_getresgid: i64 = 120 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_getresgid: i64 = 150 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_setfsuid: i64 = 122 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_setfsuid: i64 = 151 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_setfsgid: i64 = 123 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_setfsgid: i64 = 152 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_capget: i64 = 125 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_capget: i64 = 90 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_capset: i64 = 126 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_capset: i64 = 91 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_prctl: i64 = 157 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_prctl: i64 = 167 as i64;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NOCHG: u32 = -1 as i32 as u32;
pub const SECBIT_NOROOT: u32 = (1 as u32) << 0 as i32;
pub const SECBIT_NO_SETUID_FIXUP: u32 = (1 as u32) << 2 as i32;
pub const SECBIT_KEEP_CAPS: u32 = (1 as u32) << 4 as i32;
pub const SECBIT_NO_CAP_AMBIENT_RAISE: u32 = (1 as u32) << 6 as i32;
pub const SECURE_ALL_BITS: u32 = 0x55 as u32;
pub const SECURE_ALL_LOCKS: u32 = SECURE_ALL_BITS << 1 as i32;
pub const CAP_FS_MASK: u64 = (1 as u64) << 0 as i32
    | (1 as u64) << 1 as i32
    | (1 as u64) << 2 as i32
    | (1 as u64) << 3 as i32
    | (1 as u64) << 4 as i32
    | (1 as u64) << 9 as i32
    | (1 as u64) << 27 as i32
    | (1 as u64) << 32 as i32;
unsafe extern "C" fn is_one_of(mut v: u32, mut a: u32, mut b: u32, mut c: u32) -> i32 {
    return (v == a || v == b || v == c) as i32;
}
unsafe extern "C" fn put_u32s(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut first_arg: i32,
    mut v: *const u32,
    mut n: i32,
) -> i32 {
    let mut i: i32 = 0 as i32;
    while i < n {
        let mut a: u64 = eng_arg(r, first_arg + i);
        if a == 0
            || eng_mem_write(
                (*t).tid,
                a as usize,
                v.offset(i as isize) as *const ::core::ffi::c_void,
                4 as usize,
            ) != 4 as isize
        {
            return -EFAULT;
        }
        i += 1;
    }
    return 0 as i32;
}
unsafe extern "C" fn fix_setxuid(mut c: *mut eng_cred, mut old: *const eng_cred) {
    if (*c).securebits & SECBIT_NO_SETUID_FIXUP as u32 != 0 {
        return;
    }
    if ((*old).ruid == 0 as u32 || (*old).euid == 0 as u32 || (*old).suid == 0 as u32)
        && ((*c).ruid != 0 as u32 && (*c).euid != 0 as u32 && (*c).suid != 0 as u32)
    {
        if (*c).securebits & SECBIT_KEEP_CAPS as u32 == 0 {
            (*c).cap_prm = 0 as u64;
            (*c).cap_eff = 0 as u64;
        }
        (*c).cap_amb = 0 as u64;
    }
    if (*old).euid == 0 as u32 && (*c).euid != 0 as u32 {
        (*c).cap_eff = 0 as u64;
    }
    if (*old).euid != 0 as u32 && (*c).euid == 0 as u32 {
        (*c).cap_eff = (*c).cap_prm;
    }
}
unsafe extern "C" fn fix_setfsuid(mut c: *mut eng_cred, mut old_fsuid: u32) {
    if (*c).securebits & SECBIT_NO_SETUID_FIXUP as u32 != 0 {
        return;
    }
    if old_fsuid == 0 as u32 && (*c).fsuid != 0 as u32 {
        (*c).cap_eff = ((*c).cap_eff as u64 & !CAP_FS_MASK) as u64;
    }
    if old_fsuid != 0 as u32 && (*c).fsuid == 0 as u32 {
        (*c).cap_eff = ((*c).cap_eff as u64 | (*c).cap_prm as u64 & CAP_FS_MASK) as u64;
    }
}
unsafe extern "C" fn set_resuid(
    mut t: *mut eng_task,
    mut ru: u32,
    mut eu: u32,
    mut su: u32,
) -> i32 {
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
    return 0 as i32;
}
unsafe extern "C" fn set_resgid(
    mut t: *mut eng_task,
    mut rg: u32,
    mut eg: u32,
    mut sg: u32,
) -> i32 {
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
    return 0 as i32;
}
unsafe extern "C" fn h_setuid(mut t: *mut eng_task, mut u: u32) -> i32 {
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
    return 0 as i32;
}
unsafe extern "C" fn h_setgid(mut t: *mut eng_task, mut g: u32) -> i32 {
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
    return 0 as i32;
}
unsafe extern "C" fn h_setreuid(mut t: *mut eng_task, mut ru: u32, mut eu: u32) -> i32 {
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
    return 0 as i32;
}
unsafe extern "C" fn h_setregid(mut t: *mut eng_task, mut rg: u32, mut eg: u32) -> i32 {
    let mut old_r: u32 = (*t).cr.rgid;
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
    return 0 as i32;
}
unsafe extern "C" fn h_setfsuid(mut t: *mut eng_task, mut f: u32) -> i64 {
    let mut old: u32 = (*t).cr.fsuid;
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
    return old as i64;
}
unsafe extern "C" fn h_setfsgid(mut t: *mut eng_task, mut f: u32) -> i64 {
    let mut old: i64 = (*t).cr.fsgid as i64;
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
unsafe extern "C" fn h_getgroups(mut t: *mut eng_task, mut r: *mut eng_regs) -> i64 {
    let mut size: i64 = eng_arg(r, 0 as i32) as u32 as i32 as i64;
    if size < 0 as i64 {
        return -(EINVAL as i64);
    }
    if size == 0 as i64 {
        return (*t).cr.ngroups as i64;
    }
    if (size as u32) < (*t).cr.ngroups {
        return -(EINVAL as i64);
    }
    let mut n: usize = ((*t).cr.ngroups as usize).wrapping_mul(::core::mem::size_of::<u32>());
    if n != 0
        && eng_mem_write(
            (*t).tid,
            eng_arg(r, 1 as i32) as usize,
            &raw mut (*t).cr.groups as *mut u32 as *const ::core::ffi::c_void,
            n,
        ) != n as isize
    {
        return -(EFAULT as i64);
    }
    return (*t).cr.ngroups as i64;
}
unsafe extern "C" fn h_setgroups(mut t: *mut eng_task, mut r: *mut eng_regs) -> i64 {
    let mut size: i64 = eng_arg(r, 0 as i32) as u32 as i32 as i64;
    if eng_capable(t, ENG_CAP_SETGID) == 0 {
        return -(EPERM as i64);
    }
    if size < 0 as i64 || size > 65536 as i64 {
        return -(EINVAL as i64);
    }
    let mut n: u32 = size as u32;
    let cap: u32 =
        ::core::mem::size_of::<[u32; 32]>().wrapping_div(::core::mem::size_of::<u32>()) as u32;
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
            eng_arg(r, 1 as i32) as usize,
            &raw mut (*t).cr.groups as *mut u32 as *mut ::core::ffi::c_void,
            n.wrapping_mul(4 as u32) as usize,
        ) != n.wrapping_mul(4 as u32) as isize
    {
        return -(EFAULT as i64);
    }
    (*t).cr.ngroups = n;
    return 0 as i64;
}
pub const CAP_V1: u32 = 0x19980330 as u32;
pub const CAP_V2: u32 = 0x20071026 as u32;
pub const CAP_V3: u32 = 0x20080522 as u32;
unsafe extern "C" fn h_capget(mut t: *mut eng_task, mut r: *mut eng_regs) -> i64 {
    let mut hdr: [u32; 2] = [0; 2];
    if eng_mem_read(
        (*t).tid,
        eng_arg(r, 0 as i32) as usize,
        &raw mut hdr as *mut u32 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[u32; 2]>(),
    ) != ::core::mem::size_of::<[u32; 2]>() as isize
    {
        return -(EFAULT as i64);
    }
    let mut words: i32 = 0;
    if hdr[0usize] == CAP_V1 as u32 {
        words = 1 as i32;
    } else if hdr[0usize] == CAP_V2 as u32 || hdr[0usize] == CAP_V3 as u32 {
        words = 2 as i32;
    } else {
        let mut v: u32 = CAP_V3 as u32;
        eng_mem_write(
            (*t).tid,
            eng_arg(r, 0 as i32) as usize,
            &raw mut v as *const ::core::ffi::c_void,
            4 as usize,
        );
        return (if eng_arg(r, 1 as i32) != 0 {
            -EINVAL
        } else {
            0 as i32
        }) as i64;
    }
    if hdr[1usize] != 0 as u32 && hdr[1usize] != (*t).tgid as u32 && hdr[1usize] != (*t).tid as u32
    {
        let mut o: *mut eng_task = eng_task_find((*t).tr, hdr[1usize] as i32);
        if o.is_null() {
            return -(ESRCH as i64);
        }
        t = if o.is_null() { t } else { o };
    }
    if eng_arg(r, 1 as i32) == 0 {
        return 0 as i64;
    }
    let mut data: [u32; 6] = [0; 6];
    let mut w: i32 = 0 as i32;
    while w < 2 as i32 {
        data[(w * 3 as i32 + 0 as i32) as usize] = ((*t).cr.cap_eff >> 32 as i32 * w) as u32;
        data[(w * 3 as i32 + 1 as i32) as usize] = ((*t).cr.cap_prm >> 32 as i32 * w) as u32;
        data[(w * 3 as i32 + 2 as i32) as usize] = ((*t).cr.cap_inh >> 32 as i32 * w) as u32;
        w += 1;
    }
    let mut n: usize = (words as usize).wrapping_mul(12 as usize);
    return (if eng_mem_write(
        (*t).tid,
        eng_arg(r, 1 as i32) as usize,
        &raw mut data as *mut u32 as *const ::core::ffi::c_void,
        n,
    ) == n as isize
    {
        0 as i32
    } else {
        -EFAULT
    }) as i64;
}
unsafe extern "C" fn h_capset(mut t: *mut eng_task, mut r: *mut eng_regs) -> i64 {
    let mut hdr: [u32; 2] = [0; 2];
    if eng_mem_read(
        (*t).tid,
        eng_arg(r, 0 as i32) as usize,
        &raw mut hdr as *mut u32 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[u32; 2]>(),
    ) != ::core::mem::size_of::<[u32; 2]>() as isize
    {
        return -(EFAULT as i64);
    }
    let mut words: i32 = 0;
    if hdr[0usize] == CAP_V1 as u32 {
        words = 1 as i32;
    } else if hdr[0usize] == CAP_V2 as u32 || hdr[0usize] == CAP_V3 as u32 {
        words = 2 as i32;
    } else {
        let mut v: u32 = CAP_V3 as u32;
        eng_mem_write(
            (*t).tid,
            eng_arg(r, 0 as i32) as usize,
            &raw mut v as *const ::core::ffi::c_void,
            4 as usize,
        );
        return -(EINVAL as i64);
    }
    if hdr[1usize] != 0 as u32 && hdr[1usize] != (*t).tgid as u32 && hdr[1usize] != (*t).tid as u32
    {
        return -(EPERM as i64);
    }
    let mut data: [u32; 6] = [0 as u32, 0, 0, 0, 0, 0];
    let mut n: usize = (words as usize).wrapping_mul(12 as usize);
    if eng_mem_read(
        (*t).tid,
        eng_arg(r, 1 as i32) as usize,
        &raw mut data as *mut u32 as *mut ::core::ffi::c_void,
        n,
    ) != n as isize
    {
        return -(EFAULT as i64);
    }
    let mut eff: u64 = 0 as u64;
    let mut prm: u64 = 0 as u64;
    let mut inh: u64 = 0 as u64;
    let mut w: i32 = 0 as i32;
    while w < words {
        eff |= (data[(w * 3 as i32 + 0 as i32) as usize] as u64) << 32 as i32 * w;
        prm |= (data[(w * 3 as i32 + 1 as i32) as usize] as u64) << 32 as i32 * w;
        inh |= (data[(w * 3 as i32 + 2 as i32) as usize] as u64) << 32 as i32 * w;
        w += 1;
    }
    eff = (eff as u64 & ENG_CAP_FULL) as u64;
    prm = (prm as u64 & ENG_CAP_FULL) as u64;
    inh = (inh as u64 & ENG_CAP_FULL) as u64;
    if words == 1 as i32 {
        eff = (eff as u64 | (*t).cr.cap_eff as u64 & !(0xffffffff as u64)) as u64;
        prm = (prm as u64 | (*t).cr.cap_prm as u64 & !(0xffffffff as u64)) as u64;
        inh = (inh as u64 | (*t).cr.cap_inh as u64 & !(0xffffffff as u64)) as u64;
    }
    let mut allowed_inh: u64 = ((*t).cr.cap_inh as u64
        | if eng_capable(t, ENG_CAP_SETPCAP) != 0 {
            ENG_CAP_FULL
        } else {
            (*t).cr.cap_prm as u64
        }) as u64;
    if inh & !allowed_inh != 0 {
        return -(EPERM as i64);
    }
    if inh & !((*t).cr.cap_inh | (*t).cr.cap_bnd) != 0 {
        return -(EPERM as i64);
    }
    if prm & !(*t).cr.cap_prm != 0 {
        return -(EPERM as i64);
    }
    if eff & !prm != 0 {
        return -(EPERM as i64);
    }
    (*t).cr.cap_eff = eff;
    (*t).cr.cap_prm = prm;
    (*t).cr.cap_inh = inh;
    (*t).cr.cap_amb &= prm & inh;
    return 0 as i64;
}
unsafe extern "C" fn h_prctl(mut t: *mut eng_task, mut r: *mut eng_regs, mut res: *mut i64) -> i32 {
    let mut op: i64 = eng_arg(r, 0 as i32) as i64;
    let mut a2: u64 = eng_arg(r, 1 as i32);
    match op {
        PR_GET_KEEPCAPS => {
            *res = (if (*t).cr.securebits & SECBIT_KEEP_CAPS as u32 != 0 {
                1 as i32
            } else {
                0 as i32
            }) as i64;
            return 1 as i32;
        }
        PR_SET_KEEPCAPS => {
            if a2 > 1 as u64 {
                *res = -(EINVAL as i64);
                return 1 as i32;
            }
            if (*t).cr.securebits & (SECBIT_KEEP_CAPS as u32) << 1 as i32 != 0 {
                *res = -(EPERM as i64);
                return 1 as i32;
            }
            if a2 != 0 {
                (*t).cr.securebits = ((*t).cr.securebits as u32 | SECBIT_KEEP_CAPS) as u32;
            } else {
                (*t).cr.securebits = ((*t).cr.securebits as u32 & !SECBIT_KEEP_CAPS) as u32;
            }
            *res = 0 as i64;
            return 1 as i32;
        }
        PR_CAPBSET_READ => {
            *res = if a2 > ENG_CAP_LAST as u64 {
                -(EINVAL as i64)
            } else {
                ((*t).cr.cap_bnd >> a2 & 1 as u64) as i64
            };
            return 1 as i32;
        }
        PR_CAPBSET_DROP => {
            if a2 > ENG_CAP_LAST as u64 {
                *res = -(EINVAL as i64);
                return 1 as i32;
            }
            if eng_capable(t, ENG_CAP_SETPCAP) == 0 {
                *res = -(EPERM as i64);
                return 1 as i32;
            }
            (*t).cr.cap_bnd = ((*t).cr.cap_bnd as u64 & !((1 as u64) << a2)) as u64;
            *res = 0 as i64;
            return 1 as i32;
        }
        PR_GET_SECUREBITS => {
            *res = (*t).cr.securebits as i64;
            return 1 as i32;
        }
        PR_SET_SECUREBITS => {
            let mut nb: u32 = a2 as u32;
            let mut ob: u32 = (*t).cr.securebits;
            if (ob & SECURE_ALL_LOCKS as u32) >> 1 as i32 & (ob ^ nb) != 0
                || ob & SECURE_ALL_LOCKS as u32 & !nb != 0
                || nb & !(SECURE_ALL_LOCKS as u32 | SECURE_ALL_BITS as u32) != 0
                || eng_capable(t, ENG_CAP_SETPCAP) == 0
            {
                *res = -(EPERM as i64);
                return 1 as i32;
            }
            (*t).cr.securebits = nb;
            *res = 0 as i64;
            return 1 as i32;
        }
        PR_CAP_AMBIENT => {
            let mut cap: u64 = eng_arg(r, 2 as i32);
            if a2 == PR_CAP_AMBIENT_CLEAR_ALL as u64 {
                (*t).cr.cap_amb = 0 as u64;
                *res = 0 as i64;
                return 1 as i32;
            }
            if cap > ENG_CAP_LAST as u64 {
                *res = -(EINVAL as i64);
                return 1 as i32;
            }
            let mut bit: u64 = ((1 as u64) << cap) as u64;
            if a2 == PR_CAP_AMBIENT_IS_SET as u64 {
                *res = (if (*t).cr.cap_amb & bit != 0 {
                    1 as i32
                } else {
                    0 as i32
                }) as i64;
                return 1 as i32;
            }
            if a2 == PR_CAP_AMBIENT_LOWER as u64 {
                (*t).cr.cap_amb &= !bit;
                *res = 0 as i64;
                return 1 as i32;
            }
            if a2 == PR_CAP_AMBIENT_RAISE as u64 {
                if (*t).cr.cap_prm & bit == 0
                    || (*t).cr.cap_inh & bit == 0
                    || (*t).cr.securebits & SECBIT_NO_CAP_AMBIENT_RAISE as u32 != 0
                {
                    *res = -(EPERM as i64);
                    return 1 as i32;
                }
                (*t).cr.cap_amb |= bit;
                *res = 0 as i64;
                return 1 as i32;
            }
            *res = -(EINVAL as i64);
            return 1 as i32;
        }
        PR_GET_NO_NEW_PRIVS => {
            *res = (*t).cr.nnp as i64;
            return 1 as i32;
        }
        PR_GET_DUMPABLE => {
            *res = ((*t).cr.undumpable == 0) as i32 as i64;
            return 1 as i32;
        }
        PR_SET_DUMPABLE => {
            if a2 > 1 as u64 {
                *res = -(EINVAL as i64);
                return 1 as i32;
            }
            (*t).cr.undumpable = (a2 == 0 as u64) as i32;
            *res = 0 as i64;
            return 1 as i32;
        }
        _ => return 0 as i32,
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ident_entry(mut t: *mut eng_task, mut r: *mut eng_regs) -> i32 {
    let mut res: i64 = 0;
    match (*t).sysno {
        __NR_getuid => {
            res = (*t).cr.ruid as i64;
        }
        __NR_geteuid => {
            res = (*t).cr.euid as i64;
        }
        __NR_getgid => {
            res = (*t).cr.rgid as i64;
        }
        __NR_getegid => {
            res = (*t).cr.egid as i64;
        }
        __NR_getresuid => {
            let mut v: [u32; 3] = [(*t).cr.ruid, (*t).cr.euid, (*t).cr.suid];
            res = put_u32s(t, r, 0 as i32, &raw mut v as *mut u32, 3 as i32) as i64;
        }
        __NR_getresgid => {
            let mut v_0: [u32; 3] = [(*t).cr.rgid, (*t).cr.egid, (*t).cr.sgid];
            res = put_u32s(t, r, 0 as i32, &raw mut v_0 as *mut u32, 3 as i32) as i64;
        }
        __NR_setuid => {
            res = h_setuid(t, eng_arg(r, 0 as i32) as u32) as i64;
        }
        __NR_setgid => {
            res = h_setgid(t, eng_arg(r, 0 as i32) as u32) as i64;
        }
        __NR_setreuid => {
            res = h_setreuid(t, eng_arg(r, 0 as i32) as u32, eng_arg(r, 1 as i32) as u32) as i64;
        }
        __NR_setregid => {
            res = h_setregid(t, eng_arg(r, 0 as i32) as u32, eng_arg(r, 1 as i32) as u32) as i64;
        }
        __NR_setresuid => {
            res = set_resuid(
                t,
                eng_arg(r, 0 as i32) as u32,
                eng_arg(r, 1 as i32) as u32,
                eng_arg(r, 2 as i32) as u32,
            ) as i64;
        }
        __NR_setresgid => {
            res = set_resgid(
                t,
                eng_arg(r, 0 as i32) as u32,
                eng_arg(r, 1 as i32) as u32,
                eng_arg(r, 2 as i32) as u32,
            ) as i64;
        }
        __NR_setfsuid => {
            res = h_setfsuid(t, eng_arg(r, 0 as i32) as u32);
        }
        __NR_setfsgid => {
            res = h_setfsgid(t, eng_arg(r, 0 as i32) as u32);
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
            if eng_arg(r, 0 as i32) as i64 == PR_SET_NO_NEW_PRIVS as i64
                && eng_arg(r, 1 as i32) == 1 as u64
            {
                (*t).cr.nnp = 1 as i32;
                return 0 as i32;
            }
            if h_prctl(t, r, &raw mut res) == 0 {
                return 0 as i32;
            }
        }
        #[cfg(target_arch = "x86_64")]
        95 => {
            (*t).umask_old = (*t).cr.umask as i32;
            (*t).cr.umask = eng_arg(r, 0 as i32) as u32 & 0o777 as u32;
            eng_set_arg(r, 0 as i32, ((*t).cr.umask & 0o77 as u32) as u64);
            (*t).regs_modified = 1 as i32;
            return 1 as i32;
        }
        #[cfg(target_arch = "aarch64")]
        166 => {
            (*t).umask_old = (*t).cr.umask as i32;
            (*t).cr.umask = eng_arg(r, 0 as i32) as u32 & 0o777 as u32;
            eng_set_arg(r, 0 as i32, ((*t).cr.umask & 0o77 as u32) as u64);
            (*t).regs_modified = 1 as i32;
            return 1 as i32;
        }
        _ => return -1 as i32,
    }
    return eng_task_void(t, r, res);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ident_exit(mut t: *mut eng_task, mut r: *mut eng_regs) -> i32 {
    if (*t).sysno == __NR_umask as i64 && (*t).umask_old >= 0 as i32 {
        eng_set_ret(r, (*t).umask_old as u64);
        (*t).umask_old = -1 as i32;
        return 1 as i32;
    }
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ident_setid(
    mut t: *const eng_task,
    mut mode: u32,
    mut uid: u32,
    mut gid: u32,
    mut neuid: *mut u32,
    mut negid: *mut u32,
) -> i32 {
    *neuid = (*t).cr.euid;
    *negid = (*t).cr.egid;
    if (*t).cr.nnp != 0 {
        return 0 as i32;
    }
    let mut setid: i32 = 0 as i32;
    if mode & S_ISUID as u32 != 0 {
        *neuid = uid;
        setid = 1 as i32;
    }
    if mode & S_ISGID as u32 != 0 && mode & S_IXGRP as u32 != 0 {
        *negid = gid;
        setid = 1 as i32;
    }
    return setid;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ident_exec(mut t: *mut eng_task) {
    let mut c: *mut eng_cred = &raw mut (*t).cr;
    let mut setid: i32 = 0 as i32;
    if (*t).plan_setid != 0 {
        if (*t).plan_euid != NOCHG && (*t).plan_euid != (*c).euid {
            (*c).euid = (*t).plan_euid;
            setid = 1 as i32;
        }
        if (*t).plan_egid != NOCHG && (*t).plan_egid != (*c).egid {
            (*c).egid = (*t).plan_egid;
            setid = 1 as i32;
        }
    }
    if setid != 0 {
        (*c).cap_amb = 0 as u64;
    }
    if (*c).securebits & SECBIT_NOROOT as u32 == 0
        && ((*c).euid == 0 as u32 || (*c).ruid == 0 as u32)
    {
        (*c).cap_prm = (((*c).cap_bnd | (*c).cap_inh) as u64 & ENG_CAP_FULL) as u64;
        (*c).cap_eff = if (*c).euid == 0 as u32 {
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
    (*c).securebits = ((*c).securebits as u32 & !SECBIT_KEEP_CAPS) as u32;
    (*c).undumpable = setid;
    (*t).plan_setid = 0 as i32;
}
unsafe extern "C" fn read_guest_file(
    mut g: *mut eng_guest,
    mut path: *const ::core::ffi::c_char,
    mut out: *mut *mut ::core::ffi::c_char,
) -> i32 {
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    if eng_guest_to_host(
        g,
        path,
        &raw mut host as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return -1 as i32;
    }
    let mut f: *mut FILE = fopen(
        &raw mut host as *mut ::core::ffi::c_char,
        b"re\0".as_ptr() as *const ::core::ffi::c_char,
    );
    if f.is_null() {
        return -1 as i32;
    }
    let mut cap: usize = ((1 as i32) << 16 as i32) as usize;
    let mut len: usize = 0 as usize;
    let mut buf: *mut ::core::ffi::c_char = malloc(cap) as *mut ::core::ffi::c_char;
    let mut n: usize = 0;
    while !buf.is_null() && {
        n = fread(
            buf.offset(len as isize) as *mut ::core::ffi::c_void,
            1 as usize,
            cap.wrapping_sub(len).wrapping_sub(1 as usize),
            f,
        ) as usize;
        n > 0 as usize
    } {
        len = len.wrapping_add(n);
        if len.wrapping_add(1 as usize) >= cap {
            cap = cap.wrapping_mul(2 as usize);
            buf = realloc(buf as *mut ::core::ffi::c_void, cap) as *mut ::core::ffi::c_char;
        }
    }
    fclose(f);
    if buf.is_null() {
        return -1 as i32;
    }
    *buf.offset(len as isize) = 0 as ::core::ffi::c_char;
    *out = buf;
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ident_init(
    mut g: *mut eng_guest,
    mut t: *mut eng_task,
    mut uid: u32,
    mut gid: u32,
) {
    let mut c: *mut eng_cred = &raw mut (*t).cr;
    memset(
        c as *mut ::core::ffi::c_void,
        0 as i32,
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
    (*c).cap_bnd = ENG_CAP_FULL as u64;
    if uid == 0 as u32 {
        (*c).cap_eff = ENG_CAP_FULL as u64;
        (*c).cap_prm = (*c).cap_eff;
    }
    (*c).umask = 0o22 as u32;
    (*t).umask_old = -1 as i32;
    (*c).ngroups = 0 as u32;
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
    ) == 0 as i32
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
            let mut i: i32 = 0 as i32;
            while i < 4 as i32 && !p.is_null() {
                f[i as usize] = p;
                p = strchr(p, ':' as i32);
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
                    10 as i32,
                ) as u32
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
        ) != 0 as i32
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
        let mut i_0: i32 = 0 as i32;
        while i_0 < 4 as i32 && !p_0.is_null() {
            f_0[i_0 as usize] = p_0;
            p_0 = strchr(p_0, ':' as i32);
            if !p_0.is_null() {
                let c2rust_fresh2 = p_0;
                p_0 = p_0.offset(1);
                *c2rust_fresh2 = 0 as ::core::ffi::c_char;
            }
            i_0 += 1;
        }
        if !(f_0[2usize].is_null() || f_0[3usize].is_null()) {
            let mut id: u32 = strtoul(
                f_0[2usize],
                ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                10 as i32,
            ) as u32;
            let mut s2: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
            let mut m: *mut ::core::ffi::c_char = strtok_r(
                f_0[3usize],
                b",\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut s2,
            );
            while !m.is_null() {
                if !(strcmp(m, &raw mut name as *mut ::core::ffi::c_char) != 0 || id == gid) {
                    if ((*c).ngroups as usize)
                        < ::core::mem::size_of::<[u32; 32]>()
                            .wrapping_div(::core::mem::size_of::<u32>())
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
#[cfg(target_os = "android")]
#[repr(C)]
pub struct __sFILE {
    _opaque: [u8; 0],
}
