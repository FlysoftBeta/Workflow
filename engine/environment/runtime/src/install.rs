//! Streaming image validation, installation and generation lifecycle.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    unsafe fn closedir(_: *mut DIR) -> i32;
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
    static mut stderr: *mut FILE;
    unsafe fn renameat(
        _: i32,
        _: *const ::core::ffi::c_char,
        _: i32,
        _: *const ::core::ffi::c_char,
    ) -> i32;
    unsafe fn fclose(_: *mut FILE) -> i32;
    unsafe fn fopen(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> *mut FILE;
    unsafe fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> i32;
    unsafe fn printf(_: *const ::core::ffi::c_char, ...) -> i32;
    unsafe fn snprintf(
        _: *mut ::core::ffi::c_char,
        _: usize,
        _: *const ::core::ffi::c_char,
        ...
    ) -> i32;
    unsafe fn sscanf(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char, ...) -> i32;
    unsafe fn fputc(_: i32, _: *mut FILE) -> i32;
    unsafe fn fread(_: *mut ::core::ffi::c_void, _: usize, _: usize, _: *mut FILE) -> u64;
    unsafe fn strtoul(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
        _: i32,
    ) -> u64;
    unsafe fn strtoll(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
        _: i32,
    ) -> i64;
    unsafe fn strtoull(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
        _: i32,
    ) -> u64;
    unsafe fn malloc(_: usize) -> *mut ::core::ffi::c_void;
    unsafe fn calloc(_: usize, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn free(_: *mut ::core::ffi::c_void);
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: usize,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memset(_: *mut ::core::ffi::c_void, _: i32, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn memcmp(_: *const ::core::ffi::c_void, _: *const ::core::ffi::c_void, _: usize)
    -> i32;
    unsafe fn memchr(_: *const ::core::ffi::c_void, _: i32, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn strcmp(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn strncmp(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn strchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strrchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strspn(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> u64;
    unsafe fn strstr(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strtok_r(
        _: *mut ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(_: *const ::core::ffi::c_char) -> usize;
    unsafe fn strerror(_: i32) -> *mut ::core::ffi::c_char;
    #[cfg(target_os = "linux")]
    unsafe fn ioctl(_: i32, _: u64, ...) -> i32;
    #[cfg(target_os = "android")]
    unsafe fn ioctl(_: i32, _: u32, ...) -> i32;
    unsafe fn fstat(_: i32, _: *mut stat) -> i32;
    unsafe fn fstatat(_: i32, _: *const ::core::ffi::c_char, _: *mut stat, _: i32) -> i32;
    unsafe fn fchmod(_: i32, _: u32) -> i32;
    unsafe fn mkdir(_: *const ::core::ffi::c_char, _: u32) -> i32;
    unsafe fn mkdirat(_: i32, _: *const ::core::ffi::c_char, _: u32) -> i32;
    unsafe fn utimensat(_: i32, _: *const ::core::ffi::c_char, _: *const timespec, _: i32) -> i32;
    unsafe fn futimens(_: i32, _: *const timespec) -> i32;
    unsafe fn fstatvfs(_: i32, _: *mut statvfs) -> i32;
    unsafe fn fsetxattr(
        _: i32,
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_void,
        _: usize,
        _: i32,
    ) -> i32;
    unsafe fn fgetxattr(
        _: i32,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_void,
        _: usize,
    ) -> isize;
    unsafe fn flistxattr(_: i32, _: *mut ::core::ffi::c_char, _: usize) -> isize;
    unsafe fn clock_gettime(_: i32, _: *mut timespec) -> i32;
    unsafe fn close(_: i32) -> i32;
    unsafe fn read(_: i32, _: *mut ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn write(_: i32, _: *const ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn dup(_: i32) -> i32;
    unsafe fn symlinkat(
        _: *const ::core::ffi::c_char,
        _: i32,
        _: *const ::core::ffi::c_char,
    ) -> i32;
    unsafe fn readlinkat(
        _: i32,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> isize;
    unsafe fn unlinkat(_: i32, _: *const ::core::ffi::c_char, _: i32) -> i32;
    unsafe fn fsync(_: i32) -> i32;
    unsafe fn syncfs(_: i32) -> i32;
    unsafe fn eng_guest_open(_: *const ::core::ffi::c_char) -> *mut eng_guest;
    unsafe fn eng_guest_close(_: *mut eng_guest);
    unsafe fn eng_json_parse(_: *const ::core::ffi::c_char, _: usize) -> *mut eng_json;
    unsafe fn eng_json_free(_: *mut eng_json);
    unsafe fn eng_json_get(_: *const eng_json, _: *const ::core::ffi::c_char) -> *const eng_json;
    unsafe fn eng_json_str(
        _: *const eng_json,
        _: *const ::core::ffi::c_char,
    ) -> *const ::core::ffi::c_char;
    unsafe fn eng_json_int(_: *const eng_json, _: *const ::core::ffi::c_char, _: *mut i64) -> i32;
    unsafe fn eng_link_fsck(_: *mut eng_guest, _: i32, _: *mut ::core::ffi::c_void) -> i32;
    unsafe fn eng_sha256_init(_: *mut eng_sha256);
    unsafe fn eng_sha256_update(_: *mut eng_sha256, _: *const ::core::ffi::c_void, _: usize);
    unsafe fn eng_sha256_final(_: *mut eng_sha256, _: *mut u8);
    unsafe fn eng_sha256_hex(_: *const u8, _: *mut ::core::ffi::c_char);
    unsafe fn ZSTD_isError(_: usize) -> u32;
    unsafe fn ZSTD_createDCtx() -> *mut ZSTD_DCtx;
    unsafe fn ZSTD_freeDCtx(_: *mut ZSTD_DCtx) -> usize;
    unsafe fn ZSTD_decompressStream(
        _: *mut ZSTD_DStream,
        _: *mut ZSTD_outBuffer,
        _: *mut ZSTD_inBuffer,
    ) -> usize;
}
macro_rules! install_fail {
    ($code:expr, $fmt:expr $(,$arg:expr)* $(,)?) => {{
        snprintf(core::ptr::addr_of_mut!(g_err).cast(), 512, $fmt $(,$arg)*);
        $code
    }};
}
macro_rules! install_note {
    ($fmt:expr $(,$arg:expr)* $(,)?) => {{
        if g_quiet == 0 {
            crate::logging::dprintf(2, c"install: ".as_ptr());
            crate::logging::dprintf(2, $fmt $(,$arg)*);
            crate::logging::dprintf(2, c"\n".as_ptr());
        }
    }};
}
#[repr(C)]
pub struct ZSTD_DCtx_s {
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
#[cfg(target_arch = "x86_64")]
pub type __builtin_va_list = [__va_list_tag; 1];
#[cfg(target_arch = "aarch64")]
pub type __builtin_va_list = __va_list;
#[cfg(target_arch = "x86_64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __va_list_tag {
    pub gp_offset: u32,
    pub fp_offset: u32,
    pub overflow_arg_area: *mut ::core::ffi::c_void,
    pub reg_save_area: *mut ::core::ffi::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_install_opts {
    pub image: *const ::core::ffi::c_char,
    pub sha256: *const ::core::ffi::c_char,
    pub index: *const ::core::ffi::c_char,
    pub target: *const ::core::ffi::c_char,
    pub profile: *const ::core::ffi::c_char,
    pub quiet: i32,
    pub keep_partial: i32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct src_t {
    pub fd: i32,
    pub dctx: *mut ZSTD_DCtx,
    pub sha: eng_sha256,
    pub in_bytes: u64,
    pub r#in: [u8; 131072],
    pub zin: ZSTD_inBuffer,
    pub in_eof: i32,
    pub out: [u8; 131072],
    pub out_pos: usize,
    pub out_len: usize,
    pub last_ret: usize,
}
pub type ZSTD_inBuffer = ZSTD_inBuffer_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_inBuffer_s {
    pub src: *const ::core::ffi::c_void,
    pub size: usize,
    pub pos: usize,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_sha256 {
    pub h: [u32; 8],
    pub len: u64,
    pub buf: [u8; 64],
    pub n: usize,
}
pub type ZSTD_DCtx = ZSTD_DCtx_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct meta_t {
    pub rows: i64,
    pub size: i64,
    pub entries: i64,
    pub members: i64,
    pub regular_bytes: i64,
    pub attr_sha: [::core::ffi::c_char; 65],
    pub requires: u32,
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct attrs_t {
    pub rows: *mut row_t,
    pub n: usize,
    pub hash: *mut i32,
    pub hcap: usize,
    pub stores: [store_t; 8],
    pub nstores: i32,
    pub user_uid: u32,
    pub user_gid: u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct store_t {
    pub guest: [::core::ffi::c_char; 1024],
    pub store: [::core::ffi::c_char; 256],
    pub glen: usize,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct row_t {
    pub path: *mut ::core::ffi::c_char,
    pub plen: usize,
    pub r#type: ::core::ffi::c_char,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub extra: *mut ::core::ffi::c_char,
    pub major: u32,
    pub minor: u32,
    pub store: i32,
    pub primary: i32,
}
pub type va_list = __builtin_va_list;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ex_t {
    pub base_root: i32,
    pub base_seeds: i32,
    pub cache_dir: [::core::ffi::c_char; 4096],
    pub cache_fd: i32,
    pub cache_base: i32,
}
pub type ZSTD_outBuffer = ZSTD_outBuffer_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_outBuffer_s {
    pub dst: *mut ::core::ffi::c_void,
    pub size: usize,
    pub pos: usize,
}
pub type ZSTD_DStream = ZSTD_DCtx;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dtime_t {
    pub row: i32,
    pub mtime: i64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct member_t {
    pub path: [::core::ffi::c_char; 4096],
    pub link: [::core::ffi::c_char; 4096],
    pub r#type: ::core::ffi::c_char,
    pub size: u64,
    pub mtime: i64,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct statvfs {
    pub f_bsize: u64,
    pub f_frsize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_favail: u64,
    pub f_fsid: u64,
    pub f_flag: u64,
    pub f_namemax: u64,
    pub f_type: u32,
    pub __f_spare: [i32; 5],
}
#[cfg(target_os = "android")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct statvfs {
    pub f_bsize: u64,
    pub f_frsize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_favail: u64,
    pub f_fsid: u64,
    pub f_flag: u64,
    pub f_namemax: u64,
    pub __f_reserved: [u32; 6],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_json {
    pub t: eng_jtype,
    pub key: *mut ::core::ffi::c_char,
    pub s: *mut ::core::ffi::c_char,
    pub n: f64,
    pub b: i32,
    pub child: *mut eng_json,
    pub next: *mut eng_json,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_jtype(pub u32);
impl eng_jtype {
    pub const ENG_J_NULL: Self = Self(0);
    pub const ENG_J_BOOL: Self = Self(1);
    pub const ENG_J_NUM: Self = Self(2);
    pub const ENG_J_STR: Self = Self(3);
    pub const ENG_J_ARR: Self = Self(4);
    pub const ENG_J_OBJ: Self = Self(5);
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
pub const REQ_OWNERSHIP: u32 = 1;
pub const REQ_MODE: u32 = 2;
pub const REQ_HARDLINK: u32 = 4;
pub const REQ_SPECIAL: u32 = 8;
pub const REQ_SEEDS: u32 = 16;
pub const EPERM: i32 = 1 as i32;
pub const ENOENT: i32 = 2 as i32;
pub const EINTR: i32 = 4 as i32;
pub const EEXIST: i32 = 17 as i32;
pub const EISDIR: i32 = 21 as i32;
pub const EPIPE: i32 = 32 as i32;
pub const ELOOP: i32 = 40 as i32;
pub const EBADMSG: i32 = 74 as i32;
pub const O_RDONLY: i32 = 0 as i32;
pub const O_WRONLY: i32 = 0o1 as i32;
pub const O_CREAT: i32 = 0o100 as i32;
pub const O_EXCL: i32 = 0o200 as i32;
pub const O_NONBLOCK: i32 = 0o4000 as i32;
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
pub const AT_FDCWD: i32 = -100 as i32;
pub const AT_SYMLINK_NOFOLLOW: i32 = 0x100 as i32;
pub const AT_REMOVEDIR: i32 = 0x200 as i32;
pub const _IOC_NRBITS: i32 = 8 as i32;
pub const _IOC_TYPEBITS: i32 = 8 as i32;
pub const _IOC_SIZEBITS: i32 = 14 as i32;
pub const _IOC_NRSHIFT: i32 = 0 as i32;
pub const _IOC_TYPESHIFT: i32 = _IOC_NRSHIFT + _IOC_NRBITS;
pub const _IOC_SIZESHIFT: i32 = _IOC_TYPESHIFT + _IOC_TYPEBITS;
pub const _IOC_DIRSHIFT: i32 = _IOC_SIZESHIFT + _IOC_SIZEBITS;
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
pub const CLOCK_MONOTONIC: i32 = 1 as i32;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ENG_META_XATTR: [::core::ffi::c_char; 19] = unsafe {
    ::core::mem::transmute::<[u8; 19], [::core::ffi::c_char; 19]>(*b"user.workflow.meta\0")
};
#[cfg(target_arch = "x86_64")]
pub const HOST_DEB_ARCH: [::core::ffi::c_char; 6] =
    unsafe { ::core::mem::transmute::<[u8; 6], [::core::ffi::c_char; 6]>(*b"amd64\0") };
#[cfg(target_arch = "aarch64")]
pub const HOST_DEB_ARCH: [::core::ffi::c_char; 6] =
    unsafe { ::core::mem::transmute::<[u8; 6], [::core::ffi::c_char; 6]>(*b"arm64\0") };
pub const STORE_DIR: [::core::ffi::c_char; 17] = unsafe {
    ::core::mem::transmute::<[u8; 17], [::core::ffi::c_char; 17]>(*b".workflow-engine\0")
};
pub const LINKS_DIR: [::core::ffi::c_char; 23] = unsafe {
    ::core::mem::transmute::<[u8; 23], [::core::ffi::c_char; 23]>(*b".workflow-engine/links\0")
};
static mut IMPLEMENTED: [*const ::core::ffi::c_char; 5] = [
    b"virtual-ownership\0".as_ptr() as *const ::core::ffi::c_char,
    b"virtual-mode\0".as_ptr() as *const ::core::ffi::c_char,
    b"hardlink-emulation\0".as_ptr() as *const ::core::ffi::c_char,
    b"virtual-special-files\0".as_ptr() as *const ::core::ffi::c_char,
    b"store-seeds\0".as_ptr() as *const ::core::ffi::c_char,
];
static mut g_quiet: i32 = 0;
static mut g_err: [::core::ffi::c_char; 512] = [0; 512];
pub const S_IFLNK: i32 = 0o120000 as i32;
pub const S_IFMT: i32 = 0o170000 as i32;
unsafe extern "C" fn src_fill(mut s: *mut src_t) -> isize {
    (*s).out_len = 0 as usize;
    (*s).out_pos = (*s).out_len;
    loop {
        if (*s).zin.pos == (*s).zin.size && (*s).in_eof == 0 {
            let mut n: isize = 0;
            loop {
                n = read(
                    (*s).fd,
                    &raw mut (*s).r#in as *mut u8 as *mut ::core::ffi::c_void,
                    ::core::mem::size_of::<[u8; 131072]>(),
                );
                if !(n < 0 as isize && *errno() == EINTR) {
                    break;
                }
            }
            if n < 0 as isize {
                return -1 as isize;
            }
            if n == 0 as isize {
                (*s).in_eof = 1 as i32;
            } else {
                eng_sha256_update(
                    &raw mut (*s).sha,
                    &raw mut (*s).r#in as *mut u8 as *const ::core::ffi::c_void,
                    n as usize,
                );
                (*s).in_bytes = (*s).in_bytes.wrapping_add(n as u64);
                (*s).zin.src = &raw mut (*s).r#in as *mut u8 as *const ::core::ffi::c_void;
                (*s).zin.size = n as usize;
                (*s).zin.pos = 0 as usize;
            }
        }
        if (*s).zin.pos == (*s).zin.size && (*s).in_eof != 0 {
            return 0 as isize;
        }
        let mut zo: ZSTD_outBuffer = ZSTD_outBuffer_s {
            dst: &raw mut (*s).out as *mut u8 as *mut ::core::ffi::c_void,
            size: ::core::mem::size_of::<[u8; 131072]>(),
            pos: 0 as usize,
        };
        let mut r: usize = ZSTD_decompressStream((*s).dctx, &raw mut zo, &raw mut (*s).zin);
        if ZSTD_isError(r) != 0 {
            *errno() = EBADMSG;
            return -1 as isize;
        }
        (*s).last_ret = r;
        if zo.pos != 0 {
            (*s).out_len = zo.pos;
            return zo.pos as isize;
        }
    }
}
unsafe extern "C" fn src_read(
    mut s: *mut src_t,
    mut buf: *mut ::core::ffi::c_void,
    mut n: usize,
) -> i32 {
    let mut b: *mut u8 = buf as *mut u8;
    while n != 0 {
        if (*s).out_pos == (*s).out_len {
            let mut k: isize = src_fill(s);
            if k <= 0 as isize {
                if k == 0 as isize {
                    *errno() = EPIPE;
                }
                return -1 as i32;
            }
        }
        let mut take: usize = if (*s).out_len.wrapping_sub((*s).out_pos) < n {
            (*s).out_len.wrapping_sub((*s).out_pos)
        } else {
            n
        };
        if !b.is_null() {
            memcpy(
                b as *mut ::core::ffi::c_void,
                (&raw mut (*s).out as *mut u8).offset((*s).out_pos as isize)
                    as *const ::core::ffi::c_void,
                take,
            );
            b = b.offset(take as isize);
        }
        (*s).out_pos = (*s).out_pos.wrapping_add(take);
        n = n.wrapping_sub(take);
    }
    return 0 as i32;
}
unsafe extern "C" fn octal(
    mut p: *const ::core::ffi::c_char,
    mut n: usize,
    mut ok: *mut i32,
) -> u64 {
    let mut v: u64 = 0 as u64;
    let mut i: usize = 0 as usize;
    while i < n
        && (*p.offset(i as isize) as i32 == ' ' as i32 || *p.offset(i as isize) as i32 == 0 as i32)
    {
        i = i.wrapping_add(1);
    }
    while i < n
        && *p.offset(i as isize) as i32 >= '0' as i32
        && *p.offset(i as isize) as i32 <= '7' as i32
    {
        v = v
            .wrapping_mul(8 as u64)
            .wrapping_add((*p.offset(i as isize) as i32 - '0' as i32) as u64);
        i = i.wrapping_add(1);
    }
    while i < n {
        if *p.offset(i as isize) as i32 != ' ' as i32 && *p.offset(i as isize) as i32 != 0 as i32 {
            *ok = 0 as i32;
        }
        i = i.wrapping_add(1);
    }
    return v;
}
unsafe extern "C" fn valid_utf8(mut s: *const u8, mut n: usize) -> i32 {
    let mut i: usize = 0 as usize;
    while i < n {
        let mut c: u8 = *s.offset(i as isize);
        if (c as i32) < 0x80 as i32 {
            i = i.wrapping_add(1);
        } else {
            let mut k: usize = 0;
            let mut cp: u32 = 0;
            if c as i32 & 0xe0 as i32 == 0xc0 as i32 {
                k = 1 as usize;
                cp = (c as i32 & 0x1f as i32) as u32;
            } else if c as i32 & 0xf0 as i32 == 0xe0 as i32 {
                k = 2 as usize;
                cp = (c as i32 & 0xf as i32) as u32;
            } else if c as i32 & 0xf8 as i32 == 0xf0 as i32 {
                k = 3 as usize;
                cp = (c as i32 & 0x7 as i32) as u32;
            } else {
                return 0 as i32;
            }
            if i.wrapping_add(k) >= n {
                return 0 as i32;
            }
            let mut j: usize = 1 as usize;
            while j <= k {
                if *s.offset(i.wrapping_add(j) as isize) as i32 & 0xc0 as i32 != 0x80 as i32 {
                    return 0 as i32;
                }
                cp = cp << 6 as i32
                    | (*s.offset(i.wrapping_add(j) as isize) as i32 & 0x3f as i32) as u32;
                j = j.wrapping_add(1);
            }
            if k == 1 as usize && cp < 0x80 as u32
                || k == 2 as usize && cp < 0x800 as u32
                || k == 3 as usize && (cp < 0x10000 as i32 as u32 || cp > 0x10ffff as i32 as u32)
                || cp >= 0xd800 as u32 && cp <= 0xdfff as u32
            {
                return 0 as i32;
            }
            i = i.wrapping_add(k.wrapping_add(1 as usize));
        }
    }
    return 1 as i32;
}
unsafe extern "C" fn pax_apply(
    mut d: *mut ::core::ffi::c_char,
    mut n: usize,
    mut m: *mut member_t,
    mut has_size: *mut i32,
) -> i32 {
    let mut i: usize = 0 as usize;
    while i < n {
        let mut end: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
        let mut len: u64 = strtoul(d.offset(i as isize), &raw mut end, 10 as i32);
        if end == d.offset(i as isize)
            || *end as i32 != ' ' as i32
            || len < 5 as u64
            || i.wrapping_add(len as usize) > n
            || *d.offset(i.wrapping_add(len as usize).wrapping_sub(1 as usize) as isize) as i32
                != '\n' as i32
        {
            return -1 as i32;
        }
        let mut kv: *mut ::core::ffi::c_char = end.offset(1 as i32 as isize);
        let mut rec_end: *mut ::core::ffi::c_char = d
            .offset(i as isize)
            .offset(len as isize)
            .offset(-(1 as i32 as isize));
        let mut eq: *mut ::core::ffi::c_char = memchr(
            kv as *const ::core::ffi::c_void,
            '=' as i32,
            rec_end.offset_from(kv) as usize,
        ) as *mut ::core::ffi::c_char;
        if eq.is_null() {
            return -1 as i32;
        }
        *eq = 0 as ::core::ffi::c_char;
        *rec_end = 0 as ::core::ffi::c_char;
        let mut val: *const ::core::ffi::c_char = eq.offset(1 as i32 as isize);
        let mut vl: usize = rec_end.offset_from(val) as usize;
        if strcmp(kv, b"path\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            if vl >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
                || valid_utf8(val as *const u8, vl) == 0
            {
                return -1 as i32;
            }
            memcpy(
                &raw mut (*m).path as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                val as *const ::core::ffi::c_void,
                vl.wrapping_add(1 as usize),
            );
        } else if strcmp(kv, b"linkpath\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            if vl >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
                || valid_utf8(val as *const u8, vl) == 0
            {
                return -1 as i32;
            }
            memcpy(
                &raw mut (*m).link as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                val as *const ::core::ffi::c_void,
                vl.wrapping_add(1 as usize),
            );
        } else if strcmp(kv, b"size\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            (*m).size = strtoull(
                val,
                ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                10 as i32,
            ) as u64;
            *has_size = 1 as i32;
        } else if strcmp(kv, b"mtime\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            (*m).mtime = strtoll(
                val,
                ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                10 as i32,
            ) as i64;
        } else {
            return -1 as i32;
        }
        i = (i as u64).wrapping_add(len) as usize;
    }
    return 0 as i32;
}
unsafe extern "C" fn tar_next(mut s: *mut src_t, mut m: *mut member_t) -> i32 {
    let mut pax_path: [::core::ffi::c_char; 4096] = ::core::mem::transmute::<
        [u8; 4096],
        [::core::ffi::c_char; 4096],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    );
    let mut pax_link: [::core::ffi::c_char; 4096] = ::core::mem::transmute::<
        [u8; 4096],
        [::core::ffi::c_char; 4096],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    );
    let mut pax_size: u64 = 0 as u64;
    let mut pax_mtime: i64 = -1 as i64;
    let mut have_pax: i32 = 0 as i32;
    let mut pax_has_size: i32 = 0 as i32;
    loop {
        let mut h: [u8; 512] = [0; 512];
        if src_read(
            s,
            &raw mut h as *mut u8 as *mut ::core::ffi::c_void,
            512 as usize,
        ) != 0
        {
            return -1 as i32;
        }
        let mut zero: i32 = 1 as i32;
        let mut i: i32 = 0 as i32;
        while i < 512 as i32 {
            if h[i as usize] != 0 {
                zero = 0 as i32;
                break;
            } else {
                i += 1;
            }
        }
        if zero != 0 {
            if src_read(
                s,
                &raw mut h as *mut u8 as *mut ::core::ffi::c_void,
                512 as usize,
            ) != 0
            {
                return -1 as i32;
            }
            let mut i_0: i32 = 0 as i32;
            while i_0 < 512 as i32 {
                if h[i_0 as usize] != 0 {
                    return -5 as i32;
                }
                i_0 += 1;
            }
            loop {
                if (*s).out_pos == (*s).out_len {
                    let mut k: isize = src_fill(s);
                    if k < 0 as isize {
                        return -1 as i32;
                    }
                    if k == 0 as isize {
                        break;
                    }
                }
                while (*s).out_pos < (*s).out_len {
                    if (*s).out[(*s).out_pos] != 0 {
                        return -5 as i32;
                    }
                    (*s).out_pos = (*s).out_pos.wrapping_add(1);
                }
            }
            return 0 as i32;
        }
        let mut sum: u32 = 0 as u32;
        let mut i_1: i32 = 0 as i32;
        while i_1 < 512 as i32 {
            sum = sum.wrapping_add(
                (if i_1 >= 148 as i32 && i_1 < 156 as i32 {
                    ' ' as i32
                } else {
                    h[i_1 as usize] as i32
                }) as u32,
            );
            i_1 += 1;
        }
        let mut ok: i32 = 1 as i32;
        let mut hsum: u32 = octal(
            (&raw mut h as *mut u8 as *mut ::core::ffi::c_char).offset(148 as i32 as isize),
            8 as usize,
            &raw mut ok,
        ) as u32;
        if ok == 0 || hsum != sum {
            return -2 as i32;
        }
        if memcmp(
            (&raw mut h as *mut u8).offset(257 as i32 as isize) as *const ::core::ffi::c_void,
            b"ustar\x0000\0".as_ptr() as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            8 as usize,
        ) != 0
        {
            return -2 as i32;
        }
        memset(
            m as *mut ::core::ffi::c_void,
            0 as i32,
            ::core::mem::size_of::<member_t>(),
        );
        (*m).r#type = h[156usize] as ::core::ffi::c_char;
        (*m).size = octal(
            (&raw mut h as *mut u8 as *mut ::core::ffi::c_char).offset(124 as i32 as isize),
            12 as usize,
            &raw mut ok,
        );
        (*m).mtime = octal(
            (&raw mut h as *mut u8 as *mut ::core::ffi::c_char).offset(136 as i32 as isize),
            12 as usize,
            &raw mut ok,
        ) as i64;
        if ok == 0 {
            return -2 as i32;
        }
        let mut name: [::core::ffi::c_char; 101] = [0; 101];
        let mut prefix: [::core::ffi::c_char; 156] = [0; 156];
        memcpy(
            &raw mut name as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            &raw mut h as *mut u8 as *const ::core::ffi::c_void,
            100 as usize,
        );
        name[100usize] = 0 as ::core::ffi::c_char;
        memcpy(
            &raw mut prefix as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            (&raw mut h as *mut u8).offset(345 as i32 as isize) as *const ::core::ffi::c_void,
            155 as usize,
        );
        prefix[155usize] = 0 as ::core::ffi::c_char;
        if prefix[0usize] != 0 {
            snprintf(
                &raw mut (*m).path as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut prefix as *mut ::core::ffi::c_char,
                &raw mut name as *mut ::core::ffi::c_char,
            );
        } else {
            snprintf(
                &raw mut (*m).path as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut name as *mut ::core::ffi::c_char,
            );
        }
        memcpy(
            &raw mut (*m).link as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            (&raw mut h as *mut u8).offset(157 as i32 as isize) as *const ::core::ffi::c_void,
            100 as usize,
        );
        (*m).link[100usize] = 0 as ::core::ffi::c_char;
        if (*m).r#type as i32 == 'x' as i32 {
            if have_pax != 0 || (*m).size > ((1 as i32) << 20 as i32) as u64 {
                return -2 as i32;
            }
            let mut n: usize = (*m).size as usize;
            let mut d: *mut ::core::ffi::c_char =
                malloc(n.wrapping_add(1 as usize)) as *mut ::core::ffi::c_char;
            if d.is_null()
                || src_read(s, d as *mut ::core::ffi::c_void, n) != 0
                || src_read(
                    s,
                    NULL,
                    (512 as usize)
                        .wrapping_sub(n.wrapping_rem(512 as usize))
                        .wrapping_rem(512 as usize),
                ) != 0
            {
                free(d as *mut ::core::ffi::c_void);
                return -1 as i32;
            }
            let mut px: member_t = member_t {
                path: [0; 4096],
                link: [0; 4096],
                r#type: 0,
                size: 0,
                mtime: 0,
            };
            memset(
                &raw mut px as *mut ::core::ffi::c_void,
                0 as i32,
                ::core::mem::size_of::<member_t>(),
            );
            px.mtime = -1 as i64;
            let mut rc: i32 = pax_apply(d, n, &raw mut px, &raw mut pax_has_size);
            free(d as *mut ::core::ffi::c_void);
            if rc != 0 {
                return -3 as i32;
            }
            snprintf(
                &raw mut pax_path as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut px.path as *mut ::core::ffi::c_char,
            );
            snprintf(
                &raw mut pax_link as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut px.link as *mut ::core::ffi::c_char,
            );
            pax_size = px.size;
            pax_mtime = px.mtime;
            have_pax = 1 as i32;
        } else {
            if (*m).r#type as i32 != '0' as i32
                && (*m).r#type as i32 != 0 as i32
                && (*m).r#type as i32 != '5' as i32
                && (*m).r#type as i32 != '2' as i32
            {
                return -4 as i32;
            }
            if (*m).r#type as i32 == 0 as i32 {
                (*m).r#type = '0' as ::core::ffi::c_char;
            }
            if have_pax != 0 {
                if pax_path[0usize] != 0 {
                    snprintf(
                        &raw mut (*m).path as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut pax_path as *mut ::core::ffi::c_char,
                    );
                }
                if pax_link[0usize] != 0 {
                    snprintf(
                        &raw mut (*m).link as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut pax_link as *mut ::core::ffi::c_char,
                    );
                }
                if pax_has_size != 0 {
                    (*m).size = pax_size;
                }
                if pax_mtime >= 0 as i64 {
                    (*m).mtime = pax_mtime;
                }
            }
            let mut pl: usize = strlen(&raw mut (*m).path as *mut ::core::ffi::c_char);
            if (*m).r#type as i32 == '5' as i32
                && pl > 1 as usize
                && (*m).path[pl.wrapping_sub(1 as usize)] as i32 == '/' as i32
            {
                (*m).path[pl.wrapping_sub(1 as usize)] = 0 as ::core::ffi::c_char;
            }
            if valid_utf8(
                &raw mut (*m).path as *mut ::core::ffi::c_char as *mut u8,
                strlen(&raw mut (*m).path as *mut ::core::ffi::c_char),
            ) == 0
            {
                return -3 as i32;
            }
            if (*m).r#type as i32 != '0' as i32 && (*m).size != 0 {
                return -2 as i32;
            }
            return 1 as i32;
        }
    }
}
unsafe extern "C" fn read_member_data(
    mut s: *mut src_t,
    mut m: *const member_t,
    mut out: *mut *mut ::core::ffi::c_char,
    mut max: usize,
) -> i32 {
    if (*m).size > max as u64 {
        return -1 as i32;
    }
    let mut n: usize = (*m).size as usize;
    let mut d: *mut ::core::ffi::c_char =
        malloc(n.wrapping_add(1 as usize)) as *mut ::core::ffi::c_char;
    if d.is_null() {
        return -1 as i32;
    }
    if src_read(s, d as *mut ::core::ffi::c_void, n) != 0
        || src_read(
            s,
            NULL,
            (512 as usize)
                .wrapping_sub(n.wrapping_rem(512 as usize))
                .wrapping_rem(512 as usize),
        ) != 0
    {
        free(d as *mut ::core::ffi::c_void);
        return -1 as i32;
    }
    *d.offset(n as isize) = 0 as ::core::ffi::c_char;
    *out = d;
    return 0 as i32;
}
unsafe extern "C" fn hstr(mut s: *const ::core::ffi::c_char, mut n: usize) -> u64 {
    let mut h: u64 = 1469598103934665603 as u64;
    let mut i: usize = 0 as usize;
    while i < n {
        h ^= *s.offset(i as isize) as u8 as u64;
        h = (h as u64).wrapping_mul(1099511628211 as u64) as u64;
        i = i.wrapping_add(1);
    }
    return h;
}
unsafe extern "C" fn find_row(
    mut a: *const attrs_t,
    mut p: *const ::core::ffi::c_char,
    mut n: usize,
) -> i32 {
    let mut i: usize = hstr(p, n) as usize & (*a).hcap.wrapping_sub(1 as usize);
    loop {
        let mut r: i32 = *(*a).hash.offset(i as isize);
        if r < 0 as i32 {
            return -1 as i32;
        }
        if (*(*a).rows.offset(r as isize)).plen == n
            && memcmp(
                (*(*a).rows.offset(r as isize)).path as *const ::core::ffi::c_void,
                p as *const ::core::ffi::c_void,
                n,
            ) == 0
        {
            return r;
        }
        i = i.wrapping_add(1 as usize) & (*a).hcap.wrapping_sub(1 as usize);
    }
}
unsafe extern "C" fn add_hash(mut a: *mut attrs_t, mut r: i32) {
    let mut i: usize = hstr(
        (*(*a).rows.offset(r as isize)).path,
        (*(*a).rows.offset(r as isize)).plen,
    ) as usize
        & (*a).hcap.wrapping_sub(1 as usize);
    loop {
        if *(*a).hash.offset(i as isize) < 0 as i32 {
            *(*a).hash.offset(i as isize) = r;
            return;
        }
        i = i.wrapping_add(1 as usize) & (*a).hcap.wrapping_sub(1 as usize);
    }
}
unsafe extern "C" fn unpct(
    mut r#in: *const ::core::ffi::c_char,
    mut n: usize,
    mut out: *mut *mut ::core::ffi::c_char,
    mut olen: *mut usize,
) -> i32 {
    let mut o: *mut ::core::ffi::c_char =
        malloc(n.wrapping_add(1 as usize)) as *mut ::core::ffi::c_char;
    if o.is_null() {
        return -1 as i32;
    }
    let mut k: usize = 0 as usize;
    let mut i: usize = 0 as usize;
    while i < n {
        let mut c: u8 = *r#in.offset(i as isize) as u8;
        if c as i32 == '%' as i32 {
            if i.wrapping_add(2 as usize) >= n {
                free(o as *mut ::core::ffi::c_void);
                return -1 as i32;
            }
            let mut v: i32 = 0 as i32;
            let mut j: i32 = 1 as i32;
            while j <= 2 as i32 {
                let mut h: ::core::ffi::c_char = *r#in.offset(i.wrapping_add(j as usize) as isize);
                v <<= 4 as i32;
                if h as i32 >= '0' as i32 && h as i32 <= '9' as i32 {
                    v |= h as i32 - '0' as i32;
                } else if h as i32 >= 'A' as i32 && h as i32 <= 'F' as i32 {
                    v |= h as i32 - 'A' as i32 + 10 as i32;
                } else {
                    free(o as *mut ::core::ffi::c_void);
                    return -1 as i32;
                }
                j += 1;
            }
            let c2rust_fresh3 = k;
            k = k.wrapping_add(1);
            *o.offset(c2rust_fresh3 as isize) = v as ::core::ffi::c_char;
            i = i.wrapping_add(2 as usize);
        } else if c as i32 >= 0x21 as i32 && c as i32 <= 0x7e as i32 {
            let c2rust_fresh4 = k;
            k = k.wrapping_add(1);
            *o.offset(c2rust_fresh4 as isize) = c as ::core::ffi::c_char;
        } else {
            free(o as *mut ::core::ffi::c_void);
            return -1 as i32;
        }
        i = i.wrapping_add(1);
    }
    *o.offset(k as isize) = 0 as ::core::ffi::c_char;
    *out = o;
    *olen = k;
    return 0 as i32;
}
unsafe extern "C" fn path_ok(mut p: *const ::core::ffi::c_char, mut n: usize) -> i32 {
    if n == 1 as usize && *p.offset(0isize) as i32 == '/' as i32 {
        return 1 as i32;
    }
    if n == 0
        || *p.offset(0isize) as i32 != '/' as i32
        || *p.offset(n.wrapping_sub(1 as usize) as isize) as i32 == '/' as i32
        || !memchr(p as *const ::core::ffi::c_void, 0 as i32, n).is_null()
    {
        return 0 as i32;
    }
    if valid_utf8(p as *const u8, n) == 0 {
        return 0 as i32;
    }
    let mut i: usize = 1 as usize;
    while i <= n {
        let mut j: usize = i;
        while j < n && *p.offset(j as isize) as i32 != '/' as i32 {
            j = j.wrapping_add(1);
        }
        let mut cl: usize = j.wrapping_sub(i);
        if cl == 0 as usize
            || cl == 1 as usize && *p.offset(i as isize) as i32 == '.' as i32
            || cl == 2 as usize
                && *p.offset(i as isize) as i32 == '.' as i32
                && *p.offset(i.wrapping_add(1 as usize) as isize) as i32 == '.' as i32
        {
            return 0 as i32;
        }
        i = j.wrapping_add(1 as usize);
    }
    return 1 as i32;
}
unsafe extern "C" fn parse_u32(
    mut s: *const ::core::ffi::c_char,
    mut max: u32,
    mut out: *mut u32,
) -> i32 {
    if *s == 0 {
        return -1 as i32;
    }
    let mut v: u64 = 0 as u64;
    while *s != 0 {
        if (*s as i32) < '0' as i32 || *s as i32 > '9' as i32 {
            return -1 as i32;
        }
        v = v
            .wrapping_mul(10 as u64)
            .wrapping_add((*s as i32 - '0' as i32) as u64);
        if v > max as u64 {
            return -1 as i32;
        }
        s = s.offset(1);
    }
    *out = v as u32;
    return 0 as i32;
}
unsafe extern "C" fn parse_attrs(
    mut text: *mut ::core::ffi::c_char,
    mut len: usize,
    mut a: *mut attrs_t,
) -> i32 {
    let hdr: [::core::ffi::c_char; 24] = ::core::mem::transmute::<
        [u8; 24],
        [::core::ffi::c_char; 24],
    >(*b"#workflow-attributes 1\n\0");
    if len < ::core::mem::size_of::<[::core::ffi::c_char; 24]>().wrapping_sub(1usize)
        || memcmp(
            text as *const ::core::ffi::c_void,
            &raw const hdr as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            ::core::mem::size_of::<[::core::ffi::c_char; 24]>().wrapping_sub(1 as usize),
        ) != 0
    {
        return install_fail!(
            65 as i32,
            b"attributes.tsv: bad header\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut cap: usize = 0 as usize;
    let mut i: usize = 0 as usize;
    while i < len {
        if *text.offset(i as isize) as i32 == '\n' as i32 {
            cap = cap.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    (*a).rows = calloc(
        cap.wrapping_add(1 as usize),
        ::core::mem::size_of::<row_t>(),
    ) as *mut row_t;
    (*a).hcap = 1 as usize;
    while (*a).hcap < cap.wrapping_mul(2 as usize).wrapping_add(2 as usize) {
        (*a).hcap <<= 1 as i32;
    }
    (*a).hash = malloc((*a).hcap.wrapping_mul(::core::mem::size_of::<i32>())) as *mut i32;
    if (*a).rows.is_null() || (*a).hash.is_null() {
        return install_fail!(
            74 as i32,
            b"out of memory\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut i_0: usize = 0 as usize;
    while i_0 < (*a).hcap {
        *(*a).hash.offset(i_0 as isize) = -1 as i32;
        i_0 = i_0.wrapping_add(1);
    }
    let mut p: *mut ::core::ffi::c_char = text
        .offset(::core::mem::size_of::<[::core::ffi::c_char; 24]>() as isize)
        .offset(-(1 as i32 as isize));
    let mut end: *mut ::core::ffi::c_char = text.offset(len as isize);
    if len != 0 && *text.offset(len.wrapping_sub(1 as usize) as isize) as i32 != '\n' as i32 {
        return install_fail!(
            65 as i32,
            b"attributes.tsv: missing final newline\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    while p < end {
        let mut nl: *mut ::core::ffi::c_char = memchr(
            p as *const ::core::ffi::c_void,
            '\n' as i32,
            end.offset_from(p) as usize,
        ) as *mut ::core::ffi::c_char;
        *nl = 0 as ::core::ffi::c_char;
        let mut f: [*mut ::core::ffi::c_char; 6] =
            [::core::ptr::null_mut::<::core::ffi::c_char>(); 6];
        let mut nf: i32 = 0 as i32;
        let mut q: *mut ::core::ffi::c_char = p;
        while nf < 6 as i32 {
            let c2rust_fresh2 = nf;
            nf += 1;
            f[c2rust_fresh2 as usize] = q;
            let mut tab: *mut ::core::ffi::c_char = strchr(q, '\t' as i32);
            if tab.is_null() {
                break;
            }
            *tab = 0 as ::core::ffi::c_char;
            q = tab.offset(1 as i32 as isize);
        }
        let mut idx: usize = (*a).n;
        if nf != 6 as i32 || !strchr(f[5usize], '\t' as i32).is_null() {
            return install_fail!(
                65 as i32,
                b"attributes.tsv row %zu: need 6 fields\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as usize),
            );
        }
        let mut r: *mut row_t = (*a).rows.offset(idx as isize);
        if unpct(
            f[0usize],
            strlen(f[0usize]),
            &raw mut (*r).path,
            &raw mut (*r).plen,
        ) != 0
            || path_ok((*r).path, (*r).plen) == 0
        {
            return install_fail!(
                65 as i32,
                b"attributes.tsv row %zu: bad path\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as usize),
            );
        }
        if strlen(f[1usize]) != 1 as usize
            || strchr(
                b"dflhcbp\0".as_ptr() as *const ::core::ffi::c_char,
                *f[1usize].offset(0isize) as i32,
            )
            .is_null()
        {
            return install_fail!(
                65 as i32,
                b"row %zu: bad type\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as usize),
            );
        }
        (*r).r#type = *f[1usize].offset(0isize);
        if parse_u32(f[2usize], 4294967294 as u32, &raw mut (*r).uid) != 0
            || parse_u32(f[3usize], 4294967294 as u32, &raw mut (*r).gid) != 0
        {
            return install_fail!(
                65 as i32,
                b"row %zu: bad uid/gid\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as usize),
            );
        }
        if strlen(f[4usize]) != 4 as usize
            || strspn(
                f[4usize],
                b"01234567\0".as_ptr() as *const ::core::ffi::c_char,
            ) != 4 as u64
        {
            return install_fail!(
                65 as i32,
                b"row %zu: bad mode\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as usize),
            );
        }
        (*r).mode = strtoul(
            f[4usize],
            ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
            8 as i32,
        ) as u32;
        if (*r).r#type as i32 == 'l' as i32 && (*r).mode != 0o777 as u32 {
            return install_fail!(
                65 as i32,
                b"row %zu: symlink mode must be 0777\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as usize),
            );
        }
        (*r).store = -1 as i32;
        (*r).primary = -1 as i32;
        if (*r).r#type as i32 == 'h' as i32 {
            let mut el: usize = 0;
            if unpct(
                f[5usize],
                strlen(f[5usize]),
                &raw mut (*r).extra,
                &raw mut el,
            ) != 0
                || path_ok((*r).extra, el) == 0
            {
                return install_fail!(
                    65 as i32,
                    b"row %zu: bad hardlink primary\0".as_ptr() as *const ::core::ffi::c_char,
                    idx.wrapping_add(1 as usize),
                );
            }
        } else if (*r).r#type as i32 == 'c' as i32 || (*r).r#type as i32 == 'b' as i32 {
            if sscanf(
                f[5usize],
                b"%u,%u\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*r).major,
                &raw mut (*r).minor,
            ) != 2 as i32
            {
                return install_fail!(
                    65 as i32,
                    b"row %zu: bad device\0".as_ptr() as *const ::core::ffi::c_char,
                    idx.wrapping_add(1 as usize),
                );
            }
        } else if strcmp(f[5usize], b"-\0".as_ptr() as *const ::core::ffi::c_char) != 0 {
            return install_fail!(
                65 as i32,
                b"row %zu: extra must be '-'\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as usize),
            );
        }
        if idx == 0 as usize {
            if strcmp((*r).path, b"/\0".as_ptr() as *const ::core::ffi::c_char) != 0
                || (*r).r#type as i32 != 'd' as i32
            {
                return install_fail!(
                    65 as i32,
                    b"attributes.tsv: first row must be / (d)\0".as_ptr()
                        as *const ::core::ffi::c_char,
                );
            }
        } else {
            let mut pr: *const row_t = (*a).rows.offset(idx.wrapping_sub(1 as usize) as isize);
            let mut m: usize = if (*pr).plen < (*r).plen {
                (*pr).plen
            } else {
                (*r).plen
            };
            let mut c: i32 = memcmp(
                (*pr).path as *const ::core::ffi::c_void,
                (*r).path as *const ::core::ffi::c_void,
                m,
            );
            if c > 0 as i32 || c == 0 as i32 && (*pr).plen >= (*r).plen {
                return install_fail!(
                    65 as i32,
                    b"row %zu: rows not strictly sorted\0".as_ptr() as *const ::core::ffi::c_char,
                    idx.wrapping_add(1 as usize),
                );
            }
            let mut sl: *const ::core::ffi::c_char = strrchr((*r).path, '/' as i32);
            let mut plen: usize = if sl == (*r).path as *const ::core::ffi::c_char {
                1 as usize
            } else {
                sl.offset_from((*r).path) as usize
            };
            let mut pi: i32 = find_row(a, (*r).path, plen);
            if pi < 0 as i32 || (*(*a).rows.offset(pi as isize)).r#type as i32 != 'd' as i32 {
                return install_fail!(
                    65 as i32,
                    b"row %zu: parent is not a directory row\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    idx.wrapping_add(1 as usize),
                );
            }
        }
        add_hash(a, idx as i32);
        (*a).n = (*a).n.wrapping_add(1);
        p = nl.offset(1 as i32 as isize);
    }
    let mut i_1: usize = 0 as usize;
    while i_1 < (*a).n {
        let mut r_0: *mut row_t = (*a).rows.offset(i_1 as isize);
        if (*r_0).r#type as i32 == 'h' as i32 {
            let mut pi_0: i32 = find_row(a, (*r_0).extra, strlen((*r_0).extra));
            if pi_0 < 0 as i32
                || pi_0 as usize >= i_1
                || (*(*a).rows.offset(pi_0 as isize)).r#type as i32 != 'f' as i32
            {
                return install_fail!(
                    65 as i32,
                    b"row %zu: hardlink primary must be an earlier f row\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    i_1.wrapping_add(1 as usize),
                );
            }
            let mut pr_0: *const row_t = (*a).rows.offset(pi_0 as isize);
            if (*pr_0).uid != (*r_0).uid || (*pr_0).gid != (*r_0).gid || (*pr_0).mode != (*r_0).mode
            {
                return install_fail!(
                    65 as i32,
                    b"row %zu: hardlink attributes differ from the primary\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    i_1.wrapping_add(1 as usize),
                );
            }
            (*r_0).primary = pi_0;
        }
        i_1 = i_1.wrapping_add(1);
    }
    let mut s: i32 = 0 as i32;
    while s < (*a).nstores {
        let mut st: *mut store_t = (&raw mut (*a).stores as *mut store_t).offset(s as isize);
        let mut pi_1: i32 = find_row(
            a,
            &raw mut (*st).guest as *mut ::core::ffi::c_char,
            (*st).glen,
        );
        if pi_1 < 0 as i32 || (*(*a).rows.offset(pi_1 as isize)).r#type as i32 != 'd' as i32 {
            return install_fail!(
                65 as i32,
                b"store prefix %s is not a directory row\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*st).guest as *mut ::core::ffi::c_char,
            );
        }
        let mut i_2: usize = 0 as usize;
        while i_2 < (*a).n {
            let mut r_1: *mut row_t = (*a).rows.offset(i_2 as isize);
            let mut below: i32 = ((*r_1).plen > (*st).glen
                && memcmp(
                    (*r_1).path as *const ::core::ffi::c_void,
                    &raw mut (*st).guest as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                    (*st).glen,
                ) == 0
                && *(*r_1).path.offset((*st).glen as isize) as i32 == '/' as i32)
                as i32;
            let mut to_store: i32 = ((*r_1).r#type as i32 == 'h' as i32
                && strlen((*r_1).extra) > (*st).glen
                && memcmp(
                    (*r_1).extra as *const ::core::ffi::c_void,
                    &raw mut (*st).guest as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                    (*st).glen,
                ) == 0
                && *(*r_1).extra.offset((*st).glen as isize) as i32 == '/' as i32)
                as i32;
            if to_store != 0 {
                return install_fail!(
                    65 as i32,
                    b"row %zu: hardlink into store %s\0".as_ptr() as *const ::core::ffi::c_char,
                    i_2.wrapping_add(1 as usize),
                    &raw mut (*st).guest as *mut ::core::ffi::c_char,
                );
            }
            if below != 0 {
                if strchr(
                    b"dfl\0".as_ptr() as *const ::core::ffi::c_char,
                    (*r_1).r#type as i32,
                )
                .is_null()
                    || (*r_1).uid != (*a).user_uid
                    || (*r_1).gid != (*a).user_gid
                    || (*r_1).mode & 0o7000 as u32 != 0
                {
                    return install_fail!(
                        65 as i32,
                        b"row %zu: store entries must be d/f/l owned by the user without set-id\0"
                            .as_ptr() as *const ::core::ffi::c_char,
                        i_2.wrapping_add(1 as usize),
                    );
                }
                (*r_1).store = s;
            }
            i_2 = i_2.wrapping_add(1);
        }
        s += 1;
    }
    return 0 as i32;
}
unsafe extern "C" fn check_metadata(
    mut text: *const ::core::ffi::c_char,
    mut len: usize,
    mut profile: *const ::core::ffi::c_char,
    mut mt: *mut meta_t,
    mut a: *mut attrs_t,
) -> i32 {
    let mut req: *const eng_json = ::core::ptr::null::<eng_json>();
    let mut at: *const eng_json = ::core::ptr::null::<eng_json>();
    let mut rf: *const eng_json = ::core::ptr::null::<eng_json>();
    let mut user: *const eng_json = ::core::ptr::null::<eng_json>();
    let mut uu: i64 = 0;
    let mut ug: i64 = 0;
    let mut st: *const eng_json = ::core::ptr::null::<eng_json>();
    let mut j: *mut eng_json = eng_json_parse(text, len);
    if j.is_null() || (*j).t.0 != eng_jtype::ENG_J_OBJ.0 {
        eng_json_free(j);
        return install_fail!(
            65 as i32,
            b"metadata.json: not a JSON object\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut rc: i32 = 0 as i32;
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut v: i64 = 0;
    '_out: {
        s = eng_json_str(j, b"format\0".as_ptr() as *const ::core::ffi::c_char);
        if s.is_null()
            || strcmp(
                s,
                b"workflow-image\0".as_ptr() as *const ::core::ffi::c_char,
            ) != 0
        {
            rc = install_fail!(
                65 as i32,
                b"metadata: format is not workflow-image\0".as_ptr() as *const ::core::ffi::c_char,
            );
        } else if eng_json_int(
            j,
            b"formatVersion\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut v,
        ) != 0
            || v != 2 as i64
        {
            rc = install_fail!(
                65 as i32,
                b"metadata: formatVersion must be 2\0".as_ptr() as *const ::core::ffi::c_char,
            );
        } else {
            s = eng_json_str(j, b"type\0".as_ptr() as *const ::core::ffi::c_char);
            if s.is_null()
                || strcmp(s, b"debian-trixie\0".as_ptr() as *const ::core::ffi::c_char) != 0
            {
                rc = install_fail!(
                    65 as i32,
                    b"metadata: unknown image type\0".as_ptr() as *const ::core::ffi::c_char,
                );
            } else if eng_json_int(
                j,
                b"typeVersion\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut v,
            ) != 0
                || v != 1 as i64
            {
                rc = install_fail!(
                    65 as i32,
                    b"metadata: unsupported typeVersion\0".as_ptr() as *const ::core::ffi::c_char,
                );
            } else {
                s = eng_json_str(j, b"profile\0".as_ptr() as *const ::core::ffi::c_char);
                if s.is_null()
                    || strcmp(s, b"workspace\0".as_ptr() as *const ::core::ffi::c_char) != 0
                        && strcmp(s, b"base\0".as_ptr() as *const ::core::ffi::c_char) != 0
                {
                    rc = install_fail!(
                        65 as i32,
                        b"metadata: bad profile\0".as_ptr() as *const ::core::ffi::c_char,
                    );
                } else if strcmp(profile, b"any\0".as_ptr() as *const ::core::ffi::c_char) != 0
                    && strcmp(profile, s) != 0
                {
                    rc = install_fail!(
                        65 as i32,
                        b"metadata: profile %s, expected %s\0".as_ptr()
                            as *const ::core::ffi::c_char,
                        s,
                        profile,
                    );
                } else {
                    s = eng_json_str(j, b"architecture\0".as_ptr() as *const ::core::ffi::c_char);
                    if s.is_null() || strcmp(s, HOST_DEB_ARCH.as_ptr()) != 0 {
                        rc = install_fail!(
                            65 as i32,
                            b"metadata: architecture %s does not match this device (%s)\0".as_ptr()
                                as *const ::core::ffi::c_char,
                            if !s.is_null() {
                                s
                            } else {
                                b"?\0".as_ptr() as *const ::core::ffi::c_char
                            },
                            HOST_DEB_ARCH.as_ptr(),
                        );
                    } else {
                        req = eng_json_get(j, b"requires\0".as_ptr() as *const ::core::ffi::c_char);
                        if req.is_null() || (*req).t.0 != eng_jtype::ENG_J_ARR.0 {
                            rc = install_fail!(
                                65 as i32,
                                b"metadata: requires missing\0".as_ptr()
                                    as *const ::core::ffi::c_char,
                            );
                        } else {
                            let mut e: *const eng_json = (*req).child;
                            while !e.is_null() {
                                let mut known: i32 = 0 as i32;
                                let mut i: usize = 0 as usize;
                                while (*e).t.0 == eng_jtype::ENG_J_STR.0
                                    && i < ::core::mem::size_of::<[*const ::core::ffi::c_char; 5]>()
                                        .wrapping_div(::core::mem::size_of::<
                                            *const ::core::ffi::c_char,
                                        >())
                                {
                                    if strcmp((*e).s, IMPLEMENTED[i]) == 0 {
                                        known = 1 as i32;
                                        (*mt).requires |= (1 as u32) << i;
                                    }
                                    i = i.wrapping_add(1);
                                }
                                if known == 0 {
                                    rc = install_fail!(
                                        65 as i32,
                                        b"metadata: required capability %s is not implemented\0"
                                            .as_ptr()
                                            as *const ::core::ffi::c_char,
                                        if (*e).t.0 == eng_jtype::ENG_J_STR.0 {
                                            (*e).s as *const ::core::ffi::c_char
                                        } else {
                                            b"?\0".as_ptr() as *const ::core::ffi::c_char
                                        },
                                    );
                                    break '_out;
                                } else {
                                    e = (*e).next;
                                }
                            }
                            if (*mt).requires & (REQ_OWNERSHIP as i32 | REQ_MODE as i32) as u32
                                != (REQ_OWNERSHIP as i32 | REQ_MODE as i32) as u32
                            {
                                rc = install_fail!(
                                    65 as i32,
                                    b"metadata: requires must list virtual-ownership and virtual-mode\0"
                                        .as_ptr() as *const ::core::ffi::c_char,
                                );
                            } else {
                                at = eng_json_get(
                                    j,
                                    b"attributes\0".as_ptr() as *const ::core::ffi::c_char,
                                );
                                if at.is_null()
                                    || {
                                        s = eng_json_str(
                                            at,
                                            b"path\0".as_ptr() as *const ::core::ffi::c_char,
                                        );
                                        s.is_null()
                                    }
                                    || strcmp(
                                        s,
                                        b"attributes.tsv\0".as_ptr() as *const ::core::ffi::c_char,
                                    ) != 0
                                    || eng_json_int(
                                        at,
                                        b"version\0".as_ptr() as *const ::core::ffi::c_char,
                                        &raw mut v,
                                    ) != 0
                                    || v != 1 as i64
                                    || eng_json_int(
                                        at,
                                        b"rows\0".as_ptr() as *const ::core::ffi::c_char,
                                        &raw mut (*mt).rows,
                                    ) != 0
                                    || eng_json_int(
                                        at,
                                        b"size\0".as_ptr() as *const ::core::ffi::c_char,
                                        &raw mut (*mt).size,
                                    ) != 0
                                    || {
                                        s = eng_json_str(
                                            at,
                                            b"sha256\0".as_ptr() as *const ::core::ffi::c_char,
                                        );
                                        s.is_null()
                                    }
                                    || strlen(s) != 64 as usize
                                {
                                    rc = install_fail!(
                                        65 as i32,
                                        b"metadata: bad attributes descriptor\0".as_ptr()
                                            as *const ::core::ffi::c_char,
                                    );
                                } else {
                                    snprintf(
                                        &raw mut (*mt).attr_sha as *mut ::core::ffi::c_char,
                                        ::core::mem::size_of::<[::core::ffi::c_char; 65]>(),
                                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                                        s,
                                    );
                                    rf = eng_json_get(
                                        j,
                                        b"rootfs\0".as_ptr() as *const ::core::ffi::c_char,
                                    );
                                    if rf.is_null()
                                        || eng_json_int(
                                            rf,
                                            b"entries\0".as_ptr() as *const ::core::ffi::c_char,
                                            &raw mut (*mt).entries,
                                        ) != 0
                                        || eng_json_int(
                                            rf,
                                            b"members\0".as_ptr() as *const ::core::ffi::c_char,
                                            &raw mut (*mt).members,
                                        ) != 0
                                        || eng_json_int(
                                            rf,
                                            b"regularBytes\0".as_ptr()
                                                as *const ::core::ffi::c_char,
                                            &raw mut (*mt).regular_bytes,
                                        ) != 0
                                    {
                                        rc = install_fail!(
                                            65 as i32,
                                            b"metadata: bad rootfs descriptor\0".as_ptr()
                                                as *const ::core::ffi::c_char,
                                        );
                                    } else {
                                        user = eng_json_get(
                                            j,
                                            b"user\0".as_ptr() as *const ::core::ffi::c_char,
                                        );
                                        uu = 1000 as i64;
                                        ug = 1000 as i64;
                                        if !user.is_null() {
                                            eng_json_int(
                                                user,
                                                b"uid\0".as_ptr() as *const ::core::ffi::c_char,
                                                &raw mut uu,
                                            );
                                            eng_json_int(
                                                user,
                                                b"gid\0".as_ptr() as *const ::core::ffi::c_char,
                                                &raw mut ug,
                                            );
                                        }
                                        (*a).user_uid = uu as u32;
                                        (*a).user_gid = ug as u32;
                                        st = eng_json_get(
                                            j,
                                            b"stores\0".as_ptr() as *const ::core::ffi::c_char,
                                        );
                                        if !st.is_null() {
                                            if (*st).t.0 != eng_jtype::ENG_J_OBJ.0 {
                                                rc = install_fail!(
                                                    65 as i32,
                                                    b"metadata: stores must be an object\0".as_ptr()
                                                        as *const ::core::ffi::c_char,
                                                );
                                            } else {
                                                let mut e_0: *const eng_json = (*st).child;
                                                loop {
                                                    if e_0.is_null() {
                                                        break '_out;
                                                    }
                                                    let mut name: *const ::core::ffi::c_char =
                                                        eng_json_str(
                                                            e_0,
                                                            b"store\0".as_ptr()
                                                                as *const ::core::ffi::c_char,
                                                        );
                                                    let mut seed: *const ::core::ffi::c_char =
                                                        eng_json_str(
                                                            e_0,
                                                            b"seed\0".as_ptr()
                                                                as *const ::core::ffi::c_char,
                                                        );
                                                    if (*a).nstores >= 8 as i32
                                                        || name.is_null()
                                                        || seed.is_null()
                                                        || strcmp(
                                                            seed,
                                                            b"if-absent\0".as_ptr()
                                                                as *const ::core::ffi::c_char,
                                                        ) != 0
                                                            && strcmp(
                                                                seed,
                                                                b"merge\0".as_ptr()
                                                                    as *const ::core::ffi::c_char,
                                                            ) != 0
                                                        || path_ok((*e_0).key, strlen((*e_0).key))
                                                            == 0
                                                        || strcmp(
                                                            (*e_0).key,
                                                            b"/\0".as_ptr()
                                                                as *const ::core::ffi::c_char,
                                                        ) == 0
                                                        || *name.offset(0isize) as i32 == '/' as i32
                                                        || !strstr(
                                                            name,
                                                            b"..\0".as_ptr()
                                                                as *const ::core::ffi::c_char,
                                                        )
                                                        .is_null()
                                                        || strlen((*e_0).key)
                                                            >= ::core::mem::size_of::<
                                                                [::core::ffi::c_char; 1024],
                                                            >(
                                                            )
                                                        || strlen(name)
                                                            >= ::core::mem::size_of::<
                                                                [::core::ffi::c_char; 256],
                                                            >(
                                                            )
                                                    {
                                                        rc = install_fail!(
                                                            65 as i32,
                                                            b"metadata: bad store %s\0".as_ptr()
                                                                as *const ::core::ffi::c_char,
                                                            if !(*e_0).key.is_null() {
                                                                (*e_0).key
                                                                    as *const ::core::ffi::c_char
                                                            } else {
                                                                b"?\0".as_ptr()
                                                                    as *const ::core::ffi::c_char
                                                            },
                                                        );
                                                        break '_out;
                                                    } else {
                                                        let c2rust_fresh5 = (*a).nstores;
                                                        (*a).nstores += 1;
                                                        let mut sd: *mut store_t =
                                                            (&raw mut (*a).stores as *mut store_t)
                                                                .offset(c2rust_fresh5 as isize);
                                                        snprintf(
                                                            &raw mut (*sd).guest
                                                                as *mut ::core::ffi::c_char,
                                                            ::core::mem::size_of::<
                                                                [::core::ffi::c_char; 1024],
                                                            >(
                                                            ),
                                                            b"%s\0".as_ptr()
                                                                as *const ::core::ffi::c_char,
                                                            (*e_0).key,
                                                        );
                                                        snprintf(
                                                            &raw mut (*sd).store
                                                                as *mut ::core::ffi::c_char,
                                                            ::core::mem::size_of::<
                                                                [::core::ffi::c_char; 256],
                                                            >(
                                                            ),
                                                            b"%s\0".as_ptr()
                                                                as *const ::core::ffi::c_char,
                                                            name,
                                                        );
                                                        (*sd).glen = strlen(
                                                            &raw mut (*sd).guest
                                                                as *mut ::core::ffi::c_char,
                                                        );
                                                        e_0 = (*e_0).next;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    eng_json_free(j);
    return rc;
}
unsafe extern "C" fn open_dir(
    mut x: *mut ex_t,
    mut base: i32,
    mut dir: *const ::core::ffi::c_char,
) -> i32 {
    if (*x).cache_fd >= 0 as i32
        && (*x).cache_base == base
        && strcmp(&raw mut (*x).cache_dir as *mut ::core::ffi::c_char, dir) == 0
    {
        return (*x).cache_fd;
    }
    if (*x).cache_fd >= 0 as i32 {
        close((*x).cache_fd);
    }
    (*x).cache_fd = -1 as i32;
    let mut fd: i32 = dup(base);
    if fd < 0 as i32 {
        return -1 as i32;
    }
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        dir,
    );
    let mut save: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut c: *mut ::core::ffi::c_char = strtok_r(
        &raw mut buf as *mut ::core::ffi::c_char,
        b"/\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut save,
    );
    while !c.is_null() {
        let mut n: i32 = openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        if n < 0 as i32 {
            return -1 as i32;
        }
        fd = n;
        c = strtok_r(
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            b"/\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut save,
        );
    }
    snprintf(
        &raw mut (*x).cache_dir as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        dir,
    );
    (*x).cache_fd = fd;
    (*x).cache_base = base;
    return fd;
}
unsafe extern "C" fn fset_meta(
    mut fd: i32,
    mut uid: u32,
    mut gid: u32,
    mut mode: u32,
    mut nlink: u32,
    mut maj: u32,
    mut min: u32,
) -> i32 {
    let mut v: [::core::ffi::c_char; 128] = [0; 128];
    let mut n: i32 = snprintf(
        &raw mut v as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 128]>(),
        b"1 %u %u %o %u %u,%u\0".as_ptr() as *const ::core::ffi::c_char,
        uid,
        gid,
        mode,
        nlink,
        maj,
        min,
    );
    return fsetxattr(
        fd,
        ENG_META_XATTR.as_ptr(),
        &raw mut v as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
        n as usize,
        0 as i32,
    );
}
unsafe extern "C" fn locate(
    mut x: *const ex_t,
    mut a: *const attrs_t,
    mut r: *const row_t,
    mut base: *mut i32,
    mut dir: *mut ::core::ffi::c_char,
    mut dcap: usize,
) {
    let mut tmp: [::core::ffi::c_char; 4200] = [0; 4200];
    if (*r).store >= 0 as i32 {
        let mut st: *const store_t =
            (&raw const (*a).stores as *const store_t).offset((*r).store as isize);
        snprintf(
            &raw mut tmp as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4200]>(),
            b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw const (*st).store as *const ::core::ffi::c_char,
            (*r).path.offset((*st).glen as isize),
        );
        *base = (*x).base_seeds;
    } else {
        snprintf(
            &raw mut tmp as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4200]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            (*r).path.offset(1 as i32 as isize),
        );
        *base = (*x).base_root;
    }
    let mut sl: *mut ::core::ffi::c_char =
        strrchr(&raw mut tmp as *mut ::core::ffi::c_char, '/' as i32);
    if !sl.is_null() {
        *sl = 0 as ::core::ffi::c_char;
    } else {
        tmp[0usize] = 0 as ::core::ffi::c_char;
    }
    snprintf(
        dir,
        dcap,
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut tmp as *mut ::core::ffi::c_char,
    );
}
unsafe extern "C" fn mkdirs_rel(mut base: i32, mut rel: *const ::core::ffi::c_char) -> i32 {
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        rel,
    );
    let mut fd: i32 = dup(base);
    let mut save: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut c: *mut ::core::ffi::c_char = strtok_r(
        &raw mut buf as *mut ::core::ffi::c_char,
        b"/\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut save,
    );
    while !c.is_null() && fd >= 0 as i32 {
        if mkdirat(fd, c, 0o700 as u32) != 0 as i32 && *errno() != EEXIST {
            close(fd);
            return -1 as i32;
        }
        let mut n: i32 = openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        fd = n;
        c = strtok_r(
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            b"/\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut save,
        );
    }
    if fd < 0 as i32 {
        return -1 as i32;
    }
    close(fd);
    return 0 as i32;
}
unsafe extern "C" fn rm_tree_at(
    mut dfd: i32,
    mut name: *const ::core::ffi::c_char,
    mut depth: i32,
) -> i32 {
    if depth > 512 as i32 {
        return -ELOOP;
    }
    if unlinkat(dfd, name, 0 as i32) == 0 as i32 || *errno() == ENOENT {
        return 0 as i32;
    }
    if *errno() != EISDIR && *errno() != EPERM {
        return -*errno();
    }
    let mut fd: i32 = openat(dfd, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if fd < 0 as i32 {
        return -*errno();
    }
    fchmod(fd, 0o700 as u32);
    let mut d: *mut DIR = fdopendir(fd);
    if d.is_null() {
        close(fd);
        return -*errno();
    }
    let mut e: *mut dirent = ::core::ptr::null_mut::<dirent>();
    let mut rc: i32 = 0 as i32;
    loop {
        e = readdir(d);
        if e.is_null() {
            break;
        }
        if strcmp(
            &raw mut (*e).d_name as *mut ::core::ffi::c_char,
            b".\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
            || strcmp(
                &raw mut (*e).d_name as *mut ::core::ffi::c_char,
                b"..\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
        {
            continue;
        }
        let mut r: i32 = rm_tree_at(
            dirfd(d),
            &raw mut (*e).d_name as *mut ::core::ffi::c_char,
            depth + 1 as i32,
        );
        if r != 0 && rc == 0 {
            rc = r;
        }
    }
    closedir(d);
    if unlinkat(dfd, name, AT_REMOVEDIR) != 0 as i32 && rc == 0 {
        rc = -*errno();
    }
    return rc;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_remove_tree(mut path: *const ::core::ffi::c_char) -> i32 {
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        path,
    );
    let mut n: usize = strlen(&raw mut buf as *mut ::core::ffi::c_char);
    while n > 1 as usize && buf[n.wrapping_sub(1 as usize)] as i32 == '/' as i32 {
        n = n.wrapping_sub(1);
        buf[n] = 0 as ::core::ffi::c_char;
    }
    let mut sl: *mut ::core::ffi::c_char =
        strrchr(&raw mut buf as *mut ::core::ffi::c_char, '/' as i32);
    let mut dfd: i32 = 0;
    let mut name: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    if sl.is_null() {
        dfd = AT_FDCWD;
        name = &raw mut buf as *mut ::core::ffi::c_char;
    } else {
        *sl = 0 as ::core::ffi::c_char;
        dfd = open(
            if sl == &raw mut buf as *mut ::core::ffi::c_char {
                b"/\0".as_ptr() as *const ::core::ffi::c_char
            } else {
                &raw mut buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_char
            },
            O_RDONLY | O_DIRECTORY | O_CLOEXEC,
        );
        if dfd < 0 as i32 {
            return -*errno();
        }
        name = sl.offset(1 as i32 as isize);
    }
    let mut rc: i32 = rm_tree_at(dfd, name, 0 as i32);
    if dfd != AT_FDCWD {
        close(dfd);
    }
    return rc;
}
unsafe extern "C" fn now_s() -> f64 {
    let mut ts: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    clock_gettime(CLOCK_MONOTONIC, &raw mut ts);
    return ts.tv_sec as f64 + ts.tv_nsec as f64 / 1e9f64;
}
unsafe extern "C" fn do_install(
    mut o: *const eng_install_opts,
    mut s: *mut src_t,
    mut tfd: i32,
    mut mt: *mut meta_t,
    mut a: *mut attrs_t,
    mut stats: *mut i64,
) -> i32 {
    let mut m: member_t = member_t {
        path: [0; 4096],
        link: [0; 4096],
        r#type: 0,
        size: 0,
        mtime: 0,
    };
    let mut meta_text: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut attr_text: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut rc: i32 = 0;
    rc = tar_next(s, &raw mut m);
    if rc != 1 as i32
        || m.r#type as i32 != '0' as i32
        || strcmp(
            &raw mut m.path as *mut ::core::ffi::c_char,
            b"metadata.json\0".as_ptr() as *const ::core::ffi::c_char,
        ) != 0
    {
        return install_fail!(
            65 as i32,
            b"image: first member must be metadata.json\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    if read_member_data(
        s,
        &raw mut m,
        &raw mut meta_text,
        ((64 as i32) << 10 as i32) as usize,
    ) != 0
    {
        return install_fail!(
            65 as i32,
            b"image: metadata.json too large or truncated\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    rc = check_metadata(
        meta_text,
        m.size as usize,
        if !(*o).profile.is_null() {
            (*o).profile
        } else {
            b"workspace\0".as_ptr() as *const ::core::ffi::c_char
        },
        mt,
        a,
    );
    free(meta_text as *mut ::core::ffi::c_void);
    if rc != 0 {
        return rc;
    }
    if tar_next(s, &raw mut m) != 1 as i32
        || m.r#type as i32 != '0' as i32
        || strcmp(
            &raw mut m.path as *mut ::core::ffi::c_char,
            b"attributes.tsv\0".as_ptr() as *const ::core::ffi::c_char,
        ) != 0
    {
        return install_fail!(
            65 as i32,
            b"image: second member must be attributes.tsv\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    if m.size as i64 != (*mt).size {
        return install_fail!(
            65 as i32,
            b"attributes.tsv: size differs from metadata\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    if read_member_data(
        s,
        &raw mut m,
        &raw mut attr_text,
        ((64 as i32) << 20 as i32) as usize,
    ) != 0
    {
        return install_fail!(
            65 as i32,
            b"image: attributes.tsv too large or truncated\0".as_ptr()
                as *const ::core::ffi::c_char,
        );
    }
    let mut c: eng_sha256 = eng_sha256 {
        h: [0; 8],
        len: 0,
        buf: [0; 64],
        n: 0,
    };
    let mut d: [u8; 32] = [0; 32];
    let mut hex: [::core::ffi::c_char; 65] = [0; 65];
    eng_sha256_init(&raw mut c);
    eng_sha256_update(
        &raw mut c,
        attr_text as *const ::core::ffi::c_void,
        m.size as usize,
    );
    eng_sha256_final(&raw mut c, &raw mut d as *mut u8);
    eng_sha256_hex(
        &raw mut d as *mut u8 as *const u8,
        &raw mut hex as *mut ::core::ffi::c_char,
    );
    if strcmp(
        &raw mut hex as *mut ::core::ffi::c_char,
        &raw mut (*mt).attr_sha as *mut ::core::ffi::c_char,
    ) != 0
    {
        free(attr_text as *mut ::core::ffi::c_void);
        return install_fail!(
            65 as i32,
            b"attributes.tsv: sha256 mismatch\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    rc = parse_attrs(attr_text, m.size as usize, a);
    free(attr_text as *mut ::core::ffi::c_void);
    if rc != 0 {
        return rc;
    }
    if (*a).n as i64 != (*mt).rows || (*a).n as i64 != (*mt).entries {
        return install_fail!(
            65 as i32,
            b"attributes.tsv: row count differs from metadata\0".as_ptr()
                as *const ::core::ffi::c_char,
        );
    }
    let mut mem: i64 = 0 as i64;
    let mut hard: i64 = 0 as i64;
    let mut special: i64 = 0 as i64;
    let mut i: usize = 0 as usize;
    while i < (*a).n {
        let mut t: ::core::ffi::c_char = (*(*a).rows.offset(i as isize)).r#type;
        if t as i32 == 'd' as i32 || t as i32 == 'f' as i32 || t as i32 == 'l' as i32 {
            mem += 1;
        } else if t as i32 == 'h' as i32 {
            hard += 1;
        } else {
            special += 1;
        }
        i = i.wrapping_add(1);
    }
    if mem != (*mt).members {
        return install_fail!(
            65 as i32,
            b"attributes.tsv: %lld member rows, metadata says %lld\0".as_ptr()
                as *const ::core::ffi::c_char,
            mem,
            (*mt).members,
        );
    }
    if hard != 0 && (*mt).requires & REQ_HARDLINK == 0 {
        return install_fail!(
            65 as i32,
            b"metadata: hardlink rows without hardlink-emulation\0".as_ptr()
                as *const ::core::ffi::c_char,
        );
    }
    if special != 0 && (*mt).requires & REQ_SPECIAL == 0 {
        return install_fail!(
            65 as i32,
            b"metadata: special rows without virtual-special-files\0".as_ptr()
                as *const ::core::ffi::c_char,
        );
    }
    if (*a).nstores != 0 && (*mt).requires & REQ_SEEDS == 0 {
        return install_fail!(
            65 as i32,
            b"metadata: stores without store-seeds\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut vfs: statvfs = platform_empty_statvfs();
    if fstatvfs(tfd, &raw mut vfs) == 0 as i32 {
        let mut need: u64 = ((*mt).regular_bytes as u64)
            .wrapping_add(((*mt).entries as u64).wrapping_mul(4096 as u64));
        need = need.wrapping_add(need.wrapping_div(20 as u64));
        let mut have: u64 = (vfs.f_bavail as u64).wrapping_mul(vfs.f_frsize as u64);
        if have < need {
            return install_fail!(
                75 as i32,
                b"not enough space: need %llu MB, have %llu MB\0".as_ptr()
                    as *const ::core::ffi::c_char,
                need >> 20 as i32,
                have >> 20 as i32,
            );
        }
    }
    install_note!(
        b"image valid: %zu rows, %lld members, %lld MB\0".as_ptr() as *const ::core::ffi::c_char,
        (*a).n,
        (*mt).members,
        (*mt).regular_bytes >> 20 as i32,
    );
    let mut x: ex_t = ex_t {
        base_root: 0,
        base_seeds: 0,
        cache_dir: [0; 4096],
        cache_fd: -1 as i32,
        cache_base: 0,
    };
    if mkdirat(
        tfd,
        b"rootfs\0".as_ptr() as *const ::core::ffi::c_char,
        0o700 as u32,
    ) != 0
        || {
            x.base_root = openat(
                tfd,
                b"rootfs\0".as_ptr() as *const ::core::ffi::c_char,
                O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
            );
            x.base_root < 0 as i32
        }
    {
        return install_fail!(
            74 as i32,
            b"create rootfs: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*errno()),
        );
    }
    x.base_seeds = -1 as i32;
    if (*a).nstores != 0 {
        if mkdirat(
            tfd,
            b"seeds\0".as_ptr() as *const ::core::ffi::c_char,
            0o700 as u32,
        ) != 0
            || {
                x.base_seeds = openat(
                    tfd,
                    b"seeds\0".as_ptr() as *const ::core::ffi::c_char,
                    O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
                );
                x.base_seeds < 0 as i32
            }
        {
            return install_fail!(
                74 as i32,
                b"create seeds: %s\0".as_ptr() as *const ::core::ffi::c_char,
                strerror(*errno()),
            );
        }
        let mut i_0: i32 = 0 as i32;
        while i_0 < (*a).nstores {
            if mkdirs_rel(
                x.base_seeds,
                &raw mut (*(&raw mut (*a).stores as *mut store_t).offset(i_0 as isize)).store
                    as *mut ::core::ffi::c_char,
            ) != 0
            {
                return install_fail!(
                    74 as i32,
                    b"create seeds: %s\0".as_ptr() as *const ::core::ffi::c_char,
                    strerror(*errno()),
                );
            }
            i_0 += 1;
        }
    }
    let mut dts: *mut dtime_t = calloc((*a).n, ::core::mem::size_of::<dtime_t>()) as *mut dtime_t;
    let mut ndt: usize = 0 as usize;
    if dts.is_null() {
        return install_fail!(
            74 as i32,
            b"out of memory\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut ri: usize = 0 as usize;
    let mut members: i64 = 0 as i64;
    let mut seeds: i64 = 0 as i64;
    let mut regular: i64 = 0 as i64;
    let mut buf: *mut ::core::ffi::c_char =
        malloc(((1 as i32) << 20 as i32) as usize) as *mut ::core::ffi::c_char;
    if buf.is_null() {
        free(dts as *mut ::core::ffi::c_void);
        return install_fail!(
            74 as i32,
            b"out of memory\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut t0: f64 = now_s();
    let mut last: f64 = t0;
    rc = 0 as i32;
    loop {
        let mut k: i32 = tar_next(s, &raw mut m);
        if k == 0 as i32 {
            break;
        }
        if k < 0 as i32 {
            rc = install_fail!(
                65 as i32,
                b"image: %s after %lld members\0".as_ptr() as *const ::core::ffi::c_char,
                if k == -4 as i32 {
                    b"forbidden tar member type\0".as_ptr() as *const ::core::ffi::c_char
                } else if k == -3 as i32 {
                    b"bad PAX header or non-UTF-8 name\0".as_ptr() as *const ::core::ffi::c_char
                } else if k == -5 as i32 {
                    b"data after the end of the archive\0".as_ptr() as *const ::core::ffi::c_char
                } else if k == -1 as i32 {
                    b"truncated or corrupt stream\0".as_ptr() as *const ::core::ffi::c_char
                } else {
                    b"bad tar header\0".as_ptr() as *const ::core::ffi::c_char
                },
                members,
            );
            break;
        } else {
            while ri < (*a).n
                && strchr(
                    b"dfl\0".as_ptr() as *const ::core::ffi::c_char,
                    (*(*a).rows.offset(ri as isize)).r#type as i32,
                )
                .is_null()
            {
                ri = ri.wrapping_add(1);
            }
            if ri >= (*a).n {
                rc = install_fail!(
                    65 as i32,
                    b"image: member %s has no attributes row\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    &raw mut m.path as *mut ::core::ffi::c_char,
                );
                break;
            } else {
                let c2rust_fresh0 = ri;
                ri = ri.wrapping_add(1);
                let mut r: *mut row_t = (*a).rows.offset(c2rust_fresh0 as isize);
                let mut gp: [::core::ffi::c_char; 4200] = [0; 4200];
                if strcmp(
                    &raw mut m.path as *mut ::core::ffi::c_char,
                    b"rootfs\0".as_ptr() as *const ::core::ffi::c_char,
                ) == 0
                {
                    snprintf(
                        &raw mut gp as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4200]>(),
                        b"/\0".as_ptr() as *const ::core::ffi::c_char,
                    );
                } else if strncmp(
                    &raw mut m.path as *mut ::core::ffi::c_char,
                    b"rootfs/\0".as_ptr() as *const ::core::ffi::c_char,
                    7 as usize,
                ) == 0
                {
                    snprintf(
                        &raw mut gp as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4200]>(),
                        b"/%s\0".as_ptr() as *const ::core::ffi::c_char,
                        (&raw mut m.path as *mut ::core::ffi::c_char).offset(7 as i32 as isize),
                    );
                } else {
                    rc = install_fail!(
                        65 as i32,
                        b"image: member outside rootfs: %s\0".as_ptr()
                            as *const ::core::ffi::c_char,
                        &raw mut m.path as *mut ::core::ffi::c_char,
                    );
                    break;
                }
                let mut mt_: ::core::ffi::c_char = (if m.r#type as i32 == '5' as i32 {
                    'd' as i32
                } else if m.r#type as i32 == '2' as i32 {
                    'l' as i32
                } else {
                    'f' as i32
                }) as ::core::ffi::c_char;
                if strcmp(&raw mut gp as *mut ::core::ffi::c_char, (*r).path) != 0
                    || mt_ as i32 != (*r).r#type as i32
                {
                    rc = install_fail!(
                        65 as i32,
                        b"image: member %s does not match row %s (%c)\0".as_ptr()
                            as *const ::core::ffi::c_char,
                        &raw mut gp as *mut ::core::ffi::c_char,
                        (*r).path,
                        (*r).r#type as i32,
                    );
                    break;
                } else {
                    members += 1;
                    let mut is_seed: i32 = ((*r).store >= 0 as i32) as i32;
                    if strcmp((*r).path, b"/\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                        fchmod(x.base_root, (*r).mode as u32 & 0o777 as u32 | 0o700 as u32);
                        if fset_meta(
                            x.base_root,
                            (*r).uid,
                            (*r).gid,
                            S_IFDIR as u32 | (*r).mode,
                            0 as u32,
                            0 as u32,
                            0 as u32,
                        ) != 0
                        {
                            rc = install_fail!(
                                74 as i32,
                                b"xattr on /: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                strerror(*errno()),
                            );
                            break;
                        } else {
                            (*dts.offset(ndt as isize)).row = r.offset_from((*a).rows) as i32;
                            (*dts.offset(ndt as isize)).mtime = m.mtime;
                            ndt = ndt.wrapping_add(1);
                        }
                    } else {
                        let mut base: i32 = 0;
                        let mut dir: [::core::ffi::c_char; 4096] = [0; 4096];
                        locate(
                            &raw mut x,
                            a,
                            r,
                            &raw mut base,
                            &raw mut dir as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                        );
                        let mut nm: *const ::core::ffi::c_char =
                            strrchr((*r).path, '/' as i32).offset(1 as i32 as isize);
                        let mut pfd: i32 =
                            open_dir(&raw mut x, base, &raw mut dir as *mut ::core::ffi::c_char);
                        if pfd < 0 as i32 {
                            rc = install_fail!(
                                74 as i32,
                                b"open parent of %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                (*r).path,
                                strerror(*errno()),
                            );
                            break;
                        } else {
                            if (*r).r#type as i32 == 'd' as i32 {
                                if mkdirat(pfd, nm, 0o700 as u32) != 0 {
                                    rc = install_fail!(
                                        74 as i32,
                                        b"mkdir %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                        (*r).path,
                                        strerror(*errno()),
                                    );
                                    break;
                                } else {
                                    let mut dfd: i32 = openat(
                                        pfd,
                                        nm,
                                        O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
                                    );
                                    if dfd < 0 as i32 {
                                        rc = install_fail!(
                                            74 as i32,
                                            b"open %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                            (*r).path,
                                            strerror(*errno()),
                                        );
                                        break;
                                    } else {
                                        fchmod(dfd, (*r).mode as u32 & 0o777 as u32 | 0o700 as u32);
                                        let mut e: i32 = if is_seed != 0 {
                                            0 as i32
                                        } else {
                                            fset_meta(
                                                dfd,
                                                (*r).uid,
                                                (*r).gid,
                                                S_IFDIR as u32 | (*r).mode,
                                                0 as u32,
                                                0 as u32,
                                                0 as u32,
                                            )
                                        };
                                        close(dfd);
                                        if e != 0 {
                                            rc = install_fail!(
                                                74 as i32,
                                                b"xattr %s: %s\0".as_ptr()
                                                    as *const ::core::ffi::c_char,
                                                (*r).path,
                                                strerror(*errno()),
                                            );
                                            break;
                                        } else {
                                            (*dts.offset(ndt as isize)).row =
                                                r.offset_from((*a).rows) as i32;
                                            (*dts.offset(ndt as isize)).mtime = m.mtime;
                                            ndt = ndt.wrapping_add(1);
                                        }
                                    }
                                }
                            } else if (*r).r#type as i32 == 'l' as i32 {
                                if symlinkat(&raw mut m.link as *mut ::core::ffi::c_char, pfd, nm)
                                    != 0
                                {
                                    rc = install_fail!(
                                        74 as i32,
                                        b"symlink %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                        (*r).path,
                                        strerror(*errno()),
                                    );
                                    break;
                                } else {
                                    let mut ts: [timespec; 2] = [
                                        timespec {
                                            tv_sec: m.mtime as i64,
                                            tv_nsec: 0 as i64,
                                        },
                                        timespec {
                                            tv_sec: m.mtime as i64,
                                            tv_nsec: 0 as i64,
                                        },
                                    ];
                                    utimensat(
                                        pfd,
                                        nm,
                                        &raw mut ts as *mut timespec as *const timespec,
                                        AT_SYMLINK_NOFOLLOW,
                                    );
                                }
                            } else {
                                let mut fd: i32 = openat(
                                    pfd,
                                    nm,
                                    O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                                    0o600 as i32,
                                );
                                if fd < 0 as i32 {
                                    rc = install_fail!(
                                        74 as i32,
                                        b"create %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                        (*r).path,
                                        strerror(*errno()),
                                    );
                                    break;
                                } else {
                                    regular += m.size as i64;
                                    let mut left: u64 = m.size;
                                    while left != 0 && rc == 0 {
                                        let mut chunk: usize =
                                            if left < ((1 as i32) << 20 as i32) as u64 {
                                                left as usize
                                            } else {
                                                ((1 as i32) << 20 as i32) as usize
                                            };
                                        if src_read(s, buf as *mut ::core::ffi::c_void, chunk) != 0
                                        {
                                            rc = install_fail!(
                                                65 as i32,
                                                b"image: truncated data of %s\0".as_ptr()
                                                    as *const ::core::ffi::c_char,
                                                (*r).path,
                                            );
                                            break;
                                        } else {
                                            let mut off: usize = 0 as usize;
                                            while off < chunk {
                                                let mut w: isize = write(
                                                    fd,
                                                    buf.offset(off as isize)
                                                        as *const ::core::ffi::c_void,
                                                    chunk.wrapping_sub(off),
                                                );
                                                if w < 0 as isize {
                                                    if *errno() == EINTR {
                                                        continue;
                                                    }
                                                    rc = install_fail!(
                                                        74 as i32,
                                                        b"write %s: %s\0".as_ptr()
                                                            as *const ::core::ffi::c_char,
                                                        (*r).path,
                                                        strerror(*errno()),
                                                    );
                                                    break;
                                                } else {
                                                    off = off.wrapping_add(w as usize);
                                                }
                                            }
                                            left = (left as u64).wrapping_sub(chunk as u64) as u64;
                                        }
                                    }
                                    if rc == 0
                                        && src_read(
                                            s,
                                            NULL,
                                            (512 as usize)
                                                .wrapping_sub(
                                                    (m.size as usize).wrapping_rem(512 as usize),
                                                )
                                                .wrapping_rem(512 as usize),
                                        ) != 0
                                    {
                                        rc = install_fail!(
                                            65 as i32,
                                            b"image: truncated\0".as_ptr()
                                                as *const ::core::ffi::c_char,
                                        );
                                    }
                                    if rc == 0 {
                                        fchmod(
                                            fd,
                                            if is_seed != 0 {
                                                (*r).mode as u32 & 0o777 as u32
                                            } else {
                                                (*r).mode as u32 & 0o777 as u32 | 0o600 as u32
                                            },
                                        );
                                        if is_seed == 0
                                            && fset_meta(
                                                fd,
                                                (*r).uid,
                                                (*r).gid,
                                                S_IFREG as u32 | (*r).mode,
                                                0 as u32,
                                                0 as u32,
                                                0 as u32,
                                            ) != 0
                                        {
                                            rc = install_fail!(
                                                74 as i32,
                                                b"xattr %s: %s\0".as_ptr()
                                                    as *const ::core::ffi::c_char,
                                                (*r).path,
                                                strerror(*errno()),
                                            );
                                        }
                                        let mut ts_0: [timespec; 2] = [
                                            timespec {
                                                tv_sec: m.mtime as i64,
                                                tv_nsec: 0 as i64,
                                            },
                                            timespec {
                                                tv_sec: m.mtime as i64,
                                                tv_nsec: 0 as i64,
                                            },
                                        ];
                                        futimens(
                                            fd,
                                            &raw mut ts_0 as *mut timespec as *const timespec,
                                        );
                                    }
                                    close(fd);
                                    if rc != 0 {
                                        break;
                                    }
                                }
                            }
                            if is_seed != 0 {
                                seeds += 1;
                            }
                            if g_quiet == 0 && now_s() - last > 2.0f64 {
                                last = now_s();
                                install_note!(
                                    b"%lld/%lld members\0".as_ptr() as *const ::core::ffi::c_char,
                                    members,
                                    (*mt).members,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    free(buf as *mut ::core::ffi::c_void);
    if rc == 0 && members != (*mt).members {
        rc = install_fail!(
            65 as i32,
            b"image: %lld members, metadata says %lld\0".as_ptr() as *const ::core::ffi::c_char,
            members,
            (*mt).members,
        );
    }
    if rc == 0 && regular != (*mt).regular_bytes {
        rc = install_fail!(
            65 as i32,
            b"image: %lld regular bytes, metadata says %lld\0".as_ptr()
                as *const ::core::ffi::c_char,
            regular,
            (*mt).regular_bytes,
        );
    }
    if rc == 0 {
        while ri < (*a).n
            && strchr(
                b"dfl\0".as_ptr() as *const ::core::ffi::c_char,
                (*(*a).rows.offset(ri as isize)).r#type as i32,
            )
            .is_null()
        {
            ri = ri.wrapping_add(1);
        }
        if ri < (*a).n {
            rc = install_fail!(
                65 as i32,
                b"image: member for %s missing\0".as_ptr() as *const ::core::ffi::c_char,
                (*(*a).rows.offset(ri as isize)).path,
            );
        }
    }
    let mut hard_0: i64 = 0 as i64;
    let mut special_0: i64 = 0 as i64;
    let mut objects: i64 = 0 as i64;
    let mut obj_of: *mut i32 = (if rc != 0 {
        NULL
    } else {
        calloc((*a).n, ::core::mem::size_of::<i32>())
    }) as *mut i32;
    if rc == 0 && obj_of.is_null() {
        rc = install_fail!(
            74 as i32,
            b"out of memory\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    if rc == 0 {
        let mut have_links: i32 = 0 as i32;
        let mut i_1: usize = 0 as usize;
        while i_1 < (*a).n {
            if (*(*a).rows.offset(i_1 as isize)).r#type as i32 == 'h' as i32 {
                have_links = 1 as i32;
            }
            i_1 = i_1.wrapping_add(1);
        }
        let mut lfd: i32 = -1 as i32;
        if have_links != 0 {
            mkdirat(x.base_root, STORE_DIR.as_ptr(), 0o700 as u32);
            mkdirat(x.base_root, LINKS_DIR.as_ptr(), 0o700 as u32);
            lfd = openat(
                x.base_root,
                LINKS_DIR.as_ptr(),
                O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
            );
            if lfd < 0 as i32 {
                rc = install_fail!(
                    74 as i32,
                    b"create hardlink store: %s\0".as_ptr() as *const ::core::ffi::c_char,
                    strerror(*errno()),
                );
            }
        }
        let mut i_2: usize = 0 as usize;
        while i_2 < (*a).n && rc == 0 {
            let mut r_0: *mut row_t = (*a).rows.offset(i_2 as isize);
            if !strchr(
                b"hcbp\0".as_ptr() as *const ::core::ffi::c_char,
                (*r_0).r#type as i32,
            )
            .is_null()
            {
                let mut dir_0: [::core::ffi::c_char; 4096] = [0; 4096];
                let mut base_0: i32 = 0;
                locate(
                    &raw mut x,
                    a,
                    r_0,
                    &raw mut base_0,
                    &raw mut dir_0 as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                );
                let mut nm_0: *const ::core::ffi::c_char =
                    strrchr((*r_0).path, '/' as i32).offset(1 as i32 as isize);
                let mut pfd_0: i32 = open_dir(
                    &raw mut x,
                    base_0,
                    &raw mut dir_0 as *mut ::core::ffi::c_char,
                );
                if pfd_0 < 0 as i32 {
                    rc = install_fail!(
                        74 as i32,
                        b"open parent of %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                        (*r_0).path,
                        strerror(*errno()),
                    );
                    break;
                } else if (*r_0).r#type as i32 == 'h' as i32 {
                    let mut pr: *mut row_t = (*a).rows.offset((*r_0).primary as isize);
                    let mut id: [::core::ffi::c_char; 32] = [0; 32];
                    let mut stub: [::core::ffi::c_char; 64] = [0; 64];
                    if *obj_of.offset((*r_0).primary as isize) == 0 {
                        objects += 1;
                        *obj_of.offset((*r_0).primary as isize) = objects as i32;
                        snprintf(
                            &raw mut id as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 32]>(),
                            b"%016llx\0".as_ptr() as *const ::core::ffi::c_char,
                            (0x1000000000000000 as u64).wrapping_add(objects as u64),
                        );
                        let mut pdir: [::core::ffi::c_char; 4096] = [0; 4096];
                        let mut pbase: i32 = 0;
                        locate(
                            &raw mut x,
                            a,
                            pr,
                            &raw mut pbase,
                            &raw mut pdir as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                        );
                        let mut pnm: *const ::core::ffi::c_char =
                            strrchr((*pr).path, '/' as i32).offset(1 as i32 as isize);
                        let mut ppfd: i32 =
                            open_dir(&raw mut x, pbase, &raw mut pdir as *mut ::core::ffi::c_char);
                        if ppfd < 0 as i32
                            || renameat(ppfd, pnm, lfd, &raw mut id as *mut ::core::ffi::c_char)
                                != 0
                        {
                            rc = install_fail!(
                                74 as i32,
                                b"hardlink %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                (*pr).path,
                                strerror(*errno()),
                            );
                            break;
                        } else {
                            snprintf(
                                &raw mut stub as *mut ::core::ffi::c_char,
                                ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                                b"/%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
                                LINKS_DIR.as_ptr(),
                                &raw mut id as *mut ::core::ffi::c_char,
                            );
                            if symlinkat(&raw mut stub as *mut ::core::ffi::c_char, ppfd, pnm) != 0
                            {
                                rc = install_fail!(
                                    74 as i32,
                                    b"hardlink %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                    (*pr).path,
                                    strerror(*errno()),
                                );
                                break;
                            } else {
                                pfd_0 = open_dir(
                                    &raw mut x,
                                    base_0,
                                    &raw mut dir_0 as *mut ::core::ffi::c_char,
                                );
                                if pfd_0 < 0 as i32 {
                                    rc = install_fail!(
                                        74 as i32,
                                        b"open parent of %s: %s\0".as_ptr()
                                            as *const ::core::ffi::c_char,
                                        (*r_0).path,
                                        strerror(*errno()),
                                    );
                                    break;
                                }
                            }
                        }
                    }
                    snprintf(
                        &raw mut id as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 32]>(),
                        b"%016llx\0".as_ptr() as *const ::core::ffi::c_char,
                        (0x1000000000000000 as u64)
                            .wrapping_add(*obj_of.offset((*r_0).primary as isize) as u64),
                    );
                    snprintf(
                        &raw mut stub as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                        b"/%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
                        LINKS_DIR.as_ptr(),
                        &raw mut id as *mut ::core::ffi::c_char,
                    );
                    if symlinkat(&raw mut stub as *mut ::core::ffi::c_char, pfd_0, nm_0) != 0 {
                        rc = install_fail!(
                            74 as i32,
                            b"hardlink %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                            (*r_0).path,
                            strerror(*errno()),
                        );
                        break;
                    } else {
                        *obj_of.offset(i_2 as isize) = *obj_of.offset((*r_0).primary as isize);
                        hard_0 += 1;
                    }
                } else {
                    let mut fd_0: i32 = openat(
                        pfd_0,
                        nm_0,
                        O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                        0o600 as i32,
                    );
                    if fd_0 < 0 as i32 {
                        rc = install_fail!(
                            74 as i32,
                            b"create %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                            (*r_0).path,
                            strerror(*errno()),
                        );
                        break;
                    } else {
                        let mut r#type: u32 = (if (*r_0).r#type as i32 == 'c' as i32 {
                            S_IFCHR
                        } else if (*r_0).r#type as i32 == 'b' as i32 {
                            S_IFBLK
                        } else {
                            S_IFIFO
                        }) as u32;
                        if fset_meta(
                            fd_0,
                            (*r_0).uid,
                            (*r_0).gid,
                            r#type | (*r_0).mode,
                            0 as u32,
                            (*r_0).major,
                            (*r_0).minor,
                        ) != 0
                        {
                            rc = install_fail!(
                                74 as i32,
                                b"xattr %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                (*r_0).path,
                                strerror(*errno()),
                            );
                        }
                        close(fd_0);
                        special_0 += 1;
                    }
                }
            }
            i_2 = i_2.wrapping_add(1);
        }
        let mut ob: i64 = 1 as i64;
        while ob <= objects && rc == 0 {
            let mut names: u32 = 0 as u32;
            let mut prim: usize = 0 as usize;
            let mut i_3: usize = 0 as usize;
            while i_3 < (*a).n {
                if *obj_of.offset(i_3 as isize) as i64 == ob {
                    names = names.wrapping_add(1);
                    if (*(*a).rows.offset(i_3 as isize)).r#type as i32 == 'f' as i32 {
                        prim = i_3;
                    }
                }
                i_3 = i_3.wrapping_add(1);
            }
            let mut id_0: [::core::ffi::c_char; 32] = [0; 32];
            snprintf(
                &raw mut id_0 as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 32]>(),
                b"%016llx\0".as_ptr() as *const ::core::ffi::c_char,
                (0x1000000000000000 as u64).wrapping_add(ob as u64),
            );
            let mut fd_1: i32 = openat(
                lfd,
                &raw mut id_0 as *mut ::core::ffi::c_char,
                O_RDONLY | O_NOFOLLOW | O_CLOEXEC,
            );
            let mut pr_0: *const row_t = (*a).rows.offset(prim as isize);
            if fd_1 < 0 as i32
                || fset_meta(
                    fd_1,
                    (*pr_0).uid,
                    (*pr_0).gid,
                    S_IFREG as u32 | (*pr_0).mode,
                    names as u32,
                    0 as u32,
                    0 as u32,
                ) != 0
            {
                rc = install_fail!(
                    74 as i32,
                    b"hardlink count: %s\0".as_ptr() as *const ::core::ffi::c_char,
                    strerror(*errno()),
                );
            }
            if fd_1 >= 0 as i32 {
                close(fd_1);
            }
            ob += 1;
        }
        if lfd >= 0 as i32 {
            close(lfd);
        }
    }
    free(obj_of as *mut ::core::ffi::c_void);
    let mut i_4: usize = ndt;
    loop {
        let c2rust_fresh1 = i_4;
        i_4 = i_4.wrapping_sub(1);
        if !(c2rust_fresh1 > 0 as usize && rc == 0) {
            break;
        }
        let mut r_1: *const row_t = (*a).rows.offset((*dts.offset(i_4 as isize)).row as isize);
        let mut ts_1: [timespec; 2] = [
            timespec {
                tv_sec: (*dts.offset(i_4 as isize)).mtime as i64,
                tv_nsec: 0 as i64,
            },
            timespec {
                tv_sec: (*dts.offset(i_4 as isize)).mtime as i64,
                tv_nsec: 0 as i64,
            },
        ];
        if strcmp((*r_1).path, b"/\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            futimens(
                x.base_root,
                &raw mut ts_1 as *mut timespec as *const timespec,
            );
        } else {
            let mut base_1: i32 = 0;
            let mut dir_1: [::core::ffi::c_char; 4096] = [0; 4096];
            locate(
                &raw mut x,
                a,
                r_1,
                &raw mut base_1,
                &raw mut dir_1 as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            );
            let mut pfd_1: i32 = open_dir(
                &raw mut x,
                base_1,
                &raw mut dir_1 as *mut ::core::ffi::c_char,
            );
            if pfd_1 >= 0 as i32 {
                utimensat(
                    pfd_1,
                    strrchr((*r_1).path, '/' as i32).offset(1 as i32 as isize),
                    &raw mut ts_1 as *mut timespec as *const timespec,
                    AT_SYMLINK_NOFOLLOW,
                );
            }
        }
    }
    free(dts as *mut ::core::ffi::c_void);
    if x.cache_fd >= 0 as i32 {
        close(x.cache_fd);
    }
    if rc == 0 {
        let mut tmp: [u8; 4096] = [0; 4096];
        while rc == 0 {
            let mut k_0: isize = if (*s).out_pos < (*s).out_len {
                (*s).out_len.wrapping_sub((*s).out_pos) as isize
            } else {
                src_fill(s)
            };
            if k_0 < 0 as isize {
                rc = install_fail!(
                    65 as i32,
                    b"image: zstd stream error\0".as_ptr() as *const ::core::ffi::c_char,
                );
                break;
            } else {
                if k_0 == 0 as isize {
                    break;
                }
                (*s).out_pos = (*s).out_len;
            }
        }
        if rc == 0 && (*s).last_ret != 0 as usize {
            rc = install_fail!(
                65 as i32,
                b"image: zstd frame incomplete\0".as_ptr() as *const ::core::ffi::c_char,
            );
        }
    }
    if rc == 0 {
        syncfs(x.base_root);
    }
    if x.base_root >= 0 as i32 {
        close(x.base_root);
    }
    if x.base_seeds >= 0 as i32 {
        close(x.base_seeds);
    }
    *stats.offset(0isize) = members;
    *stats.offset(1isize) = hard_0;
    *stats.offset(2isize) = special_0;
    *stats.offset(3isize) = seeds;
    *stats.offset(4isize) = objects;
    return rc;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_install(mut o: *const eng_install_opts) -> i32 {
    g_quiet = (*o).quiet;
    g_err[0usize] = 0 as ::core::ffi::c_char;
    let mut want: [::core::ffi::c_char; 65] = ::core::mem::transmute::<
        [u8; 65],
        [::core::ffi::c_char; 65],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    );
    let mut want_size: i64 = -1 as i64;
    if !(*o).sha256.is_null() {
        snprintf(
            &raw mut want as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 65]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            (*o).sha256,
        );
    }
    if !(*o).index.is_null() {
        let mut f: *mut FILE = fopen((*o).index, b"re\0".as_ptr() as *const ::core::ffi::c_char);
        if f.is_null() {
            fprintf(
                stderr,
                b"install: cannot read %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                (*o).index,
                strerror(*errno()),
            );
            return 66 as i32;
        }
        let mut buf: [::core::ffi::c_char; 65536] = [0; 65536];
        let mut n: usize = fread(
            &raw mut buf as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            1 as usize,
            ::core::mem::size_of::<[::core::ffi::c_char; 65536]>(),
            f,
        ) as usize;
        fclose(f);
        let mut j: *mut eng_json = eng_json_parse(&raw mut buf as *mut ::core::ffi::c_char, n);
        let mut s: *const ::core::ffi::c_char = if !j.is_null() {
            eng_json_str(j, b"sha256\0".as_ptr() as *const ::core::ffi::c_char)
        } else {
            ::core::ptr::null::<::core::ffi::c_char>()
        };
        if s.is_null() || strlen(s) != 64 as usize {
            eng_json_free(j);
            fprintf(
                stderr,
                b"install: bad index %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                (*o).index,
            );
            return 65 as i32;
        }
        if want[0usize] as i32 != 0 && strcmp(&raw mut want as *mut ::core::ffi::c_char, s) != 0 {
            eng_json_free(j);
            fprintf(
                stderr,
                b"install: --sha256 differs from the index\n\0".as_ptr()
                    as *const ::core::ffi::c_char,
            );
            return 65 as i32;
        }
        snprintf(
            &raw mut want as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 65]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            s,
        );
        eng_json_int(
            j,
            b"size\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut want_size,
        );
        eng_json_free(j);
    }
    if want[0usize] == 0 {
        fprintf(
            stderr,
            b"install: the expected sha256 is required (--sha256 or --index)\n\0".as_ptr()
                as *const ::core::ffi::c_char,
        );
        return 2 as i32;
    }
    let mut r#in: i32 = if strcmp((*o).image, b"-\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
        dup(0 as i32)
    } else {
        open((*o).image, O_RDONLY | O_CLOEXEC)
    };
    if r#in < 0 as i32 {
        fprintf(
            stderr,
            b"install: cannot open %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            (*o).image,
            strerror(*errno()),
        );
        return 66 as i32;
    }
    if mkdir((*o).target, 0o700 as u32) != 0 as i32 {
        fprintf(
            stderr,
            b"install: cannot create %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            (*o).target,
            strerror(*errno()),
        );
        close(r#in);
        return 73 as i32;
    }
    let mut tfd: i32 = open((*o).target, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    let mut s_0: *mut src_t = calloc(1 as usize, ::core::mem::size_of::<src_t>()) as *mut src_t;
    let mut a: attrs_t = attrs_t {
        rows: ::core::ptr::null_mut::<row_t>(),
        n: 0,
        hash: ::core::ptr::null_mut::<i32>(),
        hcap: 0,
        stores: [store_t {
            guest: [0; 1024],
            store: [0; 256],
            glen: 0,
        }; 8],
        nstores: 0,
        user_uid: 0,
        user_gid: 0,
    };
    memset(
        &raw mut a as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<attrs_t>(),
    );
    let mut mt: meta_t = meta_t {
        rows: 0,
        size: 0,
        entries: 0,
        members: 0,
        regular_bytes: 0,
        attr_sha: [0; 65],
        requires: 0,
    };
    memset(
        &raw mut mt as *mut ::core::ffi::c_void,
        0 as i32,
        ::core::mem::size_of::<meta_t>(),
    );
    let mut stats: [i64; 5] = [0 as i64, 0, 0, 0, 0];
    let mut rc: i32 = 0;
    let mut t0: f64 = now_s();
    if tfd < 0 as i32 || s_0.is_null() {
        rc = install_fail!(
            74 as i32,
            b"open target: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*errno()),
        );
    } else {
        (*s_0).fd = r#in;
        (*s_0).dctx = ZSTD_createDCtx();
        eng_sha256_init(&raw mut (*s_0).sha);
        rc = if !(*s_0).dctx.is_null() {
            do_install(
                o,
                s_0,
                tfd,
                &raw mut mt,
                &raw mut a,
                &raw mut stats as *mut i64,
            )
        } else {
            install_fail!(
                74 as i32,
                b"zstd context\0".as_ptr() as *const ::core::ffi::c_char,
            )
        };
    }
    if rc == 0 {
        let mut tmp: [u8; 65536] = [0; 65536];
        let mut k: isize = 0;
        loop {
            k = read(
                r#in,
                &raw mut tmp as *mut u8 as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[u8; 65536]>(),
            );
            if k <= 0 as isize {
                break;
            }
            eng_sha256_update(
                &raw mut (*s_0).sha,
                &raw mut tmp as *mut u8 as *const ::core::ffi::c_void,
                k as usize,
            );
            (*s_0).in_bytes = (*s_0).in_bytes.wrapping_add(k as u64);
        }
        let mut d: [u8; 32] = [0; 32];
        let mut hex: [::core::ffi::c_char; 65] = [0; 65];
        eng_sha256_final(&raw mut (*s_0).sha, &raw mut d as *mut u8);
        eng_sha256_hex(
            &raw mut d as *mut u8 as *const u8,
            &raw mut hex as *mut ::core::ffi::c_char,
        );
        if strcmp(
            &raw mut hex as *mut ::core::ffi::c_char,
            &raw mut want as *mut ::core::ffi::c_char,
        ) != 0
        {
            rc = install_fail!(
                65 as i32,
                b"image sha256 %s does not match the expected %s\0".as_ptr()
                    as *const ::core::ffi::c_char,
                &raw mut hex as *mut ::core::ffi::c_char,
                &raw mut want as *mut ::core::ffi::c_char,
            );
        } else if want_size >= 0 as i64 && (*s_0).in_bytes as i64 != want_size {
            rc = install_fail!(
                65 as i32,
                b"image size %llu, index says %lld\0".as_ptr() as *const ::core::ffi::c_char,
                (*s_0).in_bytes as u64,
                want_size,
            );
        }
    }
    if !s_0.is_null() && !(*s_0).dctx.is_null() {
        ZSTD_freeDCtx((*s_0).dctx);
    }
    let mut i: usize = 0 as usize;
    while i < a.n {
        free((*a.rows.offset(i as isize)).path as *mut ::core::ffi::c_void);
        free((*a.rows.offset(i as isize)).extra as *mut ::core::ffi::c_void);
        i = i.wrapping_add(1);
    }
    free(a.rows as *mut ::core::ffi::c_void);
    free(a.hash as *mut ::core::ffi::c_void);
    close(r#in);
    if tfd >= 0 as i32 {
        if rc == 0 {
            fsync(tfd);
        }
        close(tfd);
    }
    if rc != 0 {
        fprintf(
            stderr,
            b"install: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            if g_err[0usize] as i32 != 0 {
                &raw mut g_err as *mut ::core::ffi::c_char as *const ::core::ffi::c_char
            } else {
                b"failed\0".as_ptr() as *const ::core::ffi::c_char
            },
        );
        if (*o).keep_partial == 0 {
            eng_remove_tree((*o).target);
        }
        free(s_0 as *mut ::core::ffi::c_void);
        return rc;
    }
    printf(
        b"{\"rows\":%zu,\"members\":%lld,\"hardlinks\":%lld,\"objects\":%lld,\"special\":%lld,\"seedEntries\":%lld,\"bytes\":%llu,\"sha256\":\"%s\",\"seconds\":%.1f}\n\0"
            .as_ptr() as *const ::core::ffi::c_char,
        mt.rows as usize,
        stats[0usize],
        stats[1usize],
        stats[4usize],
        stats[2usize],
        stats[3usize],
        (*s_0).in_bytes as u64,
        &raw mut want as *mut ::core::ffi::c_char,
        now_s() - t0,
    );
    free(s_0 as *mut ::core::ffi::c_void);
    return 0 as i32;
}
pub const FICLONE: usize = ((1 as u32) << _IOC_DIRSHIFT
    | ((0x94 as i32) << _IOC_TYPESHIFT) as u32
    | ((9 as i32) << _IOC_NRSHIFT) as u32) as usize
    | ::core::mem::size_of::<i32>() << _IOC_SIZESHIFT;
unsafe extern "C" fn copy_file(mut sfd: i32, mut dfd: i32) -> i32 {
    #[cfg(target_os = "linux")]
    if ioctl(dfd, FICLONE as u64, sfd) == 0 as i32 {
        return 0 as i32;
    }
    #[cfg(target_os = "android")]
    if ioctl(dfd, FICLONE as u32, sfd) == 0 as i32 {
        return 0 as i32;
    }
    let mut buf: [::core::ffi::c_char; 65536] = [0; 65536];
    loop {
        let mut n: isize = read(
            sfd,
            &raw mut buf as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<[::core::ffi::c_char; 65536]>(),
        );
        if n < 0 as isize {
            if *errno() == EINTR {
                continue;
            }
            return -*errno();
        } else {
            if n == 0 as isize {
                return 0 as i32;
            }
            let mut off: isize = 0 as isize;
            while off < n {
                let mut w: isize = write(
                    dfd,
                    (&raw mut buf as *mut ::core::ffi::c_char).offset(off as isize)
                        as *const ::core::ffi::c_void,
                    (n - off) as usize,
                );
                if w < 0 as isize {
                    if *errno() == EINTR {
                        continue;
                    }
                    return -*errno();
                } else {
                    off += w;
                }
            }
        }
    }
}
unsafe extern "C" fn copy_xattrs(mut sfd: i32, mut dfd: i32) -> i32 {
    let mut names: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut n: isize = flistxattr(
        sfd,
        &raw mut names as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    let mut i: isize = 0 as isize;
    while n > 0 as isize && i < n {
        if strncmp(
            (&raw mut names as *mut ::core::ffi::c_char).offset(i as isize),
            b"user.\0".as_ptr() as *const ::core::ffi::c_char,
            5 as usize,
        ) == 0
        {
            let mut v: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut vl: isize = fgetxattr(
                sfd,
                (&raw mut names as *mut ::core::ffi::c_char).offset(i as isize),
                &raw mut v as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            );
            if vl >= 0 as isize
                && fsetxattr(
                    dfd,
                    (&raw mut names as *mut ::core::ffi::c_char).offset(i as isize),
                    &raw mut v as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                    vl as usize,
                    0 as i32,
                ) != 0 as i32
            {
                return -*errno();
            }
        }
        i += strlen((&raw mut names as *mut ::core::ffi::c_char).offset(i as isize)) as isize
            + 1 as isize;
    }
    return 0 as i32;
}
unsafe extern "C" fn clone_dir(
    mut sdir: i32,
    mut ddir: i32,
    mut depth: i32,
    mut count: *mut i64,
) -> i32 {
    if depth > 512 as i32 {
        return -ELOOP;
    }
    let mut sfd2: i32 = dup(sdir);
    let mut d: *mut DIR = fdopendir(sfd2);
    if d.is_null() {
        close(sfd2);
        return -*errno();
    }
    let mut e: *mut dirent = ::core::ptr::null_mut::<dirent>();
    let mut rc: i32 = 0 as i32;
    while rc == 0 && {
        e = readdir(d);
        !e.is_null()
    } {
        let mut n: *const ::core::ffi::c_char = &raw mut (*e).d_name as *mut ::core::ffi::c_char;
        if strcmp(n, b".\0".as_ptr() as *const ::core::ffi::c_char) == 0
            || strcmp(n, b"..\0".as_ptr() as *const ::core::ffi::c_char) == 0
        {
            continue;
        }
        if depth == 0 as i32 && strcmp(n, STORE_DIR.as_ptr()) == 0 {
            let mut ss: i32 = openat(sdir, n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if ss < 0 as i32 {
                continue;
            }
            mkdirat(ddir, n, 0o700 as u32);
            let mut dd: i32 = openat(ddir, n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            let mut sl: i32 = openat(
                ss,
                b"links\0".as_ptr() as *const ::core::ffi::c_char,
                O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
            );
            if sl >= 0 as i32
                && dd >= 0 as i32
                && mkdirat(
                    dd,
                    b"links\0".as_ptr() as *const ::core::ffi::c_char,
                    0o700 as u32,
                ) == 0 as i32
            {
                let mut dl: i32 = openat(
                    dd,
                    b"links\0".as_ptr() as *const ::core::ffi::c_char,
                    O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
                );
                if dl >= 0 as i32 {
                    let mut ld: *mut DIR = fdopendir(sl);
                    sl = -1 as i32;
                    let mut le: *mut dirent = ::core::ptr::null_mut::<dirent>();
                    while rc == 0 && !ld.is_null() && {
                        le = readdir(ld);
                        !le.is_null()
                    } {
                        if (*le).d_name[0usize] as i32 == '.' as i32
                            || strncmp(
                                &raw mut (*le).d_name as *mut ::core::ffi::c_char,
                                b"journal-\0".as_ptr() as *const ::core::ffi::c_char,
                                8 as usize,
                            ) == 0
                        {
                            continue;
                        }
                        let mut a: i32 = openat(
                            dirfd(ld),
                            &raw mut (*le).d_name as *mut ::core::ffi::c_char,
                            O_RDONLY | O_NOFOLLOW | O_CLOEXEC,
                        );
                        let mut b: i32 = if a < 0 as i32 {
                            -1 as i32
                        } else {
                            openat(
                                dl,
                                &raw mut (*le).d_name as *mut ::core::ffi::c_char,
                                O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                                0o600 as i32,
                            )
                        };
                        let mut st: stat = platform_empty_stat();
                        if a < 0 as i32 || b < 0 as i32 || fstat(a, &raw mut st) != 0 {
                            rc = -*errno();
                        } else {
                            rc = copy_file(a, b);
                            if rc == 0 && {
                                rc = copy_xattrs(a, b);
                                rc == 0
                            } {
                                fchmod(b, st.st_mode & 0o7777 as u32);
                                let mut ts: [timespec; 2] = [st.st_atim, st.st_mtim];
                                futimens(b, &raw mut ts as *mut timespec as *const timespec);
                                *count += 1;
                            }
                        }
                        if a >= 0 as i32 {
                            close(a);
                        }
                        if b >= 0 as i32 {
                            close(b);
                        }
                    }
                    if !ld.is_null() {
                        closedir(ld);
                    }
                    close(dl);
                }
            }
            if sl >= 0 as i32 {
                close(sl);
            }
            if dd >= 0 as i32 {
                close(dd);
            }
            close(ss);
        } else {
            let mut st_0: stat = platform_empty_stat();
            if fstatat(dirfd(d), n, &raw mut st_0, AT_SYMLINK_NOFOLLOW) != 0 {
                rc = -*errno();
                break;
            } else {
                let mut ts_0: [timespec; 2] = [st_0.st_atim, st_0.st_mtim];
                if st_0.st_mode & S_IFMT as u32 == S_IFDIR as u32 {
                    if mkdirat(ddir, n, 0o700 as u32) != 0 {
                        rc = -*errno();
                        break;
                    } else {
                        let mut a_0: i32 =
                            openat(dirfd(d), n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
                        let mut b_0: i32 =
                            openat(ddir, n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
                        if a_0 < 0 as i32 || b_0 < 0 as i32 {
                            rc = -*errno();
                        } else {
                            rc = copy_xattrs(a_0, b_0);
                            if rc == 0 && {
                                rc = clone_dir(a_0, b_0, depth + 1 as i32, count);
                                rc == 0
                            } {
                                fchmod(b_0, st_0.st_mode & 0o7777 as u32);
                                futimens(b_0, &raw mut ts_0 as *mut timespec as *const timespec);
                            }
                        }
                        if a_0 >= 0 as i32 {
                            close(a_0);
                        }
                        if b_0 >= 0 as i32 {
                            close(b_0);
                        }
                    }
                } else if st_0.st_mode & S_IFMT as u32 == S_IFLNK as u32 {
                    let mut t: [::core::ffi::c_char; 4096] = [0; 4096];
                    let mut k: isize = readlinkat(
                        dirfd(d),
                        n,
                        &raw mut t as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
                            .wrapping_sub(1 as usize),
                    );
                    if k < 0 as isize {
                        rc = -*errno();
                        break;
                    } else {
                        t[k as usize] = 0 as ::core::ffi::c_char;
                        if symlinkat(&raw mut t as *mut ::core::ffi::c_char, ddir, n) != 0 {
                            rc = -*errno();
                            break;
                        } else {
                            utimensat(
                                ddir,
                                n,
                                &raw mut ts_0 as *mut timespec as *const timespec,
                                AT_SYMLINK_NOFOLLOW,
                            );
                        }
                    }
                } else {
                    if st_0.st_mode & S_IFMT as u32 != S_IFREG as u32 {
                        continue;
                    }
                    let mut a_1: i32 = openat(dirfd(d), n, O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
                    let mut b_1: i32 = if a_1 < 0 as i32 {
                        -1 as i32
                    } else {
                        openat(
                            ddir,
                            n,
                            O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                            0o600 as i32,
                        )
                    };
                    if a_1 < 0 as i32 || b_1 < 0 as i32 {
                        rc = -*errno();
                    } else {
                        rc = copy_file(a_1, b_1);
                        if rc == 0 && {
                            rc = copy_xattrs(a_1, b_1);
                            rc == 0
                        } {
                            fchmod(b_1, st_0.st_mode & 0o7777 as u32);
                            futimens(b_1, &raw mut ts_0 as *mut timespec as *const timespec);
                        }
                    }
                    if a_1 >= 0 as i32 {
                        close(a_1);
                    }
                    if b_1 >= 0 as i32 {
                        close(b_1);
                    }
                }
                *count += 1;
            }
        }
    }
    closedir(d);
    return rc;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_clone(
    mut src: *const ::core::ffi::c_char,
    mut dst: *const ::core::ffi::c_char,
    mut quiet: i32,
) -> i32 {
    let mut sr: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut sr as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/rootfs\0".as_ptr() as *const ::core::ffi::c_char,
        src,
    );
    let mut s: i32 = open(
        &raw mut sr as *mut ::core::ffi::c_char,
        O_RDONLY | O_DIRECTORY | O_CLOEXEC,
    );
    if s < 0 as i32 {
        fprintf(
            stderr,
            b"clone: %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut sr as *mut ::core::ffi::c_char,
            strerror(*errno()),
        );
        return 66 as i32;
    }
    if mkdir(dst, 0o700 as u32) != 0 {
        fprintf(
            stderr,
            b"clone: cannot create %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            dst,
            strerror(*errno()),
        );
        close(s);
        return 73 as i32;
    }
    let mut t: i32 = open(dst, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    let mut d: i32 = -1 as i32;
    if t >= 0 as i32
        && mkdirat(
            t,
            b"rootfs\0".as_ptr() as *const ::core::ffi::c_char,
            0o700 as u32,
        ) == 0 as i32
    {
        d = openat(
            t,
            b"rootfs\0".as_ptr() as *const ::core::ffi::c_char,
            O_RDONLY | O_DIRECTORY | O_CLOEXEC,
        );
    }
    let mut t0: f64 = now_s();
    let mut count: i64 = 0 as i64;
    let mut rc: i32 = if d < 0 as i32 { -*errno() } else { 0 as i32 };
    if rc == 0 {
        let mut st: stat = platform_empty_stat();
        fstat(s, &raw mut st);
        rc = copy_xattrs(s, d);
        if rc == 0 && {
            rc = clone_dir(s, d, 0 as i32, &raw mut count);
            rc == 0
        } {
            fchmod(d, st.st_mode & 0o7777 as u32);
            let mut ts: [timespec; 2] = [st.st_atim, st.st_mtim];
            futimens(d, &raw mut ts as *mut timespec as *const timespec);
            syncfs(d);
        }
    }
    close(s);
    if d >= 0 as i32 {
        close(d);
    }
    if t >= 0 as i32 {
        fsync(t);
        close(t);
    }
    if rc != 0 {
        fprintf(
            stderr,
            b"clone: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(-rc),
        );
        eng_remove_tree(dst);
        return 74 as i32;
    }
    if quiet == 0 {
        fprintf(
            stderr,
            b"clone: %lld entries\n\0".as_ptr() as *const ::core::ffi::c_char,
            count,
        );
    }
    printf(
        b"{\"entries\":%lld,\"seconds\":%.1f}\n\0".as_ptr() as *const ::core::ffi::c_char,
        count,
        now_s() - t0,
    );
    return 0 as i32;
}
unsafe extern "C" fn count_meta(
    mut dfd: i32,
    mut depth: i32,
    mut entries: *mut i64,
    mut missing: *mut i64,
    mut out: *mut FILE,
) {
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
            || depth == 0 as i32 && strcmp(n, STORE_DIR.as_ptr()) == 0
        {
            continue;
        }
        let mut st: stat = platform_empty_stat();
        if fstatat(dirfd(d), n, &raw mut st, AT_SYMLINK_NOFOLLOW) != 0 {
            continue;
        }
        *entries += 1;
        if st.st_mode & S_IFMT as u32 == S_IFLNK as u32 {
            continue;
        }
        let mut fd: i32 = openat(
            dirfd(d),
            n,
            O_RDONLY
                | O_NOFOLLOW
                | O_CLOEXEC
                | O_NONBLOCK
                | if st.st_mode & S_IFMT as u32 == S_IFDIR as u32 {
                    O_DIRECTORY
                } else {
                    0 as i32
                },
        );
        if fd < 0 as i32 {
            continue;
        }
        let mut v: [::core::ffi::c_char; 128] = [0; 128];
        if fgetxattr(
            fd,
            ENG_META_XATTR.as_ptr(),
            &raw mut v as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<[::core::ffi::c_char; 128]>(),
        ) <= 0 as isize
        {
            *missing += 1;
            if *missing <= 10 as i64 {
                fprintf(
                    out,
                    b"no metadata: %s (depth %d)\n\0".as_ptr() as *const ::core::ffi::c_char,
                    n,
                    depth,
                );
            }
        }
        if st.st_mode & S_IFMT as u32 == S_IFDIR as u32 {
            count_meta(fd, depth + 1 as i32, entries, missing, out);
        } else {
            close(fd);
        }
    }
    closedir(d);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_verify(
    mut generation: *const ::core::ffi::c_char,
    mut quiet: i32,
) -> i32 {
    let mut r: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut r as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/rootfs\0".as_ptr() as *const ::core::ffi::c_char,
        generation,
    );
    let mut g: *mut eng_guest = eng_guest_open(&raw mut r as *mut ::core::ffi::c_char);
    if g.is_null() {
        fprintf(
            stderr,
            b"verify: %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut r as *mut ::core::ffi::c_char,
            strerror(*errno()),
        );
        return 66 as i32;
    }
    let mut fd: i32 = open(
        &raw mut r as *mut ::core::ffi::c_char,
        O_RDONLY | O_DIRECTORY | O_CLOEXEC,
    );
    let mut entries: i64 = 0 as i64;
    let mut missing: i64 = 0 as i64;
    if fd >= 0 as i32 {
        count_meta(fd, 0 as i32, &raw mut entries, &raw mut missing, stderr);
    }
    let mut devnull: *mut FILE = if quiet != 0 {
        fopen(
            b"/dev/null\0".as_ptr() as *const ::core::ffi::c_char,
            b"we\0".as_ptr() as *const ::core::ffi::c_char,
        )
    } else {
        ::core::ptr::null_mut::<FILE>()
    };
    let mut problems: i32 = eng_link_fsck(
        g,
        0 as i32,
        (if !devnull.is_null() { devnull } else { stderr }) as *mut ::core::ffi::c_void,
    );
    if !devnull.is_null() {
        fclose(devnull);
    }
    eng_guest_close(g);
    printf(
        b"{\"entries\":%lld,\"withoutMetadata\":%lld,\"hardlinkProblems\":%d}\n\0".as_ptr()
            as *const ::core::ffi::c_char,
        entries,
        missing,
        problems,
    );
    return if problems == 0 as i32 && missing == 0 as i64 {
        0 as i32
    } else {
        1 as i32
    };
}
#[cfg(target_os = "linux")]
unsafe fn platform_empty_statvfs() -> statvfs {
    statvfs {
        f_bsize: 0,
        f_frsize: 0,
        f_blocks: 0,
        f_bfree: 0,
        f_bavail: 0,
        f_files: 0,
        f_ffree: 0,
        f_favail: 0,
        f_fsid: 0,
        f_flag: 0,
        f_namemax: 0,
        f_type: 0,
        __f_spare: [0; 5],
    }
}
#[cfg(target_os = "android")]
unsafe fn platform_empty_statvfs() -> statvfs {
    statvfs {
        f_bsize: 0,
        f_frsize: 0,
        f_blocks: 0,
        f_bfree: 0,
        f_bavail: 0,
        f_files: 0,
        f_ffree: 0,
        f_favail: 0,
        f_fsid: 0,
        f_flag: 0,
        f_namemax: 0,
        __f_reserved: [0; 6],
    }
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
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __va_list {
    pub __stack: *mut ::core::ffi::c_void,
    pub __gr_top: *mut ::core::ffi::c_void,
    pub __vr_top: *mut ::core::ffi::c_void,
    pub __gr_offs: i32,
    pub __vr_offs: i32,
}
