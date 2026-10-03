//! Single-writer ptrace process-tree scheduler, signals, seccomp and exit cleanup.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
#[repr(C)]
pub struct __sFILE {
    _opaque: [u8; 0],
}
use ::libc;
unsafe extern "C" {
    unsafe fn atoi(__s: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    unsafe fn sigaction(
        __signal: ::core::ffi::c_int,
        __new_action: *const sigaction,
        __old_action: *mut sigaction,
    ) -> ::core::ffi::c_int;
    unsafe fn sigemptyset(__set: *mut sigset_t) -> ::core::ffi::c_int;
    unsafe fn sigprocmask(
        __how: ::core::ffi::c_int,
        __new_set: *const sigset_t,
        __old_set: *mut sigset_t,
    ) -> ::core::ffi::c_int;
    unsafe fn kill(__pid: pid_t, __signal: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn eng_regs_get(tid: pid_t, r: *mut eng_regs) -> ::core::ffi::c_int;
    unsafe fn eng_regs_set(tid: pid_t, r: *const eng_regs) -> ::core::ffi::c_int;
    unsafe fn eng_sysno(r: *const eng_regs) -> ::core::ffi::c_long;
    unsafe fn eng_arg(r: *const eng_regs, i: ::core::ffi::c_int) -> uint64_t;
    unsafe fn eng_ret(r: *const eng_regs) -> uint64_t;
    unsafe fn eng_set_ret(r: *mut eng_regs, v: uint64_t);
    unsafe fn eng_pc(r: *const eng_regs) -> uint64_t;
    unsafe fn eng_syscall_void(tid: pid_t, r: *mut eng_regs) -> ::core::ffi::c_int;
    unsafe fn eng_restore_args(cur: *mut eng_regs, orig: *const eng_regs);
    unsafe fn eng_syscall_info_op(tid: pid_t) -> ::core::ffi::c_int;
    unsafe fn eng_sys_entry(t: *mut eng_task, r: *mut eng_regs) -> ::core::ffi::c_int;
    unsafe fn eng_sys_exit(t: *mut eng_task, r: *mut eng_regs) -> ::core::ffi::c_int;
    unsafe fn eng_sys_sigsys(
        t: *mut eng_task,
        r: *mut eng_regs,
        si: *mut siginfo_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_scratch_release(t: *mut eng_task);
    unsafe fn __errno() -> *mut ::core::ffi::c_int;
    static mut stderr: *mut FILE;
    unsafe fn fprintf(
        __fp: *mut FILE,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn sscanf(
        __s: *const ::core::ffi::c_char,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn snprintf(
        __buf: *mut ::core::ffi::c_char,
        __size: size_t,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn calloc(__item_count: size_t, __item_size: size_t) -> *mut ::core::ffi::c_void;
    unsafe fn free(__ptr: *mut ::core::ffi::c_void);
    unsafe fn exit(__status: ::core::ffi::c_int) -> !;
    unsafe fn getenv(__name: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memset(
        __dst: *mut ::core::ffi::c_void,
        __ch: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn strerror(__errno_value: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
    unsafe fn prctl(__op: ::core::ffi::c_int, ...) -> ::core::ffi::c_int;
    unsafe fn umask(__mask: mode_t) -> mode_t;
    unsafe fn ptrace(__op: ::core::ffi::c_int, ...) -> ::core::ffi::c_long;
    unsafe fn waitpid(
        __pid: pid_t,
        __status: *mut ::core::ffi::c_int,
        __options: ::core::ffi::c_int,
    ) -> pid_t;
    unsafe fn _exit(__status: ::core::ffi::c_int) -> !;
    unsafe fn fork() -> pid_t;
    unsafe fn execvpe(
        __file: *const ::core::ffi::c_char,
        __argv: *const *mut ::core::ffi::c_char,
        __envp: *const *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn execve(
        __file: *const ::core::ffi::c_char,
        __argv: *const *mut ::core::ffi::c_char,
        __envp: *const *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn chdir(__path: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    unsafe fn pipe2(
        __fds: *mut ::core::ffi::c_int,
        __flags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn close(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn read(
        __fd: ::core::ffi::c_int,
        __buf: *mut ::core::ffi::c_void,
        __count: size_t,
    ) -> ssize_t;
    unsafe fn write(
        __fd: ::core::ffi::c_int,
        __buf: *const ::core::ffi::c_void,
        __count: size_t,
    ) -> ssize_t;
    unsafe fn alarm(__seconds: ::core::ffi::c_uint) -> ::core::ffi::c_uint;
    unsafe fn eng_exec_prepare_initial(
        t: *mut eng_task,
        path: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_exec_discard(t: *mut eng_task);
    unsafe fn eng_guest_to_host(
        g: *const eng_guest,
        guest: *const ::core::ffi::c_char,
        host: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_ident_init(g: *mut eng_guest, t: *mut eng_task, uid: uint32_t, gid: uint32_t);
    unsafe fn eng_ident_exec(t: *mut eng_task);
    unsafe fn eng_link_recover(g: *mut eng_guest) -> ::core::ffi::c_int;
    unsafe fn eng_log_enabled(lv: eng_log_level) -> ::core::ffi::c_int;
    unsafe fn eng_mem_write(
        tid: pid_t,
        addr: uintptr_t,
        buf: *const ::core::ffi::c_void,
        n: size_t,
    ) -> ssize_t;
    unsafe fn eng_sysinv_name(nr: ::core::ffi::c_long) -> *const ::core::ffi::c_char;
    unsafe fn eng_sysinv_class(nr: ::core::ffi::c_long) -> eng_sc_class;
    unsafe fn eng_sysinv_max() -> ::core::ffi::c_long;
    unsafe fn uname(__buf: *mut utsname) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type uint64_t = u64;
pub type intptr_t = isize;
pub type uintptr_t = usize;
pub type __u8 = ::core::ffi::c_uchar;
pub type __u16 = ::core::ffi::c_ushort;
pub type __u32 = ::core::ffi::c_uint;
pub type __kernel_long_t = ::core::ffi::c_long;
pub type __kernel_mode_t = ::core::ffi::c_uint;
pub type __kernel_pid_t = ::core::ffi::c_int;
pub type __kernel_uid32_t = ::core::ffi::c_uint;
pub type __kernel_clock_t = __kernel_long_t;
pub type __kernel_timer_t = ::core::ffi::c_int;
pub type __pid_t = __kernel_pid_t;
pub type pid_t = __pid_t;
pub type __mode_t = __kernel_mode_t;
pub type mode_t = __mode_t;
pub type ssize_t = isize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sigset_t {
    pub sig: [::core::ffi::c_ulong; 1],
}
pub type __signalfn_t = unsafe extern "C" fn(::core::ffi::c_int) -> ();
pub type __sighandler_t = Option<__signalfn_t>;
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
pub type sig_atomic_t = ::core::ffi::c_int;
pub type sighandler_t = __sighandler_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sigaction {
    pub sa_flags: ::core::ffi::c_int,
    pub c2rust_unnamed: C2Rust_Unnamed_12,
    pub sa_mask: sigset_t,
    pub sa_restorer: Option<unsafe extern "C" fn() -> ()>,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed_12 {
    pub sa_handler: sighandler_t,
    pub sa_sigaction: Option<
        unsafe extern "C" fn(::core::ffi::c_int, *mut siginfo, *mut ::core::ffi::c_void) -> (),
    >,
}
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
pub struct eng_tracer {
    pub buckets: [*mut eng_task; 1024],
    pub ntasks: size_t,
    pub main_pid: pid_t,
    pub main_status: ::core::ffi::c_int,
    pub main_seen: ::core::ffi::c_int,
    pub guest: *mut eng_guest,
    pub sysinfo: ::core::ffi::c_int,
    pub fast: ::core::ffi::c_int,
    pub seccomp_stops: ::core::ffi::c_ulong,
    pub check_toggle: ::core::ffi::c_int,
    pub toggle_mismatch: ::core::ffi::c_ulong,
    pub syscall_stops: ::core::ffi::c_ulong,
}
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
pub struct eng_proc {
    pub refs: ::core::ffi::c_int,
    pub exe: [::core::ffi::c_char; 4096],
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
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_phase(pub ::core::ffi::c_uint);
impl eng_phase {
    pub const ENG_PH_BOOT: Self = Self(0);
    pub const ENG_PH_LOADER: Self = Self(1);
    pub const ENG_PH_GUEST: Self = Self(2);
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct C2Rust_Unnamed_13(pub ::core::ffi::c_uint);
impl C2Rust_Unnamed_13 {
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_run_cfg {
    pub argv: *mut *mut ::core::ffi::c_char,
    pub exe: *const ::core::ffi::c_char,
    pub envp: *mut *mut ::core::ffi::c_char,
    pub cwd: *const ::core::ffi::c_char,
    pub guest: *mut eng_guest,
    pub use_seccomp_fastpath: ::core::ffi::c_int,
    pub uid: uint32_t,
    pub gid: uint32_t,
    pub test_app_filter: ::core::ffi::c_int,
    pub wait_all: ::core::ffi::c_int,
}
pub type FILE = __sFILE;
pub type node = eng_task_node;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_task_node {
    pub t: eng_task,
    pub next: *mut eng_task_node,
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
pub struct sock_fprog {
    pub len: ::core::ffi::c_ushort,
    pub filter: *mut sock_filter,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sock_filter {
    pub code: __u16,
    pub jt: __u8,
    pub jf: __u8,
    pub k: __u32,
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct utsname {
    pub sysname: [::core::ffi::c_char; 65],
    pub nodename: [::core::ffi::c_char; 65],
    pub release: [::core::ffi::c_char; 65],
    pub version: [::core::ffi::c_char; 65],
    pub machine: [::core::ffi::c_char; 65],
    pub domainname: [::core::ffi::c_char; 65],
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const SIGHUP: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SIGINT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SIGQUIT: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const SIGILL: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const SIGTRAP: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const SIGBUS: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const SIGKILL: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const SIGSEGV: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const SIGPIPE: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const SIGALRM: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const SIGTERM: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const SIGSTOP: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const SIGTSTP: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const SIGTTIN: ::core::ffi::c_int = 21 as ::core::ffi::c_int;
pub const SIGTTOU: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const SIGSYS: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const SIG_SETMASK: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SIG_DFL: __sighandler_t = None;
pub const ENOENT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ESRCH: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const EINTR: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const E2BIG: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const ECHILD: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const EFAULT: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const EINVAL: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const ENOSYS: ::core::ffi::c_int = 38 as ::core::ffi::c_int;
pub const O_CLOEXEC: ::core::ffi::c_int = 0o2000000 as ::core::ffi::c_int;
pub const CLONE_VM: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const CLONE_THREAD: ::core::ffi::c_int = 0x10000 as ::core::ffi::c_int;
pub const __WALL: ::core::ffi::c_int = 0x40000000 as ::core::ffi::c_int;
pub const PR_SET_SECCOMP: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const PR_SET_CHILD_SUBREAPER: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const PR_SET_NO_NEW_PRIVS: ::core::ffi::c_int = 38 as ::core::ffi::c_int;
pub const PTRACE_CONT: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const PTRACE_SYSCALL: ::core::ffi::c_int = 24 as ::core::ffi::c_int;
pub const PTRACE_GETEVENTMSG: ::core::ffi::c_int = 0x4201 as ::core::ffi::c_int;
pub const PTRACE_GETSIGINFO: ::core::ffi::c_int = 0x4202 as ::core::ffi::c_int;
pub const PTRACE_SEIZE: ::core::ffi::c_int = 0x4206 as ::core::ffi::c_int;
pub const PTRACE_INTERRUPT: ::core::ffi::c_int = 0x4207 as ::core::ffi::c_int;
pub const PTRACE_LISTEN: ::core::ffi::c_int = 0x4208 as ::core::ffi::c_int;
pub const PTRACE_EVENT_FORK: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PTRACE_EVENT_VFORK: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PTRACE_EVENT_CLONE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const PTRACE_EVENT_EXEC: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PTRACE_EVENT_EXIT: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const PTRACE_EVENT_SECCOMP: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const PTRACE_EVENT_STOP: ::core::ffi::c_uint = 128 as ::core::ffi::c_uint;
pub const PTRACE_O_TRACESYSGOOD: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PTRACE_O_TRACEFORK: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << PTRACE_EVENT_FORK;
pub const PTRACE_O_TRACEVFORK: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << PTRACE_EVENT_VFORK;
pub const PTRACE_O_TRACECLONE: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << PTRACE_EVENT_CLONE;
pub const PTRACE_O_TRACEEXEC: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << PTRACE_EVENT_EXEC;
pub const PTRACE_O_TRACEEXIT: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << PTRACE_EVENT_EXIT;
pub const PTRACE_O_TRACESECCOMP: ::core::ffi::c_int =
    (1 as ::core::ffi::c_int) << PTRACE_EVENT_SECCOMP;
pub const PTRACE_O_EXITKILL: ::core::ffi::c_int =
    (1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int;
pub const __NR_umask: ::core::ffi::c_int = 166 as ::core::ffi::c_int;
pub const __NR_prctl: ::core::ffi::c_int = 167 as ::core::ffi::c_int;
pub const __NR_getpid: ::core::ffi::c_int = 172 as ::core::ffi::c_int;
pub const __NR_socket: ::core::ffi::c_int = 198 as ::core::ffi::c_int;
pub const __NR_recvfrom: ::core::ffi::c_int = 207 as ::core::ffi::c_int;
pub const __NR_recvmsg: ::core::ffi::c_int = 212 as ::core::ffi::c_int;
pub const __NR_execve: ::core::ffi::c_int = 221 as ::core::ffi::c_int;
pub const __NR_recvmmsg: ::core::ffi::c_int = 243 as ::core::ffi::c_int;
pub const __NR_execveat: ::core::ffi::c_int = 281 as ::core::ffi::c_int;
pub const ENG_MARK_A: ::core::ffi::c_ulonglong = 0x574f524b464c4f57 as ::core::ffi::c_ulonglong;
pub const ENG_MARK_B: ::core::ffi::c_ulonglong = 0x4c4f414445523031 as ::core::ffi::c_ulonglong;
pub const ENG_MARK_OP_QUERY: ::core::ffi::c_uint = 1 as ::core::ffi::c_uint;
pub const ENG_MARK_OP_DONE: ::core::ffi::c_uint = 2 as ::core::ffi::c_uint;
pub const ENG_SLOT_SIZE: ::core::ffi::c_uint =
    (16 as ::core::ffi::c_uint).wrapping_mul(1024 as ::core::ffi::c_uint);
pub const SECCOMP_MODE_FILTER: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const SECCOMP_RET_ERRNO: ::core::ffi::c_uint = 0x50000 as ::core::ffi::c_uint;
pub const SECCOMP_RET_TRACE: ::core::ffi::c_uint = 0x7ff00000 as ::core::ffi::c_uint;
pub const SECCOMP_RET_ALLOW: ::core::ffi::c_uint = 0x7fff0000 as ::core::ffi::c_uint;
pub const NBUCKETS: ::core::ffi::c_int = 1024 as ::core::ffi::c_int;
unsafe extern "C" fn bucket(mut tid: pid_t) -> ::core::ffi::c_uint {
    return (tid as ::core::ffi::c_uint).wrapping_rem(NBUCKETS as ::core::ffi::c_uint);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_task_find(mut tr: *mut eng_tracer, mut tid: pid_t) -> *mut eng_task {
    let mut n: *mut node = (*tr).buckets[bucket(tid) as usize] as *mut node;
    while !n.is_null() {
        if (*n).t.tid == tid {
            return &raw mut (*n).t;
        }
        n = (*n).next as *mut node;
    }
    return ::core::ptr::null_mut::<eng_task>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_tracer_guest(mut tr: *mut eng_tracer) -> *mut eng_guest {
    return if !tr.is_null() {
        (*tr).guest
    } else {
        ::core::ptr::null_mut::<eng_guest>()
    };
}
unsafe extern "C" fn task_new(mut tr: *mut eng_tracer, mut tid: pid_t) -> *mut eng_task {
    let mut n: *mut node = calloc(1 as size_t, ::core::mem::size_of::<node>()) as *mut node;
    if n.is_null() {
        eng_logf!(
            eng_log_level::ENG_LOG_ERROR,
            b"out of memory\0".as_ptr() as *const ::core::ffi::c_char,
        );
        exit(125 as ::core::ffi::c_int);
    }
    (*n).t.tid = tid;
    (*n).t.tgid = tid;
    (*n).t.in_use = 1 as ::core::ffi::c_int;
    (*n).t.at_syscall_entry = 1 as ::core::ffi::c_int;
    (*n).t.slot = -1 as ::core::ffi::c_int;
    (*n).t.umask_old = -1 as ::core::ffi::c_int;
    (*n).t.tr = tr;
    (*n).next = (*tr).buckets[bucket(tid) as usize] as *mut node as *mut eng_task_node;
    (*tr).buckets[bucket(tid) as usize] = &raw mut (*n).t;
    (*tr).ntasks = (*tr).ntasks.wrapping_add(1);
    return &raw mut (*n).t;
}
unsafe extern "C" fn mm_put(mut mm: *mut eng_mm) {
    if !mm.is_null() && {
        (*mm).refs -= 1;
        (*mm).refs <= 0 as ::core::ffi::c_int
    } {
        free(mm as *mut ::core::ffi::c_void);
    }
}
unsafe extern "C" fn proc_put(mut p: *mut eng_proc) {
    if !p.is_null() && {
        (*p).refs -= 1;
        (*p).refs <= 0 as ::core::ffi::c_int
    } {
        free(p as *mut ::core::ffi::c_void);
    }
}
unsafe extern "C" fn task_del(mut tr: *mut eng_tracer, mut tid: pid_t) {
    let mut pp: *mut *mut node = (&raw mut (*tr).buckets as *mut *mut eng_task)
        .offset(bucket(tid) as isize) as *mut *mut node;
    while !(*pp).is_null() {
        if (**pp).t.tid != tid {
            pp = &raw mut (**pp).next as *mut *mut node;
        } else {
            let mut n: *mut node = *pp;
            *pp = (*n).next as *mut node;
            eng_scratch_release(&raw mut (*n).t);
            mm_put((*n).t.mm);
            proc_put((*n).t.proc);
            free((*n).t.plan);
            free(n as *mut ::core::ffi::c_void);
            (*tr).ntasks = (*tr).ntasks.wrapping_sub(1);
            return;
        }
    }
}
unsafe extern "C" fn proc_new(mut exe: *const ::core::ffi::c_char) -> *mut eng_proc {
    let mut p: *mut eng_proc =
        calloc(1 as size_t, ::core::mem::size_of::<eng_proc>()) as *mut eng_proc;
    (*p).refs = 1 as ::core::ffi::c_int;
    if !exe.is_null() {
        snprintf(
            &raw mut (*p).exe as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            exe,
        );
    }
    return p;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_task_void(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut result: ::core::ffi::c_long,
) -> ::core::ffi::c_int {
    if eng_syscall_void((*t).tid, r) != 0 as ::core::ffi::c_int && *__errno() != ESRCH {
        eng_logf!(
            eng_log_level::ENG_LOG_WARN,
            b"void syscall tid=%d: %s\0".as_ptr() as *const ::core::ffi::c_char,
            (*t).tid,
            strerror(*__errno()),
        );
    }
    (*t).void_pending = 1 as ::core::ffi::c_int;
    (*t).inject_result = result;
    return 0 as ::core::ffi::c_int;
}
static mut g_fast: ::core::ffi::c_int = 0;
unsafe extern "C" fn resume_mode(
    mut tid: pid_t,
    mut sig: ::core::ffi::c_int,
    mut want_syscall: ::core::ffi::c_int,
) {
    let mut req: ::core::ffi::c_int = if g_fast == 0 || want_syscall != 0 {
        PTRACE_SYSCALL
    } else {
        PTRACE_CONT
    };
    if ptrace(
        req,
        tid,
        0 as ::core::ffi::c_int,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(sig as intptr_t as usize),
    ) != 0 as ::core::ffi::c_long
        && *__errno() != ESRCH
    {
        eng_logf!(
            eng_log_level::ENG_LOG_DEBUG,
            b"resume tid=%d: %s\0".as_ptr() as *const ::core::ffi::c_char,
            tid,
            strerror(*__errno()),
        );
    }
}
unsafe extern "C" fn resume_task(mut t: *mut eng_task, mut sig: ::core::ffi::c_int) {
    resume_mode((*t).tid, sig, (*t).await_exit);
}
static mut g_stop_sig: sig_atomic_t = 0;
static mut g_alarm: sig_atomic_t = 0;
unsafe extern "C" fn on_stop_signal(mut s: ::core::ffi::c_int) {
    ::core::ptr::write_volatile(&raw mut g_stop_sig, s as sig_atomic_t);
}
unsafe extern "C" fn on_alarm(mut s: ::core::ffi::c_int) {
    ::core::ptr::write_volatile(&raw mut g_alarm, 1 as ::core::ffi::c_int as sig_atomic_t);
}
unsafe extern "C" fn install_engine_signals() {
    let mut sa: sigaction = sigaction {
        sa_flags: 0,
        c2rust_unnamed: C2Rust_Unnamed_12 { sa_handler: None },
        sa_mask: sigset_t { sig: [0; 1] },
        sa_restorer: None,
    };
    memset(
        &raw mut sa as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<sigaction>(),
    );
    sa.c2rust_unnamed.sa_handler = ::core::mem::transmute::<::libc::intptr_t, __sighandler_t>(
        1 as ::core::ffi::c_int as ::libc::intptr_t,
    ) as sighandler_t;
    sigaction(SIGINT, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
    sigaction(SIGQUIT, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
    sigaction(SIGHUP, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
    sigaction(SIGPIPE, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
    sa.c2rust_unnamed.sa_handler =
        Some(on_stop_signal as unsafe extern "C" fn(::core::ffi::c_int) -> ()) as sighandler_t;
    sigaction(SIGTERM, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
    sa.c2rust_unnamed.sa_handler =
        Some(on_alarm as unsafe extern "C" fn(::core::ffi::c_int) -> ()) as sighandler_t;
    sigaction(SIGALRM, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
}
unsafe extern "C" fn reset_child_signals() {
    let mut sa: sigaction = sigaction {
        sa_flags: 0,
        c2rust_unnamed: C2Rust_Unnamed_12 { sa_handler: None },
        sa_mask: sigset_t { sig: [0; 1] },
        sa_restorer: None,
    };
    memset(
        &raw mut sa as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<sigaction>(),
    );
    sa.c2rust_unnamed.sa_handler = SIG_DFL as sighandler_t;
    let mut sigs: [::core::ffi::c_int; 6] = [SIGINT, SIGQUIT, SIGHUP, SIGPIPE, SIGTERM, SIGALRM];
    let mut i: size_t = 0 as size_t;
    while i < ::core::mem::size_of::<[::core::ffi::c_int; 6]>()
        .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>())
    {
        sigaction(sigs[i], &raw mut sa, ::core::ptr::null_mut::<sigaction>());
        i = i.wrapping_add(1);
    }
    let mut none: sigset_t = sigset_t { sig: [0; 1] };
    sigemptyset(&raw mut none);
    sigprocmask(
        SIG_SETMASK,
        &raw mut none,
        ::core::ptr::null_mut::<sigset_t>(),
    );
}
unsafe extern "C" fn signal_all(mut tr: *mut eng_tracer, mut sig: ::core::ffi::c_int) {
    let mut b: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while b < NBUCKETS as ::core::ffi::c_uint {
        let mut n: *mut node = (*tr).buckets[b as usize] as *mut node;
        while !n.is_null() {
            if (*n).t.tid == (*n).t.tgid {
                kill((*n).t.tgid, sig);
            }
            n = (*n).next as *mut node;
        }
        b = b.wrapping_add(1);
    }
}
unsafe extern "C" fn is_marker(mut r: *const eng_regs) -> ::core::ffi::c_int {
    return (eng_sysno(r) == __NR_getpid as ::core::ffi::c_long
        && eng_arg(r, 0 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong == ENG_MARK_A
        && eng_arg(r, 1 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong == ENG_MARK_B)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn handle_marker(mut t: *mut eng_task, mut r: *mut eng_regs) {
    let mut op: uint64_t = eng_arg(r, 2 as ::core::ffi::c_int);
    if op == ENG_MARK_OP_QUERY as uint64_t {
        if (*t).plan.is_null() {
            eng_task_void(t, r, -(ENOENT as ::core::ffi::c_long));
            return;
        }
        if eng_arg(r, 4 as ::core::ffi::c_int) < (*t).plan_len as uint64_t {
            eng_task_void(t, r, -(E2BIG as ::core::ffi::c_long));
            return;
        }
        if eng_mem_write(
            (*t).tid,
            eng_arg(r, 3 as ::core::ffi::c_int) as uintptr_t,
            (*t).plan,
            (*t).plan_len as size_t,
        ) != (*t).plan_len as ssize_t
        {
            eng_task_void(t, r, -(EFAULT as ::core::ffi::c_long));
            return;
        }
        let mut len: ::core::ffi::c_long = (*t).plan_len as ::core::ffi::c_long;
        eng_exec_discard(t);
        eng_task_void(t, r, len);
    } else if op == ENG_MARK_OP_DONE as uint64_t {
        eng_scratch_release(t);
        mm_put((*t).mm);
        let mut mm: *mut eng_mm =
            calloc(1 as size_t, ::core::mem::size_of::<eng_mm>()) as *mut eng_mm;
        (*mm).refs = 1 as ::core::ffi::c_int;
        (*mm).base = eng_arg(r, 3 as ::core::ffi::c_int);
        (*mm).size = eng_arg(r, 4 as ::core::ffi::c_int) as uint32_t;
        (*mm).nslots = (*mm).size.wrapping_div(ENG_SLOT_SIZE as uint32_t);
        if (*mm).nslots as usize > ::core::mem::size_of::<[uint8_t; 512]>() {
            (*mm).nslots = ::core::mem::size_of::<[uint8_t; 512]>() as uint32_t;
        }
        (*t).mm = mm;
        proc_put((*t).proc);
        (*t).proc = proc_new(&raw mut (*t).plan_exe as *mut ::core::ffi::c_char);
        (*t).phase = eng_phase::ENG_PH_GUEST;
        eng_logf!(
            eng_log_level::ENG_LOG_DEBUG,
            b"tid=%d guest phase, exe=%s scratch=%#llx\0".as_ptr() as *const ::core::ffi::c_char,
            (*t).tid,
            &raw mut (*t).plan_exe as *mut ::core::ffi::c_char,
            (*mm).base as ::core::ffi::c_ulonglong,
        );
        eng_task_void(t, r, 0 as ::core::ffi::c_long);
    } else {
        eng_task_void(t, r, -(EINVAL as ::core::ffi::c_long));
    };
}
unsafe extern "C" fn syscall_stop(
    mut tr: *mut eng_tracer,
    mut t: *mut eng_task,
    mut seccomp_event: ::core::ffi::c_int,
) {
    let mut r: eng_regs = eng_regs {
        regs: [0; 31],
        sp: 0,
        pc: 0,
        pstate: 0,
    };
    if eng_regs_get((*t).tid, &raw mut r) != 0 as ::core::ffi::c_int {
        resume_mode((*t).tid, 0 as ::core::ffi::c_int, 0 as ::core::ffi::c_int);
        return;
    }
    (*tr).syscall_stops = (*tr).syscall_stops.wrapping_add(1);
    let mut entry: ::core::ffi::c_int = (*t).at_syscall_entry;
    if seccomp_event != 0 {
        (*tr).seccomp_stops = (*tr).seccomp_stops.wrapping_add(1);
        if (*t).at_syscall_entry == 0 {
            resume_task(t, 0 as ::core::ffi::c_int);
            return;
        }
        entry = 1 as ::core::ffi::c_int;
    }
    if (*tr).sysinfo != 0 as ::core::ffi::c_int && seccomp_event == 0 {
        let mut op: ::core::ffi::c_int = eng_syscall_info_op((*t).tid);
        if op < 0 as ::core::ffi::c_int {
            (*tr).sysinfo = 0 as ::core::ffi::c_int;
        } else {
            (*tr).sysinfo = 1 as ::core::ffi::c_int;
            let mut k_entry: ::core::ffi::c_int =
                (op == 1 as ::core::ffi::c_int) as ::core::ffi::c_int;
            if op == 1 as ::core::ffi::c_int || op == 2 as ::core::ffi::c_int {
                if k_entry != entry {
                    (*tr).toggle_mismatch = (*tr).toggle_mismatch.wrapping_add(1);
                    eng_logf!(
                        eng_log_level::ENG_LOG_WARN,
                        b"entry/exit toggle mismatch tid=%d nr=%ld (toggle %s, kernel %s)\0"
                            .as_ptr() as *const ::core::ffi::c_char,
                        (*t).tid,
                        eng_sysno(&raw mut r),
                        if entry != 0 {
                            b"entry\0".as_ptr() as *const ::core::ffi::c_char
                        } else {
                            b"exit\0".as_ptr() as *const ::core::ffi::c_char
                        },
                        if k_entry != 0 {
                            b"entry\0".as_ptr() as *const ::core::ffi::c_char
                        } else {
                            b"exit\0".as_ptr() as *const ::core::ffi::c_char
                        },
                    );
                }
                if (*tr).check_toggle == 0 {
                    entry = k_entry;
                }
            }
        }
    }
    if entry != 0 {
        (*t).at_syscall_entry = 0 as ::core::ffi::c_int;
        (*t).sysno = eng_sysno(&raw mut r);
        (*t).entry_regs = r;
        (*t).regs_modified = 0 as ::core::ffi::c_int;
        (*t).void_pending = 0 as ::core::ffi::c_int;
        (*t).fixup = C2Rust_Unnamed_13::ENG_FIX_NONE.0 as ::core::ffi::c_int;
        if (*t).lg_restart != 0 && (*t).sysno != (*t).lg_nr {
            (*t).lg_restart = 0 as ::core::ffi::c_int;
        }
        if (*t).lg_restart == 0 {
            (*t).slot_off = 0 as uint32_t;
            (*t).stack_scratch = 0 as uint64_t;
            (*t).lg_fix = 0 as ::core::ffi::c_int;
        }
        let mut changed: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        if (*t).phase.0 == eng_phase::ENG_PH_LOADER.0 && is_marker(&raw mut r) != 0 {
            handle_marker(t, &raw mut r);
        } else if (*t).phase.0 == eng_phase::ENG_PH_GUEST.0 && !(*tr).guest.is_null() {
            changed = eng_sys_entry(t, &raw mut r);
        }
        if changed != 0 {
            eng_regs_set((*t).tid, &raw mut r);
        }
        (*t).await_exit = ((*tr).fast == 0
            || (*t).void_pending != 0
            || (*t).regs_modified != 0
            || (*t).fixup != 0
            || (*t).lg_fix != 0
            || !(*t).plan.is_null()
            || (*t).umask_old >= 0 as ::core::ffi::c_int
            || (*t).lg_restart != 0
            || (*t).sysno == __NR_execve as ::core::ffi::c_long
            || (*t).sysno == __NR_execveat as ::core::ffi::c_long)
            as ::core::ffi::c_int;
        if (*t).await_exit == 0 {
            (*t).at_syscall_entry = 1 as ::core::ffi::c_int;
        }
    } else {
        (*t).at_syscall_entry = 1 as ::core::ffi::c_int;
        (*t).await_exit = 0 as ::core::ffi::c_int;
        let mut changed_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        if (*t).phase.0 == eng_phase::ENG_PH_GUEST.0 && !(*tr).guest.is_null() {
            changed_0 = eng_sys_exit(t, &raw mut r);
        }
        if (*t).void_pending != 0 {
            eng_set_ret(&raw mut r, (*t).inject_result as uint64_t);
            (*t).void_pending = 0 as ::core::ffi::c_int;
            changed_0 = 1 as ::core::ffi::c_int;
        }
        if (*t).regs_modified != 0 {
            eng_restore_args(&raw mut r, &raw mut (*t).entry_regs);
            (*t).regs_modified = 0 as ::core::ffi::c_int;
            changed_0 = 1 as ::core::ffi::c_int;
        }
        if (*t).lg_restart != 0 {
            eng_restore_args(&raw mut r, &raw mut (*t).lg_saved);
            (*t).lg_restart = 0 as ::core::ffi::c_int;
            changed_0 = 1 as ::core::ffi::c_int;
        }
        if !(*t).plan.is_null()
            && (eng_ret(&raw mut r) as ::core::ffi::c_long) < 0 as ::core::ffi::c_long
            && ((*t).sysno == __NR_execve as ::core::ffi::c_long
                || (*t).sysno == __NR_execveat as ::core::ffi::c_long)
        {
            eng_exec_discard(t);
        }
        if eng_log_enabled(eng_log_level::ENG_LOG_TRACE) != 0 {
            let mut nm: *const ::core::ffi::c_char = eng_sysinv_name((*t).sysno);
            eng_logf!(
                eng_log_level::ENG_LOG_TRACE,
                b"tid=%d %s(%#llx, %#llx, %#llx, %#llx) = %lld\0".as_ptr()
                    as *const ::core::ffi::c_char,
                (*t).tid,
                if !nm.is_null() {
                    nm
                } else {
                    b"?\0".as_ptr() as *const ::core::ffi::c_char
                },
                eng_arg(&raw mut (*t).entry_regs, 0 as ::core::ffi::c_int)
                    as ::core::ffi::c_ulonglong,
                eng_arg(&raw mut (*t).entry_regs, 1 as ::core::ffi::c_int)
                    as ::core::ffi::c_ulonglong,
                eng_arg(&raw mut (*t).entry_regs, 2 as ::core::ffi::c_int)
                    as ::core::ffi::c_ulonglong,
                eng_arg(&raw mut (*t).entry_regs, 3 as ::core::ffi::c_int)
                    as ::core::ffi::c_ulonglong,
                eng_ret(&raw mut r) as ::core::ffi::c_longlong,
            );
        }
        if changed_0 != 0 {
            eng_regs_set((*t).tid, &raw mut r);
        }
    }
    resume_task(t, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn sigsys_stop(mut t: *mut eng_task) {
    let mut si: siginfo_t = siginfo_t {
        c2rust_unnamed: C2Rust_Unnamed_10 {
            c2rust_unnamed: C2Rust_Unnamed_11 {
                si_signo: 0,
                si_errno: 0,
                si_code: 0,
                _sifields: __sifields {
                    _kill: C2Rust_Unnamed_9 { _pid: 0, _uid: 0 },
                },
            },
        },
    };
    let mut r: eng_regs = eng_regs {
        regs: [0; 31],
        sp: 0,
        pc: 0,
        pstate: 0,
    };
    if ptrace(
        PTRACE_GETSIGINFO,
        (*t).tid,
        0 as ::core::ffi::c_int,
        &raw mut si,
    ) != 0 as ::core::ffi::c_long
        || eng_regs_get((*t).tid, &raw mut r) != 0 as ::core::ffi::c_int
    {
        resume_task(t, SIGSYS);
        return;
    }
    if eng_sys_sigsys(t, &raw mut r, &raw mut si) != 0 {
        eng_regs_set((*t).tid, &raw mut r);
        if (*t).lg_restart != 0 {
            (*t).at_syscall_entry = 1 as ::core::ffi::c_int;
            (*t).await_exit = 1 as ::core::ffi::c_int;
        }
        resume_task(t, 0 as ::core::ffi::c_int);
    } else {
        resume_task(t, SIGSYS);
    };
}
unsafe extern "C" fn on_new_child(
    mut tr: *mut eng_tracer,
    mut parent: *mut eng_task,
    mut ctid: pid_t,
    mut event: ::core::ffi::c_int,
) {
    let mut c: *mut eng_task = eng_task_find(tr, ctid);
    if c.is_null() {
        c = task_new(tr, ctid);
    }
    (*c).linked = 1 as ::core::ffi::c_int;
    let mut flags: ::core::ffi::c_ulong = 0 as ::core::ffi::c_ulong;
    if event == PTRACE_EVENT_CLONE {
        let mut pr: eng_regs = eng_regs {
            regs: [0; 31],
            sp: 0,
            pc: 0,
            pstate: 0,
        };
        if eng_regs_get((*parent).tid, &raw mut pr) == 0 as ::core::ffi::c_int {
            flags = eng_arg(&raw mut pr, 0 as ::core::ffi::c_int) as ::core::ffi::c_ulong;
        }
    }
    let mut thread: ::core::ffi::c_int = (event == PTRACE_EVENT_CLONE
        && flags & CLONE_THREAD as ::core::ffi::c_ulong != 0)
        as ::core::ffi::c_int;
    let mut share_vm: ::core::ffi::c_int = (event == PTRACE_EVENT_VFORK
        || event == PTRACE_EVENT_CLONE && flags & CLONE_VM as ::core::ffi::c_ulong != 0)
        as ::core::ffi::c_int;
    (*c).tgid = if thread != 0 { (*parent).tgid } else { ctid };
    (*c).phase = (*parent).phase;
    (*c).cr = (*parent).cr;
    if !(*parent).mm.is_null() {
        if share_vm != 0 {
            (*c).mm = (*parent).mm;
            (*(*c).mm).refs += 1;
        } else {
            (*c).mm = calloc(1 as size_t, ::core::mem::size_of::<eng_mm>()) as *mut eng_mm;
            (*(*c).mm).refs = 1 as ::core::ffi::c_int;
            (*(*c).mm).base = (*(*parent).mm).base;
            (*(*c).mm).size = (*(*parent).mm).size;
            (*(*c).mm).nslots = (*(*parent).mm).nslots;
        }
    }
    if !(*parent).proc.is_null() {
        if thread != 0 {
            (*c).proc = (*parent).proc;
            (*(*c).proc).refs += 1;
        } else {
            (*c).proc = proc_new(&raw mut (*(*parent).proc).exe as *mut ::core::ffi::c_char);
        }
    }
    if (*c).held != 0 {
        (*c).held = 0 as ::core::ffi::c_int;
        resume_mode(ctid, 0 as ::core::ffi::c_int, 0 as ::core::ffi::c_int);
    }
}
unsafe extern "C" fn on_exec(mut tr: *mut eng_tracer, mut t: *mut eng_task) {
    let mut former: ::core::ffi::c_ulong = 0 as ::core::ffi::c_ulong;
    ptrace(
        PTRACE_GETEVENTMSG,
        (*t).tid,
        0 as ::core::ffi::c_int,
        &raw mut former,
    );
    if former != 0 && former as pid_t != (*t).tid {
        let mut o: *mut eng_task = eng_task_find(tr, former as pid_t);
        if !o.is_null() {
            free((*t).plan);
            (*t).plan = (*o).plan;
            (*o).plan = NULL;
            (*t).plan_len = (*o).plan_len;
            memcpy(
                &raw mut (*t).plan_exe as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                &raw mut (*o).plan_exe as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            );
            (*t).cr = (*o).cr;
            (*t).plan_setid = (*o).plan_setid;
            (*t).plan_euid = (*o).plan_euid;
            (*t).plan_egid = (*o).plan_egid;
            (*t).at_syscall_entry = (*o).at_syscall_entry;
            task_del(tr, former as pid_t);
        }
    }
    eng_scratch_release(t);
    mm_put((*t).mm);
    (*t).mm = ::core::ptr::null_mut::<eng_mm>();
    (*t).regs_modified = 0 as ::core::ffi::c_int;
    (*t).void_pending = 0 as ::core::ffi::c_int;
    (*t).phase = eng_phase(
        (if !(*tr).guest.is_null() {
            eng_phase::ENG_PH_LOADER.0 as ::core::ffi::c_int
        } else {
            eng_phase::ENG_PH_BOOT.0 as ::core::ffi::c_int
        }) as ::core::ffi::c_uint,
    );
    if !(*tr).guest.is_null() {
        eng_ident_exec(t);
    }
}
unsafe extern "C" fn never_allow(mut nr: ::core::ffi::c_long) -> ::core::ffi::c_int {
    static mut list: [::core::ffi::c_long; 4] = [
        __NR_getpid as ::core::ffi::c_long,
        __NR_prctl as ::core::ffi::c_long,
        __NR_socket as ::core::ffi::c_long,
        __NR_umask as ::core::ffi::c_long,
    ];
    let mut i: size_t = 0 as size_t;
    while i < ::core::mem::size_of::<[::core::ffi::c_long; 4]>()
        .wrapping_div(::core::mem::size_of::<::core::ffi::c_long>())
    {
        if list[i] == nr {
            return 1 as ::core::ffi::c_int;
        }
        i = i.wrapping_add(1);
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn fast_allowed(mut nr: ::core::ffi::c_long) -> ::core::ffi::c_int {
    if never_allow(nr) != 0 {
        return 0 as ::core::ffi::c_int;
    }
    match eng_sysinv_class(nr) {
        eng_sc_class::ENG_SC_PASS | eng_sc_class::ENG_SC_FD | eng_sc_class::ENG_SC_PROC => {
            return 1 as ::core::ffi::c_int;
        }
        _ => {}
    }
    return (nr == __NR_recvfrom as ::core::ffi::c_long
        || nr == __NR_recvmsg as ::core::ffi::c_long
        || nr == __NR_recvmmsg as ::core::ffi::c_long) as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_fast_filter_count() -> ::core::ffi::c_int {
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut nr: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
    while nr <= eng_sysinv_max() {
        n += fast_allowed(nr);
        nr += 1;
    }
    return n;
}
unsafe extern "C" fn install_fast_filter() -> ::core::ffi::c_int {
    let mut f: [sock_filter; 1024] = [sock_filter {
        code: 0,
        jt: 0,
        jf: 0,
        k: 0,
    }; 1024];
    let mut n: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let TRACE: uint32_t = SECCOMP_RET_TRACE as uint32_t;
    let ALLOW: uint32_t = SECCOMP_RET_ALLOW as uint32_t;
    let NOSYS: uint32_t = SECCOMP_RET_ERRNO as uint32_t | ENOSYS as uint32_t;
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh0 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh0 as usize] = sock_filter {
        code: (0 as ::core::ffi::c_int | 0 as ::core::ffi::c_int | 0x20 as ::core::ffi::c_int)
            as __u16,
        jt: 0 as __u8,
        jf: 0 as __u8,
        k: 4 as ::core::ffi::c_ulong as __u32,
    };
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh1 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh1 as usize] = sock_filter {
        code: (0x5 as ::core::ffi::c_int | 0x10 as ::core::ffi::c_int | 0 as ::core::ffi::c_int)
            as __u16,
        jt: 1 as __u8,
        jf: 0 as __u8,
        k: 183 as __u32 | 0x80000000 as __u32 | 0x40000000 as ::core::ffi::c_int as __u32,
    };
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh2 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh2 as usize] = sock_filter {
        code: (0x6 as ::core::ffi::c_int | 0 as ::core::ffi::c_int) as __u16,
        jt: 0 as __u8,
        jf: 0 as __u8,
        k: NOSYS as __u32,
    };
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh3 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh3 as usize] = sock_filter {
        code: (0 as ::core::ffi::c_int | 0 as ::core::ffi::c_int | 0x20 as ::core::ffi::c_int)
            as __u16,
        jt: 0 as __u8,
        jf: 0 as __u8,
        k: 0 as ::core::ffi::c_ulong as __u32,
    };
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh4 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh4 as usize] = sock_filter {
        code: (0x5 as ::core::ffi::c_int | 0x10 as ::core::ffi::c_int | 0 as ::core::ffi::c_int)
            as __u16,
        jt: 0 as __u8,
        jf: 6 as __u8,
        k: 172 as __u32,
    };
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh5 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh5 as usize] = sock_filter {
        code: (0 as ::core::ffi::c_int | 0 as ::core::ffi::c_int | 0x20 as ::core::ffi::c_int)
            as __u16,
        jt: 0 as __u8,
        jf: 0 as __u8,
        k: 16 as ::core::ffi::c_ulong as __u32,
    };
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh6 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh6 as usize] = sock_filter {
        code: (0x5 as ::core::ffi::c_int | 0x10 as ::core::ffi::c_int | 0 as ::core::ffi::c_int)
            as __u16,
        jt: 0 as __u8,
        jf: 3 as __u8,
        k: 0x574f524b464c4f57 as ::core::ffi::c_ulonglong as __u32,
    };
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh7 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh7 as usize] = sock_filter {
        code: (0 as ::core::ffi::c_int | 0 as ::core::ffi::c_int | 0x20 as ::core::ffi::c_int)
            as __u16,
        jt: 0 as __u8,
        jf: 0 as __u8,
        k: (16 as ::core::ffi::c_ulong).wrapping_add(4 as ::core::ffi::c_ulong) as __u32,
    };
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh8 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh8 as usize] = sock_filter {
        code: (0x5 as ::core::ffi::c_int | 0x10 as ::core::ffi::c_int | 0 as ::core::ffi::c_int)
            as __u16,
        jt: 0 as __u8,
        jf: 1 as __u8,
        k: (0x574f524b464c4f57 as ::core::ffi::c_ulonglong >> 32 as ::core::ffi::c_int) as __u32,
    };
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh9 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh9 as usize] = sock_filter {
        code: (0x6 as ::core::ffi::c_int | 0 as ::core::ffi::c_int) as __u16,
        jt: 0 as __u8,
        jf: 0 as __u8,
        k: TRACE as __u32,
    };
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh10 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh10 as usize] = sock_filter {
        code: (0x6 as ::core::ffi::c_int | 0 as ::core::ffi::c_int) as __u16,
        jt: 0 as __u8,
        jf: 0 as __u8,
        k: ALLOW as __u32,
    };
    let mut nr: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
    while nr <= eng_sysinv_max() {
        if fast_allowed(nr) != 0 {
            if n as usize
                >= ::core::mem::size_of::<[sock_filter; 1024]>()
                    .wrapping_div(::core::mem::size_of::<sock_filter>())
            {
                return -E2BIG;
            }
            let c2rust_fresh11 = n;
            n = n.wrapping_add(1);
            f[c2rust_fresh11 as usize] = sock_filter {
                code: (0x5 as ::core::ffi::c_int
                    | 0x10 as ::core::ffi::c_int
                    | 0 as ::core::ffi::c_int) as __u16,
                jt: 0 as __u8,
                jf: 1 as __u8,
                k: nr as __u32,
            };
            if n as usize
                >= ::core::mem::size_of::<[sock_filter; 1024]>()
                    .wrapping_div(::core::mem::size_of::<sock_filter>())
            {
                return -E2BIG;
            }
            let c2rust_fresh12 = n;
            n = n.wrapping_add(1);
            f[c2rust_fresh12 as usize] = sock_filter {
                code: (0x6 as ::core::ffi::c_int | 0 as ::core::ffi::c_int) as __u16,
                jt: 0 as __u8,
                jf: 0 as __u8,
                k: ALLOW as __u32,
            };
        }
        nr += 1;
    }
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh13 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh13 as usize] = sock_filter {
        code: (0x6 as ::core::ffi::c_int | 0 as ::core::ffi::c_int) as __u16,
        jt: 0 as __u8,
        jf: 0 as __u8,
        k: TRACE as __u32,
    };
    let mut prog: sock_fprog = sock_fprog {
        len: n as ::core::ffi::c_ushort,
        filter: &raw mut f as *mut sock_filter,
    };
    if prctl(
        PR_SET_NO_NEW_PRIVS,
        1 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    ) != 0 as ::core::ffi::c_int
    {
        return -*__errno();
    }
    if prctl(
        PR_SET_SECCOMP,
        SECCOMP_MODE_FILTER,
        &raw mut prog,
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    ) != 0 as ::core::ffi::c_int
    {
        return -*__errno();
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn install_test_app_filter() -> ::core::ffi::c_int {
    return -ENOSYS;
}
unsafe extern "C" fn kernel_supports_fast() -> ::core::ffi::c_int {
    let mut u: utsname = utsname {
        sysname: [0; 65],
        nodename: [0; 65],
        release: [0; 65],
        version: [0; 65],
        machine: [0; 65],
        domainname: [0; 65],
    };
    if uname(&raw mut u) != 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    let mut maj: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut min: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if sscanf(
        &raw mut u.release as *mut ::core::ffi::c_char,
        b"%d.%d\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut maj,
        &raw mut min,
    ) != 2 as ::core::ffi::c_int
    {
        return 0 as ::core::ffi::c_int;
    }
    return (maj > 4 as ::core::ffi::c_int
        || maj == 4 as ::core::ffi::c_int && min >= 8 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
static mut SEIZE_OPTS: ::core::ffi::c_long = (PTRACE_O_TRACESYSGOOD
    | PTRACE_O_TRACEFORK
    | PTRACE_O_TRACEVFORK
    | PTRACE_O_TRACECLONE
    | PTRACE_O_TRACEEXEC
    | PTRACE_O_TRACEEXIT
    | PTRACE_O_EXITKILL) as ::core::ffi::c_long;
unsafe extern "C" fn chdir_guest(mut cfg: *const eng_run_cfg) -> ::core::ffi::c_int {
    if (*cfg).guest.is_null() {
        return if !(*cfg).cwd.is_null() {
            chdir((*cfg).cwd)
        } else {
            0 as ::core::ffi::c_int
        };
    }
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut gc: *const ::core::ffi::c_char = if !(*cfg).cwd.is_null() {
        (*cfg).cwd
    } else {
        b"/\0".as_ptr() as *const ::core::ffi::c_char
    };
    if eng_guest_to_host(
        (*cfg).guest,
        gc,
        &raw mut host as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return -1 as ::core::ffi::c_int;
    }
    return chdir(&raw mut host as *mut ::core::ffi::c_char);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_tracer_run(mut cfg: *const eng_run_cfg) -> ::core::ffi::c_int {
    let mut tr: *mut eng_tracer =
        calloc(1 as size_t, ::core::mem::size_of::<eng_tracer>()) as *mut eng_tracer;
    (*tr).guest = (*cfg).guest;
    (*tr).sysinfo = -1 as ::core::ffi::c_int;
    let mut ct: *const ::core::ffi::c_char =
        getenv(b"WORKFLOW_ENGINE_CHECK_TOGGLE\0".as_ptr() as *const ::core::ffi::c_char);
    (*tr).check_toggle = (!ct.is_null() && *ct as ::core::ffi::c_int == '1' as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    let mut ready: [::core::ffi::c_int; 2] = [0; 2];
    let mut go: [::core::ffi::c_int; 2] = [0; 2];
    if pipe2(&raw mut ready as *mut ::core::ffi::c_int, O_CLOEXEC) != 0
        || pipe2(&raw mut go as *mut ::core::ffi::c_int, O_CLOEXEC) != 0
    {
        eng_logf!(
            eng_log_level::ENG_LOG_ERROR,
            b"pipe: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*__errno()),
        );
        return -1 as ::core::ffi::c_int;
    }
    install_engine_signals();
    prctl(
        PR_SET_CHILD_SUBREAPER,
        1 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    );
    let mut fast_wanted: ::core::ffi::c_int =
        ((*cfg).use_seccomp_fastpath != 0 && !(*cfg).guest.is_null()) as ::core::ffi::c_int;
    if fast_wanted != 0 && kernel_supports_fast() == 0 {
        eng_logf!(
            eng_log_level::ENG_LOG_INFO,
            b"kernel older than 4.8: seccomp fast path off\0".as_ptr()
                as *const ::core::ffi::c_char,
        );
        fast_wanted = 0 as ::core::ffi::c_int;
    }
    if !(*cfg).guest.is_null() {
        eng_link_recover((*cfg).guest);
    }
    let mut child: pid_t = fork();
    if child < 0 as ::core::ffi::c_int {
        eng_logf!(
            eng_log_level::ENG_LOG_ERROR,
            b"fork: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*__errno()),
        );
        return -1 as ::core::ffi::c_int;
    }
    if child == 0 as ::core::ffi::c_int {
        reset_child_signals();
        close(ready[0usize]);
        close(go[1usize]);
        umask(0o22 as mode_t);
        if chdir_guest(cfg) != 0 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"workflow-engine: cannot enter working directory %s: %s\n\0".as_ptr()
                    as *const ::core::ffi::c_char,
                if !(*cfg).cwd.is_null() {
                    (*cfg).cwd
                } else {
                    b"/\0".as_ptr() as *const ::core::ffi::c_char
                },
                strerror(*__errno()),
            );
            _exit(126 as ::core::ffi::c_int);
        }
        if (*cfg).test_app_filter != 0 {
            let mut arc: ::core::ffi::c_int = install_test_app_filter();
            if arc != 0 {
                fprintf(
                    stderr,
                    b"workflow-engine: test app filter: %s\n\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    strerror(-arc),
                );
                _exit(126 as ::core::ffi::c_int);
            }
        }
        let mut c: ::core::ffi::c_char = 'P' as ::core::ffi::c_char;
        if fast_wanted != 0 {
            let mut frc: ::core::ffi::c_int = install_fast_filter();
            if frc == 0 as ::core::ffi::c_int {
                c = 'S' as ::core::ffi::c_char;
            } else {
                fprintf(
                    stderr,
                    b"workflow-engine: seccomp fast path unavailable (%s), tracing every syscall\n\0"
                        .as_ptr() as *const ::core::ffi::c_char,
                    strerror(-frc),
                );
            }
        }
        if write(
            ready[1usize],
            &raw mut c as *const ::core::ffi::c_void,
            1 as size_t,
        ) != 1 as ssize_t
        {
            _exit(126 as ::core::ffi::c_int);
        }
        if read(
            go[0usize],
            &raw mut c as *mut ::core::ffi::c_void,
            1 as size_t,
        ) != 1 as ssize_t
        {
            _exit(126 as ::core::ffi::c_int);
        }
        if !(*cfg).guest.is_null() {
            execve(
                &raw mut (*(*cfg).guest).loader as *mut ::core::ffi::c_char,
                (*cfg).argv,
                (*cfg).envp,
            );
            fprintf(
                stderr,
                b"workflow-engine: exec loader %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*(*cfg).guest).loader as *mut ::core::ffi::c_char,
                strerror(*__errno()),
            );
            _exit(127 as ::core::ffi::c_int);
        }
        execvpe(*(*cfg).argv.offset(0isize), (*cfg).argv, (*cfg).envp);
        fprintf(
            stderr,
            b"workflow-engine: exec %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            *(*cfg).argv.offset(0isize),
            strerror(*__errno()),
        );
        _exit(127 as ::core::ffi::c_int);
    }
    close(ready[1usize]);
    close(go[0usize]);
    (*tr).main_pid = child;
    let mut c_0: ::core::ffi::c_char = 0;
    if read(
        ready[0usize],
        &raw mut c_0 as *mut ::core::ffi::c_void,
        1 as size_t,
    ) != 1 as ssize_t
    {
        let mut st: ::core::ffi::c_int = 0;
        waitpid(child, &raw mut st, 0 as ::core::ffi::c_int);
        return if st & 0x7f as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
            (st & 0xff00 as ::core::ffi::c_int) >> 8 as ::core::ffi::c_int
        } else {
            -1 as ::core::ffi::c_int
        };
    }
    close(ready[0usize]);
    g_fast = (c_0 as ::core::ffi::c_int == 'S' as ::core::ffi::c_int) as ::core::ffi::c_int;
    (*tr).fast = g_fast;
    let mut opts: ::core::ffi::c_long = SEIZE_OPTS
        | (if (*tr).fast != 0 {
            PTRACE_O_TRACESECCOMP
        } else {
            0 as ::core::ffi::c_int
        }) as ::core::ffi::c_long;
    if ptrace(
        PTRACE_SEIZE,
        child,
        0 as ::core::ffi::c_int,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(opts as usize),
    ) != 0 as ::core::ffi::c_long
    {
        eng_logf!(
            eng_log_level::ENG_LOG_ERROR,
            b"PTRACE_SEIZE: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*__errno()),
        );
        kill(child, SIGKILL);
        return -1 as ::core::ffi::c_int;
    }
    let mut m: *mut eng_task = task_new(tr, child);
    (*m).linked = 1 as ::core::ffi::c_int;
    (*m).phase = eng_phase::ENG_PH_BOOT;
    eng_ident_init((*cfg).guest, m, (*cfg).uid, (*cfg).gid);
    if !(*cfg).guest.is_null() {
        let mut rc: ::core::ffi::c_int = eng_exec_prepare_initial(
            m,
            if !(*cfg).exe.is_null() {
                (*cfg).exe
            } else {
                *(*cfg).argv.offset(0isize) as *const ::core::ffi::c_char
            },
        );
        if rc != 0 {
            fprintf(
                stderr,
                b"workflow-engine: %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                if !(*cfg).exe.is_null() {
                    (*cfg).exe
                } else {
                    *(*cfg).argv.offset(0isize) as *const ::core::ffi::c_char
                },
                strerror(-rc),
            );
            kill(child, SIGKILL);
            waitpid(child, ::core::ptr::null_mut::<::core::ffi::c_int>(), __WALL);
            return if rc == -ENOENT {
                127 as ::core::ffi::c_int
            } else {
                126 as ::core::ffi::c_int
            };
        }
    }
    ptrace(
        PTRACE_INTERRUPT,
        child,
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    );
    let mut status: ::core::ffi::c_int = 0;
    while waitpid(child, &raw mut status, __WALL) < 0 as ::core::ffi::c_int && *__errno() == EINTR {
    }
    resume_mode(child, 0 as ::core::ffi::c_int, 0 as ::core::ffi::c_int);
    if write(
        go[1usize],
        &raw mut c_0 as *const ::core::ffi::c_void,
        1 as size_t,
    ) != 1 as ssize_t
    {
        eng_logf!(
            eng_log_level::ENG_LOG_WARN,
            b"release child: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*__errno()),
        );
    }
    close(go[1usize]);
    let mut stop_forwarded: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    loop {
        if ::core::ptr::read_volatile::<sig_atomic_t>(&raw const g_stop_sig) != 0
            && stop_forwarded == 0
        {
            eng_logf!(
                eng_log_level::ENG_LOG_INFO,
                b"stop requested (signal %d): forwarding to guest\0".as_ptr()
                    as *const ::core::ffi::c_char,
                ::core::ptr::read_volatile::<sig_atomic_t>(&raw const g_stop_sig),
            );
            signal_all(
                tr,
                ::core::ptr::read_volatile::<sig_atomic_t>(&raw const g_stop_sig),
            );
            stop_forwarded = 1 as ::core::ffi::c_int;
            let mut gs: *const ::core::ffi::c_char =
                getenv(b"WORKFLOW_ENGINE_STOP_GRACE\0".as_ptr() as *const ::core::ffi::c_char);
            alarm(if !gs.is_null() {
                atoi(gs) as ::core::ffi::c_uint
            } else {
                3 as ::core::ffi::c_uint
            });
        }
        if ::core::ptr::read_volatile::<sig_atomic_t>(&raw const g_alarm) != 0 {
            ::core::ptr::write_volatile(&raw mut g_alarm, 0 as ::core::ffi::c_int as sig_atomic_t);
            eng_logf!(
                eng_log_level::ENG_LOG_INFO,
                b"grace period over: killing guest tree\0".as_ptr() as *const ::core::ffi::c_char,
            );
            signal_all(tr, SIGKILL);
        }
        let mut tid: pid_t = waitpid(-1 as pid_t, &raw mut status, __WALL);
        if tid < 0 as ::core::ffi::c_int {
            if *__errno() == EINTR {
                continue;
            }
            if *__errno() == ECHILD {
                break;
            }
            eng_logf!(
                eng_log_level::ENG_LOG_ERROR,
                b"waitpid: %s\0".as_ptr() as *const ::core::ffi::c_char,
                strerror(*__errno()),
            );
            break;
        } else if status & 0x7f as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || status + 1 as ::core::ffi::c_int & 0x7f as ::core::ffi::c_int
                >= 2 as ::core::ffi::c_int
        {
            if tid == (*tr).main_pid {
                (*tr).main_status =
                    if status & 0x7f as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                        (status & 0xff00 as ::core::ffi::c_int) >> 8 as ::core::ffi::c_int
                    } else {
                        128 as ::core::ffi::c_int + (status & 0x7f as ::core::ffi::c_int)
                    };
                (*tr).main_seen = 1 as ::core::ffi::c_int;
                task_del(tr, tid);
                if (*cfg).wait_all == 0 && (*tr).ntasks != 0 && stop_forwarded == 0 {
                    eng_logf!(
                        eng_log_level::ENG_LOG_INFO,
                        b"main process exited: terminating %zu remaining task(s)\0".as_ptr()
                            as *const ::core::ffi::c_char,
                        (*tr).ntasks,
                    );
                    signal_all(tr, SIGTERM);
                    stop_forwarded = 1 as ::core::ffi::c_int;
                    let mut gs_0: *const ::core::ffi::c_char = getenv(
                        b"WORKFLOW_ENGINE_STOP_GRACE\0".as_ptr() as *const ::core::ffi::c_char,
                    );
                    alarm(if !gs_0.is_null() {
                        atoi(gs_0) as ::core::ffi::c_uint
                    } else {
                        3 as ::core::ffi::c_uint
                    });
                }
            } else {
                task_del(tr, tid);
            }
        } else {
            if !(status & 0xff as ::core::ffi::c_int == 0x7f as ::core::ffi::c_int) {
                continue;
            }
            let mut sig: ::core::ffi::c_int =
                (status & 0xff00 as ::core::ffi::c_int) >> 8 as ::core::ffi::c_int;
            let mut event: ::core::ffi::c_uint =
                status as ::core::ffi::c_uint >> 16 as ::core::ffi::c_int;
            let mut t: *mut eng_task = eng_task_find(tr, tid);
            if t.is_null() {
                t = task_new(tr, tid);
            }
            if (*t).linked == 0 {
                (*t).held = 1 as ::core::ffi::c_int;
            } else {
                match event {
                    0 => {
                        if sig == SIGTRAP | 0x80 as ::core::ffi::c_int {
                            syscall_stop(tr, t, 0 as ::core::ffi::c_int);
                        } else if sig == SIGSYS {
                            sigsys_stop(t);
                        } else {
                            if eng_log_enabled(eng_log_level::ENG_LOG_DEBUG) != 0
                                && (sig == SIGSEGV || sig == SIGBUS || sig == SIGILL)
                            {
                                let mut si: siginfo_t = siginfo_t {
                                    c2rust_unnamed: C2Rust_Unnamed_10 {
                                        c2rust_unnamed: C2Rust_Unnamed_11 {
                                            si_signo: 0,
                                            si_errno: 0,
                                            si_code: 0,
                                            _sifields: __sifields {
                                                _kill: C2Rust_Unnamed_9 { _pid: 0, _uid: 0 },
                                            },
                                        },
                                    },
                                };
                                let mut rr: eng_regs = eng_regs {
                                    regs: [0; 31],
                                    sp: 0,
                                    pc: 0,
                                    pstate: 0,
                                };
                                if ptrace(
                                    PTRACE_GETSIGINFO,
                                    tid,
                                    0 as ::core::ffi::c_int,
                                    &raw mut si,
                                ) == 0 as ::core::ffi::c_long
                                    && eng_regs_get(tid, &raw mut rr) == 0 as ::core::ffi::c_int
                                {
                                    eng_logf!(
                                        eng_log_level::ENG_LOG_DEBUG,
                                        b"tid=%d signal %d code=%d addr=%p pc=%#llx phase=%d\0"
                                            .as_ptr()
                                            as *const ::core::ffi::c_char,
                                        tid,
                                        sig,
                                        si.c2rust_unnamed.c2rust_unnamed.si_code,
                                        si.c2rust_unnamed.c2rust_unnamed._sifields._sigfault._addr,
                                        eng_pc(&raw mut rr) as ::core::ffi::c_ulonglong,
                                        (*t).phase.0 as ::core::ffi::c_int,
                                    );
                                }
                            }
                            resume_task(t, sig);
                        }
                    }
                    1 | 2 | 3 => {
                        let mut ctid: ::core::ffi::c_ulong = 0 as ::core::ffi::c_ulong;
                        if ptrace(
                            PTRACE_GETEVENTMSG,
                            tid,
                            0 as ::core::ffi::c_int,
                            &raw mut ctid,
                        ) == 0 as ::core::ffi::c_long
                            && ctid != 0
                        {
                            on_new_child(tr, t, ctid as pid_t, event as ::core::ffi::c_int);
                        }
                        resume_task(t, 0 as ::core::ffi::c_int);
                    }
                    4 => {
                        on_exec(tr, t);
                        resume_task(t, 0 as ::core::ffi::c_int);
                    }
                    7 => {
                        syscall_stop(tr, t, 1 as ::core::ffi::c_int);
                    }
                    PTRACE_EVENT_STOP => {
                        if sig == SIGSTOP || sig == SIGTSTP || sig == SIGTTIN || sig == SIGTTOU {
                            if ptrace(
                                PTRACE_LISTEN,
                                tid,
                                0 as ::core::ffi::c_int,
                                0 as ::core::ffi::c_int,
                            ) != 0 as ::core::ffi::c_long
                                && *__errno() != ESRCH
                            {
                                eng_logf!(
                                    eng_log_level::ENG_LOG_DEBUG,
                                    b"PTRACE_LISTEN tid=%d: %s\0".as_ptr()
                                        as *const ::core::ffi::c_char,
                                    tid,
                                    strerror(*__errno()),
                                );
                            }
                        } else {
                            resume_task(t, 0 as ::core::ffi::c_int);
                        }
                    }
                    _ => {
                        resume_task(t, 0 as ::core::ffi::c_int);
                    }
                }
            }
        }
    }
    if (*tr).check_toggle != 0
        || !getenv(b"WORKFLOW_ENGINE_STATS\0".as_ptr() as *const ::core::ffi::c_char).is_null()
    {
        fprintf(
            stderr,
            b"workflow-engine: stats syscall_stops=%lu seccomp_stops=%lu toggle_mismatch=%lu sysinfo=%d fast=%d\n\0"
                .as_ptr() as *const ::core::ffi::c_char,
            (*tr).syscall_stops,
            (*tr).seccomp_stops,
            (*tr).toggle_mismatch,
            (*tr).sysinfo,
            (*tr).fast,
        );
    }
    let mut rc_0: ::core::ffi::c_int = if (*tr).main_seen != 0 {
        (*tr).main_status
    } else {
        -1 as ::core::ffi::c_int
    };
    if (*tr).check_toggle != 0 && (*tr).toggle_mismatch != 0 {
        rc_0 = 124 as ::core::ffi::c_int;
    }
    free(tr as *mut ::core::ffi::c_void);
    return rc_0;
}
