//! Syscall dispatch, path rewriting, virtual metadata and Android SIGSYS handling.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    #[cfg(target_os = "linux")]
    #[link_name = "__errno_location"]
    unsafe fn errno() -> *mut i32;
    #[cfg(target_os = "android")]
    #[link_name = "__errno"]
    unsafe fn errno() -> *mut i32;
    unsafe fn open(_: *const ::core::ffi::c_char, _: i32, ...) -> i32;
    #[cfg(target_os = "linux")]
    unsafe fn rename(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    #[cfg(target_os = "android")]
    unsafe fn rename(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn snprintf(
        _: *mut ::core::ffi::c_char,
        _: usize,
        _: *const ::core::ffi::c_char,
        ...
    ) -> i32;
    unsafe fn malloc(_: usize) -> *mut ::core::ffi::c_void;
    unsafe fn free(_: *mut ::core::ffi::c_void);
    #[cfg(target_arch = "x86_64")]
    unsafe fn getenv(_: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: usize,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memmove(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: usize,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memset(_: *mut ::core::ffi::c_void, _: i32, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn memcmp(_: *const ::core::ffi::c_void, _: *const ::core::ffi::c_void, _: usize)
    -> i32;
    unsafe fn strcmp(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn strncmp(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn strrchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(_: *const ::core::ffi::c_char) -> usize;
    unsafe fn strnlen(_: *const ::core::ffi::c_char, _: usize) -> usize;
    unsafe fn strerror(_: i32) -> *mut ::core::ffi::c_char;
    unsafe fn stat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
    unsafe fn lstat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
    unsafe fn chmod(_: *const ::core::ffi::c_char, _: u32) -> i32;
    unsafe fn mkdir(_: *const ::core::ffi::c_char, _: u32) -> i32;
    #[cfg(target_arch = "x86_64")]
    unsafe fn time(_: *mut i64) -> i64;
    #[cfg(target_os = "linux")]
    unsafe fn gnu_dev_major(_: u64) -> u32;
    #[cfg(target_os = "linux")]
    unsafe fn gnu_dev_minor(_: u64) -> u32;
    #[cfg(target_os = "linux")]
    unsafe fn gnu_dev_makedev(_: u32, _: u32) -> u64;
    unsafe fn close(_: i32) -> i32;
    unsafe fn getpid() -> i32;
    #[cfg(target_os = "linux")]
    unsafe fn symlink(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    #[cfg(target_os = "android")]
    unsafe fn symlink(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn readlink(
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> isize;
    unsafe fn unlink(_: *const ::core::ffi::c_char) -> i32;
    unsafe fn eng_arg(_: *const eng_regs, _: i32) -> u64;
    unsafe fn eng_set_arg(_: *mut eng_regs, _: i32, _: u64);
    unsafe fn eng_ret(_: *const eng_regs) -> u64;
    unsafe fn eng_set_ret(_: *mut eng_regs, _: u64);
    #[cfg(target_arch = "x86_64")]
    unsafe fn eng_pc(_: *const eng_regs) -> u64;
    #[cfg(target_arch = "x86_64")]
    unsafe fn eng_set_pc(_: *mut eng_regs, _: u64);
    unsafe fn eng_syscall_set(_: i32, _: *mut eng_regs, _: i64) -> i32;
    #[cfg(target_arch = "x86_64")]
    unsafe fn eng_syscall_insn_len() -> i32;
    unsafe fn eng_tracer_guest(_: *mut eng_tracer) -> *mut eng_guest;
    unsafe fn eng_task_void(_: *mut eng_task, _: *mut eng_regs, _: i64) -> i32;
    unsafe fn eng_scratch_alloc(_: *mut eng_task, _: *const eng_regs, _: usize) -> u64;
    unsafe fn eng_scratch_put_str(
        _: *mut eng_task,
        _: *const eng_regs,
        _: *const ::core::ffi::c_char,
    ) -> u64;
    unsafe fn eng_exec_prepare(
        _: *mut eng_task,
        _: i32,
        _: *const ::core::ffi::c_char,
        _: i32,
    ) -> i32;
    unsafe fn eng_exec_discard(_: *mut eng_task);
    unsafe fn eng_guest_default_owner(
        _: *const eng_guest,
        _: *const ::core::ffi::c_char,
        _: *mut u32,
        _: *mut u32,
    );
    unsafe fn eng_guest_to_host(
        _: *const eng_guest,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn eng_host_to_guest(
        _: *const eng_guest,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn eng_guest_is_hidden(_: *const eng_guest, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn eng_ident_entry(_: *mut eng_task, _: *mut eng_regs) -> i32;
    unsafe fn eng_ident_exit(_: *mut eng_task, _: *mut eng_regs) -> i32;
    unsafe fn eng_log_enabled(_: eng_log_level) -> i32;
    unsafe fn eng_mem_read(_: i32, _: usize, _: *mut ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn eng_mem_write(_: i32, _: usize, _: *const ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn eng_mem_read_cstr(_: i32, _: usize, _: *mut ::core::ffi::c_char, _: usize) -> isize;
    unsafe fn eng_meta_read(
        _: *mut eng_guest,
        _: *const ::core::ffi::c_char,
        _: i32,
        _: *const stat,
        _: *mut eng_meta,
    ) -> i32;
    unsafe fn eng_meta_write(_: *const ::core::ffi::c_char, _: i32, _: *const eng_meta) -> i32;
    unsafe fn eng_meta_get(
        _: *mut eng_guest,
        _: *const ::core::ffi::c_char,
        _: i32,
        _: *const stat,
        _: *mut eng_meta,
    ) -> i32;
    unsafe fn eng_meta_in_store(_: *mut eng_guest, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn eng_meta_lock(_: *mut eng_guest) -> i32;
    unsafe fn eng_meta_unlock(_: i32);
    unsafe fn eng_meta_is_placeholder(_: *const eng_meta, _: u32) -> i32;
    unsafe fn eng_meta_fifo_path(
        _: *mut eng_guest,
        _: *const stat,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn eng_meta_apply_stat(_: *const eng_meta, _: *mut stat);
    unsafe fn eng_meta_apply_statx(_: *const eng_meta, _: *mut ::core::ffi::c_void);
    unsafe fn eng_meta_permission(_: *mut eng_task, _: *const eng_meta, _: i32, _: i32) -> i32;
    unsafe fn eng_in_group(_: *const eng_task, _: u32, _: i32) -> i32;
    unsafe fn eng_link_is_stub_text(
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn eng_link_object_path(
        _: *mut eng_guest,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn eng_link_create(
        _: *mut eng_guest,
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
    ) -> i32;
    unsafe fn eng_link_drop(_: *mut eng_guest, _: *const ::core::ffi::c_char);
    unsafe fn eng_resolve(
        _: *mut eng_task,
        _: i32,
        _: *const ::core::ffi::c_char,
        _: i32,
        _: *mut eng_resolved,
    ) -> i32;
    unsafe fn eng_task_cwd(_: *mut eng_task, _: *mut ::core::ffi::c_char, _: usize) -> i32;
    unsafe fn eng_sysinv_name(_: i64) -> *const ::core::ffi::c_char;
    unsafe fn eng_sysinv_class(_: i64) -> eng_sc_class;
    unsafe fn eng_sysinv_class_name(_: eng_sc_class) -> *const ::core::ffi::c_char;
}
#[repr(C)]
pub struct eng_tracer {
    _opaque: [u8; 0],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: i32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [i64; 3],
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __pad3: [i64; 3],
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub __pad1: u64,
    pub st_size: i64,
    pub st_blksize: i32,
    pub __pad2: i32,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __unused4: u32,
    pub __unused5: u32,
}
#[cfg(target_os = "linux")]
pub type __sigval_t = sigval;
#[derive(Copy, Clone)]
#[repr(C)]
pub union sigval {
    pub sival_int: i32,
    pub sival_ptr: *mut ::core::ffi::c_void,
}
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_resolved {
    pub guest: [::core::ffi::c_char; 4096],
    pub host: [::core::ffi::c_char; 4096],
    pub verbatim: i32,
    pub magic: i32,
    pub magic_text: [::core::ffi::c_char; 4096],
    pub exists: i32,
    pub stub: i32,
    pub stub_id: [::core::ffi::c_char; 64],
    pub entry: [::core::ffi::c_char; 4096],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct wf_msghdr {
    pub name: u64,
    pub namelen: u32,
    pub pad: u32,
    pub iov: u64,
    pub iovlen: u64,
    pub control: u64,
    pub controllen: u64,
    pub flags: i32,
    pub pad2: i32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct wf_sockaddr_un {
    pub family: u16,
    pub path: [::core::ffi::c_char; 108],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_meta {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub nlink: u32,
    pub major: u32,
    pub minor: u32,
    pub present: i32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pathsys {
    pub nr: i64,
    pub dfd: i8,
    pub path: i8,
    pub flagarg: i8,
    pub mode: u8,
    pub nullok: u8,
    pub dironly: u8,
    pub entry: u8,
}
#[cfg(target_arch = "x86_64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct lg_timespec {
    pub sec: i64,
    pub nsec: i64,
}
#[cfg(target_arch = "x86_64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct lg_timeval {
    pub sec: i64,
    pub usec: i64,
}
#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct dirent64_hdr {
    pub ino: u64,
    pub off: i64,
    pub reclen: u16,
    pub r#type: u8,
}
pub const FL_FOLLOW: u32 = 0;
pub const FL_NOFOLLOW: u32 = 1;
pub const FL_ATFLAG: u32 = 2;
pub const FL_INOTIFY: u32 = 3;
pub const XA_GET: u32 = 0;
pub const XA_SET: u32 = 1;
pub const XA_LIST: u32 = 2;
pub const XA_REMOVE: u32 = 3;
#[cfg(target_arch = "x86_64")]
pub const LG_NONE: u32 = 0;
#[cfg(target_arch = "x86_64")]
pub const LG_DUP2_SAME: u32 = 1;
#[cfg(target_arch = "x86_64")]
pub const LG_SELECT: u32 = 2;
#[cfg(target_arch = "x86_64")]
pub const LG_GETDENTS: u32 = 3;
#[cfg(target_arch = "x86_64")]
pub const LG_ALARM: u32 = 4;
pub const EPERM: i32 = 1 as i32;
pub const ENOENT: i32 = 2 as i32;
pub const ENXIO: i32 = 6 as i32;
pub const ENOMEM: i32 = 12 as i32;
pub const EFAULT: i32 = 14 as i32;
pub const EEXIST: i32 = 17 as i32;
pub const EXDEV: i32 = 18 as i32;
pub const ENODEV: i32 = 19 as i32;
pub const EINVAL: i32 = 22 as i32;
pub const ERANGE: i32 = 34 as i32;
pub const ENAMETOOLONG: i32 = 36 as i32;
pub const ENOSYS: i32 = 38 as i32;
pub const ENODATA: i32 = 61 as i32;
pub const EPROTONOSUPPORT: i32 = 93 as i32;
pub const EOPNOTSUPP: i32 = 95 as i32;
pub const EADDRINUSE: i32 = 98 as i32;
pub const ENOTSUP: i32 = EOPNOTSUPP;
pub const O_ACCMODE: i32 = 0o3 as i32;
pub const O_RDONLY: i32 = 0 as i32;
pub const O_WRONLY: i32 = 0o1 as i32;
pub const O_RDWR: i32 = 0o2 as i32;
pub const O_CREAT: i32 = 0o100 as i32;
pub const O_EXCL: i32 = 0o200 as i32;
pub const O_TRUNC: i32 = 0o1000 as i32;
#[cfg(target_os = "linux")]
pub const __O_DIRECTORY: i32 = 0o200000 as i32;
#[cfg(target_os = "linux")]
pub const __O_NOFOLLOW: i32 = 0o400000 as i32;
#[cfg(target_os = "linux")]
pub const __O_CLOEXEC: i32 = 0o2000000 as i32;
#[cfg(target_os = "linux")]
pub const __O_PATH: i32 = 0o10000000 as i32;
#[cfg(target_os = "linux")]
pub const __O_TMPFILE: i32 = 0o20000000 as i32 | __O_DIRECTORY;
#[cfg(target_os = "android")]
pub const __O_TMPFILE: i32 = 0o20000000 as i32;
#[cfg(target_os = "linux")]
pub const O_NOFOLLOW: i32 = __O_NOFOLLOW;
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
pub const O_NOFOLLOW: i32 = 0o400000 as i32;
#[cfg(target_arch = "aarch64")]
pub const O_NOFOLLOW: i32 = 0o100000 as i32;
#[cfg(target_os = "linux")]
pub const O_CLOEXEC: i32 = __O_CLOEXEC;
#[cfg(target_os = "android")]
pub const O_CLOEXEC: i32 = 0o2000000 as i32;
#[cfg(target_os = "linux")]
pub const O_PATH: i32 = __O_PATH;
#[cfg(target_os = "android")]
pub const O_PATH: i32 = 0o10000000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFMT: i32 = 0o170000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFDIR: i32 = 0o40000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFCHR: i32 = 0o20000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFBLK: i32 = 0o60000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFREG: i32 = 0o100000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFIFO: i32 = 0o10000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFSOCK: i32 = 0o140000 as i32;
#[cfg(target_os = "linux")]
pub const __S_ISUID: i32 = 0o4000 as i32;
#[cfg(target_os = "linux")]
pub const __S_ISGID: i32 = 0o2000 as i32;
#[cfg(target_os = "linux")]
pub const __S_ISVTX: i32 = 0o1000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IEXEC: i32 = 0o100 as i32;
pub const AT_FDCWD: i32 = -100 as i32;
pub const AT_SYMLINK_NOFOLLOW: i32 = 0x100 as i32;
pub const AT_REMOVEDIR: i32 = 0x200 as i32;
pub const AT_SYMLINK_FOLLOW: i32 = 0x400 as i32;
pub const AT_EMPTY_PATH: i32 = 0x1000 as i32;
pub const AT_EACCESS: i32 = 0x200 as i32;
pub const PATH_MAX: i32 = 4096 as i32;
pub const RENAME_EXCHANGE: i32 = (1 as i32) << 1 as i32;
pub const IN_DONT_FOLLOW: i32 = 0x2000000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFMT: i32 = __S_IFMT;
#[cfg(target_os = "android")]
pub const S_IFMT: i32 = 0o170000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFDIR: i32 = __S_IFDIR;
#[cfg(target_os = "android")]
pub const S_IFDIR: i32 = 0o40000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFCHR: i32 = __S_IFCHR;
#[cfg(target_os = "android")]
pub const S_IFCHR: i32 = 0o20000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFBLK: i32 = __S_IFBLK;
#[cfg(target_os = "android")]
pub const S_IFBLK: i32 = 0o60000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFREG: i32 = __S_IFREG;
#[cfg(target_os = "android")]
pub const S_IFREG: i32 = 0o100000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFIFO: i32 = __S_IFIFO;
#[cfg(target_os = "android")]
pub const S_IFIFO: i32 = 0o10000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFSOCK: i32 = __S_IFSOCK;
#[cfg(target_os = "android")]
pub const S_IFSOCK: i32 = 0o140000 as i32;
#[cfg(target_os = "linux")]
pub const S_ISUID: i32 = __S_ISUID;
#[cfg(target_os = "android")]
pub const S_ISUID: i32 = 0o4000 as i32;
#[cfg(target_os = "linux")]
pub const S_ISGID: i32 = __S_ISGID;
#[cfg(target_os = "android")]
pub const S_ISGID: i32 = 0o2000 as i32;
#[cfg(target_os = "linux")]
pub const S_ISVTX: i32 = __S_ISVTX;
#[cfg(target_os = "android")]
pub const S_ISVTX: i32 = 0o1000 as i32;
#[cfg(target_os = "linux")]
pub const S_IXUSR: i32 = __S_IEXEC;
#[cfg(target_os = "linux")]
pub const S_IXGRP: i32 = S_IXUSR >> 3 as i32;
#[cfg(target_os = "android")]
pub const S_IXGRP: i32 = 0o10 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_open: i32 = 2 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_stat: i32 = 4 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_fstat: i64 = 5 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_fstat: i64 = 80 as i64;
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
pub const __NR_setitimer: i32 = 38 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_socket: i64 = 41 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_socket: i64 = 198 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_connect: i64 = 42 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_connect: i64 = 203 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_accept: i64 = 43 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_accept: i64 = 202 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_sendto: i64 = 44 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_sendto: i64 = 206 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_sendmsg: i64 = 46 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_sendmsg: i64 = 211 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_bind: i64 = 49 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_bind: i64 = 200 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_getsockname: i64 = 51 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_getsockname: i64 = 204 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_getpeername: i64 = 52 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_getpeername: i64 = 205 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_execve: i32 = 59 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_execve: i32 = 221 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_fcntl: i32 = 72 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_truncate: i32 = 76 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_truncate: i32 = 45 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_getdents: i32 = 78 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_getcwd: i64 = 79 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_getcwd: i64 = 17 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_chdir: i32 = 80 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_chdir: i32 = 49 as i32;
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
pub const __NR_fchmod: i64 = 91 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_fchmod: i64 = 52 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_chown: i32 = 92 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_fchown: i64 = 93 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_fchown: i64 = 55 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_lchown: i32 = 94 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_ptrace: i64 = 101 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_ptrace: i64 = 117 as i64;
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
pub const __NR_getpgid: i32 = 121 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setfsuid: i32 = 122 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_setfsgid: i32 = 123 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_utime: i32 = 132 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_mknod: i32 = 133 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_uselib: i32 = 134 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_statfs: i32 = 137 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_statfs: i32 = 43 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_pivot_root: i64 = 155 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_pivot_root: i64 = 41 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_adjtimex: i32 = 159 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_chroot: i32 = 161 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_chroot: i64 = 51 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_acct: i32 = 163 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_acct: i64 = 89 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_mount: i32 = 165 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_mount: i64 = 40 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_umount2: i32 = 166 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_umount2: i64 = 39 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_swapon: i32 = 167 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_swapon: i64 = 224 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_swapoff: i64 = 168 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_swapoff: i64 = 225 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_setxattr: i64 = 188 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_setxattr: i64 = 5 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_lsetxattr: i64 = 189 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_lsetxattr: i64 = 6 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_fsetxattr: i64 = 190 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_fsetxattr: i64 = 7 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_getxattr: i64 = 191 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_getxattr: i64 = 8 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_lgetxattr: i64 = 192 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_lgetxattr: i64 = 9 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_fgetxattr: i64 = 193 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_fgetxattr: i64 = 10 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_listxattr: i64 = 194 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_listxattr: i64 = 11 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_llistxattr: i64 = 195 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_llistxattr: i64 = 12 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_flistxattr: i64 = 196 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_flistxattr: i64 = 13 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_removexattr: i64 = 197 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_removexattr: i64 = 14 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_lremovexattr: i64 = 198 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_lremovexattr: i64 = 15 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_fremovexattr: i64 = 199 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_fremovexattr: i64 = 16 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_time: i32 = 201 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_epoll_create: i32 = 213 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_getdents64: i32 = 217 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_getdents64: i64 = 61 as i64;
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
pub const __NR_inotify_add_watch: i32 = 254 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_inotify_add_watch: i32 = 27 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_migrate_pages: i32 = 256 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_openat: i32 = 257 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_openat: i64 = 56 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_mkdirat: i32 = 258 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_mkdirat: i64 = 34 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_mknodat: i32 = 259 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_mknodat: i64 = 33 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_fchownat: i32 = 260 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_fchownat: i64 = 54 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_futimesat: i32 = 261 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_newfstatat: i32 = 262 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_newfstatat: i64 = 79 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_unlinkat: i32 = 263 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_unlinkat: i64 = 35 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_renameat: i32 = 264 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_renameat: i64 = 38 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_linkat: i32 = 265 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_linkat: i64 = 37 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_symlinkat: i32 = 266 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_symlinkat: i64 = 36 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_readlinkat: i32 = 267 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_readlinkat: i64 = 78 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_fchmodat: i32 = 268 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_fchmodat: i64 = 53 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_faccessat: i32 = 269 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_faccessat: i64 = 48 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_pselect6: i32 = 270 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_ppoll: i32 = 271 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_set_robust_list: i32 = 273 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_move_pages: i32 = 279 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_utimensat: i32 = 280 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_utimensat: i32 = 88 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_epoll_pwait: i32 = 281 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_signalfd: i32 = 282 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_eventfd: i32 = 284 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_accept4: i64 = 288 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_accept4: i64 = 242 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_signalfd4: i32 = 289 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_eventfd2: i32 = 290 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_epoll_create1: i32 = 291 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_dup3: i32 = 292 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_pipe2: i32 = 293 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_inotify_init1: i32 = 294 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_fanotify_init: i32 = 300 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_fanotify_init: i64 = 262 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_fanotify_mark: i64 = 301 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_fanotify_mark: i64 = 263 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_name_to_handle_at: i32 = 303 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_name_to_handle_at: i64 = 264 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_open_by_handle_at: i32 = 304 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_open_by_handle_at: i64 = 265 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_clock_adjtime: i32 = 305 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_kcmp: i32 = 312 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_renameat2: i64 = 316 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_renameat2: i64 = 276 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_kexec_file_load: i32 = 320 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_bpf: i32 = 321 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_execveat: i64 = 322 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_execveat: i64 = 281 as i64;
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
#[cfg(target_arch = "aarch64")]
pub const __NR_statx: i64 = 291 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_io_pgetevents: i32 = 333 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_rseq: i32 = 334 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_pidfd_send_signal: i32 = 424 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_io_uring_setup: i32 = 425 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_io_uring_setup: i64 = 425 as i64;
pub const __NR_io_uring_enter: i64 = 426 as i64;
pub const __NR_io_uring_register: i64 = 427 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_open_tree: i32 = 428 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_open_tree: i64 = 428 as i64;
pub const __NR_move_mount: i64 = 429 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_fsopen: i32 = 430 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_fsopen: i64 = 430 as i64;
pub const __NR_fsconfig: i64 = 431 as i64;
pub const __NR_fsmount: i64 = 432 as i64;
pub const __NR_fspick: i64 = 433 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_pidfd_open: i32 = 434 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_clone3: i32 = 435 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_clone3: i64 = 435 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_close_range: i32 = 436 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_openat2: i32 = 437 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_openat2: i64 = 437 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_pidfd_getfd: i32 = 438 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_faccessat2: i32 = 439 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_faccessat2: i64 = 439 as i64;
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
#[cfg(target_arch = "aarch64")]
pub const __NR_fchmodat2: i64 = 452 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_map_shadow_stack: i32 = 453 as i32;
pub const R_OK: i32 = 4 as i32;
pub const W_OK: i32 = 2 as i32;
pub const X_OK: i32 = 1 as i32;
pub const F_OK: i32 = 0 as i32;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ENG_CAP_CHOWN: i32 = 0 as i32;
pub const ENG_CAP_FOWNER: i32 = 3 as i32;
pub const ENG_CAP_FSETID: i32 = 4 as i32;
pub const ENG_CAP_MKNOD: i32 = 27 as i32;
pub const S_IFLNK: i32 = 0o120000 as i32;
#[inline]
unsafe extern "C" fn eng_capable(mut t: *const eng_task, mut cap: i32) -> i32 {
    return ((*t).cr.cap_eff >> cap & 1 as u64) as i32;
}
pub const ENG_META_PREFIX: [::core::ffi::c_char; 15] =
    unsafe { ::core::mem::transmute::<[u8; 15], [::core::ffi::c_char; 15]>(*b"user.workflow.\0") };
pub const ENG_RES_FOLLOW: i32 = 0x1 as i32;
pub const ENG_RES_MISSING_OK: i32 = 0x2 as i32;
pub const ENG_RES_DIR_ONLY: i32 = 0x4 as i32;
unsafe extern "C" fn read_path(
    mut t: *mut eng_task,
    mut addr: u64,
    mut buf: *mut ::core::ffi::c_char,
) -> i64 {
    if addr == 0 {
        return -(EFAULT as i64);
    }
    let mut n: isize = eng_mem_read_cstr((*t).tid, addr as usize, buf, PATH_MAX as usize);
    if n < 0 as isize {
        return -(EFAULT as i64);
    }
    if n >= (PATH_MAX - 1 as i32) as isize {
        return -(ENAMETOOLONG as i64);
    }
    return n as i64;
}
unsafe extern "C" fn put_path(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut arg: i32,
    mut host: *const ::core::ffi::c_char,
) -> i32 {
    let mut a: u64 = eng_scratch_put_str(t, r, host);
    if a == 0 {
        return -ENOMEM;
    }
    eng_set_arg(r, arg, a);
    (*t).regs_modified = 1 as i32;
    return 0 as i32;
}
unsafe extern "C" fn resolve_arg(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut flags: i32,
    mut allow_empty: i32,
    mut res: *mut eng_resolved,
    mut empty: *mut i32,
) -> i32 {
    let mut path: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut n: i64 = read_path(
        t,
        eng_arg(r, path_arg),
        &raw mut path as *mut ::core::ffi::c_char,
    );
    if !empty.is_null() {
        *empty = 0 as i32;
    }
    if n < 0 as i64 {
        return n as i32;
    }
    if n == 0 as i64 && allow_empty != 0 {
        if !empty.is_null() {
            *empty = 1 as i32;
        }
        return 0 as i32;
    }
    let mut dfd: i32 = if dfd_arg >= 0 as i32 {
        eng_arg(r, dfd_arg) as i32
    } else {
        AT_FDCWD
    };
    let mut rc: i32 = eng_resolve(
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
    mut fd: i32,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
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
    mut nofollow: i32,
    mut st: *mut stat,
) -> i32 {
    return if if nofollow != 0 {
        lstat(host, st)
    } else {
        stat(host, st)
    } == 0 as i32
    {
        0 as i32
    } else {
        -*errno()
    };
}
unsafe extern "C" fn parent_dir(
    mut host: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    snprintf(
        out,
        cap,
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        host,
    );
    let mut n: usize = strlen(out);
    while n > 1 as usize && *out.offset(n.wrapping_sub(1 as usize) as isize) as i32 == '/' as i32 {
        n = n.wrapping_sub(1);
        *out.offset(n as isize) = 0 as ::core::ffi::c_char;
    }
    let mut sl: *mut ::core::ffi::c_char = strrchr(out, '/' as i32);
    if sl.is_null() {
        return -ENOENT;
    }
    if sl == out {
        *sl.offset(1isize) = 0 as ::core::ffi::c_char;
    } else {
        *sl = 0 as ::core::ffi::c_char;
    }
    return 0 as i32;
}
unsafe extern "C" fn may_create(
    mut t: *mut eng_task,
    mut entry_host: *const ::core::ffi::c_char,
    mut pm: *mut eng_meta,
) -> i32 {
    let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
    if parent_dir(
        entry_host,
        &raw mut dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return -ENOENT;
    }
    let mut st: stat = platform_empty_stat();
    if stat(&raw mut dir as *mut ::core::ffi::c_char, &raw mut st) != 0 as i32 {
        return -*errno();
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
        0 as i32,
        &raw mut st,
        &raw mut m,
    );
    if !pm.is_null() {
        *pm = m;
    }
    return eng_meta_permission(t, &raw mut m, W_OK | X_OK, 0 as i32);
}
unsafe extern "C" fn may_delete(
    mut t: *mut eng_task,
    mut entry_host: *const ::core::ffi::c_char,
) -> i32 {
    let mut dm: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    let mut rc: i32 = may_create(t, entry_host, &raw mut dm);
    if rc != 0 {
        return rc;
    }
    if dm.mode & S_ISVTX as u32 != 0 && eng_capable(t, ENG_CAP_FOWNER) == 0 {
        let mut st: stat = platform_empty_stat();
        if lstat(entry_host, &raw mut st) != 0 as i32 {
            return 0 as i32;
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
            1 as i32,
            &raw mut st,
            &raw mut vm,
        );
        if (*t).cr.fsuid != vm.uid && (*t).cr.fsuid != dm.uid {
            return -EPERM;
        }
    }
    return 0 as i32;
}
unsafe extern "C" fn in_rootfs(mut t: *mut eng_task, mut host: *const ::core::ffi::c_char) -> i32 {
    let mut g: *mut eng_guest = eng_tracer_guest((*t).tr);
    return ((*g).rootlen == 0 as usize
        || strncmp(
            host,
            &raw mut (*g).root as *mut ::core::ffi::c_char,
            (*g).rootlen,
        ) == 0 as i32
            && (*host.offset((*g).rootlen as isize) as i32 == '/' as i32
                || *host.offset((*g).rootlen as isize) as i32 == 0 as i32)) as i32;
}
unsafe extern "C" fn new_gid(mut t: *mut eng_task, mut parent: *const eng_meta) -> u32 {
    return if (*parent).mode & S_ISGID as u32 != 0 {
        (*parent).gid
    } else {
        (*t).cr.fsgid
    };
}
unsafe extern "C" fn virtual_device(mut m: *const eng_meta) -> *const ::core::ffi::c_char {
    if (*m).mode & S_IFMT as u32 != S_IFCHR as u32 {
        return ::core::ptr::null::<::core::ffi::c_char>();
    }
    if (*m).major == 1 as u32 {
        match (*m).minor {
            3 => return b"/dev/null\0".as_ptr() as *const ::core::ffi::c_char,
            5 => return b"/dev/zero\0".as_ptr() as *const ::core::ffi::c_char,
            7 => return b"/dev/full\0".as_ptr() as *const ::core::ffi::c_char,
            8 => return b"/dev/random\0".as_ptr() as *const ::core::ffi::c_char,
            9 => return b"/dev/urandom\0".as_ptr() as *const ::core::ffi::c_char,
            _ => {}
        }
    }
    if (*m).major == 5 as u32 && (*m).minor == 0 as u32 {
        return b"/dev/tty\0".as_ptr() as *const ::core::ffi::c_char;
    }
    if (*m).major == 5 as u32 && (*m).minor == 2 as u32 {
        return b"/dev/ptmx\0".as_ptr() as *const ::core::ffi::c_char;
    }
    return ::core::ptr::null::<::core::ffi::c_char>();
}
unsafe extern "C" fn h_open(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut flags_arg: i32,
    mut mode_arg: i32,
) -> i32 {
    let mut flags: i64 = if flags_arg >= 0 as i32 {
        eng_arg(r, flags_arg) as i64
    } else {
        (O_CREAT | O_WRONLY | O_TRUNC) as i64
    };
    let mut tmpfile: i32 = (flags & __O_TMPFILE as i64 == __O_TMPFILE as i64) as i32;
    let mut creat: i32 = (flags & O_CREAT as i64 != 0 && tmpfile == 0) as i32;
    let mut follow: i32 =
        !(flags & O_NOFOLLOW as i64 != 0 || creat != 0 && flags & O_EXCL as i64 != 0) as i32;
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
    let mut rc: i32 = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        if follow != 0 {
            ENG_RES_FOLLOW
        } else {
            0 as i32
        } | if creat != 0 {
            ENG_RES_MISSING_OK
        } else {
            0 as i32
        },
        0 as i32,
        &raw mut res,
        ::core::ptr::null_mut::<i32>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
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
                return eng_task_void(t, r, rc as i64);
            }
            let mut mode: u32 = eng_arg(r, mode_arg) as i64 as u32 & 0o7777 as u32;
            (*t).fix_mode = S_IFREG as u32 | mode & !(*t).cr.umask & 0o7777 as u32;
            (*t).fix_aux = new_gid(t, &raw mut pm) as i64;
            if (*t).fix_mode & S_ISGID as u32 != 0
                && eng_in_group(t, (*t).fix_aux as u32, 0 as i32) == 0
                && eng_capable(t, ENG_CAP_FSETID) == 0
            {
                (*t).fix_mode &= !(S_ISGID as u32);
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
                eng_set_arg(r, mode_arg, ((mode | 0o600 as u32) & 0o777 as u32) as u64);
                (*t).fixup = ENG_FIX_CREATE_FD as i32;
            } else {
                eng_set_arg(r, mode_arg, (mode & 0o777 as u32) as u64);
            }
        } else {
            let mut st: stat = platform_empty_stat();
            if stat(&raw mut res.host as *mut ::core::ffi::c_char, &raw mut st) == 0 as i32 {
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
                    0 as i32,
                    &raw mut st,
                    &raw mut m,
                );
                let mut mask: i32 = 0 as i32;
                if flags & O_PATH as i64 == 0 {
                    let mut acc: i32 = (flags & O_ACCMODE as i64) as i32;
                    if acc == O_RDONLY || acc == O_RDWR {
                        mask |= R_OK;
                    }
                    if acc == O_WRONLY || acc == O_RDWR || flags & O_TRUNC as i64 != 0 {
                        mask |= W_OK;
                    }
                }
                if st.st_mode & S_IFMT as u32 == S_IFDIR as u32 && mask & W_OK != 0 {
                    mask &= !W_OK;
                }
                if mask != 0 && {
                    rc = eng_meta_permission(t, &raw mut m, mask, 0 as i32);
                    rc != 0
                } {
                    return eng_task_void(t, r, rc as i64);
                }
                let mut dev: *const ::core::ffi::c_char = if flags & O_PATH as i64 != 0 {
                    ::core::ptr::null::<::core::ffi::c_char>()
                } else {
                    virtual_device(&raw mut m)
                };
                if !dev.is_null() && st.st_mode & S_IFMT as u32 == S_IFREG as u32 {
                    host = dev;
                }
                if flags & O_PATH as i64 == 0 {
                    if st.st_mode & S_IFMT as u32 == S_IFREG as u32
                        && m.mode & S_IFMT as u32 == S_IFIFO as u32
                    {
                        rc = eng_meta_fifo_path(
                            eng_tracer_guest((*t).tr),
                            &raw mut st,
                            &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                        );
                        if rc != 0 {
                            return eng_task_void(t, r, rc as i64);
                        }
                        host = &raw mut (*t).fix_path as *mut ::core::ffi::c_char;
                    } else if eng_meta_is_placeholder(&raw mut m, st.st_mode) != 0 && dev.is_null()
                    {
                        return eng_task_void(
                            t,
                            r,
                            (if m.mode & S_IFMT as u32 == S_IFSOCK as u32 {
                                -ENXIO
                            } else {
                                -ENODEV
                            }) as i64,
                        );
                    }
                }
            }
        }
    }
    rc = put_path(t, r, path_arg, host);
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    return 1 as i32;
}
unsafe extern "C" fn h_stat(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut flag_arg: i32,
    mut buf_arg: i32,
    mut nofollow_always: i32,
    mut statx: i32,
) -> i32 {
    let mut flags: i64 = if flag_arg >= 0 as i32 {
        eng_arg(r, flag_arg) as i64
    } else {
        0 as i64
    };
    let mut nofollow: i32 =
        (nofollow_always != 0 || flags & AT_SYMLINK_NOFOLLOW as i64 != 0) as i32;
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
    let mut empty: i32 = 0 as i32;
    let mut rc: i32 = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        if nofollow != 0 {
            0 as i32
        } else {
            ENG_RES_FOLLOW
        },
        (flags & AT_EMPTY_PATH as i64 != 0 as i64) as i32,
        &raw mut res,
        &raw mut empty,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    let mut changed: i32 = 0 as i32;
    if empty != 0 {
        fd_proc_path(
            t,
            eng_arg(r, dfd_arg) as i64 as i32,
            &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        );
        (*t).fix_nofollow = 0 as i32;
    } else {
        rc = put_path(
            t,
            r,
            path_arg,
            &raw mut res.host as *mut ::core::ffi::c_char,
        );
        if rc != 0 {
            return eng_task_void(t, r, rc as i64);
        }
        changed = 1 as i32;
        if res.magic != 0 || res.verbatim != 0 {
            return changed;
        }
        snprintf(
            &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut res.host as *mut ::core::ffi::c_char,
        );
        (*t).fix_nofollow = (nofollow != 0 && res.stub == 0) as i32;
    }
    (*t).fix_addr = eng_arg(r, buf_arg);
    (*t).fixup = if statx != 0 {
        ENG_FIX_STATX as i32
    } else {
        ENG_FIX_STAT as i32
    };
    return changed;
}
unsafe extern "C" fn h_fstat(mut t: *mut eng_task, mut r: *mut eng_regs) -> i32 {
    fd_proc_path(
        t,
        eng_arg(r, 0 as i32) as i64 as i32,
        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    (*t).fix_nofollow = 0 as i32;
    (*t).fix_addr = eng_arg(r, 1 as i32);
    (*t).fixup = ENG_FIX_STAT as i32;
    return 0 as i32;
}
unsafe extern "C" fn x_stat(mut t: *mut eng_task) {
    let mut st: stat = platform_empty_stat();
    if eng_mem_read(
        (*t).tid,
        (*t).fix_addr as usize,
        &raw mut st as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<stat>(),
    ) != ::core::mem::size_of::<stat>() as isize
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
        (*t).fix_addr as usize,
        &raw mut st as *const ::core::ffi::c_void,
        ::core::mem::size_of::<stat>(),
    );
}
unsafe extern "C" fn x_statx(mut t: *mut eng_task) {
    let mut b: [u8; 256] = [0; 256];
    if eng_mem_read(
        (*t).tid,
        (*t).fix_addr as usize,
        &raw mut b as *mut u8 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[u8; 256]>(),
    ) != ::core::mem::size_of::<[u8; 256]>() as isize
    {
        return;
    }
    let mut st: stat = platform_empty_stat();
    memset(
        &raw mut st as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<stat>(),
    );
    let mut mode: u16 = 0;
    let mut ino: u64 = 0;
    let mut dmaj: u32 = 0;
    let mut dmin: u32 = 0;
    let mut cs: i64 = 0;
    let mut cn: u32 = 0;
    memcpy(
        &raw mut mode as *mut ::core::ffi::c_void,
        (&raw mut b as *mut u8).offset(28 as i32 as isize) as *const ::core::ffi::c_void,
        2 as usize,
    );
    memcpy(
        &raw mut ino as *mut ::core::ffi::c_void,
        (&raw mut b as *mut u8).offset(32 as i32 as isize) as *const ::core::ffi::c_void,
        8 as usize,
    );
    memcpy(
        &raw mut dmaj as *mut ::core::ffi::c_void,
        (&raw mut b as *mut u8).offset(136 as i32 as isize) as *const ::core::ffi::c_void,
        4 as usize,
    );
    memcpy(
        &raw mut dmin as *mut ::core::ffi::c_void,
        (&raw mut b as *mut u8).offset(140 as i32 as isize) as *const ::core::ffi::c_void,
        4 as usize,
    );
    memcpy(
        &raw mut cs as *mut ::core::ffi::c_void,
        (&raw mut b as *mut u8).offset(96 as i32 as isize) as *const ::core::ffi::c_void,
        8 as usize,
    );
    memcpy(
        &raw mut cn as *mut ::core::ffi::c_void,
        (&raw mut b as *mut u8).offset(104 as i32 as isize) as *const ::core::ffi::c_void,
        4 as usize,
    );
    st.st_mode = mode as u32;
    st.st_ino = ino as u64;
    #[cfg(target_os = "linux")]
    {
        st.st_dev = gnu_dev_makedev(dmaj as u32, dmin as u32);
    }
    #[cfg(target_os = "android")]
    {
        st.st_dev = ((dmaj as u64 & 0xfffff000 as u64) << 32 as i32
            | (dmaj as u64 & 0xfff as u64) << 8 as i32
            | (dmin as u64 & 0xffffff00 as u64) << 12 as i32
            | dmin as u64 & 0xff as u64) as u64;
    }
    st.st_ctim.tv_sec = cs as i64;
    st.st_ctim.tv_nsec = cn as i64;
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
        &raw mut b as *mut u8 as *mut ::core::ffi::c_void,
    );
    eng_mem_write(
        (*t).tid,
        (*t).fix_addr as usize,
        &raw mut b as *mut u8 as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[u8; 256]>(),
    );
}
unsafe extern "C" fn h_access(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut mode_arg: i32,
    mut flag_arg: i32,
) -> i32 {
    let mut flags: i64 = if flag_arg >= 0 as i32 {
        eng_arg(r, flag_arg) as i64
    } else {
        0 as i64
    };
    let mut mode: i64 = eng_arg(r, mode_arg) as i64;
    if mode & !(7 as i32 as i64) != 0 {
        return eng_task_void(t, r, -(EINVAL as i64));
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
    let mut empty: i32 = 0 as i32;
    let mut rc: i32 = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        if flags & AT_SYMLINK_NOFOLLOW as i64 != 0 {
            0 as i32
        } else {
            ENG_RES_FOLLOW
        },
        (flags & AT_EMPTY_PATH as i64 != 0 as i64) as i32,
        &raw mut res,
        &raw mut empty,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    if empty != 0 {
        fd_proc_path(
            t,
            eng_arg(r, dfd_arg) as i64 as i32,
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
            return eng_task_void(t, r, rc as i64);
        }
        return 1 as i32;
    } else {
        snprintf(
            &raw mut host as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut res.host as *mut ::core::ffi::c_char,
        );
    }
    let mut st: stat = platform_empty_stat();
    let mut nf: i32 = (flags & AT_SYMLINK_NOFOLLOW as i64 != 0 && res.stub == 0) as i32;
    rc = host_stat(&raw mut host as *mut ::core::ffi::c_char, nf, &raw mut st);
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if mode == F_OK as i64 {
        return eng_task_void(t, r, 0 as i64);
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
            mode as i32,
            (flags & AT_EACCESS as i64 == 0) as i32,
        ) as i64,
    );
}
unsafe extern "C" fn h_mkdir(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut mode_arg: i32,
) -> i32 {
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
    let mut rc: i32 = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        ENG_RES_MISSING_OK,
        0 as i32,
        &raw mut res,
        ::core::ptr::null_mut::<i32>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if res.exists != 0 {
        return eng_task_void(t, r, -(EEXIST as i64));
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
        return eng_task_void(t, r, rc as i64);
    }
    let mut mode: u32 = eng_arg(r, mode_arg) as i64 as u32 & 0o7777 as u32;
    (*t).fix_mode =
        S_IFDIR as u32 | mode & !(*t).cr.umask & 0o1777 as u32 | pm.mode & S_ISGID as u32;
    (*t).fix_aux = new_gid(t, &raw mut pm) as i64;
    snprintf(
        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    let mut store: i32 = eng_meta_in_store(
        eng_tracer_guest((*t).tr),
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    eng_set_arg(
        r,
        mode_arg,
        (if store != 0 {
            (mode | 0o700 as u32) & 0o777 as u32
        } else {
            mode & 0o777 as u32
        }) as u64,
    );
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if store != 0 {
        (*t).fixup = ENG_FIX_CREATE_PATH as i32;
    }
    return 1 as i32;
}
unsafe extern "C" fn h_mknod(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut mode_arg: i32,
    mut dev_arg: i32,
) -> i32 {
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
    let mut rc: i32 = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        ENG_RES_MISSING_OK,
        0 as i32,
        &raw mut res,
        ::core::ptr::null_mut::<i32>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if res.exists != 0 {
        return eng_task_void(t, r, -(EEXIST as i64));
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
        return eng_task_void(t, r, rc as i64);
    }
    let mut mode: u32 = eng_arg(r, mode_arg) as i64 as u32;
    let mut r#type: u32 = mode & S_IFMT as u32;
    if r#type == 0 {
        r#type = S_IFREG as u32;
    }
    let mut perm: u32 = mode & 0o7777 as u32 & !(*t).cr.umask;
    let mut store: i32 = eng_meta_in_store(
        eng_tracer_guest((*t).tr),
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if r#type == S_IFCHR as u32
        || r#type == S_IFBLK as u32
        || store != 0 && (r#type == S_IFIFO as u32 || r#type == S_IFSOCK as u32)
    {
        if (r#type == S_IFCHR as u32 || r#type == S_IFBLK as u32)
            && (eng_capable(t, ENG_CAP_MKNOD) == 0 || store == 0)
        {
            return eng_task_void(t, r, -(EPERM as i64));
        }
        let mut fd: i32 = open(
            &raw mut res.entry as *mut ::core::ffi::c_char,
            O_CREAT | O_EXCL | O_WRONLY | O_CLOEXEC,
            0o600 as i32,
        );
        if fd < 0 as i32 {
            return eng_task_void(t, r, -*errno() as i64);
        }
        close(fd);
        #[cfg(target_os = "linux")]
        let mut dev: u64 = if r#type == S_IFCHR as u32 || r#type == S_IFBLK as u32 {
            eng_arg(r, dev_arg) as u64
        } else {
            0 as u64
        };
        #[cfg(target_os = "android")]
        let mut dev: u64 = if r#type == S_IFCHR as u32 || r#type == S_IFBLK as u32 {
            eng_arg(r, dev_arg)
        } else {
            0 as u64
        };
        let mut m: eng_meta = eng_meta {
            uid: (*t).cr.fsuid,
            gid: new_gid(t, &raw mut pm),
            mode: r#type | perm,
            nlink: 0,
            #[cfg(target_os = "linux")]
            major: gnu_dev_major(dev) as u32,
            #[cfg(target_os = "android")]
            major: (dev as u64 >> 32 as i32 & 0xfffff000 as u64
                | (dev >> 8 as i32 & 0xfff as u64) as u64) as u32,
            #[cfg(target_os = "linux")]
            minor: gnu_dev_minor(dev) as u32,
            #[cfg(target_os = "android")]
            minor: (dev >> 12 as i32 & 0xffffff00 as u64 | dev & 0xff as u64) as u32,
            present: 1 as i32,
        };
        rc = eng_meta_write(
            &raw mut res.entry as *mut ::core::ffi::c_char,
            0 as i32,
            &raw mut m,
        );
        if rc != 0 {
            unlink(&raw mut res.entry as *mut ::core::ffi::c_char);
        }
        return eng_task_void(t, r, rc as i64);
    }
    if r#type != S_IFREG as u32 && r#type != S_IFIFO as u32 && r#type != S_IFSOCK as u32 {
        return eng_task_void(t, r, -(EINVAL as i64));
    }
    (*t).fix_mode = r#type | perm;
    (*t).fix_aux = new_gid(t, &raw mut pm) as i64;
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
                (perm | 0o600 as u32) & 0o777 as u32
            } else {
                eng_arg(r, mode_arg) as i64 as u32 & 0o777 as u32
            }) as u64,
    );
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if store != 0 {
        (*t).fixup = ENG_FIX_CREATE_PATH as i32;
    }
    return 1 as i32;
}
unsafe extern "C" fn h_symlink(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: i32,
    mut path_arg: i32,
) -> i32 {
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
    let mut rc: i32 = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        ENG_RES_MISSING_OK,
        0 as i32,
        &raw mut res,
        ::core::ptr::null_mut::<i32>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if res.exists != 0 {
        return eng_task_void(t, r, -(EEXIST as i64));
    }
    rc = may_create(
        t,
        &raw mut res.entry as *mut ::core::ffi::c_char,
        ::core::ptr::null_mut::<eng_meta>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    return 1 as i32;
}
unsafe extern "C" fn h_unlink(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut rmdir: i32,
) -> i32 {
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
    let mut rc: i32 = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        0 as i32,
        0 as i32,
        &raw mut res,
        ::core::ptr::null_mut::<i32>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if res.verbatim == 0 && res.magic == 0 && {
        rc = may_delete(t, &raw mut res.entry as *mut ::core::ffi::c_char);
        rc != 0
    } {
        return eng_task_void(t, r, rc as i64);
    }
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if res.stub != 0 && rmdir == 0 {
        snprintf(
            &raw mut (*t).fix_id as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut res.stub_id as *mut ::core::ffi::c_char,
        );
        (*t).fixup = ENG_FIX_DROP_LINK as i32;
    }
    return 1 as i32;
}
unsafe extern "C" fn h_rename(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut d1: i32,
    mut p1: i32,
    mut d2: i32,
    mut p2: i32,
    mut flag_arg: i32,
) -> i32 {
    let mut flags: i64 = if flag_arg >= 0 as i32 {
        eng_arg(r, flag_arg) as i64
    } else {
        0 as i64
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
    let mut rc: i32 = resolve_arg(
        t,
        r,
        d1,
        p1,
        0 as i32,
        0 as i32,
        &raw mut a,
        ::core::ptr::null_mut::<i32>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    rc = resolve_arg(
        t,
        r,
        d2,
        p2,
        ENG_RES_MISSING_OK,
        0 as i32,
        &raw mut b,
        ::core::ptr::null_mut::<i32>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    rc = may_delete(t, &raw mut a.entry as *mut ::core::ffi::c_char);
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
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
        return eng_task_void(t, r, rc as i64);
    }
    let mut sa: i32 = eng_meta_in_store(
        eng_tracer_guest((*t).tr),
        &raw mut a.entry as *mut ::core::ffi::c_char,
    );
    let mut sb: i32 = eng_meta_in_store(
        eng_tracer_guest((*t).tr),
        &raw mut b.entry as *mut ::core::ffi::c_char,
    );
    if a.stub != 0 && sb == 0 || b.stub != 0 && flags & RENAME_EXCHANGE as i64 != 0 && sa == 0 {
        return eng_task_void(t, r, -(EXDEV as i64));
    }
    rc = put_path(t, r, p1, &raw mut a.entry as *mut ::core::ffi::c_char);
    if rc != 0 || {
        rc = put_path(t, r, p2, &raw mut b.entry as *mut ::core::ffi::c_char);
        rc != 0
    } {
        return eng_task_void(t, r, rc as i64);
    }
    if b.exists != 0
        && b.stub != 0
        && flags & RENAME_EXCHANGE as i64 == 0
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
        (*t).fixup = ENG_FIX_DROP_LINK as i32;
    } else if sa == 0 && sb != 0 && flags & RENAME_EXCHANGE as i64 == 0 {
        let mut u: u32 = 0 as u32;
        let mut gg: u32 = 0 as u32;
        eng_guest_default_owner(
            eng_tracer_guest((*t).tr),
            &raw mut a.entry as *mut ::core::ffi::c_char,
            &raw mut u,
            &raw mut gg,
        );
        (*t).fix_mode = u as u32;
        (*t).fix_aux = gg as i64;
        snprintf(
            &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut b.entry as *mut ::core::ffi::c_char,
        );
        (*t).fixup = ENG_FIX_RENAME_IN as i32;
    }
    return 1 as i32;
}
unsafe extern "C" fn h_link(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut d1: i32,
    mut p1: i32,
    mut d2: i32,
    mut p2: i32,
    mut flag_arg: i32,
) -> i32 {
    let mut flags: i64 = if flag_arg >= 0 as i32 {
        eng_arg(r, flag_arg) as i64
    } else {
        0 as i64
    };
    if flags & AT_EMPTY_PATH as i64 != 0 {
        return eng_task_void(t, r, -(EPERM as i64));
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
    let mut rc: i32 = resolve_arg(
        t,
        r,
        d1,
        p1,
        if flags & AT_SYMLINK_FOLLOW as i64 != 0 {
            ENG_RES_FOLLOW
        } else {
            0 as i32
        },
        0 as i32,
        &raw mut a,
        ::core::ptr::null_mut::<i32>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    rc = resolve_arg(
        t,
        r,
        d2,
        p2,
        ENG_RES_MISSING_OK,
        0 as i32,
        &raw mut b,
        ::core::ptr::null_mut::<i32>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if b.exists != 0 {
        return eng_task_void(t, r, -(EEXIST as i64));
    }
    let mut ra: i32 = in_rootfs(t, &raw mut a.entry as *mut ::core::ffi::c_char);
    let mut rb: i32 = in_rootfs(t, &raw mut b.entry as *mut ::core::ffi::c_char);
    if ra == 0 || rb == 0 {
        return eng_task_void(t, r, (if ra != rb { -EXDEV } else { -EPERM }) as i64);
    }
    rc = may_create(
        t,
        &raw mut b.entry as *mut ::core::ffi::c_char,
        ::core::ptr::null_mut::<eng_meta>(),
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    let mut st: stat = platform_empty_stat();
    if lstat(&raw mut a.host as *mut ::core::ffi::c_char, &raw mut st) == 0 as i32
        && st.st_mode & S_IFMT as u32 == S_IFDIR as u32
    {
        return eng_task_void(t, r, -(EPERM as i64));
    }
    return eng_task_void(
        t,
        r,
        eng_link_create(
            eng_tracer_guest((*t).tr),
            &raw mut a.entry as *mut ::core::ffi::c_char,
            &raw mut b.entry as *mut ::core::ffi::c_char,
        ) as i64,
    );
}
unsafe extern "C" fn meta_target(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut fd_arg: i32,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut nofollow: i32,
    mut allow_empty: i32,
    mut host: *mut ::core::ffi::c_char,
    mut st: *mut stat,
    mut nf: *mut i32,
) -> i32 {
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
    let mut empty: i32 = 0 as i32;
    if path_arg < 0 as i32 {
        fd_proc_path(t, eng_arg(r, fd_arg) as i64 as i32, host, PATH_MAX as usize);
        *nf = 0 as i32;
    } else {
        let mut rc: i32 = resolve_arg(
            t,
            r,
            dfd_arg,
            path_arg,
            if nofollow != 0 {
                0 as i32
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
                eng_arg(r, dfd_arg) as i64 as i32,
                host,
                PATH_MAX as usize,
            );
            *nf = 0 as i32;
        } else {
            snprintf(
                host,
                PATH_MAX as usize,
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut res.host as *mut ::core::ffi::c_char,
            );
            *nf = (nofollow != 0 && res.stub == 0) as i32;
        }
    }
    return host_stat(host, *nf, st);
}
unsafe extern "C" fn h_chmod(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut fd_arg: i32,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut mode_arg: i32,
    mut flag_arg: i32,
) -> i32 {
    let mut flags: i64 = if flag_arg >= 0 as i32 {
        eng_arg(r, flag_arg) as i64
    } else {
        0 as i64
    };
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut st: stat = platform_empty_stat();
    let mut nf: i32 = 0;
    let mut rc: i32 = meta_target(
        t,
        r,
        fd_arg,
        dfd_arg,
        path_arg,
        (flags & AT_SYMLINK_NOFOLLOW as i64 != 0 as i64) as i32,
        (flags & AT_EMPTY_PATH as i64 != 0 as i64) as i32,
        &raw mut host as *mut ::core::ffi::c_char,
        &raw mut st,
        &raw mut nf,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if st.st_mode & S_IFMT as u32 == S_IFLNK as u32 {
        return eng_task_void(t, r, -(EOPNOTSUPP as i64));
    }
    let mut lk: i32 = eng_meta_lock(eng_tracer_guest((*t).tr));
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
        return eng_task_void(t, r, -(EPERM as i64));
    }
    let mut mode: u32 = eng_arg(r, mode_arg) as i64 as u32 & 0o7777 as u32;
    if !(st.st_mode & S_IFMT as u32 == S_IFDIR as u32)
        && eng_in_group(t, m.gid, 0 as i32) == 0
        && eng_capable(t, ENG_CAP_FSETID) == 0
    {
        mode &= !(S_ISGID as u32);
    }
    if eng_meta_in_store(
        eng_tracer_guest((*t).tr),
        &raw mut host as *mut ::core::ffi::c_char,
    ) == 0
    {
        rc = if chmod(
            &raw mut host as *mut ::core::ffi::c_char,
            mode as u32 & 0o777 as u32,
        ) == 0 as i32
        {
            0 as i32
        } else {
            -*errno()
        };
    } else {
        m.mode = m.mode & S_IFMT as u32 | mode;
        rc = eng_meta_write(&raw mut host as *mut ::core::ffi::c_char, nf, &raw mut m);
        let mut hm: u32 = mode & 0o777 as u32
            | (if st.st_mode & S_IFMT as u32 == S_IFDIR as u32 {
                0o700 as i32
            } else {
                0o600 as i32
            }) as u32;
        if rc == -ENOTSUP || rc == -EOPNOTSUPP {
            rc = if chmod(
                &raw mut host as *mut ::core::ffi::c_char,
                mode as u32 & 0o777 as u32,
            ) == 0 as i32
            {
                0 as i32
            } else {
                -*errno()
            };
        } else if rc == 0 as i32 && st.st_mode as u32 & 0o7777 as u32 != hm {
            chmod(&raw mut host as *mut ::core::ffi::c_char, hm as u32);
        }
    }
    eng_meta_unlock(lk);
    return eng_task_void(t, r, rc as i64);
}
unsafe extern "C" fn h_chown(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut fd_arg: i32,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut uid_arg: i32,
    mut gid_arg: i32,
    mut flag_arg: i32,
    mut nofollow_always: i32,
) -> i32 {
    let mut flags: i64 = if flag_arg >= 0 as i32 {
        eng_arg(r, flag_arg) as i64
    } else {
        0 as i64
    };
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut st: stat = platform_empty_stat();
    let mut nf: i32 = 0;
    let mut rc: i32 = meta_target(
        t,
        r,
        fd_arg,
        dfd_arg,
        path_arg,
        (nofollow_always != 0 || flags & AT_SYMLINK_NOFOLLOW as i64 != 0) as i32,
        (flags & AT_EMPTY_PATH as i64 != 0 as i64) as i32,
        &raw mut host as *mut ::core::ffi::c_char,
        &raw mut st,
        &raw mut nf,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    let mut nu: u32 = eng_arg(r, uid_arg) as i64 as u32;
    let mut ng: u32 = eng_arg(r, gid_arg) as i64 as u32;
    let mut lk: i32 = eng_meta_lock(eng_tracer_guest((*t).tr));
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
    rc = 0 as i32;
    if eng_capable(t, ENG_CAP_CHOWN) == 0 {
        if nu != -1 as i32 as u32 && nu != m.uid {
            rc = -EPERM;
        } else if ng != -1 as i32 as u32
            && ((*t).cr.fsuid != m.uid || ng != m.gid && eng_in_group(t, ng, 0 as i32) == 0)
        {
            rc = -EPERM;
        } else if (*t).cr.fsuid != m.uid && (nu != -1 as i32 as u32 || ng != -1 as i32 as u32) {
            rc = -EPERM;
        }
    }
    if rc != 0
        || st.st_mode & S_IFMT as u32 == S_IFLNK as u32
        || eng_meta_in_store(
            eng_tracer_guest((*t).tr),
            &raw mut host as *mut ::core::ffi::c_char,
        ) == 0
    {
        eng_meta_unlock(lk);
        return eng_task_void(t, r, rc as i64);
    }
    if nu != -1 as i32 as u32 {
        m.uid = nu;
    }
    if ng != -1 as i32 as u32 {
        m.gid = ng;
    }
    if !(st.st_mode & S_IFMT as u32 == S_IFDIR as u32)
        && (nu != -1 as i32 as u32 || ng != -1 as i32 as u32)
    {
        m.mode &= !(S_ISUID as u32);
        if m.mode & S_IXGRP as u32 != 0 {
            m.mode &= !(S_ISGID as u32);
        }
    }
    rc = eng_meta_write(&raw mut host as *mut ::core::ffi::c_char, nf, &raw mut m);
    if rc == -ENOTSUP || rc == -EOPNOTSUPP {
        rc = if eng_capable(t, ENG_CAP_CHOWN) != 0 {
            0 as i32
        } else {
            -EPERM
        };
    }
    eng_meta_unlock(lk);
    return eng_task_void(t, r, rc as i64);
}
unsafe extern "C" fn h_readlink(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut dfd_arg: i32,
    mut path_arg: i32,
    mut buf_arg: i32,
    mut sz_arg: i32,
) -> i32 {
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
    let mut empty: i32 = 0 as i32;
    let mut rc: i32 = resolve_arg(
        t,
        r,
        dfd_arg,
        path_arg,
        0 as i32,
        1 as i32,
        &raw mut res,
        &raw mut empty,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if empty != 0 {
        return 0 as i32;
    }
    if res.stub != 0 {
        return eng_task_void(t, r, -(EINVAL as i64));
    }
    if res.magic != 0 {
        let mut bufsz: i64 = eng_arg(r, sz_arg) as i64;
        if bufsz <= 0 as i64 {
            return eng_task_void(t, r, -(EINVAL as i64));
        }
        let mut len: usize = strlen(&raw mut res.magic_text as *mut ::core::ffi::c_char);
        if len as i64 > bufsz {
            len = bufsz as usize;
        }
        if eng_mem_write(
            (*t).tid,
            eng_arg(r, buf_arg) as usize,
            &raw mut res.magic_text as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            len,
        ) != len as isize
        {
            return eng_task_void(t, r, -(EFAULT as i64));
        }
        return eng_task_void(t, r, len as i64);
    }
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.entry as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    return 1 as i32;
}
unsafe extern "C" fn h_getcwd(mut t: *mut eng_task, mut r: *mut eng_regs) -> i32 {
    let mut cwd: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut rc: i32 = eng_task_cwd(
        t,
        &raw mut cwd as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    if rc == -ENOENT {
        return 0 as i32;
    }
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    let mut p: [::core::ffi::c_char; 64] = [0; 64];
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut a: stat = platform_empty_stat();
    let mut b: stat = platform_empty_stat();
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
        b"/proc/%d/cwd\0".as_ptr() as *const ::core::ffi::c_char,
        (*t).tid,
    );
    #[cfg(target_arch = "x86_64")]
    if stat(&raw mut p as *mut ::core::ffi::c_char, &raw mut a) == 0 as i32
        && a.st_nlink == 0 as u64
    {
        return eng_task_void(t, r, -(ENOENT as i64));
    }
    #[cfg(target_arch = "aarch64")]
    if stat(&raw mut p as *mut ::core::ffi::c_char, &raw mut a) == 0 as i32
        && a.st_nlink == 0 as u32
    {
        return eng_task_void(t, r, -(ENOENT as i64));
    }
    if eng_guest_to_host(
        eng_tracer_guest((*t).tr),
        &raw mut cwd as *mut ::core::ffi::c_char,
        &raw mut host as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) == 0 as i32
        && stat(&raw mut p as *mut ::core::ffi::c_char, &raw mut a) == 0 as i32
        && (stat(&raw mut host as *mut ::core::ffi::c_char, &raw mut b) != 0 as i32
            || a.st_ino != b.st_ino
            || a.st_dev != b.st_dev)
    {
        return eng_task_void(t, r, -(ENOENT as i64));
    }
    let mut need: usize = strlen(&raw mut cwd as *mut ::core::ffi::c_char).wrapping_add(1 as usize);
    if (eng_arg(r, 1 as i32) as i64 as u64) < need as u64 {
        return eng_task_void(t, r, -(ERANGE as i64));
    }
    if eng_mem_write(
        (*t).tid,
        eng_arg(r, 0 as i32) as usize,
        &raw mut cwd as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
        need,
    ) != need as isize
    {
        return eng_task_void(t, r, -(EFAULT as i64));
    }
    return eng_task_void(t, r, need as i64);
}
unsafe extern "C" fn h_exec(mut t: *mut eng_task, mut r: *mut eng_regs, mut is_at: i32) -> i32 {
    let mut path: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut dfd: i32 = if is_at != 0 {
        eng_arg(r, 0 as i32) as i64 as i32
    } else {
        AT_FDCWD
    };
    let mut pidx: i32 = if is_at != 0 { 1 as i32 } else { 0 as i32 };
    let mut fl: i32 = if is_at != 0 {
        eng_arg(r, 4 as i32) as i64 as i32
    } else {
        0 as i32
    };
    let mut n: i64 = read_path(
        t,
        eng_arg(r, pidx),
        &raw mut path as *mut ::core::ffi::c_char,
    );
    if n < 0 as i64 {
        return eng_task_void(t, r, n);
    }
    let mut rc: i32 = eng_exec_prepare(t, dfd, &raw mut path as *mut ::core::ffi::c_char, fl);
    if rc != 0 {
        eng_logf!(
            eng_log_level::ENG_LOG_DEBUG,
            b"exec %s refused: %s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut path as *mut ::core::ffi::c_char,
            strerror(-rc),
        );
        return eng_task_void(t, r, rc as i64);
    }
    let mut a: u64 = eng_scratch_put_str(
        t,
        r,
        &raw mut (*(eng_tracer_guest as unsafe extern "C" fn(*mut eng_tracer) -> *mut eng_guest)(
            (*t).tr,
        ))
        .loader as *mut ::core::ffi::c_char,
    );
    if a == 0 {
        eng_exec_discard(t);
        return eng_task_void(t, r, -(ENOMEM as i64));
    }
    if is_at != 0 {
        let mut argv: u64 = eng_arg(r, 2 as i32);
        let mut envp: u64 = eng_arg(r, 3 as i32);
        eng_set_arg(r, 0 as i32, a);
        eng_set_arg(r, 1 as i32, argv);
        eng_set_arg(r, 2 as i32, envp);
        eng_syscall_set((*t).tid, r, __NR_execve as i64);
    } else {
        eng_set_arg(r, 0 as i32, a);
    }
    (*t).regs_modified = 1 as i32;
    return 1 as i32;
}
#[cfg(target_arch = "x86_64")]
static mut PATHSYS: [pathsys; 9] = [
    pathsys {
        nr: __NR_truncate as i64,
        dfd: -1 as i8,
        path: 0 as i8,
        flagarg: -1 as i8,
        mode: FL_FOLLOW as i32 as u8,
        nullok: 0 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_utime as i64,
        dfd: -1 as i8,
        path: 0 as i8,
        flagarg: -1 as i8,
        mode: FL_FOLLOW as i32 as u8,
        nullok: 0 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_utimes as i64,
        dfd: -1 as i8,
        path: 0 as i8,
        flagarg: -1 as i8,
        mode: FL_FOLLOW as i32 as u8,
        nullok: 0 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_futimesat as i64,
        dfd: 0 as i8,
        path: 1 as i8,
        flagarg: -1 as i8,
        mode: FL_FOLLOW as i32 as u8,
        nullok: 1 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_utimensat as i64,
        dfd: 0 as i8,
        path: 1 as i8,
        flagarg: 3 as i8,
        mode: FL_ATFLAG as i32 as u8,
        nullok: 1 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_statfs as i64,
        dfd: -1 as i8,
        path: 0 as i8,
        flagarg: -1 as i8,
        mode: FL_FOLLOW as i32 as u8,
        nullok: 0 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_chdir as i64,
        dfd: -1 as i8,
        path: 0 as i8,
        flagarg: -1 as i8,
        mode: FL_FOLLOW as i32 as u8,
        nullok: 0 as u8,
        dironly: 1 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_inotify_add_watch as i64,
        dfd: -1 as i8,
        path: 1 as i8,
        flagarg: 2 as i8,
        mode: FL_INOTIFY as i32 as u8,
        nullok: 0 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_rmdir as i64,
        dfd: -1 as i8,
        path: 0 as i8,
        flagarg: -1 as i8,
        mode: FL_NOFOLLOW as i32 as u8,
        nullok: 0 as u8,
        dironly: 0 as u8,
        entry: 1 as u8,
    },
];
#[cfg(target_arch = "aarch64")]
static mut PATHSYS: [pathsys; 5] = [
    pathsys {
        nr: __NR_truncate as i64,
        dfd: -1 as i8,
        path: 0 as i8,
        flagarg: -1 as i8,
        mode: FL_FOLLOW as i32 as u8,
        nullok: 0 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_utimensat as i64,
        dfd: 0 as i8,
        path: 1 as i8,
        flagarg: 3 as i8,
        mode: FL_ATFLAG as i32 as u8,
        nullok: 1 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_statfs as i64,
        dfd: -1 as i8,
        path: 0 as i8,
        flagarg: -1 as i8,
        mode: FL_FOLLOW as i32 as u8,
        nullok: 0 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_chdir as i64,
        dfd: -1 as i8,
        path: 0 as i8,
        flagarg: -1 as i8,
        mode: FL_FOLLOW as i32 as u8,
        nullok: 0 as u8,
        dironly: 1 as u8,
        entry: 0 as u8,
    },
    pathsys {
        nr: __NR_inotify_add_watch as i64,
        dfd: -1 as i8,
        path: 1 as i8,
        flagarg: 2 as i8,
        mode: FL_INOTIFY as i32 as u8,
        nullok: 0 as u8,
        dironly: 0 as u8,
        entry: 0 as u8,
    },
];
unsafe extern "C" fn h_pathsys(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut d: *const pathsys,
) -> i32 {
    let mut f: i64 = if (*d).flagarg as i32 >= 0 as i32 {
        eng_arg(r, (*d).flagarg as i32) as i64
    } else {
        0 as i64
    };
    let mut addr: u64 = eng_arg(r, (*d).path as i32);
    if addr == 0 && (*d).nullok as i32 != 0 {
        return 0 as i32;
    }
    let mut follow: i32 = ((*d).mode as i32 == FL_FOLLOW as i32
        || (*d).mode as i32 == FL_ATFLAG as i32 && f & AT_SYMLINK_NOFOLLOW as i64 == 0
        || (*d).mode as i32 == FL_INOTIFY as i32 && f & IN_DONT_FOLLOW as i64 == 0)
        as i32;
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
    let mut empty: i32 = 0 as i32;
    let mut rc: i32 = resolve_arg(
        t,
        r,
        (*d).dfd as i32,
        (*d).path as i32,
        if follow != 0 {
            ENG_RES_FOLLOW
        } else {
            0 as i32
        } | if (*d).dironly as i32 != 0 {
            ENG_RES_DIR_ONLY
        } else {
            0 as i32
        },
        ((*d).mode as i32 == FL_ATFLAG as i32 && f & AT_EMPTY_PATH as i64 != 0) as i32,
        &raw mut res,
        &raw mut empty,
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    if empty != 0 {
        return 0 as i32;
    }
    if (*d).nr == __NR_chdir as i64 && res.verbatim == 0 {
        let mut st: stat = platform_empty_stat();
        if stat(&raw mut res.host as *mut ::core::ffi::c_char, &raw mut st) == 0 as i32 {
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
                0 as i32,
                &raw mut st,
                &raw mut m,
            );
            rc = eng_meta_permission(t, &raw mut m, X_OK, 0 as i32);
            if rc != 0 {
                return eng_task_void(t, r, rc as i64);
            }
        }
    }
    rc = put_path(
        t,
        r,
        (*d).path as i32,
        if (*d).entry as i32 != 0 {
            &raw mut res.entry as *mut ::core::ffi::c_char
        } else {
            &raw mut res.host as *mut ::core::ffi::c_char
        },
    );
    if rc != 0 {
        return eng_task_void(t, r, rc as i64);
    }
    return 1 as i32;
}
unsafe extern "C" fn h_xattr(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut op: i32,
    mut path_arg: i32,
    mut nofollow: i32,
) -> i32 {
    if op != XA_LIST as i32 {
        let mut name: [::core::ffi::c_char; 256] = [0; 256];
        let mut n: isize = eng_mem_read_cstr(
            (*t).tid,
            eng_arg(r, 1 as i32) as usize,
            &raw mut name as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 256]>(),
        );
        if n < 0 as isize {
            return eng_task_void(t, r, -(EFAULT as i64));
        }
        if strncmp(
            &raw mut name as *mut ::core::ffi::c_char,
            ENG_META_PREFIX.as_ptr(),
            ::core::mem::size_of::<[::core::ffi::c_char; 15]>().wrapping_sub(1 as usize),
        ) == 0
        {
            return eng_task_void(
                t,
                r,
                (if op == XA_GET as i32 {
                    -ENODATA
                } else {
                    -EPERM
                }) as i64,
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
                (if op == XA_GET as i32 {
                    -ENODATA
                } else {
                    -EOPNOTSUPP
                }) as i64,
            );
        }
        if op == XA_SET as i32
            && strcmp(
                &raw mut name as *mut ::core::ffi::c_char,
                b"security.capability\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
        {
            return eng_task_void(t, r, -(EOPNOTSUPP as i64));
        }
    } else {
        (*t).fix_addr = eng_arg(r, 1 as i32);
        (*t).fix_len = eng_arg(r, 2 as i32);
        (*t).fixup = ENG_FIX_LISTXATTR as i32;
    }
    if path_arg < 0 as i32 {
        return 0 as i32;
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
    let mut rc: i32 = resolve_arg(
        t,
        r,
        -1 as i32,
        path_arg,
        if nofollow != 0 {
            0 as i32
        } else {
            ENG_RES_FOLLOW
        },
        0 as i32,
        &raw mut res,
        ::core::ptr::null_mut::<i32>(),
    );
    if rc != 0 {
        (*t).fixup = 0 as i32;
        return eng_task_void(t, r, rc as i64);
    }
    rc = put_path(
        t,
        r,
        path_arg,
        &raw mut res.host as *mut ::core::ffi::c_char,
    );
    if rc != 0 {
        (*t).fixup = 0 as i32;
        return eng_task_void(t, r, rc as i64);
    }
    return 1 as i32;
}
unsafe extern "C" fn x_listxattr(mut t: *mut eng_task, mut ret: i64) {
    if ret <= 0 as i64 || (*t).fix_addr == 0 || (*t).fix_len == 0 {
        return;
    }
    let mut buf: *mut ::core::ffi::c_char = malloc(ret as usize) as *mut ::core::ffi::c_char;
    if buf.is_null() {
        return;
    }
    if eng_mem_read(
        (*t).tid,
        (*t).fix_addr as usize,
        buf as *mut ::core::ffi::c_void,
        ret as usize,
    ) == ret as isize
    {
        let mut out: *mut ::core::ffi::c_char = malloc(ret as usize) as *mut ::core::ffi::c_char;
        let mut o: usize = 0 as usize;
        let mut i: i64 = 0 as i64;
        while i < ret {
            let mut l: usize = strnlen(buf.offset(i as isize), (ret - i) as usize);
            if strncmp(
                buf.offset(i as isize),
                ENG_META_PREFIX.as_ptr(),
                ::core::mem::size_of::<[::core::ffi::c_char; 15]>().wrapping_sub(1 as usize),
            ) != 0
                && strcmp(
                    buf.offset(i as isize),
                    b"security.selinux\0".as_ptr() as *const ::core::ffi::c_char,
                ) != 0
            {
                memcpy(
                    out.offset(o as isize) as *mut ::core::ffi::c_void,
                    buf.offset(i as isize) as *const ::core::ffi::c_void,
                    l.wrapping_add(1 as usize),
                );
                o = o.wrapping_add(l.wrapping_add(1 as usize));
            }
            i += l as i64 + 1 as i64;
        }
        if o as i64 != ret {
            eng_mem_write(
                (*t).tid,
                (*t).fix_addr as usize,
                out as *const ::core::ffi::c_void,
                o,
            );
            (*t).inject_result = o as i64;
            (*t).void_pending = 1 as i32;
        }
        free(out as *mut ::core::ffi::c_void);
    }
    free(buf as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn x_getdents(mut t: *mut eng_task, mut ret: i64) {
    if ret <= 0 as i64 {
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
        (*t).fix_aux as i32,
    );
    let mut dn: isize = readlink(
        &raw mut link as *mut ::core::ffi::c_char,
        &raw mut dirhost as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as usize),
    );
    if dn <= 0 as isize {
        return;
    }
    dirhost[dn as usize] = 0 as ::core::ffi::c_char;
    let mut have_guest: i32 = (eng_host_to_guest(
        g,
        &raw mut dirhost as *mut ::core::ffi::c_char,
        &raw mut dirguest as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) == 0 as i32) as i32;
    let mut buf: *mut u8 = malloc(ret as usize) as *mut u8;
    if buf.is_null() {
        return;
    }
    if eng_mem_read(
        (*t).tid,
        (*t).fix_addr as usize,
        buf as *mut ::core::ffi::c_void,
        ret as usize,
    ) != ret as isize
    {
        free(buf as *mut ::core::ffi::c_void);
        return;
    }
    let mut o: i64 = 0 as i64;
    let mut changed: i64 = 0 as i64;
    let mut i: i64 = 0 as i64;
    while i + ::core::mem::size_of::<dirent64_hdr>() as i64 <= ret {
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
        if h.reclen as i32 == 0 as i32 || i + h.reclen as i64 > ret {
            break;
        }
        let mut name: *const ::core::ffi::c_char = (buf as *const ::core::ffi::c_char)
            .offset(i as isize)
            .offset(::core::mem::size_of::<dirent64_hdr>() as isize);
        let mut drop_0: i32 = 0 as i32;
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
        if drop_0 == 0 && h.r#type as i32 == 10 as i32 {
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
            let mut n: isize = readlink(
                &raw mut hp as *mut ::core::ffi::c_char,
                &raw mut txt as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as usize),
            );
            if n > 0 as isize {
                txt[n as usize] = 0 as ::core::ffi::c_char;
                if eng_link_is_stub_text(
                    &raw mut txt as *mut ::core::ffi::c_char,
                    &raw mut id as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                ) != 0
                {
                    let mut obj: [::core::ffi::c_char; 4096] = [0; 4096];
                    let mut st: stat = platform_empty_stat();
                    h.r#type = 8 as u8;
                    if eng_link_object_path(
                        g,
                        &raw mut id as *mut ::core::ffi::c_char,
                        &raw mut obj as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                    ) == 0 as i32
                        && stat(&raw mut obj as *mut ::core::ffi::c_char, &raw mut st) == 0 as i32
                    {
                        h.ino = st.st_ino as u64;
                    }
                    memcpy(
                        buf.offset(i as isize) as *mut ::core::ffi::c_void,
                        &raw mut h as *const ::core::ffi::c_void,
                        ::core::mem::size_of::<dirent64_hdr>(),
                    );
                    changed = 1 as i64;
                }
            }
        }
        if drop_0 != 0 {
            changed = 1 as i64;
        } else {
            if o != i {
                memmove(
                    buf.offset(o as isize) as *mut ::core::ffi::c_void,
                    buf.offset(i as isize) as *const ::core::ffi::c_void,
                    h.reclen as usize,
                );
            }
            o += h.reclen as i64;
        }
        i += h.reclen as i64;
    }
    if changed != 0 {
        eng_mem_write(
            (*t).tid,
            (*t).fix_addr as usize,
            buf as *const ::core::ffi::c_void,
            o as usize,
        );
        if o != ret {
            (*t).inject_result = o;
            (*t).void_pending = 1 as i32;
        }
    }
    free(buf as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn default_policy(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut nr: i64,
) -> i32 {
    let mut c: eng_sc_class = eng_sysinv_class(nr);
    static mut warned: [u8; 1024] = [0; 1024];
    match c {
        eng_sc_class::ENG_SC_PASS
        | eng_sc_class::ENG_SC_FD
        | eng_sc_class::ENG_SC_PROC
        | eng_sc_class::ENG_SC_SOCK
        | eng_sc_class::ENG_SC_ID => return 0 as i32,
        eng_sc_class::ENG_SC_EPERM => {
            return eng_task_void(t, r, -(EPERM as i64));
        }
        _ => {
            if nr >= 0 as i64
                && nr < ::core::mem::size_of::<[u8; 1024]>() as i64
                && warned[nr as usize] == 0
            {
                warned[nr as usize] = 1 as u8;
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
            return eng_task_void(t, r, -(ENOSYS as i64));
        }
    };
}
pub const SUN_PATH_MAX: i32 = 108 as i32;
unsafe extern "C" fn fnv1a(mut s: *const ::core::ffi::c_char) -> u64 {
    let mut h: u64 = 1469598103934665603 as u64;
    while *s != 0 {
        h ^= *s as u8 as u64;
        h = (h as u64).wrapping_mul(1099511628211 as u64) as u64;
        s = s.offset(1);
    }
    return h;
}
unsafe extern "C" fn sock_host_name(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
) -> i32 {
    if strlen(host) < SUN_PATH_MAX as usize {
        snprintf(
            out,
            SUN_PATH_MAX as usize,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            host,
        );
        return 0 as i32;
    }
    let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        host,
    );
    let mut sl: *mut ::core::ffi::c_char =
        strrchr(&raw mut dir as *mut ::core::ffi::c_char, '/' as i32);
    if sl.is_null() || (*g).sockdir[0usize] == 0 {
        return -ENAMETOOLONG;
    }
    *sl = 0 as ::core::ffi::c_char;
    let mut base: *const ::core::ffi::c_char = sl.offset(1 as i32 as isize);
    let mut alias: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut alias as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/%016llx\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).sockdir as *mut ::core::ffi::c_char,
        fnv1a(&raw mut dir as *mut ::core::ffi::c_char) as u64,
    );
    let mut cur: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut n: isize = readlink(
        &raw mut alias as *mut ::core::ffi::c_char,
        &raw mut cur as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as usize),
    );
    if n < 0 as isize
        || n as usize != strlen(&raw mut dir as *mut ::core::ffi::c_char)
        || memcmp(
            &raw mut cur as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            &raw mut dir as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            n as usize,
        ) != 0
    {
        mkdir(
            &raw mut (*g).sockdir as *mut ::core::ffi::c_char,
            0o700 as u32,
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
        ) != 0 as i32
            || rename(
                &raw mut tmp as *mut ::core::ffi::c_char,
                &raw mut alias as *mut ::core::ffi::c_char,
            ) != 0 as i32
        {
            unlink(&raw mut tmp as *mut ::core::ffi::c_char);
            return -ENAMETOOLONG;
        }
    }
    if strlen(&raw mut alias as *mut ::core::ffi::c_char)
        .wrapping_add(1 as usize)
        .wrapping_add(strlen(base))
        >= SUN_PATH_MAX as usize
    {
        return -ENAMETOOLONG;
    }
    snprintf(
        out,
        SUN_PATH_MAX as usize,
        b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut alias as *mut ::core::ffi::c_char,
        base,
    );
    return 0 as i32;
}
unsafe extern "C" fn sock_translate(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut addr: u64,
    mut len: u64,
    mut creating: i32,
    mut new_addr: *mut u64,
    mut new_len: *mut u64,
) -> i32 {
    if addr == 0 || len <= 2 as u64 || len > ::core::mem::size_of::<wf_sockaddr_un>() as u64 {
        return 1 as i32;
    }
    let mut sa: wf_sockaddr_un = wf_sockaddr_un {
        family: 0,
        path: [0; 108],
    };
    memset(
        &raw mut sa as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<wf_sockaddr_un>(),
    );
    if eng_mem_read(
        (*t).tid,
        addr as usize,
        &raw mut sa as *mut ::core::ffi::c_void,
        len as usize,
    ) != len as isize
    {
        return -EFAULT;
    }
    if sa.family as i32 != 1 as i32 || sa.path[0usize] as i32 == 0 as i32 {
        return 1 as i32;
    }
    let mut gp: [::core::ffi::c_char; 109] = [0; 109];
    let mut pl: usize = (len as usize).wrapping_sub(2 as usize);
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
    let mut rc: i32 = eng_resolve(
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
        0 as i32,
        ::core::mem::size_of::<wf_sockaddr_un>(),
    );
    out.family = 1 as u16;
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
    let mut ol: usize = (2 as usize)
        .wrapping_add(strlen(&raw mut out.path as *mut ::core::ffi::c_char))
        .wrapping_add(1 as usize);
    let mut a: u64 = eng_scratch_alloc(t, r, ::core::mem::size_of::<wf_sockaddr_un>());
    if a == 0
        || eng_mem_write(
            (*t).tid,
            a as usize,
            &raw mut out as *const ::core::ffi::c_void,
            ol,
        ) != ol as isize
    {
        return -EFAULT;
    }
    *new_addr = a;
    *new_len = ol as u64;
    return 0 as i32;
}
unsafe extern "C" fn h_sockaddr(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut addr_arg: i32,
    mut len_arg: i32,
    mut creating: i32,
) -> i32 {
    let mut na: u64 = 0;
    let mut nl: u64 = 0;
    let mut rc: i32 = sock_translate(
        t,
        r,
        eng_arg(r, addr_arg),
        eng_arg(r, len_arg),
        creating,
        &raw mut na,
        &raw mut nl,
    );
    if rc < 0 as i32 {
        return eng_task_void(t, r, rc as i64);
    }
    if rc == 1 as i32 {
        return 0 as i32;
    }
    eng_set_arg(r, addr_arg, na);
    eng_set_arg(r, len_arg, nl);
    (*t).regs_modified = 1 as i32;
    return 1 as i32;
}
unsafe extern "C" fn h_sendmsg(mut t: *mut eng_task, mut r: *mut eng_regs) -> i32 {
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
    let mut ma: u64 = eng_arg(r, 1 as i32);
    if ma == 0
        || eng_mem_read(
            (*t).tid,
            ma as usize,
            &raw mut m as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<wf_msghdr>(),
        ) != ::core::mem::size_of::<wf_msghdr>() as isize
    {
        return 0 as i32;
    }
    let mut na: u64 = 0;
    let mut nl: u64 = 0;
    let mut rc: i32 = sock_translate(
        t,
        r,
        m.name,
        m.namelen as u64,
        0 as i32,
        &raw mut na,
        &raw mut nl,
    );
    if rc < 0 as i32 {
        return eng_task_void(t, r, rc as i64);
    }
    if rc == 1 as i32 {
        return 0 as i32;
    }
    m.name = na;
    m.namelen = nl as u32;
    let mut a: u64 = eng_scratch_alloc(t, r, ::core::mem::size_of::<wf_msghdr>());
    if a == 0
        || eng_mem_write(
            (*t).tid,
            a as usize,
            &raw mut m as *const ::core::ffi::c_void,
            ::core::mem::size_of::<wf_msghdr>(),
        ) != ::core::mem::size_of::<wf_msghdr>() as isize
    {
        return eng_task_void(t, r, -(EFAULT as i64));
    }
    eng_set_arg(r, 1 as i32, a);
    (*t).regs_modified = 1 as i32;
    return 1 as i32;
}
unsafe extern "C" fn x_sockname(mut t: *mut eng_task, mut ret: i64) {
    if ret < 0 as i64 || (*t).fix_addr == 0 || (*t).fix_len == 0 {
        return;
    }
    let mut len: u32 = 0;
    if eng_mem_read(
        (*t).tid,
        (*t).fix_len as usize,
        &raw mut len as *mut ::core::ffi::c_void,
        4 as usize,
    ) != 4 as isize
        || len <= 2 as u32
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
        0 as i32,
        ::core::mem::size_of::<wf_sockaddr_un>(),
    );
    let mut cap: u32 = (*t).fix_aux as u32;
    let mut rd: u32 = if len < cap { len } else { cap };
    if rd <= 2 as u32
        || eng_mem_read(
            (*t).tid,
            (*t).fix_addr as usize,
            &raw mut sa as *mut ::core::ffi::c_void,
            rd as usize,
        ) != rd as isize
    {
        return;
    }
    if sa.family as i32 != 1 as i32 || sa.path[0usize] as i32 == 0 as i32 || rd < len {
        return;
    }
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut guest: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut host as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%.*s\0".as_ptr() as *const ::core::ffi::c_char,
        len.wrapping_sub(2 as u32) as i32,
        &raw mut sa.path as *mut ::core::ffi::c_char,
    );
    let mut g: *mut eng_guest = eng_tracer_guest((*t).tr);
    let mut sdl: usize = strlen(&raw mut (*g).sockdir as *mut ::core::ffi::c_char);
    if sdl != 0
        && strncmp(
            &raw mut host as *mut ::core::ffi::c_char,
            &raw mut (*g).sockdir as *mut ::core::ffi::c_char,
            sdl,
        ) == 0
        && host[sdl] as i32 == '/' as i32
    {
        let mut alias: [::core::ffi::c_char; 4096] = [0; 4096];
        let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
        snprintf(
            &raw mut alias as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut host as *mut ::core::ffi::c_char,
        );
        let mut sl: *mut ::core::ffi::c_char =
            strrchr(&raw mut alias as *mut ::core::ffi::c_char, '/' as i32);
        if sl.is_null() {
            return;
        }
        *sl = 0 as ::core::ffi::c_char;
        let mut n: isize = readlink(
            &raw mut alias as *mut ::core::ffi::c_char,
            &raw mut dir as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as usize),
        );
        if n <= 0 as isize {
            return;
        }
        dir[n as usize] = 0 as ::core::ffi::c_char;
        snprintf(
            &raw mut host as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut dir as *mut ::core::ffi::c_char,
            sl.offset(1 as i32 as isize),
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
    let mut gl: usize = strlen(&raw mut guest as *mut ::core::ffi::c_char);
    if gl >= SUN_PATH_MAX as usize {
        return;
    }
    let mut out: wf_sockaddr_un = wf_sockaddr_un {
        family: 0,
        path: [0; 108],
    };
    memset(
        &raw mut out as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<wf_sockaddr_un>(),
    );
    out.family = 1 as u16;
    memcpy(
        &raw mut out.path as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        &raw mut guest as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
        gl.wrapping_add(1 as usize),
    );
    let mut olen: u32 = (2 as usize).wrapping_add(gl).wrapping_add(1 as usize) as u32;
    let mut wr: u32 = if olen < cap { olen } else { cap };
    eng_mem_write(
        (*t).tid,
        (*t).fix_addr as usize,
        &raw mut out as *const ::core::ffi::c_void,
        wr as usize,
    );
    eng_mem_write(
        (*t).tid,
        (*t).fix_len as usize,
        &raw mut olen as *const ::core::ffi::c_void,
        4 as usize,
    );
}
unsafe extern "C" fn h_sockname(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut addr_arg: i32,
    mut lenp_arg: i32,
) -> i32 {
    (*t).fix_addr = eng_arg(r, addr_arg);
    (*t).fix_len = eng_arg(r, lenp_arg);
    let mut cap: u32 = 0 as u32;
    if (*t).fix_len != 0
        && eng_mem_read(
            (*t).tid,
            (*t).fix_len as usize,
            &raw mut cap as *mut ::core::ffi::c_void,
            4 as usize,
        ) != 4 as isize
    {
        cap = 0 as u32;
    }
    (*t).fix_aux = cap as i64;
    (*t).fixup = ENG_FIX_SOCKNAME as i32;
    return 0 as i32;
}
#[cfg(target_arch = "x86_64")]
unsafe extern "C" fn lg_tv_to_ts(
    mut t: *mut eng_task,
    mut r: *const eng_regs,
    mut a: u64,
    mut n: i32,
    mut out: *mut u64,
) -> i32 {
    *out = 0 as u64;
    if a == 0 {
        return 0 as i32;
    }
    let mut tv: [lg_timeval; 2] = [lg_timeval { sec: 0, usec: 0 }; 2];
    let mut ts: [lg_timespec; 2] = [lg_timespec { sec: 0, nsec: 0 }; 2];
    if eng_mem_read(
        (*t).tid,
        a as usize,
        &raw mut tv as *mut lg_timeval as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<lg_timeval>().wrapping_mul(n as usize),
    ) != ::core::mem::size_of::<lg_timeval>().wrapping_mul(n as usize) as isize
    {
        return -EFAULT;
    }
    let mut i: i32 = 0 as i32;
    while i < n {
        ts[i as usize].sec = tv[i as usize].sec;
        ts[i as usize].nsec = tv[i as usize].usec * 1000 as i64;
        i += 1;
    }
    let mut s: u64 = eng_scratch_alloc(
        t,
        r,
        ::core::mem::size_of::<lg_timespec>().wrapping_mul(n as usize),
    );
    if s == 0
        || eng_mem_write(
            (*t).tid,
            s as usize,
            &raw mut ts as *mut lg_timespec as *const ::core::ffi::c_void,
            ::core::mem::size_of::<lg_timespec>().wrapping_mul(n as usize),
        ) != ::core::mem::size_of::<lg_timespec>().wrapping_mul(n as usize) as isize
    {
        return -EFAULT;
    }
    *out = s;
    return 0 as i32;
}
#[cfg(target_arch = "x86_64")]
unsafe extern "C" fn lg_put(
    mut t: *mut eng_task,
    mut r: *const eng_regs,
    mut v: *const ::core::ffi::c_void,
    mut n: usize,
) -> u64 {
    let mut s: u64 = eng_scratch_alloc(t, r, n);
    if s != 0 && eng_mem_write((*t).tid, s as usize, v, n) != n as isize {
        s = 0 as u64;
    }
    return s;
}
#[cfg(target_arch = "x86_64")]
unsafe extern "C" fn legacy_convert(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut nr: *mut i64,
    mut res: *mut i64,
) -> i32 {
    let mut a0: u64 = eng_arg(r, 0 as i32);
    let mut a1: u64 = eng_arg(r, 1 as i32);
    let mut a2: u64 = eng_arg(r, 2 as i32);
    let mut a3: u64 = eng_arg(r, 3 as i32);
    let mut a4: u64 = eng_arg(r, 4 as i32);
    let CWD: u64 = AT_FDCWD as i64 as u64;
    (*t).lg_fix = LG_NONE as i32;
    let mut s: u64 = 0;
    let mut rc: i32 = 0;
    match *nr {
        2 => {
            *nr = __NR_openat as i64;
            let mut _v: [u64; 4] = [CWD, a0, a1, a2];
            let mut _i: usize = 0 as usize;
            while _i
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i as i32, _v[_i]);
                _i = _i.wrapping_add(1);
            }
            return 1 as i32;
        }
        85 => {
            *nr = __NR_openat as i64;
            let mut _v_0: [u64; 4] = [
                CWD,
                a0,
                (0o100 as i32 | 0o1 as i32 | 0o1000 as i32) as u64,
                a1,
            ];
            let mut _i_0: usize = 0 as usize;
            while _i_0
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_0 as i32, _v_0[_i_0]);
                _i_0 = _i_0.wrapping_add(1);
            }
            return 1 as i32;
        }
        21 => {
            *nr = __NR_faccessat as i64;
            let mut _v_1: [u64; 3] = [CWD, a0, a1];
            let mut _i_1: usize = 0 as usize;
            while _i_1
                < ::core::mem::size_of::<[u64; 3]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_1 as i32, _v_1[_i_1]);
                _i_1 = _i_1.wrapping_add(1);
            }
            return 1 as i32;
        }
        4 => {
            *nr = __NR_newfstatat as i64;
            let mut _v_2: [u64; 4] = [CWD, a0, a1, 0 as u64];
            let mut _i_2: usize = 0 as usize;
            while _i_2
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_2 as i32, _v_2[_i_2]);
                _i_2 = _i_2.wrapping_add(1);
            }
            return 1 as i32;
        }
        6 => {
            *nr = __NR_newfstatat as i64;
            let mut _v_3: [u64; 4] = [CWD, a0, a1, 0x100 as u64];
            let mut _i_3: usize = 0 as usize;
            while _i_3
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_3 as i32, _v_3[_i_3]);
                _i_3 = _i_3.wrapping_add(1);
            }
            return 1 as i32;
        }
        83 => {
            *nr = __NR_mkdirat as i64;
            let mut _v_4: [u64; 3] = [CWD, a0, a1];
            let mut _i_4: usize = 0 as usize;
            while _i_4
                < ::core::mem::size_of::<[u64; 3]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_4 as i32, _v_4[_i_4]);
                _i_4 = _i_4.wrapping_add(1);
            }
            return 1 as i32;
        }
        84 => {
            *nr = __NR_unlinkat as i64;
            let mut _v_5: [u64; 3] = [CWD, a0, 0x200 as u64];
            let mut _i_5: usize = 0 as usize;
            while _i_5
                < ::core::mem::size_of::<[u64; 3]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_5 as i32, _v_5[_i_5]);
                _i_5 = _i_5.wrapping_add(1);
            }
            return 1 as i32;
        }
        87 => {
            *nr = __NR_unlinkat as i64;
            let mut _v_6: [u64; 3] = [CWD, a0, 0 as u64];
            let mut _i_6: usize = 0 as usize;
            while _i_6
                < ::core::mem::size_of::<[u64; 3]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_6 as i32, _v_6[_i_6]);
                _i_6 = _i_6.wrapping_add(1);
            }
            return 1 as i32;
        }
        82 => {
            *nr = __NR_renameat as i64;
            let mut _v_7: [u64; 4] = [CWD, a0, CWD, a1];
            let mut _i_7: usize = 0 as usize;
            while _i_7
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_7 as i32, _v_7[_i_7]);
                _i_7 = _i_7.wrapping_add(1);
            }
            return 1 as i32;
        }
        86 => {
            *nr = __NR_linkat as i64;
            let mut _v_8: [u64; 5] = [CWD, a0, CWD, a1, 0 as u64];
            let mut _i_8: usize = 0 as usize;
            while _i_8
                < ::core::mem::size_of::<[u64; 5]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_8 as i32, _v_8[_i_8]);
                _i_8 = _i_8.wrapping_add(1);
            }
            return 1 as i32;
        }
        88 => {
            *nr = __NR_symlinkat as i64;
            let mut _v_9: [u64; 3] = [a0, CWD, a1];
            let mut _i_9: usize = 0 as usize;
            while _i_9
                < ::core::mem::size_of::<[u64; 3]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_9 as i32, _v_9[_i_9]);
                _i_9 = _i_9.wrapping_add(1);
            }
            return 1 as i32;
        }
        89 => {
            *nr = __NR_readlinkat as i64;
            let mut _v_10: [u64; 4] = [CWD, a0, a1, a2];
            let mut _i_10: usize = 0 as usize;
            while _i_10
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_10 as i32, _v_10[_i_10]);
                _i_10 = _i_10.wrapping_add(1);
            }
            return 1 as i32;
        }
        90 => {
            *nr = __NR_fchmodat as i64;
            let mut _v_11: [u64; 3] = [CWD, a0, a1];
            let mut _i_11: usize = 0 as usize;
            while _i_11
                < ::core::mem::size_of::<[u64; 3]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_11 as i32, _v_11[_i_11]);
                _i_11 = _i_11.wrapping_add(1);
            }
            return 1 as i32;
        }
        92 => {
            *nr = __NR_fchownat as i64;
            let mut _v_12: [u64; 5] = [CWD, a0, a1, a2, 0 as u64];
            let mut _i_12: usize = 0 as usize;
            while _i_12
                < ::core::mem::size_of::<[u64; 5]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_12 as i32, _v_12[_i_12]);
                _i_12 = _i_12.wrapping_add(1);
            }
            return 1 as i32;
        }
        94 => {
            *nr = __NR_fchownat as i64;
            let mut _v_13: [u64; 5] = [CWD, a0, a1, a2, 0x100 as u64];
            let mut _i_13: usize = 0 as usize;
            while _i_13
                < ::core::mem::size_of::<[u64; 5]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_13 as i32, _v_13[_i_13]);
                _i_13 = _i_13.wrapping_add(1);
            }
            return 1 as i32;
        }
        133 => {
            *nr = __NR_mknodat as i64;
            let mut _v_14: [u64; 4] = [CWD, a0, a1, a2];
            let mut _i_14: usize = 0 as usize;
            while _i_14
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_14 as i32, _v_14[_i_14]);
                _i_14 = _i_14.wrapping_add(1);
            }
            return 1 as i32;
        }
        235 => {
            rc = lg_tv_to_ts(t, r, a1, 2 as i32, &raw mut s);
            if rc != 0 {
                *res = rc as i64;
                return 2 as i32;
            }
            *nr = __NR_utimensat as i64;
            let mut _v_15: [u64; 4] = [CWD, a0, s, 0 as u64];
            let mut _i_15: usize = 0 as usize;
            while _i_15
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_15 as i32, _v_15[_i_15]);
                _i_15 = _i_15.wrapping_add(1);
            }
            return 1 as i32;
        }
        261 => {
            rc = lg_tv_to_ts(t, r, a2, 2 as i32, &raw mut s);
            if rc != 0 {
                *res = rc as i64;
                return 2 as i32;
            }
            *nr = __NR_utimensat as i64;
            let mut _v_16: [u64; 4] = [a0, a1, s, 0 as u64];
            let mut _i_16: usize = 0 as usize;
            while _i_16
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_16 as i32, _v_16[_i_16]);
                _i_16 = _i_16.wrapping_add(1);
            }
            return 1 as i32;
        }
        132 => {
            s = 0 as u64;
            if a1 != 0 {
                let mut ub: [i64; 2] = [0; 2];
                if eng_mem_read(
                    (*t).tid,
                    a1 as usize,
                    &raw mut ub as *mut i64 as *mut ::core::ffi::c_void,
                    ::core::mem::size_of::<[i64; 2]>(),
                ) != ::core::mem::size_of::<[i64; 2]>() as isize
                {
                    *res = -(EFAULT as i64);
                    return 2 as i32;
                }
                let mut ts: [lg_timespec; 2] = [
                    lg_timespec {
                        sec: ub[0usize],
                        nsec: 0 as i64,
                    },
                    lg_timespec {
                        sec: ub[1usize],
                        nsec: 0 as i64,
                    },
                ];
                s = lg_put(
                    t,
                    r,
                    &raw mut ts as *mut lg_timespec as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<[lg_timespec; 2]>(),
                );
                if s == 0 {
                    *res = -(EFAULT as i64);
                    return 2 as i32;
                }
            }
            *nr = __NR_utimensat as i64;
            let mut _v_17: [u64; 4] = [CWD, a0, s, 0 as u64];
            let mut _i_17: usize = 0 as usize;
            while _i_17
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_17 as i32, _v_17[_i_17]);
                _i_17 = _i_17.wrapping_add(1);
            }
            return 1 as i32;
        }
        33 => {
            if a0 as i32 == a1 as i32 {
                *nr = __NR_fcntl as i64;
                let mut _v_18: [u64; 2] = [a0, 1 as u64];
                let mut _i_18: usize = 0 as usize;
                while _i_18
                    < ::core::mem::size_of::<[u64; 2]>().wrapping_div(::core::mem::size_of::<u64>())
                {
                    eng_set_arg(r, _i_18 as i32, _v_18[_i_18]);
                    _i_18 = _i_18.wrapping_add(1);
                }
                (*t).lg_fix = LG_DUP2_SAME as i32;
                (*t).lg_a = a0;
                return 1 as i32;
            }
            *nr = __NR_dup3 as i64;
            let mut _v_19: [u64; 3] = [a0, a1, 0 as u64];
            let mut _i_19: usize = 0 as usize;
            while _i_19
                < ::core::mem::size_of::<[u64; 3]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_19 as i32, _v_19[_i_19]);
                _i_19 = _i_19.wrapping_add(1);
            }
            return 1 as i32;
        }
        22 => {
            *nr = __NR_pipe2 as i64;
            let mut _v_20: [u64; 2] = [a0, 0 as u64];
            let mut _i_20: usize = 0 as usize;
            while _i_20
                < ::core::mem::size_of::<[u64; 2]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_20 as i32, _v_20[_i_20]);
                _i_20 = _i_20.wrapping_add(1);
            }
            return 1 as i32;
        }
        7 => {
            s = 0 as u64;
            if a2 as i32 >= 0 as i32 {
                let mut ts_0: lg_timespec = lg_timespec {
                    sec: (a2 as i32 / 1000 as i32) as i64,
                    nsec: (a2 as i32 % 1000 as i32) as i64 * 1000000 as i64,
                };
                s = lg_put(
                    t,
                    r,
                    &raw mut ts_0 as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<lg_timespec>(),
                );
                if s == 0 {
                    *res = -(EFAULT as i64);
                    return 2 as i32;
                }
            }
            *nr = __NR_ppoll as i64;
            let mut _v_21: [u64; 5] = [a0, a1, s, 0 as u64, 8 as u64];
            let mut _i_21: usize = 0 as usize;
            while _i_21
                < ::core::mem::size_of::<[u64; 5]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_21 as i32, _v_21[_i_21]);
                _i_21 = _i_21.wrapping_add(1);
            }
            return 1 as i32;
        }
        23 => {
            rc = lg_tv_to_ts(t, r, a4, 1 as i32, &raw mut s);
            if rc != 0 {
                *res = rc as i64;
                return 2 as i32;
            }
            *nr = __NR_pselect6 as i64;
            let mut _v_22: [u64; 6] = [a0, a1, a2, a3, s, 0 as u64];
            let mut _i_22: usize = 0 as usize;
            while _i_22
                < ::core::mem::size_of::<[u64; 6]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_22 as i32, _v_22[_i_22]);
                _i_22 = _i_22.wrapping_add(1);
            }
            if s != 0 {
                (*t).lg_fix = LG_SELECT as i32;
                (*t).lg_a = s;
                (*t).lg_b = a4;
            }
            return 1 as i32;
        }
        78 => {
            *nr = __NR_getdents64 as i64;
            let mut _v_23: [u64; 3] = [a0, a1, a2];
            let mut _i_23: usize = 0 as usize;
            while _i_23
                < ::core::mem::size_of::<[u64; 3]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_23 as i32, _v_23[_i_23]);
                _i_23 = _i_23.wrapping_add(1);
            }
            (*t).lg_fix = LG_GETDENTS as i32;
            (*t).lg_a = a1;
            return 1 as i32;
        }
        111 => {
            *nr = __NR_getpgid as i64;
            let mut _v_24: [u64; 1] = [0 as u64];
            let mut _i_24: usize = 0 as usize;
            while _i_24
                < ::core::mem::size_of::<[u64; 1]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_24 as i32, _v_24[_i_24]);
                _i_24 = _i_24.wrapping_add(1);
            }
            return 1 as i32;
        }
        213 => {
            if a0 as i32 <= 0 as i32 {
                *res = -(EINVAL as i64);
                return 2 as i32;
            }
            *nr = __NR_epoll_create1 as i64;
            let mut _v_25: [u64; 1] = [0 as u64];
            let mut _i_25: usize = 0 as usize;
            while _i_25
                < ::core::mem::size_of::<[u64; 1]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_25 as i32, _v_25[_i_25]);
                _i_25 = _i_25.wrapping_add(1);
            }
            return 1 as i32;
        }
        232 => {
            *nr = __NR_epoll_pwait as i64;
            let mut _v_26: [u64; 6] = [a0, a1, a2, a3, 0 as u64, 8 as u64];
            let mut _i_26: usize = 0 as usize;
            while _i_26
                < ::core::mem::size_of::<[u64; 6]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_26 as i32, _v_26[_i_26]);
                _i_26 = _i_26.wrapping_add(1);
            }
            return 1 as i32;
        }
        253 => {
            *nr = __NR_inotify_init1 as i64;
            let mut _v_27: [u64; 1] = [0 as u64];
            let mut _i_27: usize = 0 as usize;
            while _i_27
                < ::core::mem::size_of::<[u64; 1]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_27 as i32, _v_27[_i_27]);
                _i_27 = _i_27.wrapping_add(1);
            }
            return 1 as i32;
        }
        284 => {
            *nr = __NR_eventfd2 as i64;
            let mut _v_28: [u64; 2] = [a0, 0 as u64];
            let mut _i_28: usize = 0 as usize;
            while _i_28
                < ::core::mem::size_of::<[u64; 2]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_28 as i32, _v_28[_i_28]);
                _i_28 = _i_28.wrapping_add(1);
            }
            return 1 as i32;
        }
        282 => {
            *nr = __NR_signalfd4 as i64;
            let mut _v_29: [u64; 4] = [a0, a1, a2, 0 as u64];
            let mut _i_29: usize = 0 as usize;
            while _i_29
                < ::core::mem::size_of::<[u64; 4]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_29 as i32, _v_29[_i_29]);
                _i_29 = _i_29.wrapping_add(1);
            }
            return 1 as i32;
        }
        37 => {
            let mut it: [i64; 8] = [
                0 as i64,
                0 as i64,
                a0 as u32 as i64,
                0 as i64,
                0 as i64,
                0 as i64,
                0 as i64,
                0 as i64,
            ];
            s = lg_put(
                t,
                r,
                &raw mut it as *mut i64 as *const ::core::ffi::c_void,
                ::core::mem::size_of::<[i64; 8]>(),
            );
            if s == 0 {
                *res = -(EFAULT as i64);
                return 2 as i32;
            }
            *nr = __NR_setitimer as i64;
            let mut _v_30: [u64; 3] = [0 as u64, s, s.wrapping_add(32 as u64)];
            let mut _i_30: usize = 0 as usize;
            while _i_30
                < ::core::mem::size_of::<[u64; 3]>().wrapping_div(::core::mem::size_of::<u64>())
            {
                eng_set_arg(r, _i_30 as i32, _v_30[_i_30]);
                _i_30 = _i_30.wrapping_add(1);
            }
            (*t).lg_fix = LG_ALARM as i32;
            (*t).lg_a = s.wrapping_add(32 as u64);
            return 1 as i32;
        }
        201 => {
            let mut now: i64 = time(::core::ptr::null_mut::<i64>()) as i64;
            if a0 != 0
                && eng_mem_write(
                    (*t).tid,
                    a0 as usize,
                    &raw mut now as *const ::core::ffi::c_void,
                    8 as usize,
                ) != 8 as isize
            {
                *res = -(EFAULT as i64);
                return 2 as i32;
            }
            *res = now as i64;
            return 2 as i32;
        }
        _ => return 0 as i32,
    };
}
#[cfg(target_arch = "x86_64")]
unsafe extern "C" fn legacy_exit(mut t: *mut eng_task, mut ret: i64) -> i64 {
    let mut fix: i32 = (*t).lg_fix;
    (*t).lg_fix = LG_NONE as i32;
    match fix {
        1 => {
            return if ret < 0 as i64 {
                ret
            } else {
                (*t).lg_a as i32 as i64
            };
        }
        2 => {
            let mut ts: lg_timespec = lg_timespec { sec: 0, nsec: 0 };
            if eng_mem_read(
                (*t).tid,
                (*t).lg_a as usize,
                &raw mut ts as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<lg_timespec>(),
            ) == ::core::mem::size_of::<lg_timespec>() as isize
            {
                let mut tv: lg_timeval = lg_timeval {
                    sec: ts.sec,
                    usec: ts.nsec / 1000 as i64,
                };
                eng_mem_write(
                    (*t).tid,
                    (*t).lg_b as usize,
                    &raw mut tv as *const ::core::ffi::c_void,
                    ::core::mem::size_of::<lg_timeval>(),
                );
            }
            return ret;
        }
        3 => {
            if ret <= 0 as i64 {
                return ret;
            }
            let mut b: *mut u8 = malloc(ret as usize) as *mut u8;
            if b.is_null() {
                return ret;
            }
            if eng_mem_read(
                (*t).tid,
                (*t).lg_a as usize,
                b as *mut ::core::ffi::c_void,
                ret as usize,
            ) == ret as isize
            {
                let mut i: i64 = 0 as i64;
                while (i + 19 as i64) < ret {
                    let mut rl: u16 = 0;
                    memcpy(
                        &raw mut rl as *mut ::core::ffi::c_void,
                        b.offset(i as isize).offset(16 as i32 as isize)
                            as *const ::core::ffi::c_void,
                        2 as usize,
                    );
                    if (rl as i32) < 20 as i32 || i + rl as i64 > ret {
                        break;
                    }
                    let mut r#type: u8 = *b.offset((i + 18 as i64) as isize);
                    let mut nl: usize = strnlen(
                        (b as *mut ::core::ffi::c_char)
                            .offset(i as isize)
                            .offset(19 as i32 as isize),
                        (rl as usize).wrapping_sub(19 as usize),
                    );
                    memmove(
                        b.offset(i as isize).offset(18 as i32 as isize) as *mut ::core::ffi::c_void,
                        b.offset(i as isize).offset(19 as i32 as isize)
                            as *const ::core::ffi::c_void,
                        nl.wrapping_add(1 as usize),
                    );
                    *b.offset((i + rl as i64 - 1 as i64) as isize) = r#type;
                    i += rl as i64;
                }
                eng_mem_write(
                    (*t).tid,
                    (*t).lg_a as usize,
                    b as *const ::core::ffi::c_void,
                    ret as usize,
                );
            }
            free(b as *mut ::core::ffi::c_void);
            return ret;
        }
        4 => {
            let mut old: [i64; 4] = [0; 4];
            if ret < 0 as i64
                || eng_mem_read(
                    (*t).tid,
                    (*t).lg_a as usize,
                    &raw mut old as *mut i64 as *mut ::core::ffi::c_void,
                    ::core::mem::size_of::<[i64; 4]>(),
                ) != ::core::mem::size_of::<[i64; 4]>() as isize
            {
                return ret;
            }
            let mut secs: i64 = old[2usize] as i64;
            if old[3usize] >= 500000 as i64 {
                secs += 1;
            }
            if secs == 0 && old[3usize] != 0 {
                secs = 1 as i64;
            }
            return secs;
        }
        _ => return ret,
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sys_entry(mut t: *mut eng_task, mut r: *mut eng_regs) -> i32 {
    if (*t).sysno == -1 as i64 {
        return 0 as i32;
    }
    #[cfg(target_arch = "x86_64")]
    if (*t).lg_restart != 0 {
        return dispatch(t, r);
    }
    #[cfg(target_arch = "x86_64")]
    static mut entry_rewrite: i32 = -1 as i32;
    #[cfg(target_arch = "x86_64")]
    if entry_rewrite < 0 as i32 {
        let mut e: *const ::core::ffi::c_char =
            getenv(b"WORKFLOW_ENGINE_TEST_LEGACY\0".as_ptr() as *const ::core::ffi::c_char);
        entry_rewrite = !(!e.is_null()
            && strcmp(e, b"sigsys\0".as_ptr() as *const ::core::ffi::c_char) == 0)
            as i32;
    }
    #[cfg(target_arch = "x86_64")]
    let mut nr: i64 = (*t).sysno;
    #[cfg(target_arch = "x86_64")]
    let mut res: i64 = 0 as i64;
    #[cfg(target_arch = "x86_64")]
    if entry_rewrite == 0 {
        static mut api28_trapped: [i64; 88] = [
            __NR_access as i64,
            __NR_open as i64,
            __NR_stat as i64,
            __NR_lstat as i64,
            __NR_creat as i64,
            __NR_mkdir as i64,
            __NR_rmdir as i64,
            __NR_unlink as i64,
            __NR_rename as i64,
            __NR_link as i64,
            __NR_symlink as i64,
            __NR_readlink as i64,
            __NR_chmod as i64,
            __NR_chown as i64,
            __NR_lchown as i64,
            __NR_mknod as i64,
            __NR_utimes as i64,
            __NR_futimesat as i64,
            __NR_dup2 as i64,
            __NR_pipe as i64,
            __NR_poll as i64,
            __NR_select as i64,
            __NR_getdents as i64,
            __NR_getpgrp as i64,
            __NR_epoll_create as i64,
            __NR_epoll_wait as i64,
            __NR_inotify_init as i64,
            __NR_eventfd as i64,
            __NR_signalfd as i64,
            __NR_alarm as i64,
            __NR_time as i64,
            __NR_uselib as i64,
            __NR_set_robust_list as i64,
            __NR_rseq as i64,
            __NR_statx as i64,
            __NR_faccessat2 as i64,
            __NR_clone3 as i64,
            __NR_openat2 as i64,
            __NR_close_range as i64,
            __NR_pidfd_open as i64,
            __NR_pidfd_getfd as i64,
            __NR_pidfd_send_signal as i64,
            __NR_epoll_pwait2 as i64,
            __NR_process_madvise as i64,
            __NR_mount_setattr as i64,
            __NR_landlock_create_ruleset as i64,
            __NR_memfd_secret as i64,
            __NR_futex_waitv as i64,
            __NR_cachestat as i64,
            __NR_fchmodat2 as i64,
            __NR_map_shadow_stack as i64,
            __NR_pkey_alloc as i64,
            __NR_pkey_mprotect as i64,
            __NR_io_uring_setup as i64,
            __NR_io_pgetevents as i64,
            __NR_membarrier as i64,
            __NR_userfaultfd as i64,
            __NR_kcmp as i64,
            __NR_name_to_handle_at as i64,
            __NR_open_by_handle_at as i64,
            __NR_bpf as i64,
            __NR_kexec_file_load as i64,
            __NR_add_key as i64,
            __NR_keyctl as i64,
            __NR_get_mempolicy as i64,
            __NR_set_mempolicy as i64,
            __NR_mbind as i64,
            __NR_migrate_pages as i64,
            __NR_move_pages as i64,
            __NR_fanotify_init as i64,
            __NR_clock_adjtime as i64,
            __NR_adjtimex as i64,
            __NR_acct as i64,
            __NR_swapon as i64,
            __NR_chroot as i64,
            __NR_mount as i64,
            __NR_umount2 as i64,
            __NR_open_tree as i64,
            __NR_fsopen as i64,
            __NR_setuid as i64,
            __NR_setgid as i64,
            __NR_setreuid as i64,
            __NR_setregid as i64,
            __NR_setresuid as i64,
            __NR_setresgid as i64,
            __NR_setfsuid as i64,
            __NR_setfsgid as i64,
            __NR_setgroups as i64,
        ];
        let mut i: usize = 0 as usize;
        while i < ::core::mem::size_of::<[i64; 88]>().wrapping_div(::core::mem::size_of::<i64>()) {
            if api28_trapped[i] == nr {
                return 0 as i32;
            }
            i = i.wrapping_add(1);
        }
    }
    #[cfg(target_arch = "x86_64")]
    let mut lg: i32 = legacy_convert(t, r, &raw mut nr, &raw mut res);
    #[cfg(target_arch = "x86_64")]
    if lg == 2 as i32 {
        return eng_task_void(t, r, res);
    }
    #[cfg(target_arch = "x86_64")]
    if lg == 1 as i32 {
        #[cfg(target_os = "linux")]
        {
            (*r).orig_rax = nr as u64 as u64;
        }
        #[cfg(all(target_os = "android", target_arch = "x86_64"))]
        {
            (*r).orig_rax = nr as u64;
        }
        (*t).sysno = nr;
        (*t).regs_modified = 1 as i32;
        dispatch(t, r);
        return 1 as i32;
    }
    return dispatch(t, r);
}
unsafe extern "C" fn dispatch(mut t: *mut eng_task, mut r: *mut eng_regs) -> i32 {
    let mut nr: i64 = (*t).sysno;
    let mut h: i32 = eng_ident_entry(t, r);
    if h >= 0 as i32 {
        return h;
    }
    let mut i: usize = 0 as usize;
    #[cfg(target_arch = "x86_64")]
    while i < ::core::mem::size_of::<[pathsys; 9]>().wrapping_div(::core::mem::size_of::<pathsys>())
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
    #[cfg(target_arch = "aarch64")]
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
        #[cfg(target_arch = "x86_64")]
        2 => {
            return h_open(t, r, -1 as i32, 0 as i32, 1 as i32, 2 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_openat => {
            return h_open(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        85 => {
            return h_open(t, r, -1 as i32, 0 as i32, -1 as i32, 1 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_newfstatat => {
            return h_stat(
                t, r, 0 as i32, 1 as i32, 3 as i32, 2 as i32, 0 as i32, 0 as i32,
            );
        }
        #[cfg(target_arch = "x86_64")]
        257 => {
            return h_open(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_statx => {
            return h_stat(
                t, r, 0 as i32, 1 as i32, 2 as i32, 4 as i32, 0 as i32, 1 as i32,
            );
        }
        #[cfg(target_arch = "x86_64")]
        4 => {
            return h_stat(
                t, r, -1 as i32, 0 as i32, -1 as i32, 1 as i32, 0 as i32, 0 as i32,
            );
        }
        #[cfg(target_arch = "x86_64")]
        6 => {
            return h_stat(
                t, r, -1 as i32, 0 as i32, -1 as i32, 1 as i32, 1 as i32, 0 as i32,
            );
        }
        #[cfg(target_arch = "x86_64")]
        262 => {
            return h_stat(
                t, r, 0 as i32, 1 as i32, 3 as i32, 2 as i32, 0 as i32, 0 as i32,
            );
        }
        #[cfg(target_arch = "x86_64")]
        332 => {
            return h_stat(
                t, r, 0 as i32, 1 as i32, 2 as i32, 4 as i32, 0 as i32, 1 as i32,
            );
        }
        __NR_fstat => return h_fstat(t, r),
        #[cfg(target_arch = "x86_64")]
        21 => {
            return h_access(t, r, -1 as i32, 0 as i32, 1 as i32, -1 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_faccessat => {
            return h_access(t, r, 0 as i32, 1 as i32, 2 as i32, -1 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        269 => {
            return h_access(t, r, 0 as i32, 1 as i32, 2 as i32, -1 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_faccessat2 => {
            return h_access(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        439 => {
            return h_access(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_mkdirat => {
            return h_mkdir(t, r, 0 as i32, 1 as i32, 2 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        83 => {
            return h_mkdir(t, r, -1 as i32, 0 as i32, 1 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_mknodat => {
            return h_mknod(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        258 => {
            return h_mkdir(t, r, 0 as i32, 1 as i32, 2 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_symlinkat => {
            return h_symlink(t, r, 1 as i32, 2 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        133 => {
            return h_mknod(t, r, -1 as i32, 0 as i32, 1 as i32, 2 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_unlinkat => {
            return h_unlink(
                t,
                r,
                0 as i32,
                1 as i32,
                (eng_arg(r, 2 as i32) as i64 & AT_REMOVEDIR as i64 != 0 as i64) as i32,
            );
        }
        #[cfg(target_arch = "x86_64")]
        259 => {
            return h_mknod(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_renameat => {
            return h_rename(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32, -1 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        88 => return h_symlink(t, r, -1 as i32, 1 as i32),
        #[cfg(target_arch = "x86_64")]
        266 => return h_symlink(t, r, 1 as i32, 2 as i32),
        #[cfg(target_arch = "x86_64")]
        87 => {
            return h_unlink(t, r, -1 as i32, 0 as i32, 0 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        263 => {
            return h_unlink(
                t,
                r,
                0 as i32,
                1 as i32,
                (eng_arg(r, 2 as i32) as i64 & AT_REMOVEDIR as i64 != 0 as i64) as i32,
            );
        }
        #[cfg(target_arch = "x86_64")]
        82 => {
            return h_rename(t, r, -1 as i32, 0 as i32, -1 as i32, 1 as i32, -1 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        264 => {
            return h_rename(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32, -1 as i32);
        }
        __NR_renameat2 => {
            return h_rename(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32, 4 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        86 => {
            return h_link(t, r, -1 as i32, 0 as i32, -1 as i32, 1 as i32, -1 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_linkat => {
            return h_link(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32, 4 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        265 => {
            return h_link(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32, 4 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        90 => {
            return h_chmod(t, r, -1 as i32, -1 as i32, 0 as i32, 1 as i32, -1 as i32);
        }
        __NR_fchmod => {
            return h_chmod(t, r, 0 as i32, -1 as i32, -1 as i32, 1 as i32, -1 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        268 => {
            return h_chmod(t, r, -1 as i32, 0 as i32, 1 as i32, 2 as i32, -1 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_fchmodat => {
            return h_chmod(t, r, -1 as i32, 0 as i32, 1 as i32, 2 as i32, -1 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        452 => {
            return h_chmod(t, r, -1 as i32, 0 as i32, 1 as i32, 2 as i32, 3 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_fchmodat2 => {
            return h_chmod(t, r, -1 as i32, 0 as i32, 1 as i32, 2 as i32, 3 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        92 => {
            return h_chown(
                t, r, -1 as i32, -1 as i32, 0 as i32, 1 as i32, 2 as i32, -1 as i32, 0 as i32,
            );
        }
        #[cfg(target_arch = "x86_64")]
        94 => {
            return h_chown(
                t, r, -1 as i32, -1 as i32, 0 as i32, 1 as i32, 2 as i32, -1 as i32, 1 as i32,
            );
        }
        __NR_fchown => {
            return h_chown(
                t, r, 0 as i32, -1 as i32, -1 as i32, 1 as i32, 2 as i32, -1 as i32, 0 as i32,
            );
        }
        #[cfg(target_arch = "x86_64")]
        260 => {
            return h_chown(
                t, r, -1 as i32, 0 as i32, 1 as i32, 2 as i32, 3 as i32, 4 as i32, 0 as i32,
            );
        }
        #[cfg(target_arch = "aarch64")]
        __NR_fchownat => {
            return h_chown(
                t, r, -1 as i32, 0 as i32, 1 as i32, 2 as i32, 3 as i32, 4 as i32, 0 as i32,
            );
        }
        #[cfg(target_arch = "x86_64")]
        89 => {
            return h_readlink(t, r, -1 as i32, 0 as i32, 1 as i32, 2 as i32);
        }
        #[cfg(target_arch = "aarch64")]
        __NR_readlinkat => {
            return h_readlink(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        267 => {
            return h_readlink(t, r, 0 as i32, 1 as i32, 2 as i32, 3 as i32);
        }
        __NR_getcwd => return h_getcwd(t, r),
        #[cfg(target_arch = "x86_64")]
        59 => return h_exec(t, r, 0 as i32),
        #[cfg(target_arch = "aarch64")]
        221 => return h_exec(t, r, 0 as i32),
        __NR_execveat => return h_exec(t, r, 1 as i32),
        __NR_socket => {
            if eng_arg(r, 0 as i32) as i64 == 16 as i64 && eng_arg(r, 2 as i32) as i64 == 9 as i64 {
                return eng_task_void(t, r, -(EPROTONOSUPPORT as i64));
            }
            return 0 as i32;
        }
        __NR_bind => {
            return h_sockaddr(t, r, 1 as i32, 2 as i32, 1 as i32);
        }
        __NR_connect => {
            return h_sockaddr(t, r, 1 as i32, 2 as i32, 0 as i32);
        }
        __NR_sendto => {
            return h_sockaddr(t, r, 4 as i32, 5 as i32, 0 as i32);
        }
        __NR_sendmsg => return h_sendmsg(t, r),
        __NR_getsockname => {
            return h_sockname(t, r, 1 as i32, 2 as i32);
        }
        __NR_getpeername => {
            return h_sockname(t, r, 1 as i32, 2 as i32);
        }
        __NR_accept => {
            return h_sockname(t, r, 1 as i32, 2 as i32);
        }
        __NR_accept4 => {
            return h_sockname(t, r, 1 as i32, 2 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        217 => {
            (*t).fix_aux = eng_arg(r, 0 as i32) as i64;
            (*t).fix_addr = eng_arg(r, 1 as i32);
            (*t).fixup = ENG_FIX_GETDENTS as i32;
            return 0 as i32;
        }
        #[cfg(target_arch = "aarch64")]
        __NR_getdents64 => {
            (*t).fix_aux = eng_arg(r, 0 as i32) as i64;
            (*t).fix_addr = eng_arg(r, 1 as i32);
            (*t).fixup = ENG_FIX_GETDENTS as i32;
            return 0 as i32;
        }
        __NR_getxattr => {
            return h_xattr(t, r, XA_GET as i32, 0 as i32, 0 as i32);
        }
        __NR_lgetxattr => {
            return h_xattr(t, r, XA_GET as i32, 0 as i32, 1 as i32);
        }
        __NR_fgetxattr => {
            return h_xattr(t, r, XA_GET as i32, -1 as i32, 0 as i32);
        }
        __NR_setxattr => {
            return h_xattr(t, r, XA_SET as i32, 0 as i32, 0 as i32);
        }
        __NR_lsetxattr => {
            return h_xattr(t, r, XA_SET as i32, 0 as i32, 1 as i32);
        }
        __NR_fsetxattr => {
            return h_xattr(t, r, XA_SET as i32, -1 as i32, 0 as i32);
        }
        __NR_removexattr => {
            return h_xattr(t, r, XA_REMOVE as i32, 0 as i32, 0 as i32);
        }
        __NR_lremovexattr => {
            return h_xattr(t, r, XA_REMOVE as i32, 0 as i32, 1 as i32);
        }
        __NR_fremovexattr => {
            return h_xattr(t, r, XA_REMOVE as i32, -1 as i32, 0 as i32);
        }
        __NR_listxattr => {
            return h_xattr(t, r, XA_LIST as i32, 0 as i32, 0 as i32);
        }
        __NR_llistxattr => {
            return h_xattr(t, r, XA_LIST as i32, 0 as i32, 1 as i32);
        }
        __NR_flistxattr => {
            return h_xattr(t, r, XA_LIST as i32, -1 as i32, 0 as i32);
        }
        #[cfg(target_arch = "x86_64")]
        425 | __NR_io_uring_enter | __NR_io_uring_register | 437 | 435 | 134 => {
            return eng_task_void(t, r, -(ENOSYS as i64));
        }
        #[cfg(target_arch = "aarch64")]
        __NR_io_uring_setup
        | __NR_io_uring_enter
        | __NR_io_uring_register
        | __NR_openat2
        | __NR_clone3 => return eng_task_void(t, r, -(ENOSYS as i64)),
        #[cfg(target_arch = "x86_64")]
        303 => return eng_task_void(t, r, -(EOPNOTSUPP as i64)),
        #[cfg(target_arch = "aarch64")]
        __NR_name_to_handle_at => {
            return eng_task_void(t, r, -(EOPNOTSUPP as i64));
        }
        #[cfg(target_arch = "x86_64")]
        304 | 165 | 166 | __NR_pivot_root | 161 | 167 | __NR_swapoff | 163 | __NR_ptrace | 300
        | __NR_fanotify_mark | 428 | __NR_move_mount | 430 | __NR_fsconfig | __NR_fsmount
        | __NR_fspick => return eng_task_void(t, r, -(EPERM as i64)),
        #[cfg(target_arch = "aarch64")]
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
        | __NR_fspick => return eng_task_void(t, r, -(EPERM as i64)),
        _ => return default_policy(t, r, nr),
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sys_exit(mut t: *mut eng_task, mut r: *mut eng_regs) -> i32 {
    if eng_ident_exit(t, r) != 0 {
        return 1 as i32;
    }
    let mut ret: i64 = eng_ret(r) as i64;
    let mut fix: i32 = (*t).fixup;
    (*t).fixup = ENG_FIX_NONE as i32;
    if (*t).void_pending != 0 {
        (*t).lg_fix = 0 as i32;
        return 0 as i32;
    }
    #[cfg(target_arch = "x86_64")]
    if (*t).lg_fix != 0 {
        let mut rc: i32 = sys_exit_fix(t, fix, ret);
        let mut cur: i64 = if (*t).void_pending != 0 {
            (*t).inject_result
        } else {
            ret
        };
        let mut nv: i64 = legacy_exit(t, cur);
        if nv != cur {
            (*t).inject_result = nv;
            (*t).void_pending = 1 as i32;
        }
        return rc;
    }
    return sys_exit_fix(t, fix, ret);
}
unsafe extern "C" fn sys_exit_fix(mut t: *mut eng_task, mut fix: i32, mut ret: i64) -> i32 {
    let mut g: *mut eng_guest = eng_tracer_guest((*t).tr);
    match fix {
        1 => {
            if ret == 0 as i64 {
                x_stat(t);
            }
        }
        2 => {
            if ret == 0 as i64 {
                x_statx(t);
            }
        }
        3 => {
            if ret >= 0 as i64 {
                let mut p: [::core::ffi::c_char; 64] = [0; 64];
                fd_proc_path(
                    t,
                    ret as i32,
                    &raw mut p as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                );
                let mut m: eng_meta = eng_meta {
                    uid: (*t).cr.fsuid,
                    gid: (*t).fix_aux as u32,
                    mode: (*t).fix_mode,
                    nlink: 0,
                    major: 0,
                    minor: 0,
                    present: 1 as i32,
                };
                eng_meta_write(&raw mut p as *mut ::core::ffi::c_char, 0 as i32, &raw mut m);
            }
        }
        4 => {
            if ret == 0 as i64 {
                let mut m_0: eng_meta = eng_meta {
                    uid: (*t).cr.fsuid,
                    gid: (*t).fix_aux as u32,
                    mode: (*t).fix_mode,
                    nlink: 0,
                    major: 0,
                    minor: 0,
                    present: 1 as i32,
                };
                eng_meta_write(
                    &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
                    1 as i32,
                    &raw mut m_0,
                );
            }
        }
        5 => {
            if ret == 0 as i64 {
                eng_link_drop(g, &raw mut (*t).fix_id as *mut ::core::ffi::c_char);
            }
        }
        6 => {
            if ret == 0 as i64 {
                let mut st: stat = platform_empty_stat();
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
                ) == 0 as i32
                    && !(st.st_mode & S_IFMT as u32 == S_IFLNK as u32)
                    && eng_meta_read(
                        g,
                        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
                        1 as i32,
                        &raw mut st,
                        &raw mut m_1,
                    ) == 0 as i32
                    && m_1.present == 0
                {
                    m_1.uid = (*t).fix_mode;
                    m_1.gid = (*t).fix_aux as u32;
                    m_1.present = 1 as i32;
                    eng_meta_write(
                        &raw mut (*t).fix_path as *mut ::core::ffi::c_char,
                        1 as i32,
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
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sys_sigsys(
    mut t: *mut eng_task,
    mut r: *mut eng_regs,
    mut si: *mut siginfo_t,
) -> i32 {
    if platform_expr_sigsys_code(si) != 1 as i32 {
        return 0 as i32;
    }
    #[cfg(target_os = "linux")]
    if platform_expr_sigsys_syscall(si) == (*r).rax as i32
        && (*t).phase.0 == eng_phase::ENG_PH_GUEST.0
    {
        let mut saved: eng_regs = *r;
        let mut nr: i64 = platform_expr_sigsys_syscall(si) as i64;
        let mut res: i64 = 0 as i64;
        (*t).slot_off = 0 as u32;
        (*t).stack_scratch = 0 as u64;
        let mut lg: i32 = legacy_convert(t, r, &raw mut nr, &raw mut res);
        if lg == 2 as i32 {
            eng_set_ret(r, res as u64);
            return 1 as i32;
        }
        if lg == 1 as i32 {
            (*t).lg_saved = saved;
            (*t).lg_saved.orig_rax = platform_expr_sigsys_syscall(si) as u64 as u64;
            (*t).lg_restart = 1 as i32;
            (*t).lg_nr = nr;
            (*r).rax = nr as u64 as u64;
            eng_set_pc(r, eng_pc(r).wrapping_sub(eng_syscall_insn_len() as u64));
            eng_logf!(
                eng_log_level::ENG_LOG_DEBUG,
                b"SIGSYS tid=%d syscall=%d(%s) -> re-issued as %ld\0".as_ptr()
                    as *const ::core::ffi::c_char,
                (*t).tid,
                platform_expr_sigsys_syscall(si),
                if !eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64,).is_null() {
                    eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64)
                } else {
                    b"?\0".as_ptr() as *const ::core::ffi::c_char
                },
                nr,
            );
            return 1 as i32;
        }
    }
    #[cfg(all(target_os = "android", target_arch = "x86_64"))]
    if platform_expr_sigsys_syscall(si) == (*r).rax as i32
        && (*t).phase.0 == eng_phase::ENG_PH_GUEST.0
    {
        let mut saved: eng_regs = *r;
        let mut nr: i64 = platform_expr_sigsys_syscall(si) as i64;
        let mut res: i64 = 0 as i64;
        (*t).slot_off = 0 as u32;
        (*t).stack_scratch = 0 as u64;
        let mut lg: i32 = legacy_convert(t, r, &raw mut nr, &raw mut res);
        if lg == 2 as i32 {
            eng_set_ret(r, res as u64);
            return 1 as i32;
        }
        if lg == 1 as i32 {
            (*t).lg_saved = saved;
            (*t).lg_saved.orig_rax = platform_expr_sigsys_syscall(si) as u64;
            (*t).lg_restart = 1 as i32;
            (*t).lg_nr = nr;
            (*r).rax = nr as u64;
            eng_set_pc(r, eng_pc(r).wrapping_sub(eng_syscall_insn_len() as u64));
            eng_logf!(
                eng_log_level::ENG_LOG_DEBUG,
                b"SIGSYS tid=%d syscall=%d(%s) -> re-issued as %ld\0".as_ptr()
                    as *const ::core::ffi::c_char,
                (*t).tid,
                platform_expr_sigsys_syscall(si),
                if !eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64,).is_null() {
                    eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64)
                } else {
                    b"?\0".as_ptr() as *const ::core::ffi::c_char
                },
                nr,
            );
            return 1 as i32;
        }
    }
    #[cfg(target_arch = "aarch64")]
    if (*t).phase.0 == eng_phase::ENG_PH_GUEST.0 && !eng_tracer_guest((*t).tr).is_null() {
        let mut saved: eng_regs = *r;
        let mut save: i64 = (*t).sysno;
        (*t).sysno = platform_expr_sigsys_syscall(si) as i64;
        (*t).void_pending = 0 as i32;
        (*t).regs_modified = 0 as i32;
        (*t).fixup = ENG_FIX_NONE as i32;
        (*t).slot_off = 0 as u32;
        (*t).stack_scratch = 0 as u64;
        dispatch(t, r);
        let mut emulated: i32 = (*t).void_pending;
        let mut res: i64 = (*t).inject_result;
        (*t).void_pending = 0 as i32;
        (*t).regs_modified = 0 as i32;
        (*t).fixup = ENG_FIX_NONE as i32;
        (*t).umask_old = -1 as i32;
        (*t).sysno = save;
        *r = saved;
        if emulated != 0 {
            eng_set_ret(r, res as u64);
            eng_logf!(
                eng_log_level::ENG_LOG_DEBUG,
                b"SIGSYS tid=%d syscall=%d(%s) -> emulated (%ld)\0".as_ptr()
                    as *const ::core::ffi::c_char,
                (*t).tid,
                platform_expr_sigsys_syscall(si),
                if !eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64,).is_null() {
                    eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64)
                } else {
                    b"?\0".as_ptr() as *const ::core::ffi::c_char
                },
                res,
            );
            return 1 as i32;
        }
    }
    #[cfg(target_arch = "x86_64")]
    if (*t).phase.0 == eng_phase::ENG_PH_GUEST.0 && !eng_tracer_guest((*t).tr).is_null() {
        let mut saved_0: eng_regs = *r;
        let mut save: i64 = (*t).sysno;
        (*t).sysno = platform_expr_sigsys_syscall(si) as i64;
        (*t).void_pending = 0 as i32;
        (*t).regs_modified = 0 as i32;
        (*t).fixup = ENG_FIX_NONE as i32;
        (*t).slot_off = 0 as u32;
        (*t).stack_scratch = 0 as u64;
        dispatch(t, r);
        let mut emulated: i32 = (*t).void_pending;
        let mut res_0: i64 = (*t).inject_result;
        (*t).void_pending = 0 as i32;
        (*t).regs_modified = 0 as i32;
        (*t).fixup = ENG_FIX_NONE as i32;
        (*t).umask_old = -1 as i32;
        (*t).sysno = save;
        *r = saved_0;
        if emulated != 0 {
            eng_set_ret(r, res_0 as u64);
            eng_logf!(
                eng_log_level::ENG_LOG_DEBUG,
                b"SIGSYS tid=%d syscall=%d(%s) -> emulated (%ld)\0".as_ptr()
                    as *const ::core::ffi::c_char,
                (*t).tid,
                platform_expr_sigsys_syscall(si),
                if !eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64,).is_null() {
                    eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64)
                } else {
                    b"?\0".as_ptr() as *const ::core::ffi::c_char
                },
                res_0,
            );
            return 1 as i32;
        }
    }
    eng_set_ret(r, -ENOSYS as u64);
    #[cfg(target_os = "linux")]
    eng_logf!(
        eng_log_level::ENG_LOG_DEBUG,
        b"SIGSYS tid=%d syscall=%d(%s) -> ENOSYS\0".as_ptr() as *const ::core::ffi::c_char,
        (*t).tid,
        platform_expr_sigsys_syscall(si),
        if !eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64).is_null() {
            eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64)
        } else {
            b"?\0".as_ptr() as *const ::core::ffi::c_char
        },
    );
    #[cfg(target_os = "android")]
    eng_logf!(
        eng_log_level::ENG_LOG_DEBUG,
        b"SIGSYS tid=%d syscall=%d(%s) -> ENOSYS\0".as_ptr() as *const ::core::ffi::c_char,
        (*t).tid,
        platform_expr_sigsys_syscall(si),
        if !eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64,).is_null() {
            eng_sysinv_name(platform_expr_sigsys_syscall(si) as i64)
        } else {
            b"?\0".as_ptr() as *const ::core::ffi::c_char
        },
    );
    return 1 as i32;
}
#[cfg(target_os = "linux")]
unsafe fn platform_empty_stat() -> stat {
    stat {
        st_dev: 0,
        st_ino: 0,
        st_nlink: 0,
        st_mode: 0,
        st_uid: 0,
        st_gid: 0,
        __pad0: 0,
        st_rdev: 0,
        st_size: 0,
        st_blksize: 0,
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
        __glibc_reserved: [0; 3],
    }
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
unsafe fn platform_empty_stat() -> stat {
    stat {
        st_dev: 0,
        st_ino: 0,
        st_nlink: 0,
        st_mode: 0,
        st_uid: 0,
        st_gid: 0,
        __pad0: 0,
        st_rdev: 0,
        st_size: 0,
        st_blksize: 0,
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
        __pad3: [0; 3],
    }
}
#[cfg(target_arch = "aarch64")]
unsafe fn platform_empty_stat() -> stat {
    stat {
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
    }
}
#[cfg(target_os = "linux")]
unsafe fn platform_expr_sigsys_code(si: *mut siginfo_t) -> i32 {
    (*si).si_code
}
#[cfg(target_os = "android")]
unsafe fn platform_expr_sigsys_code(si: *mut siginfo_t) -> i32 {
    (*si).c2rust_unnamed.c2rust_unnamed.si_code
}
#[cfg(target_os = "linux")]
unsafe fn platform_expr_sigsys_syscall(si: *mut siginfo_t) -> i32 {
    (*si)._sifields._sigsys._syscall
}
#[cfg(target_os = "android")]
unsafe fn platform_expr_sigsys_syscall(si: *mut siginfo_t) -> i32 {
    (*si)
        .c2rust_unnamed
        .c2rust_unnamed
        ._sifields
        ._sigsys
        ._syscall
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
pub type siginfo_t = siginfo;
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
