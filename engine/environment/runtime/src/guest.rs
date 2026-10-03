//! Guest roots, binds, hidden paths, virtual identity and binfmt configuration.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    unsafe fn closedir(_: *mut DIR) -> i32;
    unsafe fn opendir(_: *const ::core::ffi::c_char) -> *mut DIR;
    unsafe fn readdir(_: *mut DIR) -> *mut dirent;
    #[cfg(target_os = "linux")]
    #[link_name = "__errno_location"]
    unsafe fn errno() -> *mut i32;
    #[cfg(target_os = "android")]
    #[link_name = "__errno"]
    unsafe fn errno() -> *mut i32;
    unsafe fn open(_: *const ::core::ffi::c_char, _: i32, ...) -> i32;
    unsafe fn openat(_: i32, _: *const ::core::ffi::c_char, _: i32, ...) -> i32;
    unsafe fn fclose(_: *mut FILE) -> i32;
    unsafe fn fopen(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> *mut FILE;
    unsafe fn snprintf(
        _: *mut ::core::ffi::c_char,
        _: usize,
        _: *const ::core::ffi::c_char,
        ...
    ) -> i32;
    unsafe fn fgets(_: *mut ::core::ffi::c_char, _: i32, _: *mut FILE) -> *mut ::core::ffi::c_char;
    unsafe fn strtoul(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
        _: i32,
    ) -> u64;
    unsafe fn calloc(_: usize, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn free(_: *mut ::core::ffi::c_void);
    unsafe fn realpath(
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn memset(_: *mut ::core::ffi::c_void, _: i32, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn strcmp(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn strncmp(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn strdup(_: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn strchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strtok_r(
        _: *mut ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(_: *const ::core::ffi::c_char) -> usize;
    unsafe fn strerror(_: i32) -> *mut ::core::ffi::c_char;
    unsafe fn stat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
    unsafe fn fstatat(_: i32, _: *const ::core::ffi::c_char, _: *mut stat, _: i32) -> i32;
    unsafe fn lstat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
    unsafe fn mkdir(_: *const ::core::ffi::c_char, _: u32) -> i32;
    unsafe fn mkdirat(_: i32, _: *const ::core::ffi::c_char, _: u32) -> i32;
    unsafe fn access(_: *const ::core::ffi::c_char, _: i32) -> i32;
    unsafe fn close(_: i32) -> i32;
    unsafe fn dup(_: i32) -> i32;
    #[cfg(target_os = "linux")]
    unsafe fn symlink(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    #[cfg(target_os = "android")]
    unsafe fn symlink(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn eng_meta_write(_: *const ::core::ffi::c_char, _: i32, _: *const eng_meta) -> i32;
    #[cfg(target_os = "linux")]
    unsafe fn gnu_dev_major(_: u64) -> u32;
    #[cfg(target_os = "linux")]
    unsafe fn gnu_dev_minor(_: u64) -> u32;
    unsafe fn fsetxattr(
        _: i32,
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_void,
        _: usize,
        _: i32,
    ) -> i32;
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
#[cfg(target_os = "linux")]
#[repr(C)]
pub struct __dirstream {
    _opaque: [u8; 0],
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
pub struct eng_meta {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub nlink: u32,
    pub major: u32,
    pub minor: u32,
    pub present: i32,
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
pub struct timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
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
pub const PATH_MAX: i32 = 4096 as i32;
pub const ENG_MAX_BINDS: i32 = 32 as i32;
pub const ENG_MAX_HIDES: i32 = 16 as i32;
pub const ENG_MAX_BINFMT: i32 = 32 as i32;
pub const ENG_LOC_ROOTFS: i32 = -1 as i32;
pub const ENG_LOC_NONE: i32 = -2 as i32;
pub const EINVAL: i32 = 22 as i32;
pub const ENOSPC: i32 = 28 as i32;
pub const ENAMETOOLONG: i32 = 36 as i32;
pub const O_RDONLY: i32 = 0 as i32;
pub const O_WRONLY: i32 = 0o1 as i32;
pub const O_CREAT: i32 = 0o100 as i32;
pub const O_EXCL: i32 = 0o200 as i32;
#[cfg(target_os = "linux")]
pub const __O_DIRECTORY: i32 = 0o200000 as i32;
#[cfg(target_os = "linux")]
pub const __O_NOFOLLOW: i32 = 0o400000 as i32;
#[cfg(target_os = "linux")]
pub const __O_CLOEXEC: i32 = 0o2000000 as i32;
#[cfg(target_os = "linux")]
pub const __O_PATH: i32 = 0o10000000 as i32;
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
pub const __S_IFREG: i32 = 0o100000 as i32;
pub const AT_SYMLINK_NOFOLLOW: i32 = 0x100 as i32;
#[cfg(target_os = "linux")]
pub const S_IFDIR: i32 = __S_IFDIR;
#[cfg(target_os = "android")]
pub const S_IFDIR: i32 = 0o40000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFCHR: i32 = __S_IFCHR;
#[cfg(target_os = "android")]
pub const S_IFCHR: i32 = 0o20000 as i32;
#[cfg(target_os = "linux")]
pub const S_IFREG: i32 = __S_IFREG;
#[cfg(target_os = "android")]
pub const S_IFREG: i32 = 0o100000 as i32;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const F_OK: i32 = 0 as i32;
pub const ENG_META_XATTR: [::core::ffi::c_char; 19] = unsafe {
    ::core::mem::transmute::<[u8; 19], [::core::ffi::c_char; 19]>(*b"user.workflow.meta\0")
};
pub const S_IFMT: i32 = 0o170000 as i32;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_open(mut root: *const ::core::ffi::c_char) -> *mut eng_guest {
    let mut g: *mut eng_guest =
        calloc(1 as usize, ::core::mem::size_of::<eng_guest>()) as *mut eng_guest;
    if g.is_null() {
        return ::core::ptr::null_mut::<eng_guest>();
    }
    if realpath(root, &raw mut (*g).root as *mut ::core::ffi::c_char).is_null() {
        free(g as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<eng_guest>();
    }
    (*g).rootlen = strlen(&raw mut (*g).root as *mut ::core::ffi::c_char);
    if (*g).rootlen == 1 as usize {
        (*g).rootlen = 0 as usize;
    }
    (*g).rootfd = open(
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        O_PATH | O_DIRECTORY | O_CLOEXEC,
    );
    if (*g).rootfd < 0 as i32 {
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
    if (*g).rootfd >= 0 as i32 {
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
        if *r as i32 == '/' as i32 {
            while *r.offset(1isize) as i32 == '/' as i32 {
                r = r.offset(1);
            }
        }
        r = r.offset(1);
    }
    *w = 0 as ::core::ffi::c_char;
    let mut n: usize = strlen(p);
    while n > 1 as usize && *p.offset(n.wrapping_sub(1 as usize) as isize) as i32 == '/' as i32 {
        n = n.wrapping_sub(1);
        *p.offset(n as isize) = 0 as ::core::ffi::c_char;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_add_bind(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut guest: *const ::core::ffi::c_char,
) -> i32 {
    return eng_guest_add_bind_owned(g, host, guest, 1000 as u32, 1000 as u32);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_add_bind_owned(
    mut g: *mut eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut guest: *const ::core::ffi::c_char,
    mut uid: u32,
    mut gid: u32,
) -> i32 {
    if (*g).nbinds >= ENG_MAX_BINDS {
        return -ENOSPC;
    }
    if *guest.offset(0isize) as i32 != '/' as i32 {
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
        0 as i32,
        ::core::mem::size_of::<eng_bind>(),
    );
    let mut real: [::core::ffi::c_char; 4096] = [0; 4096];
    if realpath(host, &raw mut real as *mut ::core::ffi::c_char).is_null() {
        return -*errno();
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
    if b.hlen == 1 as usize {
        b.hlen = 0 as usize;
    }
    let mut i: i32 = 0 as i32;
    while i < (*g).nbinds {
        if strcmp(
            &raw mut (*(&raw mut (*g).binds as *mut eng_bind).offset(i as isize)).guest
                as *mut ::core::ffi::c_char,
            &raw mut b.guest as *mut ::core::ffi::c_char,
        ) == 0
        {
            (*g).binds[i as usize] = b;
            return 0 as i32;
        }
        i += 1;
    }
    let mut pos: i32 = (*g).nbinds;
    while pos > 0 as i32 && (*g).binds[(pos - 1 as i32) as usize].glen < b.glen {
        (*g).binds[pos as usize] = (*g).binds[(pos - 1 as i32) as usize];
        pos -= 1;
    }
    (*g).binds[pos as usize] = b;
    (*g).nbinds += 1;
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_add_hide(
    mut g: *mut eng_guest,
    mut guest: *const ::core::ffi::c_char,
) -> i32 {
    if (*g).nhides >= ENG_MAX_HIDES {
        return -ENOSPC;
    }
    snprintf(
        &raw mut *(&raw mut (*g).hides as *mut [::core::ffi::c_char; 4096])
            .offset((*g).nhides as isize) as *mut ::core::ffi::c_char,
        PATH_MAX as usize,
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        guest,
    );
    canon_guest(
        &raw mut *(&raw mut (*g).hides as *mut [::core::ffi::c_char; 4096])
            .offset((*g).nhides as isize) as *mut ::core::ffi::c_char,
    );
    (*g).nhides += 1;
    return 0 as i32;
}
unsafe extern "C" fn prefix_match(
    mut path: *const ::core::ffi::c_char,
    mut pre: *const ::core::ffi::c_char,
    mut plen: usize,
) -> i32 {
    return (strncmp(path, pre, plen) == 0 as i32
        && (*path.offset(plen as isize) as i32 == 0 as i32
            || *path.offset(plen as isize) as i32 == '/' as i32)) as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_locate(
    mut g: *const eng_guest,
    mut host: *const ::core::ffi::c_char,
) -> i32 {
    let mut best: i32 = -1 as i32;
    let mut i: i32 = 0 as i32;
    while i < (*g).nbinds {
        let mut b: *const eng_bind = (&raw const (*g).binds as *const eng_bind).offset(i as isize);
        if ((*b).hlen == 0 as usize
            || prefix_match(
                host,
                &raw const (*b).host as *const ::core::ffi::c_char,
                (*b).hlen,
            ) != 0)
            && (best < 0 as i32 || (*b).hlen > (*g).binds[best as usize].hlen)
        {
            best = i;
        }
        i += 1;
    }
    let mut rl: usize = (*g).rootlen;
    let mut in_root: i32 = (rl == 0 as usize
        || prefix_match(host, &raw const (*g).root as *const ::core::ffi::c_char, rl) != 0)
        as i32;
    if best >= 0 as i32 && (in_root == 0 || (*g).binds[best as usize].hlen >= rl) {
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
    mut uid: *mut u32,
    mut gid: *mut u32,
) {
    let mut loc: i32 = eng_guest_locate(g, host);
    if loc >= 0 as i32 {
        *uid = (*g).binds[loc as usize].uid;
        *gid = (*g).binds[loc as usize].gid;
        return;
    }
    *uid = 0 as u32;
    *gid = 0 as u32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_is_hidden(
    mut g: *const eng_guest,
    mut guest: *const ::core::ffi::c_char,
) -> i32 {
    let mut i: i32 = 0 as i32;
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
            return 1 as i32;
        }
        i += 1;
    }
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_to_host(
    mut g: *const eng_guest,
    mut guest: *const ::core::ffi::c_char,
    mut host: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut i: i32 = 0 as i32;
    while i < (*g).nbinds {
        let mut b: *const eng_bind = (&raw const (*g).binds as *const eng_bind).offset(i as isize);
        if prefix_match(
            guest,
            &raw const (*b).guest as *const ::core::ffi::c_char,
            (*b).glen,
        ) != 0
        {
            let mut n: i32 = snprintf(
                host,
                cap,
                b"%.*s%s\0".as_ptr() as *const ::core::ffi::c_char,
                (*b).hlen as i32,
                &raw const (*b).host as *const ::core::ffi::c_char,
                guest.offset((*b).glen as isize),
            );
            if n <= 0 as i32 || n as usize >= cap {
                return -ENAMETOOLONG;
            }
            if *host.offset(0isize) as i32 == 0 as i32 {
                *host.offset(0isize) = '/' as ::core::ffi::c_char;
                *host.offset(1isize) = 0 as ::core::ffi::c_char;
            }
            return 0 as i32;
        }
        i += 1;
    }
    let mut n_0: i32 = snprintf(
        host,
        cap,
        b"%.*s%s\0".as_ptr() as *const ::core::ffi::c_char,
        (*g).rootlen as i32,
        &raw const (*g).root as *const ::core::ffi::c_char,
        guest,
    );
    if n_0 <= 0 as i32 || n_0 as usize >= cap {
        return -ENAMETOOLONG;
    }
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_host_to_guest(
    mut g: *const eng_guest,
    mut host: *const ::core::ffi::c_char,
    mut guest: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut best: *const eng_bind = ::core::ptr::null::<eng_bind>();
    let mut i: i32 = 0 as i32;
    while i < (*g).nbinds {
        let mut b: *const eng_bind = (&raw const (*g).binds as *const eng_bind).offset(i as isize);
        if (*b).hlen == 0 as usize
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
    let mut rl: usize = (*g).rootlen;
    let mut in_root: i32 = (rl == 0 as usize
        || prefix_match(host, &raw const (*g).root as *const ::core::ffi::c_char, rl) != 0)
        as i32;
    if !best.is_null() && (in_root == 0 || (*best).hlen >= rl) {
        let mut n: i32 = snprintf(
            guest,
            cap,
            b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw const (*best).guest as *const ::core::ffi::c_char,
            host.offset((*best).hlen as isize),
        );
        return if n > 0 as i32 && (n as usize) < cap {
            0 as i32
        } else {
            -1 as i32
        };
    }
    if in_root != 0 {
        let mut rest: *const ::core::ffi::c_char = host.offset(rl as isize);
        let mut n_0: i32 = snprintf(
            guest,
            cap,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            if *rest as i32 != 0 {
                rest
            } else {
                b"/\0".as_ptr() as *const ::core::ffi::c_char
            },
        );
        return if n_0 > 0 as i32 && (n_0 as usize) < cap {
            0 as i32
        } else {
            -1 as i32
        };
    }
    return -1 as i32;
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
    let mut st: stat = platform_empty_stat();
    let mut hs: stat = platform_empty_stat();
    if lstat(&raw mut p as *mut ::core::ffi::c_char, &raw mut st) == 0 as i32 {
        return;
    }
    let mut f: i32 = open(
        &raw mut p as *mut ::core::ffi::c_char,
        O_CREAT | O_EXCL | O_WRONLY | O_CLOEXEC,
        0o600 as i32,
    );
    if f < 0 as i32 {
        return;
    }
    close(f);
    let mut m: eng_meta = eng_meta {
        uid: 0 as u32,
        gid: 0 as u32,
        mode: (S_IFREG | 0o666 as i32) as u32,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 1 as i32,
    };
    if stat(hostdev, &raw mut hs) == 0 as i32 && hs.st_mode & S_IFMT as u32 == S_IFCHR as u32 {
        m.mode = (S_IFCHR | 0o666 as i32) as u32;
        #[cfg(target_os = "linux")]
        {
            m.major = gnu_dev_major(hs.st_rdev) as u32;
        }
        #[cfg(target_os = "android")]
        {
            m.major = (hs.st_rdev as u64 >> 32 as i32 & 0xfffff000 as u64
                | (hs.st_rdev >> 8 as i32 & 0xfff as u64) as u64) as u32
                as u32;
        }
        #[cfg(target_os = "linux")]
        {
            m.minor = gnu_dev_minor(hs.st_rdev) as u32;
        }
        #[cfg(target_os = "android")]
        {
            m.minor = (hs.st_rdev >> 12 as i32 & 0xffffff00 as u64 | hs.st_rdev & 0xff as u64)
                as u32 as u32;
        }
    }
    eng_meta_write(&raw mut p as *mut ::core::ffi::c_char, 1 as i32, &raw mut m);
}
unsafe extern "C" fn ensure_dir(
    mut g: *const eng_guest,
    mut rel: *const ::core::ffi::c_char,
    mut mode: u32,
) {
    let mut p: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw const (*g).root as *const ::core::ffi::c_char,
        rel,
    );
    if mkdir(&raw mut p as *mut ::core::ffi::c_char, 0o700 as u32) != 0 as i32 {
        return;
    }
    let mut m: eng_meta = eng_meta {
        uid: 0 as u32,
        gid: 0 as u32,
        mode: S_IFDIR as u32 | mode,
        nlink: 0,
        major: 0,
        minor: 0,
        present: 1 as i32,
    };
    eng_meta_write(&raw mut p as *mut ::core::ffi::c_char, 1 as i32, &raw mut m);
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
    let mut st: stat = platform_empty_stat();
    if lstat(&raw mut p as *mut ::core::ffi::c_char, &raw mut st) == 0 as i32 {
        return;
    }
    if symlink(target, &raw mut p as *mut ::core::ffi::c_char) != 0 as i32 {
        eng_logf!(
            eng_log_level::ENG_LOG_DEBUG,
            b"placeholder symlink %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut p as *mut ::core::ffi::c_char,
            strerror(*errno()),
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_default_binds(mut g: *mut eng_guest) -> i32 {
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
        0o555 as u32,
    );
    ensure_dir(
        g,
        b"sys\0".as_ptr() as *const ::core::ffi::c_char,
        0o555 as u32,
    );
    ensure_dir(
        g,
        b"dev\0".as_ptr() as *const ::core::ffi::c_char,
        0o755 as u32,
    );
    ensure_dir(
        g,
        b"dev/pts\0".as_ptr() as *const ::core::ffi::c_char,
        0o755 as u32,
    );
    ensure_dir(
        g,
        b"dev/shm\0".as_ptr() as *const ::core::ffi::c_char,
        0o1777 as u32,
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
    let mut rc: i32 = 0 as i32;
    if access(b"/proc\0".as_ptr() as *const ::core::ffi::c_char, F_OK) == 0 as i32 {
        rc |= eng_guest_add_bind_owned(
            g,
            b"/proc\0".as_ptr() as *const ::core::ffi::c_char,
            b"/proc\0".as_ptr() as *const ::core::ffi::c_char,
            0 as u32,
            0 as u32,
        );
    }
    if access(b"/sys\0".as_ptr() as *const ::core::ffi::c_char, F_OK) == 0 as i32 {
        eng_guest_add_bind_owned(
            g,
            b"/sys\0".as_ptr() as *const ::core::ffi::c_char,
            b"/sys\0".as_ptr() as *const ::core::ffi::c_char,
            0 as u32,
            0 as u32,
        );
    }
    eng_guest_add_hide(
        g,
        b"/sys/fs/selinux\0".as_ptr() as *const ::core::ffi::c_char,
    );
    let mut i: usize = 0 as usize;
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
        if access(&raw mut host as *mut ::core::ffi::c_char, F_OK) == 0 as i32 {
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
                0 as u32,
                0 as u32,
            );
        }
        i = i.wrapping_add(1);
    }
    if access(b"/dev/pts\0".as_ptr() as *const ::core::ffi::c_char, F_OK) == 0 as i32 {
        eng_guest_add_bind_owned(
            g,
            b"/dev/pts\0".as_ptr() as *const ::core::ffi::c_char,
            b"/dev/pts\0".as_ptr() as *const ::core::ffi::c_char,
            0 as u32,
            0 as u32,
        );
    }
    #[cfg(target_os = "android")]
    static mut sysdirs: [*const ::core::ffi::c_char; 8] = [
        b"/system\0".as_ptr() as *const ::core::ffi::c_char,
        b"/apex\0".as_ptr() as *const ::core::ffi::c_char,
        b"/vendor\0".as_ptr() as *const ::core::ffi::c_char,
        b"/product\0".as_ptr() as *const ::core::ffi::c_char,
        b"/system_ext\0".as_ptr() as *const ::core::ffi::c_char,
        b"/odm\0".as_ptr() as *const ::core::ffi::c_char,
        b"/dev/__properties__\0".as_ptr() as *const ::core::ffi::c_char,
        b"/dev/socket\0".as_ptr() as *const ::core::ffi::c_char,
    ];
    #[cfg(target_os = "android")]
    let mut st: stat = platform_empty_stat();
    #[cfg(target_os = "android")]
    let mut lc: *const ::core::ffi::c_char =
        b"/linkerconfig/ld.config.txt\0".as_ptr() as *const ::core::ffi::c_char;
    #[cfg(target_os = "android")]
    if stat(lc, &raw mut st) == 0 as i32 && st.st_mode & S_IFMT as u32 == S_IFREG as u32 {
        eng_guest_make_mountpoint(g, lc, 0 as i32);
        eng_guest_add_bind_owned(g, lc, lc, 0 as u32, 0 as u32);
    }
    #[cfg(target_os = "android")]
    let mut i_0: usize = 0 as usize;
    #[cfg(target_os = "android")]
    while i_0
        < ::core::mem::size_of::<[*const ::core::ffi::c_char; 8]>()
            .wrapping_div(::core::mem::size_of::<*const ::core::ffi::c_char>())
    {
        let mut st_0: stat = platform_empty_stat();
        if stat(sysdirs[i_0], &raw mut st_0) != 0 as i32
            || !(st_0.st_mode & S_IFMT as u32 == S_IFDIR as u32)
        {
            eng_logf!(
                eng_log_level::ENG_LOG_DEBUG,
                b"android bind %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                sysdirs[i_0],
                strerror(*errno()),
            );
        } else {
            eng_guest_make_mountpoint(g, sysdirs[i_0], 1 as i32);
            let mut brc: i32 =
                eng_guest_add_bind_owned(g, sysdirs[i_0], sysdirs[i_0], 0 as u32, 0 as u32);
            if brc != 0 {
                eng_logf!(
                    eng_log_level::ENG_LOG_WARN,
                    b"android bind %s: %s\0".as_ptr() as *const ::core::ffi::c_char,
                    sysdirs[i_0],
                    strerror(-brc),
                );
            }
        }
        i_0 = i_0.wrapping_add(1);
    }
    return rc;
}
unsafe extern "C" fn unescape(
    mut r#in: *const ::core::ffi::c_char,
    mut out: *mut u8,
    mut cap: u32,
    mut len: *mut u32,
) -> i32 {
    let mut n: u32 = 0 as u32;
    let mut p: *const ::core::ffi::c_char = r#in;
    while *p != 0 {
        let mut c: u32 = *p as u8 as u32;
        if c == '\\' as u32
            && *p.offset(1isize) as i32 == 'x' as i32
            && *p.offset(2isize) as i32 != 0
            && *p.offset(3isize) as i32 != 0
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
                16 as i32,
            ) as u32;
            if *end != 0 {
                return -EINVAL;
            }
            p = p.offset(3 as i32 as isize);
        } else if c == '\\' as u32 && *p.offset(1isize) as i32 == '\\' as i32 {
            p = p.offset(1);
        }
        if n >= cap {
            return -EINVAL;
        }
        let c2rust_fresh2 = n;
        n = n.wrapping_add(1);
        *out.offset(c2rust_fresh2 as isize) = c as u8;
        p = p.offset(1);
    }
    *len = n;
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_add_binfmt(
    mut g: *mut eng_guest,
    mut rule: *const ::core::ffi::c_char,
) -> i32 {
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
    let mut bl: usize = strlen(&raw mut buf as *mut ::core::ffi::c_char);
    while bl != 0
        && (buf[bl.wrapping_sub(1 as usize)] as i32 == '\n' as i32
            || buf[bl.wrapping_sub(1 as usize)] as i32 == '\r' as i32
            || buf[bl.wrapping_sub(1 as usize)] as i32 == ' ' as i32)
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
        (&raw mut buf as *mut ::core::ffi::c_char).offset(1 as i32 as isize);
    let mut i: i32 = 0 as i32;
    while i < 7 as i32 {
        f[i as usize] = p;
        let mut q: *mut ::core::ffi::c_char = strchr(p, del as i32);
        if q.is_null() {
            if i < 5 as i32 {
                return -EINVAL;
            }
            let mut k: i32 = i + 1 as i32;
            while k < 7 as i32 {
                f[k as usize] = p.offset(strlen(p) as isize);
                k += 1;
            }
            break;
        } else {
            *q = 0 as ::core::ffi::c_char;
            p = q.offset(1 as i32 as isize);
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
        0 as i32,
        ::core::mem::size_of::<eng_binfmt>(),
    );
    snprintf(
        &raw mut b.name as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        f[0usize],
    );
    if b.name[0usize] == 0
        || strlen(f[1usize]) != 1 as usize
        || *f[1usize].offset(0isize) as i32 != 'M' as i32
            && *f[1usize].offset(0isize) as i32 != 'E' as i32
    {
        return -EINVAL;
    }
    b.r#type = *f[1usize].offset(0isize);
    if *f[5usize].offset(0isize) as i32 != '/' as i32 {
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
        if *fl as i32 == 'P' as i32 {
            b.preserve_argv0 = 1 as i32;
        } else if *fl as i32 != 'O' as i32 && *fl as i32 != 'C' as i32 && *fl as i32 != 'F' as i32 {
            return -EINVAL;
        }
        fl = fl.offset(1);
    }
    if b.r#type as i32 == 'E' as i32 {
        if *f[3usize].offset(0isize) == 0 || !strchr(f[3usize], '/' as i32).is_null() {
            return -EINVAL;
        }
        snprintf(
            &raw mut b.ext as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            f[3usize],
        );
    } else {
        b.offset = if *f[2usize].offset(0isize) as i32 != 0 {
            strtoul(
                f[2usize],
                ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                10 as i32,
            ) as u32
        } else {
            0 as u32
        };
        if unescape(
            f[3usize],
            &raw mut b.magic as *mut u8,
            ::core::mem::size_of::<[u8; 128]>() as u32,
            &raw mut b.len,
        ) != 0
            || b.len == 0
        {
            return -EINVAL;
        }
        let mut ml: u32 = 0 as u32;
        if *f[4usize].offset(0isize) != 0 {
            if unescape(
                f[4usize],
                &raw mut b.mask as *mut u8,
                ::core::mem::size_of::<[u8; 128]>() as u32,
                &raw mut ml,
            ) != 0
                || ml != b.len
            {
                return -EINVAL;
            }
        } else {
            memset(
                &raw mut b.mask as *mut u8 as *mut ::core::ffi::c_void,
                0xff as i32,
                b.len as usize,
            );
        }
        if b.offset.wrapping_add(b.len) > 256 as u32 {
            return -EINVAL;
        }
        let mut i_0: u32 = 0 as u32;
        while i_0 < b.len {
            b.magic[i_0 as usize] =
                (b.magic[i_0 as usize] as i32 & b.mask[i_0 as usize] as i32) as u8;
            i_0 = i_0.wrapping_add(1);
        }
    }
    let mut i_1: i32 = 0 as i32;
    while i_1 < (*g).nbinfmt {
        if strcmp(
            &raw mut (*(&raw mut (*g).binfmt as *mut eng_binfmt).offset(i_1 as isize)).name
                as *mut ::core::ffi::c_char,
            &raw mut b.name as *mut ::core::ffi::c_char,
        ) == 0
        {
            (*g).binfmt[i_1 as usize] = b;
            return 0 as i32;
        }
        i_1 += 1;
    }
    let c2rust_fresh1 = (*g).nbinfmt;
    (*g).nbinfmt += 1;
    (*g).binfmt[c2rust_fresh1 as usize] = b;
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_load_binfmt_file(
    mut g: *mut eng_guest,
    mut host_path: *const ::core::ffi::c_char,
) -> i32 {
    let mut f: *mut FILE = fopen(host_path, b"re\0".as_ptr() as *const ::core::ffi::c_char);
    if f.is_null() {
        return -*errno();
    }
    let mut line: [::core::ffi::c_char; 8192] = [0; 8192];
    let mut n: i32 = 0 as i32;
    while !fgets(
        &raw mut line as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 8192]>() as i32,
        f,
    )
    .is_null()
    {
        if line[0usize] as i32 == '#' as i32
            || line[0usize] as i32 == ';' as i32
            || line[0usize] as i32 == '\n' as i32
            || line[0usize] == 0
        {
            continue;
        }
        if eng_guest_add_binfmt(g, &raw mut line as *mut ::core::ffi::c_char) == 0 as i32 {
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
) -> i32 {
    return strcmp(
        *(a as *const *mut ::core::ffi::c_char),
        *(b as *const *mut ::core::ffi::c_char),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_guest_load_binfmt_dirs(mut g: *mut eng_guest) -> i32 {
    static mut dirs: [*const ::core::ffi::c_char; 2] = [
        b"/usr/lib/binfmt.d\0".as_ptr() as *const ::core::ffi::c_char,
        b"/etc/binfmt.d\0".as_ptr() as *const ::core::ffi::c_char,
    ];
    let mut names: [*mut ::core::ffi::c_char; 128] =
        [::core::ptr::null_mut::<::core::ffi::c_char>(); 128];
    let mut paths: [*mut ::core::ffi::c_char; 128] =
        [::core::ptr::null_mut::<::core::ffi::c_char>(); 128];
    let mut n: i32 = 0 as i32;
    let mut d: usize = 0 as usize;
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
                    if !(!e.is_null() && n < 128 as i32) {
                        break;
                    }
                    let mut l: usize = strlen(&raw mut (*e).d_name as *mut ::core::ffi::c_char);
                    if l < 6 as usize
                        || strcmp(
                            (&raw mut (*e).d_name as *mut ::core::ffi::c_char)
                                .offset(l as isize)
                                .offset(-(5 as i32 as isize)),
                            b".conf\0".as_ptr() as *const ::core::ffi::c_char,
                        ) != 0
                    {
                        continue;
                    }
                    let mut dup_0: i32 = -1 as i32;
                    let mut i: i32 = 0 as i32;
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
                    if dup_0 >= 0 as i32 {
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
    let mut i_0: i32 = 1 as i32;
    while i_0 < n {
        let mut j: i32 = i_0;
        while j > 0 as i32 && strcmp(names[(j - 1 as i32) as usize], names[j as usize]) > 0 as i32 {
            let mut t: *mut ::core::ffi::c_char = names[j as usize];
            names[j as usize] = names[(j - 1 as i32) as usize];
            names[(j - 1 as i32) as usize] = t;
            t = paths[j as usize];
            paths[j as usize] = paths[(j - 1 as i32) as usize];
            paths[(j - 1 as i32) as usize] = t;
            j -= 1;
        }
        i_0 += 1;
    }
    let mut rules: i32 = 0 as i32;
    let mut i_1: i32 = 0 as i32;
    while i_1 < n {
        let mut r: i32 = eng_guest_load_binfmt_file(g, paths[i_1 as usize]);
        if r > 0 as i32 {
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
    mut is_dir: i32,
) {
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        guest,
    );
    let mut fd: i32 = dup((*g).rootfd);
    let mut rfd: i32 = open(
        &raw mut (*g).root as *mut ::core::ffi::c_char,
        O_RDONLY | O_DIRECTORY | O_CLOEXEC,
    );
    if fd >= 0 as i32 {
        close(fd);
    }
    fd = rfd;
    let mut save: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut c: *mut ::core::ffi::c_char = strtok_r(
        &raw mut buf as *mut ::core::ffi::c_char,
        b"/\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut save,
    );
    while !c.is_null() && fd >= 0 as i32 {
        let mut next: *mut ::core::ffi::c_char = strtok_r(
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            b"/\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut save,
        );
        let mut last: i32 = next.is_null() as i32;
        let mut st: stat = platform_empty_stat();
        if fstatat(fd, c, &raw mut st, AT_SYMLINK_NOFOLLOW) != 0 as i32 {
            let mut m: eng_meta = eng_meta {
                uid: 0 as u32,
                gid: 0 as u32,
                mode: 0,
                nlink: 0,
                major: 0,
                minor: 0,
                present: 1 as i32,
            };
            if last == 0 || is_dir != 0 {
                if mkdirat(fd, c, 0o700 as u32) != 0 as i32 {
                    break;
                }
                m.mode = (S_IFDIR | 0o755 as i32) as u32;
                let mut d: i32 = openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
                if d >= 0 as i32 {
                    let mut v: [::core::ffi::c_char; 64] = [0; 64];
                    let mut n: i32 = snprintf(
                        &raw mut v as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                        b"1 0 0 %o 0 0,0\0".as_ptr() as *const ::core::ffi::c_char,
                        m.mode,
                    );
                    fsetxattr(
                        d,
                        ENG_META_XATTR.as_ptr(),
                        &raw mut v as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                        n as usize,
                        0 as i32,
                    );
                    close(d);
                }
            } else {
                let mut f: i32 = openat(
                    fd,
                    c,
                    O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                    0o600 as i32,
                );
                if f >= 0 as i32 {
                    let mut v_0: [::core::ffi::c_char; 64] = [0; 64];
                    let mut n_0: i32 = snprintf(
                        &raw mut v_0 as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                        b"1 0 0 %o 0 0,0\0".as_ptr() as *const ::core::ffi::c_char,
                        (S_IFREG | 0o644 as i32) as u32,
                    );
                    fsetxattr(
                        f,
                        ENG_META_XATTR.as_ptr(),
                        &raw mut v_0 as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                        n_0 as usize,
                        0 as i32,
                    );
                    close(f);
                }
                break;
            }
        } else if !(st.st_mode & S_IFMT as u32 == S_IFDIR as u32) {
            break;
        }
        if last != 0 {
            break;
        }
        let mut nfd: i32 = openat(fd, c, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        fd = nfd;
        c = next;
    }
    if fd >= 0 as i32 {
        close(fd);
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
pub struct __sFILE {
    _opaque: [u8; 0],
}
#[cfg(target_os = "android")]
#[repr(C)]
pub struct DIR {
    _opaque: [u8; 0],
}
