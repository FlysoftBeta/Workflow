//! Xattr ownership and permissions, emulated hardlinks and crash recovery.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
#[repr(C)]
pub struct eng_tracer {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct __dirstream {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct _IO_wide_data {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct _IO_codecvt {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct _IO_marker {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    unsafe fn stat(__file: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn fstatat(
        __fd: ::core::ffi::c_int,
        __file: *const ::core::ffi::c_char,
        __buf: *mut stat,
        __flag: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn lstat(__file: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn mkdir(__path: *const ::core::ffi::c_char, __mode: __mode_t) -> ::core::ffi::c_int;
    unsafe fn mkfifo(__path: *const ::core::ffi::c_char, __mode: __mode_t) -> ::core::ffi::c_int;
    unsafe fn close(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn read(
        __fd: ::core::ffi::c_int,
        __buf: *mut ::core::ffi::c_void,
        __nbytes: size_t,
    ) -> ssize_t;
    unsafe fn write(
        __fd: ::core::ffi::c_int,
        __buf: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ssize_t;
    unsafe fn _exit(__status: ::core::ffi::c_int) -> !;
    unsafe fn getpid() -> __pid_t;
    unsafe fn symlink(
        __from: *const ::core::ffi::c_char,
        __to: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn readlink(
        __path: *const ::core::ffi::c_char,
        __buf: *mut ::core::ffi::c_char,
        __len: size_t,
    ) -> ssize_t;
    unsafe fn readlinkat(
        __fd: ::core::ffi::c_int,
        __path: *const ::core::ffi::c_char,
        __buf: *mut ::core::ffi::c_char,
        __len: size_t,
    ) -> ssize_t;
    unsafe fn unlink(__name: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    unsafe fn fsync(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn eng_tracer_guest(tr: *mut eng_tracer) -> *mut eng_guest;
    unsafe fn closedir(__dirp: *mut DIR) -> ::core::ffi::c_int;
    unsafe fn opendir(__name: *const ::core::ffi::c_char) -> *mut DIR;
    unsafe fn fdopendir(__fd: ::core::ffi::c_int) -> *mut DIR;
    unsafe fn readdir(__dirp: *mut DIR) -> *mut dirent;
    unsafe fn dirfd(__dirp: *mut DIR) -> ::core::ffi::c_int;
    unsafe fn __errno_location() -> *mut ::core::ffi::c_int;
    unsafe fn open(
        __file: *const ::core::ffi::c_char,
        __oflag: ::core::ffi::c_int,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn openat(
        __fd: ::core::ffi::c_int,
        __file: *const ::core::ffi::c_char,
        __oflag: ::core::ffi::c_int,
        ...
    ) -> ::core::ffi::c_int;
    static mut stdout: *mut FILE;
    unsafe fn rename(
        __old: *const ::core::ffi::c_char,
        __new: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn sscanf(
        __s: *const ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    unsafe fn free(__ptr: *mut ::core::ffi::c_void);
    unsafe fn getenv(__name: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn strchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strstr(
        __haystack: *const ::core::ffi::c_char,
        __needle: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    unsafe fn flock(
        __fd: ::core::ffi::c_int,
        __operation: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn gnu_dev_makedev(
        __major: ::core::ffi::c_uint,
        __minor: ::core::ffi::c_uint,
    ) -> __dev_t;
    unsafe fn setxattr(
        __path: *const ::core::ffi::c_char,
        __name: *const ::core::ffi::c_char,
        __value: *const ::core::ffi::c_void,
        __size: size_t,
        __flags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn lsetxattr(
        __path: *const ::core::ffi::c_char,
        __name: *const ::core::ffi::c_char,
        __value: *const ::core::ffi::c_void,
        __size: size_t,
        __flags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn getxattr(
        __path: *const ::core::ffi::c_char,
        __name: *const ::core::ffi::c_char,
        __value: *mut ::core::ffi::c_void,
        __size: size_t,
    ) -> ssize_t;
    unsafe fn lgetxattr(
        __path: *const ::core::ffi::c_char,
        __name: *const ::core::ffi::c_char,
        __value: *mut ::core::ffi::c_void,
        __size: size_t,
    ) -> ssize_t;
    unsafe fn clock_gettime(__clock_id: clockid_t, __tp: *mut timespec) -> ::core::ffi::c_int;
    unsafe fn eng_guest_locate(
        g: *const eng_guest,
        host: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
pub type __uint64_t = u64;
pub type __dev_t = ::core::ffi::c_ulong;
pub type __uid_t = ::core::ffi::c_uint;
pub type __gid_t = ::core::ffi::c_uint;
pub type __ino_t = ::core::ffi::c_ulong;
pub type __mode_t = ::core::ffi::c_uint;
pub type __nlink_t = ::core::ffi::c_ulong;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
pub type __pid_t = ::core::ffi::c_int;
pub type __time_t = ::core::ffi::c_long;
pub type __clockid_t = ::core::ffi::c_int;
pub type __blksize_t = ::core::ffi::c_long;
pub type __blkcnt_t = ::core::ffi::c_long;
pub type __syscall_slong_t = ::core::ffi::c_long;
pub type uint8_t = u8;
pub type uint16_t = u16;
pub type uint32_t = u32;
pub type uint64_t = u64;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __time_t,
    pub tv_nsec: __syscall_slong_t,
}
pub type mode_t = __mode_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: __dev_t,
    pub st_ino: __ino_t,
    pub st_nlink: __nlink_t,
    pub st_mode: __mode_t,
    pub st_uid: __uid_t,
    pub st_gid: __gid_t,
    pub __pad0: ::core::ffi::c_int,
    pub st_rdev: __dev_t,
    pub st_size: __off_t,
    pub st_blksize: __blksize_t,
    pub st_blocks: __blkcnt_t,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [__syscall_slong_t; 3],
}
pub type pid_t = __pid_t;
pub type size_t = usize;
pub type ssize_t = isize;
pub type clockid_t = __clockid_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct user_regs_struct {
    pub r15: ::core::ffi::c_ulonglong,
    pub r14: ::core::ffi::c_ulonglong,
    pub r13: ::core::ffi::c_ulonglong,
    pub r12: ::core::ffi::c_ulonglong,
    pub rbp: ::core::ffi::c_ulonglong,
    pub rbx: ::core::ffi::c_ulonglong,
    pub r11: ::core::ffi::c_ulonglong,
    pub r10: ::core::ffi::c_ulonglong,
    pub r9: ::core::ffi::c_ulonglong,
    pub r8: ::core::ffi::c_ulonglong,
    pub rax: ::core::ffi::c_ulonglong,
    pub rcx: ::core::ffi::c_ulonglong,
    pub rdx: ::core::ffi::c_ulonglong,
    pub rsi: ::core::ffi::c_ulonglong,
    pub rdi: ::core::ffi::c_ulonglong,
    pub orig_rax: ::core::ffi::c_ulonglong,
    pub rip: ::core::ffi::c_ulonglong,
    pub cs: ::core::ffi::c_ulonglong,
    pub eflags: ::core::ffi::c_ulonglong,
    pub rsp: ::core::ffi::c_ulonglong,
    pub ss: ::core::ffi::c_ulonglong,
    pub fs_base: ::core::ffi::c_ulonglong,
    pub gs_base: ::core::ffi::c_ulonglong,
    pub ds: ::core::ffi::c_ulonglong,
    pub es: ::core::ffi::c_ulonglong,
    pub fs: ::core::ffi::c_ulonglong,
    pub gs: ::core::ffi::c_ulonglong,
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
pub type DIR = __dirstream;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dirent {
    pub d_ino: __ino_t,
    pub d_off: __off_t,
    pub d_reclen: ::core::ffi::c_ushort,
    pub d_type: ::core::ffi::c_uchar,
    pub d_name: [::core::ffi::c_char; 256],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2Rust_Unnamed {
    pub v: *mut fsck_ent,
    pub n: size_t,
    pub cap: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct fsck_ent {
    pub id: [::core::ffi::c_char; 24],
    pub names: ::core::ffi::c_uint,
    pub nlink: ::core::ffi::c_uint,
    pub have_obj: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
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
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: [u8; 3],
    pub _short_backupbuf: [::core::ffi::c_char; 1],
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut ::core::ffi::c_void,
    pub _prevchain: *mut *mut _IO_FILE,
    pub _mode: ::core::ffi::c_int,
    pub _unused3: ::core::ffi::c_int,
    pub _total_written: __uint64_t,
    pub _unused2: [::core::ffi::c_char; 8],
}
pub type _IO_lock_t = ();
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct C2Rust_Unnamed_0(pub ::core::ffi::c_uint);
impl C2Rust_Unnamed_0 {
    pub const DT_UNKNOWN: Self = Self(0);
    pub const DT_FIFO: Self = Self(1);
    pub const DT_CHR: Self = Self(2);
    pub const DT_DIR: Self = Self(4);
    pub const DT_BLK: Self = Self(6);
    pub const DT_REG: Self = Self(8);
    pub const DT_LNK: Self = Self(10);
    pub const DT_SOCK: Self = Self(12);
    pub const DT_WHT: Self = Self(14);
}
pub const __S_IFMT: ::core::ffi::c_int = 0o170000 as ::core::ffi::c_int;
pub const __S_IFCHR: ::core::ffi::c_int = 0o20000 as ::core::ffi::c_int;
pub const __S_IFBLK: ::core::ffi::c_int = 0o60000 as ::core::ffi::c_int;
pub const __S_IFIFO: ::core::ffi::c_int = 0o10000 as ::core::ffi::c_int;
pub const __S_IFLNK: ::core::ffi::c_int = 0o120000 as ::core::ffi::c_int;
pub const __S_IFSOCK: ::core::ffi::c_int = 0o140000 as ::core::ffi::c_int;
pub const R_OK: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const W_OK: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const X_OK: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ENG_META_XATTR: [::core::ffi::c_char; 19] = unsafe {
    ::core::mem::transmute::<[u8; 19], [::core::ffi::c_char; 19]>(*b"user.workflow.meta\0")
};
pub const ENG_STORE_GUEST: [::core::ffi::c_char; 18] = unsafe {
    ::core::mem::transmute::<[u8; 18], [::core::ffi::c_char; 18]>(*b"/.workflow-engine\0")
};
pub const ENG_LINK_PREFIX: [::core::ffi::c_char; 25] = unsafe {
    ::core::mem::transmute::<[u8; 25], [::core::ffi::c_char; 25]>(*b"/.workflow-engine/links/\0")
};
pub const EPERM: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const EINTR: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const EIO: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const EACCES: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const EEXIST: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const ENAMETOOLONG: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const O_RDONLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const O_WRONLY: ::core::ffi::c_int = 0o1 as ::core::ffi::c_int;
pub const O_RDWR: ::core::ffi::c_int = 0o2 as ::core::ffi::c_int;
pub const O_CREAT: ::core::ffi::c_int = 0o100 as ::core::ffi::c_int;
pub const O_EXCL: ::core::ffi::c_int = 0o200 as ::core::ffi::c_int;
pub const __O_DIRECTORY: ::core::ffi::c_int = 0o200000 as ::core::ffi::c_int;
pub const __O_NOFOLLOW: ::core::ffi::c_int = 0o400000 as ::core::ffi::c_int;
pub const __O_CLOEXEC: ::core::ffi::c_int = 0o2000000 as ::core::ffi::c_int;
pub const O_DIRECTORY: ::core::ffi::c_int = __O_DIRECTORY;
pub const O_NOFOLLOW: ::core::ffi::c_int = __O_NOFOLLOW;
pub const O_CLOEXEC: ::core::ffi::c_int = __O_CLOEXEC;
pub const S_IFMT: ::core::ffi::c_int = __S_IFMT;
pub const S_IFCHR: ::core::ffi::c_int = __S_IFCHR;
pub const S_IFBLK: ::core::ffi::c_int = __S_IFBLK;
pub const S_IFIFO: ::core::ffi::c_int = __S_IFIFO;
pub const S_IFLNK: ::core::ffi::c_int = __S_IFLNK;
pub const S_IFSOCK: ::core::ffi::c_int = __S_IFSOCK;
pub const AT_SYMLINK_NOFOLLOW: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const LOCK_SH: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const LOCK_EX: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const LOCK_UN: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const LOCK_NB: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const CLOCK_REALTIME: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ENG_LOC_ROOTFS: ::core::ffi::c_int = -1 as ::core::ffi::c_int;
pub const ENG_CAP_DAC_OVERRIDE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ENG_CAP_DAC_READ_SEARCH: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
unsafe extern "C" fn parse_meta(
    mut s: *const ::core::ffi::c_char,
    mut m: *mut eng_meta,
) -> ::core::ffi::c_int {
    let mut ver: ::core::ffi::c_uint = 0;
    let mut uid: ::core::ffi::c_uint = 0;
    let mut gid: ::core::ffi::c_uint = 0;
    let mut mode: ::core::ffi::c_uint = 0;
    let mut nlink: ::core::ffi::c_uint = 0;
    let mut maj: ::core::ffi::c_uint = 0;
    let mut min: ::core::ffi::c_uint = 0;
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
    ) != 7 as ::core::ffi::c_int
        || ver != 1 as ::core::ffi::c_uint
    {
        return -1 as ::core::ffi::c_int;
    }
    (*m).uid = uid as uint32_t;
    (*m).gid = gid as uint32_t;
    (*m).mode = mode as uint32_t;
    (*m).nlink = nlink as uint32_t;
    (*m).major = maj as uint32_t;
    (*m).minor = min as uint32_t;
    (*m).present = 1 as ::core::ffi::c_int;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn object_path(
    mut host: *const ::core::ffi::c_char,
    mut buf: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> *const ::core::ffi::c_char {
    if strncmp(
        host,
        b"/proc/\0".as_ptr() as *const ::core::ffi::c_char,
        6 as size_t,
    ) != 0
        || (strstr(host, b"/fd/\0".as_ptr() as *const ::core::ffi::c_char)
            as *const ::core::ffi::c_char)
            .is_null()
    {
        return host;
    }
    let mut n: ssize_t = readlink(host, buf, cap.wrapping_sub(1 as size_t));
    if n <= 0 as ssize_t {
        return host;
    }
    *buf.offset(n) = 0 as ::core::ffi::c_char;
    return if *buf.offset(0isize) as ::core::ffi::c_int == '/' as ::core::ffi::c_int {
        buf as *const ::core::ffi::c_char
    } else {
        host
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_in_store(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if g.is_null() {
        return 1 as ::core::ffi::c_int;
    }
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    return (eng_guest_locate(
        g,
        object_path(
            host,
            &raw mut buf as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        ),
    ) == ENG_LOC_ROOTFS) as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_read(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut nofollow: ::core::ffi::c_int,
    mut st: *const stat,
    mut m: *mut eng_meta,
) -> ::core::ffi::c_int {
    memset(
        m as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<eng_meta>(),
    );
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut obj: *const ::core::ffi::c_char = object_path(
        host,
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    let mut loc: ::core::ffi::c_int = if !g.is_null() {
        eng_guest_locate(g, obj)
    } else {
        ENG_LOC_ROOTFS
    };
    if loc == ENG_LOC_ROOTFS && !((*st).st_mode & __S_IFMT as __mode_t == 0o120000 as __mode_t) {
        let mut val: [::core::ffi::c_char; 128] = [0; 128];
        let mut n: ssize_t = if nofollow != 0 {
            lgetxattr(
                host,
                ENG_META_XATTR.as_ptr(),
                &raw mut val as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_char; 128]>().wrapping_sub(1 as size_t),
            )
        } else {
            getxattr(
                host,
                ENG_META_XATTR.as_ptr(),
                &raw mut val as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_char; 128]>().wrapping_sub(1 as size_t),
            )
        };
        if n > 0 as ssize_t {
            val[n as usize] = 0 as ::core::ffi::c_char;
            if parse_meta(&raw mut val as *mut ::core::ffi::c_char, m) == 0 as ::core::ffi::c_int {
                if eng_meta_is_placeholder(m, (*st).st_mode) == 0 {
                    (*m).mode = (*st).st_mode as uint32_t & S_IFMT as uint32_t
                        | (*m).mode & 0o7777 as uint32_t;
                }
                return 0 as ::core::ffi::c_int;
            }
        }
    }
    if loc >= 0 as ::core::ffi::c_int {
        (*m).uid = (*g).binds[loc as usize].uid as uint32_t;
        (*m).gid = (*g).binds[loc as usize].gid as uint32_t;
    }
    if (*st).st_mode & __S_IFMT as __mode_t == 0o120000 as __mode_t {
        if loc == ENG_LOC_ROOTFS {
            let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
            snprintf(
                &raw mut dir as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                obj,
            );
            let mut sl: *mut ::core::ffi::c_char = strrchr(
                &raw mut dir as *mut ::core::ffi::c_char,
                '/' as ::core::ffi::c_int,
            );
            if !sl.is_null() && sl != &raw mut dir as *mut ::core::ffi::c_char {
                *sl = 0 as ::core::ffi::c_char;
                let mut dst: stat = stat {
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
                };
                let mut dm: eng_meta = eng_meta {
                    uid: 0,
                    gid: 0,
                    mode: 0,
                    nlink: 0,
                    major: 0,
                    minor: 0,
                    present: 0,
                };
                if lstat(&raw mut dir as *mut ::core::ffi::c_char, &raw mut dst)
                    == 0 as ::core::ffi::c_int
                    && !(dst.st_mode & __S_IFMT as __mode_t == 0o120000 as __mode_t)
                    && eng_meta_read(
                        g,
                        &raw mut dir as *mut ::core::ffi::c_char,
                        1 as ::core::ffi::c_int,
                        &raw mut dst,
                        &raw mut dm,
                    ) == 0 as ::core::ffi::c_int
                {
                    (*m).uid = dm.uid;
                    (*m).gid = dm.gid;
                }
            }
        }
        (*m).mode = (S_IFLNK | 0o777 as ::core::ffi::c_int) as uint32_t;
    } else {
        (*m).mode =
            ((*st).st_mode & (S_IFMT | 0o777 as ::core::ffi::c_int) as __mode_t) as uint32_t;
    }
    (*m).present = 0 as ::core::ffi::c_int;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_get(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut nofollow: ::core::ffi::c_int,
    mut st: *const stat,
    mut m: *mut eng_meta,
) -> ::core::ffi::c_int {
    return eng_meta_read(g, host, nofollow, st, m);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_write(
    mut host: *const ::core::ffi::c_char,
    mut nofollow: ::core::ffi::c_int,
    mut m: *const eng_meta,
) -> ::core::ffi::c_int {
    let mut buf: [::core::ffi::c_char; 128] = [0; 128];
    let mut n: ::core::ffi::c_int = snprintf(
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
    let mut rc: ::core::ffi::c_int = if nofollow != 0 {
        lsetxattr(
            host,
            ENG_META_XATTR.as_ptr(),
            &raw mut buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            n as size_t,
            0 as ::core::ffi::c_int,
        )
    } else {
        setxattr(
            host,
            ENG_META_XATTR.as_ptr(),
            &raw mut buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            n as size_t,
            0 as ::core::ffi::c_int,
        )
    };
    return if rc == 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_int
    } else {
        -*__errno_location()
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_is_placeholder(
    mut m: *const eng_meta,
    mut host_mode: mode_t,
) -> ::core::ffi::c_int {
    let mut vt: uint32_t = (*m).mode & S_IFMT as uint32_t;
    return (host_mode & __S_IFMT as mode_t == 0o100000 as mode_t
        && (vt == S_IFCHR as uint32_t
            || vt == S_IFBLK as uint32_t
            || vt == S_IFIFO as uint32_t
            || vt == S_IFSOCK as uint32_t)) as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_fifo_path(
    mut g: *mut eng_guest,
    mut ph: *const stat,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        ENG_STORE_GUEST.as_ptr(),
    );
    mkdir(&raw mut dir as *mut ::core::ffi::c_char, 0o700 as __mode_t);
    snprintf(
        &raw mut dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s%s/fifo\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        ENG_STORE_GUEST.as_ptr(),
    );
    mkdir(&raw mut dir as *mut ::core::ffi::c_char, 0o700 as __mode_t);
    let mut n: ::core::ffi::c_int = snprintf(
        out,
        cap,
        b"%s/%llx-%llx\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut dir as *mut ::core::ffi::c_char,
        (*ph).st_dev as ::core::ffi::c_ulonglong,
        (*ph).st_ino as ::core::ffi::c_ulonglong,
    );
    if n <= 0 as ::core::ffi::c_int || n as size_t >= cap {
        return -ENAMETOOLONG;
    }
    if mkfifo(out, 0o600 as __mode_t) != 0 as ::core::ffi::c_int && *__errno_location() != EEXIST {
        return -*__errno_location();
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_apply_stat(mut m: *const eng_meta, mut st: *mut stat) {
    (*st).st_uid = (*m).uid as __uid_t;
    (*st).st_gid = (*m).gid as __gid_t;
    let mut vt: uint32_t = (*m).mode & S_IFMT as uint32_t;
    if eng_meta_is_placeholder(m, (*st).st_mode) != 0 {
        (*st).st_mode = (*m).mode as __mode_t;
        (*st).st_rdev = if vt == S_IFCHR as uint32_t || vt == S_IFBLK as uint32_t {
            gnu_dev_makedev(
                (*m).major as ::core::ffi::c_uint,
                (*m).minor as ::core::ffi::c_uint,
            )
        } else {
            0 as __dev_t
        };
        (*st).st_size = 0 as __off_t;
    } else {
        (*st).st_mode = ((*st).st_mode as uint32_t & S_IFMT as uint32_t
            | (*m).mode & 0o7777 as uint32_t) as __mode_t;
    }
    if (*m).nlink != 0 {
        (*st).st_nlink = (*m).nlink as __nlink_t;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_apply_statx(
    mut m: *const eng_meta,
    mut stx: *mut ::core::ffi::c_void,
) {
    let mut b: *mut ::core::ffi::c_uchar = stx as *mut ::core::ffi::c_uchar;
    let mut v: uint32_t = 0;
    let mut mode: uint16_t = 0;
    memcpy(
        &raw mut mode as *mut ::core::ffi::c_void,
        b.offset(28 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
        2 as size_t,
    );
    memcpy(
        b.offset(20 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
        &raw const (*m).uid as *const ::core::ffi::c_void,
        4 as size_t,
    );
    memcpy(
        b.offset(24 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
        &raw const (*m).gid as *const ::core::ffi::c_void,
        4 as size_t,
    );
    let mut vt: uint32_t = (*m).mode & S_IFMT as uint32_t;
    if eng_meta_is_placeholder(m, mode as mode_t) != 0 {
        mode = (*m).mode as uint16_t;
        let mut zero32: uint32_t = 0 as uint32_t;
        let mut dev: ::core::ffi::c_int =
            (vt == S_IFCHR as uint32_t || vt == S_IFBLK as uint32_t) as ::core::ffi::c_int;
        memcpy(
            b.offset(128 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            (if dev != 0 {
                &raw const (*m).major
            } else {
                &raw mut zero32 as *const uint32_t
            }) as *const ::core::ffi::c_void,
            4 as size_t,
        );
        memcpy(
            b.offset(132 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            (if dev != 0 {
                &raw const (*m).minor
            } else {
                &raw mut zero32 as *const uint32_t
            }) as *const ::core::ffi::c_void,
            4 as size_t,
        );
        let mut zero: uint64_t = 0 as uint64_t;
        memcpy(
            b.offset(40 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            &raw mut zero as *const ::core::ffi::c_void,
            8 as size_t,
        );
    } else {
        mode = ((mode as ::core::ffi::c_int & S_IFMT) as uint32_t | (*m).mode & 0o7777 as uint32_t)
            as uint16_t;
    }
    memcpy(
        b.offset(28 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
        &raw mut mode as *const ::core::ffi::c_void,
        2 as size_t,
    );
    if (*m).nlink != 0 {
        v = (*m).nlink;
        memcpy(
            b.offset(16 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            &raw mut v as *const ::core::ffi::c_void,
            4 as size_t,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_in_group(
    mut t: *const eng_task,
    mut gid: uint32_t,
    mut use_real: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if if use_real != 0 {
        (*t).cr.rgid
    } else {
        (*t).cr.fsgid
    } == gid
    {
        return 1 as ::core::ffi::c_int;
    }
    let mut i: uint32_t = 0 as uint32_t;
    while i < (*t).cr.ngroups {
        if (*t).cr.groups[i as usize] == gid {
            return 1 as ::core::ffi::c_int;
        }
        i = i.wrapping_add(1);
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_permission(
    mut t: *mut eng_task,
    mut m: *const eng_meta,
    mut mask: ::core::ffi::c_int,
    mut use_real: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut uid: uint32_t = if use_real != 0 {
        (*t).cr.ruid
    } else {
        (*t).cr.fsuid
    };
    let mut caps: uint64_t = if use_real != 0 {
        if (*t).cr.ruid == 0 as uint32_t {
            (*t).cr.cap_prm
        } else {
            0 as uint64_t
        }
    } else {
        (*t).cr.cap_eff
    };
    let mut bits: ::core::ffi::c_uint = 0;
    if uid == (*m).uid {
        bits = ((*m).mode >> 6 as ::core::ffi::c_int & 7 as uint32_t) as ::core::ffi::c_uint;
    } else if eng_in_group(t, (*m).gid, use_real) != 0 {
        bits = ((*m).mode >> 3 as ::core::ffi::c_int & 7 as uint32_t) as ::core::ffi::c_uint;
    } else {
        bits = ((*m).mode & 7 as uint32_t) as ::core::ffi::c_uint;
    }
    let mut want: ::core::ffi::c_uint = (if mask & R_OK != 0 {
        4 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } | if mask & W_OK != 0 {
        2 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    } | if mask & X_OK != 0 {
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    }) as ::core::ffi::c_uint;
    if bits & want == want {
        return 0 as ::core::ffi::c_int;
    }
    let mut dir: ::core::ffi::c_int =
        ((*m).mode & __S_IFMT as uint32_t == 0o40000 as uint32_t) as ::core::ffi::c_int;
    if caps >> ENG_CAP_DAC_READ_SEARCH & 1 as uint64_t != 0 {
        if dir != 0 && mask & W_OK == 0 {
            return 0 as ::core::ffi::c_int;
        }
        if dir == 0 && want == 4 as ::core::ffi::c_uint {
            return 0 as ::core::ffi::c_int;
        }
    }
    if caps >> ENG_CAP_DAC_OVERRIDE & 1 as uint64_t != 0 {
        if dir != 0 || mask & X_OK == 0 || (*m).mode & 0o111 as uint32_t != 0 {
            return 0 as ::core::ffi::c_int;
        }
    }
    return -EACCES;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_may_exec(
    mut t: *mut eng_task,
    mut host: *const ::core::ffi::c_char,
    mut st: *const stat,
) -> ::core::ffi::c_int {
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
        host,
        0 as ::core::ffi::c_int,
        st,
        &raw mut m,
    );
    return eng_meta_permission(t, &raw mut m, X_OK, 0 as ::core::ffi::c_int);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_link_is_stub_text(
    mut text: *const ::core::ffi::c_char,
    mut id: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut pl: size_t =
        ::core::mem::size_of::<[::core::ffi::c_char; 25]>().wrapping_sub(1 as size_t);
    if strncmp(text, ENG_LINK_PREFIX.as_ptr(), pl) != 0 {
        return 0 as ::core::ffi::c_int;
    }
    let mut p: *const ::core::ffi::c_char = text.offset(pl as isize);
    let mut n: size_t = strlen(p);
    if n == 0 as size_t
        || n >= cap
        || !(strchr(p, '/' as ::core::ffi::c_int) as *const ::core::ffi::c_char).is_null()
    {
        return 0 as ::core::ffi::c_int;
    }
    memcpy(
        id as *mut ::core::ffi::c_void,
        p as *const ::core::ffi::c_void,
        n.wrapping_add(1 as size_t),
    );
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn store_dir(
    mut g: *mut eng_guest,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut n: ::core::ffi::c_int = snprintf(
        out,
        cap,
        b"%s%s/links\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        ENG_STORE_GUEST.as_ptr(),
    );
    return if n > 0 as ::core::ffi::c_int && (n as size_t) < cap {
        0 as ::core::ffi::c_int
    } else {
        -ENAMETOOLONG
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_link_object_path(
    mut g: *mut eng_guest,
    mut id: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut n: ::core::ffi::c_int = snprintf(
        out,
        cap,
        b"%s%s/links/%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        ENG_STORE_GUEST.as_ptr(),
        id,
    );
    return if n > 0 as ::core::ffi::c_int && (n as size_t) < cap {
        0 as ::core::ffi::c_int
    } else {
        -ENAMETOOLONG
    };
}
static mut g_meta_lock_fd: ::core::ffi::c_int = -1 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_lock(mut g: *mut eng_guest) -> ::core::ffi::c_int {
    if g.is_null() {
        return -1 as ::core::ffi::c_int;
    }
    if g_meta_lock_fd < 0 as ::core::ffi::c_int {
        let mut top: [::core::ffi::c_char; 4096] = [0; 4096];
        let mut p: [::core::ffi::c_char; 4112] = [0; 4112];
        snprintf(
            &raw mut top as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut (*g).root as *mut ::core::ffi::c_char,
            ENG_STORE_GUEST.as_ptr(),
        );
        mkdir(&raw mut top as *mut ::core::ffi::c_char, 0o700 as __mode_t);
        snprintf(
            &raw mut p as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4112]>(),
            b"%s/meta.lock\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut top as *mut ::core::ffi::c_char,
        );
        g_meta_lock_fd = open(
            &raw mut p as *mut ::core::ffi::c_char,
            O_RDWR | O_CREAT | O_CLOEXEC,
            0o600 as ::core::ffi::c_int,
        );
        if g_meta_lock_fd < 0 as ::core::ffi::c_int {
            return -1 as ::core::ffi::c_int;
        }
    }
    while flock(g_meta_lock_fd, LOCK_EX) != 0 as ::core::ffi::c_int && *__errno_location() == EINTR
    {
    }
    return g_meta_lock_fd;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_meta_unlock(mut token: ::core::ffi::c_int) {
    if token >= 0 as ::core::ffi::c_int {
        flock(token, LOCK_UN);
    }
}
unsafe extern "C" fn store_lock(mut g: *mut eng_guest) -> ::core::ffi::c_int {
    let mut d: [::core::ffi::c_char; 4096] = [0; 4096];
    if store_dir(
        g,
        &raw mut d as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return -1 as ::core::ffi::c_int;
    }
    let mut fd: ::core::ffi::c_int = eng_meta_lock(g);
    mkdir(&raw mut d as *mut ::core::ffi::c_char, 0o700 as __mode_t);
    return fd;
}
unsafe extern "C" fn store_unlock(mut fd: ::core::ffi::c_int) {
    eng_meta_unlock(fd);
}
unsafe extern "C" fn crash_point(mut name: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    let mut c: *const ::core::ffi::c_char =
        getenv(b"WORKFLOW_ENGINE_CRASH_AT\0".as_ptr() as *const ::core::ffi::c_char);
    if !c.is_null() && strcmp(c, name) == 0 {
        eng_logf!(
            eng_log_level::ENG_LOG_WARN,
            b"crash injection at %s\0".as_ptr() as *const ::core::ffi::c_char,
            name,
        );
        _exit(99 as ::core::ffi::c_int);
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn obj_adjust(
    mut obj: *const ::core::ffi::c_char,
    mut delta: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut st: stat = stat {
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
    };
    if stat(obj, &raw mut st) != 0 as ::core::ffi::c_int {
        return -*__errno_location();
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
        0 as ::core::ffi::c_int,
        &raw mut st,
        &raw mut m,
    );
    if m.present == 0 {
        m.uid = 0 as uint32_t;
        m.gid = 0 as uint32_t;
        m.mode = st.st_mode as uint32_t;
    }
    let mut n: ::core::ffi::c_long = m.nlink as ::core::ffi::c_long + delta as ::core::ffi::c_long;
    if n < 0 as ::core::ffi::c_long {
        n = 0 as ::core::ffi::c_long;
    }
    m.nlink = n as uint32_t;
    return eng_meta_write(obj, 0 as ::core::ffi::c_int, &raw mut m);
}
unsafe extern "C" fn new_id(mut id: *mut ::core::ffi::c_char, mut cap: size_t) {
    let mut r: ::core::ffi::c_ulonglong = 0 as ::core::ffi::c_ulonglong;
    let mut fd: ::core::ffi::c_int = open(
        b"/dev/urandom\0".as_ptr() as *const ::core::ffi::c_char,
        O_RDONLY | O_CLOEXEC,
    );
    if fd >= 0 as ::core::ffi::c_int {
        if read(
            fd,
            &raw mut r as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<::core::ffi::c_ulonglong>(),
        ) != ::core::mem::size_of::<::core::ffi::c_ulonglong>() as ssize_t
        {
            r = 0 as ::core::ffi::c_ulonglong;
        }
        close(fd);
    }
    let mut ts: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    clock_gettime(CLOCK_REALTIME, &raw mut ts);
    r ^= (ts.tv_nsec as ::core::ffi::c_ulonglong) << 20 as ::core::ffi::c_int
        ^ ts.tv_sec as ::core::ffi::c_ulonglong
        ^ (getpid() as ::core::ffi::c_ulonglong) << 40 as ::core::ffi::c_int;
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
) -> ::core::ffi::c_int {
    let mut st: stat = stat {
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
    };
    if lstat(old_entry, &raw mut st) != 0 as ::core::ffi::c_int {
        return -*__errno_location();
    }
    let mut nst: stat = stat {
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
    };
    if lstat(new_entry, &raw mut nst) == 0 as ::core::ffi::c_int {
        return -EEXIST;
    }
    let mut lk: ::core::ffi::c_int = store_lock(g);
    if lk < 0 as ::core::ffi::c_int {
        return -EIO;
    }
    let mut id: [::core::ffi::c_char; 64] = [0; 64];
    let mut obj: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut text: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut sd: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut rc: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    store_dir(
        g,
        &raw mut sd as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    '_out: {
        if st.st_mode & __S_IFMT as __mode_t == 0o120000 as __mode_t {
            let mut t: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut n: ssize_t = readlink(
                old_entry,
                &raw mut t as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as size_t),
            );
            if n < 0 as ssize_t {
                rc = -*__errno_location();
                break '_out;
            } else {
                t[n as usize] = 0 as ::core::ffi::c_char;
                if eng_link_is_stub_text(
                    &raw mut t as *mut ::core::ffi::c_char,
                    &raw mut id as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                ) == 0
                {
                    rc = if symlink(&raw mut t as *mut ::core::ffi::c_char, new_entry)
                        == 0 as ::core::ffi::c_int
                    {
                        0 as ::core::ffi::c_int
                    } else {
                        -*__errno_location()
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
        } else if st.st_mode & __S_IFMT as __mode_t == 0o100000 as __mode_t
            || st.st_mode & __S_IFMT as __mode_t == 0o10000 as __mode_t
            || st.st_mode & __S_IFMT as __mode_t == 0o140000 as __mode_t
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
            let mut jf: ::core::ffi::c_int = open(
                &raw mut jpath as *mut ::core::ffi::c_char,
                O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC,
                0o600 as ::core::ffi::c_int,
            );
            if jf < 0 as ::core::ffi::c_int {
                rc = -*__errno_location();
                break '_out;
            } else {
                let mut l: size_t = strlen(old_entry);
                if write(jf, old_entry as *const ::core::ffi::c_void, l) != l as ssize_t
                    || fsync(jf) != 0 as ::core::ffi::c_int
                {
                    rc = -EIO;
                    close(jf);
                    unlink(&raw mut jpath as *mut ::core::ffi::c_char);
                    break '_out;
                } else {
                    close(jf);
                    crash_point(b"link-journaled\0".as_ptr() as *const ::core::ffi::c_char);
                    if rename(old_entry, &raw mut obj as *mut ::core::ffi::c_char)
                        != 0 as ::core::ffi::c_int
                    {
                        rc = -*__errno_location();
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
                        if symlink(&raw mut text as *mut ::core::ffi::c_char, old_entry)
                            != 0 as ::core::ffi::c_int
                        {
                            rc = -*__errno_location();
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
                                0 as ::core::ffi::c_int,
                                &raw mut st,
                                &raw mut m,
                            );
                            if m.present == 0 {
                                m.mode = st.st_mode as uint32_t;
                            }
                            m.nlink = 1 as uint32_t;
                            eng_meta_write(
                                &raw mut obj as *mut ::core::ffi::c_char,
                                0 as ::core::ffi::c_int,
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
        rc = obj_adjust(
            &raw mut obj as *mut ::core::ffi::c_char,
            1 as ::core::ffi::c_int,
        );
        if rc == 0 {
            crash_point(b"link-counted\0".as_ptr() as *const ::core::ffi::c_char);
            snprintf(
                &raw mut text as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                ENG_LINK_PREFIX.as_ptr(),
                &raw mut id as *mut ::core::ffi::c_char,
            );
            if symlink(&raw mut text as *mut ::core::ffi::c_char, new_entry)
                != 0 as ::core::ffi::c_int
            {
                rc = -*__errno_location();
                obj_adjust(
                    &raw mut obj as *mut ::core::ffi::c_char,
                    -1 as ::core::ffi::c_int,
                );
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
    let mut lk: ::core::ffi::c_int = store_lock(g);
    let mut st: stat = stat {
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
    };
    if stat(&raw mut obj as *mut ::core::ffi::c_char, &raw mut st) == 0 as ::core::ffi::c_int {
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
            0 as ::core::ffi::c_int,
            &raw mut st,
            &raw mut m,
        );
        if m.nlink <= 1 as uint32_t {
            unlink(&raw mut obj as *mut ::core::ffi::c_char);
        } else {
            obj_adjust(
                &raw mut obj as *mut ::core::ffi::c_char,
                -1 as ::core::ffi::c_int,
            );
        }
    }
    store_unlock(lk);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_link_recover(mut g: *mut eng_guest) -> ::core::ffi::c_int {
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
        return 0 as ::core::ffi::c_int;
    }
    let mut lk: ::core::ffi::c_int = store_lock(g);
    let mut fixed: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut e: *mut dirent = ::core::ptr::null_mut::<dirent>();
    loop {
        e = readdir(d);
        if e.is_null() {
            break;
        }
        if strncmp(
            &raw mut (*e).d_name as *mut ::core::ffi::c_char,
            b"journal-\0".as_ptr() as *const ::core::ffi::c_char,
            8 as size_t,
        ) != 0
        {
            continue;
        }
        let mut id: *const ::core::ffi::c_char = (&raw mut (*e).d_name as *mut ::core::ffi::c_char)
            .offset(8 as ::core::ffi::c_int as isize);
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
        let mut jf: ::core::ffi::c_int = open(
            &raw mut jpath as *mut ::core::ffi::c_char,
            O_RDONLY | O_CLOEXEC,
        );
        if jf < 0 as ::core::ffi::c_int {
            continue;
        }
        let mut n: ssize_t = read(
            jf,
            &raw mut entry as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as size_t),
        );
        close(jf);
        if n <= 0 as ssize_t {
            unlink(&raw mut jpath as *mut ::core::ffi::c_char);
        } else {
            entry[n as usize] = 0 as ::core::ffi::c_char;
            eng_link_object_path(
                g,
                id,
                &raw mut obj as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            );
            let mut est: stat = stat {
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
            };
            let mut ost: stat = stat {
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
            };
            let mut have_entry: ::core::ffi::c_int =
                (lstat(&raw mut entry as *mut ::core::ffi::c_char, &raw mut est)
                    == 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
            let mut have_obj: ::core::ffi::c_int =
                (stat(&raw mut obj as *mut ::core::ffi::c_char, &raw mut ost)
                    == 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
            if have_entry == 0 && have_obj != 0 {
                rename(
                    &raw mut obj as *mut ::core::ffi::c_char,
                    &raw mut entry as *mut ::core::ffi::c_char,
                );
            } else if have_entry != 0
                && est.st_mode & __S_IFMT as __mode_t == 0o120000 as __mode_t
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
                    0 as ::core::ffi::c_int,
                    &raw mut ost,
                    &raw mut m,
                );
                if m.present == 0 {
                    m.mode = ost.st_mode as uint32_t;
                }
                if m.nlink == 0 as uint32_t {
                    m.nlink = 1 as uint32_t;
                    eng_meta_write(
                        &raw mut obj as *mut ::core::ffi::c_char,
                        0 as ::core::ffi::c_int,
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
pub unsafe extern "C" fn eng_instance_lock(
    mut g: *mut eng_guest,
    mut exclusive: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut top: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut p: [::core::ffi::c_char; 4128] = [0; 4128];
    snprintf(
        &raw mut top as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        ENG_STORE_GUEST.as_ptr(),
    );
    mkdir(&raw mut top as *mut ::core::ffi::c_char, 0o700 as __mode_t);
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4128]>(),
        b"%s/instances.lock\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut top as *mut ::core::ffi::c_char,
    );
    let mut fd: ::core::ffi::c_int = open(
        &raw mut p as *mut ::core::ffi::c_char,
        O_RDWR | O_CREAT | O_CLOEXEC,
        0o600 as ::core::ffi::c_int,
    );
    if fd < 0 as ::core::ffi::c_int {
        return -*__errno_location();
    }
    let mut op: ::core::ffi::c_int = if exclusive != 0 {
        LOCK_EX | LOCK_NB
    } else {
        LOCK_SH
    };
    while flock(fd, op) != 0 as ::core::ffi::c_int {
        if *__errno_location() == EINTR {
            continue;
        }
        let mut e: ::core::ffi::c_int = *__errno_location();
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
    let mut i: size_t = 0 as size_t;
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
        let mut nc: size_t = if g_fsck.cap != 0 {
            g_fsck.cap.wrapping_mul(2 as size_t)
        } else {
            64 as size_t
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
        0 as ::core::ffi::c_int,
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
unsafe extern "C" fn fsck_walk(mut dfd: ::core::ffi::c_int, mut depth: ::core::ffi::c_int) {
    if depth > 256 as ::core::ffi::c_int {
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
        if depth == 0 as ::core::ffi::c_int
            && strcmp(n, ENG_STORE_GUEST.as_ptr().offset(1isize)) == 0
        {
            continue;
        }
        let mut r#type: ::core::ffi::c_uchar = (*e).d_type;
        if r#type as ::core::ffi::c_int == C2Rust_Unnamed_0::DT_UNKNOWN.0 as ::core::ffi::c_int {
            let mut st: stat = stat {
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
            };
            if fstatat(dirfd(d), n, &raw mut st, AT_SYMLINK_NOFOLLOW) != 0 as ::core::ffi::c_int {
                continue;
            }
            r#type = (if st.st_mode & __S_IFMT as __mode_t == 0o40000 as __mode_t {
                C2Rust_Unnamed_0::DT_DIR.0 as ::core::ffi::c_int
            } else if st.st_mode & __S_IFMT as __mode_t == 0o120000 as __mode_t {
                C2Rust_Unnamed_0::DT_LNK.0 as ::core::ffi::c_int
            } else {
                C2Rust_Unnamed_0::DT_REG.0 as ::core::ffi::c_int
            }) as ::core::ffi::c_uchar;
        }
        if r#type as ::core::ffi::c_int == C2Rust_Unnamed_0::DT_DIR.0 as ::core::ffi::c_int {
            let mut fd: ::core::ffi::c_int =
                openat(dirfd(d), n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if fd >= 0 as ::core::ffi::c_int {
                fsck_walk(fd, depth + 1 as ::core::ffi::c_int);
            }
        } else {
            if r#type as ::core::ffi::c_int != C2Rust_Unnamed_0::DT_LNK.0 as ::core::ffi::c_int {
                continue;
            }
            let mut txt: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut id: [::core::ffi::c_char; 64] = [0; 64];
            let mut k: ssize_t = readlinkat(
                dirfd(d),
                n,
                &raw mut txt as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as size_t),
            );
            if k <= 0 as ssize_t {
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
    mut repair: ::core::ffi::c_int,
    mut outp: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
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
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<C2Rust_Unnamed>(),
    );
    let mut lk: ::core::ffi::c_int = store_lock(g);
    let mut d: *mut DIR = opendir(&raw mut sd as *mut ::core::ffi::c_char);
    if !d.is_null() {
        let mut e: *mut dirent = ::core::ptr::null_mut::<dirent>();
        loop {
            e = readdir(d);
            if e.is_null() {
                break;
            }
            if (*e).d_name[0usize] as ::core::ffi::c_int == '.' as ::core::ffi::c_int {
                continue;
            }
            if strncmp(
                &raw mut (*e).d_name as *mut ::core::ffi::c_char,
                b"journal-\0".as_ptr() as *const ::core::ffi::c_char,
                8 as size_t,
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
            let mut st: stat = stat {
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
            };
            if stat(&raw mut obj as *mut ::core::ffi::c_char, &raw mut st)
                != 0 as ::core::ffi::c_int
            {
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
                0 as ::core::ffi::c_int,
                &raw mut st,
                &raw mut m,
            );
            let mut fe: *mut fsck_ent = fsck_find(&raw mut (*e).d_name as *mut ::core::ffi::c_char);
            if !fe.is_null() {
                (*fe).have_obj = 1 as ::core::ffi::c_int;
                (*fe).nlink = m.nlink as ::core::ffi::c_uint;
            }
        }
        closedir(d);
    }
    let mut problems: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut rfd: ::core::ffi::c_int = open(
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        O_RDONLY | O_DIRECTORY | O_CLOEXEC,
    );
    if rfd < 0 as ::core::ffi::c_int {
        store_unlock(lk);
        free(g_fsck.v as *mut ::core::ffi::c_void);
        return -*__errno_location();
    }
    fsck_walk(rfd, 0 as ::core::ffi::c_int);
    let mut i: size_t = 0 as size_t;
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
            } else if (*e_0).names == 0 as ::core::ffi::c_uint {
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
                if !(repair != 0
                    && unlink(&raw mut obj_0 as *mut ::core::ffi::c_char)
                        == 0 as ::core::ffi::c_int)
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
                    let mut st_0: stat = stat {
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
                    };
                    let mut m_0: eng_meta = eng_meta {
                        uid: 0,
                        gid: 0,
                        mode: 0,
                        nlink: 0,
                        major: 0,
                        minor: 0,
                        present: 0,
                    };
                    if stat(&raw mut obj_0 as *mut ::core::ffi::c_char, &raw mut st_0)
                        == 0 as ::core::ffi::c_int
                        && eng_meta_read(
                            ::core::ptr::null_mut::<eng_guest>(),
                            &raw mut obj_0 as *mut ::core::ffi::c_char,
                            0 as ::core::ffi::c_int,
                            &raw mut st_0,
                            &raw mut m_0,
                        ) == 0 as ::core::ffi::c_int
                    {
                        if m_0.present == 0 {
                            m_0.mode = st_0.st_mode as uint32_t;
                        }
                        m_0.nlink = (*e_0).names as uint32_t;
                        if eng_meta_write(
                            &raw mut obj_0 as *mut ::core::ffi::c_char,
                            0 as ::core::ffi::c_int,
                            &raw mut m_0,
                        ) == 0 as ::core::ffi::c_int
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
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<C2Rust_Unnamed>(),
    );
    return problems;
}
