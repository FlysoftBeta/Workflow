//! Streaming image validation, installation and generation lifecycle.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
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
#[repr(C)]
pub struct DIR {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct __sFILE {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    unsafe fn fdopendir(__dir_fd: ::core::ffi::c_int) -> *mut DIR;
    unsafe fn readdir(__dir: *mut DIR) -> *mut dirent;
    unsafe fn closedir(__dir: *mut DIR) -> ::core::ffi::c_int;
    unsafe fn dirfd(__dir: *mut DIR) -> ::core::ffi::c_int;
    unsafe fn __errno() -> *mut ::core::ffi::c_int;
    unsafe fn openat(
        __dir_fd: ::core::ffi::c_int,
        __path: *const ::core::ffi::c_char,
        __flags: ::core::ffi::c_int,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn open(
        __path: *const ::core::ffi::c_char,
        __flags: ::core::ffi::c_int,
        ...
    ) -> ::core::ffi::c_int;
    static mut stderr: *mut FILE;
    unsafe fn fclose(__fp: *mut FILE) -> ::core::ffi::c_int;
    unsafe fn fprintf(
        __fp: *mut FILE,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn fputc(__ch: ::core::ffi::c_int, __fp: *mut FILE) -> ::core::ffi::c_int;
    unsafe fn fread(
        __buf: *mut ::core::ffi::c_void,
        __size: size_t,
        __count: size_t,
        __fp: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    unsafe fn printf(__fmt: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    unsafe fn sscanf(
        __s: *const ::core::ffi::c_char,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn renameat(
        __old_dir_fd: ::core::ffi::c_int,
        __old_path: *const ::core::ffi::c_char,
        __new_dir_fd: ::core::ffi::c_int,
        __new_path: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
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
    unsafe fn calloc(__item_count: size_t, __item_size: size_t) -> *mut ::core::ffi::c_void;
    unsafe fn free(__ptr: *mut ::core::ffi::c_void);
    unsafe fn strtoll(
        __s: *const ::core::ffi::c_char,
        __end_ptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_longlong;
    unsafe fn strtoul(
        __s: *const ::core::ffi::c_char,
        __end_ptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulong;
    unsafe fn strtoull(
        __s: *const ::core::ffi::c_char,
        __end_ptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulonglong;
    unsafe fn memchr(
        __s: *const ::core::ffi::c_void,
        __ch: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
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
    unsafe fn memset(
        __dst: *mut ::core::ffi::c_void,
        __ch: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn strchr(
        __s: *const ::core::ffi::c_char,
        __ch: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strrchr(
        __s: *const ::core::ffi::c_char,
        __ch: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    unsafe fn strcmp(
        __lhs: *const ::core::ffi::c_char,
        __rhs: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn strstr(
        __haystack: *const ::core::ffi::c_char,
        __needle: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strtok_r(
        __s: *mut ::core::ffi::c_char,
        __delimiter: *const ::core::ffi::c_char,
        __pos_ptr: *mut *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strerror(__errno_value: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
    unsafe fn strncmp(
        __lhs: *const ::core::ffi::c_char,
        __rhs: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn strspn(
        __s: *const ::core::ffi::c_char,
        __accept: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_ulong;
    unsafe fn ioctl(__fd: ::core::ffi::c_int, __op: ::core::ffi::c_uint, ...)
    -> ::core::ffi::c_int;
    unsafe fn fchmod(__fd: ::core::ffi::c_int, __mode: mode_t) -> ::core::ffi::c_int;
    unsafe fn mkdir(__path: *const ::core::ffi::c_char, __mode: mode_t) -> ::core::ffi::c_int;
    unsafe fn mkdirat(
        __dir_fd: ::core::ffi::c_int,
        __path: *const ::core::ffi::c_char,
        __mode: mode_t,
    ) -> ::core::ffi::c_int;
    unsafe fn fstat(__fd: ::core::ffi::c_int, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn fstatat(
        __dir_fd: ::core::ffi::c_int,
        __path: *const ::core::ffi::c_char,
        __buf: *mut stat,
        __flags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn utimensat(
        __dir_fd: ::core::ffi::c_int,
        __path: *const ::core::ffi::c_char,
        __times: *const timespec,
        __flags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn futimens(__fd: ::core::ffi::c_int, __times: *const timespec) -> ::core::ffi::c_int;
    unsafe fn fstatvfs(__fd: ::core::ffi::c_int, __buf: *mut statvfs) -> ::core::ffi::c_int;
    unsafe fn fsetxattr(
        __fd: ::core::ffi::c_int,
        __name: *const ::core::ffi::c_char,
        __value: *const ::core::ffi::c_void,
        __size: size_t,
        __flags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn fgetxattr(
        __fd: ::core::ffi::c_int,
        __name: *const ::core::ffi::c_char,
        __value: *mut ::core::ffi::c_void,
        __size: size_t,
    ) -> ssize_t;
    unsafe fn flistxattr(
        __fd: ::core::ffi::c_int,
        __list: *mut ::core::ffi::c_char,
        __size: size_t,
    ) -> ssize_t;
    unsafe fn clock_gettime(__clock: clockid_t, __ts: *mut timespec) -> ::core::ffi::c_int;
    unsafe fn unlinkat(
        __dirfd: ::core::ffi::c_int,
        __path: *const ::core::ffi::c_char,
        __flags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn symlinkat(
        __old_path: *const ::core::ffi::c_char,
        __new_dir_fd: ::core::ffi::c_int,
        __new_path: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn readlinkat(
        __dir_fd: ::core::ffi::c_int,
        __path: *const ::core::ffi::c_char,
        __buf: *mut ::core::ffi::c_char,
        __buf_size: size_t,
    ) -> ssize_t;
    unsafe fn syncfs(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
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
    unsafe fn dup(__old_fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn fsync(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn eng_guest_open(root: *const ::core::ffi::c_char) -> *mut eng_guest;
    unsafe fn eng_guest_close(g: *mut eng_guest);
    unsafe fn eng_json_parse(text: *const ::core::ffi::c_char, len: size_t) -> *mut eng_json;
    unsafe fn eng_json_free(n: *mut eng_json);
    unsafe fn eng_json_get(
        obj: *const eng_json,
        key: *const ::core::ffi::c_char,
    ) -> *const eng_json;
    unsafe fn eng_json_str(
        obj: *const eng_json,
        key: *const ::core::ffi::c_char,
    ) -> *const ::core::ffi::c_char;
    unsafe fn eng_json_int(
        obj: *const eng_json,
        key: *const ::core::ffi::c_char,
        out: *mut ::core::ffi::c_longlong,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_link_fsck(
        g: *mut eng_guest,
        repair: ::core::ffi::c_int,
        out: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_sha256_init(c: *mut eng_sha256);
    unsafe fn eng_sha256_update(c: *mut eng_sha256, data: *const ::core::ffi::c_void, n: size_t);
    unsafe fn eng_sha256_final(c: *mut eng_sha256, out: *mut uint8_t);
    unsafe fn eng_sha256_hex(d: *const uint8_t, out: *mut ::core::ffi::c_char);
    unsafe fn ZSTD_isError(result: size_t) -> ::core::ffi::c_uint;
    unsafe fn ZSTD_createDCtx() -> *mut ZSTD_DCtx;
    unsafe fn ZSTD_freeDCtx(dctx: *mut ZSTD_DCtx) -> size_t;
    unsafe fn ZSTD_decompressStream(
        zds: *mut ZSTD_DStream,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
    ) -> size_t;
}
pub type __builtin_va_list = __va_list;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __va_list {
    pub __stack: *mut ::core::ffi::c_void,
    pub __gr_top: *mut ::core::ffi::c_void,
    pub __vr_top: *mut ::core::ffi::c_void,
    pub __gr_offs: ::core::ffi::c_int,
    pub __vr_offs: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_install_opts {
    pub image: *const ::core::ffi::c_char,
    pub sha256: *const ::core::ffi::c_char,
    pub index: *const ::core::ffi::c_char,
    pub target: *const ::core::ffi::c_char,
    pub profile: *const ::core::ffi::c_char,
    pub quiet: ::core::ffi::c_int,
    pub keep_partial: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct src_t {
    pub fd: ::core::ffi::c_int,
    pub dctx: *mut ZSTD_DCtx,
    pub sha: eng_sha256,
    pub in_bytes: uint64_t,
    pub r#in: [::core::ffi::c_uchar; 131072],
    pub zin: ZSTD_inBuffer,
    pub in_eof: ::core::ffi::c_int,
    pub out: [::core::ffi::c_uchar; 131072],
    pub out_pos: size_t,
    pub out_len: size_t,
    pub last_ret: size_t,
}
pub type size_t = usize;
pub type ZSTD_inBuffer = ZSTD_inBuffer_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_inBuffer_s {
    pub src: *const ::core::ffi::c_void,
    pub size: size_t,
    pub pos: size_t,
}
pub type uint64_t = u64;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_sha256 {
    pub h: [uint32_t; 8],
    pub len: uint64_t,
    pub buf: [uint8_t; 64],
    pub n: size_t,
}
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type ZSTD_DCtx = ZSTD_DCtx_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: time_t,
    pub tv_nsec: ::core::ffi::c_long,
}
pub type time_t = __time_t;
pub type __time_t = __kernel_time_t;
pub type __kernel_time_t = __kernel_long_t;
pub type __kernel_long_t = ::core::ffi::c_long;
pub type clockid_t = __clockid_t;
pub type __clockid_t = __kernel_clockid_t;
pub type __kernel_clockid_t = ::core::ffi::c_int;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct meta_t {
    pub rows: ::core::ffi::c_longlong,
    pub size: ::core::ffi::c_longlong,
    pub entries: ::core::ffi::c_longlong,
    pub members: ::core::ffi::c_longlong,
    pub regular_bytes: ::core::ffi::c_longlong,
    pub attr_sha: [::core::ffi::c_char; 65],
    pub requires: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dirent {
    pub d_ino: ino_t,
    pub d_off: off64_t,
    pub d_reclen: ::core::ffi::c_ushort,
    pub d_type: ::core::ffi::c_uchar,
    pub d_name: [::core::ffi::c_char; 256],
}
pub type off64_t = loff_t;
pub type loff_t = off_t;
pub type off_t = int64_t;
pub type int64_t = i64;
pub type ino_t = __ino_t;
pub type __ino_t = __kernel_ino_t;
pub type __kernel_ino_t = __kernel_ulong_t;
pub type __kernel_ulong_t = ::core::ffi::c_ulong;
pub type mode_t = __mode_t;
pub type __mode_t = __kernel_mode_t;
pub type __kernel_mode_t = ::core::ffi::c_uint;
pub type FILE = __sFILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct attrs_t {
    pub rows: *mut row_t,
    pub n: size_t,
    pub hash: *mut ::core::ffi::c_int,
    pub hcap: size_t,
    pub stores: [store_t; 8],
    pub nstores: ::core::ffi::c_int,
    pub user_uid: uint32_t,
    pub user_gid: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct store_t {
    pub guest: [::core::ffi::c_char; 1024],
    pub store: [::core::ffi::c_char; 256],
    pub glen: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct row_t {
    pub path: *mut ::core::ffi::c_char,
    pub plen: size_t,
    pub r#type: ::core::ffi::c_char,
    pub uid: uint32_t,
    pub gid: uint32_t,
    pub mode: uint32_t,
    pub extra: *mut ::core::ffi::c_char,
    pub major: uint32_t,
    pub minor: uint32_t,
    pub store: ::core::ffi::c_int,
    pub primary: ::core::ffi::c_int,
}
pub type va_list = __builtin_va_list;
pub type ssize_t = isize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ex_t {
    pub base_root: ::core::ffi::c_int,
    pub base_seeds: ::core::ffi::c_int,
    pub cache_dir: [::core::ffi::c_char; 4096],
    pub cache_fd: ::core::ffi::c_int,
    pub cache_base: ::core::ffi::c_int,
}
pub type ZSTD_outBuffer = ZSTD_outBuffer_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_outBuffer_s {
    pub dst: *mut ::core::ffi::c_void,
    pub size: size_t,
    pub pos: size_t,
}
pub type ZSTD_DStream = ZSTD_DCtx;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dtime_t {
    pub row: ::core::ffi::c_int,
    pub mtime: int64_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct member_t {
    pub path: [::core::ffi::c_char; 4096],
    pub link: [::core::ffi::c_char; 4096],
    pub r#type: ::core::ffi::c_char,
    pub size: uint64_t,
    pub mtime: int64_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct statvfs {
    pub f_bsize: ::core::ffi::c_ulong,
    pub f_frsize: ::core::ffi::c_ulong,
    pub f_blocks: fsblkcnt_t,
    pub f_bfree: fsblkcnt_t,
    pub f_bavail: fsblkcnt_t,
    pub f_files: fsfilcnt_t,
    pub f_ffree: fsfilcnt_t,
    pub f_favail: fsfilcnt_t,
    pub f_fsid: ::core::ffi::c_ulong,
    pub f_flag: ::core::ffi::c_ulong,
    pub f_namemax: ::core::ffi::c_ulong,
    pub __f_reserved: [uint32_t; 6],
}
pub type fsfilcnt_t = ::core::ffi::c_ulong;
pub type fsblkcnt_t = ::core::ffi::c_ulong;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_json {
    pub t: eng_jtype,
    pub key: *mut ::core::ffi::c_char,
    pub s: *mut ::core::ffi::c_char,
    pub n: ::core::ffi::c_double,
    pub b: ::core::ffi::c_int,
    pub child: *mut eng_json,
    pub next: *mut eng_json,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_jtype(pub ::core::ffi::c_uint);
impl eng_jtype {
    pub const ENG_J_NULL: Self = Self(0);
    pub const ENG_J_BOOL: Self = Self(1);
    pub const ENG_J_NUM: Self = Self(2);
    pub const ENG_J_STR: Self = Self(3);
    pub const ENG_J_ARR: Self = Self(4);
    pub const ENG_J_OBJ: Self = Self(5);
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
pub type dev_t = uint64_t;
pub type gid_t = __gid_t;
pub type __gid_t = __kernel_gid32_t;
pub type __kernel_gid32_t = ::core::ffi::c_uint;
pub type uid_t = __uid_t;
pub type __uid_t = __kernel_uid32_t;
pub type __kernel_uid32_t = ::core::ffi::c_uint;
pub type nlink_t = __nlink_t;
pub type __nlink_t = uint32_t;
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
pub struct C2Rust_Unnamed(pub ::core::ffi::c_uint);
impl C2Rust_Unnamed {
    pub const REQ_OWNERSHIP: Self = Self(1);
    pub const REQ_MODE: Self = Self(2);
    pub const REQ_HARDLINK: Self = Self(4);
    pub const REQ_SPECIAL: Self = Self(8);
    pub const REQ_SEEDS: Self = Self(16);
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const EPERM: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ENOENT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const EINTR: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const EEXIST: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const EISDIR: ::core::ffi::c_int = 21 as ::core::ffi::c_int;
pub const EPIPE: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const ELOOP: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const EBADMSG: ::core::ffi::c_int = 74 as ::core::ffi::c_int;
pub const O_DIRECTORY: ::core::ffi::c_int = 0o40000 as ::core::ffi::c_int;
pub const O_NOFOLLOW: ::core::ffi::c_int = 0o100000 as ::core::ffi::c_int;
pub const O_RDONLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const O_WRONLY: ::core::ffi::c_int = 0o1 as ::core::ffi::c_int;
pub const O_CREAT: ::core::ffi::c_int = 0o100 as ::core::ffi::c_int;
pub const O_EXCL: ::core::ffi::c_int = 0o200 as ::core::ffi::c_int;
pub const O_NONBLOCK: ::core::ffi::c_int = 0o4000 as ::core::ffi::c_int;
pub const O_CLOEXEC: ::core::ffi::c_int = 0o2000000 as ::core::ffi::c_int;
pub const AT_FDCWD: ::core::ffi::c_int = -100 as ::core::ffi::c_int;
pub const AT_SYMLINK_NOFOLLOW: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const AT_REMOVEDIR: ::core::ffi::c_int = 0x200 as ::core::ffi::c_int;
pub const S_IFMT: ::core::ffi::c_int = 0o170000 as ::core::ffi::c_int;
pub const S_IFLNK: ::core::ffi::c_int = 0o120000 as ::core::ffi::c_int;
pub const S_IFREG: ::core::ffi::c_int = 0o100000 as ::core::ffi::c_int;
pub const S_IFBLK: ::core::ffi::c_int = 0o60000 as ::core::ffi::c_int;
pub const S_IFDIR: ::core::ffi::c_int = 0o40000 as ::core::ffi::c_int;
pub const S_IFCHR: ::core::ffi::c_int = 0o20000 as ::core::ffi::c_int;
pub const S_IFIFO: ::core::ffi::c_int = 0o10000 as ::core::ffi::c_int;
pub const _IOC_NRBITS: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const _IOC_TYPEBITS: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const _IOC_SIZEBITS: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const _IOC_NRSHIFT: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const _IOC_TYPESHIFT: ::core::ffi::c_int = _IOC_NRSHIFT + _IOC_NRBITS;
pub const _IOC_SIZESHIFT: ::core::ffi::c_int = _IOC_TYPESHIFT + _IOC_TYPEBITS;
pub const _IOC_DIRSHIFT: ::core::ffi::c_int = _IOC_SIZESHIFT + _IOC_SIZEBITS;
pub const CLOCK_MONOTONIC: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ENG_META_XATTR: [::core::ffi::c_char; 19] = unsafe {
    ::core::mem::transmute::<[u8; 19], [::core::ffi::c_char; 19]>(*b"user.workflow.meta\0")
};
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
static mut g_quiet: ::core::ffi::c_int = 0;
static mut g_err: [::core::ffi::c_char; 512] = [0; 512];

unsafe extern "C" fn src_fill(mut s: *mut src_t) -> ssize_t {
    (*s).out_len = 0 as size_t;
    (*s).out_pos = (*s).out_len;
    loop {
        if (*s).zin.pos == (*s).zin.size && (*s).in_eof == 0 {
            let mut n: ssize_t = 0;
            loop {
                n = read(
                    (*s).fd,
                    &raw mut (*s).r#in as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
                    ::core::mem::size_of::<[::core::ffi::c_uchar; 131072]>(),
                );
                if !(n < 0 as ssize_t && *__errno() == EINTR) {
                    break;
                }
            }
            if n < 0 as ssize_t {
                return -1 as ssize_t;
            }
            if n == 0 as ssize_t {
                (*s).in_eof = 1 as ::core::ffi::c_int;
            } else {
                eng_sha256_update(
                    &raw mut (*s).sha,
                    &raw mut (*s).r#in as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
                    n as size_t,
                );
                (*s).in_bytes = (*s).in_bytes.wrapping_add(n as uint64_t);
                (*s).zin.src =
                    &raw mut (*s).r#in as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void;
                (*s).zin.size = n as size_t;
                (*s).zin.pos = 0 as size_t;
            }
        }
        if (*s).zin.pos == (*s).zin.size && (*s).in_eof != 0 {
            return 0 as ssize_t;
        }
        let mut zo: ZSTD_outBuffer = ZSTD_outBuffer_s {
            dst: &raw mut (*s).out as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
            size: ::core::mem::size_of::<[::core::ffi::c_uchar; 131072]>(),
            pos: 0 as size_t,
        };
        let mut r: size_t = ZSTD_decompressStream((*s).dctx, &raw mut zo, &raw mut (*s).zin);
        if ZSTD_isError(r) != 0 {
            *__errno() = EBADMSG;
            return -1 as ssize_t;
        }
        (*s).last_ret = r;
        if zo.pos != 0 {
            (*s).out_len = zo.pos;
            return zo.pos as ssize_t;
        }
    }
}
unsafe extern "C" fn src_read(
    mut s: *mut src_t,
    mut buf: *mut ::core::ffi::c_void,
    mut n: size_t,
) -> ::core::ffi::c_int {
    let mut b: *mut ::core::ffi::c_uchar = buf as *mut ::core::ffi::c_uchar;
    while n != 0 {
        if (*s).out_pos == (*s).out_len {
            let mut k: ssize_t = src_fill(s);
            if k <= 0 as ssize_t {
                if k == 0 as ssize_t {
                    *__errno() = EPIPE;
                }
                return -1 as ::core::ffi::c_int;
            }
        }
        let mut take: size_t = if (*s).out_len.wrapping_sub((*s).out_pos) < n {
            (*s).out_len.wrapping_sub((*s).out_pos)
        } else {
            n
        };
        if !b.is_null() {
            memcpy(
                b as *mut ::core::ffi::c_void,
                (&raw mut (*s).out as *mut ::core::ffi::c_uchar).offset((*s).out_pos as isize)
                    as *const ::core::ffi::c_void,
                take,
            );
            b = b.offset(take as isize);
        }
        (*s).out_pos = (*s).out_pos.wrapping_add(take);
        n = n.wrapping_sub(take);
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn octal(
    mut p: *const ::core::ffi::c_char,
    mut n: size_t,
    mut ok: *mut ::core::ffi::c_int,
) -> uint64_t {
    let mut v: uint64_t = 0 as uint64_t;
    let mut i: size_t = 0 as size_t;
    while i < n
        && (*p.offset(i as isize) as ::core::ffi::c_int == ' ' as ::core::ffi::c_int
            || *p.offset(i as isize) as ::core::ffi::c_int == 0 as ::core::ffi::c_int)
    {
        i = i.wrapping_add(1);
    }
    while i < n
        && *p.offset(i as isize) as ::core::ffi::c_int >= '0' as ::core::ffi::c_int
        && *p.offset(i as isize) as ::core::ffi::c_int <= '7' as ::core::ffi::c_int
    {
        v = v.wrapping_mul(8 as uint64_t).wrapping_add(
            (*p.offset(i as isize) as ::core::ffi::c_int - '0' as ::core::ffi::c_int) as uint64_t,
        );
        i = i.wrapping_add(1);
    }
    while i < n {
        if *p.offset(i as isize) as ::core::ffi::c_int != ' ' as ::core::ffi::c_int
            && *p.offset(i as isize) as ::core::ffi::c_int != 0 as ::core::ffi::c_int
        {
            *ok = 0 as ::core::ffi::c_int;
        }
        i = i.wrapping_add(1);
    }
    return v;
}
unsafe extern "C" fn valid_utf8(
    mut s: *const ::core::ffi::c_uchar,
    mut n: size_t,
) -> ::core::ffi::c_int {
    let mut i: size_t = 0 as size_t;
    while i < n {
        let mut c: ::core::ffi::c_uchar = *s.offset(i as isize);
        if (c as ::core::ffi::c_int) < 0x80 as ::core::ffi::c_int {
            i = i.wrapping_add(1);
        } else {
            let mut k: size_t = 0;
            let mut cp: ::core::ffi::c_uint = 0;
            if c as ::core::ffi::c_int & 0xe0 as ::core::ffi::c_int == 0xc0 as ::core::ffi::c_int {
                k = 1 as size_t;
                cp = (c as ::core::ffi::c_int & 0x1f as ::core::ffi::c_int) as ::core::ffi::c_uint;
            } else if c as ::core::ffi::c_int & 0xf0 as ::core::ffi::c_int
                == 0xe0 as ::core::ffi::c_int
            {
                k = 2 as size_t;
                cp = (c as ::core::ffi::c_int & 0xf as ::core::ffi::c_int) as ::core::ffi::c_uint;
            } else if c as ::core::ffi::c_int & 0xf8 as ::core::ffi::c_int
                == 0xf0 as ::core::ffi::c_int
            {
                k = 3 as size_t;
                cp = (c as ::core::ffi::c_int & 0x7 as ::core::ffi::c_int) as ::core::ffi::c_uint;
            } else {
                return 0 as ::core::ffi::c_int;
            }
            if i.wrapping_add(k) >= n {
                return 0 as ::core::ffi::c_int;
            }
            let mut j: size_t = 1 as size_t;
            while j <= k {
                if *s.offset(i.wrapping_add(j) as isize) as ::core::ffi::c_int
                    & 0xc0 as ::core::ffi::c_int
                    != 0x80 as ::core::ffi::c_int
                {
                    return 0 as ::core::ffi::c_int;
                }
                cp = cp << 6 as ::core::ffi::c_int
                    | (*s.offset(i.wrapping_add(j) as isize) as ::core::ffi::c_int
                        & 0x3f as ::core::ffi::c_int) as ::core::ffi::c_uint;
                j = j.wrapping_add(1);
            }
            if k == 1 as size_t && cp < 0x80 as ::core::ffi::c_uint
                || k == 2 as size_t && cp < 0x800 as ::core::ffi::c_uint
                || k == 3 as size_t
                    && (cp < 0x10000 as ::core::ffi::c_int as ::core::ffi::c_uint
                        || cp > 0x10ffff as ::core::ffi::c_int as ::core::ffi::c_uint)
                || cp >= 0xd800 as ::core::ffi::c_uint && cp <= 0xdfff as ::core::ffi::c_uint
            {
                return 0 as ::core::ffi::c_int;
            }
            i = i.wrapping_add(k.wrapping_add(1 as size_t));
        }
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn pax_apply(
    mut d: *mut ::core::ffi::c_char,
    mut n: size_t,
    mut m: *mut member_t,
    mut has_size: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut i: size_t = 0 as size_t;
    while i < n {
        let mut end: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
        let mut len: ::core::ffi::c_ulong =
            strtoul(d.offset(i as isize), &raw mut end, 10 as ::core::ffi::c_int);
        if end == d.offset(i as isize)
            || *end as ::core::ffi::c_int != ' ' as ::core::ffi::c_int
            || len < 5 as ::core::ffi::c_ulong
            || i.wrapping_add(len as size_t) > n
            || *d.offset(i.wrapping_add(len as size_t).wrapping_sub(1 as size_t) as isize)
                as ::core::ffi::c_int
                != '\n' as ::core::ffi::c_int
        {
            return -1 as ::core::ffi::c_int;
        }
        let mut kv: *mut ::core::ffi::c_char = end.offset(1 as ::core::ffi::c_int as isize);
        let mut rec_end: *mut ::core::ffi::c_char = d
            .offset(i as isize)
            .offset(len as isize)
            .offset(-(1 as ::core::ffi::c_int as isize));
        let mut eq: *mut ::core::ffi::c_char = memchr(
            kv as *const ::core::ffi::c_void,
            '=' as ::core::ffi::c_int,
            rec_end.offset_from(kv) as size_t,
        ) as *mut ::core::ffi::c_char;
        if eq.is_null() {
            return -1 as ::core::ffi::c_int;
        }
        *eq = 0 as ::core::ffi::c_char;
        *rec_end = 0 as ::core::ffi::c_char;
        let mut val: *const ::core::ffi::c_char = eq.offset(1 as ::core::ffi::c_int as isize);
        let mut vl: size_t = rec_end.offset_from(val) as size_t;
        if strcmp(kv, b"path\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            if vl >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
                || valid_utf8(val as *const ::core::ffi::c_uchar, vl) == 0
            {
                return -1 as ::core::ffi::c_int;
            }
            memcpy(
                &raw mut (*m).path as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                val as *const ::core::ffi::c_void,
                vl.wrapping_add(1 as size_t),
            );
        } else if strcmp(kv, b"linkpath\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            if vl >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
                || valid_utf8(val as *const ::core::ffi::c_uchar, vl) == 0
            {
                return -1 as ::core::ffi::c_int;
            }
            memcpy(
                &raw mut (*m).link as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                val as *const ::core::ffi::c_void,
                vl.wrapping_add(1 as size_t),
            );
        } else if strcmp(kv, b"size\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            (*m).size = strtoull(
                val,
                ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                10 as ::core::ffi::c_int,
            ) as uint64_t;
            *has_size = 1 as ::core::ffi::c_int;
        } else if strcmp(kv, b"mtime\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            (*m).mtime = strtoll(
                val,
                ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                10 as ::core::ffi::c_int,
            ) as int64_t;
        } else {
            return -1 as ::core::ffi::c_int;
        }
        i = (i as ::core::ffi::c_ulong).wrapping_add(len) as size_t;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn tar_next(mut s: *mut src_t, mut m: *mut member_t) -> ::core::ffi::c_int {
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
    let mut pax_size: uint64_t = 0 as uint64_t;
    let mut pax_mtime: int64_t = -1 as int64_t;
    let mut have_pax: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut pax_has_size: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    loop {
        let mut h: [::core::ffi::c_uchar; 512] = [0; 512];
        if src_read(
            s,
            &raw mut h as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
            512 as size_t,
        ) != 0
        {
            return -1 as ::core::ffi::c_int;
        }
        let mut zero: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < 512 as ::core::ffi::c_int {
            if h[i as usize] != 0 {
                zero = 0 as ::core::ffi::c_int;
                break;
            } else {
                i += 1;
            }
        }
        if zero != 0 {
            if src_read(
                s,
                &raw mut h as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
                512 as size_t,
            ) != 0
            {
                return -1 as ::core::ffi::c_int;
            }
            let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_0 < 512 as ::core::ffi::c_int {
                if h[i_0 as usize] != 0 {
                    return -5 as ::core::ffi::c_int;
                }
                i_0 += 1;
            }
            loop {
                if (*s).out_pos == (*s).out_len {
                    let mut k: ssize_t = src_fill(s);
                    if k < 0 as ssize_t {
                        return -1 as ::core::ffi::c_int;
                    }
                    if k == 0 as ssize_t {
                        break;
                    }
                }
                while (*s).out_pos < (*s).out_len {
                    if (*s).out[(*s).out_pos] != 0 {
                        return -5 as ::core::ffi::c_int;
                    }
                    (*s).out_pos = (*s).out_pos.wrapping_add(1);
                }
            }
            return 0 as ::core::ffi::c_int;
        }
        let mut sum: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
        let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i_1 < 512 as ::core::ffi::c_int {
            sum = sum.wrapping_add(
                (if i_1 >= 148 as ::core::ffi::c_int && i_1 < 156 as ::core::ffi::c_int {
                    ' ' as ::core::ffi::c_int
                } else {
                    h[i_1 as usize] as ::core::ffi::c_int
                }) as ::core::ffi::c_uint,
            );
            i_1 += 1;
        }
        let mut ok: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
        let mut hsum: ::core::ffi::c_uint = octal(
            (&raw mut h as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_char)
                .offset(148 as ::core::ffi::c_int as isize),
            8 as size_t,
            &raw mut ok,
        ) as ::core::ffi::c_uint;
        if ok == 0 || hsum != sum {
            return -2 as ::core::ffi::c_int;
        }
        if memcmp(
            (&raw mut h as *mut ::core::ffi::c_uchar).offset(257 as ::core::ffi::c_int as isize)
                as *const ::core::ffi::c_void,
            b"ustar\x0000\0".as_ptr() as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            8 as size_t,
        ) != 0
        {
            return -2 as ::core::ffi::c_int;
        }
        memset(
            m as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<member_t>(),
        );
        (*m).r#type = h[156usize] as ::core::ffi::c_char;
        (*m).size = octal(
            (&raw mut h as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_char)
                .offset(124 as ::core::ffi::c_int as isize),
            12 as size_t,
            &raw mut ok,
        );
        (*m).mtime = octal(
            (&raw mut h as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_char)
                .offset(136 as ::core::ffi::c_int as isize),
            12 as size_t,
            &raw mut ok,
        ) as int64_t;
        if ok == 0 {
            return -2 as ::core::ffi::c_int;
        }
        let mut name: [::core::ffi::c_char; 101] = [0; 101];
        let mut prefix: [::core::ffi::c_char; 156] = [0; 156];
        memcpy(
            &raw mut name as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            &raw mut h as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
            100 as size_t,
        );
        name[100usize] = 0 as ::core::ffi::c_char;
        memcpy(
            &raw mut prefix as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            (&raw mut h as *mut ::core::ffi::c_uchar).offset(345 as ::core::ffi::c_int as isize)
                as *const ::core::ffi::c_void,
            155 as size_t,
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
            (&raw mut h as *mut ::core::ffi::c_uchar).offset(157 as ::core::ffi::c_int as isize)
                as *const ::core::ffi::c_void,
            100 as size_t,
        );
        (*m).link[100usize] = 0 as ::core::ffi::c_char;
        if (*m).r#type as ::core::ffi::c_int == 'x' as ::core::ffi::c_int {
            if have_pax != 0
                || (*m).size > ((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int) as uint64_t
            {
                return -2 as ::core::ffi::c_int;
            }
            let mut n: size_t = (*m).size as size_t;
            let mut d: *mut ::core::ffi::c_char =
                malloc(n.wrapping_add(1 as size_t)) as *mut ::core::ffi::c_char;
            if d.is_null()
                || src_read(s, d as *mut ::core::ffi::c_void, n) != 0
                || src_read(
                    s,
                    NULL,
                    (512 as size_t)
                        .wrapping_sub(n.wrapping_rem(512 as size_t))
                        .wrapping_rem(512 as size_t),
                ) != 0
            {
                free(d as *mut ::core::ffi::c_void);
                return -1 as ::core::ffi::c_int;
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
                0 as ::core::ffi::c_int,
                ::core::mem::size_of::<member_t>(),
            );
            px.mtime = -1 as int64_t;
            let mut rc: ::core::ffi::c_int = pax_apply(d, n, &raw mut px, &raw mut pax_has_size);
            free(d as *mut ::core::ffi::c_void);
            if rc != 0 {
                return -3 as ::core::ffi::c_int;
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
            have_pax = 1 as ::core::ffi::c_int;
        } else {
            if (*m).r#type as ::core::ffi::c_int != '0' as ::core::ffi::c_int
                && (*m).r#type as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                && (*m).r#type as ::core::ffi::c_int != '5' as ::core::ffi::c_int
                && (*m).r#type as ::core::ffi::c_int != '2' as ::core::ffi::c_int
            {
                return -4 as ::core::ffi::c_int;
            }
            if (*m).r#type as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
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
                if pax_mtime >= 0 as int64_t {
                    (*m).mtime = pax_mtime;
                }
            }
            let mut pl: size_t = strlen(&raw mut (*m).path as *mut ::core::ffi::c_char);
            if (*m).r#type as ::core::ffi::c_int == '5' as ::core::ffi::c_int
                && pl > 1 as size_t
                && (*m).path[pl.wrapping_sub(1 as size_t)] as ::core::ffi::c_int
                    == '/' as ::core::ffi::c_int
            {
                (*m).path[pl.wrapping_sub(1 as size_t)] = 0 as ::core::ffi::c_char;
            }
            if valid_utf8(
                &raw mut (*m).path as *mut ::core::ffi::c_char as *mut ::core::ffi::c_uchar,
                strlen(&raw mut (*m).path as *mut ::core::ffi::c_char),
            ) == 0
            {
                return -3 as ::core::ffi::c_int;
            }
            if (*m).r#type as ::core::ffi::c_int != '0' as ::core::ffi::c_int && (*m).size != 0 {
                return -2 as ::core::ffi::c_int;
            }
            return 1 as ::core::ffi::c_int;
        }
    }
}
unsafe extern "C" fn read_member_data(
    mut s: *mut src_t,
    mut m: *const member_t,
    mut out: *mut *mut ::core::ffi::c_char,
    mut max: size_t,
) -> ::core::ffi::c_int {
    if (*m).size > max as uint64_t {
        return -1 as ::core::ffi::c_int;
    }
    let mut n: size_t = (*m).size as size_t;
    let mut d: *mut ::core::ffi::c_char =
        malloc(n.wrapping_add(1 as size_t)) as *mut ::core::ffi::c_char;
    if d.is_null() {
        return -1 as ::core::ffi::c_int;
    }
    if src_read(s, d as *mut ::core::ffi::c_void, n) != 0
        || src_read(
            s,
            NULL,
            (512 as size_t)
                .wrapping_sub(n.wrapping_rem(512 as size_t))
                .wrapping_rem(512 as size_t),
        ) != 0
    {
        free(d as *mut ::core::ffi::c_void);
        return -1 as ::core::ffi::c_int;
    }
    *d.offset(n as isize) = 0 as ::core::ffi::c_char;
    *out = d;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn hstr(mut s: *const ::core::ffi::c_char, mut n: size_t) -> uint64_t {
    let mut h: uint64_t = 1469598103934665603 as uint64_t;
    let mut i: size_t = 0 as size_t;
    while i < n {
        h ^= *s.offset(i as isize) as ::core::ffi::c_uchar as uint64_t;
        h = (h as ::core::ffi::c_ulonglong).wrapping_mul(1099511628211 as ::core::ffi::c_ulonglong)
            as uint64_t;
        i = i.wrapping_add(1);
    }
    return h;
}
unsafe extern "C" fn find_row(
    mut a: *const attrs_t,
    mut p: *const ::core::ffi::c_char,
    mut n: size_t,
) -> ::core::ffi::c_int {
    let mut i: size_t = hstr(p, n) as size_t & (*a).hcap.wrapping_sub(1 as size_t);
    loop {
        let mut r: ::core::ffi::c_int = *(*a).hash.offset(i as isize);
        if r < 0 as ::core::ffi::c_int {
            return -1 as ::core::ffi::c_int;
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
        i = i.wrapping_add(1 as size_t) & (*a).hcap.wrapping_sub(1 as size_t);
    }
}
unsafe extern "C" fn add_hash(mut a: *mut attrs_t, mut r: ::core::ffi::c_int) {
    let mut i: size_t = hstr(
        (*(*a).rows.offset(r as isize)).path,
        (*(*a).rows.offset(r as isize)).plen,
    ) as size_t
        & (*a).hcap.wrapping_sub(1 as size_t);
    loop {
        if *(*a).hash.offset(i as isize) < 0 as ::core::ffi::c_int {
            *(*a).hash.offset(i as isize) = r;
            return;
        }
        i = i.wrapping_add(1 as size_t) & (*a).hcap.wrapping_sub(1 as size_t);
    }
}
unsafe extern "C" fn unpct(
    mut r#in: *const ::core::ffi::c_char,
    mut n: size_t,
    mut out: *mut *mut ::core::ffi::c_char,
    mut olen: *mut size_t,
) -> ::core::ffi::c_int {
    let mut o: *mut ::core::ffi::c_char =
        malloc(n.wrapping_add(1 as size_t)) as *mut ::core::ffi::c_char;
    if o.is_null() {
        return -1 as ::core::ffi::c_int;
    }
    let mut k: size_t = 0 as size_t;
    let mut i: size_t = 0 as size_t;
    while i < n {
        let mut c: ::core::ffi::c_uchar = *r#in.offset(i as isize) as ::core::ffi::c_uchar;
        if c as ::core::ffi::c_int == '%' as ::core::ffi::c_int {
            if i.wrapping_add(2 as size_t) >= n {
                free(o as *mut ::core::ffi::c_void);
                return -1 as ::core::ffi::c_int;
            }
            let mut v: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            let mut j: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
            while j <= 2 as ::core::ffi::c_int {
                let mut h: ::core::ffi::c_char = *r#in.offset(i.wrapping_add(j as size_t) as isize);
                v <<= 4 as ::core::ffi::c_int;
                if h as ::core::ffi::c_int >= '0' as ::core::ffi::c_int
                    && h as ::core::ffi::c_int <= '9' as ::core::ffi::c_int
                {
                    v |= h as ::core::ffi::c_int - '0' as ::core::ffi::c_int;
                } else if h as ::core::ffi::c_int >= 'A' as ::core::ffi::c_int
                    && h as ::core::ffi::c_int <= 'F' as ::core::ffi::c_int
                {
                    v |= h as ::core::ffi::c_int - 'A' as ::core::ffi::c_int
                        + 10 as ::core::ffi::c_int;
                } else {
                    free(o as *mut ::core::ffi::c_void);
                    return -1 as ::core::ffi::c_int;
                }
                j += 1;
            }
            let c2rust_fresh3 = k;
            k = k.wrapping_add(1);
            *o.offset(c2rust_fresh3 as isize) = v as ::core::ffi::c_char;
            i = i.wrapping_add(2 as size_t);
        } else if c as ::core::ffi::c_int >= 0x21 as ::core::ffi::c_int
            && c as ::core::ffi::c_int <= 0x7e as ::core::ffi::c_int
        {
            let c2rust_fresh4 = k;
            k = k.wrapping_add(1);
            *o.offset(c2rust_fresh4 as isize) = c as ::core::ffi::c_char;
        } else {
            free(o as *mut ::core::ffi::c_void);
            return -1 as ::core::ffi::c_int;
        }
        i = i.wrapping_add(1);
    }
    *o.offset(k as isize) = 0 as ::core::ffi::c_char;
    *out = o;
    *olen = k;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn path_ok(
    mut p: *const ::core::ffi::c_char,
    mut n: size_t,
) -> ::core::ffi::c_int {
    if n == 1 as size_t && *p.offset(0isize) as ::core::ffi::c_int == '/' as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    if n == 0
        || *p.offset(0isize) as ::core::ffi::c_int != '/' as ::core::ffi::c_int
        || *p.offset(n.wrapping_sub(1 as size_t) as isize) as ::core::ffi::c_int
            == '/' as ::core::ffi::c_int
        || !memchr(p as *const ::core::ffi::c_void, 0 as ::core::ffi::c_int, n).is_null()
    {
        return 0 as ::core::ffi::c_int;
    }
    if valid_utf8(p as *const ::core::ffi::c_uchar, n) == 0 {
        return 0 as ::core::ffi::c_int;
    }
    let mut i: size_t = 1 as size_t;
    while i <= n {
        let mut j: size_t = i;
        while j < n && *p.offset(j as isize) as ::core::ffi::c_int != '/' as ::core::ffi::c_int {
            j = j.wrapping_add(1);
        }
        let mut cl: size_t = j.wrapping_sub(i);
        if cl == 0 as size_t
            || cl == 1 as size_t
                && *p.offset(i as isize) as ::core::ffi::c_int == '.' as ::core::ffi::c_int
            || cl == 2 as size_t
                && *p.offset(i as isize) as ::core::ffi::c_int == '.' as ::core::ffi::c_int
                && *p.offset(i.wrapping_add(1 as size_t) as isize) as ::core::ffi::c_int
                    == '.' as ::core::ffi::c_int
        {
            return 0 as ::core::ffi::c_int;
        }
        i = j.wrapping_add(1 as size_t);
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn parse_u32(
    mut s: *const ::core::ffi::c_char,
    mut max: uint32_t,
    mut out: *mut uint32_t,
) -> ::core::ffi::c_int {
    if *s == 0 {
        return -1 as ::core::ffi::c_int;
    }
    let mut v: uint64_t = 0 as uint64_t;
    while *s != 0 {
        if (*s as ::core::ffi::c_int) < '0' as ::core::ffi::c_int
            || *s as ::core::ffi::c_int > '9' as ::core::ffi::c_int
        {
            return -1 as ::core::ffi::c_int;
        }
        v = v
            .wrapping_mul(10 as uint64_t)
            .wrapping_add((*s as ::core::ffi::c_int - '0' as ::core::ffi::c_int) as uint64_t);
        if v > max as uint64_t {
            return -1 as ::core::ffi::c_int;
        }
        s = s.offset(1);
    }
    *out = v as uint32_t;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn parse_attrs(
    mut text: *mut ::core::ffi::c_char,
    mut len: size_t,
    mut a: *mut attrs_t,
) -> ::core::ffi::c_int {
    let hdr: [::core::ffi::c_char; 24] = ::core::mem::transmute::<
        [u8; 24],
        [::core::ffi::c_char; 24],
    >(*b"#workflow-attributes 1\n\0");
    if len < ::core::mem::size_of::<[::core::ffi::c_char; 24]>().wrapping_sub(1usize)
        || memcmp(
            text as *const ::core::ffi::c_void,
            &raw const hdr as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            ::core::mem::size_of::<[::core::ffi::c_char; 24]>().wrapping_sub(1 as size_t),
        ) != 0
    {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"attributes.tsv: bad header\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut cap: size_t = 0 as size_t;
    let mut i: size_t = 0 as size_t;
    while i < len {
        if *text.offset(i as isize) as ::core::ffi::c_int == '\n' as ::core::ffi::c_int {
            cap = cap.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    (*a).rows = calloc(
        cap.wrapping_add(1 as size_t),
        ::core::mem::size_of::<row_t>(),
    ) as *mut row_t;
    (*a).hcap = 1 as size_t;
    while (*a).hcap < cap.wrapping_mul(2 as size_t).wrapping_add(2 as size_t) {
        (*a).hcap <<= 1 as ::core::ffi::c_int;
    }
    (*a).hash = malloc(
        (*a).hcap
            .wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>()),
    ) as *mut ::core::ffi::c_int;
    if (*a).rows.is_null() || (*a).hash.is_null() {
        return install_fail!(
            74 as ::core::ffi::c_int,
            b"out of memory\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut i_0: size_t = 0 as size_t;
    while i_0 < (*a).hcap {
        *(*a).hash.offset(i_0 as isize) = -1 as ::core::ffi::c_int;
        i_0 = i_0.wrapping_add(1);
    }
    let mut p: *mut ::core::ffi::c_char = text
        .offset(::core::mem::size_of::<[::core::ffi::c_char; 24]>() as isize)
        .offset(-(1 as ::core::ffi::c_int as isize));
    let mut end: *mut ::core::ffi::c_char = text.offset(len as isize);
    if len != 0
        && *text.offset(len.wrapping_sub(1 as size_t) as isize) as ::core::ffi::c_int
            != '\n' as ::core::ffi::c_int
    {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"attributes.tsv: missing final newline\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    while p < end {
        let mut nl: *mut ::core::ffi::c_char = memchr(
            p as *const ::core::ffi::c_void,
            '\n' as ::core::ffi::c_int,
            end.offset_from(p) as size_t,
        ) as *mut ::core::ffi::c_char;
        *nl = 0 as ::core::ffi::c_char;
        let mut f: [*mut ::core::ffi::c_char; 6] =
            [::core::ptr::null_mut::<::core::ffi::c_char>(); 6];
        let mut nf: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        let mut q: *mut ::core::ffi::c_char = p;
        while nf < 6 as ::core::ffi::c_int {
            let c2rust_fresh2 = nf;
            nf += 1;
            f[c2rust_fresh2 as usize] = q;
            let mut tab: *mut ::core::ffi::c_char = strchr(q, '\t' as ::core::ffi::c_int);
            if tab.is_null() {
                break;
            }
            *tab = 0 as ::core::ffi::c_char;
            q = tab.offset(1 as ::core::ffi::c_int as isize);
        }
        let mut idx: size_t = (*a).n;
        if nf != 6 as ::core::ffi::c_int || !strchr(f[5usize], '\t' as ::core::ffi::c_int).is_null()
        {
            return install_fail!(
                65 as ::core::ffi::c_int,
                b"attributes.tsv row %zu: need 6 fields\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as size_t),
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
                65 as ::core::ffi::c_int,
                b"attributes.tsv row %zu: bad path\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as size_t),
            );
        }
        if strlen(f[1usize]) != 1 as size_t
            || strchr(
                b"dflhcbp\0".as_ptr() as *const ::core::ffi::c_char,
                *f[1usize].offset(0isize) as ::core::ffi::c_int,
            )
            .is_null()
        {
            return install_fail!(
                65 as ::core::ffi::c_int,
                b"row %zu: bad type\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as size_t),
            );
        }
        (*r).r#type = *f[1usize].offset(0isize);
        if parse_u32(f[2usize], 4294967294 as uint32_t, &raw mut (*r).uid) != 0
            || parse_u32(f[3usize], 4294967294 as uint32_t, &raw mut (*r).gid) != 0
        {
            return install_fail!(
                65 as ::core::ffi::c_int,
                b"row %zu: bad uid/gid\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as size_t),
            );
        }
        if strlen(f[4usize]) != 4 as size_t
            || strspn(
                f[4usize],
                b"01234567\0".as_ptr() as *const ::core::ffi::c_char,
            ) != 4 as ::core::ffi::c_ulong
        {
            return install_fail!(
                65 as ::core::ffi::c_int,
                b"row %zu: bad mode\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as size_t),
            );
        }
        (*r).mode = strtoul(
            f[4usize],
            ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
            8 as ::core::ffi::c_int,
        ) as uint32_t;
        if (*r).r#type as ::core::ffi::c_int == 'l' as ::core::ffi::c_int
            && (*r).mode != 0o777 as uint32_t
        {
            return install_fail!(
                65 as ::core::ffi::c_int,
                b"row %zu: symlink mode must be 0777\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as size_t),
            );
        }
        (*r).store = -1 as ::core::ffi::c_int;
        (*r).primary = -1 as ::core::ffi::c_int;
        if (*r).r#type as ::core::ffi::c_int == 'h' as ::core::ffi::c_int {
            let mut el: size_t = 0;
            if unpct(
                f[5usize],
                strlen(f[5usize]),
                &raw mut (*r).extra,
                &raw mut el,
            ) != 0
                || path_ok((*r).extra, el) == 0
            {
                return install_fail!(
                    65 as ::core::ffi::c_int,
                    b"row %zu: bad hardlink primary\0".as_ptr() as *const ::core::ffi::c_char,
                    idx.wrapping_add(1 as size_t),
                );
            }
        } else if (*r).r#type as ::core::ffi::c_int == 'c' as ::core::ffi::c_int
            || (*r).r#type as ::core::ffi::c_int == 'b' as ::core::ffi::c_int
        {
            if sscanf(
                f[5usize],
                b"%u,%u\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*r).major,
                &raw mut (*r).minor,
            ) != 2 as ::core::ffi::c_int
            {
                return install_fail!(
                    65 as ::core::ffi::c_int,
                    b"row %zu: bad device\0".as_ptr() as *const ::core::ffi::c_char,
                    idx.wrapping_add(1 as size_t),
                );
            }
        } else if strcmp(f[5usize], b"-\0".as_ptr() as *const ::core::ffi::c_char) != 0 {
            return install_fail!(
                65 as ::core::ffi::c_int,
                b"row %zu: extra must be '-'\0".as_ptr() as *const ::core::ffi::c_char,
                idx.wrapping_add(1 as size_t),
            );
        }
        if idx == 0 as size_t {
            if strcmp((*r).path, b"/\0".as_ptr() as *const ::core::ffi::c_char) != 0
                || (*r).r#type as ::core::ffi::c_int != 'd' as ::core::ffi::c_int
            {
                return install_fail!(
                    65 as ::core::ffi::c_int,
                    b"attributes.tsv: first row must be / (d)\0".as_ptr()
                        as *const ::core::ffi::c_char,
                );
            }
        } else {
            let mut pr: *const row_t = (*a).rows.offset(idx.wrapping_sub(1 as size_t) as isize);
            let mut m: size_t = if (*pr).plen < (*r).plen {
                (*pr).plen
            } else {
                (*r).plen
            };
            let mut c: ::core::ffi::c_int = memcmp(
                (*pr).path as *const ::core::ffi::c_void,
                (*r).path as *const ::core::ffi::c_void,
                m,
            );
            if c > 0 as ::core::ffi::c_int
                || c == 0 as ::core::ffi::c_int && (*pr).plen >= (*r).plen
            {
                return install_fail!(
                    65 as ::core::ffi::c_int,
                    b"row %zu: rows not strictly sorted\0".as_ptr() as *const ::core::ffi::c_char,
                    idx.wrapping_add(1 as size_t),
                );
            }
            let mut sl: *const ::core::ffi::c_char = strrchr((*r).path, '/' as ::core::ffi::c_int);
            let mut plen: size_t = if sl == (*r).path as *const ::core::ffi::c_char {
                1 as size_t
            } else {
                sl.offset_from((*r).path) as size_t
            };
            let mut pi: ::core::ffi::c_int = find_row(a, (*r).path, plen);
            if pi < 0 as ::core::ffi::c_int
                || (*(*a).rows.offset(pi as isize)).r#type as ::core::ffi::c_int
                    != 'd' as ::core::ffi::c_int
            {
                return install_fail!(
                    65 as ::core::ffi::c_int,
                    b"row %zu: parent is not a directory row\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    idx.wrapping_add(1 as size_t),
                );
            }
        }
        add_hash(a, idx as ::core::ffi::c_int);
        (*a).n = (*a).n.wrapping_add(1);
        p = nl.offset(1 as ::core::ffi::c_int as isize);
    }
    let mut i_1: size_t = 0 as size_t;
    while i_1 < (*a).n {
        let mut r_0: *mut row_t = (*a).rows.offset(i_1 as isize);
        if (*r_0).r#type as ::core::ffi::c_int == 'h' as ::core::ffi::c_int {
            let mut pi_0: ::core::ffi::c_int = find_row(a, (*r_0).extra, strlen((*r_0).extra));
            if pi_0 < 0 as ::core::ffi::c_int
                || pi_0 as size_t >= i_1
                || (*(*a).rows.offset(pi_0 as isize)).r#type as ::core::ffi::c_int
                    != 'f' as ::core::ffi::c_int
            {
                return install_fail!(
                    65 as ::core::ffi::c_int,
                    b"row %zu: hardlink primary must be an earlier f row\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    i_1.wrapping_add(1 as size_t),
                );
            }
            let mut pr_0: *const row_t = (*a).rows.offset(pi_0 as isize);
            if (*pr_0).uid != (*r_0).uid || (*pr_0).gid != (*r_0).gid || (*pr_0).mode != (*r_0).mode
            {
                return install_fail!(
                    65 as ::core::ffi::c_int,
                    b"row %zu: hardlink attributes differ from the primary\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    i_1.wrapping_add(1 as size_t),
                );
            }
            (*r_0).primary = pi_0;
        }
        i_1 = i_1.wrapping_add(1);
    }
    let mut s: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while s < (*a).nstores {
        let mut st: *mut store_t = (&raw mut (*a).stores as *mut store_t).offset(s as isize);
        let mut pi_1: ::core::ffi::c_int = find_row(
            a,
            &raw mut (*st).guest as *mut ::core::ffi::c_char,
            (*st).glen,
        );
        if pi_1 < 0 as ::core::ffi::c_int
            || (*(*a).rows.offset(pi_1 as isize)).r#type as ::core::ffi::c_int
                != 'd' as ::core::ffi::c_int
        {
            return install_fail!(
                65 as ::core::ffi::c_int,
                b"store prefix %s is not a directory row\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*st).guest as *mut ::core::ffi::c_char,
            );
        }
        let mut i_2: size_t = 0 as size_t;
        while i_2 < (*a).n {
            let mut r_1: *mut row_t = (*a).rows.offset(i_2 as isize);
            let mut below: ::core::ffi::c_int = ((*r_1).plen > (*st).glen
                && memcmp(
                    (*r_1).path as *const ::core::ffi::c_void,
                    &raw mut (*st).guest as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                    (*st).glen,
                ) == 0
                && *(*r_1).path.offset((*st).glen as isize) as ::core::ffi::c_int
                    == '/' as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            let mut to_store: ::core::ffi::c_int = ((*r_1).r#type as ::core::ffi::c_int
                == 'h' as ::core::ffi::c_int
                && strlen((*r_1).extra) > (*st).glen
                && memcmp(
                    (*r_1).extra as *const ::core::ffi::c_void,
                    &raw mut (*st).guest as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                    (*st).glen,
                ) == 0
                && *(*r_1).extra.offset((*st).glen as isize) as ::core::ffi::c_int
                    == '/' as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            if to_store != 0 {
                return install_fail!(
                    65 as ::core::ffi::c_int,
                    b"row %zu: hardlink into store %s\0".as_ptr() as *const ::core::ffi::c_char,
                    i_2.wrapping_add(1 as size_t),
                    &raw mut (*st).guest as *mut ::core::ffi::c_char,
                );
            }
            if below != 0 {
                if strchr(
                    b"dfl\0".as_ptr() as *const ::core::ffi::c_char,
                    (*r_1).r#type as ::core::ffi::c_int,
                )
                .is_null()
                    || (*r_1).uid != (*a).user_uid
                    || (*r_1).gid != (*a).user_gid
                    || (*r_1).mode & 0o7000 as uint32_t != 0
                {
                    return install_fail!(
                        65 as ::core::ffi::c_int,
                        b"row %zu: store entries must be d/f/l owned by the user without set-id\0"
                            .as_ptr() as *const ::core::ffi::c_char,
                        i_2.wrapping_add(1 as size_t),
                    );
                }
                (*r_1).store = s;
            }
            i_2 = i_2.wrapping_add(1);
        }
        s += 1;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn check_metadata(
    mut text: *const ::core::ffi::c_char,
    mut len: size_t,
    mut profile: *const ::core::ffi::c_char,
    mut mt: *mut meta_t,
    mut a: *mut attrs_t,
) -> ::core::ffi::c_int {
    let mut req: *const eng_json = ::core::ptr::null::<eng_json>();
    let mut at: *const eng_json = ::core::ptr::null::<eng_json>();
    let mut rf: *const eng_json = ::core::ptr::null::<eng_json>();
    let mut user: *const eng_json = ::core::ptr::null::<eng_json>();
    let mut uu: ::core::ffi::c_longlong = 0;
    let mut ug: ::core::ffi::c_longlong = 0;
    let mut st: *const eng_json = ::core::ptr::null::<eng_json>();
    let mut j: *mut eng_json = eng_json_parse(text, len);
    if j.is_null() || (*j).t.0 != eng_jtype::ENG_J_OBJ.0 {
        eng_json_free(j);
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"metadata.json: not a JSON object\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut rc: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut v: ::core::ffi::c_longlong = 0;
    '_out: {
        s = eng_json_str(j, b"format\0".as_ptr() as *const ::core::ffi::c_char);
        if s.is_null()
            || strcmp(
                s,
                b"workflow-image\0".as_ptr() as *const ::core::ffi::c_char,
            ) != 0
        {
            rc = install_fail!(
                65 as ::core::ffi::c_int,
                b"metadata: format is not workflow-image\0".as_ptr() as *const ::core::ffi::c_char,
            );
        } else if eng_json_int(
            j,
            b"formatVersion\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut v,
        ) != 0
            || v != 2 as ::core::ffi::c_longlong
        {
            rc = install_fail!(
                65 as ::core::ffi::c_int,
                b"metadata: formatVersion must be 2\0".as_ptr() as *const ::core::ffi::c_char,
            );
        } else {
            s = eng_json_str(j, b"type\0".as_ptr() as *const ::core::ffi::c_char);
            if s.is_null()
                || strcmp(s, b"debian-trixie\0".as_ptr() as *const ::core::ffi::c_char) != 0
            {
                rc = install_fail!(
                    65 as ::core::ffi::c_int,
                    b"metadata: unknown image type\0".as_ptr() as *const ::core::ffi::c_char,
                );
            } else if eng_json_int(
                j,
                b"typeVersion\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut v,
            ) != 0
                || v != 1 as ::core::ffi::c_longlong
            {
                rc = install_fail!(
                    65 as ::core::ffi::c_int,
                    b"metadata: unsupported typeVersion\0".as_ptr() as *const ::core::ffi::c_char,
                );
            } else {
                s = eng_json_str(j, b"profile\0".as_ptr() as *const ::core::ffi::c_char);
                if s.is_null()
                    || strcmp(s, b"workspace\0".as_ptr() as *const ::core::ffi::c_char) != 0
                        && strcmp(s, b"base\0".as_ptr() as *const ::core::ffi::c_char) != 0
                {
                    rc = install_fail!(
                        65 as ::core::ffi::c_int,
                        b"metadata: bad profile\0".as_ptr() as *const ::core::ffi::c_char,
                    );
                } else if strcmp(profile, b"any\0".as_ptr() as *const ::core::ffi::c_char) != 0
                    && strcmp(profile, s) != 0
                {
                    rc = install_fail!(
                        65 as ::core::ffi::c_int,
                        b"metadata: profile %s, expected %s\0".as_ptr()
                            as *const ::core::ffi::c_char,
                        s,
                        profile,
                    );
                } else {
                    s = eng_json_str(j, b"architecture\0".as_ptr() as *const ::core::ffi::c_char);
                    if s.is_null() || strcmp(s, HOST_DEB_ARCH.as_ptr()) != 0 {
                        rc = install_fail!(
                            65 as ::core::ffi::c_int,
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
                                65 as ::core::ffi::c_int,
                                b"metadata: requires missing\0".as_ptr()
                                    as *const ::core::ffi::c_char,
                            );
                        } else {
                            let mut e: *const eng_json = (*req).child;
                            while !e.is_null() {
                                let mut known: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                                let mut i: size_t = 0 as size_t;
                                while (*e).t.0 == eng_jtype::ENG_J_STR.0
                                    && i < ::core::mem::size_of::<[*const ::core::ffi::c_char; 5]>()
                                        .wrapping_div(::core::mem::size_of::<
                                            *const ::core::ffi::c_char,
                                        >())
                                {
                                    if strcmp((*e).s, IMPLEMENTED[i]) == 0 {
                                        known = 1 as ::core::ffi::c_int;
                                        (*mt).requires |= (1 as ::core::ffi::c_uint) << i;
                                    }
                                    i = i.wrapping_add(1);
                                }
                                if known == 0 {
                                    rc = install_fail!(
                                        65 as ::core::ffi::c_int,
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
                            if (*mt).requires
                                & (C2Rust_Unnamed::REQ_OWNERSHIP.0 as ::core::ffi::c_int
                                    | C2Rust_Unnamed::REQ_MODE.0 as ::core::ffi::c_int)
                                    as ::core::ffi::c_uint
                                != (C2Rust_Unnamed::REQ_OWNERSHIP.0 as ::core::ffi::c_int
                                    | C2Rust_Unnamed::REQ_MODE.0 as ::core::ffi::c_int)
                                    as ::core::ffi::c_uint
                            {
                                rc = install_fail!(
                                    65 as ::core::ffi::c_int,
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
                                    || v != 1 as ::core::ffi::c_longlong
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
                                    || strlen(s) != 64 as size_t
                                {
                                    rc = install_fail!(
                                        65 as ::core::ffi::c_int,
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
                                            65 as ::core::ffi::c_int,
                                            b"metadata: bad rootfs descriptor\0".as_ptr()
                                                as *const ::core::ffi::c_char,
                                        );
                                    } else {
                                        user = eng_json_get(
                                            j,
                                            b"user\0".as_ptr() as *const ::core::ffi::c_char,
                                        );
                                        uu = 1000 as ::core::ffi::c_longlong;
                                        ug = 1000 as ::core::ffi::c_longlong;
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
                                        (*a).user_uid = uu as uint32_t;
                                        (*a).user_gid = ug as uint32_t;
                                        st = eng_json_get(
                                            j,
                                            b"stores\0".as_ptr() as *const ::core::ffi::c_char,
                                        );
                                        if !st.is_null() {
                                            if (*st).t.0 != eng_jtype::ENG_J_OBJ.0 {
                                                rc = install_fail!(
                                                    65 as ::core::ffi::c_int,
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
                                                    if (*a).nstores >= 8 as ::core::ffi::c_int
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
                                                        || *name.offset(0isize)
                                                            as ::core::ffi::c_int
                                                            == '/' as ::core::ffi::c_int
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
                                                            65 as ::core::ffi::c_int,
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
    mut base: ::core::ffi::c_int,
    mut dir: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if (*x).cache_fd >= 0 as ::core::ffi::c_int
        && (*x).cache_base == base
        && strcmp(&raw mut (*x).cache_dir as *mut ::core::ffi::c_char, dir) == 0
    {
        return (*x).cache_fd;
    }
    if (*x).cache_fd >= 0 as ::core::ffi::c_int {
        close((*x).cache_fd);
    }
    (*x).cache_fd = -1 as ::core::ffi::c_int;
    let mut fd: ::core::ffi::c_int = dup(base);
    if fd < 0 as ::core::ffi::c_int {
        return -1 as ::core::ffi::c_int;
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
        let mut n: ::core::ffi::c_int =
            openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        if n < 0 as ::core::ffi::c_int {
            return -1 as ::core::ffi::c_int;
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
    mut fd: ::core::ffi::c_int,
    mut uid: uint32_t,
    mut gid: uint32_t,
    mut mode: uint32_t,
    mut nlink: uint32_t,
    mut maj: uint32_t,
    mut min: uint32_t,
) -> ::core::ffi::c_int {
    let mut v: [::core::ffi::c_char; 128] = [0; 128];
    let mut n: ::core::ffi::c_int = snprintf(
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
        n as size_t,
        0 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn locate(
    mut x: *const ex_t,
    mut a: *const attrs_t,
    mut r: *const row_t,
    mut base: *mut ::core::ffi::c_int,
    mut dir: *mut ::core::ffi::c_char,
    mut dcap: size_t,
) {
    let mut tmp: [::core::ffi::c_char; 4200] = [0; 4200];
    if (*r).store >= 0 as ::core::ffi::c_int {
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
            (*r).path.offset(1 as ::core::ffi::c_int as isize),
        );
        *base = (*x).base_root;
    }
    let mut sl: *mut ::core::ffi::c_char = strrchr(
        &raw mut tmp as *mut ::core::ffi::c_char,
        '/' as ::core::ffi::c_int,
    );
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
unsafe extern "C" fn mkdirs_rel(
    mut base: ::core::ffi::c_int,
    mut rel: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        rel,
    );
    let mut fd: ::core::ffi::c_int = dup(base);
    let mut save: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut c: *mut ::core::ffi::c_char = strtok_r(
        &raw mut buf as *mut ::core::ffi::c_char,
        b"/\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut save,
    );
    while !c.is_null() && fd >= 0 as ::core::ffi::c_int {
        if mkdirat(fd, c, 0o700 as mode_t) != 0 as ::core::ffi::c_int && *__errno() != EEXIST {
            close(fd);
            return -1 as ::core::ffi::c_int;
        }
        let mut n: ::core::ffi::c_int =
            openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        fd = n;
        c = strtok_r(
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            b"/\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut save,
        );
    }
    if fd < 0 as ::core::ffi::c_int {
        return -1 as ::core::ffi::c_int;
    }
    close(fd);
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn rm_tree_at(
    mut dfd: ::core::ffi::c_int,
    mut name: *const ::core::ffi::c_char,
    mut depth: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if depth > 512 as ::core::ffi::c_int {
        return -ELOOP;
    }
    if unlinkat(dfd, name, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int
        || *__errno() == ENOENT
    {
        return 0 as ::core::ffi::c_int;
    }
    if *__errno() != EISDIR && *__errno() != EPERM {
        return -*__errno();
    }
    let mut fd: ::core::ffi::c_int =
        openat(dfd, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if fd < 0 as ::core::ffi::c_int {
        return -*__errno();
    }
    fchmod(fd, 0o700 as mode_t);
    let mut d: *mut DIR = fdopendir(fd);
    if d.is_null() {
        close(fd);
        return -*__errno();
    }
    let mut e: *mut dirent = ::core::ptr::null_mut::<dirent>();
    let mut rc: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
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
        let mut r: ::core::ffi::c_int = rm_tree_at(
            dirfd(d),
            &raw mut (*e).d_name as *mut ::core::ffi::c_char,
            depth + 1 as ::core::ffi::c_int,
        );
        if r != 0 && rc == 0 {
            rc = r;
        }
    }
    closedir(d);
    if unlinkat(dfd, name, AT_REMOVEDIR) != 0 as ::core::ffi::c_int && rc == 0 {
        rc = -*__errno();
    }
    return rc;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_remove_tree(
    mut path: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        path,
    );
    let mut n: size_t = strlen(&raw mut buf as *mut ::core::ffi::c_char);
    while n > 1 as size_t
        && buf[n.wrapping_sub(1 as size_t)] as ::core::ffi::c_int == '/' as ::core::ffi::c_int
    {
        n = n.wrapping_sub(1);
        buf[n] = 0 as ::core::ffi::c_char;
    }
    let mut sl: *mut ::core::ffi::c_char = strrchr(
        &raw mut buf as *mut ::core::ffi::c_char,
        '/' as ::core::ffi::c_int,
    );
    let mut dfd: ::core::ffi::c_int = 0;
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
        if dfd < 0 as ::core::ffi::c_int {
            return -*__errno();
        }
        name = sl.offset(1 as ::core::ffi::c_int as isize);
    }
    let mut rc: ::core::ffi::c_int = rm_tree_at(dfd, name, 0 as ::core::ffi::c_int);
    if dfd != AT_FDCWD {
        close(dfd);
    }
    return rc;
}
unsafe extern "C" fn now_s() -> ::core::ffi::c_double {
    let mut ts: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    clock_gettime(CLOCK_MONOTONIC, &raw mut ts);
    return ts.tv_sec as ::core::ffi::c_double + ts.tv_nsec as ::core::ffi::c_double / 1e9f64;
}
unsafe extern "C" fn do_install(
    mut o: *const eng_install_opts,
    mut s: *mut src_t,
    mut tfd: ::core::ffi::c_int,
    mut mt: *mut meta_t,
    mut a: *mut attrs_t,
    mut stats: *mut ::core::ffi::c_longlong,
) -> ::core::ffi::c_int {
    let mut m: member_t = member_t {
        path: [0; 4096],
        link: [0; 4096],
        r#type: 0,
        size: 0,
        mtime: 0,
    };
    let mut meta_text: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut attr_text: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut rc: ::core::ffi::c_int = 0;
    rc = tar_next(s, &raw mut m);
    if rc != 1 as ::core::ffi::c_int
        || m.r#type as ::core::ffi::c_int != '0' as ::core::ffi::c_int
        || strcmp(
            &raw mut m.path as *mut ::core::ffi::c_char,
            b"metadata.json\0".as_ptr() as *const ::core::ffi::c_char,
        ) != 0
    {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"image: first member must be metadata.json\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    if read_member_data(
        s,
        &raw mut m,
        &raw mut meta_text,
        ((64 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int) as size_t,
    ) != 0
    {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"image: metadata.json too large or truncated\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    rc = check_metadata(
        meta_text,
        m.size as size_t,
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
    if tar_next(s, &raw mut m) != 1 as ::core::ffi::c_int
        || m.r#type as ::core::ffi::c_int != '0' as ::core::ffi::c_int
        || strcmp(
            &raw mut m.path as *mut ::core::ffi::c_char,
            b"attributes.tsv\0".as_ptr() as *const ::core::ffi::c_char,
        ) != 0
    {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"image: second member must be attributes.tsv\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    if m.size as ::core::ffi::c_longlong != (*mt).size {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"attributes.tsv: size differs from metadata\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    if read_member_data(
        s,
        &raw mut m,
        &raw mut attr_text,
        ((64 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int) as size_t,
    ) != 0
    {
        return install_fail!(
            65 as ::core::ffi::c_int,
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
    let mut d: [uint8_t; 32] = [0; 32];
    let mut hex: [::core::ffi::c_char; 65] = [0; 65];
    eng_sha256_init(&raw mut c);
    eng_sha256_update(
        &raw mut c,
        attr_text as *const ::core::ffi::c_void,
        m.size as size_t,
    );
    eng_sha256_final(&raw mut c, &raw mut d as *mut uint8_t);
    eng_sha256_hex(
        &raw mut d as *mut uint8_t as *const uint8_t,
        &raw mut hex as *mut ::core::ffi::c_char,
    );
    if strcmp(
        &raw mut hex as *mut ::core::ffi::c_char,
        &raw mut (*mt).attr_sha as *mut ::core::ffi::c_char,
    ) != 0
    {
        free(attr_text as *mut ::core::ffi::c_void);
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"attributes.tsv: sha256 mismatch\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    rc = parse_attrs(attr_text, m.size as size_t, a);
    free(attr_text as *mut ::core::ffi::c_void);
    if rc != 0 {
        return rc;
    }
    if (*a).n as ::core::ffi::c_longlong != (*mt).rows
        || (*a).n as ::core::ffi::c_longlong != (*mt).entries
    {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"attributes.tsv: row count differs from metadata\0".as_ptr()
                as *const ::core::ffi::c_char,
        );
    }
    let mut mem: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut hard: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut special: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut i: size_t = 0 as size_t;
    while i < (*a).n {
        let mut t: ::core::ffi::c_char = (*(*a).rows.offset(i as isize)).r#type;
        if t as ::core::ffi::c_int == 'd' as ::core::ffi::c_int
            || t as ::core::ffi::c_int == 'f' as ::core::ffi::c_int
            || t as ::core::ffi::c_int == 'l' as ::core::ffi::c_int
        {
            mem += 1;
        } else if t as ::core::ffi::c_int == 'h' as ::core::ffi::c_int {
            hard += 1;
        } else {
            special += 1;
        }
        i = i.wrapping_add(1);
    }
    if mem != (*mt).members {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"attributes.tsv: %lld member rows, metadata says %lld\0".as_ptr()
                as *const ::core::ffi::c_char,
            mem,
            (*mt).members,
        );
    }
    if hard != 0 && (*mt).requires & C2Rust_Unnamed::REQ_HARDLINK.0 == 0 {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"metadata: hardlink rows without hardlink-emulation\0".as_ptr()
                as *const ::core::ffi::c_char,
        );
    }
    if special != 0 && (*mt).requires & C2Rust_Unnamed::REQ_SPECIAL.0 == 0 {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"metadata: special rows without virtual-special-files\0".as_ptr()
                as *const ::core::ffi::c_char,
        );
    }
    if (*a).nstores != 0 && (*mt).requires & C2Rust_Unnamed::REQ_SEEDS.0 == 0 {
        return install_fail!(
            65 as ::core::ffi::c_int,
            b"metadata: stores without store-seeds\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut vfs: statvfs = statvfs {
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
    };
    if fstatvfs(tfd, &raw mut vfs) == 0 as ::core::ffi::c_int {
        let mut need: ::core::ffi::c_ulonglong = ((*mt).regular_bytes as ::core::ffi::c_ulonglong)
            .wrapping_add(
                ((*mt).entries as ::core::ffi::c_ulonglong)
                    .wrapping_mul(4096 as ::core::ffi::c_ulonglong),
            );
        need = need.wrapping_add(need.wrapping_div(20 as ::core::ffi::c_ulonglong));
        let mut have: ::core::ffi::c_ulonglong = (vfs.f_bavail as ::core::ffi::c_ulonglong)
            .wrapping_mul(vfs.f_frsize as ::core::ffi::c_ulonglong);
        if have < need {
            return install_fail!(
                75 as ::core::ffi::c_int,
                b"not enough space: need %llu MB, have %llu MB\0".as_ptr()
                    as *const ::core::ffi::c_char,
                need >> 20 as ::core::ffi::c_int,
                have >> 20 as ::core::ffi::c_int,
            );
        }
    }
    install_note!(
        b"image valid: %zu rows, %lld members, %lld MB\0".as_ptr() as *const ::core::ffi::c_char,
        (*a).n,
        (*mt).members,
        (*mt).regular_bytes >> 20 as ::core::ffi::c_int,
    );
    let mut x: ex_t = ex_t {
        base_root: 0,
        base_seeds: 0,
        cache_dir: [0; 4096],
        cache_fd: -1 as ::core::ffi::c_int,
        cache_base: 0,
    };
    if mkdirat(
        tfd,
        b"rootfs\0".as_ptr() as *const ::core::ffi::c_char,
        0o700 as mode_t,
    ) != 0
        || {
            x.base_root = openat(
                tfd,
                b"rootfs\0".as_ptr() as *const ::core::ffi::c_char,
                O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
            );
            x.base_root < 0 as ::core::ffi::c_int
        }
    {
        return install_fail!(
            74 as ::core::ffi::c_int,
            b"create rootfs: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*__errno()),
        );
    }
    x.base_seeds = -1 as ::core::ffi::c_int;
    if (*a).nstores != 0 {
        if mkdirat(
            tfd,
            b"seeds\0".as_ptr() as *const ::core::ffi::c_char,
            0o700 as mode_t,
        ) != 0
            || {
                x.base_seeds = openat(
                    tfd,
                    b"seeds\0".as_ptr() as *const ::core::ffi::c_char,
                    O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
                );
                x.base_seeds < 0 as ::core::ffi::c_int
            }
        {
            return install_fail!(
                74 as ::core::ffi::c_int,
                b"create seeds: %s\0".as_ptr() as *const ::core::ffi::c_char,
                strerror(*__errno()),
            );
        }
        let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i_0 < (*a).nstores {
            if mkdirs_rel(
                x.base_seeds,
                &raw mut (*(&raw mut (*a).stores as *mut store_t).offset(i_0 as isize)).store
                    as *mut ::core::ffi::c_char,
            ) != 0
            {
                return install_fail!(
                    74 as ::core::ffi::c_int,
                    b"create seeds: %s\0".as_ptr() as *const ::core::ffi::c_char,
                    strerror(*__errno()),
                );
            }
            i_0 += 1;
        }
    }
    let mut dts: *mut dtime_t = calloc((*a).n, ::core::mem::size_of::<dtime_t>()) as *mut dtime_t;
    let mut ndt: size_t = 0 as size_t;
    if dts.is_null() {
        return install_fail!(
            74 as ::core::ffi::c_int,
            b"out of memory\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut ri: size_t = 0 as size_t;
    let mut members: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut seeds: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut regular: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut buf: *mut ::core::ffi::c_char =
        malloc(((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int) as size_t)
            as *mut ::core::ffi::c_char;
    if buf.is_null() {
        free(dts as *mut ::core::ffi::c_void);
        return install_fail!(
            74 as ::core::ffi::c_int,
            b"out of memory\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    let mut t0: ::core::ffi::c_double = now_s();
    let mut last: ::core::ffi::c_double = t0;
    rc = 0 as ::core::ffi::c_int;
    loop {
        let mut k: ::core::ffi::c_int = tar_next(s, &raw mut m);
        if k == 0 as ::core::ffi::c_int {
            break;
        }
        if k < 0 as ::core::ffi::c_int {
            rc = install_fail!(
                65 as ::core::ffi::c_int,
                b"image: %s after %lld members\0".as_ptr() as *const ::core::ffi::c_char,
                if k == -4 as ::core::ffi::c_int {
                    b"forbidden tar member type\0".as_ptr() as *const ::core::ffi::c_char
                } else if k == -3 as ::core::ffi::c_int {
                    b"bad PAX header or non-UTF-8 name\0".as_ptr() as *const ::core::ffi::c_char
                } else if k == -5 as ::core::ffi::c_int {
                    b"data after the end of the archive\0".as_ptr() as *const ::core::ffi::c_char
                } else if k == -1 as ::core::ffi::c_int {
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
                    (*(*a).rows.offset(ri as isize)).r#type as ::core::ffi::c_int,
                )
                .is_null()
            {
                ri = ri.wrapping_add(1);
            }
            if ri >= (*a).n {
                rc = install_fail!(
                    65 as ::core::ffi::c_int,
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
                    7 as size_t,
                ) == 0
                {
                    snprintf(
                        &raw mut gp as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4200]>(),
                        b"/%s\0".as_ptr() as *const ::core::ffi::c_char,
                        (&raw mut m.path as *mut ::core::ffi::c_char)
                            .offset(7 as ::core::ffi::c_int as isize),
                    );
                } else {
                    rc = install_fail!(
                        65 as ::core::ffi::c_int,
                        b"image: member outside rootfs: %s\0".as_ptr()
                            as *const ::core::ffi::c_char,
                        &raw mut m.path as *mut ::core::ffi::c_char,
                    );
                    break;
                }
                let mut mt_: ::core::ffi::c_char =
                    (if m.r#type as ::core::ffi::c_int == '5' as ::core::ffi::c_int {
                        'd' as ::core::ffi::c_int
                    } else if m.r#type as ::core::ffi::c_int == '2' as ::core::ffi::c_int {
                        'l' as ::core::ffi::c_int
                    } else {
                        'f' as ::core::ffi::c_int
                    }) as ::core::ffi::c_char;
                if strcmp(&raw mut gp as *mut ::core::ffi::c_char, (*r).path) != 0
                    || mt_ as ::core::ffi::c_int != (*r).r#type as ::core::ffi::c_int
                {
                    rc = install_fail!(
                        65 as ::core::ffi::c_int,
                        b"image: member %s does not match row %s (%c)\0".as_ptr()
                            as *const ::core::ffi::c_char,
                        &raw mut gp as *mut ::core::ffi::c_char,
                        (*r).path,
                        (*r).r#type as ::core::ffi::c_int,
                    );
                    break;
                } else {
                    members += 1;
                    let mut is_seed: ::core::ffi::c_int =
                        ((*r).store >= 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
                    if strcmp((*r).path, b"/\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                        fchmod(
                            x.base_root,
                            (*r).mode as mode_t & 0o777 as mode_t | 0o700 as mode_t,
                        );
                        if fset_meta(
                            x.base_root,
                            (*r).uid,
                            (*r).gid,
                            S_IFDIR as uint32_t | (*r).mode,
                            0 as uint32_t,
                            0 as uint32_t,
                            0 as uint32_t,
                        ) != 0
                        {
                            rc = install_fail!(
                                74 as ::core::ffi::c_int,
                                b"xattr on /: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                strerror(*__errno()),
                            );
                            break;
                        } else {
                            (*dts.offset(ndt as isize)).row =
                                r.offset_from((*a).rows) as ::core::ffi::c_int;
                            (*dts.offset(ndt as isize)).mtime = m.mtime;
                            ndt = ndt.wrapping_add(1);
                        }
                    } else {
                        let mut base: ::core::ffi::c_int = 0;
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
                            strrchr((*r).path, '/' as ::core::ffi::c_int)
                                .offset(1 as ::core::ffi::c_int as isize);
                        let mut pfd: ::core::ffi::c_int =
                            open_dir(&raw mut x, base, &raw mut dir as *mut ::core::ffi::c_char);
                        if pfd < 0 as ::core::ffi::c_int {
                            rc = install_fail!(
                                74 as ::core::ffi::c_int,
                                b"open parent of %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                (*r).path,
                                strerror(*__errno()),
                            );
                            break;
                        } else {
                            if (*r).r#type as ::core::ffi::c_int == 'd' as ::core::ffi::c_int {
                                if mkdirat(pfd, nm, 0o700 as mode_t) != 0 {
                                    rc = install_fail!(
                                        74 as ::core::ffi::c_int,
                                        b"mkdir %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                        (*r).path,
                                        strerror(*__errno()),
                                    );
                                    break;
                                } else {
                                    let mut dfd: ::core::ffi::c_int = openat(
                                        pfd,
                                        nm,
                                        O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
                                    );
                                    if dfd < 0 as ::core::ffi::c_int {
                                        rc = install_fail!(
                                            74 as ::core::ffi::c_int,
                                            b"open %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                            (*r).path,
                                            strerror(*__errno()),
                                        );
                                        break;
                                    } else {
                                        fchmod(
                                            dfd,
                                            (*r).mode as mode_t & 0o777 as mode_t | 0o700 as mode_t,
                                        );
                                        let mut e: ::core::ffi::c_int = if is_seed != 0 {
                                            0 as ::core::ffi::c_int
                                        } else {
                                            fset_meta(
                                                dfd,
                                                (*r).uid,
                                                (*r).gid,
                                                S_IFDIR as uint32_t | (*r).mode,
                                                0 as uint32_t,
                                                0 as uint32_t,
                                                0 as uint32_t,
                                            )
                                        };
                                        close(dfd);
                                        if e != 0 {
                                            rc = install_fail!(
                                                74 as ::core::ffi::c_int,
                                                b"xattr %s: %s\0".as_ptr()
                                                    as *const ::core::ffi::c_char,
                                                (*r).path,
                                                strerror(*__errno()),
                                            );
                                            break;
                                        } else {
                                            (*dts.offset(ndt as isize)).row =
                                                r.offset_from((*a).rows) as ::core::ffi::c_int;
                                            (*dts.offset(ndt as isize)).mtime = m.mtime;
                                            ndt = ndt.wrapping_add(1);
                                        }
                                    }
                                }
                            } else if (*r).r#type as ::core::ffi::c_int == 'l' as ::core::ffi::c_int
                            {
                                if symlinkat(&raw mut m.link as *mut ::core::ffi::c_char, pfd, nm)
                                    != 0
                                {
                                    rc = install_fail!(
                                        74 as ::core::ffi::c_int,
                                        b"symlink %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                        (*r).path,
                                        strerror(*__errno()),
                                    );
                                    break;
                                } else {
                                    let mut ts: [timespec; 2] = [
                                        timespec {
                                            tv_sec: m.mtime as time_t,
                                            tv_nsec: 0 as ::core::ffi::c_long,
                                        },
                                        timespec {
                                            tv_sec: m.mtime as time_t,
                                            tv_nsec: 0 as ::core::ffi::c_long,
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
                                let mut fd: ::core::ffi::c_int = openat(
                                    pfd,
                                    nm,
                                    O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                                    0o600 as ::core::ffi::c_int,
                                );
                                if fd < 0 as ::core::ffi::c_int {
                                    rc = install_fail!(
                                        74 as ::core::ffi::c_int,
                                        b"create %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                        (*r).path,
                                        strerror(*__errno()),
                                    );
                                    break;
                                } else {
                                    regular += m.size as ::core::ffi::c_longlong;
                                    let mut left: uint64_t = m.size;
                                    while left != 0 && rc == 0 {
                                        let mut chunk: size_t = if left
                                            < ((1 as ::core::ffi::c_int)
                                                << 20 as ::core::ffi::c_int)
                                                as uint64_t
                                        {
                                            left as size_t
                                        } else {
                                            ((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int)
                                                as size_t
                                        };
                                        if src_read(s, buf as *mut ::core::ffi::c_void, chunk) != 0
                                        {
                                            rc = install_fail!(
                                                65 as ::core::ffi::c_int,
                                                b"image: truncated data of %s\0".as_ptr()
                                                    as *const ::core::ffi::c_char,
                                                (*r).path,
                                            );
                                            break;
                                        } else {
                                            let mut off: size_t = 0 as size_t;
                                            while off < chunk {
                                                let mut w: ssize_t = write(
                                                    fd,
                                                    buf.offset(off as isize)
                                                        as *const ::core::ffi::c_void,
                                                    chunk.wrapping_sub(off),
                                                );
                                                if w < 0 as ssize_t {
                                                    if *__errno() == EINTR {
                                                        continue;
                                                    }
                                                    rc = install_fail!(
                                                        74 as ::core::ffi::c_int,
                                                        b"write %s: %s\0".as_ptr()
                                                            as *const ::core::ffi::c_char,
                                                        (*r).path,
                                                        strerror(*__errno()),
                                                    );
                                                    break;
                                                } else {
                                                    off = off.wrapping_add(w as size_t);
                                                }
                                            }
                                            left = (left as ::core::ffi::c_ulong)
                                                .wrapping_sub(chunk as ::core::ffi::c_ulong)
                                                as uint64_t;
                                        }
                                    }
                                    if rc == 0
                                        && src_read(
                                            s,
                                            NULL,
                                            (512 as size_t)
                                                .wrapping_sub(
                                                    (m.size as size_t).wrapping_rem(512 as size_t),
                                                )
                                                .wrapping_rem(512 as size_t),
                                        ) != 0
                                    {
                                        rc = install_fail!(
                                            65 as ::core::ffi::c_int,
                                            b"image: truncated\0".as_ptr()
                                                as *const ::core::ffi::c_char,
                                        );
                                    }
                                    if rc == 0 {
                                        fchmod(
                                            fd,
                                            if is_seed != 0 {
                                                (*r).mode as mode_t & 0o777 as mode_t
                                            } else {
                                                (*r).mode as mode_t & 0o777 as mode_t
                                                    | 0o600 as mode_t
                                            },
                                        );
                                        if is_seed == 0
                                            && fset_meta(
                                                fd,
                                                (*r).uid,
                                                (*r).gid,
                                                S_IFREG as uint32_t | (*r).mode,
                                                0 as uint32_t,
                                                0 as uint32_t,
                                                0 as uint32_t,
                                            ) != 0
                                        {
                                            rc = install_fail!(
                                                74 as ::core::ffi::c_int,
                                                b"xattr %s: %s\0".as_ptr()
                                                    as *const ::core::ffi::c_char,
                                                (*r).path,
                                                strerror(*__errno()),
                                            );
                                        }
                                        let mut ts_0: [timespec; 2] = [
                                            timespec {
                                                tv_sec: m.mtime as time_t,
                                                tv_nsec: 0 as ::core::ffi::c_long,
                                            },
                                            timespec {
                                                tv_sec: m.mtime as time_t,
                                                tv_nsec: 0 as ::core::ffi::c_long,
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
            65 as ::core::ffi::c_int,
            b"image: %lld members, metadata says %lld\0".as_ptr() as *const ::core::ffi::c_char,
            members,
            (*mt).members,
        );
    }
    if rc == 0 && regular != (*mt).regular_bytes {
        rc = install_fail!(
            65 as ::core::ffi::c_int,
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
                (*(*a).rows.offset(ri as isize)).r#type as ::core::ffi::c_int,
            )
            .is_null()
        {
            ri = ri.wrapping_add(1);
        }
        if ri < (*a).n {
            rc = install_fail!(
                65 as ::core::ffi::c_int,
                b"image: member for %s missing\0".as_ptr() as *const ::core::ffi::c_char,
                (*(*a).rows.offset(ri as isize)).path,
            );
        }
    }
    let mut hard_0: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut special_0: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut objects: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut obj_of: *mut ::core::ffi::c_int = (if rc != 0 {
        NULL
    } else {
        calloc((*a).n, ::core::mem::size_of::<::core::ffi::c_int>())
    }) as *mut ::core::ffi::c_int;
    if rc == 0 && obj_of.is_null() {
        rc = install_fail!(
            74 as ::core::ffi::c_int,
            b"out of memory\0".as_ptr() as *const ::core::ffi::c_char,
        );
    }
    if rc == 0 {
        let mut have_links: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        let mut i_1: size_t = 0 as size_t;
        while i_1 < (*a).n {
            if (*(*a).rows.offset(i_1 as isize)).r#type as ::core::ffi::c_int
                == 'h' as ::core::ffi::c_int
            {
                have_links = 1 as ::core::ffi::c_int;
            }
            i_1 = i_1.wrapping_add(1);
        }
        let mut lfd: ::core::ffi::c_int = -1 as ::core::ffi::c_int;
        if have_links != 0 {
            mkdirat(x.base_root, STORE_DIR.as_ptr(), 0o700 as mode_t);
            mkdirat(x.base_root, LINKS_DIR.as_ptr(), 0o700 as mode_t);
            lfd = openat(
                x.base_root,
                LINKS_DIR.as_ptr(),
                O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
            );
            if lfd < 0 as ::core::ffi::c_int {
                rc = install_fail!(
                    74 as ::core::ffi::c_int,
                    b"create hardlink store: %s\0".as_ptr() as *const ::core::ffi::c_char,
                    strerror(*__errno()),
                );
            }
        }
        let mut i_2: size_t = 0 as size_t;
        while i_2 < (*a).n && rc == 0 {
            let mut r_0: *mut row_t = (*a).rows.offset(i_2 as isize);
            if !strchr(
                b"hcbp\0".as_ptr() as *const ::core::ffi::c_char,
                (*r_0).r#type as ::core::ffi::c_int,
            )
            .is_null()
            {
                let mut dir_0: [::core::ffi::c_char; 4096] = [0; 4096];
                let mut base_0: ::core::ffi::c_int = 0;
                locate(
                    &raw mut x,
                    a,
                    r_0,
                    &raw mut base_0,
                    &raw mut dir_0 as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                );
                let mut nm_0: *const ::core::ffi::c_char =
                    strrchr((*r_0).path, '/' as ::core::ffi::c_int)
                        .offset(1 as ::core::ffi::c_int as isize);
                let mut pfd_0: ::core::ffi::c_int = open_dir(
                    &raw mut x,
                    base_0,
                    &raw mut dir_0 as *mut ::core::ffi::c_char,
                );
                if pfd_0 < 0 as ::core::ffi::c_int {
                    rc = install_fail!(
                        74 as ::core::ffi::c_int,
                        b"open parent of %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                        (*r_0).path,
                        strerror(*__errno()),
                    );
                    break;
                } else if (*r_0).r#type as ::core::ffi::c_int == 'h' as ::core::ffi::c_int {
                    let mut pr: *mut row_t = (*a).rows.offset((*r_0).primary as isize);
                    let mut id: [::core::ffi::c_char; 32] = [0; 32];
                    let mut stub: [::core::ffi::c_char; 64] = [0; 64];
                    if *obj_of.offset((*r_0).primary as isize) == 0 {
                        objects += 1;
                        *obj_of.offset((*r_0).primary as isize) = objects as ::core::ffi::c_int;
                        snprintf(
                            &raw mut id as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 32]>(),
                            b"%016llx\0".as_ptr() as *const ::core::ffi::c_char,
                            (0x1000000000000000 as ::core::ffi::c_ulonglong)
                                .wrapping_add(objects as ::core::ffi::c_ulonglong),
                        );
                        let mut pdir: [::core::ffi::c_char; 4096] = [0; 4096];
                        let mut pbase: ::core::ffi::c_int = 0;
                        locate(
                            &raw mut x,
                            a,
                            pr,
                            &raw mut pbase,
                            &raw mut pdir as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                        );
                        let mut pnm: *const ::core::ffi::c_char =
                            strrchr((*pr).path, '/' as ::core::ffi::c_int)
                                .offset(1 as ::core::ffi::c_int as isize);
                        let mut ppfd: ::core::ffi::c_int =
                            open_dir(&raw mut x, pbase, &raw mut pdir as *mut ::core::ffi::c_char);
                        if ppfd < 0 as ::core::ffi::c_int
                            || renameat(ppfd, pnm, lfd, &raw mut id as *mut ::core::ffi::c_char)
                                != 0
                        {
                            rc = install_fail!(
                                74 as ::core::ffi::c_int,
                                b"hardlink %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                (*pr).path,
                                strerror(*__errno()),
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
                                    74 as ::core::ffi::c_int,
                                    b"hardlink %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                    (*pr).path,
                                    strerror(*__errno()),
                                );
                                break;
                            } else {
                                pfd_0 = open_dir(
                                    &raw mut x,
                                    base_0,
                                    &raw mut dir_0 as *mut ::core::ffi::c_char,
                                );
                                if pfd_0 < 0 as ::core::ffi::c_int {
                                    rc = install_fail!(
                                        74 as ::core::ffi::c_int,
                                        b"open parent of %s: %s\0".as_ptr()
                                            as *const ::core::ffi::c_char,
                                        (*r_0).path,
                                        strerror(*__errno()),
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
                        (0x1000000000000000 as ::core::ffi::c_ulonglong).wrapping_add(
                            *obj_of.offset((*r_0).primary as isize) as ::core::ffi::c_ulonglong,
                        ),
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
                            74 as ::core::ffi::c_int,
                            b"hardlink %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                            (*r_0).path,
                            strerror(*__errno()),
                        );
                        break;
                    } else {
                        *obj_of.offset(i_2 as isize) = *obj_of.offset((*r_0).primary as isize);
                        hard_0 += 1;
                    }
                } else {
                    let mut fd_0: ::core::ffi::c_int = openat(
                        pfd_0,
                        nm_0,
                        O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                        0o600 as ::core::ffi::c_int,
                    );
                    if fd_0 < 0 as ::core::ffi::c_int {
                        rc = install_fail!(
                            74 as ::core::ffi::c_int,
                            b"create %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                            (*r_0).path,
                            strerror(*__errno()),
                        );
                        break;
                    } else {
                        let mut r#type: uint32_t = (if (*r_0).r#type as ::core::ffi::c_int
                            == 'c' as ::core::ffi::c_int
                        {
                            S_IFCHR
                        } else if (*r_0).r#type as ::core::ffi::c_int == 'b' as ::core::ffi::c_int {
                            S_IFBLK
                        } else {
                            S_IFIFO
                        }) as uint32_t;
                        if fset_meta(
                            fd_0,
                            (*r_0).uid,
                            (*r_0).gid,
                            r#type | (*r_0).mode,
                            0 as uint32_t,
                            (*r_0).major,
                            (*r_0).minor,
                        ) != 0
                        {
                            rc = install_fail!(
                                74 as ::core::ffi::c_int,
                                b"xattr %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                                (*r_0).path,
                                strerror(*__errno()),
                            );
                        }
                        close(fd_0);
                        special_0 += 1;
                    }
                }
            }
            i_2 = i_2.wrapping_add(1);
        }
        let mut ob: ::core::ffi::c_longlong = 1 as ::core::ffi::c_longlong;
        while ob <= objects && rc == 0 {
            let mut names: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
            let mut prim: size_t = 0 as size_t;
            let mut i_3: size_t = 0 as size_t;
            while i_3 < (*a).n {
                if *obj_of.offset(i_3 as isize) as ::core::ffi::c_longlong == ob {
                    names = names.wrapping_add(1);
                    if (*(*a).rows.offset(i_3 as isize)).r#type as ::core::ffi::c_int
                        == 'f' as ::core::ffi::c_int
                    {
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
                (0x1000000000000000 as ::core::ffi::c_ulonglong)
                    .wrapping_add(ob as ::core::ffi::c_ulonglong),
            );
            let mut fd_1: ::core::ffi::c_int = openat(
                lfd,
                &raw mut id_0 as *mut ::core::ffi::c_char,
                O_RDONLY | O_NOFOLLOW | O_CLOEXEC,
            );
            let mut pr_0: *const row_t = (*a).rows.offset(prim as isize);
            if fd_1 < 0 as ::core::ffi::c_int
                || fset_meta(
                    fd_1,
                    (*pr_0).uid,
                    (*pr_0).gid,
                    S_IFREG as uint32_t | (*pr_0).mode,
                    names as uint32_t,
                    0 as uint32_t,
                    0 as uint32_t,
                ) != 0
            {
                rc = install_fail!(
                    74 as ::core::ffi::c_int,
                    b"hardlink count: %s\0".as_ptr() as *const ::core::ffi::c_char,
                    strerror(*__errno()),
                );
            }
            if fd_1 >= 0 as ::core::ffi::c_int {
                close(fd_1);
            }
            ob += 1;
        }
        if lfd >= 0 as ::core::ffi::c_int {
            close(lfd);
        }
    }
    free(obj_of as *mut ::core::ffi::c_void);
    let mut i_4: size_t = ndt;
    loop {
        let c2rust_fresh1 = i_4;
        i_4 = i_4.wrapping_sub(1);
        if !(c2rust_fresh1 > 0 as size_t && rc == 0) {
            break;
        }
        let mut r_1: *const row_t = (*a).rows.offset((*dts.offset(i_4 as isize)).row as isize);
        let mut ts_1: [timespec; 2] = [
            timespec {
                tv_sec: (*dts.offset(i_4 as isize)).mtime as time_t,
                tv_nsec: 0 as ::core::ffi::c_long,
            },
            timespec {
                tv_sec: (*dts.offset(i_4 as isize)).mtime as time_t,
                tv_nsec: 0 as ::core::ffi::c_long,
            },
        ];
        if strcmp((*r_1).path, b"/\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            futimens(
                x.base_root,
                &raw mut ts_1 as *mut timespec as *const timespec,
            );
        } else {
            let mut base_1: ::core::ffi::c_int = 0;
            let mut dir_1: [::core::ffi::c_char; 4096] = [0; 4096];
            locate(
                &raw mut x,
                a,
                r_1,
                &raw mut base_1,
                &raw mut dir_1 as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            );
            let mut pfd_1: ::core::ffi::c_int = open_dir(
                &raw mut x,
                base_1,
                &raw mut dir_1 as *mut ::core::ffi::c_char,
            );
            if pfd_1 >= 0 as ::core::ffi::c_int {
                utimensat(
                    pfd_1,
                    strrchr((*r_1).path, '/' as ::core::ffi::c_int)
                        .offset(1 as ::core::ffi::c_int as isize),
                    &raw mut ts_1 as *mut timespec as *const timespec,
                    AT_SYMLINK_NOFOLLOW,
                );
            }
        }
    }
    free(dts as *mut ::core::ffi::c_void);
    if x.cache_fd >= 0 as ::core::ffi::c_int {
        close(x.cache_fd);
    }
    if rc == 0 {
        let mut tmp: [::core::ffi::c_uchar; 4096] = [0; 4096];
        while rc == 0 {
            let mut k_0: ssize_t = if (*s).out_pos < (*s).out_len {
                (*s).out_len.wrapping_sub((*s).out_pos) as ssize_t
            } else {
                src_fill(s)
            };
            if k_0 < 0 as ssize_t {
                rc = install_fail!(
                    65 as ::core::ffi::c_int,
                    b"image: zstd stream error\0".as_ptr() as *const ::core::ffi::c_char,
                );
                break;
            } else {
                if k_0 == 0 as ssize_t {
                    break;
                }
                (*s).out_pos = (*s).out_len;
            }
        }
        if rc == 0 && (*s).last_ret != 0 as size_t {
            rc = install_fail!(
                65 as ::core::ffi::c_int,
                b"image: zstd frame incomplete\0".as_ptr() as *const ::core::ffi::c_char,
            );
        }
    }
    if rc == 0 {
        syncfs(x.base_root);
    }
    if x.base_root >= 0 as ::core::ffi::c_int {
        close(x.base_root);
    }
    if x.base_seeds >= 0 as ::core::ffi::c_int {
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
pub unsafe extern "C" fn eng_install(mut o: *const eng_install_opts) -> ::core::ffi::c_int {
    g_quiet = (*o).quiet;
    g_err[0usize] = 0 as ::core::ffi::c_char;
    let mut want: [::core::ffi::c_char; 65] = ::core::mem::transmute::<
        [u8; 65],
        [::core::ffi::c_char; 65],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    );
    let mut want_size: ::core::ffi::c_longlong = -1 as ::core::ffi::c_longlong;
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
                strerror(*__errno()),
            );
            return 66 as ::core::ffi::c_int;
        }
        let mut buf: [::core::ffi::c_char; 65536] = [0; 65536];
        let mut n: size_t = fread(
            &raw mut buf as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            1 as size_t,
            ::core::mem::size_of::<[::core::ffi::c_char; 65536]>(),
            f,
        ) as size_t;
        fclose(f);
        let mut j: *mut eng_json = eng_json_parse(&raw mut buf as *mut ::core::ffi::c_char, n);
        let mut s: *const ::core::ffi::c_char = if !j.is_null() {
            eng_json_str(j, b"sha256\0".as_ptr() as *const ::core::ffi::c_char)
        } else {
            ::core::ptr::null::<::core::ffi::c_char>()
        };
        if s.is_null() || strlen(s) != 64 as size_t {
            eng_json_free(j);
            fprintf(
                stderr,
                b"install: bad index %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                (*o).index,
            );
            return 65 as ::core::ffi::c_int;
        }
        if want[0usize] as ::core::ffi::c_int != 0
            && strcmp(&raw mut want as *mut ::core::ffi::c_char, s) != 0
        {
            eng_json_free(j);
            fprintf(
                stderr,
                b"install: --sha256 differs from the index\n\0".as_ptr()
                    as *const ::core::ffi::c_char,
            );
            return 65 as ::core::ffi::c_int;
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
        return 2 as ::core::ffi::c_int;
    }
    let mut r#in: ::core::ffi::c_int =
        if strcmp((*o).image, b"-\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            dup(0 as ::core::ffi::c_int)
        } else {
            open((*o).image, O_RDONLY | O_CLOEXEC)
        };
    if r#in < 0 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"install: cannot open %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            (*o).image,
            strerror(*__errno()),
        );
        return 66 as ::core::ffi::c_int;
    }
    if mkdir((*o).target, 0o700 as mode_t) != 0 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"install: cannot create %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            (*o).target,
            strerror(*__errno()),
        );
        close(r#in);
        return 73 as ::core::ffi::c_int;
    }
    let mut tfd: ::core::ffi::c_int =
        open((*o).target, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    let mut s_0: *mut src_t = calloc(1 as size_t, ::core::mem::size_of::<src_t>()) as *mut src_t;
    let mut a: attrs_t = attrs_t {
        rows: ::core::ptr::null_mut::<row_t>(),
        n: 0,
        hash: ::core::ptr::null_mut::<::core::ffi::c_int>(),
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
        0 as ::core::ffi::c_int,
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
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<meta_t>(),
    );
    let mut stats: [::core::ffi::c_longlong; 5] = [0 as ::core::ffi::c_longlong, 0, 0, 0, 0];
    let mut rc: ::core::ffi::c_int = 0;
    let mut t0: ::core::ffi::c_double = now_s();
    if tfd < 0 as ::core::ffi::c_int || s_0.is_null() {
        rc = install_fail!(
            74 as ::core::ffi::c_int,
            b"open target: %s\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(*__errno()),
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
                &raw mut stats as *mut ::core::ffi::c_longlong,
            )
        } else {
            install_fail!(
                74 as ::core::ffi::c_int,
                b"zstd context\0".as_ptr() as *const ::core::ffi::c_char,
            )
        };
    }
    if rc == 0 {
        let mut tmp: [::core::ffi::c_uchar; 65536] = [0; 65536];
        let mut k: ssize_t = 0;
        loop {
            k = read(
                r#in,
                &raw mut tmp as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_uchar; 65536]>(),
            );
            if k <= 0 as ssize_t {
                break;
            }
            eng_sha256_update(
                &raw mut (*s_0).sha,
                &raw mut tmp as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
                k as size_t,
            );
            (*s_0).in_bytes = (*s_0).in_bytes.wrapping_add(k as uint64_t);
        }
        let mut d: [uint8_t; 32] = [0; 32];
        let mut hex: [::core::ffi::c_char; 65] = [0; 65];
        eng_sha256_final(&raw mut (*s_0).sha, &raw mut d as *mut uint8_t);
        eng_sha256_hex(
            &raw mut d as *mut uint8_t as *const uint8_t,
            &raw mut hex as *mut ::core::ffi::c_char,
        );
        if strcmp(
            &raw mut hex as *mut ::core::ffi::c_char,
            &raw mut want as *mut ::core::ffi::c_char,
        ) != 0
        {
            rc = install_fail!(
                65 as ::core::ffi::c_int,
                b"image sha256 %s does not match the expected %s\0".as_ptr()
                    as *const ::core::ffi::c_char,
                &raw mut hex as *mut ::core::ffi::c_char,
                &raw mut want as *mut ::core::ffi::c_char,
            );
        } else if want_size >= 0 as ::core::ffi::c_longlong
            && (*s_0).in_bytes as ::core::ffi::c_longlong != want_size
        {
            rc = install_fail!(
                65 as ::core::ffi::c_int,
                b"image size %llu, index says %lld\0".as_ptr() as *const ::core::ffi::c_char,
                (*s_0).in_bytes as ::core::ffi::c_ulonglong,
                want_size,
            );
        }
    }
    if !s_0.is_null() && !(*s_0).dctx.is_null() {
        ZSTD_freeDCtx((*s_0).dctx);
    }
    let mut i: size_t = 0 as size_t;
    while i < a.n {
        free((*a.rows.offset(i as isize)).path as *mut ::core::ffi::c_void);
        free((*a.rows.offset(i as isize)).extra as *mut ::core::ffi::c_void);
        i = i.wrapping_add(1);
    }
    free(a.rows as *mut ::core::ffi::c_void);
    free(a.hash as *mut ::core::ffi::c_void);
    close(r#in);
    if tfd >= 0 as ::core::ffi::c_int {
        if rc == 0 {
            fsync(tfd);
        }
        close(tfd);
    }
    if rc != 0 {
        fprintf(
            stderr,
            b"install: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            if g_err[0usize] as ::core::ffi::c_int != 0 {
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
        mt.rows as size_t,
        stats[0usize],
        stats[1usize],
        stats[4usize],
        stats[2usize],
        stats[3usize],
        (*s_0).in_bytes as ::core::ffi::c_ulonglong,
        &raw mut want as *mut ::core::ffi::c_char,
        now_s() - t0,
    );
    free(s_0 as *mut ::core::ffi::c_void);
    return 0 as ::core::ffi::c_int;
}
pub const FICLONE: usize = ((1 as ::core::ffi::c_uint) << _IOC_DIRSHIFT
    | ((0x94 as ::core::ffi::c_int) << _IOC_TYPESHIFT) as ::core::ffi::c_uint
    | ((9 as ::core::ffi::c_int) << _IOC_NRSHIFT) as ::core::ffi::c_uint)
    as usize
    | ::core::mem::size_of::<::core::ffi::c_int>() << _IOC_SIZESHIFT;
unsafe extern "C" fn copy_file(
    mut sfd: ::core::ffi::c_int,
    mut dfd: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if ioctl(dfd, FICLONE as ::core::ffi::c_uint, sfd) == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    let mut buf: [::core::ffi::c_char; 65536] = [0; 65536];
    loop {
        let mut n: ssize_t = read(
            sfd,
            &raw mut buf as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<[::core::ffi::c_char; 65536]>(),
        );
        if n < 0 as ssize_t {
            if *__errno() == EINTR {
                continue;
            }
            return -*__errno();
        } else {
            if n == 0 as ssize_t {
                return 0 as ::core::ffi::c_int;
            }
            let mut off: ssize_t = 0 as ssize_t;
            while off < n {
                let mut w: ssize_t = write(
                    dfd,
                    (&raw mut buf as *mut ::core::ffi::c_char).offset(off as isize)
                        as *const ::core::ffi::c_void,
                    (n - off) as size_t,
                );
                if w < 0 as ssize_t {
                    if *__errno() == EINTR {
                        continue;
                    }
                    return -*__errno();
                } else {
                    off += w;
                }
            }
        }
    }
}
unsafe extern "C" fn copy_xattrs(
    mut sfd: ::core::ffi::c_int,
    mut dfd: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut names: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut n: ssize_t = flistxattr(
        sfd,
        &raw mut names as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    let mut i: ssize_t = 0 as ssize_t;
    while n > 0 as ssize_t && i < n {
        if strncmp(
            (&raw mut names as *mut ::core::ffi::c_char).offset(i as isize),
            b"user.\0".as_ptr() as *const ::core::ffi::c_char,
            5 as size_t,
        ) == 0
        {
            let mut v: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut vl: ssize_t = fgetxattr(
                sfd,
                (&raw mut names as *mut ::core::ffi::c_char).offset(i as isize),
                &raw mut v as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            );
            if vl >= 0 as ssize_t
                && fsetxattr(
                    dfd,
                    (&raw mut names as *mut ::core::ffi::c_char).offset(i as isize),
                    &raw mut v as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                    vl as size_t,
                    0 as ::core::ffi::c_int,
                ) != 0 as ::core::ffi::c_int
            {
                return -*__errno();
            }
        }
        i += strlen((&raw mut names as *mut ::core::ffi::c_char).offset(i as isize)) as ssize_t
            + 1 as ssize_t;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn clone_dir(
    mut sdir: ::core::ffi::c_int,
    mut ddir: ::core::ffi::c_int,
    mut depth: ::core::ffi::c_int,
    mut count: *mut ::core::ffi::c_longlong,
) -> ::core::ffi::c_int {
    if depth > 512 as ::core::ffi::c_int {
        return -ELOOP;
    }
    let mut sfd2: ::core::ffi::c_int = dup(sdir);
    let mut d: *mut DIR = fdopendir(sfd2);
    if d.is_null() {
        close(sfd2);
        return -*__errno();
    }
    let mut e: *mut dirent = ::core::ptr::null_mut::<dirent>();
    let mut rc: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
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
        if depth == 0 as ::core::ffi::c_int && strcmp(n, STORE_DIR.as_ptr()) == 0 {
            let mut ss: ::core::ffi::c_int =
                openat(sdir, n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if ss < 0 as ::core::ffi::c_int {
                continue;
            }
            mkdirat(ddir, n, 0o700 as mode_t);
            let mut dd: ::core::ffi::c_int =
                openat(ddir, n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            let mut sl: ::core::ffi::c_int = openat(
                ss,
                b"links\0".as_ptr() as *const ::core::ffi::c_char,
                O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
            );
            if sl >= 0 as ::core::ffi::c_int
                && dd >= 0 as ::core::ffi::c_int
                && mkdirat(
                    dd,
                    b"links\0".as_ptr() as *const ::core::ffi::c_char,
                    0o700 as mode_t,
                ) == 0 as ::core::ffi::c_int
            {
                let mut dl: ::core::ffi::c_int = openat(
                    dd,
                    b"links\0".as_ptr() as *const ::core::ffi::c_char,
                    O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
                );
                if dl >= 0 as ::core::ffi::c_int {
                    let mut ld: *mut DIR = fdopendir(sl);
                    sl = -1 as ::core::ffi::c_int;
                    let mut le: *mut dirent = ::core::ptr::null_mut::<dirent>();
                    while rc == 0 && !ld.is_null() && {
                        le = readdir(ld);
                        !le.is_null()
                    } {
                        if (*le).d_name[0usize] as ::core::ffi::c_int == '.' as ::core::ffi::c_int
                            || strncmp(
                                &raw mut (*le).d_name as *mut ::core::ffi::c_char,
                                b"journal-\0".as_ptr() as *const ::core::ffi::c_char,
                                8 as size_t,
                            ) == 0
                        {
                            continue;
                        }
                        let mut a: ::core::ffi::c_int = openat(
                            dirfd(ld),
                            &raw mut (*le).d_name as *mut ::core::ffi::c_char,
                            O_RDONLY | O_NOFOLLOW | O_CLOEXEC,
                        );
                        let mut b: ::core::ffi::c_int = if a < 0 as ::core::ffi::c_int {
                            -1 as ::core::ffi::c_int
                        } else {
                            openat(
                                dl,
                                &raw mut (*le).d_name as *mut ::core::ffi::c_char,
                                O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                                0o600 as ::core::ffi::c_int,
                            )
                        };
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
                        if a < 0 as ::core::ffi::c_int
                            || b < 0 as ::core::ffi::c_int
                            || fstat(a, &raw mut st) != 0
                        {
                            rc = -*__errno();
                        } else {
                            rc = copy_file(a, b);
                            if rc == 0 && {
                                rc = copy_xattrs(a, b);
                                rc == 0
                            } {
                                fchmod(b, st.st_mode & 0o7777 as mode_t);
                                let mut ts: [timespec; 2] = [st.st_atim, st.st_mtim];
                                futimens(b, &raw mut ts as *mut timespec as *const timespec);
                                *count += 1;
                            }
                        }
                        if a >= 0 as ::core::ffi::c_int {
                            close(a);
                        }
                        if b >= 0 as ::core::ffi::c_int {
                            close(b);
                        }
                    }
                    if !ld.is_null() {
                        closedir(ld);
                    }
                    close(dl);
                }
            }
            if sl >= 0 as ::core::ffi::c_int {
                close(sl);
            }
            if dd >= 0 as ::core::ffi::c_int {
                close(dd);
            }
            close(ss);
        } else {
            let mut st_0: stat = stat {
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
            if fstatat(dirfd(d), n, &raw mut st_0, AT_SYMLINK_NOFOLLOW) != 0 {
                rc = -*__errno();
                break;
            } else {
                let mut ts_0: [timespec; 2] = [st_0.st_atim, st_0.st_mtim];
                if st_0.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t {
                    if mkdirat(ddir, n, 0o700 as mode_t) != 0 {
                        rc = -*__errno();
                        break;
                    } else {
                        let mut a_0: ::core::ffi::c_int =
                            openat(dirfd(d), n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
                        let mut b_0: ::core::ffi::c_int =
                            openat(ddir, n, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
                        if a_0 < 0 as ::core::ffi::c_int || b_0 < 0 as ::core::ffi::c_int {
                            rc = -*__errno();
                        } else {
                            rc = copy_xattrs(a_0, b_0);
                            if rc == 0 && {
                                rc = clone_dir(a_0, b_0, depth + 1 as ::core::ffi::c_int, count);
                                rc == 0
                            } {
                                fchmod(b_0, st_0.st_mode & 0o7777 as mode_t);
                                futimens(b_0, &raw mut ts_0 as *mut timespec as *const timespec);
                            }
                        }
                        if a_0 >= 0 as ::core::ffi::c_int {
                            close(a_0);
                        }
                        if b_0 >= 0 as ::core::ffi::c_int {
                            close(b_0);
                        }
                    }
                } else if st_0.st_mode & S_IFMT as mode_t == S_IFLNK as mode_t {
                    let mut t: [::core::ffi::c_char; 4096] = [0; 4096];
                    let mut k: ssize_t = readlinkat(
                        dirfd(d),
                        n,
                        &raw mut t as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
                            .wrapping_sub(1 as size_t),
                    );
                    if k < 0 as ssize_t {
                        rc = -*__errno();
                        break;
                    } else {
                        t[k as usize] = 0 as ::core::ffi::c_char;
                        if symlinkat(&raw mut t as *mut ::core::ffi::c_char, ddir, n) != 0 {
                            rc = -*__errno();
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
                    if st_0.st_mode & S_IFMT as mode_t != S_IFREG as mode_t {
                        continue;
                    }
                    let mut a_1: ::core::ffi::c_int =
                        openat(dirfd(d), n, O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
                    let mut b_1: ::core::ffi::c_int = if a_1 < 0 as ::core::ffi::c_int {
                        -1 as ::core::ffi::c_int
                    } else {
                        openat(
                            ddir,
                            n,
                            O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                            0o600 as ::core::ffi::c_int,
                        )
                    };
                    if a_1 < 0 as ::core::ffi::c_int || b_1 < 0 as ::core::ffi::c_int {
                        rc = -*__errno();
                    } else {
                        rc = copy_file(a_1, b_1);
                        if rc == 0 && {
                            rc = copy_xattrs(a_1, b_1);
                            rc == 0
                        } {
                            fchmod(b_1, st_0.st_mode & 0o7777 as mode_t);
                            futimens(b_1, &raw mut ts_0 as *mut timespec as *const timespec);
                        }
                    }
                    if a_1 >= 0 as ::core::ffi::c_int {
                        close(a_1);
                    }
                    if b_1 >= 0 as ::core::ffi::c_int {
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
    mut quiet: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut sr: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut sr as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/rootfs\0".as_ptr() as *const ::core::ffi::c_char,
        src,
    );
    let mut s: ::core::ffi::c_int = open(
        &raw mut sr as *mut ::core::ffi::c_char,
        O_RDONLY | O_DIRECTORY | O_CLOEXEC,
    );
    if s < 0 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"clone: %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut sr as *mut ::core::ffi::c_char,
            strerror(*__errno()),
        );
        return 66 as ::core::ffi::c_int;
    }
    if mkdir(dst, 0o700 as mode_t) != 0 {
        fprintf(
            stderr,
            b"clone: cannot create %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            dst,
            strerror(*__errno()),
        );
        close(s);
        return 73 as ::core::ffi::c_int;
    }
    let mut t: ::core::ffi::c_int = open(dst, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    let mut d: ::core::ffi::c_int = -1 as ::core::ffi::c_int;
    if t >= 0 as ::core::ffi::c_int
        && mkdirat(
            t,
            b"rootfs\0".as_ptr() as *const ::core::ffi::c_char,
            0o700 as mode_t,
        ) == 0 as ::core::ffi::c_int
    {
        d = openat(
            t,
            b"rootfs\0".as_ptr() as *const ::core::ffi::c_char,
            O_RDONLY | O_DIRECTORY | O_CLOEXEC,
        );
    }
    let mut t0: ::core::ffi::c_double = now_s();
    let mut count: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut rc: ::core::ffi::c_int = if d < 0 as ::core::ffi::c_int {
        -*__errno()
    } else {
        0 as ::core::ffi::c_int
    };
    if rc == 0 {
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
        fstat(s, &raw mut st);
        rc = copy_xattrs(s, d);
        if rc == 0 && {
            rc = clone_dir(s, d, 0 as ::core::ffi::c_int, &raw mut count);
            rc == 0
        } {
            fchmod(d, st.st_mode & 0o7777 as mode_t);
            let mut ts: [timespec; 2] = [st.st_atim, st.st_mtim];
            futimens(d, &raw mut ts as *mut timespec as *const timespec);
            syncfs(d);
        }
    }
    close(s);
    if d >= 0 as ::core::ffi::c_int {
        close(d);
    }
    if t >= 0 as ::core::ffi::c_int {
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
        return 74 as ::core::ffi::c_int;
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
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn count_meta(
    mut dfd: ::core::ffi::c_int,
    mut depth: ::core::ffi::c_int,
    mut entries: *mut ::core::ffi::c_longlong,
    mut missing: *mut ::core::ffi::c_longlong,
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
            || depth == 0 as ::core::ffi::c_int && strcmp(n, STORE_DIR.as_ptr()) == 0
        {
            continue;
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
        if fstatat(dirfd(d), n, &raw mut st, AT_SYMLINK_NOFOLLOW) != 0 {
            continue;
        }
        *entries += 1;
        if st.st_mode & S_IFMT as mode_t == S_IFLNK as mode_t {
            continue;
        }
        let mut fd: ::core::ffi::c_int = openat(
            dirfd(d),
            n,
            O_RDONLY
                | O_NOFOLLOW
                | O_CLOEXEC
                | O_NONBLOCK
                | if st.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t {
                    O_DIRECTORY
                } else {
                    0 as ::core::ffi::c_int
                },
        );
        if fd < 0 as ::core::ffi::c_int {
            continue;
        }
        let mut v: [::core::ffi::c_char; 128] = [0; 128];
        if fgetxattr(
            fd,
            ENG_META_XATTR.as_ptr(),
            &raw mut v as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<[::core::ffi::c_char; 128]>(),
        ) <= 0 as ssize_t
        {
            *missing += 1;
            if *missing <= 10 as ::core::ffi::c_longlong {
                fprintf(
                    out,
                    b"no metadata: %s (depth %d)\n\0".as_ptr() as *const ::core::ffi::c_char,
                    n,
                    depth,
                );
            }
        }
        if st.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t {
            count_meta(fd, depth + 1 as ::core::ffi::c_int, entries, missing, out);
        } else {
            close(fd);
        }
    }
    closedir(d);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_verify(
    mut generation: *const ::core::ffi::c_char,
    mut quiet: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
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
            strerror(*__errno()),
        );
        return 66 as ::core::ffi::c_int;
    }
    let mut fd: ::core::ffi::c_int = open(
        &raw mut r as *mut ::core::ffi::c_char,
        O_RDONLY | O_DIRECTORY | O_CLOEXEC,
    );
    let mut entries: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    let mut missing: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    if fd >= 0 as ::core::ffi::c_int {
        count_meta(
            fd,
            0 as ::core::ffi::c_int,
            &raw mut entries,
            &raw mut missing,
            stderr,
        );
    }
    let mut devnull: *mut FILE = if quiet != 0 {
        fopen(
            b"/dev/null\0".as_ptr() as *const ::core::ffi::c_char,
            b"we\0".as_ptr() as *const ::core::ffi::c_char,
        )
    } else {
        ::core::ptr::null_mut::<FILE>()
    };
    let mut problems: ::core::ffi::c_int = eng_link_fsck(
        g,
        0 as ::core::ffi::c_int,
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
    return if problems == 0 as ::core::ffi::c_int && missing == 0 as ::core::ffi::c_longlong {
        0 as ::core::ffi::c_int
    } else {
        1 as ::core::ffi::c_int
    };
}
