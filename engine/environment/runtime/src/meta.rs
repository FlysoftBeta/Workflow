//! Xattr ownership and permissions, emulated hardlinks and crash recovery.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    unsafe fn stat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
    unsafe fn fstatat(_: i32, _: *const ::core::ffi::c_char, _: *mut stat, _: i32) -> i32;
    unsafe fn lstat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
    unsafe fn mkdir(_: *const ::core::ffi::c_char, _: u32) -> i32;
    unsafe fn mkfifo(_: *const ::core::ffi::c_char, _: u32) -> i32;
    unsafe fn close(_: i32) -> i32;
    unsafe fn read(_: i32, _: *mut ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn write(_: i32, _: *const ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn _exit(_: i32) -> !;
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
    unsafe fn readlinkat(
        _: i32,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> isize;
    unsafe fn unlink(_: *const ::core::ffi::c_char) -> i32;
    unsafe fn fsync(_: i32) -> i32;
    unsafe fn eng_tracer_guest(_: *mut eng_tracer) -> *mut eng_guest;
    unsafe fn closedir(_: *mut DIR) -> i32;
    unsafe fn opendir(_: *const ::core::ffi::c_char) -> *mut DIR;
    unsafe fn fdopendir(_: i32) -> *mut DIR;
    unsafe fn readdir(_: *mut DIR) -> *mut dirent;
    unsafe fn dirfd(_: *mut DIR) -> i32;
    #[cfg(target_os = "linux")]
    #[link_name = "__errno_location"]
    unsafe fn errno() -> *mut i32;
    #[cfg(target_os = "android")]
    #[link_name = "__errno"]
    unsafe fn errno() -> *mut i32;
    unsafe fn open(_: *const ::core::ffi::c_char, _: i32, ...) -> i32;
    unsafe fn openat(_: i32, _: *const ::core::ffi::c_char, _: i32, ...) -> i32;
    static mut stdout: *mut FILE;
    #[cfg(target_os = "linux")]
    unsafe fn rename(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    #[cfg(target_os = "android")]
    unsafe fn rename(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> i32;
    unsafe fn snprintf(
        _: *mut ::core::ffi::c_char,
        _: usize,
        _: *const ::core::ffi::c_char,
        ...
    ) -> i32;
    unsafe fn sscanf(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char, ...) -> i32;
    #[cfg(target_os = "linux")]
    unsafe fn realloc(_: *mut ::core::ffi::c_void, _: usize) -> *mut ::core::ffi::c_void;
    #[cfg(target_os = "android")]
    unsafe fn realloc(_: *mut ::core::ffi::c_void, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn free(_: *mut ::core::ffi::c_void);
    unsafe fn getenv(_: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: usize,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memset(_: *mut ::core::ffi::c_void, _: i32, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn strcmp(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn strncmp(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn strchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strrchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strstr(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(_: *const ::core::ffi::c_char) -> usize;
    unsafe fn flock(_: i32, _: i32) -> i32;
    #[cfg(target_os = "linux")]
    unsafe fn gnu_dev_makedev(_: u32, _: u32) -> u64;
    unsafe fn setxattr(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_void,
        _: usize,
        _: i32,
    ) -> i32;
    unsafe fn lsetxattr(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_void,
        _: usize,
        _: i32,
    ) -> i32;
    unsafe fn getxattr(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_void,
        _: usize,
    ) -> isize;
    unsafe fn lgetxattr(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_void,
        _: usize,
    ) -> isize;
    unsafe fn clock_gettime(_: i32, _: *mut timespec) -> i32;
    unsafe fn eng_guest_locate(_: *const eng_guest, _: *const ::core::ffi::c_char) -> i32;
}
#[repr(C)]
pub struct eng_tracer {
    _opaque: [u8; 0],
}
#[cfg(target_os = "linux")]
#[repr(C)]
pub struct __dirstream {
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
pub type DIR = __dirstream;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dirent {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
    pub d_name: [::core::ffi::c_char; 256],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed {
    pub v: *mut fsck_ent,
    pub n: usize,
    pub cap: usize,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct fsck_ent {
    pub id: [::core::ffi::c_char; 24],
    pub names: u32,
    pub nlink: u32,
    pub have_obj: i32,
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
#[cfg(target_os = "linux")]
pub const DT_UNKNOWN: u32 = 0;
#[cfg(target_os = "android")]
pub const DT_UNKNOWN: i32 = 0 as i32;
#[cfg(target_os = "linux")]
pub const DT_FIFO: u32 = 1;
#[cfg(target_os = "linux")]
pub const DT_CHR: u32 = 2;
#[cfg(target_os = "linux")]
pub const DT_DIR: u32 = 4;
#[cfg(target_os = "android")]
pub const DT_DIR: i32 = 4 as i32;
#[cfg(target_os = "linux")]
pub const DT_BLK: u32 = 6;
#[cfg(target_os = "linux")]
pub const DT_REG: u32 = 8;
#[cfg(target_os = "android")]
pub const DT_REG: i32 = 8 as i32;
#[cfg(target_os = "linux")]
pub const DT_LNK: u32 = 10;
#[cfg(target_os = "android")]
pub const DT_LNK: i32 = 10 as i32;
#[cfg(target_os = "linux")]
pub const DT_SOCK: u32 = 12;
#[cfg(target_os = "linux")]
pub const DT_WHT: u32 = 14;
#[cfg(target_os = "linux")]
pub const __S_IFMT: i32 = 0o170000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFCHR: i32 = 0o20000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFBLK: i32 = 0o60000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFIFO: i32 = 0o10000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFLNK: i32 = 0o120000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFSOCK: i32 = 0o140000 as i32;
pub const R_OK: i32 = 4 as i32;
pub const W_OK: i32 = 2 as i32;
pub const X_OK: i32 = 1 as i32;
pub const ENG_META_XATTR: [::core::ffi::c_char; 19] = unsafe {
    ::core::mem::transmute::<[u8; 19], [::core::ffi::c_char; 19]>(*b"user.workflow.meta\0")
};
pub const ENG_STORE_GUEST: [::core::ffi::c_char; 18] = unsafe {
    ::core::mem::transmute::<[u8; 18], [::core::ffi::c_char; 18]>(*b"/.workflow-engine\0")
};
pub const ENG_LINK_PREFIX: [::core::ffi::c_char; 25] = unsafe {
    ::core::mem::transmute::<[u8; 25], [::core::ffi::c_char; 25]>(*b"/.workflow-engine/links/\0")
};
pub const EPERM: i32 = 1 as i32;
pub const EINTR: i32 = 4 as i32;
pub const EIO: i32 = 5 as i32;
pub const EACCES: i32 = 13 as i32;
pub const EEXIST: i32 = 17 as i32;
pub const ENAMETOOLONG: i32 = 36 as i32;
pub const O_RDONLY: i32 = 0 as i32;
pub const O_WRONLY: i32 = 0o1 as i32;
pub const O_RDWR: i32 = 0o2 as i32;
pub const O_CREAT: i32 = 0o100 as i32;
pub const O_EXCL: i32 = 0o200 as i32;
#[cfg(target_os = "linux")]
pub const __O_DIRECTORY: i32 = 0o200000 as i32;
#[cfg(target_os = "linux")]
pub const __O_NOFOLLOW: i32 = 0o400000 as i32;
#[cfg(target_os = "linux")]
pub const __O_CLOEXEC: i32 = 0o2000000 as i32;
#[cfg(target_os = "linux")]
pub const O_DIRECTORY: i32 = __O_DIRECTORY;
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
pub const O_DIRECTORY: i32 = 0o200000 as i32;
#[cfg(target_arch = "aarch64")]
pub const O_DIRECTORY: i32 = 0o40000 as i32;
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
pub const S_IFMT: i32 = __S_IFMT;
#[cfg(target_os = "android")]
pub const S_IFMT: i32 = 0o170000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFCHR: i32 = __S_IFCHR;
#[cfg(target_os = "android")]
pub const S_IFCHR: i32 = 0o20000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFBLK: i32 = __S_IFBLK;
#[cfg(target_os = "android")]
pub const S_IFBLK: i32 = 0o60000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFIFO: i32 = __S_IFIFO;
#[cfg(target_os = "android")]
pub const S_IFIFO: i32 = 0o10000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFLNK: i32 = __S_IFLNK;
#[cfg(target_os = "android")]
pub const S_IFLNK: i32 = 0o120000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFSOCK: i32 = __S_IFSOCK;
#[cfg(target_os = "android")]
pub const S_IFSOCK: i32 = 0o140000 as i32;
pub const AT_SYMLINK_NOFOLLOW: i32 = 0x100 as i32;
pub const LOCK_SH: i32 = 1 as i32;
pub const LOCK_EX: i32 = 2 as i32;
pub const LOCK_UN: i32 = 8 as i32;
pub const LOCK_NB: i32 = 4 as i32;
pub const CLOCK_REALTIME: i32 = 0 as i32;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ENG_LOC_ROOTFS: i32 = -1 as i32;
pub const ENG_CAP_DAC_OVERRIDE: i32 = 1 as i32;
pub const ENG_CAP_DAC_READ_SEARCH: i32 = 2 as i32;
pub const S_IFREG: i32 = 0o100000 as i32;
pub const S_IFDIR: i32 = 0o40000 as i32;
unsafe extern "C" fn parse_meta(mut s: *const ::core::ffi::c_char, mut m: *mut eng_meta) -> i32 {
    let mut ver: u32 = 0;
    let mut uid: u32 = 0;
    let mut gid: u32 = 0;
    let mut mode: u32 = 0;
    let mut nlink: u32 = 0;
    let mut maj: u32 = 0;
    let mut min: u32 = 0;
    if sscanf(
        s,
        b"%u %u %u %o %u %u,%u\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut ver,
        &raw mut uid,
        &raw mut gid,
        &raw mut mode,
        &raw mut nlink,
        &raw mut maj,
        &raw mut min,
    ) != 7 as i32
        || ver != 1 as u32
    {
        return -1 as i32;
    }
    (*m).uid = uid as u32;
    (*m).gid = gid as u32;
    (*m).mode = mode as u32;
    (*m).nlink = nlink as u32;
    (*m).major = maj as u32;
    (*m).minor = min as u32;
    (*m).present = 1 as i32;
    return 0 as i32;
}
unsafe extern "C" fn object_path(
    mut host: *const ::core::ffi::c_char,
    mut buf: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> *const ::core::ffi::c_char {
    if strncmp(
        host,
        b"/proc/\0".as_ptr() as *const ::core::ffi::c_char,
        6 as usize,
    ) != 0
        || strstr(host, b"/fd/\0".as_ptr() as *const ::core::ffi::c_char).is_null()
    {
        return host;
    }
    let mut n: isize = readlink(host, buf, cap.wrapping_sub(1 as usize));
    if n <= 0 as isize {
        return host;
    }
    *buf.offset(n) = 0 as ::core::ffi::c_char;
    return if *buf.offset(0isize) as i32 == '/' as i32 {
        buf as *const ::core::ffi::c_char
    } else {
        host
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_in_store(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
) -> i32 {
    if g.is_null() {
        return 1 as i32;
    }
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    return (eng_guest_locate(
        g,
        object_path(
            host,
            &raw mut buf as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        ),
    ) == ENG_LOC_ROOTFS) as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_read(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut nofollow: i32,
    mut st: *const stat,
    mut m: *mut eng_meta,
) -> i32 {
    memset(
        m as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<eng_meta>(),
    );
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut obj: *const ::core::ffi::c_char = object_path(
        host,
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    let mut loc: i32 = if !g.is_null() {
        eng_guest_locate(g, obj)
    } else {
        ENG_LOC_ROOTFS
    };
    if loc == ENG_LOC_ROOTFS && !((*st).st_mode & S_IFMT as u32 == S_IFLNK as u32) {
        let mut val: [::core::ffi::c_char; 128] = [0; 128];
        let mut n: isize = if nofollow != 0 {
            lgetxattr(
                host,
                ENG_META_XATTR.as_ptr(),
                &raw mut val as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_char; 128]>().wrapping_sub(1 as usize),
            )
        } else {
            getxattr(
                host,
                ENG_META_XATTR.as_ptr(),
                &raw mut val as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_char; 128]>().wrapping_sub(1 as usize),
            )
        };
        if n > 0 as isize {
            val[n as usize] = 0 as ::core::ffi::c_char;
            if parse_meta(&raw mut val as *mut ::core::ffi::c_char, m) == 0 as i32 {
                if eng_meta_is_placeholder(m, (*st).st_mode) == 0 {
                    (*m).mode = (*st).st_mode as u32 & S_IFMT as u32 | (*m).mode & 0o7777 as u32;
                }
                return 0 as i32;
            }
        }
    }
    if loc >= 0 as i32 {
        (*m).uid = (*g).binds[loc as usize].uid as u32;
        (*m).gid = (*g).binds[loc as usize].gid as u32;
    }
    if (*st).st_mode & S_IFMT as u32 == S_IFLNK as u32 {
        if loc == ENG_LOC_ROOTFS {
            let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
            snprintf(
                &raw mut dir as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                obj,
            );
            let mut sl: *mut ::core::ffi::c_char =
                strrchr(&raw mut dir as *mut ::core::ffi::c_char, '/' as i32);
            if !sl.is_null() && sl != &raw mut dir as *mut ::core::ffi::c_char {
                *sl = 0 as ::core::ffi::c_char;
                let mut dst: stat = platform_empty_stat();
                let mut dm: eng_meta = eng_meta {
                    uid: 0,
                    gid: 0,
                    mode: 0,
                    nlink: 0,
                    major: 0,
                    minor: 0,
                    present: 0,
                };
                if lstat(&raw mut dir as *mut ::core::ffi::c_char, &raw mut dst) == 0 as i32
                    && !(dst.st_mode & S_IFMT as u32 == S_IFLNK as u32)
                    && eng_meta_read(
                        g,
                        &raw mut dir as *mut ::core::ffi::c_char,
                        1 as i32,
                        &raw mut dst,
                        &raw mut dm,
                    ) == 0 as i32
                {
                    (*m).uid = dm.uid;
                    (*m).gid = dm.gid;
                }
            }
        }
        (*m).mode = (S_IFLNK | 0o777 as i32) as u32;
    } else {
        (*m).mode = ((*st).st_mode & (S_IFMT | 0o777 as i32) as u32) as u32;
    }
    (*m).present = 0 as i32;
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_get(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut nofollow: i32,
    mut st: *const stat,
    mut m: *mut eng_meta,
) -> i32 {
    return eng_meta_read(g, host, nofollow, st, m);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_write(
    mut host: *const ::core::ffi::c_char,
    mut nofollow: i32,
    mut m: *const eng_meta,
) -> i32 {
    let mut buf: [::core::ffi::c_char; 128] = [0; 128];
    let mut n: i32 = snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 128]>(),
        b"1 %u %u %o %u %u,%u\0".as_ptr() as *const ::core::ffi::c_char,
        (*m).uid,
        (*m).gid,
        (*m).mode,
        (*m).nlink,
        (*m).major,
        (*m).minor,
    );
    let mut rc: i32 = if nofollow != 0 {
        lsetxattr(
            host,
            ENG_META_XATTR.as_ptr(),
            &raw mut buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            n as usize,
            0 as i32,
        )
    } else {
        setxattr(
            host,
            ENG_META_XATTR.as_ptr(),
            &raw mut buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            n as usize,
            0 as i32,
        )
    };
    return if rc == 0 as i32 { 0 as i32 } else { -*errno() };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_is_placeholder(
    mut m: *const eng_meta,
    mut host_mode: u32,
) -> i32 {
    let mut vt: u32 = (*m).mode & S_IFMT as u32;
    return (host_mode & S_IFMT as u32 == S_IFREG as u32
        && (vt == S_IFCHR as u32
            || vt == S_IFBLK as u32
            || vt == S_IFIFO as u32
            || vt == S_IFSOCK as u32)) as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_fifo_path(
    mut g: *mut eng_guest,
    mut ph: *const stat,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        ENG_STORE_GUEST.as_ptr(),
    );
    mkdir(&raw mut dir as *mut ::core::ffi::c_char, 0o700 as u32);
    snprintf(
        &raw mut dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s%s/fifo\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        ENG_STORE_GUEST.as_ptr(),
    );
    mkdir(&raw mut dir as *mut ::core::ffi::c_char, 0o700 as u32);
    let mut n: i32 = snprintf(
        out,
        cap,
        b"%s/%llx-%llx\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut dir as *mut ::core::ffi::c_char,
        (*ph).st_dev as u64,
        (*ph).st_ino as u64,
    );
    if n <= 0 as i32 || n as usize >= cap {
        return -ENAMETOOLONG;
    }
    if mkfifo(out, 0o600 as u32) != 0 as i32 && *errno() != EEXIST {
        return -*errno();
    }
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_apply_stat(mut m: *const eng_meta, mut st: *mut stat) {
    (*st).st_uid = (*m).uid as u32;
    (*st).st_gid = (*m).gid as u32;
    let mut vt: u32 = (*m).mode & S_IFMT as u32;
    if eng_meta_is_placeholder(m, (*st).st_mode) != 0 {
        (*st).st_mode = (*m).mode as u32;
        #[cfg(target_os = "linux")]
        {
            (*st).st_rdev = if vt == S_IFCHR as u32 || vt == S_IFBLK as u32 {
                gnu_dev_makedev((*m).major as u32, (*m).minor as u32)
            } else {
                0 as u64
            };
        }
        #[cfg(target_os = "android")]
        {
            (*st).st_rdev = (if vt == S_IFCHR as u32 || vt == S_IFBLK as u32 {
                ((*m).major as u64 & 0xfffff000 as u64) << 32 as i32
                    | ((*m).major as u64 & 0xfff as u64) << 8 as i32
                    | ((*m).minor as u64 & 0xffffff00 as u64) << 12 as i32
                    | (*m).minor as u64 & 0xff as u64
            } else {
                0 as u64
            }) as u64;
        }
        (*st).st_size = 0 as i64;
    } else {
        (*st).st_mode = ((*st).st_mode as u32 & S_IFMT as u32 | (*m).mode & 0o7777 as u32) as u32;
    }
    if (*m).nlink != 0 {
        #[cfg(target_arch = "x86_64")]
        {
            (*st).st_nlink = (*m).nlink as u64;
        }
        #[cfg(target_arch = "aarch64")]
        {
            (*st).st_nlink = (*m).nlink as u32;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_apply_statx(
    mut m: *const eng_meta,
    mut stx: *mut ::core::ffi::c_void,
) {
    let mut b: *mut u8 = stx as *mut u8;
    let mut v: u32 = 0;
    let mut mode: u16 = 0;
    memcpy(
        &raw mut mode as *mut ::core::ffi::c_void,
        b.offset(28 as i32 as isize) as *const ::core::ffi::c_void,
        2 as usize,
    );
    memcpy(
        b.offset(20 as i32 as isize) as *mut ::core::ffi::c_void,
        &raw const (*m).uid as *const ::core::ffi::c_void,
        4 as usize,
    );
    memcpy(
        b.offset(24 as i32 as isize) as *mut ::core::ffi::c_void,
        &raw const (*m).gid as *const ::core::ffi::c_void,
        4 as usize,
    );
    let mut vt: u32 = (*m).mode & S_IFMT as u32;
    if eng_meta_is_placeholder(m, mode as u32) != 0 {
        mode = (*m).mode as u16;
        let mut zero32: u32 = 0 as u32;
        let mut dev: i32 = (vt == S_IFCHR as u32 || vt == S_IFBLK as u32) as i32;
        memcpy(
            b.offset(128 as i32 as isize) as *mut ::core::ffi::c_void,
            (if dev != 0 {
                &raw const (*m).major
            } else {
                &raw mut zero32 as *const u32
            }) as *const ::core::ffi::c_void,
            4 as usize,
        );
        memcpy(
            b.offset(132 as i32 as isize) as *mut ::core::ffi::c_void,
            (if dev != 0 {
                &raw const (*m).minor
            } else {
                &raw mut zero32 as *const u32
            }) as *const ::core::ffi::c_void,
            4 as usize,
        );
        let mut zero: u64 = 0 as u64;
        memcpy(
            b.offset(40 as i32 as isize) as *mut ::core::ffi::c_void,
            &raw mut zero as *const ::core::ffi::c_void,
            8 as usize,
        );
    } else {
        mode = ((mode as i32 & S_IFMT) as u32 | (*m).mode & 0o7777 as u32) as u16;
    }
    memcpy(
        b.offset(28 as i32 as isize) as *mut ::core::ffi::c_void,
        &raw mut mode as *const ::core::ffi::c_void,
        2 as usize,
    );
    if (*m).nlink != 0 {
        v = (*m).nlink;
        memcpy(
            b.offset(16 as i32 as isize) as *mut ::core::ffi::c_void,
            &raw mut v as *const ::core::ffi::c_void,
            4 as usize,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_in_group(
    mut t: *const eng_task,
    mut gid: u32,
    mut use_real: i32,
) -> i32 {
    if if use_real != 0 {
        (*t).cr.rgid
    } else {
        (*t).cr.fsgid
    } == gid
    {
        return 1 as i32;
    }
    let mut i: u32 = 0 as u32;
    while i < (*t).cr.ngroups {
        if (*t).cr.groups[i as usize] == gid {
            return 1 as i32;
        }
        i = i.wrapping_add(1);
    }
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_permission(
    mut t: *mut eng_task,
    mut m: *const eng_meta,
    mut mask: i32,
    mut use_real: i32,
) -> i32 {
    let mut uid: u32 = if use_real != 0 {
        (*t).cr.ruid
    } else {
        (*t).cr.fsuid
    };
    let mut caps: u64 = if use_real != 0 {
        if (*t).cr.ruid == 0 as u32 {
            (*t).cr.cap_prm
        } else {
            0 as u64
        }
    } else {
        (*t).cr.cap_eff
    };
    let mut bits: u32 = 0;
    if uid == (*m).uid {
        bits = ((*m).mode >> 6 as i32 & 7 as u32) as u32;
    } else if eng_in_group(t, (*m).gid, use_real) != 0 {
        bits = ((*m).mode >> 3 as i32 & 7 as u32) as u32;
    } else {
        bits = ((*m).mode & 7 as u32) as u32;
    }
    let mut want: u32 = (if mask & R_OK != 0 { 4 as i32 } else { 0 as i32 }
        | if mask & W_OK != 0 { 2 as i32 } else { 0 as i32 }
        | if mask & X_OK != 0 { 1 as i32 } else { 0 as i32 }) as u32;
    if bits & want == want {
        return 0 as i32;
    }
    let mut dir: i32 = ((*m).mode & S_IFMT as u32 == S_IFDIR as u32) as i32;
    if caps >> ENG_CAP_DAC_READ_SEARCH & 1 as u64 != 0 {
        if dir != 0 && mask & W_OK == 0 {
            return 0 as i32;
        }
        if dir == 0 && want == 4 as u32 {
            return 0 as i32;
        }
    }
    if caps >> ENG_CAP_DAC_OVERRIDE & 1 as u64 != 0 {
        if dir != 0 || mask & X_OK == 0 || (*m).mode & 0o111 as u32 != 0 {
            return 0 as i32;
        }
    }
    return -EACCES;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_may_exec(
    mut t: *mut eng_task,
    mut host: *const ::core::ffi::c_char,
    mut st: *const stat,
) -> i32 {
    let mut m: eng_meta = eng_meta {
        uid: 0,
        gid: 0,
        mode: 0,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 0,
    };
    eng_meta_get(eng_tracer_guest((*t).tr), host, 0 as i32, st, &raw mut m);
    return eng_meta_permission(t, &raw mut m, X_OK, 0 as i32);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_link_is_stub_text(
    mut text: *const ::core::ffi::c_char,
    mut id: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut pl: usize =
        ::core::mem::size_of::<[::core::ffi::c_char; 25]>().wrapping_sub(1 as usize);
    if strncmp(text, ENG_LINK_PREFIX.as_ptr(), pl) != 0 {
        return 0 as i32;
    }
    let mut p: *const ::core::ffi::c_char = text.offset(pl as isize);
    let mut n: usize = strlen(p);
    if n == 0 as usize || n >= cap || !strchr(p, '/' as i32).is_null() {
        return 0 as i32;
    }
    memcpy(
        id as *mut ::core::ffi::c_void,
        p as *const ::core::ffi::c_void,
        n.wrapping_add(1 as usize),
    );
    return 1 as i32;
}
unsafe extern "C" fn store_dir(
    mut g: *mut eng_guest,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut n: i32 = snprintf(
        out,
        cap,
        b"%s%s/links\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        ENG_STORE_GUEST.as_ptr(),
    );
    return if n > 0 as i32 && (n as usize) < cap {
        0 as i32
    } else {
        -ENAMETOOLONG
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_link_object_path(
    mut g: *mut eng_guest,
    mut id: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut n: i32 = snprintf(
        out,
        cap,
        b"%s%s/links/%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        ENG_STORE_GUEST.as_ptr(),
        id,
    );
    return if n > 0 as i32 && (n as usize) < cap {
        0 as i32
    } else {
        -ENAMETOOLONG
    };
}
static mut g_meta_lock_fd: i32 = -1 as i32;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_lock(mut g: *mut eng_guest) -> i32 {
    if g.is_null() {
        return -1 as i32;
    }
    if g_meta_lock_fd < 0 as i32 {
        let mut top: [::core::ffi::c_char; 4096] = [0; 4096];
        let mut p: [::core::ffi::c_char; 4112] = [0; 4112];
        snprintf(
            &raw mut top as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut (*g).root as *mut ::core::ffi::c_char,
            ENG_STORE_GUEST.as_ptr(),
        );
        mkdir(&raw mut top as *mut ::core::ffi::c_char, 0o700 as u32);
        snprintf(
            &raw mut p as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4112]>(),
            b"%s/meta.lock\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut top as *mut ::core::ffi::c_char,
        );
        g_meta_lock_fd = open(
            &raw mut p as *mut ::core::ffi::c_char,
            O_RDWR | O_CREAT | O_CLOEXEC,
            0o600 as i32,
        );
        if g_meta_lock_fd < 0 as i32 {
            return -1 as i32;
        }
    }
    while flock(g_meta_lock_fd, LOCK_EX) != 0 as i32 && *errno() == EINTR {}
    return g_meta_lock_fd;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_unlock(mut token: i32) {
    if token >= 0 as i32 {
        flock(token, LOCK_UN);
    }
}
unsafe extern "C" fn store_lock(mut g: *mut eng_guest) -> i32 {
    let mut d: [::core::ffi::c_char; 4096] = [0; 4096];
    if store_dir(
        g,
        &raw mut d as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return -1 as i32;
    }
    let mut fd: i32 = eng_meta_lock(g);
    mkdir(&raw mut d as *mut ::core::ffi::c_char, 0o700 as u32);
    return fd;
}
unsafe extern "C" fn store_unlock(mut fd: i32) {
    eng_meta_unlock(fd);
}
unsafe extern "C" fn crash_point(mut name: *const ::core::ffi::c_char) -> i32 {
    let mut c: *const ::core::ffi::c_char =
        getenv(b"WORKFLOW_ENGINE_CRASH_AT\0".as_ptr() as *const ::core::ffi::c_char);
    if !c.is_null() && strcmp(c, name) == 0 {
        eng_logf!(
            eng_log_level::ENG_LOG_WARN,
            b"crash injection at %s\0".as_ptr() as *const ::core::ffi::c_char,
            name,
        );
        _exit(99 as i32);
    }
    return 0 as i32;
}
unsafe extern "C" fn obj_adjust(mut obj: *const ::core::ffi::c_char, mut delta: i32) -> i32 {
    let mut st: stat = platform_empty_stat();
    if stat(obj, &raw mut st) != 0 as i32 {
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
    eng_meta_read(
        ::core::ptr::null_mut::<eng_guest>(),
        obj,
        0 as i32,
        &raw mut st,
        &raw mut m,
    );
    if m.present == 0 {
        m.uid = 0 as u32;
        m.gid = 0 as u32;
        m.mode = st.st_mode as u32;
    }
    let mut n: i64 = m.nlink as i64 + delta as i64;
    if n < 0 as i64 {
        n = 0 as i64;
    }
    m.nlink = n as u32;
    return eng_meta_write(obj, 0 as i32, &raw mut m);
}
unsafe extern "C" fn new_id(mut id: *mut ::core::ffi::c_char, mut cap: usize) {
    let mut r: u64 = 0 as u64;
    let mut fd: i32 = open(
        b"/dev/urandom\0".as_ptr() as *const ::core::ffi::c_char,
        O_RDONLY | O_CLOEXEC,
    );
    if fd >= 0 as i32 {
        if read(
            fd,
            &raw mut r as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<u64>(),
        ) != ::core::mem::size_of::<u64>() as isize
        {
            r = 0 as u64;
        }
        close(fd);
    }
    let mut ts: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    clock_gettime(CLOCK_REALTIME, &raw mut ts);
    r ^= (ts.tv_nsec as u64) << 20 as i32 ^ ts.tv_sec as u64 ^ (getpid() as u64) << 40 as i32;
    snprintf(
        id,
        cap,
        b"%016llx\0".as_ptr() as *const ::core::ffi::c_char,
        r,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_link_create(
    mut g: *mut eng_guest,
    mut old_entry: *const ::core::ffi::c_char,
    mut new_entry: *const ::core::ffi::c_char,
) -> i32 {
    let mut st: stat = platform_empty_stat();
    if lstat(old_entry, &raw mut st) != 0 as i32 {
        return -*errno();
    }
    let mut nst: stat = platform_empty_stat();
    if lstat(new_entry, &raw mut nst) == 0 as i32 {
        return -EEXIST;
    }
    let mut lk: i32 = store_lock(g);
    if lk < 0 as i32 {
        return -EIO;
    }
    let mut id: [::core::ffi::c_char; 64] = [0; 64];
    let mut obj: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut text: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut sd: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut rc: i32 = 0 as i32;
    store_dir(
        g,
        &raw mut sd as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    '_out: {
        if st.st_mode & S_IFMT as u32 == S_IFLNK as u32 {
            let mut t: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut n: isize = readlink(
                old_entry,
                &raw mut t as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as usize),
            );
            if n < 0 as isize {
                rc = -*errno();
                break '_out;
            } else {
                t[n as usize] = 0 as ::core::ffi::c_char;
                if eng_link_is_stub_text(
                    &raw mut t as *mut ::core::ffi::c_char,
                    &raw mut id as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                ) == 0
                {
                    rc = if symlink(&raw mut t as *mut ::core::ffi::c_char, new_entry) == 0 as i32 {
                        0 as i32
                    } else {
                        -*errno()
                    };
                    break '_out;
                } else {
                    eng_link_object_path(
                        g,
                        &raw mut id as *mut ::core::ffi::c_char,
                        &raw mut obj as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                    );
                }
            }
        } else if st.st_mode & S_IFMT as u32 == S_IFREG as u32
            || st.st_mode & S_IFMT as u32 == S_IFIFO as u32
            || st.st_mode & S_IFMT as u32 == S_IFSOCK as u32
        {
            new_id(
                &raw mut id as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            );
            eng_link_object_path(
                g,
                &raw mut id as *mut ::core::ffi::c_char,
                &raw mut obj as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            );
            let mut jpath: [::core::ffi::c_char; 4160] = [0; 4160];
            snprintf(
                &raw mut jpath as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4160]>(),
                b"%s/journal-%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut sd as *mut ::core::ffi::c_char,
                &raw mut id as *mut ::core::ffi::c_char,
            );
            let mut jf: i32 = open(
                &raw mut jpath as *mut ::core::ffi::c_char,
                O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC,
                0o600 as i32,
            );
            if jf < 0 as i32 {
                rc = -*errno();
                break '_out;
            } else {
                let mut l: usize = strlen(old_entry);
                if write(jf, old_entry as *const ::core::ffi::c_void, l) != l as isize
                    || fsync(jf) != 0 as i32
                {
                    rc = -EIO;
                    close(jf);
                    unlink(&raw mut jpath as *mut ::core::ffi::c_char);
                    break '_out;
                } else {
                    close(jf);
                    crash_point(b"link-journaled\0".as_ptr() as *const ::core::ffi::c_char);
                    if rename(old_entry, &raw mut obj as *mut ::core::ffi::c_char) != 0 as i32 {
                        rc = -*errno();
                        unlink(&raw mut jpath as *mut ::core::ffi::c_char);
                        break '_out;
                    } else {
                        crash_point(b"link-moved\0".as_ptr() as *const ::core::ffi::c_char);
                        snprintf(
                            &raw mut text as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                            ENG_LINK_PREFIX.as_ptr(),
                            &raw mut id as *mut ::core::ffi::c_char,
                        );
                        if symlink(&raw mut text as *mut ::core::ffi::c_char, old_entry) != 0 as i32
                        {
                            rc = -*errno();
                            rename(&raw mut obj as *mut ::core::ffi::c_char, old_entry);
                            unlink(&raw mut jpath as *mut ::core::ffi::c_char);
                            break '_out;
                        } else {
                            crash_point(b"link-stubbed\0".as_ptr() as *const ::core::ffi::c_char);
                            let mut m: eng_meta = eng_meta {
                                uid: 0,
                                gid: 0,
                                mode: 0,
                                nlink: 0,
                                major: 0,
                                minor: 0,
                                present: 0,
                            };
                            eng_meta_read(
                                g,
                                &raw mut obj as *mut ::core::ffi::c_char,
                                0 as i32,
                                &raw mut st,
                                &raw mut m,
                            );
                            if m.present == 0 {
                                m.mode = st.st_mode as u32;
                            }
                            m.nlink = 1 as u32;
                            eng_meta_write(
                                &raw mut obj as *mut ::core::ffi::c_char,
                                0 as i32,
                                &raw mut m,
                            );
                            unlink(&raw mut jpath as *mut ::core::ffi::c_char);
                        }
                    }
                }
            }
        } else {
            rc = -EPERM;
            break '_out;
        }
        rc = obj_adjust(&raw mut obj as *mut ::core::ffi::c_char, 1 as i32);
        if rc == 0 {
            crash_point(b"link-counted\0".as_ptr() as *const ::core::ffi::c_char);
            snprintf(
                &raw mut text as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                ENG_LINK_PREFIX.as_ptr(),
                &raw mut id as *mut ::core::ffi::c_char,
            );
            if symlink(&raw mut text as *mut ::core::ffi::c_char, new_entry) != 0 as i32 {
                rc = -*errno();
                obj_adjust(&raw mut obj as *mut ::core::ffi::c_char, -1 as i32);
            }
        }
    }
    store_unlock(lk);
    return rc;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_link_drop(mut g: *mut eng_guest, mut id: *const ::core::ffi::c_char) {
    let mut obj: [::core::ffi::c_char; 4096] = [0; 4096];
    if eng_link_object_path(
        g,
        id,
        &raw mut obj as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return;
    }
    crash_point(b"link-drop\0".as_ptr() as *const ::core::ffi::c_char);
    let mut lk: i32 = store_lock(g);
    let mut st: stat = platform_empty_stat();
    if stat(&raw mut obj as *mut ::core::ffi::c_char, &raw mut st) == 0 as i32 {
        let mut m: eng_meta = eng_meta {
            uid: 0,
            gid: 0,
            mode: 0,
            nlink: 0,
            major: 0,
            minor: 0,
            present: 0,
        };
        eng_meta_read(
            g,
            &raw mut obj as *mut ::core::ffi::c_char,
            0 as i32,
            &raw mut st,
            &raw mut m,
        );
        if m.nlink <= 1 as u32 {
            unlink(&raw mut obj as *mut ::core::ffi::c_char);
        } else {
            obj_adjust(&raw mut obj as *mut ::core::ffi::c_char, -1 as i32);
        }
    }
    store_unlock(lk);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_link_recover(mut g: *mut eng_guest) -> i32 {
    let mut sd: [::core::ffi::c_char; 4096] = [0; 4096];
    if store_dir(
        g,
        &raw mut sd as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return -ENAMETOOLONG;
    }
    let mut d: *mut DIR = opendir(&raw mut sd as *mut ::core::ffi::c_char);
    if d.is_null() {
        return 0 as i32;
    }
    let mut lk: i32 = store_lock(g);
    let mut fixed: i32 = 0 as i32;
    let mut e: *mut dirent = ::core::ptr::null_mut::<dirent>();
    loop {
        e = readdir(d);
        if e.is_null() {
            break;
        }
        if strncmp(
            &raw mut (*e).d_name as *mut ::core::ffi::c_char,
            b"journal-\0".as_ptr() as *const ::core::ffi::c_char,
            8 as usize,
        ) != 0
        {
            continue;
        }
        let mut id: *const ::core::ffi::c_char =
            (&raw mut (*e).d_name as *mut ::core::ffi::c_char).offset(8 as i32 as isize);
        let mut jpath: [::core::ffi::c_char; 4396] = [0; 4396];
        let mut entry: [::core::ffi::c_char; 4096] = [0; 4096];
        let mut obj: [::core::ffi::c_char; 4096] = [0; 4096];
        snprintf(
            &raw mut jpath as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4396]>(),
            b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut sd as *mut ::core::ffi::c_char,
            &raw mut (*e).d_name as *mut ::core::ffi::c_char,
        );
        let mut jf: i32 = open(
            &raw mut jpath as *mut ::core::ffi::c_char,
            O_RDONLY | O_CLOEXEC,
        );
        if jf < 0 as i32 {
            continue;
        }
        let mut n: isize = read(
            jf,
            &raw mut entry as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as usize),
        );
        close(jf);
        if n <= 0 as isize {
            unlink(&raw mut jpath as *mut ::core::ffi::c_char);
        } else {
            entry[n as usize] = 0 as ::core::ffi::c_char;
            eng_link_object_path(
                g,
                id,
                &raw mut obj as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            );
            let mut est: stat = platform_empty_stat();
            let mut ost: stat = platform_empty_stat();
            let mut have_entry: i32 =
                (lstat(&raw mut entry as *mut ::core::ffi::c_char, &raw mut est) == 0 as i32)
                    as i32;
            let mut have_obj: i32 =
                (stat(&raw mut obj as *mut ::core::ffi::c_char, &raw mut ost) == 0 as i32) as i32;
            if have_entry == 0 && have_obj != 0 {
                rename(
                    &raw mut obj as *mut ::core::ffi::c_char,
                    &raw mut entry as *mut ::core::ffi::c_char,
                );
            } else if have_entry != 0
                && est.st_mode & S_IFMT as u32 == S_IFLNK as u32
                && have_obj != 0
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
                eng_meta_read(
                    g,
                    &raw mut obj as *mut ::core::ffi::c_char,
                    0 as i32,
                    &raw mut ost,
                    &raw mut m,
                );
                if m.present == 0 {
                    m.mode = ost.st_mode as u32;
                }
                if m.nlink == 0 as u32 {
                    m.nlink = 1 as u32;
                    eng_meta_write(
                        &raw mut obj as *mut ::core::ffi::c_char,
                        0 as i32,
                        &raw mut m,
                    );
                }
            }
            unlink(&raw mut jpath as *mut ::core::ffi::c_char);
            fixed += 1;
        }
    }
    closedir(d);
    store_unlock(lk);
    if fixed != 0 {
        eng_logf!(
            eng_log_level::ENG_LOG_INFO,
            b"hardlink store: recovered %d interrupted operation(s)\0".as_ptr()
                as *const ::core::ffi::c_char,
            fixed,
        );
    }
    return fixed;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_instance_lock(mut g: *mut eng_guest, mut exclusive: i32) -> i32 {
    let mut top: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut p: [::core::ffi::c_char; 4128] = [0; 4128];
    snprintf(
        &raw mut top as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        ENG_STORE_GUEST.as_ptr(),
    );
    mkdir(&raw mut top as *mut ::core::ffi::c_char, 0o700 as u32);
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4128]>(),
        b"%s/instances.lock\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut top as *mut ::core::ffi::c_char,
    );
    let mut fd: i32 = open(
        &raw mut p as *mut ::core::ffi::c_char,
        O_RDWR | O_CREAT | O_CLOEXEC,
        0o600 as i32,
    );
    if fd < 0 as i32 {
        return -*errno();
    }
    let mut op: i32 = if exclusive != 0 {
        LOCK_EX | LOCK_NB
    } else {
        LOCK_SH
    };
    while flock(fd, op) != 0 as i32 {
        if *errno() == EINTR {
            continue;
        }
        let mut e: i32 = *errno();
        close(fd);
        return -e;
    }
    return fd;
}
static mut g_fsck: C2Rust_Unnamed = C2Rust_Unnamed {
    v: ::core::ptr::null_mut::<fsck_ent>(),
    n: 0,
    cap: 0,
};
unsafe extern "C" fn fsck_find(mut id: *const ::core::ffi::c_char) -> *mut fsck_ent {
    let mut i: usize = 0 as usize;
    while i < g_fsck.n {
        if strcmp(
            &raw mut (*g_fsck.v.offset(i as isize)).id as *mut ::core::ffi::c_char,
            id,
        ) == 0
        {
            return g_fsck.v.offset(i as isize);
        }
        i = i.wrapping_add(1);
    }
    if g_fsck.n == g_fsck.cap {
        let mut nc: usize = if g_fsck.cap != 0 {
            g_fsck.cap.wrapping_mul(2 as usize)
        } else {
            64 as usize
        };
        let mut nv: *mut fsck_ent = realloc(
            g_fsck.v as *mut ::core::ffi::c_void,
            nc.wrapping_mul(::core::mem::size_of::<fsck_ent>()),
        ) as *mut fsck_ent;
        if nv.is_null() {
            return ::core::ptr::null_mut::<fsck_ent>();
        }
        g_fsck.v = nv;
        g_fsck.cap = nc;
    }
    let c2rust_fresh0 = g_fsck.n;
    g_fsck.n = g_fsck.n.wrapping_add(1);
    let mut e: *mut fsck_ent = g_fsck.v.offset(c2rust_fresh0 as isize);
    memset(
        e as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<fsck_ent>(),
    );
    snprintf(
        &raw mut (*e).id as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 24]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        id,
    );
    return e;
}
unsafe extern "C" fn fsck_walk(mut dfd: i32, mut depth: i32) {
    if depth > 256 as i32 {
        return;
    }
    let mut d: *mut DIR = fdopendir(dfd);
    if d.is_null() {
        close(dfd);
        return;
    }
    let mut e: *mut dirent = ::core::ptr::null_mut::<dirent>();
    loop {
        e = readdir(d);
        if e.is_null() {
            break;
        }
        let mut n: *const ::core::ffi::c_char = &raw mut (*e).d_name as *mut ::core::ffi::c_char;
        if strcmp(n, b".\0".as_ptr() as *const ::core::ffi::c_char) == 0
            || strcmp(n, b"..\0".as_ptr() as *const ::core::ffi::c_char) == 0
        {
            continue;
        }
        if depth == 0 as i32 && strcmp(n, ENG_STORE_GUEST.as_ptr().offset(1isize)) == 0 {
            continue;
        }
        let mut r#type: u8 = (*e).d_type;
        #[cfg(target_os = "linux")]
        if r#type as i32 == DT_UNKNOWN as i32 {
            let mut st: stat = platform_empty_stat();
            if fstatat(dirfd(d), n, &raw mut st, AT_SYMLINK_NOFOLLOW) != 0 as i32 {
                continue;
            }
            r#type = (if st.st_mode & S_IFMT as u32 == S_IFDIR as u32 {
                DT_DIR as i32
            } else if st.st_mode & S_IFMT as u32 == S_IFLNK as u32 {
                DT_LNK as i32
            } else {
                DT_REG as i32
            }) as u8;
        }
        #[cfg(target_os = "android")]
        if r#type as i32 == DT_UNKNOWN {
            let mut st: stat = platform_empty_stat();
            if fstatat(dirfd(d), n, &raw mut st, AT_SYMLINK_NOFOLLOW) != 0 as i32 {
                continue;
            }
            r#type = (if st.st_mode & S_IFMT as u32 == S_IFDIR as u32 {
                DT_DIR
            } else if st.st_mode & S_IFMT as u32 == S_IFLNK as u32 {
                DT_LNK
            } else {
                DT_REG
            }) as u8;
        }
        #[cfg(target_os = "linux")]
        if r#type as i32 == DT_DIR as i32 {
            let mut fd: i32 = openat(dirfd(d), n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if fd >= 0 as i32 {
                fsck_walk(fd, depth + 1 as i32);
            }
        } else {
            if r#type as i32 != DT_LNK as i32 {
                continue;
            }
            let mut txt: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut id: [::core::ffi::c_char; 64] = [0; 64];
            let mut k: isize = readlinkat(
                dirfd(d),
                n,
                &raw mut txt as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as usize),
            );
            if k <= 0 as isize {
                continue;
            }
            txt[k as usize] = 0 as ::core::ffi::c_char;
            if eng_link_is_stub_text(
                &raw mut txt as *mut ::core::ffi::c_char,
                &raw mut id as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            ) == 0
            {
                continue;
            }
            let mut fe: *mut fsck_ent = fsck_find(&raw mut id as *mut ::core::ffi::c_char);
            if !fe.is_null() {
                (*fe).names = (*fe).names.wrapping_add(1);
            }
        }
        #[cfg(target_os = "android")]
        if r#type as i32 == DT_DIR {
            let mut fd: i32 = openat(dirfd(d), n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if fd >= 0 as i32 {
                fsck_walk(fd, depth + 1 as i32);
            }
        } else {
            if r#type as i32 != DT_LNK {
                continue;
            }
            let mut txt: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut id: [::core::ffi::c_char; 64] = [0; 64];
            let mut k: isize = readlinkat(
                dirfd(d),
                n,
                &raw mut txt as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as usize),
            );
            if k <= 0 as isize {
                continue;
            }
            txt[k as usize] = 0 as ::core::ffi::c_char;
            if eng_link_is_stub_text(
                &raw mut txt as *mut ::core::ffi::c_char,
                &raw mut id as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            ) == 0
            {
                continue;
            }
            let mut fe: *mut fsck_ent = fsck_find(&raw mut id as *mut ::core::ffi::c_char);
            if !fe.is_null() {
                (*fe).names = (*fe).names.wrapping_add(1);
            }
        }
    }
    closedir(d);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_link_fsck(
    mut g: *mut eng_guest,
    mut repair: i32,
    mut outp: *mut ::core::ffi::c_void,
) -> i32 {
    let mut out: *mut FILE = (if !outp.is_null() {
        outp
    } else {
        stdout as *mut ::core::ffi::c_void
    }) as *mut FILE;
    eng_link_recover(g);
    let mut sd: [::core::ffi::c_char; 4096] = [0; 4096];
    if store_dir(
        g,
        &raw mut sd as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return -ENAMETOOLONG;
    }
    memset(
        &raw mut g_fsck as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<C2Rust_Unnamed>(),
    );
    let mut lk: i32 = store_lock(g);
    let mut d: *mut DIR = opendir(&raw mut sd as *mut ::core::ffi::c_char);
    if !d.is_null() {
        let mut e: *mut dirent = ::core::ptr::null_mut::<dirent>();
        loop {
            e = readdir(d);
            if e.is_null() {
                break;
            }
            if (*e).d_name[0usize] as i32 == '.' as i32 {
                continue;
            }
            if strncmp(
                &raw mut (*e).d_name as *mut ::core::ffi::c_char,
                b"journal-\0".as_ptr() as *const ::core::ffi::c_char,
                8 as usize,
            ) == 0
            {
                continue;
            }
            let mut obj: [::core::ffi::c_char; 4096] = [0; 4096];
            if eng_link_object_path(
                g,
                &raw mut (*e).d_name as *mut ::core::ffi::c_char,
                &raw mut obj as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            ) != 0
            {
                continue;
            }
            let mut st: stat = platform_empty_stat();
            if stat(&raw mut obj as *mut ::core::ffi::c_char, &raw mut st) != 0 as i32 {
                continue;
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
            eng_meta_read(
                ::core::ptr::null_mut::<eng_guest>(),
                &raw mut obj as *mut ::core::ffi::c_char,
                0 as i32,
                &raw mut st,
                &raw mut m,
            );
            let mut fe: *mut fsck_ent = fsck_find(&raw mut (*e).d_name as *mut ::core::ffi::c_char);
            if !fe.is_null() {
                (*fe).have_obj = 1 as i32;
                (*fe).nlink = m.nlink as u32;
            }
        }
        closedir(d);
    }
    let mut problems: i32 = 0 as i32;
    let mut rfd: i32 = open(
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        O_RDONLY | O_DIRECTORY | O_CLOEXEC,
    );
    if rfd < 0 as i32 {
        store_unlock(lk);
        free(g_fsck.v as *mut ::core::ffi::c_void);
        return -*errno();
    }
    fsck_walk(rfd, 0 as i32);
    let mut i: usize = 0 as usize;
    while i < g_fsck.n {
        let mut e_0: *mut fsck_ent = g_fsck.v.offset(i as isize);
        let mut obj_0: [::core::ffi::c_char; 4096] = [0; 4096];
        eng_link_object_path(
            g,
            &raw mut (*e_0).id as *mut ::core::ffi::c_char,
            &raw mut obj_0 as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        );
        's_117: {
            if (*e_0).have_obj == 0 {
                fprintf(
                    out,
                    b"dangling: %u name(s) of missing object %s\n\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    (*e_0).names,
                    &raw mut (*e_0).id as *mut ::core::ffi::c_char,
                );
                problems += 1;
            } else if (*e_0).names == 0 as u32 {
                fprintf(
                    out,
                    b"orphan: object %s has no names (nlink %u)%s\n\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    &raw mut (*e_0).id as *mut ::core::ffi::c_char,
                    (*e_0).nlink,
                    if repair != 0 {
                        b": removed\0".as_ptr() as *const ::core::ffi::c_char
                    } else {
                        b"\0".as_ptr() as *const ::core::ffi::c_char
                    },
                );
                if !(repair != 0 && unlink(&raw mut obj_0 as *mut ::core::ffi::c_char) == 0 as i32)
                {
                    problems += 1;
                }
            } else if (*e_0).names != (*e_0).nlink {
                fprintf(
                    out,
                    b"count: object %s nlink %u, names %u%s\n\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    &raw mut (*e_0).id as *mut ::core::ffi::c_char,
                    (*e_0).nlink,
                    (*e_0).names,
                    if repair != 0 {
                        b": fixed\0".as_ptr() as *const ::core::ffi::c_char
                    } else {
                        b"\0".as_ptr() as *const ::core::ffi::c_char
                    },
                );
                if repair != 0 {
                    let mut st_0: stat = platform_empty_stat();
                    let mut m_0: eng_meta = eng_meta {
                        uid: 0,
                        gid: 0,
                        mode: 0,
                        nlink: 0,
                        major: 0,
                        minor: 0,
                        present: 0,
                    };
                    if stat(&raw mut obj_0 as *mut ::core::ffi::c_char, &raw mut st_0) == 0 as i32
                        && eng_meta_read(
                            ::core::ptr::null_mut::<eng_guest>(),
                            &raw mut obj_0 as *mut ::core::ffi::c_char,
                            0 as i32,
                            &raw mut st_0,
                            &raw mut m_0,
                        ) == 0 as i32
                    {
                        if m_0.present == 0 {
                            m_0.mode = st_0.st_mode as u32;
                        }
                        m_0.nlink = (*e_0).names as u32;
                        if eng_meta_write(
                            &raw mut obj_0 as *mut ::core::ffi::c_char,
                            0 as i32,
                            &raw mut m_0,
                        ) == 0 as i32
                        {
                            break 's_117;
                        }
                    }
                }
                problems += 1;
            }
        }
        i = i.wrapping_add(1);
    }
    store_unlock(lk);
    fprintf(
        out,
        b"hardlink store: %zu object(s), %d problem(s)%s\n\0".as_ptr()
            as *const ::core::ffi::c_char,
        g_fsck.n,
        problems,
        if repair != 0 {
            b" left\0".as_ptr() as *const ::core::ffi::c_char
        } else {
            b"\0".as_ptr() as *const ::core::ffi::c_char
        },
    );
    free(g_fsck.v as *mut ::core::ffi::c_void);
    memset(
        &raw mut g_fsck as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<C2Rust_Unnamed>(),
    );
    return problems;
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
#[cfg(target_os = "android")]
#[repr(C)]
pub struct DIR {
    _opaque: [u8; 0],
}
#[cfg(target_os = "android")]
#[repr(C)]
pub struct __sFILE {
    _opaque: [u8; 0],
}
