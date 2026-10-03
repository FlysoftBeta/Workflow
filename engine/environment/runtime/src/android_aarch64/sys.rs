//! Syscall dispatch, path rewriting, virtual metadata and Android SIGSYS handling.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
#[repr(C)]
pub struct eng_tracer {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    unsafe fn __errno() -> *mut ::core::ffi::c_int;
    unsafe fn open(
        __path: *const ::core::ffi::c_char,
        __flags: ::core::ffi::c_int,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn rename(
        __old_path: *const ::core::ffi::c_char,
        __new_path: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn snprintf(
        __buf: *mut ::core::ffi::c_char,
        __size: size_t,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn malloc(__byte_count: size_t) -> *mut ::core::ffi::c_void;
    unsafe fn free(__ptr: *mut ::core::ffi::c_void);
    unsafe fn memcmp(
        __lhs: *const ::core::ffi::c_void,
        __rhs: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memmove(
        __dst: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memset(
        __dst: *mut ::core::ffi::c_void,
        __ch: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn strrchr(
        __s: *const ::core::ffi::c_char,
        __ch: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    unsafe fn strcmp(
        __lhs: *const ::core::ffi::c_char,
        __rhs: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn strerror(__errno_value: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
    unsafe fn strnlen(__s: *const ::core::ffi::c_char, __n: size_t) -> size_t;
    unsafe fn strncmp(
        __lhs: *const ::core::ffi::c_char,
        __rhs: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn chmod(__path: *const ::core::ffi::c_char, __mode: mode_t) -> ::core::ffi::c_int;
    unsafe fn mkdir(__path: *const ::core::ffi::c_char, __mode: mode_t) -> ::core::ffi::c_int;
    unsafe fn lstat(__path: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn stat(__path: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn getpid() -> pid_t;
    unsafe fn unlink(__path: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    unsafe fn symlink(
        __old_path: *const ::core::ffi::c_char,
        __new_path: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn readlink(
        __path: *const ::core::ffi::c_char,
        __buf: *mut ::core::ffi::c_char,
        __buf_size: size_t,
    ) -> ssize_t;
    unsafe fn close(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn eng_arg(r: *const eng_regs, i: ::core::ffi::c_int) -> uint64_t;
    unsafe fn eng_set_arg(r: *mut eng_regs, i: ::core::ffi::c_int, v: uint64_t);
    unsafe fn eng_ret(r: *const eng_regs) -> uint64_t;
    unsafe fn eng_set_ret(r: *mut eng_regs, v: uint64_t);
    unsafe fn eng_syscall_set(
        tid: pid_t,
        r: *mut eng_regs,
        nr: ::core::ffi::c_long,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_tracer_guest(tr: *mut eng_tracer) -> *mut eng_guest;
    unsafe fn eng_task_void(
        t: *mut eng_task,
        r: *mut eng_regs,
        result: ::core::ffi::c_long,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_scratch_alloc(t: *mut eng_task, r: *const eng_regs, n: size_t) -> uint64_t;
    unsafe fn eng_scratch_put_str(
        t: *mut eng_task,
        r: *const eng_regs,
        s: *const ::core::ffi::c_char,
    ) -> uint64_t;
    unsafe fn eng_exec_prepare(
        t: *mut eng_task,
        dirfd: ::core::ffi::c_int,
        path: *const ::core::ffi::c_char,
        at_flags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_exec_discard(t: *mut eng_task);
    unsafe fn eng_guest_default_owner(
        g: *const eng_guest,
        host: *const ::core::ffi::c_char,
        uid: *mut ::core::ffi::c_uint,
        gid: *mut ::core::ffi::c_uint,
    );
    unsafe fn eng_guest_to_host(
        g: *const eng_guest,
        guest: *const ::core::ffi::c_char,
        host: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_host_to_guest(
        g: *const eng_guest,
        host: *const ::core::ffi::c_char,
        guest: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_guest_is_hidden(
        g: *const eng_guest,
        guest: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_ident_entry(t: *mut eng_task, r: *mut eng_regs) -> ::core::ffi::c_int;
    unsafe fn eng_ident_exit(t: *mut eng_task, r: *mut eng_regs) -> ::core::ffi::c_int;
    unsafe fn eng_log_enabled(lv: eng_log_level) -> ::core::ffi::c_int;
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
    unsafe fn eng_mem_read_cstr(
        tid: pid_t,
        addr: uintptr_t,
        buf: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ssize_t;
    unsafe fn eng_meta_read(
        g: *mut eng_guest,
        host: *const ::core::ffi::c_char,
        nofollow: ::core::ffi::c_int,
        st: *const stat,
        m: *mut eng_meta,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_meta_write(
        host: *const ::core::ffi::c_char,
        nofollow: ::core::ffi::c_int,
        m: *const eng_meta,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_meta_get(
        g: *mut eng_guest,
        host: *const ::core::ffi::c_char,
        nofollow: ::core::ffi::c_int,
        st: *const stat,
        m: *mut eng_meta,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_meta_in_store(
        g: *mut eng_guest,
        host: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_meta_lock(g: *mut eng_guest) -> ::core::ffi::c_int;
    unsafe fn eng_meta_unlock(token: ::core::ffi::c_int);
    unsafe fn eng_meta_is_placeholder(m: *const eng_meta, host_mode: mode_t) -> ::core::ffi::c_int;
    unsafe fn eng_meta_fifo_path(
        g: *mut eng_guest,
        placeholder: *const stat,
        out: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_meta_apply_stat(m: *const eng_meta, st: *mut stat);
    unsafe fn eng_meta_apply_statx(m: *const eng_meta, stx: *mut ::core::ffi::c_void);
    unsafe fn eng_meta_permission(
        t: *mut eng_task,
        m: *const eng_meta,
        mask: ::core::ffi::c_int,
        use_real: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_in_group(
        t: *const eng_task,
        gid: uint32_t,
        use_real: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_link_is_stub_text(
        text: *const ::core::ffi::c_char,
        id: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_link_object_path(
        g: *mut eng_guest,
        id: *const ::core::ffi::c_char,
        out: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_link_create(
        g: *mut eng_guest,
        old_entry: *const ::core::ffi::c_char,
        new_entry: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_link_drop(g: *mut eng_guest, id: *const ::core::ffi::c_char);
    unsafe fn eng_resolve(
        t: *mut eng_task,
        dirfd: ::core::ffi::c_int,
        path: *const ::core::ffi::c_char,
        flags: ::core::ffi::c_int,
        out: *mut eng_resolved,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_task_cwd(
        t: *mut eng_task,
        out: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_sysinv_name(nr: ::core::ffi::c_long) -> *const ::core::ffi::c_char;
    unsafe fn eng_sysinv_class(nr: ::core::ffi::c_long) -> eng_sc_class;
    unsafe fn eng_sysinv_class_name(c: eng_sc_class) -> *const ::core::ffi::c_char;
}
pub type size_t = usize;
pub type uint8_t = u8;
pub type uint16_t = u16;
pub type int32_t = i32;
pub type uint32_t = u32;
pub type int64_t = i64;
pub type uint64_t = u64;
pub type uintptr_t = usize;
pub type __u32 = ::core::ffi::c_uint;
pub type __kernel_long_t = ::core::ffi::c_long;
pub type __kernel_ulong_t = ::core::ffi::c_ulong;
pub type __kernel_ino_t = __kernel_ulong_t;
pub type __kernel_mode_t = ::core::ffi::c_uint;
pub type __kernel_pid_t = ::core::ffi::c_int;
pub type __kernel_uid32_t = ::core::ffi::c_uint;
pub type __kernel_gid32_t = ::core::ffi::c_uint;
pub type __kernel_time_t = __kernel_long_t;
pub type __kernel_clock_t = __kernel_long_t;
pub type __kernel_timer_t = ::core::ffi::c_int;
pub type __gid_t = __kernel_gid32_t;
pub type gid_t = __gid_t;
pub type __uid_t = __kernel_uid32_t;
pub type uid_t = __uid_t;
pub type __pid_t = __kernel_pid_t;
pub type pid_t = __pid_t;
pub type __mode_t = __kernel_mode_t;
pub type mode_t = __mode_t;
pub type __ino_t = __kernel_ino_t;
pub type ino_t = __ino_t;
pub type __nlink_t = uint32_t;
pub type nlink_t = __nlink_t;
pub type dev_t = uint64_t;
pub type __time_t = __kernel_time_t;
pub type time_t = __time_t;
pub type off_t = int64_t;
pub type ssize_t = isize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: time_t,
    pub tv_nsec: ::core::ffi::c_long,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: dev_t,
    pub st_ino: ino_t,
    pub st_mode: mode_t,
    pub st_nlink: nlink_t,
    pub st_uid: uid_t,
    pub st_gid: gid_t,
    pub st_rdev: dev_t,
    pub __pad1: ::core::ffi::c_ulong,
    pub st_size: off_t,
    pub st_blksize: ::core::ffi::c_int,
    pub __pad2: ::core::ffi::c_int,
    pub st_blocks: ::core::ffi::c_long,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __unused4: ::core::ffi::c_uint,
    pub __unused5: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union sigval {
    pub sival_int: ::core::ffi::c_int,
    pub sival_ptr: *mut ::core::ffi::c_void,
}
pub type sigval_t = sigval;
#[derive(Copy, Clone)]
#[repr(C)]
pub union __sifields {
    pub _kill: C2Rust_Unnamed_9,
    pub _timer: C2Rust_Unnamed_8,
    pub _rt: C2Rust_Unnamed_7,
    pub _sigchld: C2Rust_Unnamed_6,
    pub _sigfault: C2Rust_Unnamed_1,
    pub _sigpoll: C2Rust_Unnamed_0,
    pub _sigsys: C2Rust_Unnamed,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed {
    pub _call_addr: *mut ::core::ffi::c_void,
    pub _syscall: ::core::ffi::c_int,
    pub _arch: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_0 {
    pub _band: ::core::ffi::c_long,
    pub _fd: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_1 {
    pub _addr: *mut ::core::ffi::c_void,
    pub c2rust_unnamed: C2Rust_Unnamed_2,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed_2 {
    pub _trapno: ::core::ffi::c_int,
    pub _addr_lsb: ::core::ffi::c_short,
    pub _addr_bnd: C2Rust_Unnamed_5,
    pub _addr_pkey: C2Rust_Unnamed_4,
    pub _perf: C2Rust_Unnamed_3,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_3 {
    pub _data: ::core::ffi::c_ulong,
    pub _type: __u32,
    pub _flags: __u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_4 {
    pub _dummy_pkey: [::core::ffi::c_char; 8],
    pub _pkey: __u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_5 {
    pub _dummy_bnd: [::core::ffi::c_char; 8],
    pub _lower: *mut ::core::ffi::c_void,
    pub _upper: *mut ::core::ffi::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_6 {
    pub _pid: __kernel_pid_t,
    pub _uid: __kernel_uid32_t,
    pub _status: ::core::ffi::c_int,
    pub _utime: __kernel_clock_t,
    pub _stime: __kernel_clock_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_7 {
    pub _pid: __kernel_pid_t,
    pub _uid: __kernel_uid32_t,
    pub _sigval: sigval_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_8 {
    pub _tid: __kernel_timer_t,
    pub _overrun: ::core::ffi::c_int,
    pub _sigval: sigval_t,
    pub _sys_private: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_9 {
    pub _pid: __kernel_pid_t,
    pub _uid: __kernel_uid32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct siginfo {
    pub c2rust_unnamed: C2Rust_Unnamed_10,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed_10 {
    pub c2rust_unnamed: C2Rust_Unnamed_11,
    pub _si_pad: [::core::ffi::c_int; 32],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_11 {
    pub si_signo: ::core::ffi::c_int,
    pub si_errno: ::core::ffi::c_int,
    pub si_code: ::core::ffi::c_int,
    pub _sifields: __sifields,
}
pub type siginfo_t = siginfo;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct user_regs_struct {
    pub regs: [uint64_t; 31],
    pub sp: uint64_t,
    pub pc: uint64_t,
    pub pstate: uint64_t,
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
pub struct C2Rust_Unnamed_12(pub ::core::ffi::c_uint);
impl C2Rust_Unnamed_12 {
    pub const ENG_FIX_NONE: Self = Self(0);
    pub const ENG_FIX_STAT: Self = Self(1);
    pub const ENG_FIX_STATX: Self = Self(2);
    pub const ENG_FIX_CREATE_FD: Self = Self(3);
    pub const ENG_FIX_CREATE_PATH: Self = Self(4);
    pub const ENG_FIX_DROP_LINK: Self = Self(5);
    pub const ENG_FIX_RENAME_IN: Self = Self(6);
    pub const ENG_FIX_SOCKNAME: Self = Self(7);
    pub const ENG_FIX_GETDENTS: Self = Self(8);
    pub const ENG_FIX_LISTXATTR: Self = Self(9);
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_sc_class(pub ::core::ffi::c_uint);
impl eng_sc_class {
    pub const ENG_SC_UNKNOWN: Self = Self(0);
    pub const ENG_SC_PASS: Self = Self(1);
    pub const ENG_SC_PATH: Self = Self(2);
    pub const ENG_SC_FD: Self = Self(3);
    pub const ENG_SC_ID: Self = Self(4);
    pub const ENG_SC_META: Self = Self(5);
    pub const ENG_SC_EXEC: Self = Self(6);
    pub const ENG_SC_PROC: Self = Self(7);
    pub const ENG_SC_SOCK: Self = Self(8);
    pub const ENG_SC_ENOSYS: Self = Self(9);
    pub const ENG_SC_EPERM: Self = Self(10);
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_resolved {
    pub guest: [::core::ffi::c_char; 4096],
    pub host: [::core::ffi::c_char; 4096],
    pub verbatim: ::core::ffi::c_int,
    pub magic: ::core::ffi::c_int,
    pub magic_text: [::core::ffi::c_char; 4096],
    pub exists: ::core::ffi::c_int,
    pub stub: ::core::ffi::c_int,
    pub stub_id: [::core::ffi::c_char; 64],
    pub entry: [::core::ffi::c_char; 4096],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct wf_msghdr {
    pub name: uint64_t,
    pub namelen: uint32_t,
    pub pad: uint32_t,
    pub iov: uint64_t,
    pub iovlen: uint64_t,
    pub control: uint64_t,
    pub controllen: uint64_t,
    pub flags: int32_t,
    pub pad2: int32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct wf_sockaddr_un {
    pub family: uint16_t,
    pub path: [::core::ffi::c_char; 108],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_meta {
    pub uid: uint32_t,
    pub gid: uint32_t,
    pub mode: uint32_t,
    pub nlink: uint32_t,
    pub major: uint32_t,
    pub minor: uint32_t,
    pub present: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pathsys {
    pub nr: ::core::ffi::c_long,
    pub dfd: ::core::ffi::c_schar,
    pub path: ::core::ffi::c_schar,
    pub flagarg: ::core::ffi::c_schar,
    pub mode: ::core::ffi::c_uchar,
    pub nullok: ::core::ffi::c_uchar,
    pub dironly: ::core::ffi::c_uchar,
    pub entry: ::core::ffi::c_uchar,
}
#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct dirent64_hdr {
    pub ino: uint64_t,
    pub off: int64_t,
    pub reclen: uint16_t,
    pub r#type: uint8_t,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct C2Rust_Unnamed_13(pub ::core::ffi::c_uint);
impl C2Rust_Unnamed_13 {
    pub const FL_FOLLOW: Self = Self(0);
    pub const FL_NOFOLLOW: Self = Self(1);
    pub const FL_ATFLAG: Self = Self(2);
    pub const FL_INOTIFY: Self = Self(3);
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct C2Rust_Unnamed_14(pub ::core::ffi::c_uint);
impl C2Rust_Unnamed_14 {
    pub const XA_GET: Self = Self(0);
    pub const XA_SET: Self = Self(1);
    pub const XA_LIST: Self = Self(2);
    pub const XA_REMOVE: Self = Self(3);
}
pub const EPERM: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ENOENT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ENXIO: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const ENOMEM: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const EFAULT: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const EEXIST: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const EXDEV: ::core::ffi::c_int = 18 as ::core::ffi::c_int;
pub const ENODEV: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const EINVAL: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const ERANGE: ::core::ffi::c_int = 34 as ::core::ffi::c_int;
pub const ENAMETOOLONG: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const ENOSYS: ::core::ffi::c_int = 38 as ::core::ffi::c_int;
pub const ENODATA: ::core::ffi::c_int = 61 as ::core::ffi::c_int;
pub const EPROTONOSUPPORT: ::core::ffi::c_int = 93 as ::core::ffi::c_int;
pub const EOPNOTSUPP: ::core::ffi::c_int = 95 as ::core::ffi::c_int;
pub const EADDRINUSE: ::core::ffi::c_int = 98 as ::core::ffi::c_int;
pub const ENOTSUP: ::core::ffi::c_int = EOPNOTSUPP;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const O_NOFOLLOW: ::core::ffi::c_int = 0o100000 as ::core::ffi::c_int;
pub const O_ACCMODE: ::core::ffi::c_int = 0o3 as ::core::ffi::c_int;
pub const O_RDONLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const O_WRONLY: ::core::ffi::c_int = 0o1 as ::core::ffi::c_int;
pub const O_RDWR: ::core::ffi::c_int = 0o2 as ::core::ffi::c_int;
pub const O_CREAT: ::core::ffi::c_int = 0o100 as ::core::ffi::c_int;
pub const O_EXCL: ::core::ffi::c_int = 0o200 as ::core::ffi::c_int;
pub const O_TRUNC: ::core::ffi::c_int = 0o1000 as ::core::ffi::c_int;
pub const O_CLOEXEC: ::core::ffi::c_int = 0o2000000 as ::core::ffi::c_int;
pub const O_PATH: ::core::ffi::c_int = 0o10000000 as ::core::ffi::c_int;
pub const __O_TMPFILE: ::core::ffi::c_int = 0o20000000 as ::core::ffi::c_int;
pub const AT_FDCWD: ::core::ffi::c_int = -100 as ::core::ffi::c_int;
pub const AT_SYMLINK_NOFOLLOW: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const AT_SYMLINK_FOLLOW: ::core::ffi::c_int = 0x400 as ::core::ffi::c_int;
pub const AT_EMPTY_PATH: ::core::ffi::c_int = 0x1000 as ::core::ffi::c_int;
pub const AT_EACCESS: ::core::ffi::c_int = 0x200 as ::core::ffi::c_int;
pub const AT_REMOVEDIR: ::core::ffi::c_int = 0x200 as ::core::ffi::c_int;
pub const S_IFMT: ::core::ffi::c_int = 0o170000 as ::core::ffi::c_int;
pub const S_IFSOCK: ::core::ffi::c_int = 0o140000 as ::core::ffi::c_int;
pub const S_IFLNK: ::core::ffi::c_int = 0o120000 as ::core::ffi::c_int;
pub const S_IFREG: ::core::ffi::c_int = 0o100000 as ::core::ffi::c_int;
pub const S_IFBLK: ::core::ffi::c_int = 0o60000 as ::core::ffi::c_int;
pub const S_IFDIR: ::core::ffi::c_int = 0o40000 as ::core::ffi::c_int;
pub const S_IFCHR: ::core::ffi::c_int = 0o20000 as ::core::ffi::c_int;
pub const S_IFIFO: ::core::ffi::c_int = 0o10000 as ::core::ffi::c_int;
pub const S_ISUID: ::core::ffi::c_int = 0o4000 as ::core::ffi::c_int;
pub const S_ISGID: ::core::ffi::c_int = 0o2000 as ::core::ffi::c_int;
pub const S_ISVTX: ::core::ffi::c_int = 0o1000 as ::core::ffi::c_int;
pub const S_IXGRP: ::core::ffi::c_int = 0o10 as ::core::ffi::c_int;
pub const PATH_MAX: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
pub const RENAME_EXCHANGE: ::core::ffi::c_int =
    (1 as ::core::ffi::c_int) << 1 as ::core::ffi::c_int;
pub const IN_DONT_FOLLOW: ::core::ffi::c_int = 0x2000000 as ::core::ffi::c_int;
pub const __NR_setxattr: ::core::ffi::c_long = 5 as ::core::ffi::c_long;
pub const __NR_lsetxattr: ::core::ffi::c_long = 6 as ::core::ffi::c_long;
pub const __NR_fsetxattr: ::core::ffi::c_long = 7 as ::core::ffi::c_long;
pub const __NR_getxattr: ::core::ffi::c_long = 8 as ::core::ffi::c_long;
pub const __NR_lgetxattr: ::core::ffi::c_long = 9 as ::core::ffi::c_long;
pub const __NR_fgetxattr: ::core::ffi::c_long = 10 as ::core::ffi::c_long;
pub const __NR_listxattr: ::core::ffi::c_long = 11 as ::core::ffi::c_long;
pub const __NR_llistxattr: ::core::ffi::c_long = 12 as ::core::ffi::c_long;
pub const __NR_flistxattr: ::core::ffi::c_long = 13 as ::core::ffi::c_long;
pub const __NR_removexattr: ::core::ffi::c_long = 14 as ::core::ffi::c_long;
pub const __NR_lremovexattr: ::core::ffi::c_long = 15 as ::core::ffi::c_long;
pub const __NR_fremovexattr: ::core::ffi::c_long = 16 as ::core::ffi::c_long;
pub const __NR_getcwd: ::core::ffi::c_long = 17 as ::core::ffi::c_long;
pub const __NR_inotify_add_watch: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const __NR_mknodat: ::core::ffi::c_long = 33 as ::core::ffi::c_long;
pub const __NR_mkdirat: ::core::ffi::c_long = 34 as ::core::ffi::c_long;
pub const __NR_unlinkat: ::core::ffi::c_long = 35 as ::core::ffi::c_long;
pub const __NR_symlinkat: ::core::ffi::c_long = 36 as ::core::ffi::c_long;
pub const __NR_linkat: ::core::ffi::c_long = 37 as ::core::ffi::c_long;
pub const __NR_renameat: ::core::ffi::c_long = 38 as ::core::ffi::c_long;
pub const __NR_umount2: ::core::ffi::c_long = 39 as ::core::ffi::c_long;
pub const __NR_mount: ::core::ffi::c_long = 40 as ::core::ffi::c_long;
pub const __NR_pivot_root: ::core::ffi::c_long = 41 as ::core::ffi::c_long;
pub const __NR_statfs: ::core::ffi::c_int = 43 as ::core::ffi::c_int;
pub const __NR_truncate: ::core::ffi::c_int = 45 as ::core::ffi::c_int;
pub const __NR_faccessat: ::core::ffi::c_long = 48 as ::core::ffi::c_long;
pub const __NR_chdir: ::core::ffi::c_int = 49 as ::core::ffi::c_int;
pub const __NR_chroot: ::core::ffi::c_long = 51 as ::core::ffi::c_long;
pub const __NR_fchmod: ::core::ffi::c_long = 52 as ::core::ffi::c_long;
pub const __NR_fchmodat: ::core::ffi::c_long = 53 as ::core::ffi::c_long;
pub const __NR_fchownat: ::core::ffi::c_long = 54 as ::core::ffi::c_long;
pub const __NR_fchown: ::core::ffi::c_long = 55 as ::core::ffi::c_long;
pub const __NR_openat: ::core::ffi::c_long = 56 as ::core::ffi::c_long;
pub const __NR_getdents64: ::core::ffi::c_long = 61 as ::core::ffi::c_long;
pub const __NR_readlinkat: ::core::ffi::c_long = 78 as ::core::ffi::c_long;
pub const __NR_newfstatat: ::core::ffi::c_long = 79 as ::core::ffi::c_long;
pub const __NR_fstat: ::core::ffi::c_long = 80 as ::core::ffi::c_long;
pub const __NR_utimensat: ::core::ffi::c_int = 88 as ::core::ffi::c_int;
pub const __NR_acct: ::core::ffi::c_long = 89 as ::core::ffi::c_long;
pub const __NR_ptrace: ::core::ffi::c_long = 117 as ::core::ffi::c_long;
pub const __NR_socket: ::core::ffi::c_long = 198 as ::core::ffi::c_long;
pub const __NR_bind: ::core::ffi::c_long = 200 as ::core::ffi::c_long;
pub const __NR_accept: ::core::ffi::c_long = 202 as ::core::ffi::c_long;
pub const __NR_connect: ::core::ffi::c_long = 203 as ::core::ffi::c_long;
pub const __NR_getsockname: ::core::ffi::c_long = 204 as ::core::ffi::c_long;
pub const __NR_getpeername: ::core::ffi::c_long = 205 as ::core::ffi::c_long;
pub const __NR_sendto: ::core::ffi::c_long = 206 as ::core::ffi::c_long;
pub const __NR_sendmsg: ::core::ffi::c_long = 211 as ::core::ffi::c_long;
pub const __NR_execve: ::core::ffi::c_int = 221 as ::core::ffi::c_int;
pub const __NR_swapon: ::core::ffi::c_long = 224 as ::core::ffi::c_long;
pub const __NR_swapoff: ::core::ffi::c_long = 225 as ::core::ffi::c_long;
pub const __NR_accept4: ::core::ffi::c_long = 242 as ::core::ffi::c_long;
pub const __NR_fanotify_init: ::core::ffi::c_long = 262 as ::core::ffi::c_long;
pub const __NR_fanotify_mark: ::core::ffi::c_long = 263 as ::core::ffi::c_long;
pub const __NR_name_to_handle_at: ::core::ffi::c_long = 264 as ::core::ffi::c_long;
pub const __NR_open_by_handle_at: ::core::ffi::c_long = 265 as ::core::ffi::c_long;
pub const __NR_renameat2: ::core::ffi::c_long = 276 as ::core::ffi::c_long;
pub const __NR_execveat: ::core::ffi::c_long = 281 as ::core::ffi::c_long;
pub const __NR_statx: ::core::ffi::c_long = 291 as ::core::ffi::c_long;
pub const __NR_io_uring_setup: ::core::ffi::c_long = 425 as ::core::ffi::c_long;
pub const __NR_io_uring_enter: ::core::ffi::c_long = 426 as ::core::ffi::c_long;
pub const __NR_io_uring_register: ::core::ffi::c_long = 427 as ::core::ffi::c_long;
pub const __NR_open_tree: ::core::ffi::c_long = 428 as ::core::ffi::c_long;
pub const __NR_move_mount: ::core::ffi::c_long = 429 as ::core::ffi::c_long;
pub const __NR_fsopen: ::core::ffi::c_long = 430 as ::core::ffi::c_long;
pub const __NR_fsconfig: ::core::ffi::c_long = 431 as ::core::ffi::c_long;
pub const __NR_fsmount: ::core::ffi::c_long = 432 as ::core::ffi::c_long;
pub const __NR_fspick: ::core::ffi::c_long = 433 as ::core::ffi::c_long;
pub const __NR_clone3: ::core::ffi::c_long = 435 as ::core::ffi::c_long;
pub const __NR_openat2: ::core::ffi::c_long = 437 as ::core::ffi::c_long;
pub const __NR_faccessat2: ::core::ffi::c_long = 439 as ::core::ffi::c_long;
pub const __NR_fchmodat2: ::core::ffi::c_long = 452 as ::core::ffi::c_long;
pub const F_OK: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const X_OK: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const W_OK: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const R_OK: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ENG_CAP_CHOWN: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ENG_CAP_FOWNER: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const ENG_CAP_FSETID: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ENG_CAP_MKNOD: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn eng_capable(
    mut t: *const eng_task,
    mut cap: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*t).cr.cap_eff >> cap & 1 as uint64_t) as ::core::ffi::c_int;
}
pub const ENG_META_PREFIX: [::core::ffi::c_char; 15] =
    unsafe { ::core::mem::transmute::<[u8; 15], [::core::ffi::c_char; 15]>(*b"user.workflow.\0") };
pub const ENG_RES_FOLLOW: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const ENG_RES_MISSING_OK: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const ENG_RES_DIR_ONLY: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
unsafe extern "C" fn read_path(
    mut t: *mut eng_task,
    mut addr: uint64_t,
    mut buf: *mut ::core::ffi::c_char,
) -> ::core::ffi::c_long {
    if addr == 0 {
        return -(EFAULT as ::core::ffi::c_long);
    }
    let mut n: ssize_t = eng_mem_read_cstr((*t).tid, addr as uintptr_t, buf, PATH_MAX as size_t);
    if n < 0 as ssize_t {
        return -(EFAULT as ::core::ffi::c_long);
    }
    if n >= (PATH_MAX - 1 as ::core::ffi::c_int) as ssize_t {
        return -(ENAMETOOLONG as ::core::ffi::c_long);
    }
    return n as ::core::ffi::c_long;
}
unsafe extern "C" fn put_path(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut arg: ::core::ffi::c_int,
    mut host: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut a: uint64_t = eng_scratch_put_str(t, r, host);
    if a == 0 {
        return -ENOMEM;
    }
    eng_set_arg(r, arg, a);
    (*t).regs_modified = 1 as ::core::ffi::c_int;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn resolve_arg(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut flags: ::core::ffi::c_int,
    mut allow_empty: ::core::ffi::c_int,
    mut res: *mut eng_resolved,
    mut empty: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut path: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut n: ::core::ffi::c_long = read_path(
        t,
        eng_arg(r, path_arg),
        &raw mut path as *mut ::core::ffi::c_char,
    );
    if !empty.is_null() {
        *empty = 0 as ::core::ffi::c_int;
    }
    if n < 0 as ::core::ffi::c_long {
        return n as ::core::ffi::c_int;
    }
    if n == 0 as ::core::ffi::c_long && allow_empty != 0 {
        if !empty.is_null() {
            *empty = 1 as ::core::ffi::c_int;
        }
        return 0 as ::core::ffi::c_int;
    }
    let mut dfd: ::core::ffi::c_int = if dfd_arg >= 0 as ::core::ffi::c_int {
        eng_arg(r, dfd_arg) as ::core::ffi::c_int
    } else {
        AT_FDCWD
    };
    let mut rc: ::core::ffi::c_int = eng_resolve(
        t,
        dfd,
        &raw mut path as *mut ::core::ffi::c_char,
        flags,
        res,
    );
    if eng_log_enabled(eng_log_level::ENG_LOG_TRACE) != 0 {
        eng_logf!(
            eng_log_level::ENG_LOG_TRACE,
            b"tid=%d resolve \"%s\" -> %s (%d)\0".as_ptr() as *const ::core::ffi::c_char,
            (*t).tid,
            &raw mut path as *mut ::core::ffi::c_char,
            if rc != 0 {
                b"-\0".as_ptr() as *const ::core::ffi::c_char
            } else {
                &raw mut (*res).host as *mut ::core::ffi::c_char as *const ::core::ffi::c_char
            },
            rc,
        );
    }
    return rc;
}
unsafe extern "C" fn fd_proc_path(
    mut t: *mut eng_task,
    mut fd: ::core::ffi::c_int,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) {
    snprintf(
        out,
        cap,
        b"/proc/%d/fd/%d\0".as_ptr() as *const ::core::ffi::c_char,
        (*t).tid,
        fd,
    );
}
unsafe extern "C" fn host_stat(
    mut host: *const ::core::ffi::c_char,
    mut nofollow: ::core::ffi::c_int,
    mut st: *mut stat,
) -> ::core::ffi::c_int {
    return if if nofollow != 0 {
        lstat(host, st)
    } else {
        stat(host, st)
    } == 0 as ::core::ffi::c_int
    {
        0 as ::core::ffi::c_int
    } else {
        -*__errno()
    };
}
unsafe extern "C" fn parent_dir(
    mut host: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    snprintf(
        out,
        cap,
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        host,
    );
    let mut n: size_t = strlen(out);
    while n > 1 as size_t
        && *out.offset(n.wrapping_sub(1 as size_t) as isize) as ::core::ffi::c_int
            == '/' as ::core::ffi::c_int
    {
        n = n.wrapping_sub(1);
        *out.offset(n as isize) = 0 as ::core::ffi::c_char;
    }
    let mut sl: *mut ::core::ffi::c_char = strrchr(out, '/' as ::core::ffi::c_int);
    if sl.is_null() {
        return -ENOENT;
    }
    if sl == out {
        *sl.offset(1isize) = 0 as ::core::ffi::c_char;
    } else {
        *sl = 0 as ::core::ffi::c_char;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn may_create(
    mut t: *mut eng_task,
    mut entry_host: *const ::core::ffi::c_char,
    mut pm: *mut eng_meta,
) -> ::core::ffi::c_int {
    let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
    if parent_dir(
        entry_host,
        &raw mut dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return -ENOENT;
    }
    let mut st: stat = stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad1: 0,
        st_size: 0,
        st_blksize: 0,
        __pad2: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __unused4: 0,
        __unused5: 0,
    };
    if stat(&raw mut dir as *mut ::core::ffi::c_char, &raw mut st) != 0 as ::core::ffi::c_int {
        return -*__errno();
    }
    let mut m: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    eng_meta_get(
        eng_tracer_guest((*t).tr),
        &raw mut dir as *mut ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
        &raw mut st,
        &raw mut m,
    );
    if !pm.is_null() {
        *pm = m;
    }
    return eng_meta_permission(t, &raw mut m, W_OK | X_OK, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn may_delete(
    mut t: *mut eng_task,
    mut entry_host: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut dm: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    let mut rc: ::core::ffi::c_int = may_create(t, entry_host, &raw mut dm);
    if rc != 0 {
        return rc;
    }
    if dm.mode & S_ISVTX as uint32_t != 0 && eng_capable(t, ENG_CAP_FOWNER) == 0 {
        let mut st: stat = stat {
            st_dev: 0,
            st_ino: 0,
            st_mode: 0,
            st_nlink: 0,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad1: 0,
            st_size: 0,
            st_blksize: 0,
            __pad2: 0,
            st_blocks: 0,
            st_atim: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            st_mtim: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            st_ctim: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            __unused4: 0,
            __unused5: 0,
        };
        if lstat(entry_host, &raw mut st) != 0 as ::core::ffi::c_int {
            return 0 as ::core::ffi::c_int;
        }
        let mut vm: eng_meta = eng_meta {
            uid: 0,
            gid: 0,
            mode: 0,
            nlink: 0,
            major: 0,
            minor: 0,
            present: 0,
        };
        eng_meta_get(
            eng_tracer_guest((*t).tr),
            entry_host,
            1 as ::core::ffi::c_int,
            &raw mut st,
            &raw mut vm,
        );
        if (*t).cr.fsuid != vm.uid && (*t).cr.fsuid != dm.uid {
            return -EPERM;
        }
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn in_rootfs(
    mut t: *mut eng_task,
    mut host: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut g: *mut eng_guest = eng_tracer_guest((*t).tr);
    return ((*g).rootlen == 0 as size_t
        || strncmp(
            host,
            &raw mut (*g).root as *mut ::core::ffi::c_char,
            (*g).rootlen,
        ) == 0 as ::core::ffi::c_int
            && (*host.offset((*g).rootlen as isize) as ::core::ffi::c_int
                == '/' as ::core::ffi::c_int
                || *host.offset((*g).rootlen as isize) as ::core::ffi::c_int
                    == 0 as ::core::ffi::c_int)) as ::core::ffi::c_int;
}
unsafe extern "C" fn new_gid(mut t: *mut eng_task, mut parent: *const eng_meta) -> uint32_t {
    return if (*parent).mode & S_ISGID as uint32_t != 0 {
        (*parent).gid
    } else {
        (*t).cr.fsgid
    };
}
unsafe extern "C" fn virtual_device(mut m: *const eng_meta) -> *const ::core::ffi::c_char {
    if (*m).mode & S_IFMT as uint32_t != S_IFCHR as uint32_t {
        return ::core::ptr::null::<::core::ffi::c_char>();
    }
    if (*m).major == 1 as uint32_t {
        match (*m).minor {
            3 => return b"/dev/null\0".as_ptr() as *const ::core::ffi::c_char,
            5 => return b"/dev/zero\0".as_ptr() as *const ::core::ffi::c_char,
            7 => return b"/dev/full\0".as_ptr() as *const ::core::ffi::c_char,
            8 => return b"/dev/random\0".as_ptr() as *const ::core::ffi::c_char,
            9 => return b"/dev/urandom\0".as_ptr() as *const ::core::ffi::c_char,
            _ => {}
        }
    }
    if (*m).major == 5 as uint32_t && (*m).minor == 0 as uint32_t {
        return b"/dev/tty\0".as_ptr() as *const ::core::ffi::c_char;
    }
    if (*m).major == 5 as uint32_t && (*m).minor == 2 as uint32_t {
        return b"/dev/ptmx\0".as_ptr() as *const ::core::ffi::c_char;
    }
    return ::core::ptr::null::<::core::ffi::c_char>();
}
unsafe extern "C" fn h_open(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut flags_arg: ::core::ffi::c_int,
    mut mode_arg: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut flags: ::core::ffi::c_long = if flags_arg >= 0 as ::core::ffi::c_int {
        eng_arg(r, flags_arg) as ::core::ffi::c_long
    } else {
        (O_CREAT | O_WRONLY | O_TRUNC) as ::core::ffi::c_long
    };
    let mut tmpfile: ::core::ffi::c_int = (flags & __O_TMPFILE as ::core::ffi::c_long
        == __O_TMPFILE as ::core::ffi::c_long)
        as ::core::ffi::c_int;
    let mut creat: ::core::ffi::c_int =
        (flags & O_CREAT as ::core::ffi::c_long != 0 && tmpfile == 0) as ::core::ffi::c_int;
    let mut follow: ::core::ffi::c_int = !(flags & O_NOFOLLOW as ::core::ffi::c_long != 0
        || creat != 0 && flags & O_EXCL as ::core::ffi::c_long != 0)
        as ::core::ffi::c_int;
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        if follow != 0 {
            ENG_RES_FOLLOW
        } else {
            0 as ::core::ffi::c_int
        } | if creat != 0 {
            ENG_RES_MISSING_OK
        } else {
            0 as ::core::ffi::c_int
        },
        0 as ::core::ffi::c_int,
        &raw mut res,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    let mut host: *const ::core::ffi::c_char = &raw mut res.host as *mut ::core::ffi::c_char;
    if res.verbatim == 0 && res.magic == 0 {
        if res.exists == 0 || tmpfile != 0 {
            let mut pm: eng_meta = eng_meta {
                uid: 0,
                gid: 0,
                mode: 0,
                nlink: 0,
                major: 0,
                minor: 0,
                present: 0,
            };
            let mut probe: [::core::ffi::c_char; 4100] = [0; 4100];
            snprintf(
                &raw mut probe as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4100]>(),
                b"%s/x\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut res.host as *mut ::core::ffi::c_char,
            );
            rc = may_create(
                t,
                if tmpfile != 0 {
                    &raw mut probe as *mut ::core::ffi::c_char
                } else {
                    &raw mut res.entry as *mut ::core::ffi::c_char
                },
                &raw mut pm,
            );
            if rc != 0 {
                return eng_task_void(t, r, rc as ::core::ffi::c_long);
            }
            let mut mode: uint32_t =
                eng_arg(r, mode_arg) as ::core::ffi::c_long as uint32_t & 0o7777 as uint32_t;
            (*t).fix_mode = S_IFREG as uint32_t | mode & !(*t).cr.umask & 0o7777 as uint32_t;
            (*t).fix_aux = new_gid(t, &raw mut pm) as ::core::ffi::c_long;
            if (*t).fix_mode & S_ISGID as uint32_t != 0
                && eng_in_group(t, (*t).fix_aux as uint32_t, 0 as ::core::ffi::c_int) == 0
                && eng_capable(t, ENG_CAP_FSETID) == 0
            {
                (*t).fix_mode &= !(S_ISGID as uint32_t);
            }
            if eng_meta_in_store(
                eng_tracer_guest((*t).tr),
                if tmpfile != 0 {
                    &raw mut res.host as *mut ::core::ffi::c_char
                } else {
                    &raw mut res.entry as *mut ::core::ffi::c_char
                },
            ) != 0
            {
                eng_set_arg(
                    r,
                    mode_arg,
                    ((mode | 0o600 as uint32_t) & 0o777 as uint32_t) as uint64_t,
                );
                (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_CREATE_FD.0 as ::core::ffi::c_int;
            } else {
                eng_set_arg(r, mode_arg, (mode & 0o777 as uint32_t) as uint64_t);
            }
        } else {
            let mut st: stat = stat {
                st_dev: 0,
                st_ino: 0,
                st_mode: 0,
                st_nlink: 0,
                st_uid: 0,
                st_gid: 0,
                st_rdev: 0,
                __pad1: 0,
                st_size: 0,
                st_blksize: 0,
                __pad2: 0,
                st_blocks: 0,
                st_atim: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                st_mtim: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                st_ctim: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                __unused4: 0,
                __unused5: 0,
            };
            if stat(&raw mut res.host as *mut ::core::ffi::c_char, &raw mut st)
                == 0 as ::core::ffi::c_int
            {
                let mut m: eng_meta = eng_meta {
                    uid: 0,
                    gid: 0,
                    mode: 0,
                    nlink: 0,
                    major: 0,
                    minor: 0,
                    present: 0,
                };
                eng_meta_get(
                    eng_tracer_guest((*t).tr),
                    &raw mut res.host as *mut ::core::ffi::c_char,
                    0 as ::core::ffi::c_int,
                    &raw mut st,
                    &raw mut m,
                );
                let mut mask: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                if flags & O_PATH as ::core::ffi::c_long == 0 {
                    let mut acc: ::core::ffi::c_int =
                        (flags & O_ACCMODE as ::core::ffi::c_long) as ::core::ffi::c_int;
                    if acc == O_RDONLY || acc == O_RDWR {
                        mask |= R_OK;
                    }
                    if acc == O_WRONLY
                        || acc == O_RDWR
                        || flags & O_TRUNC as ::core::ffi::c_long != 0
                    {
                        mask |= W_OK;
                    }
                }
                if st.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t && mask & W_OK != 0 {
                    mask &= !W_OK;
                }
                if mask != 0 && {
                    rc = eng_meta_permission(t, &raw mut m, mask, 0 as ::core::ffi::c_int);
                    rc != 0
                } {
                    return eng_task_void(t, r, rc as ::core::ffi::c_long);
                }
                let mut dev: *const ::core::ffi::c_char =
                    if flags & O_PATH as ::core::ffi::c_long != 0 {
                        ::core::ptr::null::<::core::ffi::c_char>()
                    } else {
                        virtual_device(&raw mut m)
                    };
                if !dev.is_null() && st.st_mode & S_IFMT as mode_t == S_IFREG as mode_t {
                    host = dev;
                }
                if flags & O_PATH as ::core::ffi::c_long == 0 {
                    if st.st_mode & S_IFMT as mode_t == S_IFREG as mode_t
                        && m.mode & S_IFMT as uint32_t == S_IFIFO as uint32_t
                    {
                        rc = eng_meta_fifo_path(
                            eng_tracer_guest((*t).tr),
                            &raw mut st,
                            &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                        );
                        if rc != 0 {
                            return eng_task_void(t, r, rc as ::core::ffi::c_long);
                        }
                        host = &raw mut (*t).fix_path as *mut ::core::ffi::c_char;
                    } else if eng_meta_is_placeholder(&raw mut m, st.st_mode) != 0 && dev.is_null()
                    {
                        return eng_task_void(
                            t,
                            r,
                            (if m.mode & S_IFMT as uint32_t == S_IFSOCK as uint32_t {
                                -ENXIO
                            } else {
                                -ENODEV
                            }) as ::core::ffi::c_long,
                        );
                    }
                }
            }
        }
    }
    rc = put_path(t, r, path_arg, host);
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_stat(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut flag_arg: ::core::ffi::c_int,
    mut buf_arg: ::core::ffi::c_int,
    mut nofollow_always: ::core::ffi::c_int,
    mut statx: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut flags: ::core::ffi::c_long = if flag_arg >= 0 as ::core::ffi::c_int {
        eng_arg(r, flag_arg) as ::core::ffi::c_long
    } else {
        0 as ::core::ffi::c_long
    };
    let mut nofollow: ::core::ffi::c_int = (nofollow_always != 0
        || flags & AT_SYMLINK_NOFOLLOW as ::core::ffi::c_long != 0)
        as ::core::ffi::c_int;
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut empty: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        if nofollow != 0 {
            0 as ::core::ffi::c_int
        } else {
            ENG_RES_FOLLOW
        },
        (flags & AT_EMPTY_PATH as ::core::ffi::c_long != 0 as ::core::ffi::c_long)
            as ::core::ffi::c_int,
        &raw mut res,
        &raw mut empty,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    let mut changed: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if empty != 0 {
        fd_proc_path(
            t,
            eng_arg(r, dfd_arg) as ::core::ffi::c_long as ::core::ffi::c_int,
            &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        );
        (*t).fix_nofollow = 0 as ::core::ffi::c_int;
    } else {
        rc = put_path(
            t,
            r,
            path_arg,
            &raw mut res.host as *mut ::core::ffi::c_char,
        );
        if rc != 0 {
            return eng_task_void(t, r, rc as ::core::ffi::c_long);
        }
        changed = 1 as ::core::ffi::c_int;
        if res.magic != 0 || res.verbatim != 0 {
            return changed;
        }
        snprintf(
            &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut res.host as *mut ::core::ffi::c_char,
        );
        (*t).fix_nofollow = (nofollow != 0 && res.stub == 0) as ::core::ffi::c_int;
    }
    (*t).fix_addr = eng_arg(r, buf_arg);
    (*t).fixup = if statx != 0 {
        C2Rust_Unnamed_12::ENG_FIX_STATX.0 as ::core::ffi::c_int
    } else {
        C2Rust_Unnamed_12::ENG_FIX_STAT.0 as ::core::ffi::c_int
    };
    return changed;
}
unsafe extern "C" fn h_fstat(mut t: *mut eng_task, mut r: *mut eng_regs) -> ::core::ffi::c_int {
    fd_proc_path(
        t,
        eng_arg(r, 0 as ::core::ffi::c_int) as ::core::ffi::c_long as ::core::ffi::c_int,
        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    (*t).fix_nofollow = 0 as ::core::ffi::c_int;
    (*t).fix_addr = eng_arg(r, 1 as ::core::ffi::c_int);
    (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_STAT.0 as ::core::ffi::c_int;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn x_stat(mut t: *mut eng_task) {
    let mut st: stat = stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad1: 0,
        st_size: 0,
        st_blksize: 0,
        __pad2: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __unused4: 0,
        __unused5: 0,
    };
    if eng_mem_read(
        (*t).tid,
        (*t).fix_addr as uintptr_t,
        &raw mut st as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<stat>(),
    ) != ::core::mem::size_of::<stat>() as ssize_t
    {
        return;
    }
    let mut m: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    if eng_meta_get(
        eng_tracer_guest((*t).tr),
        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
        (*t).fix_nofollow,
        &raw mut st,
        &raw mut m,
    ) != 0
    {
        return;
    }
    eng_meta_apply_stat(&raw mut m, &raw mut st);
    eng_mem_write(
        (*t).tid,
        (*t).fix_addr as uintptr_t,
        &raw mut st as *const ::core::ffi::c_void,
        ::core::mem::size_of::<stat>(),
    );
}
unsafe extern "C" fn x_statx(mut t: *mut eng_task) {
    let mut b: [::core::ffi::c_uchar; 256] = [0; 256];
    if eng_mem_read(
        (*t).tid,
        (*t).fix_addr as uintptr_t,
        &raw mut b as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[::core::ffi::c_uchar; 256]>(),
    ) != ::core::mem::size_of::<[::core::ffi::c_uchar; 256]>() as ssize_t
    {
        return;
    }
    let mut st: stat = stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad1: 0,
        st_size: 0,
        st_blksize: 0,
        __pad2: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __unused4: 0,
        __unused5: 0,
    };
    memset(
        &raw mut st as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<stat>(),
    );
    let mut mode: uint16_t = 0;
    let mut ino: uint64_t = 0;
    let mut dmaj: uint32_t = 0;
    let mut dmin: uint32_t = 0;
    let mut cs: int64_t = 0;
    let mut cn: uint32_t = 0;
    memcpy(
        &raw mut mode as *mut ::core::ffi::c_void,
        (&raw mut b as *mut ::core::ffi::c_uchar).offset(28 as ::core::ffi::c_int as isize)
            as *const ::core::ffi::c_void,
        2 as size_t,
    );
    memcpy(
        &raw mut ino as *mut ::core::ffi::c_void,
        (&raw mut b as *mut ::core::ffi::c_uchar).offset(32 as ::core::ffi::c_int as isize)
            as *const ::core::ffi::c_void,
        8 as size_t,
    );
    memcpy(
        &raw mut dmaj as *mut ::core::ffi::c_void,
        (&raw mut b as *mut ::core::ffi::c_uchar).offset(136 as ::core::ffi::c_int as isize)
            as *const ::core::ffi::c_void,
        4 as size_t,
    );
    memcpy(
        &raw mut dmin as *mut ::core::ffi::c_void,
        (&raw mut b as *mut ::core::ffi::c_uchar).offset(140 as ::core::ffi::c_int as isize)
            as *const ::core::ffi::c_void,
        4 as size_t,
    );
    memcpy(
        &raw mut cs as *mut ::core::ffi::c_void,
        (&raw mut b as *mut ::core::ffi::c_uchar).offset(96 as ::core::ffi::c_int as isize)
            as *const ::core::ffi::c_void,
        8 as size_t,
    );
    memcpy(
        &raw mut cn as *mut ::core::ffi::c_void,
        (&raw mut b as *mut ::core::ffi::c_uchar).offset(104 as ::core::ffi::c_int as isize)
            as *const ::core::ffi::c_void,
        4 as size_t,
    );
    st.st_mode = mode as mode_t;
    st.st_ino = ino as ino_t;
    st.st_dev = ((dmaj as ::core::ffi::c_ulonglong & 0xfffff000 as ::core::ffi::c_ulonglong)
        << 32 as ::core::ffi::c_int
        | (dmaj as ::core::ffi::c_ulonglong & 0xfff as ::core::ffi::c_ulonglong)
            << 8 as ::core::ffi::c_int
        | (dmin as ::core::ffi::c_ulonglong & 0xffffff00 as ::core::ffi::c_ulonglong)
            << 12 as ::core::ffi::c_int
        | dmin as ::core::ffi::c_ulonglong & 0xff as ::core::ffi::c_ulonglong)
        as dev_t;
    st.st_ctim.tv_sec = cs as time_t;
    st.st_ctim.tv_nsec = cn as ::core::ffi::c_long;
    let mut m: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    if eng_meta_get(
        eng_tracer_guest((*t).tr),
        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
        (*t).fix_nofollow,
        &raw mut st,
        &raw mut m,
    ) != 0
    {
        return;
    }
    eng_meta_apply_statx(
        &raw mut m,
        &raw mut b as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
    );
    eng_mem_write(
        (*t).tid,
        (*t).fix_addr as uintptr_t,
        &raw mut b as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[::core::ffi::c_uchar; 256]>(),
    );
}
unsafe extern "C" fn h_access(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut mode_arg: ::core::ffi::c_int,
    mut flag_arg: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut flags: ::core::ffi::c_long = if flag_arg >= 0 as ::core::ffi::c_int {
        eng_arg(r, flag_arg) as ::core::ffi::c_long
    } else {
        0 as ::core::ffi::c_long
    };
    let mut mode: ::core::ffi::c_long = eng_arg(r, mode_arg) as ::core::ffi::c_long;
    if mode & !(7 as ::core::ffi::c_int as ::core::ffi::c_long) != 0 {
        return eng_task_void(t, r, -(EINVAL as ::core::ffi::c_long));
    }
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut empty: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        if flags & AT_SYMLINK_NOFOLLOW as ::core::ffi::c_long != 0 {
            0 as ::core::ffi::c_int
        } else {
            ENG_RES_FOLLOW
        },
        (flags & AT_EMPTY_PATH as ::core::ffi::c_long != 0 as ::core::ffi::c_long)
            as ::core::ffi::c_int,
        &raw mut res,
        &raw mut empty,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    if empty != 0 {
        fd_proc_path(
            t,
            eng_arg(r, dfd_arg) as ::core::ffi::c_long as ::core::ffi::c_int,
            &raw mut host as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        );
    } else if res.magic != 0 || res.verbatim != 0 {
        rc = put_path(
            t,
            r,
            path_arg,
            &raw mut res.host as *mut ::core::ffi::c_char,
        );
        if rc != 0 {
            return eng_task_void(t, r, rc as ::core::ffi::c_long);
        }
        return 1 as ::core::ffi::c_int;
    } else {
        snprintf(
            &raw mut host as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut res.host as *mut ::core::ffi::c_char,
        );
    }
    let mut st: stat = stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad1: 0,
        st_size: 0,
        st_blksize: 0,
        __pad2: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __unused4: 0,
        __unused5: 0,
    };
    let mut nf: ::core::ffi::c_int = (flags & AT_SYMLINK_NOFOLLOW as ::core::ffi::c_long != 0
        && res.stub == 0) as ::core::ffi::c_int;
    rc = host_stat(&raw mut host as *mut ::core::ffi::c_char, nf, &raw mut st);
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if mode == F_OK as ::core::ffi::c_long {
        return eng_task_void(t, r, 0 as ::core::ffi::c_long);
    }
    let mut m: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    eng_meta_get(
        eng_tracer_guest((*t).tr),
        &raw mut host as *mut ::core::ffi::c_char,
        nf,
        &raw mut st,
        &raw mut m,
    );
    return eng_task_void(
        t,
        r,
        eng_meta_permission(
            t,
            &raw mut m,
            mode as ::core::ffi::c_int,
            (flags & AT_EACCESS as ::core::ffi::c_long == 0) as ::core::ffi::c_int,
        ) as ::core::ffi::c_long,
    );
}
unsafe extern "C" fn h_mkdir(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut mode_arg: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        ENG_RES_MISSING_OK,
        0 as ::core::ffi::c_int,
        &raw mut res,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if res.exists != 0 {
        return eng_task_void(t, r, -(EEXIST as ::core::ffi::c_long));
    }
    let mut pm: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    rc = may_create(
        t,
        &raw mut res.entry as *mut ::core::ffi::c_char,
        &raw mut pm,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    let mut mode: uint32_t =
        eng_arg(r, mode_arg) as ::core::ffi::c_long as uint32_t & 0o7777 as uint32_t;
    (*t).fix_mode = S_IFDIR as uint32_t
        | mode & !(*t).cr.umask & 0o1777 as uint32_t
        | pm.mode & S_ISGID as uint32_t;
    (*t).fix_aux = new_gid(t, &raw mut pm) as ::core::ffi::c_long;
    snprintf(
        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    let mut store: ::core::ffi::c_int = eng_meta_in_store(
        eng_tracer_guest((*t).tr),
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    eng_set_arg(
        r,
        mode_arg,
        (if store != 0 {
            (mode | 0o700 as uint32_t) & 0o777 as uint32_t
        } else {
            mode & 0o777 as uint32_t
        }) as uint64_t,
    );
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if store != 0 {
        (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_CREATE_PATH.0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_mknod(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut mode_arg: ::core::ffi::c_int,
    mut dev_arg: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        ENG_RES_MISSING_OK,
        0 as ::core::ffi::c_int,
        &raw mut res,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if res.exists != 0 {
        return eng_task_void(t, r, -(EEXIST as ::core::ffi::c_long));
    }
    let mut pm: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    rc = may_create(
        t,
        &raw mut res.entry as *mut ::core::ffi::c_char,
        &raw mut pm,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    let mut mode: uint32_t = eng_arg(r, mode_arg) as ::core::ffi::c_long as uint32_t;
    let mut r#type: uint32_t = mode & S_IFMT as uint32_t;
    if r#type == 0 {
        r#type = S_IFREG as uint32_t;
    }
    let mut perm: uint32_t = mode & 0o7777 as uint32_t & !(*t).cr.umask;
    let mut store: ::core::ffi::c_int = eng_meta_in_store(
        eng_tracer_guest((*t).tr),
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if r#type == S_IFCHR as uint32_t
        || r#type == S_IFBLK as uint32_t
        || store != 0 && (r#type == S_IFIFO as uint32_t || r#type == S_IFSOCK as uint32_t)
    {
        if (r#type == S_IFCHR as uint32_t || r#type == S_IFBLK as uint32_t)
            && (eng_capable(t, ENG_CAP_MKNOD) == 0 || store == 0)
        {
            return eng_task_void(t, r, -(EPERM as ::core::ffi::c_long));
        }
        let mut fd: ::core::ffi::c_int = open(
            &raw mut res.entry as *mut ::core::ffi::c_char,
            O_CREAT | O_EXCL | O_WRONLY | O_CLOEXEC,
            0o600 as ::core::ffi::c_int,
        );
        if fd < 0 as ::core::ffi::c_int {
            return eng_task_void(t, r, -*__errno() as ::core::ffi::c_long);
        }
        close(fd);
        let mut dev: dev_t = if r#type == S_IFCHR as uint32_t || r#type == S_IFBLK as uint32_t {
            eng_arg(r, dev_arg)
        } else {
            0 as dev_t
        };
        let mut m: eng_meta = eng_meta {
            uid: (*t).cr.fsuid,
            gid: new_gid(t, &raw mut pm),
            mode: r#type | perm,
            nlink: 0,
            major: (dev as ::core::ffi::c_ulonglong >> 32 as ::core::ffi::c_int
                & 0xfffff000 as ::core::ffi::c_ulonglong
                | (dev >> 8 as ::core::ffi::c_int & 0xfff as dev_t) as ::core::ffi::c_ulonglong)
                as uint32_t,
            minor: (dev >> 12 as ::core::ffi::c_int & 0xffffff00 as dev_t | dev & 0xff as dev_t)
                as uint32_t,
            present: 1 as ::core::ffi::c_int,
        };
        rc = eng_meta_write(
            &raw mut res.entry as *mut ::core::ffi::c_char,
            0 as ::core::ffi::c_int,
            &raw mut m,
        );
        if rc != 0 {
            unlink(&raw mut res.entry as *mut ::core::ffi::c_char);
        }
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if r#type != S_IFREG as uint32_t
        && r#type != S_IFIFO as uint32_t
        && r#type != S_IFSOCK as uint32_t
    {
        return eng_task_void(t, r, -(EINVAL as ::core::ffi::c_long));
    }
    (*t).fix_mode = r#type | perm;
    (*t).fix_aux = new_gid(t, &raw mut pm) as ::core::ffi::c_long;
    snprintf(
        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    eng_set_arg(
        r,
        mode_arg,
        (r#type
            | if store != 0 {
                (perm | 0o600 as uint32_t) & 0o777 as uint32_t
            } else {
                eng_arg(r, mode_arg) as ::core::ffi::c_long as uint32_t & 0o777 as uint32_t
            }) as uint64_t,
    );
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if store != 0 {
        (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_CREATE_PATH.0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_symlink(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        ENG_RES_MISSING_OK,
        0 as ::core::ffi::c_int,
        &raw mut res,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if res.exists != 0 {
        return eng_task_void(t, r, -(EEXIST as ::core::ffi::c_long));
    }
    rc = may_create(
        t,
        &raw mut res.entry as *mut ::core::ffi::c_char,
        ::core::ptr::null_mut::<eng_meta>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_unlink(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut rmdir: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        &raw mut res,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if res.verbatim == 0 && res.magic == 0 && {
        rc = may_delete(t, &raw mut res.entry as *mut ::core::ffi::c_char);
        rc != 0
    } {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if res.stub != 0 && rmdir == 0 {
        snprintf(
            &raw mut (*t).fix_id as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut res.stub_id as *mut ::core::ffi::c_char,
        );
        (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_DROP_LINK.0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_rename(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut d1: ::core::ffi::c_int,
    mut p1: ::core::ffi::c_int,
    mut d2: ::core::ffi::c_int,
    mut p2: ::core::ffi::c_int,
    mut flag_arg: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut flags: ::core::ffi::c_long = if flag_arg >= 0 as ::core::ffi::c_int {
        eng_arg(r, flag_arg) as ::core::ffi::c_long
    } else {
        0 as ::core::ffi::c_long
    };
    let mut a: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut b: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        d1,
        p1,
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        &raw mut a,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    rc = resolve_arg(
        t,
        r,
        d2,
        p2,
        ENG_RES_MISSING_OK,
        0 as ::core::ffi::c_int,
        &raw mut b,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    rc = may_delete(t, &raw mut a.entry as *mut ::core::ffi::c_char);
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if if b.exists != 0 {
        rc = may_delete(t, &raw mut b.entry as *mut ::core::ffi::c_char);
        rc
    } else {
        rc = may_create(
            t,
            &raw mut b.entry as *mut ::core::ffi::c_char,
            ::core::ptr::null_mut::<eng_meta>(),
        );
        rc
    } != 0
    {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    let mut sa: ::core::ffi::c_int = eng_meta_in_store(
        eng_tracer_guest((*t).tr),
        &raw mut a.entry as *mut ::core::ffi::c_char,
    );
    let mut sb: ::core::ffi::c_int = eng_meta_in_store(
        eng_tracer_guest((*t).tr),
        &raw mut b.entry as *mut ::core::ffi::c_char,
    );
    if a.stub != 0 && sb == 0
        || b.stub != 0 && flags & RENAME_EXCHANGE as ::core::ffi::c_long != 0 && sa == 0
    {
        return eng_task_void(t, r, -(EXDEV as ::core::ffi::c_long));
    }
    rc = put_path(t, r, p1, &raw mut a.entry as *mut ::core::ffi::c_char);
    if rc != 0 || {
        rc = put_path(t, r, p2, &raw mut b.entry as *mut ::core::ffi::c_char);
        rc != 0
    } {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if b.exists != 0
        && b.stub != 0
        && flags & RENAME_EXCHANGE as ::core::ffi::c_long == 0
        && !(a.stub != 0
            && strcmp(
                &raw mut a.stub_id as *mut ::core::ffi::c_char,
                &raw mut b.stub_id as *mut ::core::ffi::c_char,
            ) == 0)
    {
        snprintf(
            &raw mut (*t).fix_id as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut b.stub_id as *mut ::core::ffi::c_char,
        );
        (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_DROP_LINK.0 as ::core::ffi::c_int;
    } else if sa == 0 && sb != 0 && flags & RENAME_EXCHANGE as ::core::ffi::c_long == 0 {
        let mut u: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
        let mut gg: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
        eng_guest_default_owner(
            eng_tracer_guest((*t).tr),
            &raw mut a.entry as *mut ::core::ffi::c_char,
            &raw mut u,
            &raw mut gg,
        );
        (*t).fix_mode = u as uint32_t;
        (*t).fix_aux = gg as ::core::ffi::c_long;
        snprintf(
            &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut b.entry as *mut ::core::ffi::c_char,
        );
        (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_RENAME_IN.0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_link(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut d1: ::core::ffi::c_int,
    mut p1: ::core::ffi::c_int,
    mut d2: ::core::ffi::c_int,
    mut p2: ::core::ffi::c_int,
    mut flag_arg: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut flags: ::core::ffi::c_long = if flag_arg >= 0 as ::core::ffi::c_int {
        eng_arg(r, flag_arg) as ::core::ffi::c_long
    } else {
        0 as ::core::ffi::c_long
    };
    if flags & AT_EMPTY_PATH as ::core::ffi::c_long != 0 {
        return eng_task_void(t, r, -(EPERM as ::core::ffi::c_long));
    }
    let mut a: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut b: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        d1,
        p1,
        if flags & AT_SYMLINK_FOLLOW as ::core::ffi::c_long != 0 {
            ENG_RES_FOLLOW
        } else {
            0 as ::core::ffi::c_int
        },
        0 as ::core::ffi::c_int,
        &raw mut a,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    rc = resolve_arg(
        t,
        r,
        d2,
        p2,
        ENG_RES_MISSING_OK,
        0 as ::core::ffi::c_int,
        &raw mut b,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if b.exists != 0 {
        return eng_task_void(t, r, -(EEXIST as ::core::ffi::c_long));
    }
    let mut ra: ::core::ffi::c_int = in_rootfs(t, &raw mut a.entry as *mut ::core::ffi::c_char);
    let mut rb: ::core::ffi::c_int = in_rootfs(t, &raw mut b.entry as *mut ::core::ffi::c_char);
    if ra == 0 || rb == 0 {
        return eng_task_void(
            t,
            r,
            (if ra != rb { -EXDEV } else { -EPERM }) as ::core::ffi::c_long,
        );
    }
    rc = may_create(
        t,
        &raw mut b.entry as *mut ::core::ffi::c_char,
        ::core::ptr::null_mut::<eng_meta>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    let mut st: stat = stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad1: 0,
        st_size: 0,
        st_blksize: 0,
        __pad2: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __unused4: 0,
        __unused5: 0,
    };
    if lstat(&raw mut a.host as *mut ::core::ffi::c_char, &raw mut st) == 0 as ::core::ffi::c_int
        && st.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t
    {
        return eng_task_void(t, r, -(EPERM as ::core::ffi::c_long));
    }
    return eng_task_void(
        t,
        r,
        eng_link_create(
            eng_tracer_guest((*t).tr),
            &raw mut a.entry as *mut ::core::ffi::c_char,
            &raw mut b.entry as *mut ::core::ffi::c_char,
        ) as ::core::ffi::c_long,
    );
}
unsafe extern "C" fn meta_target(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut fd_arg: ::core::ffi::c_int,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut nofollow: ::core::ffi::c_int,
    mut allow_empty: ::core::ffi::c_int,
    mut host: *mut ::core::ffi::c_char,
    mut st: *mut stat,
    mut nf: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut empty: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if path_arg < 0 as ::core::ffi::c_int {
        fd_proc_path(
            t,
            eng_arg(r, fd_arg) as ::core::ffi::c_long as ::core::ffi::c_int,
            host,
            PATH_MAX as size_t,
        );
        *nf = 0 as ::core::ffi::c_int;
    } else {
        let mut rc: ::core::ffi::c_int = resolve_arg(
            t,
            r,
            dfd_arg,
            path_arg,
            if nofollow != 0 {
                0 as ::core::ffi::c_int
            } else {
                ENG_RES_FOLLOW
            },
            allow_empty,
            &raw mut res,
            &raw mut empty,
        );
        if rc != 0 {
            return rc;
        }
        if empty != 0 {
            fd_proc_path(
                t,
                eng_arg(r, dfd_arg) as ::core::ffi::c_long as ::core::ffi::c_int,
                host,
                PATH_MAX as size_t,
            );
            *nf = 0 as ::core::ffi::c_int;
        } else {
            snprintf(
                host,
                PATH_MAX as size_t,
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut res.host as *mut ::core::ffi::c_char,
            );
            *nf = (nofollow != 0 && res.stub == 0) as ::core::ffi::c_int;
        }
    }
    return host_stat(host, *nf, st);
}
unsafe extern "C" fn h_chmod(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut fd_arg: ::core::ffi::c_int,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut mode_arg: ::core::ffi::c_int,
    mut flag_arg: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut flags: ::core::ffi::c_long = if flag_arg >= 0 as ::core::ffi::c_int {
        eng_arg(r, flag_arg) as ::core::ffi::c_long
    } else {
        0 as ::core::ffi::c_long
    };
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut st: stat = stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad1: 0,
        st_size: 0,
        st_blksize: 0,
        __pad2: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __unused4: 0,
        __unused5: 0,
    };
    let mut nf: ::core::ffi::c_int = 0;
    let mut rc: ::core::ffi::c_int = meta_target(
        t,
        r,
        fd_arg,
        dfd_arg,
        path_arg,
        (flags & AT_SYMLINK_NOFOLLOW as ::core::ffi::c_long != 0 as ::core::ffi::c_long)
            as ::core::ffi::c_int,
        (flags & AT_EMPTY_PATH as ::core::ffi::c_long != 0 as ::core::ffi::c_long)
            as ::core::ffi::c_int,
        &raw mut host as *mut ::core::ffi::c_char,
        &raw mut st,
        &raw mut nf,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if st.st_mode & S_IFMT as mode_t == S_IFLNK as mode_t {
        return eng_task_void(t, r, -(EOPNOTSUPP as ::core::ffi::c_long));
    }
    let mut lk: ::core::ffi::c_int = eng_meta_lock(eng_tracer_guest((*t).tr));
    let mut m: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    eng_meta_get(
        eng_tracer_guest((*t).tr),
        &raw mut host as *mut ::core::ffi::c_char,
        nf,
        &raw mut st,
        &raw mut m,
    );
    if (*t).cr.fsuid != m.uid && eng_capable(t, ENG_CAP_FOWNER) == 0 {
        eng_meta_unlock(lk);
        return eng_task_void(t, r, -(EPERM as ::core::ffi::c_long));
    }
    let mut mode: uint32_t =
        eng_arg(r, mode_arg) as ::core::ffi::c_long as uint32_t & 0o7777 as uint32_t;
    if !(st.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t)
        && eng_in_group(t, m.gid, 0 as ::core::ffi::c_int) == 0
        && eng_capable(t, ENG_CAP_FSETID) == 0
    {
        mode &= !(S_ISGID as uint32_t);
    }
    if eng_meta_in_store(
        eng_tracer_guest((*t).tr),
        &raw mut host as *mut ::core::ffi::c_char,
    ) == 0
    {
        rc = if chmod(
            &raw mut host as *mut ::core::ffi::c_char,
            mode as mode_t & 0o777 as mode_t,
        ) == 0 as ::core::ffi::c_int
        {
            0 as ::core::ffi::c_int
        } else {
            -*__errno()
        };
    } else {
        m.mode = m.mode & S_IFMT as uint32_t | mode;
        rc = eng_meta_write(&raw mut host as *mut ::core::ffi::c_char, nf, &raw mut m);
        let mut hm: uint32_t = mode & 0o777 as uint32_t
            | (if st.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t {
                0o700 as ::core::ffi::c_int
            } else {
                0o600 as ::core::ffi::c_int
            }) as uint32_t;
        if rc == -ENOTSUP || rc == -EOPNOTSUPP {
            rc = if chmod(
                &raw mut host as *mut ::core::ffi::c_char,
                mode as mode_t & 0o777 as mode_t,
            ) == 0 as ::core::ffi::c_int
            {
                0 as ::core::ffi::c_int
            } else {
                -*__errno()
            };
        } else if rc == 0 as ::core::ffi::c_int && st.st_mode as uint32_t & 0o7777 as uint32_t != hm
        {
            chmod(&raw mut host as *mut ::core::ffi::c_char, hm as mode_t);
        }
    }
    eng_meta_unlock(lk);
    return eng_task_void(t, r, rc as ::core::ffi::c_long);
}
unsafe extern "C" fn h_chown(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut fd_arg: ::core::ffi::c_int,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut uid_arg: ::core::ffi::c_int,
    mut gid_arg: ::core::ffi::c_int,
    mut flag_arg: ::core::ffi::c_int,
    mut nofollow_always: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut flags: ::core::ffi::c_long = if flag_arg >= 0 as ::core::ffi::c_int {
        eng_arg(r, flag_arg) as ::core::ffi::c_long
    } else {
        0 as ::core::ffi::c_long
    };
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut st: stat = stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad1: 0,
        st_size: 0,
        st_blksize: 0,
        __pad2: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __unused4: 0,
        __unused5: 0,
    };
    let mut nf: ::core::ffi::c_int = 0;
    let mut rc: ::core::ffi::c_int = meta_target(
        t,
        r,
        fd_arg,
        dfd_arg,
        path_arg,
        (nofollow_always != 0 || flags & AT_SYMLINK_NOFOLLOW as ::core::ffi::c_long != 0)
            as ::core::ffi::c_int,
        (flags & AT_EMPTY_PATH as ::core::ffi::c_long != 0 as ::core::ffi::c_long)
            as ::core::ffi::c_int,
        &raw mut host as *mut ::core::ffi::c_char,
        &raw mut st,
        &raw mut nf,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    let mut nu: uint32_t = eng_arg(r, uid_arg) as ::core::ffi::c_long as uint32_t;
    let mut ng: uint32_t = eng_arg(r, gid_arg) as ::core::ffi::c_long as uint32_t;
    let mut lk: ::core::ffi::c_int = eng_meta_lock(eng_tracer_guest((*t).tr));
    let mut m: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    eng_meta_get(
        eng_tracer_guest((*t).tr),
        &raw mut host as *mut ::core::ffi::c_char,
        nf,
        &raw mut st,
        &raw mut m,
    );
    rc = 0 as ::core::ffi::c_int;
    if eng_capable(t, ENG_CAP_CHOWN) == 0 {
        if nu != -1 as ::core::ffi::c_int as uint32_t && nu != m.uid {
            rc = -EPERM;
        } else if ng != -1 as ::core::ffi::c_int as uint32_t
            && ((*t).cr.fsuid != m.uid
                || ng != m.gid && eng_in_group(t, ng, 0 as ::core::ffi::c_int) == 0)
        {
            rc = -EPERM;
        } else if (*t).cr.fsuid != m.uid
            && (nu != -1 as ::core::ffi::c_int as uint32_t
                || ng != -1 as ::core::ffi::c_int as uint32_t)
        {
            rc = -EPERM;
        }
    }
    if rc != 0
        || st.st_mode & S_IFMT as mode_t == S_IFLNK as mode_t
        || eng_meta_in_store(
            eng_tracer_guest((*t).tr),
            &raw mut host as *mut ::core::ffi::c_char,
        ) == 0
    {
        eng_meta_unlock(lk);
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if nu != -1 as ::core::ffi::c_int as uint32_t {
        m.uid = nu;
    }
    if ng != -1 as ::core::ffi::c_int as uint32_t {
        m.gid = ng;
    }
    if !(st.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t)
        && (nu != -1 as ::core::ffi::c_int as uint32_t
            || ng != -1 as ::core::ffi::c_int as uint32_t)
    {
        m.mode &= !(S_ISUID as uint32_t);
        if m.mode & S_IXGRP as uint32_t != 0 {
            m.mode &= !(S_ISGID as uint32_t);
        }
    }
    rc = eng_meta_write(&raw mut host as *mut ::core::ffi::c_char, nf, &raw mut m);
    if rc == -ENOTSUP || rc == -EOPNOTSUPP {
        rc = if eng_capable(t, ENG_CAP_CHOWN) != 0 {
            0 as ::core::ffi::c_int
        } else {
            -EPERM
        };
    }
    eng_meta_unlock(lk);
    return eng_task_void(t, r, rc as ::core::ffi::c_long);
}
unsafe extern "C" fn h_readlink(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut buf_arg: ::core::ffi::c_int,
    mut sz_arg: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut empty: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        0 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
        &raw mut res,
        &raw mut empty,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if empty != 0 {
        return 0 as ::core::ffi::c_int;
    }
    if res.stub != 0 {
        return eng_task_void(t, r, -(EINVAL as ::core::ffi::c_long));
    }
    if res.magic != 0 {
        let mut bufsz: ::core::ffi::c_long = eng_arg(r, sz_arg) as ::core::ffi::c_long;
        if bufsz <= 0 as ::core::ffi::c_long {
            return eng_task_void(t, r, -(EINVAL as ::core::ffi::c_long));
        }
        let mut len: size_t = strlen(&raw mut res.magic_text as *mut ::core::ffi::c_char);
        if len as ::core::ffi::c_long > bufsz {
            len = bufsz as size_t;
        }
        if eng_mem_write(
            (*t).tid,
            eng_arg(r, buf_arg) as uintptr_t,
            &raw mut res.magic_text as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            len,
        ) != len as ssize_t
        {
            return eng_task_void(t, r, -(EFAULT as ::core::ffi::c_long));
        }
        return eng_task_void(t, r, len as ::core::ffi::c_long);
    }
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_getcwd(mut t: *mut eng_task, mut r: *mut eng_regs) -> ::core::ffi::c_int {
    let mut cwd: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut rc: ::core::ffi::c_int = eng_task_cwd(
        t,
        &raw mut cwd as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    if rc == -ENOENT {
        return 0 as ::core::ffi::c_int;
    }
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    let mut p: [::core::ffi::c_char; 64] = [0; 64];
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut a: stat = stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad1: 0,
        st_size: 0,
        st_blksize: 0,
        __pad2: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __unused4: 0,
        __unused5: 0,
    };
    let mut b: stat = stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad1: 0,
        st_size: 0,
        st_blksize: 0,
        __pad2: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __unused4: 0,
        __unused5: 0,
    };
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
        b"/proc/%d/cwd\0".as_ptr() as *const ::core::ffi::c_char,
        (*t).tid,
    );
    if stat(&raw mut p as *mut ::core::ffi::c_char, &raw mut a) == 0 as ::core::ffi::c_int
        && a.st_nlink == 0 as nlink_t
    {
        return eng_task_void(t, r, -(ENOENT as ::core::ffi::c_long));
    }
    if eng_guest_to_host(
        eng_tracer_guest((*t).tr),
        &raw mut cwd as *mut ::core::ffi::c_char,
        &raw mut host as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) == 0 as ::core::ffi::c_int
        && stat(&raw mut p as *mut ::core::ffi::c_char, &raw mut a) == 0 as ::core::ffi::c_int
        && (stat(&raw mut host as *mut ::core::ffi::c_char, &raw mut b) != 0 as ::core::ffi::c_int
            || a.st_ino != b.st_ino
            || a.st_dev != b.st_dev)
    {
        return eng_task_void(t, r, -(ENOENT as ::core::ffi::c_long));
    }
    let mut need: size_t =
        strlen(&raw mut cwd as *mut ::core::ffi::c_char).wrapping_add(1 as size_t);
    if (eng_arg(r, 1 as ::core::ffi::c_int) as ::core::ffi::c_long as uint64_t) < need as uint64_t {
        return eng_task_void(t, r, -(ERANGE as ::core::ffi::c_long));
    }
    if eng_mem_write(
        (*t).tid,
        eng_arg(r, 0 as ::core::ffi::c_int) as uintptr_t,
        &raw mut cwd as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
        need,
    ) != need as ssize_t
    {
        return eng_task_void(t, r, -(EFAULT as ::core::ffi::c_long));
    }
    return eng_task_void(t, r, need as ::core::ffi::c_long);
}
unsafe extern "C" fn h_exec(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut is_at: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut path: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut dfd: ::core::ffi::c_int = if is_at != 0 {
        eng_arg(r, 0 as ::core::ffi::c_int) as ::core::ffi::c_long as ::core::ffi::c_int
    } else {
        AT_FDCWD
    };
    let mut pidx: ::core::ffi::c_int = if is_at != 0 {
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
    let mut fl: ::core::ffi::c_int = if is_at != 0 {
        eng_arg(r, 4 as ::core::ffi::c_int) as ::core::ffi::c_long as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
    let mut n: ::core::ffi::c_long = read_path(
        t,
        eng_arg(r, pidx),
        &raw mut path as *mut ::core::ffi::c_char,
    );
    if n < 0 as ::core::ffi::c_long {
        return eng_task_void(t, r, n);
    }
    let mut rc: ::core::ffi::c_int =
        eng_exec_prepare(t, dfd, &raw mut path as *mut ::core::ffi::c_char, fl);
    if rc != 0 {
        eng_logf!(
            eng_log_level::ENG_LOG_DEBUG,
            b"exec %s refused: %s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut path as *mut ::core::ffi::c_char,
            strerror(-rc),
        );
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    let mut a: uint64_t = eng_scratch_put_str(
        t,
        r,
        &raw mut (*(eng_tracer_guest as unsafe extern "C" fn(*mut eng_tracer) -> *mut eng_guest)(
            (*t).tr,
        ))
        .loader as *mut ::core::ffi::c_char,
    );
    if a == 0 {
        eng_exec_discard(t);
        return eng_task_void(t, r, -(ENOMEM as ::core::ffi::c_long));
    }
    if is_at != 0 {
        let mut argv: uint64_t = eng_arg(r, 2 as ::core::ffi::c_int);
        let mut envp: uint64_t = eng_arg(r, 3 as ::core::ffi::c_int);
        eng_set_arg(r, 0 as ::core::ffi::c_int, a);
        eng_set_arg(r, 1 as ::core::ffi::c_int, argv);
        eng_set_arg(r, 2 as ::core::ffi::c_int, envp);
        eng_syscall_set((*t).tid, r, __NR_execve as ::core::ffi::c_long);
    } else {
        eng_set_arg(r, 0 as ::core::ffi::c_int, a);
    }
    (*t).regs_modified = 1 as ::core::ffi::c_int;
    return 1 as ::core::ffi::c_int;
}
static mut PATHSYS: [pathsys; 5] = [
    pathsys {
        nr: __NR_truncate as ::core::ffi::c_long,
        dfd: -1 as ::core::ffi::c_schar,
        path: 0 as ::core::ffi::c_schar,
        flagarg: -1 as ::core::ffi::c_schar,
        mode: C2Rust_Unnamed_13::FL_FOLLOW.0 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        nullok: 0 as ::core::ffi::c_uchar,
        dironly: 0 as ::core::ffi::c_uchar,
        entry: 0 as ::core::ffi::c_uchar,
    },
    pathsys {
        nr: __NR_utimensat as ::core::ffi::c_long,
        dfd: 0 as ::core::ffi::c_schar,
        path: 1 as ::core::ffi::c_schar,
        flagarg: 3 as ::core::ffi::c_schar,
        mode: C2Rust_Unnamed_13::FL_ATFLAG.0 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        nullok: 1 as ::core::ffi::c_uchar,
        dironly: 0 as ::core::ffi::c_uchar,
        entry: 0 as ::core::ffi::c_uchar,
    },
    pathsys {
        nr: __NR_statfs as ::core::ffi::c_long,
        dfd: -1 as ::core::ffi::c_schar,
        path: 0 as ::core::ffi::c_schar,
        flagarg: -1 as ::core::ffi::c_schar,
        mode: C2Rust_Unnamed_13::FL_FOLLOW.0 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        nullok: 0 as ::core::ffi::c_uchar,
        dironly: 0 as ::core::ffi::c_uchar,
        entry: 0 as ::core::ffi::c_uchar,
    },
    pathsys {
        nr: __NR_chdir as ::core::ffi::c_long,
        dfd: -1 as ::core::ffi::c_schar,
        path: 0 as ::core::ffi::c_schar,
        flagarg: -1 as ::core::ffi::c_schar,
        mode: C2Rust_Unnamed_13::FL_FOLLOW.0 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        nullok: 0 as ::core::ffi::c_uchar,
        dironly: 1 as ::core::ffi::c_uchar,
        entry: 0 as ::core::ffi::c_uchar,
    },
    pathsys {
        nr: __NR_inotify_add_watch as ::core::ffi::c_long,
        dfd: -1 as ::core::ffi::c_schar,
        path: 1 as ::core::ffi::c_schar,
        flagarg: 2 as ::core::ffi::c_schar,
        mode: C2Rust_Unnamed_13::FL_INOTIFY.0 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        nullok: 0 as ::core::ffi::c_uchar,
        dironly: 0 as ::core::ffi::c_uchar,
        entry: 0 as ::core::ffi::c_uchar,
    },
];
unsafe extern "C" fn h_pathsys(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut d: *const pathsys,
) -> ::core::ffi::c_int {
    let mut f: ::core::ffi::c_long =
        if (*d).flagarg as ::core::ffi::c_int >= 0 as ::core::ffi::c_int {
            eng_arg(r, (*d).flagarg as ::core::ffi::c_int) as ::core::ffi::c_long
        } else {
            0 as ::core::ffi::c_long
        };
    let mut addr: uint64_t = eng_arg(r, (*d).path as ::core::ffi::c_int);
    if addr == 0 && (*d).nullok as ::core::ffi::c_int != 0 {
        return 0 as ::core::ffi::c_int;
    }
    let mut follow: ::core::ffi::c_int = ((*d).mode as ::core::ffi::c_int
        == C2Rust_Unnamed_13::FL_FOLLOW.0 as ::core::ffi::c_int
        || (*d).mode as ::core::ffi::c_int == C2Rust_Unnamed_13::FL_ATFLAG.0 as ::core::ffi::c_int
            && f & AT_SYMLINK_NOFOLLOW as ::core::ffi::c_long == 0
        || (*d).mode as ::core::ffi::c_int == C2Rust_Unnamed_13::FL_INOTIFY.0 as ::core::ffi::c_int
            && f & IN_DONT_FOLLOW as ::core::ffi::c_long == 0)
        as ::core::ffi::c_int;
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut empty: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        (*d).dfd as ::core::ffi::c_int,
        (*d).path as ::core::ffi::c_int,
        if follow != 0 {
            ENG_RES_FOLLOW
        } else {
            0 as ::core::ffi::c_int
        } | if (*d).dironly as ::core::ffi::c_int != 0 {
            ENG_RES_DIR_ONLY
        } else {
            0 as ::core::ffi::c_int
        },
        ((*d).mode as ::core::ffi::c_int == C2Rust_Unnamed_13::FL_ATFLAG.0 as ::core::ffi::c_int
            && f & AT_EMPTY_PATH as ::core::ffi::c_long != 0) as ::core::ffi::c_int,
        &raw mut res,
        &raw mut empty,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if empty != 0 {
        return 0 as ::core::ffi::c_int;
    }
    if (*d).nr == __NR_chdir as ::core::ffi::c_long && res.verbatim == 0 {
        let mut st: stat = stat {
            st_dev: 0,
            st_ino: 0,
            st_mode: 0,
            st_nlink: 0,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad1: 0,
            st_size: 0,
            st_blksize: 0,
            __pad2: 0,
            st_blocks: 0,
            st_atim: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            st_mtim: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            st_ctim: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            __unused4: 0,
            __unused5: 0,
        };
        if stat(&raw mut res.host as *mut ::core::ffi::c_char, &raw mut st)
            == 0 as ::core::ffi::c_int
        {
            let mut m: eng_meta = eng_meta {
                uid: 0,
                gid: 0,
                mode: 0,
                nlink: 0,
                major: 0,
                minor: 0,
                present: 0,
            };
            eng_meta_get(
                eng_tracer_guest((*t).tr),
                &raw mut res.host as *mut ::core::ffi::c_char,
                0 as ::core::ffi::c_int,
                &raw mut st,
                &raw mut m,
            );
            rc = eng_meta_permission(t, &raw mut m, X_OK, 0 as ::core::ffi::c_int);
            if rc != 0 {
                return eng_task_void(t, r, rc as ::core::ffi::c_long);
            }
        }
    }
    rc = put_path(
        t,
        r,
        (*d).path as ::core::ffi::c_int,
        if (*d).entry as ::core::ffi::c_int != 0 {
            &raw mut res.entry as *mut ::core::ffi::c_char
        } else {
            &raw mut res.host as *mut ::core::ffi::c_char
        },
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_xattr(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut op: ::core::ffi::c_int,
    mut path_arg: ::core::ffi::c_int,
    mut nofollow: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if op != C2Rust_Unnamed_14::XA_LIST.0 as ::core::ffi::c_int {
        let mut name: [::core::ffi::c_char; 256] = [0; 256];
        let mut n: ssize_t = eng_mem_read_cstr(
            (*t).tid,
            eng_arg(r, 1 as ::core::ffi::c_int) as uintptr_t,
            &raw mut name as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 256]>(),
        );
        if n < 0 as ssize_t {
            return eng_task_void(t, r, -(EFAULT as ::core::ffi::c_long));
        }
        if strncmp(
            &raw mut name as *mut ::core::ffi::c_char,
            ENG_META_PREFIX.as_ptr(),
            ::core::mem::size_of::<[::core::ffi::c_char; 15]>().wrapping_sub(1 as size_t),
        ) == 0
        {
            return eng_task_void(
                t,
                r,
                (if op == C2Rust_Unnamed_14::XA_GET.0 as ::core::ffi::c_int {
                    -ENODATA
                } else {
                    -EPERM
                }) as ::core::ffi::c_long,
            );
        }
        if strcmp(
            &raw mut name as *mut ::core::ffi::c_char,
            b"security.selinux\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        {
            return eng_task_void(
                t,
                r,
                (if op == C2Rust_Unnamed_14::XA_GET.0 as ::core::ffi::c_int {
                    -ENODATA
                } else {
                    -EOPNOTSUPP
                }) as ::core::ffi::c_long,
            );
        }
        if op == C2Rust_Unnamed_14::XA_SET.0 as ::core::ffi::c_int
            && strcmp(
                &raw mut name as *mut ::core::ffi::c_char,
                b"security.capability\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
        {
            return eng_task_void(t, r, -(EOPNOTSUPP as ::core::ffi::c_long));
        }
    } else {
        (*t).fix_addr = eng_arg(r, 1 as ::core::ffi::c_int);
        (*t).fix_len = eng_arg(r, 2 as ::core::ffi::c_int);
        (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_LISTXATTR.0 as ::core::ffi::c_int;
    }
    if path_arg < 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut rc: ::core::ffi::c_int = resolve_arg(
        t,
        r,
        -1 as ::core::ffi::c_int,
        path_arg,
        if nofollow != 0 {
            0 as ::core::ffi::c_int
        } else {
            ENG_RES_FOLLOW
        },
        0 as ::core::ffi::c_int,
        &raw mut res,
        ::core::ptr::null_mut::<::core::ffi::c_int>(),
    );
    if rc != 0 {
        (*t).fixup = 0 as ::core::ffi::c_int;
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.host as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        (*t).fixup = 0 as ::core::ffi::c_int;
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn x_listxattr(mut t: *mut eng_task, mut ret: ::core::ffi::c_long) {
    if ret <= 0 as ::core::ffi::c_long || (*t).fix_addr == 0 || (*t).fix_len == 0 {
        return;
    }
    let mut buf: *mut ::core::ffi::c_char = malloc(ret as size_t) as *mut ::core::ffi::c_char;
    if buf.is_null() {
        return;
    }
    if eng_mem_read(
        (*t).tid,
        (*t).fix_addr as uintptr_t,
        buf as *mut ::core::ffi::c_void,
        ret as size_t,
    ) == ret as ssize_t
    {
        let mut out: *mut ::core::ffi::c_char = malloc(ret as size_t) as *mut ::core::ffi::c_char;
        let mut o: size_t = 0 as size_t;
        let mut i: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
        while i < ret {
            let mut l: size_t = strnlen(buf.offset(i as isize), (ret - i) as size_t);
            if strncmp(
                buf.offset(i as isize),
                ENG_META_PREFIX.as_ptr(),
                ::core::mem::size_of::<[::core::ffi::c_char; 15]>().wrapping_sub(1 as size_t),
            ) != 0
                && strcmp(
                    buf.offset(i as isize),
                    b"security.selinux\0".as_ptr() as *const ::core::ffi::c_char,
                ) != 0
            {
                memcpy(
                    out.offset(o as isize) as *mut ::core::ffi::c_void,
                    buf.offset(i as isize) as *const ::core::ffi::c_void,
                    l.wrapping_add(1 as size_t),
                );
                o = o.wrapping_add(l.wrapping_add(1 as size_t));
            }
            i += l as ::core::ffi::c_long + 1 as ::core::ffi::c_long;
        }
        if o as ::core::ffi::c_long != ret {
            eng_mem_write(
                (*t).tid,
                (*t).fix_addr as uintptr_t,
                out as *const ::core::ffi::c_void,
                o,
            );
            (*t).inject_result = o as ::core::ffi::c_long;
            (*t).void_pending = 1 as ::core::ffi::c_int;
        }
        free(out as *mut ::core::ffi::c_void);
    }
    free(buf as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn x_getdents(mut t: *mut eng_task, mut ret: ::core::ffi::c_long) {
    if ret <= 0 as ::core::ffi::c_long {
        return;
    }
    let mut g: *mut eng_guest = eng_tracer_guest((*t).tr);
    let mut dirhost: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut dirguest: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut link: [::core::ffi::c_char; 64] = [0; 64];
    snprintf(
        &raw mut link as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
        b"/proc/%d/fd/%d\0".as_ptr() as *const ::core::ffi::c_char,
        (*t).tid,
        (*t).fix_aux as ::core::ffi::c_int,
    );
    let mut dn: ssize_t = readlink(
        &raw mut link as *mut ::core::ffi::c_char,
        &raw mut dirhost as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as size_t),
    );
    if dn <= 0 as ssize_t {
        return;
    }
    dirhost[dn as usize] = 0 as ::core::ffi::c_char;
    let mut have_guest: ::core::ffi::c_int = (eng_host_to_guest(
        g,
        &raw mut dirhost as *mut ::core::ffi::c_char,
        &raw mut dirguest as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) == 0 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    let mut buf: *mut ::core::ffi::c_uchar = malloc(ret as size_t) as *mut ::core::ffi::c_uchar;
    if buf.is_null() {
        return;
    }
    if eng_mem_read(
        (*t).tid,
        (*t).fix_addr as uintptr_t,
        buf as *mut ::core::ffi::c_void,
        ret as size_t,
    ) != ret as ssize_t
    {
        free(buf as *mut ::core::ffi::c_void);
        return;
    }
    let mut o: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
    let mut changed: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
    let mut i: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
    while i + ::core::mem::size_of::<dirent64_hdr>() as ::core::ffi::c_long <= ret {
        let mut h: dirent64_hdr = dirent64_hdr {
            ino: 0,
            off: 0,
            reclen: 0,
            r#type: 0,
        };
        memcpy(
            &raw mut h as *mut ::core::ffi::c_void,
            buf.offset(i as isize) as *const ::core::ffi::c_void,
            ::core::mem::size_of::<dirent64_hdr>(),
        );
        if h.reclen as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || i + h.reclen as ::core::ffi::c_long > ret
        {
            break;
        }
        let mut name: *const ::core::ffi::c_char = (buf as *const ::core::ffi::c_char)
            .offset(i as isize)
            .offset(::core::mem::size_of::<dirent64_hdr>() as isize);
        let mut drop_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        if have_guest != 0 && (*g).nhides != 0 {
            let mut gp: [::core::ffi::c_char; 4096] = [0; 4096];
            snprintf(
                &raw mut gp as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut dirguest as *mut ::core::ffi::c_char,
                if strcmp(
                    &raw mut dirguest as *mut ::core::ffi::c_char,
                    b"/\0".as_ptr() as *const ::core::ffi::c_char,
                ) != 0
                {
                    b"/\0".as_ptr() as *const ::core::ffi::c_char
                } else {
                    b"\0".as_ptr() as *const ::core::ffi::c_char
                },
                name,
            );
            drop_0 = eng_guest_is_hidden(g, &raw mut gp as *mut ::core::ffi::c_char);
        }
        if drop_0 == 0 && h.r#type as ::core::ffi::c_int == 10 as ::core::ffi::c_int {
            let mut hp: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut txt: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut id: [::core::ffi::c_char; 64] = [0; 64];
            snprintf(
                &raw mut hp as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut dirhost as *mut ::core::ffi::c_char,
                name,
            );
            let mut n: ssize_t = readlink(
                &raw mut hp as *mut ::core::ffi::c_char,
                &raw mut txt as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as size_t),
            );
            if n > 0 as ssize_t {
                txt[n as usize] = 0 as ::core::ffi::c_char;
                if eng_link_is_stub_text(
                    &raw mut txt as *mut ::core::ffi::c_char,
                    &raw mut id as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                ) != 0
                {
                    let mut obj: [::core::ffi::c_char; 4096] = [0; 4096];
                    let mut st: stat = stat {
                        st_dev: 0,
                        st_ino: 0,
                        st_mode: 0,
                        st_nlink: 0,
                        st_uid: 0,
                        st_gid: 0,
                        st_rdev: 0,
                        __pad1: 0,
                        st_size: 0,
                        st_blksize: 0,
                        __pad2: 0,
                        st_blocks: 0,
                        st_atim: timespec {
                            tv_sec: 0,
                            tv_nsec: 0,
                        },
                        st_mtim: timespec {
                            tv_sec: 0,
                            tv_nsec: 0,
                        },
                        st_ctim: timespec {
                            tv_sec: 0,
                            tv_nsec: 0,
                        },
                        __unused4: 0,
                        __unused5: 0,
                    };
                    h.r#type = 8 as uint8_t;
                    if eng_link_object_path(
                        g,
                        &raw mut id as *mut ::core::ffi::c_char,
                        &raw mut obj as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                    ) == 0 as ::core::ffi::c_int
                        && stat(&raw mut obj as *mut ::core::ffi::c_char, &raw mut st)
                            == 0 as ::core::ffi::c_int
                    {
                        h.ino = st.st_ino as uint64_t;
                    }
                    memcpy(
                        buf.offset(i as isize) as *mut ::core::ffi::c_void,
                        &raw mut h as *const ::core::ffi::c_void,
                        ::core::mem::size_of::<dirent64_hdr>(),
                    );
                    changed = 1 as ::core::ffi::c_long;
                }
            }
        }
        if drop_0 != 0 {
            changed = 1 as ::core::ffi::c_long;
        } else {
            if o != i {
                memmove(
                    buf.offset(o as isize) as *mut ::core::ffi::c_void,
                    buf.offset(i as isize) as *const ::core::ffi::c_void,
                    h.reclen as size_t,
                );
            }
            o += h.reclen as ::core::ffi::c_long;
        }
        i += h.reclen as ::core::ffi::c_long;
    }
    if changed != 0 {
        eng_mem_write(
            (*t).tid,
            (*t).fix_addr as uintptr_t,
            buf as *const ::core::ffi::c_void,
            o as size_t,
        );
        if o != ret {
            (*t).inject_result = o;
            (*t).void_pending = 1 as ::core::ffi::c_int;
        }
    }
    free(buf as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn default_policy(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut nr: ::core::ffi::c_long,
) -> ::core::ffi::c_int {
    let mut c: eng_sc_class = eng_sysinv_class(nr);
    static mut warned: [::core::ffi::c_uchar; 1024] = [0; 1024];
    match c {
        eng_sc_class::ENG_SC_PASS
        | eng_sc_class::ENG_SC_FD
        | eng_sc_class::ENG_SC_PROC
        | eng_sc_class::ENG_SC_SOCK
        | eng_sc_class::ENG_SC_ID => return 0 as ::core::ffi::c_int,
        eng_sc_class::ENG_SC_EPERM => {
            return eng_task_void(t, r, -(EPERM as ::core::ffi::c_long));
        }
        _ => {
            if nr >= 0 as ::core::ffi::c_long
                && nr
                    < ::core::mem::size_of::<[::core::ffi::c_uchar; 1024]>() as ::core::ffi::c_long
                && warned[nr as usize] == 0
            {
                warned[nr as usize] = 1 as ::core::ffi::c_uchar;
                eng_logf!(
                    eng_log_level::ENG_LOG_INFO,
                    b"refusing syscall %ld (%s, class %s) with ENOSYS\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    nr,
                    if !eng_sysinv_name(nr).is_null() {
                        eng_sysinv_name(nr)
                    } else {
                        b"unknown\0".as_ptr() as *const ::core::ffi::c_char
                    },
                    eng_sysinv_class_name(c),
                );
            }
            return eng_task_void(t, r, -(ENOSYS as ::core::ffi::c_long));
        }
    };
}
pub const SUN_PATH_MAX: ::core::ffi::c_int = 108 as ::core::ffi::c_int;
unsafe extern "C" fn fnv1a(mut s: *const ::core::ffi::c_char) -> uint64_t {
    let mut h: uint64_t = 1469598103934665603 as uint64_t;
    while *s != 0 {
        h ^= *s as ::core::ffi::c_uchar as uint64_t;
        h = (h as ::core::ffi::c_ulonglong).wrapping_mul(1099511628211 as ::core::ffi::c_ulonglong)
            as uint64_t;
        s = s.offset(1);
    }
    return h;
}
unsafe extern "C" fn sock_host_name(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if strlen(host) < SUN_PATH_MAX as size_t {
        snprintf(
            out,
            SUN_PATH_MAX as size_t,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            host,
        );
        return 0 as ::core::ffi::c_int;
    }
    let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        host,
    );
    let mut sl: *mut ::core::ffi::c_char = strrchr(
        &raw mut dir as *mut ::core::ffi::c_char,
        '/' as ::core::ffi::c_int,
    );
    if sl.is_null() || (*g).sockdir[0usize] == 0 {
        return -ENAMETOOLONG;
    }
    *sl = 0 as ::core::ffi::c_char;
    let mut base: *const ::core::ffi::c_char = sl.offset(1 as ::core::ffi::c_int as isize);
    let mut alias: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut alias as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/%016llx\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).sockdir as *mut ::core::ffi::c_char,
        fnv1a(&raw mut dir as *mut ::core::ffi::c_char) as ::core::ffi::c_ulonglong,
    );
    let mut cur: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut n: ssize_t = readlink(
        &raw mut alias as *mut ::core::ffi::c_char,
        &raw mut cur as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as size_t),
    );
    if n < 0 as ssize_t
        || n as size_t != strlen(&raw mut dir as *mut ::core::ffi::c_char)
        || memcmp(
            &raw mut cur as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            &raw mut dir as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            n as size_t,
        ) != 0
    {
        mkdir(
            &raw mut (*g).sockdir as *mut ::core::ffi::c_char,
            0o700 as mode_t,
        );
        let mut tmp: [::core::ffi::c_char; 4128] = [0; 4128];
        snprintf(
            &raw mut tmp as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4128]>(),
            b"%s.%d\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut alias as *mut ::core::ffi::c_char,
            getpid(),
        );
        unlink(&raw mut tmp as *mut ::core::ffi::c_char);
        if symlink(
            &raw mut dir as *mut ::core::ffi::c_char,
            &raw mut tmp as *mut ::core::ffi::c_char,
        ) != 0 as ::core::ffi::c_int
            || rename(
                &raw mut tmp as *mut ::core::ffi::c_char,
                &raw mut alias as *mut ::core::ffi::c_char,
            ) != 0 as ::core::ffi::c_int
        {
            unlink(&raw mut tmp as *mut ::core::ffi::c_char);
            return -ENAMETOOLONG;
        }
    }
    if strlen(&raw mut alias as *mut ::core::ffi::c_char)
        .wrapping_add(1 as size_t)
        .wrapping_add(strlen(base))
        >= SUN_PATH_MAX as size_t
    {
        return -ENAMETOOLONG;
    }
    snprintf(
        out,
        SUN_PATH_MAX as size_t,
        b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut alias as *mut ::core::ffi::c_char,
        base,
    );
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn sock_translate(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut addr: uint64_t,
    mut len: uint64_t,
    mut creating: ::core::ffi::c_int,
    mut new_addr: *mut uint64_t,
    mut new_len: *mut uint64_t,
) -> ::core::ffi::c_int {
    if addr == 0
        || len <= 2 as uint64_t
        || len > ::core::mem::size_of::<wf_sockaddr_un>() as uint64_t
    {
        return 1 as ::core::ffi::c_int;
    }
    let mut sa: wf_sockaddr_un = wf_sockaddr_un {
        family: 0,
        path: [0; 108],
    };
    memset(
        &raw mut sa as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<wf_sockaddr_un>(),
    );
    if eng_mem_read(
        (*t).tid,
        addr as uintptr_t,
        &raw mut sa as *mut ::core::ffi::c_void,
        len as size_t,
    ) != len as ssize_t
    {
        return -EFAULT;
    }
    if sa.family as ::core::ffi::c_int != 1 as ::core::ffi::c_int
        || sa.path[0usize] as ::core::ffi::c_int == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    let mut gp: [::core::ffi::c_char; 109] = [0; 109];
    let mut pl: size_t = (len as size_t).wrapping_sub(2 as size_t);
    memcpy(
        &raw mut gp as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        &raw mut sa.path as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
        pl,
    );
    gp[pl] = 0 as ::core::ffi::c_char;
    let mut res: eng_resolved = eng_resolved {
        guest: [0; 4096],
        host: [0; 4096],
        verbatim: 0,
        magic: 0,
        magic_text: [0; 4096],
        exists: 0,
        stub: 0,
        stub_id: [0; 64],
        entry: [0; 4096],
    };
    let mut rc: ::core::ffi::c_int = eng_resolve(
        t,
        AT_FDCWD,
        &raw mut gp as *mut ::core::ffi::c_char,
        if creating != 0 {
            ENG_RES_MISSING_OK
        } else {
            ENG_RES_FOLLOW
        },
        &raw mut res,
    );
    if rc != 0 {
        return rc;
    }
    if creating != 0 && res.exists != 0 {
        return -EADDRINUSE;
    }
    let mut out: wf_sockaddr_un = wf_sockaddr_un {
        family: 0,
        path: [0; 108],
    };
    memset(
        &raw mut out as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<wf_sockaddr_un>(),
    );
    out.family = 1 as uint16_t;
    rc = sock_host_name(
        eng_tracer_guest((*t).tr),
        if creating != 0 {
            &raw mut res.entry as *mut ::core::ffi::c_char
        } else {
            &raw mut res.host as *mut ::core::ffi::c_char
        },
        &raw mut out.path as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return rc;
    }
    let mut ol: size_t = (2 as size_t)
        .wrapping_add(strlen(&raw mut out.path as *mut ::core::ffi::c_char))
        .wrapping_add(1 as size_t);
    let mut a: uint64_t = eng_scratch_alloc(t, r, ::core::mem::size_of::<wf_sockaddr_un>());
    if a == 0
        || eng_mem_write(
            (*t).tid,
            a as uintptr_t,
            &raw mut out as *const ::core::ffi::c_void,
            ol,
        ) != ol as ssize_t
    {
        return -EFAULT;
    }
    *new_addr = a;
    *new_len = ol as uint64_t;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_sockaddr(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut addr_arg: ::core::ffi::c_int,
    mut len_arg: ::core::ffi::c_int,
    mut creating: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut na: uint64_t = 0;
    let mut nl: uint64_t = 0;
    let mut rc: ::core::ffi::c_int = sock_translate(
        t,
        r,
        eng_arg(r, addr_arg),
        eng_arg(r, len_arg),
        creating,
        &raw mut na,
        &raw mut nl,
    );
    if rc < 0 as ::core::ffi::c_int {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if rc == 1 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    eng_set_arg(r, addr_arg, na);
    eng_set_arg(r, len_arg, nl);
    (*t).regs_modified = 1 as ::core::ffi::c_int;
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn h_sendmsg(mut t: *mut eng_task, mut r: *mut eng_regs) -> ::core::ffi::c_int {
    let mut m: wf_msghdr = wf_msghdr {
        name: 0,
        namelen: 0,
        pad: 0,
        iov: 0,
        iovlen: 0,
        control: 0,
        controllen: 0,
        flags: 0,
        pad2: 0,
    };
    let mut ma: uint64_t = eng_arg(r, 1 as ::core::ffi::c_int);
    if ma == 0
        || eng_mem_read(
            (*t).tid,
            ma as uintptr_t,
            &raw mut m as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<wf_msghdr>(),
        ) != ::core::mem::size_of::<wf_msghdr>() as ssize_t
    {
        return 0 as ::core::ffi::c_int;
    }
    let mut na: uint64_t = 0;
    let mut nl: uint64_t = 0;
    let mut rc: ::core::ffi::c_int = sock_translate(
        t,
        r,
        m.name,
        m.namelen as uint64_t,
        0 as ::core::ffi::c_int,
        &raw mut na,
        &raw mut nl,
    );
    if rc < 0 as ::core::ffi::c_int {
        return eng_task_void(t, r, rc as ::core::ffi::c_long);
    }
    if rc == 1 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    m.name = na;
    m.namelen = nl as uint32_t;
    let mut a: uint64_t = eng_scratch_alloc(t, r, ::core::mem::size_of::<wf_msghdr>());
    if a == 0
        || eng_mem_write(
            (*t).tid,
            a as uintptr_t,
            &raw mut m as *const ::core::ffi::c_void,
            ::core::mem::size_of::<wf_msghdr>(),
        ) != ::core::mem::size_of::<wf_msghdr>() as ssize_t
    {
        return eng_task_void(t, r, -(EFAULT as ::core::ffi::c_long));
    }
    eng_set_arg(r, 1 as ::core::ffi::c_int, a);
    (*t).regs_modified = 1 as ::core::ffi::c_int;
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn x_sockname(mut t: *mut eng_task, mut ret: ::core::ffi::c_long) {
    if ret < 0 as ::core::ffi::c_long || (*t).fix_addr == 0 || (*t).fix_len == 0 {
        return;
    }
    let mut len: uint32_t = 0;
    if eng_mem_read(
        (*t).tid,
        (*t).fix_len as uintptr_t,
        &raw mut len as *mut ::core::ffi::c_void,
        4 as size_t,
    ) != 4 as ssize_t
        || len <= 2 as uint32_t
        || len as usize > ::core::mem::size_of::<wf_sockaddr_un>()
    {
        return;
    }
    let mut sa: wf_sockaddr_un = wf_sockaddr_un {
        family: 0,
        path: [0; 108],
    };
    memset(
        &raw mut sa as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<wf_sockaddr_un>(),
    );
    let mut cap: uint32_t = (*t).fix_aux as uint32_t;
    let mut rd: uint32_t = if len < cap { len } else { cap };
    if rd <= 2 as uint32_t
        || eng_mem_read(
            (*t).tid,
            (*t).fix_addr as uintptr_t,
            &raw mut sa as *mut ::core::ffi::c_void,
            rd as size_t,
        ) != rd as ssize_t
    {
        return;
    }
    if sa.family as ::core::ffi::c_int != 1 as ::core::ffi::c_int
        || sa.path[0usize] as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        || rd < len
    {
        return;
    }
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut guest: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut host as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%.*s\0".as_ptr() as *const ::core::ffi::c_char,
        len.wrapping_sub(2 as uint32_t) as ::core::ffi::c_int,
        &raw mut sa.path as *mut ::core::ffi::c_char,
    );
    let mut g: *mut eng_guest = eng_tracer_guest((*t).tr);
    let mut sdl: size_t = strlen(&raw mut (*g).sockdir as *mut ::core::ffi::c_char);
    if sdl != 0
        && strncmp(
            &raw mut host as *mut ::core::ffi::c_char,
            &raw mut (*g).sockdir as *mut ::core::ffi::c_char,
            sdl,
        ) == 0
        && host[sdl] as ::core::ffi::c_int == '/' as ::core::ffi::c_int
    {
        let mut alias: [::core::ffi::c_char; 4096] = [0; 4096];
        let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
        snprintf(
            &raw mut alias as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut host as *mut ::core::ffi::c_char,
        );
        let mut sl: *mut ::core::ffi::c_char = strrchr(
            &raw mut alias as *mut ::core::ffi::c_char,
            '/' as ::core::ffi::c_int,
        );
        if sl.is_null() {
            return;
        }
        *sl = 0 as ::core::ffi::c_char;
        let mut n: ssize_t = readlink(
            &raw mut alias as *mut ::core::ffi::c_char,
            &raw mut dir as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as size_t),
        );
        if n <= 0 as ssize_t {
            return;
        }
        dir[n as usize] = 0 as ::core::ffi::c_char;
        snprintf(
            &raw mut host as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut dir as *mut ::core::ffi::c_char,
            sl.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if eng_host_to_guest(
        g,
        &raw mut host as *mut ::core::ffi::c_char,
        &raw mut guest as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return;
    }
    let mut gl: size_t = strlen(&raw mut guest as *mut ::core::ffi::c_char);
    if gl >= SUN_PATH_MAX as size_t {
        return;
    }
    let mut out: wf_sockaddr_un = wf_sockaddr_un {
        family: 0,
        path: [0; 108],
    };
    memset(
        &raw mut out as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<wf_sockaddr_un>(),
    );
    out.family = 1 as uint16_t;
    memcpy(
        &raw mut out.path as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        &raw mut guest as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
        gl.wrapping_add(1 as size_t),
    );
    let mut olen: uint32_t = (2 as size_t).wrapping_add(gl).wrapping_add(1 as size_t) as uint32_t;
    let mut wr: uint32_t = if olen < cap { olen } else { cap };
    eng_mem_write(
        (*t).tid,
        (*t).fix_addr as uintptr_t,
        &raw mut out as *const ::core::ffi::c_void,
        wr as size_t,
    );
    eng_mem_write(
        (*t).tid,
        (*t).fix_len as uintptr_t,
        &raw mut olen as *const ::core::ffi::c_void,
        4 as size_t,
    );
}
unsafe extern "C" fn h_sockname(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut addr_arg: ::core::ffi::c_int,
    mut lenp_arg: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    (*t).fix_addr = eng_arg(r, addr_arg);
    (*t).fix_len = eng_arg(r, lenp_arg);
    let mut cap: uint32_t = 0 as uint32_t;
    if (*t).fix_len != 0
        && eng_mem_read(
            (*t).tid,
            (*t).fix_len as uintptr_t,
            &raw mut cap as *mut ::core::ffi::c_void,
            4 as size_t,
        ) != 4 as ssize_t
    {
        cap = 0 as uint32_t;
    }
    (*t).fix_aux = cap as ::core::ffi::c_long;
    (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_SOCKNAME.0 as ::core::ffi::c_int;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sys_entry(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
) -> ::core::ffi::c_int {
    if (*t).sysno == -1 as ::core::ffi::c_long {
        return 0 as ::core::ffi::c_int;
    }
    return dispatch(t, r);
}
unsafe extern "C" fn dispatch(mut t: *mut eng_task, mut r: *mut eng_regs) -> ::core::ffi::c_int {
    let mut nr: ::core::ffi::c_long = (*t).sysno;
    let mut h: ::core::ffi::c_int = eng_ident_entry(t, r);
    if h >= 0 as ::core::ffi::c_int {
        return h;
    }
    let mut i: size_t = 0 as size_t;
    while i < ::core::mem::size_of::<[pathsys; 5]>().wrapping_div(::core::mem::size_of::<pathsys>())
    {
        if PATHSYS[i].nr == nr {
            return h_pathsys(
                t,
                r,
                (&raw const PATHSYS as *const pathsys).offset(i as isize),
            );
        }
        i = i.wrapping_add(1);
    }
    match nr {
        __NR_openat => {
            return h_open(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
            );
        }
        __NR_newfstatat => {
            return h_stat(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_statx => {
            return h_stat(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                4 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
            );
        }
        __NR_fstat => return h_fstat(t, r),
        __NR_faccessat => {
            return h_access(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
            );
        }
        __NR_faccessat2 => {
            return h_access(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
            );
        }
        __NR_mkdirat => {
            return h_mkdir(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
            );
        }
        __NR_mknodat => {
            return h_mknod(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
            );
        }
        __NR_symlinkat => {
            return h_symlink(t, r, 1 as ::core::ffi::c_int, 2 as ::core::ffi::c_int);
        }
        __NR_unlinkat => {
            return h_unlink(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                (eng_arg(r, 2 as ::core::ffi::c_int) as ::core::ffi::c_long
                    & AT_REMOVEDIR as ::core::ffi::c_long
                    != 0 as ::core::ffi::c_long) as ::core::ffi::c_int,
            );
        }
        __NR_renameat => {
            return h_rename(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
            );
        }
        __NR_renameat2 => {
            return h_rename(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
                4 as ::core::ffi::c_int,
            );
        }
        __NR_linkat => {
            return h_link(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
                4 as ::core::ffi::c_int,
            );
        }
        __NR_fchmod => {
            return h_chmod(
                t,
                r,
                0 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
            );
        }
        __NR_fchmodat => {
            return h_chmod(
                t,
                r,
                -1 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
            );
        }
        __NR_fchmodat2 => {
            return h_chmod(
                t,
                r,
                -1 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
            );
        }
        __NR_fchown => {
            return h_chown(
                t,
                r,
                0 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_fchownat => {
            return h_chown(
                t,
                r,
                -1 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
                4 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_readlinkat => {
            return h_readlink(
                t,
                r,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
            );
        }
        __NR_getcwd => return h_getcwd(t, r),
        221 => return h_exec(t, r, 0 as ::core::ffi::c_int),
        __NR_execveat => return h_exec(t, r, 1 as ::core::ffi::c_int),
        __NR_socket => {
            if eng_arg(r, 0 as ::core::ffi::c_int) as ::core::ffi::c_long
                == 16 as ::core::ffi::c_long
                && eng_arg(r, 2 as ::core::ffi::c_int) as ::core::ffi::c_long
                    == 9 as ::core::ffi::c_long
            {
                return eng_task_void(t, r, -(EPROTONOSUPPORT as ::core::ffi::c_long));
            }
            return 0 as ::core::ffi::c_int;
        }
        __NR_bind => {
            return h_sockaddr(
                t,
                r,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
            );
        }
        __NR_connect => {
            return h_sockaddr(
                t,
                r,
                1 as ::core::ffi::c_int,
                2 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_sendto => {
            return h_sockaddr(
                t,
                r,
                4 as ::core::ffi::c_int,
                5 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_sendmsg => return h_sendmsg(t, r),
        __NR_getsockname => {
            return h_sockname(t, r, 1 as ::core::ffi::c_int, 2 as ::core::ffi::c_int);
        }
        __NR_getpeername => {
            return h_sockname(t, r, 1 as ::core::ffi::c_int, 2 as ::core::ffi::c_int);
        }
        __NR_accept => {
            return h_sockname(t, r, 1 as ::core::ffi::c_int, 2 as ::core::ffi::c_int);
        }
        __NR_accept4 => {
            return h_sockname(t, r, 1 as ::core::ffi::c_int, 2 as ::core::ffi::c_int);
        }
        __NR_getdents64 => {
            (*t).fix_aux = eng_arg(r, 0 as ::core::ffi::c_int) as ::core::ffi::c_long;
            (*t).fix_addr = eng_arg(r, 1 as ::core::ffi::c_int);
            (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_GETDENTS.0 as ::core::ffi::c_int;
            return 0 as ::core::ffi::c_int;
        }
        __NR_getxattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_GET.0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_lgetxattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_GET.0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
            );
        }
        __NR_fgetxattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_GET.0 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_setxattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_SET.0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_lsetxattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_SET.0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
            );
        }
        __NR_fsetxattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_SET.0 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_removexattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_REMOVE.0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_lremovexattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_REMOVE.0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
            );
        }
        __NR_fremovexattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_REMOVE.0 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_listxattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_LIST.0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_llistxattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_LIST.0 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
                1 as ::core::ffi::c_int,
            );
        }
        __NR_flistxattr => {
            return h_xattr(
                t,
                r,
                C2Rust_Unnamed_14::XA_LIST.0 as ::core::ffi::c_int,
                -1 as ::core::ffi::c_int,
                0 as ::core::ffi::c_int,
            );
        }
        __NR_io_uring_setup
        | __NR_io_uring_enter
        | __NR_io_uring_register
        | __NR_openat2
        | __NR_clone3 => return eng_task_void(t, r, -(ENOSYS as ::core::ffi::c_long)),
        __NR_name_to_handle_at => {
            return eng_task_void(t, r, -(EOPNOTSUPP as ::core::ffi::c_long));
        }
        __NR_open_by_handle_at
        | __NR_mount
        | __NR_umount2
        | __NR_pivot_root
        | __NR_chroot
        | __NR_swapon
        | __NR_swapoff
        | __NR_acct
        | __NR_ptrace
        | __NR_fanotify_init
        | __NR_fanotify_mark
        | __NR_open_tree
        | __NR_move_mount
        | __NR_fsopen
        | __NR_fsconfig
        | __NR_fsmount
        | __NR_fspick => return eng_task_void(t, r, -(EPERM as ::core::ffi::c_long)),
        _ => return default_policy(t, r, nr),
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sys_exit(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
) -> ::core::ffi::c_int {
    if eng_ident_exit(t, r) != 0 {
        return 1 as ::core::ffi::c_int;
    }
    let mut ret: ::core::ffi::c_long = eng_ret(r) as ::core::ffi::c_long;
    let mut fix: ::core::ffi::c_int = (*t).fixup;
    (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_NONE.0 as ::core::ffi::c_int;
    if (*t).void_pending != 0 {
        (*t).lg_fix = 0 as ::core::ffi::c_int;
        return 0 as ::core::ffi::c_int;
    }
    return sys_exit_fix(t, fix, ret);
}
unsafe extern "C" fn sys_exit_fix(
    mut t: *mut eng_task,
    mut fix: ::core::ffi::c_int,
    mut ret: ::core::ffi::c_long,
) -> ::core::ffi::c_int {
    let mut g: *mut eng_guest = eng_tracer_guest((*t).tr);
    match fix {
        1 => {
            if ret == 0 as ::core::ffi::c_long {
                x_stat(t);
            }
        }
        2 => {
            if ret == 0 as ::core::ffi::c_long {
                x_statx(t);
            }
        }
        3 => {
            if ret >= 0 as ::core::ffi::c_long {
                let mut p: [::core::ffi::c_char; 64] = [0; 64];
                fd_proc_path(
                    t,
                    ret as ::core::ffi::c_int,
                    &raw mut p as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                );
                let mut m: eng_meta = eng_meta {
                    uid: (*t).cr.fsuid,
                    gid: (*t).fix_aux as uint32_t,
                    mode: (*t).fix_mode,
                    nlink: 0,
                    major: 0,
                    minor: 0,
                    present: 1 as ::core::ffi::c_int,
                };
                eng_meta_write(
                    &raw mut p as *mut ::core::ffi::c_char,
                    0 as ::core::ffi::c_int,
                    &raw mut m,
                );
            }
        }
        4 => {
            if ret == 0 as ::core::ffi::c_long {
                let mut m_0: eng_meta = eng_meta {
                    uid: (*t).cr.fsuid,
                    gid: (*t).fix_aux as uint32_t,
                    mode: (*t).fix_mode,
                    nlink: 0,
                    major: 0,
                    minor: 0,
                    present: 1 as ::core::ffi::c_int,
                };
                eng_meta_write(
                    &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
                    1 as ::core::ffi::c_int,
                    &raw mut m_0,
                );
            }
        }
        5 => {
            if ret == 0 as ::core::ffi::c_long {
                eng_link_drop(g, &raw mut (*t).fix_id as *mut ::core::ffi::c_char);
            }
        }
        6 => {
            if ret == 0 as ::core::ffi::c_long {
                let mut st: stat = stat {
                    st_dev: 0,
                    st_ino: 0,
                    st_mode: 0,
                    st_nlink: 0,
                    st_uid: 0,
                    st_gid: 0,
                    st_rdev: 0,
                    __pad1: 0,
                    st_size: 0,
                    st_blksize: 0,
                    __pad2: 0,
                    st_blocks: 0,
                    st_atim: timespec {
                        tv_sec: 0,
                        tv_nsec: 0,
                    },
                    st_mtim: timespec {
                        tv_sec: 0,
                        tv_nsec: 0,
                    },
                    st_ctim: timespec {
                        tv_sec: 0,
                        tv_nsec: 0,
                    },
                    __unused4: 0,
                    __unused5: 0,
                };
                let mut m_1: eng_meta = eng_meta {
                    uid: 0,
                    gid: 0,
                    mode: 0,
                    nlink: 0,
                    major: 0,
                    minor: 0,
                    present: 0,
                };
                if lstat(
                    &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
                    &raw mut st,
                ) == 0 as ::core::ffi::c_int
                    && !(st.st_mode & S_IFMT as mode_t == S_IFLNK as mode_t)
                    && eng_meta_read(
                        g,
                        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
                        1 as ::core::ffi::c_int,
                        &raw mut st,
                        &raw mut m_1,
                    ) == 0 as ::core::ffi::c_int
                    && m_1.present == 0
                {
                    m_1.uid = (*t).fix_mode;
                    m_1.gid = (*t).fix_aux as uint32_t;
                    m_1.present = 1 as ::core::ffi::c_int;
                    eng_meta_write(
                        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
                        1 as ::core::ffi::c_int,
                        &raw mut m_1,
                    );
                }
            }
        }
        8 => {
            x_getdents(t, ret);
        }
        7 => {
            x_sockname(t, ret);
        }
        9 => {
            x_listxattr(t, ret);
        }
        _ => {}
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sys_sigsys(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut si: *mut siginfo_t,
) -> ::core::ffi::c_int {
    if (*si).c2rust_unnamed.c2rust_unnamed.si_code != 1 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    if (*t).phase.0 == eng_phase::ENG_PH_GUEST.0 && !eng_tracer_guest((*t).tr).is_null() {
        let mut saved: eng_regs = *r;
        let mut save: ::core::ffi::c_long = (*t).sysno;
        (*t).sysno = (*si)
            .c2rust_unnamed
            .c2rust_unnamed
            ._sifields
            ._sigsys
            ._syscall as ::core::ffi::c_long;
        (*t).void_pending = 0 as ::core::ffi::c_int;
        (*t).regs_modified = 0 as ::core::ffi::c_int;
        (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_NONE.0 as ::core::ffi::c_int;
        (*t).slot_off = 0 as uint32_t;
        (*t).stack_scratch = 0 as uint64_t;
        dispatch(t, r);
        let mut emulated: ::core::ffi::c_int = (*t).void_pending;
        let mut res: ::core::ffi::c_long = (*t).inject_result;
        (*t).void_pending = 0 as ::core::ffi::c_int;
        (*t).regs_modified = 0 as ::core::ffi::c_int;
        (*t).fixup = C2Rust_Unnamed_12::ENG_FIX_NONE.0 as ::core::ffi::c_int;
        (*t).umask_old = -1 as ::core::ffi::c_int;
        (*t).sysno = save;
        *r = saved;
        if emulated != 0 {
            eng_set_ret(r, res as uint64_t);
            eng_logf!(
                eng_log_level::ENG_LOG_DEBUG,
                b"SIGSYS tid=%d syscall=%d(%s) -> emulated (%ld)\0".as_ptr()
                    as *const ::core::ffi::c_char,
                (*t).tid,
                (*si)
                    .c2rust_unnamed
                    .c2rust_unnamed
                    ._sifields
                    ._sigsys
                    ._syscall,
                if !eng_sysinv_name(
                    (*si)
                        .c2rust_unnamed
                        .c2rust_unnamed
                        ._sifields
                        ._sigsys
                        ._syscall as ::core::ffi::c_long,
                )
                .is_null()
                {
                    eng_sysinv_name(
                        (*si)
                            .c2rust_unnamed
                            .c2rust_unnamed
                            ._sifields
                            ._sigsys
                            ._syscall as ::core::ffi::c_long,
                    )
                } else {
                    b"?\0".as_ptr() as *const ::core::ffi::c_char
                },
                res,
            );
            return 1 as ::core::ffi::c_int;
        }
    }
    eng_set_ret(r, -ENOSYS as uint64_t);
    eng_logf!(
        eng_log_level::ENG_LOG_DEBUG,
        b"SIGSYS tid=%d syscall=%d(%s) -> ENOSYS\0".as_ptr() as *const ::core::ffi::c_char,
        (*t).tid,
        (*si)
            .c2rust_unnamed
            .c2rust_unnamed
            ._sifields
            ._sigsys
            ._syscall,
        if !eng_sysinv_name(
            (*si)
                .c2rust_unnamed
                .c2rust_unnamed
                ._sifields
                ._sigsys
                ._syscall as ::core::ffi::c_long,
        )
        .is_null()
        {
            eng_sysinv_name(
                (*si)
                    .c2rust_unnamed
                    .c2rust_unnamed
                    ._sifields
                    ._sigsys
                    ._syscall as ::core::ffi::c_long,
            )
        } else {
            b"?\0".as_ptr() as *const ::core::ffi::c_char
        },
    );
    return 1 as ::core::ffi::c_int;
}
