//! Single-writer ptrace process-tree scheduler, signals, seccomp and exit cleanup.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    unsafe fn kill(_: i32, _: i32) -> i32;
    #[cfg(any(target_os = "linux", target_arch = "aarch64"))]
    unsafe fn sigemptyset(_: *mut sigset_t) -> i32;
    #[cfg(all(target_os = "android", target_arch = "x86_64"))]
    unsafe fn sigemptyset(_: *mut u64) -> i32;
    #[cfg(any(target_os = "linux", target_arch = "aarch64"))]
    unsafe fn sigprocmask(_: i32, _: *const sigset_t, _: *mut sigset_t) -> i32;
    #[cfg(all(target_os = "android", target_arch = "x86_64"))]
    unsafe fn sigprocmask(_: i32, _: *const u64, _: *mut u64) -> i32;
    #[cfg(target_os = "linux")]
    unsafe fn sigaction(_: i32, _: *const sigaction, _: *mut sigaction) -> i32;
    #[cfg(target_os = "android")]
    unsafe fn sigaction(_: i32, _: *const sigaction, _: *mut sigaction) -> i32;
    unsafe fn close(_: i32) -> i32;
    unsafe fn read(_: i32, _: *mut ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn write(_: i32, _: *const ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn pipe2(_: *mut i32, _: i32) -> i32;
    unsafe fn alarm(_: u32) -> u32;
    unsafe fn chdir(_: *const ::core::ffi::c_char) -> i32;
    unsafe fn execve(
        _: *const ::core::ffi::c_char,
        _: *const *mut ::core::ffi::c_char,
        _: *const *mut ::core::ffi::c_char,
    ) -> i32;
    unsafe fn execvpe(
        _: *const ::core::ffi::c_char,
        _: *const *mut ::core::ffi::c_char,
        _: *const *mut ::core::ffi::c_char,
    ) -> i32;
    unsafe fn _exit(_: i32) -> !;
    unsafe fn fork() -> i32;
    unsafe fn eng_regs_get(_: i32, _: *mut eng_regs) -> i32;
    unsafe fn eng_regs_set(_: i32, _: *const eng_regs) -> i32;
    unsafe fn eng_sysno(_: *const eng_regs) -> i64;
    unsafe fn eng_arg(_: *const eng_regs, _: i32) -> u64;
    unsafe fn eng_ret(_: *const eng_regs) -> u64;
    unsafe fn eng_set_ret(_: *mut eng_regs, _: u64);
    unsafe fn eng_pc(_: *const eng_regs) -> u64;
    unsafe fn eng_syscall_void(_: i32, _: *mut eng_regs) -> i32;
    unsafe fn eng_restore_args(_: *mut eng_regs, _: *const eng_regs);
    unsafe fn eng_syscall_info_op(_: i32) -> i32;
    unsafe fn eng_sys_entry(_: *mut eng_task, _: *mut eng_regs) -> i32;
    unsafe fn eng_sys_exit(_: *mut eng_task, _: *mut eng_regs) -> i32;
    unsafe fn eng_sys_sigsys(_: *mut eng_task, _: *mut eng_regs, _: *mut siginfo_t) -> i32;
    unsafe fn eng_scratch_release(_: *mut eng_task);
    #[cfg(target_os = "linux")]
    #[link_name = "__errno_location"]
    unsafe fn errno() -> *mut i32;
    #[cfg(target_os = "android")]
    #[link_name = "__errno"]
    unsafe fn errno() -> *mut i32;
    static mut stderr: *mut FILE;
    unsafe fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> i32;
    unsafe fn snprintf(
        _: *mut ::core::ffi::c_char,
        _: usize,
        _: *const ::core::ffi::c_char,
        ...
    ) -> i32;
    unsafe fn sscanf(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char, ...) -> i32;
    unsafe fn atoi(_: *const ::core::ffi::c_char) -> i32;
    unsafe fn calloc(_: usize, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn free(_: *mut ::core::ffi::c_void);
    unsafe fn exit(_: i32) -> !;
    unsafe fn getenv(_: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: usize,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memset(_: *mut ::core::ffi::c_void, _: i32, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn strerror(_: i32) -> *mut ::core::ffi::c_char;
    unsafe fn prctl(_: i32, ...) -> i32;
    unsafe fn umask(_: u32) -> u32;
    #[cfg(target_os = "linux")]
    unsafe fn ptrace(_: u32, ...) -> i64;
    #[cfg(target_os = "android")]
    unsafe fn ptrace(_: i32, ...) -> i64;
    unsafe fn waitpid(_: i32, _: *mut i32, _: i32) -> i32;
    unsafe fn eng_exec_prepare_initial(_: *mut eng_task, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn eng_exec_discard(_: *mut eng_task);
    unsafe fn eng_guest_to_host(
        _: *const eng_guest,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn eng_ident_init(_: *mut eng_guest, _: *mut eng_task, _: u32, _: u32);
    unsafe fn eng_ident_exec(_: *mut eng_task);
    unsafe fn eng_link_recover(_: *mut eng_guest) -> i32;
    unsafe fn eng_log_enabled(_: eng_log_level) -> i32;
    unsafe fn eng_mem_write(_: i32, _: usize, _: *const ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn eng_sysinv_name(_: i64) -> *const ::core::ffi::c_char;
    unsafe fn eng_sysinv_class(_: i64) -> eng_sc_class;
    unsafe fn eng_sysinv_max() -> i64;
    unsafe fn uname(_: *mut utsname) -> i32;
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
use ::libc;
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sigset_t {
    pub __val: [u64; 16],
}
#[cfg(target_os = "linux")]
pub type sigset_t = __sigset_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub union sigval {
    pub sival_int: i32,
    pub sival_ptr: *mut ::core::ffi::c_void,
}
#[cfg(target_os = "linux")]
pub type __sigval_t = sigval;
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct siginfo_t {
    pub si_signo: i32,
    pub si_errno: i32,
    pub si_code: i32,
    pub __pad0: i32,
    pub _sifields: C2Rust_Unnamed,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed {
    pub _pad: [i32; 28],
    pub _kill: C2Rust_Unnamed_8,
    pub _timer: C2Rust_Unnamed_7,
    pub _rt: C2Rust_Unnamed_6,
    pub _sigchld: C2Rust_Unnamed_5,
    pub _sigfault: C2Rust_Unnamed_2,
    pub _sigpoll: C2Rust_Unnamed_1,
    pub _sigsys: C2Rust_Unnamed_0,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed {
    pub c2rust_unnamed: C2Rust_Unnamed_0,
    pub _si_pad: [i32; 32],
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_0 {
    pub _call_addr: *mut ::core::ffi::c_void,
    pub _syscall: i32,
    pub _arch: u32,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_0 {
    pub si_signo: i32,
    pub si_errno: i32,
    pub si_code: i32,
    pub _sifields: __sifields,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_0 {
    pub _band: i64,
    pub _fd: i32,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_1 {
    pub si_band: i64,
    pub si_fd: i32,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_1 {
    pub _call_addr: *mut ::core::ffi::c_void,
    pub _syscall: i32,
    pub _arch: u32,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_1 {
    pub _addr: *mut ::core::ffi::c_void,
    pub c2rust_unnamed: C2Rust_Unnamed_2,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_2 {
    pub si_addr: *mut ::core::ffi::c_void,
    pub si_addr_lsb: i16,
    pub _bounds: C2Rust_Unnamed_3,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_2 {
    pub _band: i64,
    pub _fd: i32,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed_3 {
    pub _addr_bnd: C2Rust_Unnamed_4,
    pub _pkey: u32,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_4 {
    pub _lower: *mut ::core::ffi::c_void,
    pub _upper: *mut ::core::ffi::c_void,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_4 {
    pub _dummy_pkey: [::core::ffi::c_char; 8],
    pub _pkey: u32,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_5 {
    pub si_pid: i32,
    pub si_uid: u32,
    pub si_status: i32,
    pub si_utime: i64,
    pub si_stime: i64,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_5 {
    pub _data: u64,
    pub _type: u32,
    pub _flags: u32,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_5 {
    pub _dummy_bnd: [::core::ffi::c_char; 8],
    pub _lower: *mut ::core::ffi::c_void,
    pub _upper: *mut ::core::ffi::c_void,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_6 {
    pub si_pid: i32,
    pub si_uid: u32,
    pub si_sigval: __sigval_t,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_6 {
    pub _dummy_pkey: [::core::ffi::c_char; 8],
    pub _pkey: u32,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_6 {
    pub _pid: i32,
    pub _uid: u32,
    pub _status: i32,
    pub _utime: i64,
    pub _stime: i64,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_7 {
    pub si_tid: i32,
    pub si_overrun: i32,
    pub si_sigval: __sigval_t,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_7 {
    pub _dummy_bnd: [::core::ffi::c_char; 8],
    pub _lower: *mut ::core::ffi::c_void,
    pub _upper: *mut ::core::ffi::c_void,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_7 {
    pub _pid: i32,
    pub _uid: u32,
    pub _sigval: sigval_t,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_8 {
    pub si_pid: i32,
    pub si_uid: u32,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_8 {
    pub _pid: i32,
    pub _uid: u32,
    pub _status: i32,
    pub _utime: i64,
    pub _stime: i64,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_8 {
    pub _tid: i32,
    pub _overrun: i32,
    pub _sigval: sigval_t,
    pub _sys_private: i32,
}
#[cfg(target_os = "linux")]
pub type __sighandler_t = Option<unsafe extern "C" fn(i32) -> ()>;
#[cfg(target_os = "android")]
pub type __sighandler_t = Option<__signalfn_t>;
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sigaction {
    pub __sigaction_handler: C2Rust_Unnamed_9,
    pub sa_mask: __sigset_t,
    pub sa_flags: i32,
    pub sa_restorer: Option<unsafe extern "C" fn() -> ()>,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sigaction {
    pub sa_flags: i32,
    pub c2rust_unnamed: C2Rust_Unnamed_12,
    pub sa_mask: u64,
    pub sa_restorer: Option<unsafe extern "C" fn() -> ()>,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sigaction {
    pub sa_flags: i32,
    pub c2rust_unnamed: C2Rust_Unnamed_12,
    pub sa_mask: sigset_t,
    pub sa_restorer: Option<unsafe extern "C" fn() -> ()>,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed_9 {
    pub sa_handler: __sighandler_t,
    pub sa_sigaction:
        Option<unsafe extern "C" fn(i32, *mut siginfo_t, *mut ::core::ffi::c_void) -> ()>,
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
pub struct eng_tracer {
    pub buckets: [*mut eng_task; 1024],
    pub ntasks: usize,
    pub main_pid: i32,
    pub main_status: i32,
    pub main_seen: i32,
    pub guest: *mut eng_guest,
    pub sysinfo: i32,
    pub fast: i32,
    pub seccomp_stops: u64,
    pub check_toggle: i32,
    pub toggle_mismatch: u64,
    pub syscall_stops: u64,
}
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
pub struct eng_proc {
    pub refs: i32,
    pub exe: [::core::ffi::c_char; 4096],
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
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_phase(pub u32);
impl eng_phase {
    pub const ENG_PH_BOOT: Self = Self(0);
    pub const ENG_PH_LOADER: Self = Self(1);
    pub const ENG_PH_GUEST: Self = Self(2);
}
pub const ENG_FIX_NONE: u32 = 0;
pub const ENG_FIX_STAT: u32 = 1;
pub const ENG_FIX_STATX: u32 = 2;
pub const ENG_FIX_CREATE_FD: u32 = 3;
pub const ENG_FIX_CREATE_PATH: u32 = 4;
pub const ENG_FIX_DROP_LINK: u32 = 5;
pub const ENG_FIX_RENAME_IN: u32 = 6;
pub const ENG_FIX_SOCKNAME: u32 = 7;
pub const ENG_FIX_GETDENTS: u32 = 8;
pub const ENG_FIX_LISTXATTR: u32 = 9;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_run_cfg {
    pub argv: *mut *mut ::core::ffi::c_char,
    pub exe: *const ::core::ffi::c_char,
    pub envp: *mut *mut ::core::ffi::c_char,
    pub cwd: *const ::core::ffi::c_char,
    pub guest: *mut eng_guest,
    pub use_seccomp_fastpath: i32,
    pub uid: u32,
    pub gid: u32,
    pub test_app_filter: i32,
    pub wait_all: i32,
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
pub type node = eng_task_node;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_task_node {
    pub t: eng_task,
    pub next: *mut eng_task_node,
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
pub const PTRACE_TRACEME: u32 = 0;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKTEXT: u32 = 1;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKDATA: u32 = 2;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKUSER: u32 = 3;
#[cfg(target_os = "linux")]
pub const PTRACE_POKETEXT: u32 = 4;
#[cfg(target_os = "linux")]
pub const PTRACE_POKEDATA: u32 = 5;
#[cfg(target_os = "linux")]
pub const PTRACE_POKEUSER: u32 = 6;
#[cfg(target_os = "linux")]
pub const PTRACE_CONT: u32 = 7;
#[cfg(target_os = "android")]
pub const PTRACE_CONT: i32 = 7 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_KILL: u32 = 8;
#[cfg(target_os = "linux")]
pub const PTRACE_SINGLESTEP: u32 = 9;
#[cfg(target_os = "linux")]
pub const PTRACE_GETREGS: u32 = 12;
#[cfg(target_os = "linux")]
pub const PTRACE_SETREGS: u32 = 13;
#[cfg(target_os = "linux")]
pub const PTRACE_GETFPREGS: u32 = 14;
#[cfg(target_os = "linux")]
pub const PTRACE_SETFPREGS: u32 = 15;
#[cfg(target_os = "linux")]
pub const PTRACE_ATTACH: u32 = 16;
#[cfg(target_os = "linux")]
pub const PTRACE_DETACH: u32 = 17;
#[cfg(target_os = "linux")]
pub const PTRACE_GETFPXREGS: u32 = 18;
#[cfg(target_os = "linux")]
pub const PTRACE_SETFPXREGS: u32 = 19;
#[cfg(target_os = "linux")]
pub const PTRACE_SYSCALL: u32 = 24;
#[cfg(target_os = "android")]
pub const PTRACE_SYSCALL: i32 = 24 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_GET_THREAD_AREA: u32 = 25;
#[cfg(target_os = "linux")]
pub const PTRACE_SET_THREAD_AREA: u32 = 26;
#[cfg(target_os = "linux")]
pub const PTRACE_ARCH_PRCTL: u32 = 30;
#[cfg(target_os = "linux")]
pub const PTRACE_SYSEMU: u32 = 31;
#[cfg(target_os = "linux")]
pub const PTRACE_SYSEMU_SINGLESTEP: u32 = 32;
#[cfg(target_os = "linux")]
pub const PTRACE_SINGLEBLOCK: u32 = 33;
#[cfg(target_os = "linux")]
pub const PTRACE_SETOPTIONS: u32 = 16896;
#[cfg(target_os = "linux")]
pub const PTRACE_GETEVENTMSG: u32 = 16897;
#[cfg(target_os = "android")]
pub const PTRACE_GETEVENTMSG: i32 = 0x4201 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_GETSIGINFO: u32 = 16898;
#[cfg(target_os = "android")]
pub const PTRACE_GETSIGINFO: i32 = 0x4202 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_SETSIGINFO: u32 = 16899;
#[cfg(target_os = "linux")]
pub const PTRACE_GETREGSET: u32 = 16900;
#[cfg(target_os = "linux")]
pub const PTRACE_SETREGSET: u32 = 16901;
#[cfg(target_os = "linux")]
pub const PTRACE_SEIZE: u32 = 16902;
#[cfg(target_os = "android")]
pub const PTRACE_SEIZE: i32 = 0x4206 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_INTERRUPT: u32 = 16903;
#[cfg(target_os = "android")]
pub const PTRACE_INTERRUPT: i32 = 0x4207 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_LISTEN: u32 = 16904;
#[cfg(target_os = "android")]
pub const PTRACE_LISTEN: i32 = 0x4208 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKSIGINFO: u32 = 16905;
#[cfg(target_os = "linux")]
pub const PTRACE_GETSIGMASK: u32 = 16906;
#[cfg(target_os = "linux")]
pub const PTRACE_SETSIGMASK: u32 = 16907;
#[cfg(target_os = "linux")]
pub const PTRACE_SECCOMP_GET_FILTER: u32 = 16908;
#[cfg(target_os = "linux")]
pub const PTRACE_SECCOMP_GET_METADATA: u32 = 16909;
#[cfg(target_os = "linux")]
pub const PTRACE_GET_SYSCALL_INFO: u32 = 16910;
#[cfg(target_os = "linux")]
pub const PTRACE_GET_RSEQ_CONFIGURATION: u32 = 16911;
#[cfg(target_os = "linux")]
pub const PTRACE_SET_SYSCALL_USER_DISPATCH_CONFIG: u32 = 16912;
#[cfg(target_os = "linux")]
pub const PTRACE_GET_SYSCALL_USER_DISPATCH_CONFIG: u32 = 16913;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sock_fprog {
    pub len: u16,
    pub filter: *mut sock_filter,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sock_filter {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_sc_class(pub u32);
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
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct __ptrace_setoptions(pub u32);
#[cfg(target_os = "linux")]
impl __ptrace_setoptions {
    pub const PTRACE_O_TRACESYSGOOD: Self = Self(1);
    pub const PTRACE_O_TRACEFORK: Self = Self(2);
    pub const PTRACE_O_TRACEVFORK: Self = Self(4);
    pub const PTRACE_O_TRACECLONE: Self = Self(8);
    pub const PTRACE_O_TRACEEXEC: Self = Self(16);
    pub const PTRACE_O_TRACEVFORKDONE: Self = Self(32);
    pub const PTRACE_O_TRACEEXIT: Self = Self(64);
    pub const PTRACE_O_TRACESECCOMP: Self = Self(128);
    pub const PTRACE_O_EXITKILL: Self = Self(1048576);
    pub const PTRACE_O_SUSPEND_SECCOMP: Self = Self(2097152);
    pub const PTRACE_O_MASK: Self = Self(3145983);
}
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct __ptrace_eventcodes(pub u32);
#[cfg(target_os = "linux")]
impl __ptrace_eventcodes {
    pub const PTRACE_EVENT_FORK: Self = Self(1);
    pub const PTRACE_EVENT_VFORK: Self = Self(2);
    pub const PTRACE_EVENT_CLONE: Self = Self(3);
    pub const PTRACE_EVENT_EXEC: Self = Self(4);
    pub const PTRACE_EVENT_VFORK_DONE: Self = Self(5);
    pub const PTRACE_EVENT_EXIT: Self = Self(6);
    pub const PTRACE_EVENT_SECCOMP: Self = Self(7);
    pub const PTRACE_EVENT_STOP: Self = Self(128);
}
pub const SIG_DFL: __sighandler_t = None;
pub const SIGINT: i32 = 2 as i32;
pub const SIGILL: i32 = 4 as i32;
pub const SIGSEGV: i32 = 11 as i32;
pub const SIGTERM: i32 = 15 as i32;
pub const SIGHUP: i32 = 1 as i32;
pub const SIGQUIT: i32 = 3 as i32;
pub const SIGTRAP: i32 = 5 as i32;
pub const SIGKILL: i32 = 9 as i32;
pub const SIGPIPE: i32 = 13 as i32;
pub const SIGALRM: i32 = 14 as i32;
pub const SIGBUS: i32 = 7 as i32;
pub const SIGSYS: i32 = 31 as i32;
pub const SIGSTOP: i32 = 19 as i32;
pub const SIGTSTP: i32 = 20 as i32;
pub const SIGTTIN: i32 = 21 as i32;
pub const SIGTTOU: i32 = 22 as i32;
pub const SIG_SETMASK: i32 = 2 as i32;
pub const ENOENT: i32 = 2 as i32;
pub const ESRCH: i32 = 3 as i32;
pub const EINTR: i32 = 4 as i32;
pub const E2BIG: i32 = 7 as i32;
pub const ECHILD: i32 = 10 as i32;
pub const EFAULT: i32 = 14 as i32;
pub const EINVAL: i32 = 22 as i32;
pub const ENOSYS: i32 = 38 as i32;
#[cfg(target_os = "linux")]
pub const __O_CLOEXEC: i32 = 0o2000000 as i32;
#[cfg(target_os = "linux")]
pub const O_CLOEXEC: i32 = __O_CLOEXEC;
#[cfg(target_os = "android")]
pub const O_CLOEXEC: i32 = 0o2000000 as i32;
pub const CLONE_VM: i32 = 0x100 as i32;
pub const CLONE_THREAD: i32 = 0x10000 as i32;
pub const __WALL: i32 = 0x40000000 as i32;
pub const PR_SET_SECCOMP: i32 = 22 as i32;
pub const PR_SET_CHILD_SUBREAPER: i32 = 36 as i32;
pub const PR_SET_NO_NEW_PRIVS: i32 = 38 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_open: i32 = 2 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_stat: i32 = 4 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_lstat: i32 = 6 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_poll: i32 = 7 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_access: i32 = 21 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_pipe: i32 = 22 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_select: i32 = 23 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_dup2: i32 = 33 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_alarm: i32 = 37 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_getpid: i32 = 39 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_getpid: i32 = 172 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_socket: i32 = 41 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_socket: i32 = 198 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_recvfrom: i32 = 45 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_recvfrom: i32 = 207 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_recvmsg: i32 = 47 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_recvmsg: i32 = 212 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_execve: i32 = 59 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_execve: i32 = 221 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_getdents: i32 = 78 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_rename: i32 = 82 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_mkdir: i32 = 83 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_rmdir: i32 = 84 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_creat: i32 = 85 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_link: i32 = 86 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_unlink: i32 = 87 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_symlink: i32 = 88 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_readlink: i32 = 89 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_chmod: i32 = 90 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_chown: i32 = 92 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_lchown: i32 = 94 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_umask: i32 = 95 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_umask: i32 = 166 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setuid: i32 = 105 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setgid: i32 = 106 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_getpgrp: i32 = 111 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setreuid: i32 = 113 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setregid: i32 = 114 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setgroups: i32 = 116 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setresuid: i32 = 117 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setresgid: i32 = 119 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setfsuid: i32 = 122 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setfsgid: i32 = 123 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_mknod: i32 = 133 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_uselib: i32 = 134 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_prctl: i32 = 157 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_prctl: i32 = 167 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_adjtimex: i32 = 159 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_chroot: i32 = 161 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_acct: i32 = 163 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_mount: i32 = 165 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_umount2: i32 = 166 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_swapon: i32 = 167 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_time: i32 = 201 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_epoll_create: i32 = 213 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_epoll_wait: i32 = 232 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_utimes: i32 = 235 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_mbind: i32 = 237 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_set_mempolicy: i32 = 238 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_get_mempolicy: i32 = 239 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_add_key: i32 = 248 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_keyctl: i32 = 250 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_inotify_init: i32 = 253 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_migrate_pages: i32 = 256 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_futimesat: i32 = 261 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_set_robust_list: i32 = 273 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_move_pages: i32 = 279 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_signalfd: i32 = 282 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_eventfd: i32 = 284 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_recvmmsg: i32 = 299 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_recvmmsg: i32 = 243 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_fanotify_init: i32 = 300 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_name_to_handle_at: i32 = 303 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_open_by_handle_at: i32 = 304 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_clock_adjtime: i32 = 305 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_kcmp: i32 = 312 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_kexec_file_load: i32 = 320 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_bpf: i32 = 321 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_execveat: i32 = 322 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_execveat: i32 = 281 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_userfaultfd: i32 = 323 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_membarrier: i32 = 324 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_pkey_mprotect: i32 = 329 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_pkey_alloc: i32 = 330 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_statx: i32 = 332 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_io_pgetevents: i32 = 333 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_rseq: i32 = 334 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_pidfd_send_signal: i32 = 424 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_io_uring_setup: i32 = 425 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_open_tree: i32 = 428 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_fsopen: i32 = 430 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_pidfd_open: i32 = 434 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_clone3: i32 = 435 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_close_range: i32 = 436 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_openat2: i32 = 437 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_pidfd_getfd: i32 = 438 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_faccessat2: i32 = 439 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_process_madvise: i32 = 440 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_epoll_pwait2: i32 = 441 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_mount_setattr: i32 = 442 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_landlock_create_ruleset: i32 = 444 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_memfd_secret: i32 = 447 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_futex_waitv: i32 = 449 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_cachestat: i32 = 451 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_fchmodat2: i32 = 452 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_map_shadow_stack: i32 = 453 as i32;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ENG_MARK_A: u64 = 0x574f524b464c4f57 as u64;
pub const ENG_MARK_B: u64 = 0x4c4f414445523031 as u64;
pub const ENG_MARK_OP_QUERY: u32 = 1 as u32;
pub const ENG_MARK_OP_DONE: u32 = 2 as u32;
pub const ENG_SLOT_SIZE: u32 = (16 as u32).wrapping_mul(1024 as u32);
pub const SECCOMP_MODE_FILTER: i32 = 2 as i32;
pub const SECCOMP_RET_ERRNO: u32 = 0x50000 as u32;
pub const SECCOMP_RET_TRACE: u32 = 0x7ff00000 as u32;
pub const SECCOMP_RET_ALLOW: u32 = 0x7fff0000 as u32;
#[cfg(target_os = "linux")]
pub const PTRACE_EVENT_SECCOMP: u32 = 7 as u32;
#[cfg(target_os = "android")]
pub const PTRACE_EVENT_SECCOMP: i32 = 7 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_O_TRACESECCOMP: i32 = 0x80 as i32;
#[cfg(target_os = "android")]
pub const PTRACE_O_TRACESECCOMP: i32 = (1 as i32) << PTRACE_EVENT_SECCOMP;
pub const PTRACE_EVENT_STOP: u32 = 128 as u32;
pub const NBUCKETS: i32 = 1024 as i32;
unsafe extern "C" fn bucket(mut tid: i32) -> u32 {
    return (tid as u32).wrapping_rem(NBUCKETS as u32);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_task_find(mut tr: *mut eng_tracer, mut tid: i32) -> *mut eng_task {
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
unsafe extern "C" fn task_new(mut tr: *mut eng_tracer, mut tid: i32) -> *mut eng_task {
    let mut n: *mut node = calloc(1 as usize, ::core::mem::size_of::<node>()) as *mut node;
    if n.is_null() {
        eng_logf!(
            eng_log_level::ENG_LOG_ERROR,
            b"out of memory\0".as_ptr() as *const ::core::ffi::c_char,
        );
        exit(125 as i32);
    }
    (*n).t.tid = tid;
    (*n).t.tgid = tid;
    (*n).t.in_use = 1 as i32;
    (*n).t.at_syscall_entry = 1 as i32;
    (*n).t.slot = -1 as i32;
    (*n).t.umask_old = -1 as i32;
    (*n).t.tr = tr;
    (*n).next = (*tr).buckets[bucket(tid) as usize] as *mut node as *mut eng_task_node;
    (*tr).buckets[bucket(tid) as usize] = &raw mut (*n).t;
    (*tr).ntasks = (*tr).ntasks.wrapping_add(1);
    return &raw mut (*n).t;
}
unsafe extern "C" fn mm_put(mut mm: *mut eng_mm) {
    if !mm.is_null() && {
        (*mm).refs -= 1;
        (*mm).refs <= 0 as i32
    } {
        free(mm as *mut ::core::ffi::c_void);
    }
}
unsafe extern "C" fn proc_put(mut p: *mut eng_proc) {
    if !p.is_null() && {
        (*p).refs -= 1;
        (*p).refs <= 0 as i32
    } {
        free(p as *mut ::core::ffi::c_void);
    }
}
unsafe extern "C" fn task_del(mut tr: *mut eng_tracer, mut tid: i32) {
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
        calloc(1 as usize, ::core::mem::size_of::<eng_proc>()) as *mut eng_proc;
    (*p).refs = 1 as i32;
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
    mut result: i64,
) -> i32 {
    if eng_syscall_void((*t).tid, r) != 0 as i32 && *errno() != ESRCH {
        eng_logf!(
            eng_log_level::ENG_LOG_WARN,
            b"void syscall tid=%d: %s\0".as_ptr() as *const ::core::ffi::c_char,
            (*t).tid,
            strerror(*errno()),
        );
    }
    (*t).void_pending = 1 as i32;
    (*t).inject_result = result;
    return 0 as i32;
}
static mut g_fast: i32 = 0;
unsafe extern "C" fn resume_mode(mut tid: i32, mut sig: i32, mut want_syscall: i32) {
    #[cfg(target_os = "linux")]
    let mut req: i32 = if g_fast == 0 || want_syscall != 0 {
        PTRACE_SYSCALL as i32
    } else {
        PTRACE_CONT as i32
    };
    #[cfg(target_os = "android")]
    let mut req: i32 = if g_fast == 0 || want_syscall != 0 {
        PTRACE_SYSCALL
    } else {
        PTRACE_CONT
    };
    #[cfg(target_os = "linux")]
    if ptrace(
        req as u32,
        tid,
        0 as i32,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(sig as i64 as usize),
    ) != 0 as i64
        && *errno() != ESRCH
    {
        eng_logf!(
            eng_log_level::ENG_LOG_DEBUG,
            b"resume tid=%d: %s\0".as_ptr() as *const ::core::ffi::c_char,
            tid,
            strerror(*errno()),
        );
    }
    #[cfg(target_os = "android")]
    if ptrace(
        req,
        tid,
        0 as i32,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(sig as isize as usize),
    ) != 0 as i64
        && *errno() != ESRCH
    {
        eng_logf!(
            eng_log_level::ENG_LOG_DEBUG,
            b"resume tid=%d: %s\0".as_ptr() as *const ::core::ffi::c_char,
            tid,
            strerror(*errno()),
        );
    }
}
unsafe extern "C" fn resume_task(mut t: *mut eng_task, mut sig: i32) {
    resume_mode((*t).tid, sig, (*t).await_exit);
}
static mut g_stop_sig: i32 = 0;
static mut g_alarm: i32 = 0;
unsafe extern "C" fn on_stop_signal(mut s: i32) {
    ::core::ptr::write_volatile(&raw mut g_stop_sig, s as i32);
}
unsafe extern "C" fn on_alarm(mut s: i32) {
    ::core::ptr::write_volatile(&raw mut g_alarm, 1 as i32 as i32);
}
unsafe extern "C" fn install_engine_signals() {
    let mut sa: sigaction = platform_empty_sigaction();
    memset(
        &raw mut sa as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<sigaction>(),
    );
    #[cfg(target_os = "linux")]
    {
        sa.__sigaction_handler.sa_handler = ::core::mem::transmute::<
            ::libc::intptr_t,
            __sighandler_t,
        >(1 as i32 as ::libc::intptr_t);
    }
    #[cfg(target_os = "android")]
    {
        sa.c2rust_unnamed.sa_handler = ::core::mem::transmute::<::libc::intptr_t, __sighandler_t>(
            1 as i32 as ::libc::intptr_t,
        ) as sighandler_t;
    }
    sigaction(SIGINT, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
    sigaction(SIGQUIT, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
    sigaction(SIGHUP, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
    sigaction(SIGPIPE, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
    #[cfg(target_os = "linux")]
    {
        sa.__sigaction_handler.sa_handler =
            Some(on_stop_signal as unsafe extern "C" fn(i32) -> ()) as __sighandler_t;
    }
    #[cfg(target_os = "android")]
    {
        sa.c2rust_unnamed.sa_handler =
            Some(on_stop_signal as unsafe extern "C" fn(i32) -> ()) as sighandler_t;
    }
    sigaction(SIGTERM, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
    #[cfg(target_os = "linux")]
    {
        sa.__sigaction_handler.sa_handler =
            Some(on_alarm as unsafe extern "C" fn(i32) -> ()) as __sighandler_t;
    }
    #[cfg(target_os = "android")]
    {
        sa.c2rust_unnamed.sa_handler =
            Some(on_alarm as unsafe extern "C" fn(i32) -> ()) as sighandler_t;
    }
    sigaction(SIGALRM, &raw mut sa, ::core::ptr::null_mut::<sigaction>());
}
unsafe extern "C" fn reset_child_signals() {
    let mut sa: sigaction = platform_empty_sigaction();
    memset(
        &raw mut sa as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<sigaction>(),
    );
    #[cfg(target_os = "linux")]
    {
        sa.__sigaction_handler.sa_handler = SIG_DFL;
    }
    #[cfg(target_os = "android")]
    {
        sa.c2rust_unnamed.sa_handler = SIG_DFL as sighandler_t;
    }
    let mut sigs: [i32; 6] = [SIGINT, SIGQUIT, SIGHUP, SIGPIPE, SIGTERM, SIGALRM];
    let mut i: usize = 0 as usize;
    while i < ::core::mem::size_of::<[i32; 6]>().wrapping_div(::core::mem::size_of::<i32>()) {
        sigaction(sigs[i], &raw mut sa, ::core::ptr::null_mut::<sigaction>());
        i = i.wrapping_add(1);
    }
    #[cfg(target_os = "linux")]
    let mut none: sigset_t = __sigset_t { __val: [0; 16] };
    #[cfg(all(target_os = "android", target_arch = "x86_64"))]
    let mut none: u64 = 0;
    #[cfg(target_arch = "aarch64")]
    let mut none: sigset_t = sigset_t { sig: [0; 1] };
    sigemptyset(&raw mut none);
    #[cfg(any(target_os = "linux", target_arch = "aarch64"))]
    sigprocmask(
        SIG_SETMASK,
        &raw mut none,
        ::core::ptr::null_mut::<sigset_t>(),
    );
    #[cfg(all(target_os = "android", target_arch = "x86_64"))]
    sigprocmask(SIG_SETMASK, &raw mut none, ::core::ptr::null_mut::<u64>());
}
unsafe extern "C" fn signal_all(mut tr: *mut eng_tracer, mut sig: i32) {
    let mut b: u32 = 0 as u32;
    while b < NBUCKETS as u32 {
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
unsafe extern "C" fn is_marker(mut r: *const eng_regs) -> i32 {
    return (eng_sysno(r) == __NR_getpid as i64
        && eng_arg(r, 0 as i32) as u64 == ENG_MARK_A
        && eng_arg(r, 1 as i32) as u64 == ENG_MARK_B) as i32;
}
unsafe extern "C" fn handle_marker(mut t: *mut eng_task, mut r: *mut eng_regs) {
    let mut op: u64 = eng_arg(r, 2 as i32);
    if op == ENG_MARK_OP_QUERY as u64 {
        if (*t).plan.is_null() {
            eng_task_void(t, r, -(ENOENT as i64));
            return;
        }
        if eng_arg(r, 4 as i32) < (*t).plan_len as u64 {
            eng_task_void(t, r, -(E2BIG as i64));
            return;
        }
        if eng_mem_write(
            (*t).tid,
            eng_arg(r, 3 as i32) as usize,
            (*t).plan,
            (*t).plan_len as usize,
        ) != (*t).plan_len as isize
        {
            eng_task_void(t, r, -(EFAULT as i64));
            return;
        }
        let mut len: i64 = (*t).plan_len as i64;
        eng_exec_discard(t);
        eng_task_void(t, r, len);
    } else if op == ENG_MARK_OP_DONE as u64 {
        eng_scratch_release(t);
        mm_put((*t).mm);
        let mut mm: *mut eng_mm =
            calloc(1 as usize, ::core::mem::size_of::<eng_mm>()) as *mut eng_mm;
        (*mm).refs = 1 as i32;
        (*mm).base = eng_arg(r, 3 as i32);
        (*mm).size = eng_arg(r, 4 as i32) as u32;
        (*mm).nslots = (*mm).size.wrapping_div(ENG_SLOT_SIZE as u32);
        if (*mm).nslots as usize > ::core::mem::size_of::<[u8; 512]>() {
            (*mm).nslots = ::core::mem::size_of::<[u8; 512]>() as u32;
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
            (*mm).base as u64,
        );
        eng_task_void(t, r, 0 as i64);
    } else {
        eng_task_void(t, r, -(EINVAL as i64));
    };
}
unsafe extern "C" fn syscall_stop(
    mut tr: *mut eng_tracer,
    mut t: *mut eng_task,
    mut seccomp_event: i32,
) {
    let mut r: eng_regs = platform_empty_eng_regs();
    if eng_regs_get((*t).tid, &raw mut r) != 0 as i32 {
        resume_mode((*t).tid, 0 as i32, 0 as i32);
        return;
    }
    (*tr).syscall_stops = (*tr).syscall_stops.wrapping_add(1);
    let mut entry: i32 = (*t).at_syscall_entry;
    if seccomp_event != 0 {
        (*tr).seccomp_stops = (*tr).seccomp_stops.wrapping_add(1);
        if (*t).at_syscall_entry == 0 {
            resume_task(t, 0 as i32);
            return;
        }
        entry = 1 as i32;
    }
    if (*tr).sysinfo != 0 as i32 && seccomp_event == 0 {
        let mut op: i32 = eng_syscall_info_op((*t).tid);
        if op < 0 as i32 {
            (*tr).sysinfo = 0 as i32;
        } else {
            (*tr).sysinfo = 1 as i32;
            let mut k_entry: i32 = (op == 1 as i32) as i32;
            if op == 1 as i32 || op == 2 as i32 {
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
        (*t).at_syscall_entry = 0 as i32;
        (*t).sysno = eng_sysno(&raw mut r);
        (*t).entry_regs = r;
        (*t).regs_modified = 0 as i32;
        (*t).void_pending = 0 as i32;
        (*t).fixup = ENG_FIX_NONE as i32;
        if (*t).lg_restart != 0 && (*t).sysno != (*t).lg_nr {
            (*t).lg_restart = 0 as i32;
        }
        if (*t).lg_restart == 0 {
            (*t).slot_off = 0 as u32;
            (*t).stack_scratch = 0 as u64;
            (*t).lg_fix = 0 as i32;
        }
        let mut changed: i32 = 0 as i32;
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
            || (*t).umask_old >= 0 as i32
            || (*t).lg_restart != 0
            || (*t).sysno == __NR_execve as i64
            || (*t).sysno == __NR_execveat as i64) as i32;
        if (*t).await_exit == 0 {
            (*t).at_syscall_entry = 1 as i32;
        }
    } else {
        (*t).at_syscall_entry = 1 as i32;
        (*t).await_exit = 0 as i32;
        let mut changed_0: i32 = 0 as i32;
        if (*t).phase.0 == eng_phase::ENG_PH_GUEST.0 && !(*tr).guest.is_null() {
            changed_0 = eng_sys_exit(t, &raw mut r);
        }
        if (*t).void_pending != 0 {
            eng_set_ret(&raw mut r, (*t).inject_result as u64);
            (*t).void_pending = 0 as i32;
            changed_0 = 1 as i32;
        }
        if (*t).regs_modified != 0 {
            eng_restore_args(&raw mut r, &raw mut (*t).entry_regs);
            (*t).regs_modified = 0 as i32;
            changed_0 = 1 as i32;
        }
        if (*t).lg_restart != 0 {
            eng_restore_args(&raw mut r, &raw mut (*t).lg_saved);
            (*t).lg_restart = 0 as i32;
            changed_0 = 1 as i32;
        }
        if !(*t).plan.is_null()
            && (eng_ret(&raw mut r) as i64) < 0 as i64
            && ((*t).sysno == __NR_execve as i64 || (*t).sysno == __NR_execveat as i64)
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
                eng_arg(&raw mut (*t).entry_regs, 0 as i32) as u64,
                eng_arg(&raw mut (*t).entry_regs, 1 as i32) as u64,
                eng_arg(&raw mut (*t).entry_regs, 2 as i32) as u64,
                eng_arg(&raw mut (*t).entry_regs, 3 as i32) as u64,
                eng_ret(&raw mut r) as i64,
            );
        }
        if changed_0 != 0 {
            eng_regs_set((*t).tid, &raw mut r);
        }
    }
    resume_task(t, 0 as i32);
}
unsafe extern "C" fn sigsys_stop(mut t: *mut eng_task) {
    let mut si: siginfo_t = platform_empty_siginfo_t();
    let mut r: eng_regs = platform_empty_eng_regs();
    if ptrace(PTRACE_GETSIGINFO, (*t).tid, 0 as i32, &raw mut si) != 0 as i64
        || eng_regs_get((*t).tid, &raw mut r) != 0 as i32
    {
        resume_task(t, SIGSYS);
        return;
    }
    if eng_sys_sigsys(t, &raw mut r, &raw mut si) != 0 {
        eng_regs_set((*t).tid, &raw mut r);
        if (*t).lg_restart != 0 {
            (*t).at_syscall_entry = 1 as i32;
            (*t).await_exit = 1 as i32;
        }
        resume_task(t, 0 as i32);
    } else {
        resume_task(t, SIGSYS);
    };
}
unsafe extern "C" fn on_new_child(
    mut tr: *mut eng_tracer,
    mut parent: *mut eng_task,
    mut ctid: i32,
    mut event: i32,
) {
    let mut c: *mut eng_task = eng_task_find(tr, ctid);
    if c.is_null() {
        c = task_new(tr, ctid);
    }
    (*c).linked = 1 as i32;
    let mut flags: u64 = 0 as u64;
    #[cfg(target_os = "linux")]
    if event == __ptrace_eventcodes::PTRACE_EVENT_CLONE.0 as i32 {
        let mut pr: eng_regs = platform_empty_eng_regs();
        if eng_regs_get((*parent).tid, &raw mut pr) == 0 as i32 {
            flags = eng_arg(&raw mut pr, 0 as i32) as u64;
        }
    }
    #[cfg(target_os = "android")]
    if event == PTRACE_EVENT_CLONE {
        let mut pr: eng_regs = platform_empty_eng_regs();
        if eng_regs_get((*parent).tid, &raw mut pr) == 0 as i32 {
            flags = eng_arg(&raw mut pr, 0 as i32) as u64;
        }
    }
    #[cfg(target_os = "linux")]
    let mut thread: i32 = (event == __ptrace_eventcodes::PTRACE_EVENT_CLONE.0 as i32
        && flags & CLONE_THREAD as u64 != 0) as i32;
    #[cfg(target_os = "android")]
    let mut thread: i32 = (event == PTRACE_EVENT_CLONE && flags & CLONE_THREAD as u64 != 0) as i32;
    #[cfg(target_os = "linux")]
    let mut share_vm: i32 = (event == __ptrace_eventcodes::PTRACE_EVENT_VFORK.0 as i32
        || event == __ptrace_eventcodes::PTRACE_EVENT_CLONE.0 as i32
            && flags & CLONE_VM as u64 != 0) as i32;
    #[cfg(target_os = "android")]
    let mut share_vm: i32 = (event == PTRACE_EVENT_VFORK
        || event == PTRACE_EVENT_CLONE && flags & CLONE_VM as u64 != 0)
        as i32;
    (*c).tgid = if thread != 0 { (*parent).tgid } else { ctid };
    (*c).phase = (*parent).phase;
    (*c).cr = (*parent).cr;
    if !(*parent).mm.is_null() {
        if share_vm != 0 {
            (*c).mm = (*parent).mm;
            (*(*c).mm).refs += 1;
        } else {
            (*c).mm = calloc(1 as usize, ::core::mem::size_of::<eng_mm>()) as *mut eng_mm;
            (*(*c).mm).refs = 1 as i32;
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
        (*c).held = 0 as i32;
        resume_mode(ctid, 0 as i32, 0 as i32);
    }
}
unsafe extern "C" fn on_exec(mut tr: *mut eng_tracer, mut t: *mut eng_task) {
    let mut former: u64 = 0 as u64;
    ptrace(PTRACE_GETEVENTMSG, (*t).tid, 0 as i32, &raw mut former);
    if former != 0 && former as i32 != (*t).tid {
        let mut o: *mut eng_task = eng_task_find(tr, former as i32);
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
            task_del(tr, former as i32);
        }
    }
    eng_scratch_release(t);
    mm_put((*t).mm);
    (*t).mm = ::core::ptr::null_mut::<eng_mm>();
    (*t).regs_modified = 0 as i32;
    (*t).void_pending = 0 as i32;
    (*t).phase = eng_phase(
        (if !(*tr).guest.is_null() {
            eng_phase::ENG_PH_LOADER.0 as i32
        } else {
            eng_phase::ENG_PH_BOOT.0 as i32
        }) as u32,
    );
    if !(*tr).guest.is_null() {
        eng_ident_exec(t);
    }
}
unsafe extern "C" fn never_allow(mut nr: i64) -> i32 {
    #[cfg(target_arch = "x86_64")]
    static mut list: [i64; 17] = [
        __NR_getpid as i64,
        __NR_prctl as i64,
        __NR_socket as i64,
        __NR_umask as i64,
        __NR_time as i64,
        __NR_getpgrp as i64,
        __NR_alarm as i64,
        __NR_pipe as i64,
        __NR_dup2 as i64,
        __NR_poll as i64,
        __NR_select as i64,
        __NR_eventfd as i64,
        __NR_signalfd as i64,
        __NR_epoll_create as i64,
        __NR_epoll_wait as i64,
        __NR_inotify_init as i64,
        __NR_getdents as i64,
    ];
    #[cfg(target_arch = "aarch64")]
    static mut list: [i64; 4] = [
        __NR_getpid as i64,
        __NR_prctl as i64,
        __NR_socket as i64,
        __NR_umask as i64,
    ];
    let mut i: usize = 0 as usize;
    #[cfg(target_arch = "x86_64")]
    while i < ::core::mem::size_of::<[i64; 17]>().wrapping_div(::core::mem::size_of::<i64>()) {
        if list[i] == nr {
            return 1 as i32;
        }
        i = i.wrapping_add(1);
    }
    #[cfg(target_arch = "aarch64")]
    while i < ::core::mem::size_of::<[i64; 4]>().wrapping_div(::core::mem::size_of::<i64>()) {
        if list[i] == nr {
            return 1 as i32;
        }
        i = i.wrapping_add(1);
    }
    return 0 as i32;
}
unsafe extern "C" fn fast_allowed(mut nr: i64) -> i32 {
    if never_allow(nr) != 0 {
        return 0 as i32;
    }
    match eng_sysinv_class(nr) {
        eng_sc_class::ENG_SC_PASS | eng_sc_class::ENG_SC_FD | eng_sc_class::ENG_SC_PROC => {
            return 1 as i32;
        }
        _ => {}
    }
    return (nr == __NR_recvfrom as i64 || nr == __NR_recvmsg as i64 || nr == __NR_recvmmsg as i64)
        as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_fast_filter_count() -> i32 {
    let mut n: i32 = 0 as i32;
    let mut nr: i64 = 0 as i64;
    while nr <= eng_sysinv_max() {
        n += fast_allowed(nr);
        nr += 1;
    }
    return n;
}
unsafe extern "C" fn install_fast_filter() -> i32 {
    let mut f: [sock_filter; 1024] = [sock_filter {
        code: 0,
        jt: 0,
        jf: 0,
        k: 0,
    }; 1024];
    let mut n: u32 = 0 as u32;
    let TRACE: u32 = SECCOMP_RET_TRACE as u32;
    let ALLOW: u32 = SECCOMP_RET_ALLOW as u32;
    let NOSYS: u32 = SECCOMP_RET_ERRNO as u32 | ENOSYS as u32;
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    let c2rust_fresh0 = n;
    n = n.wrapping_add(1);
    f[c2rust_fresh0 as usize] = sock_filter {
        code: (0 as i32 | 0 as i32 | 0x20 as i32) as u16,
        jt: 0 as u8,
        jf: 0 as u8,
        k: 4 as u64 as u32,
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
        code: (0x5 as i32 | 0x10 as i32 | 0 as i32) as u16,
        jt: 1 as u8,
        jf: 0 as u8,
        #[cfg(target_arch = "x86_64")]
        k: 62 as u32 | 0x80000000 as u32 | 0x40000000 as i32 as u32,
        #[cfg(target_arch = "aarch64")]
        k: 183 as u32 | 0x80000000 as u32 | 0x40000000 as i32 as u32,
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
        code: (0x6 as i32 | 0 as i32) as u16,
        jt: 0 as u8,
        jf: 0 as u8,
        k: NOSYS as u32,
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
        code: (0 as i32 | 0 as i32 | 0x20 as i32) as u16,
        jt: 0 as u8,
        jf: 0 as u8,
        k: 0 as u64 as u32,
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
        #[cfg(target_arch = "x86_64")]
        code: (0x5 as i32 | 0x30 as i32 | 0 as i32) as u16,
        #[cfg(target_arch = "aarch64")]
        code: (0x5 as i32 | 0x10 as i32 | 0 as i32) as u16,
        jt: 0 as u8,
        #[cfg(target_arch = "x86_64")]
        jf: 1 as u8,
        #[cfg(target_arch = "aarch64")]
        jf: 6 as u8,
        #[cfg(target_arch = "x86_64")]
        k: 0x40000000 as u32,
        #[cfg(target_arch = "aarch64")]
        k: 172 as u32,
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
        #[cfg(target_arch = "x86_64")]
        code: (0x6 as i32 | 0 as i32) as u16,
        #[cfg(target_arch = "aarch64")]
        code: (0 as i32 | 0 as i32 | 0x20 as i32) as u16,
        jt: 0 as u8,
        jf: 0 as u8,
        #[cfg(target_arch = "x86_64")]
        k: TRACE as u32,
        #[cfg(target_arch = "aarch64")]
        k: 16 as u64 as u32,
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
        code: (0x5 as i32 | 0x10 as i32 | 0 as i32) as u16,
        jt: 0 as u8,
        #[cfg(target_arch = "x86_64")]
        jf: 6 as u8,
        #[cfg(target_arch = "aarch64")]
        jf: 3 as u8,
        #[cfg(target_arch = "x86_64")]
        k: 39 as u32,
        #[cfg(target_arch = "aarch64")]
        k: 0x574f524b464c4f57 as u64 as u32,
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
        code: (0 as i32 | 0 as i32 | 0x20 as i32) as u16,
        jt: 0 as u8,
        jf: 0 as u8,
        #[cfg(target_arch = "x86_64")]
        k: 16 as u64 as u32,
        #[cfg(target_arch = "aarch64")]
        k: (16 as u64).wrapping_add(4 as u64) as u32,
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
        code: (0x5 as i32 | 0x10 as i32 | 0 as i32) as u16,
        jt: 0 as u8,
        #[cfg(target_arch = "x86_64")]
        jf: 3 as u8,
        #[cfg(target_arch = "aarch64")]
        jf: 1 as u8,
        #[cfg(target_arch = "x86_64")]
        k: 0x574f524b464c4f57 as u64 as u32,
        #[cfg(target_arch = "aarch64")]
        k: (0x574f524b464c4f57 as u64 >> 32 as i32) as u32,
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
        #[cfg(target_arch = "x86_64")]
        code: (0 as i32 | 0 as i32 | 0x20 as i32) as u16,
        #[cfg(target_arch = "aarch64")]
        code: (0x6 as i32 | 0 as i32) as u16,
        jt: 0 as u8,
        jf: 0 as u8,
        #[cfg(target_arch = "x86_64")]
        k: (16 as u64).wrapping_add(4 as u64) as u32,
        #[cfg(target_arch = "aarch64")]
        k: TRACE as u32,
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
        #[cfg(target_arch = "x86_64")]
        code: (0x5 as i32 | 0x10 as i32 | 0 as i32) as u16,
        #[cfg(target_arch = "aarch64")]
        code: (0x6 as i32 | 0 as i32) as u16,
        jt: 0 as u8,
        #[cfg(target_arch = "x86_64")]
        jf: 1 as u8,
        #[cfg(target_arch = "aarch64")]
        jf: 0 as u8,
        #[cfg(target_arch = "x86_64")]
        k: (0x574f524b464c4f57 as u64 >> 32 as i32) as u32,
        #[cfg(target_arch = "aarch64")]
        k: ALLOW as u32,
    };
    #[cfg(target_arch = "aarch64")]
    let mut nr: i64 = 0 as i64;
    #[cfg(target_arch = "aarch64")]
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
                code: (0x5 as i32 | 0x10 as i32 | 0 as i32) as u16,
                jt: 0 as u8,
                jf: 1 as u8,
                k: nr as u32,
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
                code: (0x6 as i32 | 0 as i32) as u16,
                jt: 0 as u8,
                jf: 0 as u8,
                k: ALLOW as u32,
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
    #[cfg(target_arch = "x86_64")]
    let c2rust_fresh11 = n;
    #[cfg(target_arch = "aarch64")]
    let c2rust_fresh13 = n;
    n = n.wrapping_add(1);
    #[cfg(target_arch = "x86_64")]
    {
        f[c2rust_fresh11 as usize] = sock_filter {
            code: (0x6 as i32 | 0 as i32) as u16,
            jt: 0 as u8,
            jf: 0 as u8,
            k: TRACE as u32,
        };
    }
    #[cfg(target_arch = "aarch64")]
    {
        f[c2rust_fresh13 as usize] = sock_filter {
            code: (0x6 as i32 | 0 as i32) as u16,
            jt: 0 as u8,
            jf: 0 as u8,
            k: TRACE as u32,
        };
    }
    #[cfg(target_arch = "x86_64")]
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    #[cfg(target_arch = "x86_64")]
    let c2rust_fresh12 = n;
    #[cfg(target_arch = "x86_64")]
    {
        n = n.wrapping_add(1);
    }
    #[cfg(target_arch = "x86_64")]
    {
        f[c2rust_fresh12 as usize] = sock_filter {
            code: (0x6 as i32 | 0 as i32) as u16,
            jt: 0 as u8,
            jf: 0 as u8,
            k: ALLOW as u32,
        };
    }
    #[cfg(target_arch = "x86_64")]
    let mut nr: i64 = 0 as i64;
    #[cfg(target_arch = "x86_64")]
    while nr <= eng_sysinv_max() {
        if fast_allowed(nr) != 0 {
            if n as usize
                >= ::core::mem::size_of::<[sock_filter; 1024]>()
                    .wrapping_div(::core::mem::size_of::<sock_filter>())
            {
                return -E2BIG;
            }
            let c2rust_fresh13 = n;
            n = n.wrapping_add(1);
            f[c2rust_fresh13 as usize] = sock_filter {
                code: (0x5 as i32 | 0x10 as i32 | 0 as i32) as u16,
                jt: 0 as u8,
                jf: 1 as u8,
                k: nr as u32,
            };
            if n as usize
                >= ::core::mem::size_of::<[sock_filter; 1024]>()
                    .wrapping_div(::core::mem::size_of::<sock_filter>())
            {
                return -E2BIG;
            }
            let c2rust_fresh14 = n;
            n = n.wrapping_add(1);
            f[c2rust_fresh14 as usize] = sock_filter {
                code: (0x6 as i32 | 0 as i32) as u16,
                jt: 0 as u8,
                jf: 0 as u8,
                k: ALLOW as u32,
            };
        }
        nr += 1;
    }
    #[cfg(target_arch = "x86_64")]
    if n as usize
        >= ::core::mem::size_of::<[sock_filter; 1024]>()
            .wrapping_div(::core::mem::size_of::<sock_filter>())
    {
        return -E2BIG;
    }
    #[cfg(target_arch = "x86_64")]
    let c2rust_fresh15 = n;
    #[cfg(target_arch = "x86_64")]
    {
        n = n.wrapping_add(1);
    }
    #[cfg(target_arch = "x86_64")]
    {
        f[c2rust_fresh15 as usize] = sock_filter {
            code: (0x6 as i32 | 0 as i32) as u16,
            jt: 0 as u8,
            jf: 0 as u8,
            k: TRACE as u32,
        };
    }
    let mut prog: sock_fprog = sock_fprog {
        len: n as u16,
        filter: &raw mut f as *mut sock_filter,
    };
    if prctl(PR_SET_NO_NEW_PRIVS, 1 as i32, 0 as i32, 0 as i32, 0 as i32) != 0 as i32 {
        return -*errno();
    }
    if prctl(
        PR_SET_SECCOMP,
        SECCOMP_MODE_FILTER,
        &raw mut prog,
        0 as i32,
        0 as i32,
    ) != 0 as i32
    {
        return -*errno();
    }
    return 0 as i32;
}
unsafe extern "C" fn install_test_app_filter() -> i32 {
    #[cfg(target_arch = "x86_64")]
    static mut trapped: [i32; 88] = [
        __NR_access,
        __NR_open,
        __NR_stat,
        __NR_lstat,
        __NR_creat,
        __NR_mkdir,
        __NR_rmdir,
        __NR_unlink,
        __NR_rename,
        __NR_link,
        __NR_symlink,
        __NR_readlink,
        __NR_chmod,
        __NR_chown,
        __NR_lchown,
        __NR_mknod,
        __NR_utimes,
        __NR_futimesat,
        __NR_dup2,
        __NR_pipe,
        __NR_poll,
        __NR_select,
        __NR_getdents,
        __NR_getpgrp,
        __NR_epoll_create,
        __NR_epoll_wait,
        __NR_inotify_init,
        __NR_eventfd,
        __NR_signalfd,
        __NR_alarm,
        __NR_time,
        __NR_uselib,
        __NR_set_robust_list,
        __NR_rseq,
        __NR_statx,
        __NR_faccessat2,
        __NR_clone3,
        __NR_openat2,
        __NR_close_range,
        __NR_pidfd_open,
        __NR_pidfd_getfd,
        __NR_pidfd_send_signal,
        __NR_epoll_pwait2,
        __NR_process_madvise,
        __NR_mount_setattr,
        __NR_landlock_create_ruleset,
        __NR_memfd_secret,
        __NR_futex_waitv,
        __NR_cachestat,
        __NR_fchmodat2,
        __NR_map_shadow_stack,
        __NR_pkey_alloc,
        __NR_pkey_mprotect,
        __NR_io_uring_setup,
        __NR_io_pgetevents,
        __NR_membarrier,
        __NR_userfaultfd,
        __NR_kcmp,
        __NR_name_to_handle_at,
        __NR_open_by_handle_at,
        __NR_bpf,
        __NR_kexec_file_load,
        __NR_add_key,
        __NR_keyctl,
        __NR_get_mempolicy,
        __NR_set_mempolicy,
        __NR_mbind,
        __NR_migrate_pages,
        __NR_move_pages,
        __NR_fanotify_init,
        __NR_clock_adjtime,
        __NR_adjtimex,
        __NR_acct,
        __NR_swapon,
        __NR_chroot,
        __NR_mount,
        __NR_umount2,
        __NR_open_tree,
        __NR_fsopen,
        __NR_setuid,
        __NR_setgid,
        __NR_setreuid,
        __NR_setregid,
        __NR_setresuid,
        __NR_setresgid,
        __NR_setfsuid,
        __NR_setfsgid,
        __NR_setgroups,
    ];
    #[cfg(target_arch = "aarch64")]
    return -ENOSYS;
    #[cfg(target_arch = "x86_64")]
    let mut f: [sock_filter; 256] = [sock_filter {
        code: 0,
        jt: 0,
        jf: 0,
        k: 0,
    }; 256];
    #[cfg(target_arch = "x86_64")]
    let mut n: u32 = 0 as u32;
    #[cfg(target_arch = "x86_64")]
    let mut cnt: u32 =
        ::core::mem::size_of::<[i32; 88]>().wrapping_div(::core::mem::size_of::<i32>()) as u32;
    #[cfg(target_arch = "x86_64")]
    let c2rust_fresh16 = n;
    #[cfg(target_arch = "x86_64")]
    {
        n = n.wrapping_add(1);
    }
    #[cfg(target_arch = "x86_64")]
    {
        f[c2rust_fresh16 as usize] = sock_filter {
            code: (0 as i32 | 0 as i32 | 0x20 as i32) as u16,
            jt: 0 as u8,
            jf: 0 as u8,
            k: 0 as u64 as u32,
        };
    }
    #[cfg(target_arch = "x86_64")]
    let mut i: u32 = 0 as u32;
    #[cfg(target_arch = "x86_64")]
    while i < cnt {
        let c2rust_fresh17 = n;
        n = n.wrapping_add(1);
        f[c2rust_fresh17 as usize] = sock_filter {
            code: (0x5 as i32 | 0x10 as i32 | 0 as i32) as u16,
            jt: 0 as u8,
            jf: 1 as u8,
            k: trapped[i as usize] as u32,
        };
        let c2rust_fresh18 = n;
        n = n.wrapping_add(1);
        f[c2rust_fresh18 as usize] = sock_filter {
            code: (0x6 as i32 | 0 as i32) as u16,
            jt: 0 as u8,
            jf: 0 as u8,
            k: 0x30000 as u32,
        };
        i = i.wrapping_add(1);
    }
    #[cfg(target_arch = "x86_64")]
    let c2rust_fresh19 = n;
    #[cfg(target_arch = "x86_64")]
    {
        n = n.wrapping_add(1);
    }
    #[cfg(target_arch = "x86_64")]
    {
        f[c2rust_fresh19 as usize] = sock_filter {
            code: (0x6 as i32 | 0 as i32) as u16,
            jt: 0 as u8,
            jf: 0 as u8,
            k: 0x7fff0000 as u32,
        };
    }
    #[cfg(target_arch = "x86_64")]
    let mut prog: sock_fprog = sock_fprog {
        len: n as u16,
        filter: &raw mut f as *mut sock_filter,
    };
    #[cfg(target_arch = "x86_64")]
    if prctl(PR_SET_NO_NEW_PRIVS, 1 as i32, 0 as i32, 0 as i32, 0 as i32) != 0 as i32 {
        return -*errno();
    }
    #[cfg(target_arch = "x86_64")]
    return if prctl(
        PR_SET_SECCOMP,
        SECCOMP_MODE_FILTER,
        &raw mut prog,
        0 as i32,
        0 as i32,
    ) == 0 as i32
    {
        0 as i32
    } else {
        -*errno()
    };
}
unsafe extern "C" fn kernel_supports_fast() -> i32 {
    let mut u: utsname = utsname {
        sysname: [0; 65],
        nodename: [0; 65],
        release: [0; 65],
        version: [0; 65],
        machine: [0; 65],
        domainname: [0; 65],
    };
    if uname(&raw mut u) != 0 as i32 {
        return 0 as i32;
    }
    let mut maj: i32 = 0 as i32;
    let mut min: i32 = 0 as i32;
    if sscanf(
        &raw mut u.release as *mut ::core::ffi::c_char,
        b"%d.%d\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut maj,
        &raw mut min,
    ) != 2 as i32
    {
        return 0 as i32;
    }
    return (maj > 4 as i32 || maj == 4 as i32 && min >= 8 as i32) as i32;
}
#[cfg(target_os = "linux")]
static mut SEIZE_OPTS: i64 = (__ptrace_setoptions::PTRACE_O_TRACESYSGOOD.0 as i32
    | __ptrace_setoptions::PTRACE_O_TRACEFORK.0 as i32
    | __ptrace_setoptions::PTRACE_O_TRACEVFORK.0 as i32
    | __ptrace_setoptions::PTRACE_O_TRACECLONE.0 as i32
    | __ptrace_setoptions::PTRACE_O_TRACEEXEC.0 as i32
    | __ptrace_setoptions::PTRACE_O_TRACEEXIT.0 as i32
    | __ptrace_setoptions::PTRACE_O_EXITKILL.0 as i32) as i64;
#[cfg(target_os = "android")]
static mut SEIZE_OPTS: i64 = (PTRACE_O_TRACESYSGOOD
    | PTRACE_O_TRACEFORK
    | PTRACE_O_TRACEVFORK
    | PTRACE_O_TRACECLONE
    | PTRACE_O_TRACEEXEC
    | PTRACE_O_TRACEEXIT
    | PTRACE_O_EXITKILL) as i64;
unsafe extern "C" fn chdir_guest(mut cfg: *const eng_run_cfg) -> i32 {
    if (*cfg).guest.is_null() {
        return if !(*cfg).cwd.is_null() {
            chdir((*cfg).cwd)
        } else {
            0 as i32
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
        return -1 as i32;
    }
    return chdir(&raw mut host as *mut ::core::ffi::c_char);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_tracer_run(mut cfg: *const eng_run_cfg) -> i32 {
    let mut tr: *mut eng_tracer =
        calloc(1 as usize, ::core::mem::size_of::<eng_tracer>()) as *mut eng_tracer;
    (*tr).guest = (*cfg).guest;
    (*tr).sysinfo = -1 as i32;
    let mut ct: *const ::core::ffi::c_char =
        getenv(b"WORKFLOW_ENGINE_CHECK_TOGGLE\0".as_ptr() as *const ::core::ffi::c_char);
    (*tr).check_toggle = (!ct.is_null() && *ct as i32 == '1' as i32) as i32;
    let mut ready: [i32; 2] = [0; 2];
    let mut go: [i32; 2] = [0; 2];
    if pipe2(&raw mut ready as *mut i32, O_CLOEXEC) != 0
        || pipe2(&raw mut go as *mut i32, O_CLOEXEC) != 0
    {
        eng_logf!(
            eng_log_level::ENG_LOG_ERROR,
            b"pipe: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*errno()),
        );
        return -1 as i32;
    }
    install_engine_signals();
    prctl(
        PR_SET_CHILD_SUBREAPER,
        1 as i32,
        0 as i32,
        0 as i32,
        0 as i32,
    );
    let mut fast_wanted: i32 = ((*cfg).use_seccomp_fastpath != 0 && !(*cfg).guest.is_null()) as i32;
    if fast_wanted != 0 && kernel_supports_fast() == 0 {
        eng_logf!(
            eng_log_level::ENG_LOG_INFO,
            b"kernel older than 4.8: seccomp fast path off\0".as_ptr()
                as *const ::core::ffi::c_char,
        );
        fast_wanted = 0 as i32;
    }
    if !(*cfg).guest.is_null() {
        eng_link_recover((*cfg).guest);
    }
    let mut child: i32 = fork();
    if child < 0 as i32 {
        eng_logf!(
            eng_log_level::ENG_LOG_ERROR,
            b"fork: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*errno()),
        );
        return -1 as i32;
    }
    if child == 0 as i32 {
        reset_child_signals();
        close(ready[0usize]);
        close(go[1usize]);
        umask(0o22 as u32);
        if chdir_guest(cfg) != 0 as i32 {
            fprintf(
                stderr,
                b"workflow-engine: cannot enter working directory %s: %s\n\0".as_ptr()
                    as *const ::core::ffi::c_char,
                if !(*cfg).cwd.is_null() {
                    (*cfg).cwd
                } else {
                    b"/\0".as_ptr() as *const ::core::ffi::c_char
                },
                strerror(*errno()),
            );
            _exit(126 as i32);
        }
        if (*cfg).test_app_filter != 0 {
            let mut arc: i32 = install_test_app_filter();
            if arc != 0 {
                fprintf(
                    stderr,
                    b"workflow-engine: test app filter: %s\n\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    strerror(-arc),
                );
                _exit(126 as i32);
            }
        }
        let mut c: ::core::ffi::c_char = 'P' as ::core::ffi::c_char;
        if fast_wanted != 0 {
            let mut frc: i32 = install_fast_filter();
            if frc == 0 as i32 {
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
            1 as usize,
        ) != 1 as isize
        {
            _exit(126 as i32);
        }
        if read(
            go[0usize],
            &raw mut c as *mut ::core::ffi::c_void,
            1 as usize,
        ) != 1 as isize
        {
            _exit(126 as i32);
        }
        if !(*cfg).guest.is_null() {
            #[cfg(target_os = "linux")]
            execve(
                &raw mut (*(*cfg).guest).loader as *mut ::core::ffi::c_char,
                (*cfg).argv as *const *mut ::core::ffi::c_char,
                (*cfg).envp as *const *mut ::core::ffi::c_char,
            );
            #[cfg(target_os = "android")]
            execve(
                &raw mut (*(*cfg).guest).loader as *mut ::core::ffi::c_char,
                (*cfg).argv,
                (*cfg).envp,
            );
            fprintf(
                stderr,
                b"workflow-engine: exec loader %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*(*cfg).guest).loader as *mut ::core::ffi::c_char,
                strerror(*errno()),
            );
            _exit(127 as i32);
        }
        #[cfg(target_os = "linux")]
        execvpe(
            *(*cfg).argv.offset(0isize),
            (*cfg).argv as *const *mut ::core::ffi::c_char,
            (*cfg).envp as *const *mut ::core::ffi::c_char,
        );
        #[cfg(target_os = "android")]
        execvpe(*(*cfg).argv.offset(0isize), (*cfg).argv, (*cfg).envp);
        fprintf(
            stderr,
            b"workflow-engine: exec %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            *(*cfg).argv.offset(0isize),
            strerror(*errno()),
        );
        _exit(127 as i32);
    }
    close(ready[1usize]);
    close(go[0usize]);
    (*tr).main_pid = child;
    let mut c_0: ::core::ffi::c_char = 0;
    if read(
        ready[0usize],
        &raw mut c_0 as *mut ::core::ffi::c_void,
        1 as usize,
    ) != 1 as isize
    {
        let mut st: i32 = 0;
        waitpid(child, &raw mut st, 0 as i32);
        return if st & 0x7f as i32 == 0 as i32 {
            (st & 0xff00 as i32) >> 8 as i32
        } else {
            -1 as i32
        };
    }
    close(ready[0usize]);
    g_fast = (c_0 as i32 == 'S' as i32) as i32;
    (*tr).fast = g_fast;
    let mut opts: i64 = SEIZE_OPTS
        | (if (*tr).fast != 0 {
            PTRACE_O_TRACESECCOMP
        } else {
            0 as i32
        }) as i64;
    if ptrace(
        PTRACE_SEIZE,
        child,
        0 as i32,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(opts as usize),
    ) != 0 as i64
    {
        eng_logf!(
            eng_log_level::ENG_LOG_ERROR,
            b"PTRACE_SEIZE: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*errno()),
        );
        kill(child, SIGKILL);
        return -1 as i32;
    }
    let mut m: *mut eng_task = task_new(tr, child);
    (*m).linked = 1 as i32;
    (*m).phase = eng_phase::ENG_PH_BOOT;
    eng_ident_init((*cfg).guest, m, (*cfg).uid, (*cfg).gid);
    if !(*cfg).guest.is_null() {
        let mut rc: i32 = eng_exec_prepare_initial(
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
            waitpid(child, ::core::ptr::null_mut::<i32>(), __WALL);
            return if rc == -ENOENT {
                127 as i32
            } else {
                126 as i32
            };
        }
    }
    ptrace(PTRACE_INTERRUPT, child, 0 as i32, 0 as i32);
    let mut status: i32 = 0;
    while waitpid(child, &raw mut status, __WALL) < 0 as i32 && *errno() == EINTR {}
    resume_mode(child, 0 as i32, 0 as i32);
    if write(
        go[1usize],
        &raw mut c_0 as *const ::core::ffi::c_void,
        1 as usize,
    ) != 1 as isize
    {
        eng_logf!(
            eng_log_level::ENG_LOG_WARN,
            b"release child: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*errno()),
        );
    }
    close(go[1usize]);
    let mut stop_forwarded: i32 = 0 as i32;
    loop {
        if ::core::ptr::read_volatile::<i32>(&raw const g_stop_sig) != 0 && stop_forwarded == 0 {
            eng_logf!(
                eng_log_level::ENG_LOG_INFO,
                b"stop requested (signal %d): forwarding to guest\0".as_ptr()
                    as *const ::core::ffi::c_char,
                ::core::ptr::read_volatile::<i32>(&raw const g_stop_sig),
            );
            signal_all(tr, ::core::ptr::read_volatile::<i32>(&raw const g_stop_sig));
            stop_forwarded = 1 as i32;
            let mut gs: *const ::core::ffi::c_char =
                getenv(b"WORKFLOW_ENGINE_STOP_GRACE\0".as_ptr() as *const ::core::ffi::c_char);
            alarm(if !gs.is_null() {
                atoi(gs) as u32
            } else {
                3 as u32
            });
        }
        if ::core::ptr::read_volatile::<i32>(&raw const g_alarm) != 0 {
            ::core::ptr::write_volatile(&raw mut g_alarm, 0 as i32 as i32);
            eng_logf!(
                eng_log_level::ENG_LOG_INFO,
                b"grace period over: killing guest tree\0".as_ptr() as *const ::core::ffi::c_char,
            );
            signal_all(tr, SIGKILL);
        }
        let mut tid: i32 = waitpid(-1 as i32, &raw mut status, __WALL);
        if tid < 0 as i32 {
            if *errno() == EINTR {
                continue;
            }
            if *errno() == ECHILD {
                break;
            }
            eng_logf!(
                eng_log_level::ENG_LOG_ERROR,
                b"waitpid: %s\0".as_ptr() as *const ::core::ffi::c_char,
                strerror(*errno()),
            );
            break;
        } else if platform_expr_wait_finished(status) {
            if tid == (*tr).main_pid {
                (*tr).main_status = if status & 0x7f as i32 == 0 as i32 {
                    (status & 0xff00 as i32) >> 8 as i32
                } else {
                    128 as i32 + (status & 0x7f as i32)
                };
                (*tr).main_seen = 1 as i32;
                task_del(tr, tid);
                if (*cfg).wait_all == 0 && (*tr).ntasks != 0 && stop_forwarded == 0 {
                    eng_logf!(
                        eng_log_level::ENG_LOG_INFO,
                        b"main process exited: terminating %zu remaining task(s)\0".as_ptr()
                            as *const ::core::ffi::c_char,
                        (*tr).ntasks,
                    );
                    signal_all(tr, SIGTERM);
                    stop_forwarded = 1 as i32;
                    let mut gs_0: *const ::core::ffi::c_char = getenv(
                        b"WORKFLOW_ENGINE_STOP_GRACE\0".as_ptr() as *const ::core::ffi::c_char,
                    );
                    alarm(if !gs_0.is_null() {
                        atoi(gs_0) as u32
                    } else {
                        3 as u32
                    });
                }
            } else {
                task_del(tr, tid);
            }
        } else {
            if !(status & 0xff as i32 == 0x7f as i32) {
                continue;
            }
            let mut sig: i32 = (status & 0xff00 as i32) >> 8 as i32;
            let mut event: u32 = status as u32 >> 16 as i32;
            let mut t: *mut eng_task = eng_task_find(tr, tid);
            if t.is_null() {
                t = task_new(tr, tid);
            }
            if (*t).linked == 0 {
                (*t).held = 1 as i32;
            } else {
                match event {
                    0 => {
                        if sig == SIGTRAP | 0x80 as i32 {
                            syscall_stop(tr, t, 0 as i32);
                        } else if sig == SIGSYS {
                            sigsys_stop(t);
                        } else {
                            if eng_log_enabled(eng_log_level::ENG_LOG_DEBUG) != 0
                                && (sig == SIGSEGV || sig == SIGBUS || sig == SIGILL)
                            {
                                let mut si: siginfo_t = platform_empty_siginfo_t();
                                let mut rr: eng_regs = platform_empty_eng_regs();
                                if ptrace(PTRACE_GETSIGINFO, tid, 0 as i32, &raw mut si) == 0 as i64
                                    && eng_regs_get(tid, &raw mut rr) == 0 as i32
                                {
                                    #[cfg(target_os = "linux")]
                                    eng_logf!(
                                        eng_log_level::ENG_LOG_DEBUG,
                                        b"tid=%d signal %d code=%d addr=%p pc=%#llx phase=%d\0"
                                            .as_ptr()
                                            as *const ::core::ffi::c_char,
                                        tid,
                                        sig,
                                        si.si_code,
                                        si._sifields._sigfault.si_addr,
                                        eng_pc(&raw mut rr) as u64,
                                        (*t).phase.0 as i32,
                                    );
                                    #[cfg(target_os = "android")]
                                    eng_logf!(
                                        eng_log_level::ENG_LOG_DEBUG,
                                        b"tid=%d signal %d code=%d addr=%p pc=%#llx phase=%d\0"
                                            .as_ptr()
                                            as *const ::core::ffi::c_char,
                                        tid,
                                        sig,
                                        si.c2rust_unnamed.c2rust_unnamed.si_code,
                                        si.c2rust_unnamed.c2rust_unnamed._sifields._sigfault._addr,
                                        eng_pc(&raw mut rr) as u64,
                                        (*t).phase.0 as i32,
                                    );
                                }
                            }
                            resume_task(t, sig);
                        }
                    }
                    1 | 2 | 3 => {
                        let mut ctid: u64 = 0 as u64;
                        if ptrace(PTRACE_GETEVENTMSG, tid, 0 as i32, &raw mut ctid) == 0 as i64
                            && ctid != 0
                        {
                            on_new_child(tr, t, ctid as i32, event as i32);
                        }
                        resume_task(t, 0 as i32);
                    }
                    4 => {
                        on_exec(tr, t);
                        resume_task(t, 0 as i32);
                    }
                    #[cfg(target_os = "linux")]
                    PTRACE_EVENT_SECCOMP => {
                        syscall_stop(tr, t, 1 as i32);
                    }
                    #[cfg(target_os = "android")]
                    7 => {
                        syscall_stop(tr, t, 1 as i32);
                    }
                    PTRACE_EVENT_STOP => {
                        if sig == SIGSTOP || sig == SIGTSTP || sig == SIGTTIN || sig == SIGTTOU {
                            if ptrace(PTRACE_LISTEN, tid, 0 as i32, 0 as i32) != 0 as i64
                                && *errno() != ESRCH
                            {
                                eng_logf!(
                                    eng_log_level::ENG_LOG_DEBUG,
                                    b"PTRACE_LISTEN tid=%d: %s\0".as_ptr()
                                        as *const ::core::ffi::c_char,
                                    tid,
                                    strerror(*errno()),
                                );
                            }
                        } else {
                            resume_task(t, 0 as i32);
                        }
                    }
                    _ => {
                        resume_task(t, 0 as i32);
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
    let mut rc_0: i32 = if (*tr).main_seen != 0 {
        (*tr).main_status
    } else {
        -1 as i32
    };
    if (*tr).check_toggle != 0 && (*tr).toggle_mismatch != 0 {
        rc_0 = 124 as i32;
    }
    free(tr as *mut ::core::ffi::c_void);
    return rc_0;
}
#[cfg(target_os = "linux")]
unsafe fn platform_empty_sigaction() -> sigaction {
    sigaction {
        __sigaction_handler: C2Rust_Unnamed_9 { sa_handler: None },
        sa_mask: __sigset_t { __val: [0; 16] },
        sa_flags: 0,
        sa_restorer: None,
    }
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
unsafe fn platform_empty_sigaction() -> sigaction {
    sigaction {
        sa_flags: 0,
        c2rust_unnamed: C2Rust_Unnamed_12 { sa_handler: None },
        sa_mask: 0,
        sa_restorer: None,
    }
}
#[cfg(target_arch = "aarch64")]
unsafe fn platform_empty_sigaction() -> sigaction {
    sigaction {
        sa_flags: 0,
        c2rust_unnamed: C2Rust_Unnamed_12 { sa_handler: None },
        sa_mask: sigset_t { sig: [0; 1] },
        sa_restorer: None,
    }
}
#[cfg(target_arch = "x86_64")]
unsafe fn platform_empty_eng_regs() -> eng_regs {
    eng_regs {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        rbp: 0,
        rbx: 0,
        r11: 0,
        r10: 0,
        r9: 0,
        r8: 0,
        rax: 0,
        rcx: 0,
        rdx: 0,
        rsi: 0,
        rdi: 0,
        orig_rax: 0,
        rip: 0,
        cs: 0,
        eflags: 0,
        rsp: 0,
        ss: 0,
        fs_base: 0,
        gs_base: 0,
        ds: 0,
        es: 0,
        fs: 0,
        gs: 0,
    }
}
#[cfg(target_arch = "aarch64")]
unsafe fn platform_empty_eng_regs() -> eng_regs {
    eng_regs {
        regs: [0; 31],
        sp: 0,
        pc: 0,
        pstate: 0,
    }
}
#[cfg(target_os = "linux")]
unsafe fn platform_empty_siginfo_t() -> siginfo_t {
    siginfo_t {
        si_signo: 0,
        si_errno: 0,
        si_code: 0,
        __pad0: 0,
        _sifields: C2Rust_Unnamed { _pad: [0; 28] },
    }
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
unsafe fn platform_empty_siginfo_t() -> siginfo_t {
    siginfo_t {
        c2rust_unnamed: C2Rust_Unnamed {
            c2rust_unnamed: C2Rust_Unnamed_0 {
                si_signo: 0,
                si_errno: 0,
                si_code: 0,
                _sifields: __sifields {
                    _kill: C2Rust_Unnamed_11 { _pid: 0, _uid: 0 },
                },
            },
        },
    }
}
#[cfg(target_arch = "aarch64")]
unsafe fn platform_empty_siginfo_t() -> siginfo_t {
    siginfo_t {
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
    }
}
#[cfg(target_os = "linux")]
fn platform_expr_wait_finished(status: i32) -> bool {
    status & 0x7f as i32 == 0 as i32
        || ((status & 0x7f as i32) + 1 as i32) as i8 as i32 >> 1 as i32 > 0 as i32
}
#[cfg(target_os = "android")]
fn platform_expr_wait_finished(status: i32) -> bool {
    status & 0x7f as i32 == 0 as i32 || status + 1 as i32 & 0x7f as i32 >= 2 as i32
}
#[cfg(target_os = "android")]
#[repr(C)]
pub struct __sFILE {
    _opaque: [u8; 0],
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct siginfo {
    pub c2rust_unnamed: C2Rust_Unnamed,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct siginfo {
    pub c2rust_unnamed: C2Rust_Unnamed_10,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub union __sifields {
    pub _kill: C2Rust_Unnamed_11,
    pub _timer: C2Rust_Unnamed_10,
    pub _rt: C2Rust_Unnamed_9,
    pub _sigchld: C2Rust_Unnamed_8,
    pub _sigfault: C2Rust_Unnamed_3,
    pub _sigpoll: C2Rust_Unnamed_2,
    pub _sigsys: C2Rust_Unnamed_1,
}
#[cfg(target_arch = "aarch64")]
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
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_3 {
    pub _addr: *mut ::core::ffi::c_void,
    pub c2rust_unnamed: C2Rust_Unnamed_4,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_3 {
    pub _data: u64,
    pub _type: u32,
    pub _flags: u32,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed_4 {
    pub _trapno: i32,
    pub _addr_lsb: i16,
    pub _addr_bnd: C2Rust_Unnamed_7,
    pub _addr_pkey: C2Rust_Unnamed_6,
    pub _perf: C2Rust_Unnamed_5,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_9 {
    pub _pid: i32,
    pub _uid: u32,
    pub _sigval: sigval_t,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_9 {
    pub _pid: i32,
    pub _uid: u32,
}
#[cfg(target_os = "android")]
pub type sigval_t = sigval;
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_10 {
    pub _tid: i32,
    pub _overrun: i32,
    pub _sigval: sigval_t,
    pub _sys_private: i32,
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_11 {
    pub _pid: i32,
    pub _uid: u32,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed_11 {
    pub si_signo: i32,
    pub si_errno: i32,
    pub si_code: i32,
    pub _sifields: __sifields,
}
#[cfg(target_os = "android")]
pub type __signalfn_t = unsafe extern "C" fn(i32) -> ();
#[cfg(target_os = "android")]
pub type siginfo_t = siginfo;
#[cfg(target_os = "android")]
pub type sighandler_t = __sighandler_t;
#[cfg(target_os = "android")]
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed_12 {
    pub sa_handler: sighandler_t,
    pub sa_sigaction:
        Option<unsafe extern "C" fn(i32, *mut siginfo, *mut ::core::ffi::c_void) -> ()>,
}
#[cfg(target_os = "android")]
pub const PTRACE_EVENT_FORK: i32 = 1 as i32;
#[cfg(target_os = "android")]
pub const PTRACE_EVENT_VFORK: i32 = 2 as i32;
#[cfg(target_os = "android")]
pub const PTRACE_EVENT_CLONE: i32 = 3 as i32;
#[cfg(target_os = "android")]
pub const PTRACE_EVENT_EXEC: i32 = 4 as i32;
#[cfg(target_os = "android")]
pub const PTRACE_EVENT_EXIT: i32 = 6 as i32;
#[cfg(target_os = "android")]
pub const PTRACE_O_TRACESYSGOOD: i32 = 1 as i32;
#[cfg(target_os = "android")]
pub const PTRACE_O_TRACEFORK: i32 = (1 as i32) << PTRACE_EVENT_FORK;
#[cfg(target_os = "android")]
pub const PTRACE_O_TRACEVFORK: i32 = (1 as i32) << PTRACE_EVENT_VFORK;
#[cfg(target_os = "android")]
pub const PTRACE_O_TRACECLONE: i32 = (1 as i32) << PTRACE_EVENT_CLONE;
#[cfg(target_os = "android")]
pub const PTRACE_O_TRACEEXEC: i32 = (1 as i32) << PTRACE_EVENT_EXEC;
#[cfg(target_os = "android")]
pub const PTRACE_O_TRACEEXIT: i32 = (1 as i32) << PTRACE_EVENT_EXIT;
#[cfg(target_os = "android")]
pub const PTRACE_O_EXITKILL: i32 = (1 as i32) << 20 as i32;
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct sigset_t {
    pub sig: [u64; 1],
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed {
    pub _call_addr: *mut ::core::ffi::c_void,
    pub _syscall: i32,
    pub _arch: u32,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed_2 {
    pub _trapno: i32,
    pub _addr_lsb: i16,
    pub _addr_bnd: C2Rust_Unnamed_5,
    pub _addr_pkey: C2Rust_Unnamed_4,
    pub _perf: C2Rust_Unnamed_3,
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2Rust_Unnamed_10 {
    pub c2rust_unnamed: C2Rust_Unnamed_11,
    pub _si_pad: [i32; 32],
}
