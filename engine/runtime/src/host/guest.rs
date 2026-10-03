//! Guest roots, binds, hidden paths, virtual identity and binfmt configuration.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
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
#[repr(C)]
pub struct __dirstream {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    unsafe fn closedir(__dirp: *mut DIR) -> ::core::ffi::c_int;
    unsafe fn opendir(__name: *const ::core::ffi::c_char) -> *mut DIR;
    unsafe fn readdir(__dirp: *mut DIR) -> *mut dirent;
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
    unsafe fn fclose(__stream: *mut FILE) -> ::core::ffi::c_int;
    unsafe fn fopen(
        __filename: *const ::core::ffi::c_char,
        __modes: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    unsafe fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn fgets(
        __s: *mut ::core::ffi::c_char,
        __n: ::core::ffi::c_int,
        __stream: *mut FILE,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strtoul(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulong;
    unsafe fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    unsafe fn free(__ptr: *mut ::core::ffi::c_void);
    unsafe fn realpath(
        __name: *const ::core::ffi::c_char,
        __resolved: *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
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
    unsafe fn strdup(__s: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn strchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strtok_r(
        __s: *mut ::core::ffi::c_char,
        __delim: *const ::core::ffi::c_char,
        __save_ptr: *mut *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    unsafe fn strerror(__errnum: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
    unsafe fn stat(__file: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn fstatat(
        __fd: ::core::ffi::c_int,
        __file: *const ::core::ffi::c_char,
        __buf: *mut stat,
        __flag: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn lstat(__file: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn mkdir(__path: *const ::core::ffi::c_char, __mode: __mode_t) -> ::core::ffi::c_int;
    unsafe fn mkdirat(
        __fd: ::core::ffi::c_int,
        __path: *const ::core::ffi::c_char,
        __mode: __mode_t,
    ) -> ::core::ffi::c_int;
    unsafe fn access(
        __name: *const ::core::ffi::c_char,
        __type: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn close(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn dup(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn symlink(
        __from: *const ::core::ffi::c_char,
        __to: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_meta_write(
        host: *const ::core::ffi::c_char,
        nofollow: ::core::ffi::c_int,
        m: *const eng_meta,
    ) -> ::core::ffi::c_int;
    unsafe fn gnu_dev_major(__dev: __dev_t) -> ::core::ffi::c_uint;
    unsafe fn gnu_dev_minor(__dev: __dev_t) -> ::core::ffi::c_uint;
    unsafe fn fsetxattr(
        __fd: ::core::ffi::c_int,
        __name: *const ::core::ffi::c_char,
        __value: *const ::core::ffi::c_void,
        __size: size_t,
        __flags: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
}
pub type size_t = usize;
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
pub struct eng_meta {
    pub uid: uint32_t,
    pub gid: uint32_t,
    pub mode: uint32_t,
    pub nlink: uint32_t,
    pub major: uint32_t,
    pub minor: uint32_t,
    pub present: ::core::ffi::c_int,
}
pub type uint32_t = u32;
pub type __dev_t = ::core::ffi::c_ulong;
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
pub type __syscall_slong_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __time_t,
    pub tv_nsec: __syscall_slong_t,
}
pub type __time_t = ::core::ffi::c_long;
pub type __blkcnt_t = ::core::ffi::c_long;
pub type __blksize_t = ::core::ffi::c_long;
pub type __off_t = ::core::ffi::c_long;
pub type __gid_t = ::core::ffi::c_uint;
pub type __uid_t = ::core::ffi::c_uint;
pub type __mode_t = ::core::ffi::c_uint;
pub type __nlink_t = ::core::ffi::c_ulong;
pub type __ino_t = ::core::ffi::c_ulong;
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
pub type __uint64_t = u64;
pub type __off64_t = ::core::ffi::c_long;
pub type _IO_lock_t = ();
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
pub const PATH_MAX: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
pub const ENG_MAX_BINDS: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const ENG_MAX_HIDES: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const ENG_MAX_BINFMT: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const ENG_LOC_ROOTFS: ::core::ffi::c_int = -1 as ::core::ffi::c_int;
pub const ENG_LOC_NONE: ::core::ffi::c_int = -2 as ::core::ffi::c_int;
pub const EINVAL: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
pub const ENOSPC: ::core::ffi::c_int = 28 as ::core::ffi::c_int;
pub const ENAMETOOLONG: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const O_RDONLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const O_WRONLY: ::core::ffi::c_int = 0o1 as ::core::ffi::c_int;
pub const O_CREAT: ::core::ffi::c_int = 0o100 as ::core::ffi::c_int;
pub const O_EXCL: ::core::ffi::c_int = 0o200 as ::core::ffi::c_int;
pub const __O_DIRECTORY: ::core::ffi::c_int = 0o200000 as ::core::ffi::c_int;
pub const __O_NOFOLLOW: ::core::ffi::c_int = 0o400000 as ::core::ffi::c_int;
pub const __O_CLOEXEC: ::core::ffi::c_int = 0o2000000 as ::core::ffi::c_int;
pub const __O_PATH: ::core::ffi::c_int = 0o10000000 as ::core::ffi::c_int;
pub const O_DIRECTORY: ::core::ffi::c_int = __O_DIRECTORY;
pub const O_NOFOLLOW: ::core::ffi::c_int = __O_NOFOLLOW;
pub const O_CLOEXEC: ::core::ffi::c_int = __O_CLOEXEC;
pub const O_PATH: ::core::ffi::c_int = __O_PATH;
pub const __S_IFMT: ::core::ffi::c_int = 0o170000 as ::core::ffi::c_int;
pub const __S_IFDIR: ::core::ffi::c_int = 0o40000 as ::core::ffi::c_int;
pub const __S_IFCHR: ::core::ffi::c_int = 0o20000 as ::core::ffi::c_int;
pub const __S_IFREG: ::core::ffi::c_int = 0o100000 as ::core::ffi::c_int;
pub const AT_SYMLINK_NOFOLLOW: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const S_IFDIR: ::core::ffi::c_int = __S_IFDIR;
pub const S_IFCHR: ::core::ffi::c_int = __S_IFCHR;
pub const S_IFREG: ::core::ffi::c_int = __S_IFREG;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const F_OK: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ENG_META_XATTR: [::core::ffi::c_char; 19] = unsafe {
    ::core::mem::transmute::<[u8; 19], [::core::ffi::c_char; 19]>(*b"user.workflow.meta\0")
};
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_open(mut root: *const ::core::ffi::c_char) -> *mut eng_guest {
    let mut g: *mut eng_guest =
        calloc(1 as size_t, ::core::mem::size_of::<eng_guest>()) as *mut eng_guest;
    if g.is_null() {
        return ::core::ptr::null_mut::<eng_guest>();
    }
    if realpath(root, &raw mut (*g).root as *mut ::core::ffi::c_char).is_null() {
        free(g as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<eng_guest>();
    }
    (*g).rootlen = strlen(&raw mut (*g).root as *mut ::core::ffi::c_char);
    if (*g).rootlen == 1 as size_t {
        (*g).rootlen = 0 as size_t;
    }
    (*g).rootfd = open(
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        O_PATH | O_DIRECTORY | O_CLOEXEC,
    );
    if (*g).rootfd < 0 as ::core::ffi::c_int {
        free(g as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<eng_guest>();
    }
    eng_guest_add_hide(
        g,
        b"/.workflow-engine\0".as_ptr() as *const ::core::ffi::c_char,
    );
    return g;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_close(mut g: *mut eng_guest) {
    if g.is_null() {
        return;
    }
    if (*g).rootfd >= 0 as ::core::ffi::c_int {
        close((*g).rootfd);
    }
    free(g as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn canon_guest(mut p: *mut ::core::ffi::c_char) {
    let mut w: *mut ::core::ffi::c_char = p;
    let mut r: *mut ::core::ffi::c_char = p;
    while *r != 0 {
        let c2rust_fresh0 = w;
        w = w.offset(1);
        *c2rust_fresh0 = *r;
        if *r as ::core::ffi::c_int == '/' as ::core::ffi::c_int {
            while *r.offset(1isize) as ::core::ffi::c_int == '/' as ::core::ffi::c_int {
                r = r.offset(1);
            }
        }
        r = r.offset(1);
    }
    *w = 0 as ::core::ffi::c_char;
    let mut n: size_t = strlen(p);
    while n > 1 as size_t
        && *p.offset(n.wrapping_sub(1 as size_t) as isize) as ::core::ffi::c_int
            == '/' as ::core::ffi::c_int
    {
        n = n.wrapping_sub(1);
        *p.offset(n as isize) = 0 as ::core::ffi::c_char;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_add_bind(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut guest: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    return eng_guest_add_bind_owned(
        g,
        host,
        guest,
        1000 as ::core::ffi::c_uint,
        1000 as ::core::ffi::c_uint,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_add_bind_owned(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut guest: *const ::core::ffi::c_char,
    mut uid: ::core::ffi::c_uint,
    mut gid: ::core::ffi::c_uint,
) -> ::core::ffi::c_int {
    if (*g).nbinds >= ENG_MAX_BINDS {
        return -ENOSPC;
    }
    if *guest.offset(0isize) as ::core::ffi::c_int != '/' as ::core::ffi::c_int {
        return -EINVAL;
    }
    let mut b: eng_bind = eng_bind {
        guest: [0; 4096],
        host: [0; 4096],
        glen: 0,
        hlen: 0,
        uid: 0,
        gid: 0,
    };
    memset(
        &raw mut b as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<eng_bind>(),
    );
    let mut real: [::core::ffi::c_char; 4096] = [0; 4096];
    if realpath(host, &raw mut real as *mut ::core::ffi::c_char).is_null() {
        return -*__errno_location();
    }
    snprintf(
        &raw mut b.host as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut real as *mut ::core::ffi::c_char,
    );
    snprintf(
        &raw mut b.guest as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        guest,
    );
    canon_guest(&raw mut b.guest as *mut ::core::ffi::c_char);
    if strcmp(
        &raw mut b.guest as *mut ::core::ffi::c_char,
        b"/\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        return -EINVAL;
    }
    b.glen = strlen(&raw mut b.guest as *mut ::core::ffi::c_char);
    b.hlen = strlen(&raw mut b.host as *mut ::core::ffi::c_char);
    b.uid = uid;
    b.gid = gid;
    if b.hlen == 1 as size_t {
        b.hlen = 0 as size_t;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*g).nbinds {
        if strcmp(
            &raw mut (*(&raw mut (*g).binds as *mut eng_bind).offset(i as isize)).guest
                as *mut ::core::ffi::c_char,
            &raw mut b.guest as *mut ::core::ffi::c_char,
        ) == 0
        {
            (*g).binds[i as usize] = b;
            return 0 as ::core::ffi::c_int;
        }
        i += 1;
    }
    let mut pos: ::core::ffi::c_int = (*g).nbinds;
    while pos > 0 as ::core::ffi::c_int
        && (*g).binds[(pos - 1 as ::core::ffi::c_int) as usize].glen < b.glen
    {
        (*g).binds[pos as usize] = (*g).binds[(pos - 1 as ::core::ffi::c_int) as usize];
        pos -= 1;
    }
    (*g).binds[pos as usize] = b;
    (*g).nbinds += 1;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_add_hide(
    mut g: *mut eng_guest,
    mut guest: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if (*g).nhides >= ENG_MAX_HIDES {
        return -ENOSPC;
    }
    snprintf(
        &raw mut *(&raw mut (*g).hides as *mut [::core::ffi::c_char; 4096])
            .offset((*g).nhides as isize) as *mut ::core::ffi::c_char,
        PATH_MAX as size_t,
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        guest,
    );
    canon_guest(
        &raw mut *(&raw mut (*g).hides as *mut [::core::ffi::c_char; 4096])
            .offset((*g).nhides as isize) as *mut ::core::ffi::c_char,
    );
    (*g).nhides += 1;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn prefix_match(
    mut path: *const ::core::ffi::c_char,
    mut pre: *const ::core::ffi::c_char,
    mut plen: size_t,
) -> ::core::ffi::c_int {
    return (strncmp(path, pre, plen) == 0 as ::core::ffi::c_int
        && (*path.offset(plen as isize) as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || *path.offset(plen as isize) as ::core::ffi::c_int == '/' as ::core::ffi::c_int))
        as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_locate(
    mut g: *const eng_guest,
    mut host: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut best: ::core::ffi::c_int = -1 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*g).nbinds {
        let mut b: *const eng_bind = (&raw const (*g).binds as *const eng_bind).offset(i as isize);
        if ((*b).hlen == 0 as size_t
            || prefix_match(
                host,
                &raw const (*b).host as *const ::core::ffi::c_char,
                (*b).hlen,
            ) != 0)
            && (best < 0 as ::core::ffi::c_int || (*b).hlen > (*g).binds[best as usize].hlen)
        {
            best = i;
        }
        i += 1;
    }
    let mut rl: size_t = (*g).rootlen;
    let mut in_root: ::core::ffi::c_int = (rl == 0 as size_t
        || prefix_match(host, &raw const (*g).root as *const ::core::ffi::c_char, rl) != 0)
        as ::core::ffi::c_int;
    if best >= 0 as ::core::ffi::c_int && (in_root == 0 || (*g).binds[best as usize].hlen >= rl) {
        return best;
    }
    return if in_root != 0 {
        ENG_LOC_ROOTFS
    } else {
        ENG_LOC_NONE
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_default_owner(
    mut g: *const eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut uid: *mut ::core::ffi::c_uint,
    mut gid: *mut ::core::ffi::c_uint,
) {
    let mut loc: ::core::ffi::c_int = eng_guest_locate(g, host);
    if loc >= 0 as ::core::ffi::c_int {
        *uid = (*g).binds[loc as usize].uid;
        *gid = (*g).binds[loc as usize].gid;
        return;
    }
    *uid = 0 as ::core::ffi::c_uint;
    *gid = 0 as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_is_hidden(
    mut g: *const eng_guest,
    mut guest: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*g).nhides {
        if prefix_match(
            guest,
            &raw const *(&raw const (*g).hides as *const [::core::ffi::c_char; 4096])
                .offset(i as isize) as *const ::core::ffi::c_char,
            strlen(
                &raw const *(&raw const (*g).hides as *const [::core::ffi::c_char; 4096])
                    .offset(i as isize) as *const ::core::ffi::c_char,
            ),
        ) != 0
        {
            return 1 as ::core::ffi::c_int;
        }
        i += 1;
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_to_host(
    mut g: *const eng_guest,
    mut guest: *const ::core::ffi::c_char,
    mut host: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*g).nbinds {
        let mut b: *const eng_bind = (&raw const (*g).binds as *const eng_bind).offset(i as isize);
        if prefix_match(
            guest,
            &raw const (*b).guest as *const ::core::ffi::c_char,
            (*b).glen,
        ) != 0
        {
            let mut n: ::core::ffi::c_int = snprintf(
                host,
                cap,
                b"%.*s%s\0".as_ptr() as *const ::core::ffi::c_char,
                (*b).hlen as ::core::ffi::c_int,
                &raw const (*b).host as *const ::core::ffi::c_char,
                guest.offset((*b).glen as isize),
            );
            if n <= 0 as ::core::ffi::c_int || n as size_t >= cap {
                return -ENAMETOOLONG;
            }
            if *host.offset(0isize) as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                *host.offset(0isize) = '/' as ::core::ffi::c_char;
                *host.offset(1isize) = 0 as ::core::ffi::c_char;
            }
            return 0 as ::core::ffi::c_int;
        }
        i += 1;
    }
    let mut n_0: ::core::ffi::c_int = snprintf(
        host,
        cap,
        b"%.*s%s\0".as_ptr() as *const ::core::ffi::c_char,
        (*g).rootlen as ::core::ffi::c_int,
        &raw const (*g).root as *const ::core::ffi::c_char,
        guest,
    );
    if n_0 <= 0 as ::core::ffi::c_int || n_0 as size_t >= cap {
        return -ENAMETOOLONG;
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_host_to_guest(
    mut g: *const eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut guest: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut best: *const eng_bind = ::core::ptr::null::<eng_bind>();
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*g).nbinds {
        let mut b: *const eng_bind = (&raw const (*g).binds as *const eng_bind).offset(i as isize);
        if (*b).hlen == 0 as size_t
            || prefix_match(
                host,
                &raw const (*b).host as *const ::core::ffi::c_char,
                (*b).hlen,
            ) != 0
        {
            if best.is_null() || (*b).hlen > (*best).hlen {
                best = b;
            }
        }
        i += 1;
    }
    let mut rl: size_t = (*g).rootlen;
    let mut in_root: ::core::ffi::c_int = (rl == 0 as size_t
        || prefix_match(host, &raw const (*g).root as *const ::core::ffi::c_char, rl) != 0)
        as ::core::ffi::c_int;
    if !best.is_null() && (in_root == 0 || (*best).hlen >= rl) {
        let mut n: ::core::ffi::c_int = snprintf(
            guest,
            cap,
            b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw const (*best).guest as *const ::core::ffi::c_char,
            host.offset((*best).hlen as isize),
        );
        return if n > 0 as ::core::ffi::c_int && (n as size_t) < cap {
            0 as ::core::ffi::c_int
        } else {
            -1 as ::core::ffi::c_int
        };
    }
    if in_root != 0 {
        let mut rest: *const ::core::ffi::c_char = host.offset(rl as isize);
        let mut n_0: ::core::ffi::c_int = snprintf(
            guest,
            cap,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            if *rest as ::core::ffi::c_int != 0 {
                rest
            } else {
                b"/\0".as_ptr() as *const ::core::ffi::c_char
            },
        );
        return if n_0 > 0 as ::core::ffi::c_int && (n_0 as size_t) < cap {
            0 as ::core::ffi::c_int
        } else {
            -1 as ::core::ffi::c_int
        };
    }
    return -1 as ::core::ffi::c_int;
}
unsafe extern "C" fn ensure_file(
    mut g: *const eng_guest,
    mut rel: *const ::core::ffi::c_char,
    mut hostdev: *const ::core::ffi::c_char,
) {
    let mut p: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw const (*g).root as *const ::core::ffi::c_char,
        rel,
    );
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
    let mut hs: stat = stat {
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
    if lstat(&raw mut p as *mut ::core::ffi::c_char, &raw mut st) == 0 as ::core::ffi::c_int {
        return;
    }
    let mut f: ::core::ffi::c_int = open(
        &raw mut p as *mut ::core::ffi::c_char,
        O_CREAT | O_EXCL | O_WRONLY | O_CLOEXEC,
        0o600 as ::core::ffi::c_int,
    );
    if f < 0 as ::core::ffi::c_int {
        return;
    }
    close(f);
    let mut m: eng_meta = eng_meta {
        uid: 0 as uint32_t,
        gid: 0 as uint32_t,
        mode: (S_IFREG | 0o666 as ::core::ffi::c_int) as uint32_t,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 1 as ::core::ffi::c_int,
    };
    if stat(hostdev, &raw mut hs) == 0 as ::core::ffi::c_int
        && hs.st_mode & __S_IFMT as __mode_t == 0o20000 as __mode_t
    {
        m.mode = (S_IFCHR | 0o666 as ::core::ffi::c_int) as uint32_t;
        m.major = gnu_dev_major(hs.st_rdev) as uint32_t;
        m.minor = gnu_dev_minor(hs.st_rdev) as uint32_t;
    }
    eng_meta_write(
        &raw mut p as *mut ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        &raw mut m,
    );
}
unsafe extern "C" fn ensure_dir(
    mut g: *const eng_guest,
    mut rel: *const ::core::ffi::c_char,
    mut mode: uint32_t,
) {
    let mut p: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw const (*g).root as *const ::core::ffi::c_char,
        rel,
    );
    if mkdir(&raw mut p as *mut ::core::ffi::c_char, 0o700 as __mode_t) != 0 as ::core::ffi::c_int {
        return;
    }
    let mut m: eng_meta = eng_meta {
        uid: 0 as uint32_t,
        gid: 0 as uint32_t,
        mode: S_IFDIR as uint32_t | mode,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 1 as ::core::ffi::c_int,
    };
    eng_meta_write(
        &raw mut p as *mut ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        &raw mut m,
    );
}
unsafe extern "C" fn ensure_link(
    mut g: *const eng_guest,
    mut rel: *const ::core::ffi::c_char,
    mut target: *const ::core::ffi::c_char,
) {
    let mut p: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw const (*g).root as *const ::core::ffi::c_char,
        rel,
    );
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
    if lstat(&raw mut p as *mut ::core::ffi::c_char, &raw mut st) == 0 as ::core::ffi::c_int {
        return;
    }
    if symlink(target, &raw mut p as *mut ::core::ffi::c_char) != 0 as ::core::ffi::c_int {
        eng_logf!(
            eng_log_level::ENG_LOG_DEBUG,
            b"placeholder symlink %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut p as *mut ::core::ffi::c_char,
            strerror(*__errno_location()),
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_default_binds(mut g: *mut eng_guest) -> ::core::ffi::c_int {
    static mut devs: [*const ::core::ffi::c_char; 7] = [
        b"null\0".as_ptr() as *const ::core::ffi::c_char,
        b"zero\0".as_ptr() as *const ::core::ffi::c_char,
        b"full\0".as_ptr() as *const ::core::ffi::c_char,
        b"random\0".as_ptr() as *const ::core::ffi::c_char,
        b"urandom\0".as_ptr() as *const ::core::ffi::c_char,
        b"tty\0".as_ptr() as *const ::core::ffi::c_char,
        b"ptmx\0".as_ptr() as *const ::core::ffi::c_char,
    ];
    ensure_dir(
        g,
        b"proc\0".as_ptr() as *const ::core::ffi::c_char,
        0o555 as uint32_t,
    );
    ensure_dir(
        g,
        b"sys\0".as_ptr() as *const ::core::ffi::c_char,
        0o555 as uint32_t,
    );
    ensure_dir(
        g,
        b"dev\0".as_ptr() as *const ::core::ffi::c_char,
        0o755 as uint32_t,
    );
    ensure_dir(
        g,
        b"dev/pts\0".as_ptr() as *const ::core::ffi::c_char,
        0o755 as uint32_t,
    );
    ensure_dir(
        g,
        b"dev/shm\0".as_ptr() as *const ::core::ffi::c_char,
        0o1777 as uint32_t,
    );
    ensure_link(
        g,
        b"dev/fd\0".as_ptr() as *const ::core::ffi::c_char,
        b"/proc/self/fd\0".as_ptr() as *const ::core::ffi::c_char,
    );
    ensure_link(
        g,
        b"dev/stdin\0".as_ptr() as *const ::core::ffi::c_char,
        b"/proc/self/fd/0\0".as_ptr() as *const ::core::ffi::c_char,
    );
    ensure_link(
        g,
        b"dev/stdout\0".as_ptr() as *const ::core::ffi::c_char,
        b"/proc/self/fd/1\0".as_ptr() as *const ::core::ffi::c_char,
    );
    ensure_link(
        g,
        b"dev/stderr\0".as_ptr() as *const ::core::ffi::c_char,
        b"/proc/self/fd/2\0".as_ptr() as *const ::core::ffi::c_char,
    );
    let mut rc: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if access(b"/proc\0".as_ptr() as *const ::core::ffi::c_char, F_OK) == 0 as ::core::ffi::c_int {
        rc |= eng_guest_add_bind_owned(
            g,
            b"/proc\0".as_ptr() as *const ::core::ffi::c_char,
            b"/proc\0".as_ptr() as *const ::core::ffi::c_char,
            0 as ::core::ffi::c_uint,
            0 as ::core::ffi::c_uint,
        );
    }
    if access(b"/sys\0".as_ptr() as *const ::core::ffi::c_char, F_OK) == 0 as ::core::ffi::c_int {
        eng_guest_add_bind_owned(
            g,
            b"/sys\0".as_ptr() as *const ::core::ffi::c_char,
            b"/sys\0".as_ptr() as *const ::core::ffi::c_char,
            0 as ::core::ffi::c_uint,
            0 as ::core::ffi::c_uint,
        );
    }
    eng_guest_add_hide(
        g,
        b"/sys/fs/selinux\0".as_ptr() as *const ::core::ffi::c_char,
    );
    let mut i: size_t = 0 as size_t;
    while i < ::core::mem::size_of::<[*const ::core::ffi::c_char; 7]>()
        .wrapping_div(::core::mem::size_of::<*const ::core::ffi::c_char>())
    {
        let mut host: [::core::ffi::c_char; 64] = [0; 64];
        let mut guest: [::core::ffi::c_char; 64] = [0; 64];
        snprintf(
            &raw mut host as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            b"/dev/%s\0".as_ptr() as *const ::core::ffi::c_char,
            devs[i],
        );
        snprintf(
            &raw mut guest as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            b"/dev/%s\0".as_ptr() as *const ::core::ffi::c_char,
            devs[i],
        );
        if access(&raw mut host as *mut ::core::ffi::c_char, F_OK) == 0 as ::core::ffi::c_int {
            let mut rel: [::core::ffi::c_char; 64] = [0; 64];
            snprintf(
                &raw mut rel as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                b"dev/%s\0".as_ptr() as *const ::core::ffi::c_char,
                devs[i],
            );
            ensure_file(
                g,
                &raw mut rel as *mut ::core::ffi::c_char,
                &raw mut host as *mut ::core::ffi::c_char,
            );
            eng_guest_add_bind_owned(
                g,
                &raw mut host as *mut ::core::ffi::c_char,
                &raw mut guest as *mut ::core::ffi::c_char,
                0 as ::core::ffi::c_uint,
                0 as ::core::ffi::c_uint,
            );
        }
        i = i.wrapping_add(1);
    }
    if access(b"/dev/pts\0".as_ptr() as *const ::core::ffi::c_char, F_OK) == 0 as ::core::ffi::c_int
    {
        eng_guest_add_bind_owned(
            g,
            b"/dev/pts\0".as_ptr() as *const ::core::ffi::c_char,
            b"/dev/pts\0".as_ptr() as *const ::core::ffi::c_char,
            0 as ::core::ffi::c_uint,
            0 as ::core::ffi::c_uint,
        );
    }
    return rc;
}
unsafe extern "C" fn unescape(
    mut r#in: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_uchar,
    mut cap: ::core::ffi::c_uint,
    mut len: *mut ::core::ffi::c_uint,
) -> ::core::ffi::c_int {
    let mut n: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut p: *const ::core::ffi::c_char = r#in;
    while *p != 0 {
        let mut c: ::core::ffi::c_uint = *p as ::core::ffi::c_uchar as ::core::ffi::c_uint;
        if c == '\\' as ::core::ffi::c_uint
            && *p.offset(1isize) as ::core::ffi::c_int == 'x' as ::core::ffi::c_int
            && *p.offset(2isize) as ::core::ffi::c_int != 0
            && *p.offset(3isize) as ::core::ffi::c_int != 0
        {
            let mut hx: [::core::ffi::c_char; 3] = [
                *p.offset(2isize),
                *p.offset(3isize),
                0 as ::core::ffi::c_char,
            ];
            let mut end: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
            c = strtoul(
                &raw mut hx as *mut ::core::ffi::c_char,
                &raw mut end,
                16 as ::core::ffi::c_int,
            ) as ::core::ffi::c_uint;
            if *end != 0 {
                return -EINVAL;
            }
            p = p.offset(3 as ::core::ffi::c_int as isize);
        } else if c == '\\' as ::core::ffi::c_uint
            && *p.offset(1isize) as ::core::ffi::c_int == '\\' as ::core::ffi::c_int
        {
            p = p.offset(1);
        }
        if n >= cap {
            return -EINVAL;
        }
        let c2rust_fresh2 = n;
        n = n.wrapping_add(1);
        *out.offset(c2rust_fresh2 as isize) = c as ::core::ffi::c_uchar;
        p = p.offset(1);
    }
    *len = n;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_add_binfmt(
    mut g: *mut eng_guest,
    mut rule: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if (*g).nbinfmt >= ENG_MAX_BINFMT || *rule.offset(0isize) == 0 {
        return -EINVAL;
    }
    let mut buf: [::core::ffi::c_char; 8192] = [0; 8192];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 8192]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        rule,
    );
    let mut bl: size_t = strlen(&raw mut buf as *mut ::core::ffi::c_char);
    while bl != 0
        && (buf[bl.wrapping_sub(1 as size_t)] as ::core::ffi::c_int == '\n' as ::core::ffi::c_int
            || buf[bl.wrapping_sub(1 as size_t)] as ::core::ffi::c_int
                == '\r' as ::core::ffi::c_int
            || buf[bl.wrapping_sub(1 as size_t)] as ::core::ffi::c_int == ' ' as ::core::ffi::c_int)
    {
        bl = bl.wrapping_sub(1);
        buf[bl] = 0 as ::core::ffi::c_char;
    }
    let mut del: ::core::ffi::c_char = buf[0usize];
    let mut f: [*mut ::core::ffi::c_char; 7] = [
        ::core::ptr::null_mut::<::core::ffi::c_char>(),
        ::core::ptr::null_mut::<::core::ffi::c_char>(),
        ::core::ptr::null_mut::<::core::ffi::c_char>(),
        ::core::ptr::null_mut::<::core::ffi::c_char>(),
        ::core::ptr::null_mut::<::core::ffi::c_char>(),
        ::core::ptr::null_mut::<::core::ffi::c_char>(),
        ::core::ptr::null_mut::<::core::ffi::c_char>(),
    ];
    let mut p: *mut ::core::ffi::c_char =
        (&raw mut buf as *mut ::core::ffi::c_char).offset(1 as ::core::ffi::c_int as isize);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 7 as ::core::ffi::c_int {
        f[i as usize] = p;
        let mut q: *mut ::core::ffi::c_char = strchr(p, del as ::core::ffi::c_int);
        if q.is_null() {
            if i < 5 as ::core::ffi::c_int {
                return -EINVAL;
            }
            let mut k: ::core::ffi::c_int = i + 1 as ::core::ffi::c_int;
            while k < 7 as ::core::ffi::c_int {
                f[k as usize] = p.offset(strlen(p) as isize);
                k += 1;
            }
            break;
        } else {
            *q = 0 as ::core::ffi::c_char;
            p = q.offset(1 as ::core::ffi::c_int as isize);
            i += 1;
        }
    }
    let mut b: eng_binfmt = eng_binfmt {
        name: [0; 64],
        r#type: 0,
        offset: 0,
        magic: [0; 128],
        mask: [0; 128],
        len: 0,
        ext: [0; 64],
        interp: [0; 4096],
        preserve_argv0: 0,
    };
    memset(
        &raw mut b as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<eng_binfmt>(),
    );
    snprintf(
        &raw mut b.name as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        f[0usize],
    );
    if b.name[0usize] == 0
        || strlen(f[1usize]) != 1 as size_t
        || *f[1usize].offset(0isize) as ::core::ffi::c_int != 'M' as ::core::ffi::c_int
            && *f[1usize].offset(0isize) as ::core::ffi::c_int != 'E' as ::core::ffi::c_int
    {
        return -EINVAL;
    }
    b.r#type = *f[1usize].offset(0isize);
    if *f[5usize].offset(0isize) as ::core::ffi::c_int != '/' as ::core::ffi::c_int {
        return -EINVAL;
    }
    snprintf(
        &raw mut b.interp as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        f[5usize],
    );
    let mut fl: *const ::core::ffi::c_char = f[6usize];
    while *fl != 0 {
        if *fl as ::core::ffi::c_int == 'P' as ::core::ffi::c_int {
            b.preserve_argv0 = 1 as ::core::ffi::c_int;
        } else if *fl as ::core::ffi::c_int != 'O' as ::core::ffi::c_int
            && *fl as ::core::ffi::c_int != 'C' as ::core::ffi::c_int
            && *fl as ::core::ffi::c_int != 'F' as ::core::ffi::c_int
        {
            return -EINVAL;
        }
        fl = fl.offset(1);
    }
    if b.r#type as ::core::ffi::c_int == 'E' as ::core::ffi::c_int {
        if *f[3usize].offset(0isize) == 0 || !strchr(f[3usize], '/' as ::core::ffi::c_int).is_null()
        {
            return -EINVAL;
        }
        snprintf(
            &raw mut b.ext as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            f[3usize],
        );
    } else {
        b.offset = if *f[2usize].offset(0isize) as ::core::ffi::c_int != 0 {
            strtoul(
                f[2usize],
                ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                10 as ::core::ffi::c_int,
            ) as ::core::ffi::c_uint
        } else {
            0 as ::core::ffi::c_uint
        };
        if unescape(
            f[3usize],
            &raw mut b.magic as *mut ::core::ffi::c_uchar,
            ::core::mem::size_of::<[::core::ffi::c_uchar; 128]>() as ::core::ffi::c_uint,
            &raw mut b.len,
        ) != 0
            || b.len == 0
        {
            return -EINVAL;
        }
        let mut ml: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
        if *f[4usize].offset(0isize) != 0 {
            if unescape(
                f[4usize],
                &raw mut b.mask as *mut ::core::ffi::c_uchar,
                ::core::mem::size_of::<[::core::ffi::c_uchar; 128]>() as ::core::ffi::c_uint,
                &raw mut ml,
            ) != 0
                || ml != b.len
            {
                return -EINVAL;
            }
        } else {
            memset(
                &raw mut b.mask as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
                0xff as ::core::ffi::c_int,
                b.len as size_t,
            );
        }
        if b.offset.wrapping_add(b.len) > 256 as ::core::ffi::c_uint {
            return -EINVAL;
        }
        let mut i_0: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
        while i_0 < b.len {
            b.magic[i_0 as usize] = (b.magic[i_0 as usize] as ::core::ffi::c_int
                & b.mask[i_0 as usize] as ::core::ffi::c_int)
                as ::core::ffi::c_uchar;
            i_0 = i_0.wrapping_add(1);
        }
    }
    let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_1 < (*g).nbinfmt {
        if strcmp(
            &raw mut (*(&raw mut (*g).binfmt as *mut eng_binfmt).offset(i_1 as isize)).name
                as *mut ::core::ffi::c_char,
            &raw mut b.name as *mut ::core::ffi::c_char,
        ) == 0
        {
            (*g).binfmt[i_1 as usize] = b;
            return 0 as ::core::ffi::c_int;
        }
        i_1 += 1;
    }
    let c2rust_fresh1 = (*g).nbinfmt;
    (*g).nbinfmt += 1;
    (*g).binfmt[c2rust_fresh1 as usize] = b;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_load_binfmt_file(
    mut g: *mut eng_guest,
    mut host_path: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut f: *mut FILE = fopen(host_path, b"re\0".as_ptr() as *const ::core::ffi::c_char);
    if f.is_null() {
        return -*__errno_location();
    }
    let mut line: [::core::ffi::c_char; 8192] = [0; 8192];
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while !fgets(
        &raw mut line as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 8192]>() as ::core::ffi::c_int,
        f,
    )
    .is_null()
    {
        if line[0usize] as ::core::ffi::c_int == '#' as ::core::ffi::c_int
            || line[0usize] as ::core::ffi::c_int == ';' as ::core::ffi::c_int
            || line[0usize] as ::core::ffi::c_int == '\n' as ::core::ffi::c_int
            || line[0usize] == 0
        {
            continue;
        }
        if eng_guest_add_binfmt(g, &raw mut line as *mut ::core::ffi::c_char)
            == 0 as ::core::ffi::c_int
        {
            n += 1;
        } else {
            eng_logf!(
                eng_log_level::ENG_LOG_WARN,
                b"binfmt: ignoring invalid rule in %s: %.80s\0".as_ptr()
                    as *const ::core::ffi::c_char,
                host_path,
                &raw mut line as *mut ::core::ffi::c_char,
            );
        }
    }
    fclose(f);
    return n;
}
unsafe extern "C" fn cmp_str(
    mut a: *const ::core::ffi::c_void,
    mut b: *const ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    return strcmp(
        *(a as *const *mut ::core::ffi::c_char),
        *(b as *const *mut ::core::ffi::c_char),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_load_binfmt_dirs(mut g: *mut eng_guest) -> ::core::ffi::c_int {
    static mut dirs: [*const ::core::ffi::c_char; 2] = [
        b"/usr/lib/binfmt.d\0".as_ptr() as *const ::core::ffi::c_char,
        b"/etc/binfmt.d\0".as_ptr() as *const ::core::ffi::c_char,
    ];
    let mut names: [*mut ::core::ffi::c_char; 128] =
        [::core::ptr::null_mut::<::core::ffi::c_char>(); 128];
    let mut paths: [*mut ::core::ffi::c_char; 128] =
        [::core::ptr::null_mut::<::core::ffi::c_char>(); 128];
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut d: size_t = 0 as size_t;
    while d < ::core::mem::size_of::<[*const ::core::ffi::c_char; 2]>()
        .wrapping_div(::core::mem::size_of::<*const ::core::ffi::c_char>())
    {
        let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
        if eng_guest_to_host(
            g,
            dirs[d],
            &raw mut host as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        ) == 0
        {
            let mut dp: *mut DIR = opendir(&raw mut host as *mut ::core::ffi::c_char);
            if !dp.is_null() {
                let mut e: *mut dirent = ::core::ptr::null_mut::<dirent>();
                loop {
                    e = readdir(dp);
                    if !(!e.is_null() && n < 128 as ::core::ffi::c_int) {
                        break;
                    }
                    let mut l: size_t = strlen(&raw mut (*e).d_name as *mut ::core::ffi::c_char);
                    if l < 6 as size_t
                        || strcmp(
                            (&raw mut (*e).d_name as *mut ::core::ffi::c_char)
                                .offset(l as isize)
                                .offset(-(5 as ::core::ffi::c_int as isize)),
                            b".conf\0".as_ptr() as *const ::core::ffi::c_char,
                        ) != 0
                    {
                        continue;
                    }
                    let mut dup_0: ::core::ffi::c_int = -1 as ::core::ffi::c_int;
                    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    while i < n {
                        if strcmp(
                            names[i as usize],
                            &raw mut (*e).d_name as *mut ::core::ffi::c_char,
                        ) == 0
                        {
                            dup_0 = i;
                        }
                        i += 1;
                    }
                    let mut p: [::core::ffi::c_char; 4352] = [0; 4352];
                    snprintf(
                        &raw mut p as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4352]>(),
                        b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut host as *mut ::core::ffi::c_char,
                        &raw mut (*e).d_name as *mut ::core::ffi::c_char,
                    );
                    if dup_0 >= 0 as ::core::ffi::c_int {
                        free(paths[dup_0 as usize] as *mut ::core::ffi::c_void);
                        paths[dup_0 as usize] = strdup(&raw mut p as *mut ::core::ffi::c_char);
                    } else {
                        names[n as usize] =
                            strdup(&raw mut (*e).d_name as *mut ::core::ffi::c_char);
                        paths[n as usize] = strdup(&raw mut p as *mut ::core::ffi::c_char);
                        n += 1;
                    }
                }
                closedir(dp);
            }
        }
        d = d.wrapping_add(1);
    }
    let mut i_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while i_0 < n {
        let mut j: ::core::ffi::c_int = i_0;
        while j > 0 as ::core::ffi::c_int
            && strcmp(
                names[(j - 1 as ::core::ffi::c_int) as usize],
                names[j as usize],
            ) > 0 as ::core::ffi::c_int
        {
            let mut t: *mut ::core::ffi::c_char = names[j as usize];
            names[j as usize] = names[(j - 1 as ::core::ffi::c_int) as usize];
            names[(j - 1 as ::core::ffi::c_int) as usize] = t;
            t = paths[j as usize];
            paths[j as usize] = paths[(j - 1 as ::core::ffi::c_int) as usize];
            paths[(j - 1 as ::core::ffi::c_int) as usize] = t;
            j -= 1;
        }
        i_0 += 1;
    }
    let mut rules: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_1 < n {
        let mut r: ::core::ffi::c_int = eng_guest_load_binfmt_file(g, paths[i_1 as usize]);
        if r > 0 as ::core::ffi::c_int {
            rules += r;
        }
        free(names[i_1 as usize] as *mut ::core::ffi::c_void);
        free(paths[i_1 as usize] as *mut ::core::ffi::c_void);
        i_1 += 1;
    }
    return rules;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_make_mountpoint(
    mut g: *mut eng_guest,
    mut guest: *const ::core::ffi::c_char,
    mut is_dir: ::core::ffi::c_int,
) {
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        guest,
    );
    let mut fd: ::core::ffi::c_int = dup((*g).rootfd);
    let mut rfd: ::core::ffi::c_int = open(
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        O_RDONLY | O_DIRECTORY | O_CLOEXEC,
    );
    if fd >= 0 as ::core::ffi::c_int {
        close(fd);
    }
    fd = rfd;
    let mut save: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut c: *mut ::core::ffi::c_char = strtok_r(
        &raw mut buf as *mut ::core::ffi::c_char,
        b"/\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut save,
    );
    while !c.is_null() && fd >= 0 as ::core::ffi::c_int {
        let mut next: *mut ::core::ffi::c_char = strtok_r(
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            b"/\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut save,
        );
        let mut last: ::core::ffi::c_int = next.is_null() as ::core::ffi::c_int;
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
        if fstatat(fd, c, &raw mut st, AT_SYMLINK_NOFOLLOW) != 0 as ::core::ffi::c_int {
            let mut m: eng_meta = eng_meta {
                uid: 0 as uint32_t,
                gid: 0 as uint32_t,
                mode: 0,
                nlink: 0,
                major: 0,
                minor: 0,
                present: 1 as ::core::ffi::c_int,
            };
            if last == 0 || is_dir != 0 {
                if mkdirat(fd, c, 0o700 as __mode_t) != 0 as ::core::ffi::c_int {
                    break;
                }
                m.mode = (S_IFDIR | 0o755 as ::core::ffi::c_int) as uint32_t;
                let mut d: ::core::ffi::c_int =
                    openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
                if d >= 0 as ::core::ffi::c_int {
                    let mut v: [::core::ffi::c_char; 64] = [0; 64];
                    let mut n: ::core::ffi::c_int = snprintf(
                        &raw mut v as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                        b"1 0 0 %o 0 0,0\0".as_ptr() as *const ::core::ffi::c_char,
                        m.mode,
                    );
                    fsetxattr(
                        d,
                        ENG_META_XATTR.as_ptr(),
                        &raw mut v as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                        n as size_t,
                        0 as ::core::ffi::c_int,
                    );
                    close(d);
                }
            } else {
                let mut f: ::core::ffi::c_int = openat(
                    fd,
                    c,
                    O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                    0o600 as ::core::ffi::c_int,
                );
                if f >= 0 as ::core::ffi::c_int {
                    let mut v_0: [::core::ffi::c_char; 64] = [0; 64];
                    let mut n_0: ::core::ffi::c_int = snprintf(
                        &raw mut v_0 as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                        b"1 0 0 %o 0 0,0\0".as_ptr() as *const ::core::ffi::c_char,
                        (S_IFREG | 0o644 as ::core::ffi::c_int) as ::core::ffi::c_uint,
                    );
                    fsetxattr(
                        f,
                        ENG_META_XATTR.as_ptr(),
                        &raw mut v_0 as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                        n_0 as size_t,
                        0 as ::core::ffi::c_int,
                    );
                    close(f);
                }
                break;
            }
        } else if !(st.st_mode & __S_IFMT as __mode_t == 0o40000 as __mode_t) {
            break;
        }
        if last != 0 {
            break;
        }
        let mut nfd: ::core::ffi::c_int =
            openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        fd = nfd;
        c = next;
    }
    if fd >= 0 as ::core::ffi::c_int {
        close(fd);
    }
}
