//! Compatibility CLI contract consumed by the Workspace Engine server.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
#[repr(C)]
pub struct __sFILE {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    unsafe fn __errno() -> *mut ::core::ffi::c_int;
    unsafe fn dirname(__path: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    static mut stdout: *mut FILE;
    static mut stderr: *mut FILE;
    unsafe fn fprintf(
        __fp: *mut FILE,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn printf(__fmt: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    unsafe fn snprintf(
        __buf: *mut ::core::ffi::c_char,
        __size: size_t,
        __fmt: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn getenv(__name: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn realpath(
        __path: *const ::core::ffi::c_char,
        __resolved: *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strtol(
        __s: *const ::core::ffi::c_char,
        __end_ptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    unsafe fn strtoul(
        __s: *const ::core::ffi::c_char,
        __end_ptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulong;
    unsafe fn strchr(
        __s: *const ::core::ffi::c_char,
        __ch: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strrchr(
        __s: *const ::core::ffi::c_char,
        __ch: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strcmp(
        __lhs: *const ::core::ffi::c_char,
        __rhs: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
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
    unsafe fn lstat(__path: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn stat(__path: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn sysconf(__name: ::core::ffi::c_int) -> ::core::ffi::c_long;
    static mut environ: *mut *mut ::core::ffi::c_char;
    unsafe fn getuid() -> uid_t;
    unsafe fn access(
        __path: *const ::core::ffi::c_char,
        __mode: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn readlink(
        __path: *const ::core::ffi::c_char,
        __buf: *mut ::core::ffi::c_char,
        __buf_size: size_t,
    ) -> ssize_t;
    unsafe fn close(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn eng_guest_open(root: *const ::core::ffi::c_char) -> *mut eng_guest;
    unsafe fn eng_guest_close(g: *mut eng_guest);
    unsafe fn eng_guest_add_bind(
        g: *mut eng_guest,
        host: *const ::core::ffi::c_char,
        guest: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_guest_default_binds(g: *mut eng_guest) -> ::core::ffi::c_int;
    unsafe fn eng_guest_add_hide(
        g: *mut eng_guest,
        guest: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_guest_make_mountpoint(
        g: *mut eng_guest,
        guest: *const ::core::ffi::c_char,
        is_dir: ::core::ffi::c_int,
    );
    unsafe fn eng_guest_add_binfmt(
        g: *mut eng_guest,
        rule: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_guest_load_binfmt_file(
        g: *mut eng_guest,
        host_path: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_guest_load_binfmt_dirs(g: *mut eng_guest) -> ::core::ffi::c_int;
    unsafe fn eng_guest_to_host(
        g: *const eng_guest,
        guest: *const ::core::ffi::c_char,
        host: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_install(o: *const eng_install_opts) -> ::core::ffi::c_int;
    unsafe fn eng_clone(
        src_generation: *const ::core::ffi::c_char,
        dst_generation: *const ::core::ffi::c_char,
        quiet: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_verify(
        generation: *const ::core::ffi::c_char,
        quiet: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_remove_tree(path: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    unsafe fn eng_log_init(verbosity: ::core::ffi::c_int);
    unsafe fn eng_log_enabled(lv: eng_log_level) -> ::core::ffi::c_int;
    unsafe fn eng_tracer_run(cfg: *const eng_run_cfg) -> ::core::ffi::c_int;
    unsafe fn eng_link_fsck(
        g: *mut eng_guest,
        repair: ::core::ffi::c_int,
        out: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_instance_lock(
        g: *mut eng_guest,
        exclusive: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type uint32_t = u32;
pub type int64_t = i64;
pub type uint64_t = u64;
pub type __kernel_long_t = ::core::ffi::c_long;
pub type __kernel_ulong_t = ::core::ffi::c_ulong;
pub type __kernel_ino_t = __kernel_ulong_t;
pub type __kernel_mode_t = ::core::ffi::c_uint;
pub type __kernel_uid32_t = ::core::ffi::c_uint;
pub type __kernel_gid32_t = ::core::ffi::c_uint;
pub type __kernel_time_t = __kernel_long_t;
pub type __gid_t = __kernel_gid32_t;
pub type gid_t = __gid_t;
pub type __uid_t = __kernel_uid32_t;
pub type uid_t = __uid_t;
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
pub type FILE = __sFILE;
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
pub struct eng_install_opts {
    pub image: *const ::core::ffi::c_char,
    pub sha256: *const ::core::ffi::c_char,
    pub index: *const ::core::ffi::c_char,
    pub target: *const ::core::ffi::c_char,
    pub profile: *const ::core::ffi::c_char,
    pub quiet: ::core::ffi::c_int,
    pub keep_partial: ::core::ffi::c_int,
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
pub const ENOENT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const EAGAIN: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const EWOULDBLOCK: ::core::ffi::c_int = EAGAIN;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const S_IFMT: ::core::ffi::c_int = 0o170000 as ::core::ffi::c_int;
pub const S_IFLNK: ::core::ffi::c_int = 0o120000 as ::core::ffi::c_int;
pub const S_IFREG: ::core::ffi::c_int = 0o100000 as ::core::ffi::c_int;
pub const S_IFDIR: ::core::ffi::c_int = 0o40000 as ::core::ffi::c_int;
pub const _SC_PAGESIZE: ::core::ffi::c_int = 0x27 as ::core::ffi::c_int;
pub const X_OK: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ENG_MAX_BINDS: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const ENG_MAX_HIDES: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const ENG_MAX_BINFMT: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const ENG_ARCH_NAME: [::core::ffi::c_char; 6] =
    unsafe { ::core::mem::transmute::<[u8; 6], [::core::ffi::c_char; 6]>(*b"arm64\0") };
unsafe extern "C" fn usage() {
    fprintf(
        stderr,
        b"usage: workflow-engine run [--root DIR] [--cwd GUESTDIR] [--bind HOST:GUEST]...\n                           [--hide GUESTPATH]... [--loader FILE] [--user work|root]\n                           [--uid N] [--gid N] [--no-default-binds] [--no-filemap]\n                           [--wait-all] [-v] -- CMD [ARGS...]\n       workflow-engine probe\n       workflow-engine fsck --root DIR [--repair]\n       workflow-engine install --image FILE|- (--index image.json | --sha256 HEX) --target GEN\n                               [--profile workspace|base|any] [--quiet]\n       workflow-engine clone --from GEN --to GEN\n       workflow-engine verify --generation GEN\n       workflow-engine remove --generation GEN\n\0"
            .as_ptr() as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn default_loader(
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut env: *const ::core::ffi::c_char =
        getenv(b"WORKFLOW_ENGINE_LOADER\0".as_ptr() as *const ::core::ffi::c_char);
    if !env.is_null() && *env as ::core::ffi::c_int != 0 {
        snprintf(
            out,
            cap,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            env,
        );
        return 0 as ::core::ffi::c_int;
    }
    let mut self_0: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut n: ssize_t = readlink(
        b"/proc/self/exe\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut self_0 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as size_t),
    );
    if n <= 0 as ssize_t {
        return -1 as ::core::ffi::c_int;
    }
    self_0[n as usize] = 0 as ::core::ffi::c_char;
    let mut dir: *mut ::core::ffi::c_char = dirname(&raw mut self_0 as *mut ::core::ffi::c_char);
    snprintf(
        out,
        cap,
        b"%s/libworkflow-loader.so\0".as_ptr() as *const ::core::ffi::c_char,
        dir,
    );
    if access(out, X_OK) == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    snprintf(
        out,
        cap,
        b"%s/loader\0".as_ptr() as *const ::core::ffi::c_char,
        dir,
    );
    return if access(out, X_OK) == 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_int
    } else {
        -1 as ::core::ffi::c_int
    };
}
unsafe extern "C" fn guest_path_search(
    mut g: *mut eng_guest,
    mut name: *const ::core::ffi::c_char,
    mut envp: *mut *mut ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut path: *const ::core::ffi::c_char =
        b"/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\0".as_ptr()
            as *const ::core::ffi::c_char;
    let mut e: *mut *mut ::core::ffi::c_char = envp;
    while !e.is_null() && !(*e).is_null() {
        if strncmp(
            *e,
            b"PATH=\0".as_ptr() as *const ::core::ffi::c_char,
            5 as size_t,
        ) == 0
        {
            path = (*e).offset(5 as ::core::ffi::c_int as isize);
            break;
        } else {
            e = e.offset(1);
        }
    }
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        path,
    );
    let mut save: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut dir: *mut ::core::ffi::c_char = strtok_r(
        &raw mut buf as *mut ::core::ffi::c_char,
        b":\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut save,
    );
    while !dir.is_null() {
        if *dir.offset(0isize) as ::core::ffi::c_int == '/' as ::core::ffi::c_int {
            let mut gp: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut hp: [::core::ffi::c_char; 4096] = [0; 4096];
            snprintf(
                &raw mut gp as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
                dir,
                name,
            );
            if eng_guest_to_host(
                g,
                &raw mut gp as *mut ::core::ffi::c_char,
                &raw mut hp as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            ) == 0
            {
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
                if stat(&raw mut hp as *mut ::core::ffi::c_char, &raw mut st)
                    == 0 as ::core::ffi::c_int
                    && st.st_mode & S_IFMT as mode_t == S_IFREG as mode_t
                {
                    snprintf(
                        out,
                        cap,
                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut gp as *mut ::core::ffi::c_char,
                    );
                    return 0 as ::core::ffi::c_int;
                }
                if lstat(&raw mut hp as *mut ::core::ffi::c_char, &raw mut st)
                    == 0 as ::core::ffi::c_int
                    && st.st_mode & S_IFMT as mode_t == S_IFLNK as mode_t
                {
                    snprintf(
                        out,
                        cap,
                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut gp as *mut ::core::ffi::c_char,
                    );
                    return 0 as ::core::ffi::c_int;
                }
            }
        }
        dir = strtok_r(
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            b":\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut save,
        );
    }
    return -1 as ::core::ffi::c_int;
}
unsafe extern "C" fn cmd_fsck(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut root: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut repair: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    while i < argc {
        if strcmp(
            *argv.offset(i as isize),
            b"--root\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
            && (i + 1 as ::core::ffi::c_int) < argc
        {
            i += 1;
            root = *argv.offset(i as isize);
        } else if strcmp(
            *argv.offset(i as isize),
            b"--repair\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        {
            repair = 1 as ::core::ffi::c_int;
        } else {
            usage();
            return 2 as ::core::ffi::c_int;
        }
        i += 1;
    }
    if root.is_null() {
        usage();
        return 2 as ::core::ffi::c_int;
    }
    let mut g: *mut eng_guest = eng_guest_open(root);
    if g.is_null() {
        fprintf(
            stderr,
            b"fsck: cannot open %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            root,
            strerror(*__errno()),
        );
        return 125 as ::core::ffi::c_int;
    }
    let mut lk: ::core::ffi::c_int = eng_instance_lock(g, repair);
    if lk < 0 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"fsck: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            if lk == -EWOULDBLOCK {
                b"rootfs in use by a running engine\0".as_ptr() as *const ::core::ffi::c_char
            } else {
                strerror(-lk) as *const ::core::ffi::c_char
            },
        );
        return if lk == -EWOULDBLOCK {
            3 as ::core::ffi::c_int
        } else {
            125 as ::core::ffi::c_int
        };
    }
    let mut rc: ::core::ffi::c_int = eng_link_fsck(g, repair, stdout as *mut ::core::ffi::c_void);
    close(lk);
    return if rc < 0 as ::core::ffi::c_int {
        125 as ::core::ffi::c_int
    } else if rc > 0 as ::core::ffi::c_int {
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
}
unsafe extern "C" fn cmd_lifecycle(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut o: eng_install_opts = eng_install_opts {
        image: ::core::ptr::null::<::core::ffi::c_char>(),
        sha256: ::core::ptr::null::<::core::ffi::c_char>(),
        index: ::core::ptr::null::<::core::ffi::c_char>(),
        target: ::core::ptr::null::<::core::ffi::c_char>(),
        profile: b"workspace\0".as_ptr() as *const ::core::ffi::c_char,
        quiet: 0,
        keep_partial: 0,
    };
    let mut from: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut r#gen: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut i: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    while i < argc {
        let mut a: *const ::core::ffi::c_char = *argv.offset(i as isize);
        let mut more: ::core::ffi::c_int =
            ((i + 1 as ::core::ffi::c_int) < argc) as ::core::ffi::c_int;
        if strcmp(a, b"--image\0".as_ptr() as *const ::core::ffi::c_char) == 0 && more != 0 {
            i += 1;
            o.image = *argv.offset(i as isize);
        } else if strcmp(a, b"--sha256\0".as_ptr() as *const ::core::ffi::c_char) == 0 && more != 0
        {
            i += 1;
            o.sha256 = *argv.offset(i as isize);
        } else if strcmp(a, b"--index\0".as_ptr() as *const ::core::ffi::c_char) == 0 && more != 0 {
            i += 1;
            o.index = *argv.offset(i as isize);
        } else if (strcmp(a, b"--target\0".as_ptr() as *const ::core::ffi::c_char) == 0
            || strcmp(a, b"--to\0".as_ptr() as *const ::core::ffi::c_char) == 0)
            && more != 0
        {
            i += 1;
            o.target = *argv.offset(i as isize);
        } else if strcmp(a, b"--profile\0".as_ptr() as *const ::core::ffi::c_char) == 0 && more != 0
        {
            i += 1;
            o.profile = *argv.offset(i as isize);
        } else if strcmp(a, b"--from\0".as_ptr() as *const ::core::ffi::c_char) == 0 && more != 0 {
            i += 1;
            from = *argv.offset(i as isize);
        } else if strcmp(a, b"--generation\0".as_ptr() as *const ::core::ffi::c_char) == 0
            && more != 0
        {
            i += 1;
            r#gen = *argv.offset(i as isize);
        } else if strcmp(a, b"--quiet\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            o.quiet = 1 as ::core::ffi::c_int;
        } else if strcmp(
            a,
            b"--keep-partial\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        {
            o.keep_partial = 1 as ::core::ffi::c_int;
        } else {
            fprintf(
                stderr,
                b"%s: unknown option %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                *argv.offset(1isize),
                a,
            );
            usage();
            return 2 as ::core::ffi::c_int;
        }
        i += 1;
    }
    if strcmp(
        *argv.offset(1isize),
        b"install\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        if o.image.is_null() || o.target.is_null() {
            usage();
            return 2 as ::core::ffi::c_int;
        }
        return eng_install(&raw mut o);
    }
    if strcmp(
        *argv.offset(1isize),
        b"clone\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        if from.is_null() || o.target.is_null() {
            usage();
            return 2 as ::core::ffi::c_int;
        }
        return eng_clone(from, o.target, o.quiet);
    }
    if r#gen.is_null() {
        usage();
        return 2 as ::core::ffi::c_int;
    }
    if strcmp(
        *argv.offset(1isize),
        b"verify\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        return eng_verify(r#gen, o.quiet);
    }
    let mut root: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut root as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/rootfs\0".as_ptr() as *const ::core::ffi::c_char,
        r#gen,
    );
    let mut g: *mut eng_guest = eng_guest_open(&raw mut root as *mut ::core::ffi::c_char);
    if !g.is_null() {
        let mut lk: ::core::ffi::c_int = eng_instance_lock(g, 1 as ::core::ffi::c_int);
        eng_guest_close(g);
        if lk == -EWOULDBLOCK {
            fprintf(
                stderr,
                b"remove: generation in use by a running engine\n\0".as_ptr()
                    as *const ::core::ffi::c_char,
            );
            return 3 as ::core::ffi::c_int;
        }
        if lk >= 0 as ::core::ffi::c_int {
            close(lk);
        }
    }
    let mut rc: ::core::ffi::c_int = eng_remove_tree(r#gen);
    if rc != 0 && rc != -ENOENT {
        fprintf(
            stderr,
            b"remove: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(-rc),
        );
        return 74 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if argc < 2 as ::core::ffi::c_int {
        usage();
        return 2 as ::core::ffi::c_int;
    }
    if strcmp(
        *argv.offset(1isize),
        b"probe\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        eng_log_init(0 as ::core::ffi::c_int);
        return cmd_probe();
    }
    if strcmp(
        *argv.offset(1isize),
        b"fsck\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        eng_log_init(0 as ::core::ffi::c_int);
        return cmd_fsck(argc, argv);
    }
    if strcmp(
        *argv.offset(1isize),
        b"install\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
        || strcmp(
            *argv.offset(1isize),
            b"clone\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        || strcmp(
            *argv.offset(1isize),
            b"verify\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        || strcmp(
            *argv.offset(1isize),
            b"remove\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
    {
        eng_log_init(0 as ::core::ffi::c_int);
        return cmd_lifecycle(argc, argv);
    }
    if strcmp(
        *argv.offset(1isize),
        b"run\0".as_ptr() as *const ::core::ffi::c_char,
    ) != 0
    {
        usage();
        return 2 as ::core::ffi::c_int;
    }
    let mut root: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut cwd: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut loader: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut binds: [*const ::core::ffi::c_char; 32] =
        [::core::ptr::null::<::core::ffi::c_char>(); 32];
    let mut hides: [*const ::core::ffi::c_char; 16] =
        [::core::ptr::null::<::core::ffi::c_char>(); 16];
    let mut nb: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut nh: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut verbosity: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut default_binds: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut no_filemap: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut seccomp: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut test_pagesz: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut test_app_filter: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut wait_all: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut binfmts: [*const ::core::ffi::c_char; 32] =
        [::core::ptr::null::<::core::ffi::c_char>(); 32];
    let mut nbf: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut no_guest_binfmt: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut sockdir: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut uid: ::core::ffi::c_long = 1000 as ::core::ffi::c_long;
    let mut gid: ::core::ffi::c_long = 1000 as ::core::ffi::c_long;
    let mut i: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    while i < argc {
        let mut a: *const ::core::ffi::c_char = *argv.offset(i as isize);
        if strcmp(a, b"--\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            i += 1;
            break;
        } else {
            if strcmp(a, b"--root\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                root = *argv.offset(i as isize);
            } else if strcmp(a, b"--cwd\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                cwd = *argv.offset(i as isize);
            } else if strcmp(a, b"--loader\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                loader = *argv.offset(i as isize);
            } else if strcmp(a, b"--bind\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
                && nb < ENG_MAX_BINDS
            {
                i += 1;
                let c2rust_fresh0 = nb;
                nb += 1;
                binds[c2rust_fresh0 as usize] = *argv.offset(i as isize);
            } else if strcmp(a, b"--hide\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
                && nh < ENG_MAX_HIDES
            {
                i += 1;
                let c2rust_fresh1 = nh;
                nh += 1;
                hides[c2rust_fresh1 as usize] = *argv.offset(i as isize);
            } else if strcmp(a, b"--uid\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                uid = strtol(
                    *argv.offset(i as isize),
                    ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                    10 as ::core::ffi::c_int,
                );
            } else if strcmp(a, b"--gid\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                gid = strtol(
                    *argv.offset(i as isize),
                    ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                    10 as ::core::ffi::c_int,
                );
            } else if strcmp(a, b"--user\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                let mut u: *const ::core::ffi::c_char = *argv.offset(i as isize);
                if strcmp(u, b"root\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                    gid = 0 as ::core::ffi::c_long;
                    uid = gid;
                } else if strcmp(u, b"work\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                    gid = 1000 as ::core::ffi::c_long;
                    uid = gid;
                } else {
                    fprintf(
                        stderr,
                        b"run: unknown user %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                        u,
                    );
                    return 2 as ::core::ffi::c_int;
                }
            } else if strcmp(
                a,
                b"--no-default-binds\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                default_binds = 0 as ::core::ffi::c_int;
            } else if strcmp(a, b"--binfmt\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
                && nbf < ENG_MAX_BINFMT
            {
                i += 1;
                let c2rust_fresh2 = nbf;
                nbf += 1;
                binfmts[c2rust_fresh2 as usize] = *argv.offset(i as isize);
            } else if strcmp(
                a,
                b"--no-guest-binfmt\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                no_guest_binfmt = 1 as ::core::ffi::c_int;
            } else if strcmp(a, b"--socket-dir\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                sockdir = *argv.offset(i as isize);
            } else if strcmp(a, b"--no-filemap\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                no_filemap = 1 as ::core::ffi::c_int;
            } else if strcmp(
                a,
                b"--test-page-size\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                test_pagesz = strtoul(
                    *argv.offset(i as isize),
                    ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                    0 as ::core::ffi::c_int,
                ) as ::core::ffi::c_uint;
            } else if strcmp(a, b"--seccomp\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                seccomp = 1 as ::core::ffi::c_int;
            } else if strcmp(a, b"--no-seccomp\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                seccomp = 0 as ::core::ffi::c_int;
            } else if strcmp(
                a,
                b"--test-app-filter\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                test_app_filter = 1 as ::core::ffi::c_int;
            } else if strcmp(a, b"--wait-all\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                wait_all = 1 as ::core::ffi::c_int;
            } else if strcmp(a, b"-v\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                verbosity += 1;
            } else if strcmp(a, b"-vv\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                verbosity += 2 as ::core::ffi::c_int;
            } else {
                fprintf(
                    stderr,
                    b"run: unknown option %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                    a,
                );
                usage();
                return 2 as ::core::ffi::c_int;
            }
            i += 1;
        }
    }
    if i >= argc {
        fprintf(
            stderr,
            b"run: missing command\n\0".as_ptr() as *const ::core::ffi::c_char,
        );
        return 2 as ::core::ffi::c_int;
    }
    eng_log_init(verbosity);
    let mut guest: *mut eng_guest = ::core::ptr::null_mut::<eng_guest>();
    let mut cmd: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut gargv: *mut *mut ::core::ffi::c_char = argv.offset(i as isize);
    if !root.is_null() {
        guest = eng_guest_open(root);
        if guest.is_null() {
            fprintf(
                stderr,
                b"run: cannot open guest root %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                root,
                strerror(*__errno()),
            );
            return 125 as ::core::ffi::c_int;
        }
        (*guest).no_filemap = no_filemap;
        (*guest).test_pagesz = test_pagesz;
        if !sockdir.is_null() {
            snprintf(
                &raw mut (*guest).sockdir as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                sockdir,
            );
        } else {
            let mut td: *const ::core::ffi::c_char =
                getenv(b"TMPDIR\0".as_ptr() as *const ::core::ffi::c_char);
            snprintf(
                &raw mut (*guest).sockdir as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s/wfs-%d\0".as_ptr() as *const ::core::ffi::c_char,
                if !td.is_null() && *td as ::core::ffi::c_int != 0 {
                    td
                } else {
                    b"/tmp\0".as_ptr() as *const ::core::ffi::c_char
                },
                getuid() as ::core::ffi::c_int,
            );
        }
        if !loader.is_null() {
            snprintf(
                &raw mut (*guest).loader as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                loader,
            );
        } else if default_loader(
            &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        ) != 0
        {
            fprintf(
                stderr,
                b"run: loader not found (set --loader or WORKFLOW_ENGINE_LOADER)\n\0".as_ptr()
                    as *const ::core::ffi::c_char,
            );
            return 125 as ::core::ffi::c_int;
        }
        let mut real: [::core::ffi::c_char; 4096] = [0; 4096];
        if realpath(
            &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            &raw mut real as *mut ::core::ffi::c_char,
        )
        .is_null()
            || access(&raw mut real as *mut ::core::ffi::c_char, X_OK) != 0 as ::core::ffi::c_int
        {
            fprintf(
                stderr,
                b"run: loader %s not executable\n\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            );
            return 125 as ::core::ffi::c_int;
        }
        snprintf(
            &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut real as *mut ::core::ffi::c_char,
        );
        if eng_instance_lock(guest, 0 as ::core::ffi::c_int) < 0 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"run: cannot register with %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                root,
                strerror(*__errno()),
            );
            return 125 as ::core::ffi::c_int;
        }
        if default_binds != 0 && eng_guest_default_binds(guest) != 0 {
            fprintf(
                stderr,
                b"run: default binds failed\n\0".as_ptr() as *const ::core::ffi::c_char,
            );
            return 125 as ::core::ffi::c_int;
        }
        let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while k < nb {
            let mut spec: [::core::ffi::c_char; 8192] = [0; 8192];
            snprintf(
                &raw mut spec as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 8192]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                binds[k as usize],
            );
            let mut colon: *mut ::core::ffi::c_char = strrchr(
                &raw mut spec as *mut ::core::ffi::c_char,
                ':' as ::core::ffi::c_int,
            );
            if colon.is_null() {
                fprintf(
                    stderr,
                    b"run: --bind needs HOST:GUEST\n\0".as_ptr() as *const ::core::ffi::c_char,
                );
                return 2 as ::core::ffi::c_int;
            }
            *colon = 0 as ::core::ffi::c_char;
            let mut rc: ::core::ffi::c_int = eng_guest_add_bind(
                guest,
                &raw mut spec as *mut ::core::ffi::c_char,
                colon.offset(1 as ::core::ffi::c_int as isize),
            );
            if rc != 0 {
                fprintf(
                    stderr,
                    b"run: bind %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                    binds[k as usize],
                    strerror(-rc),
                );
                return 125 as ::core::ffi::c_int;
            }
            let mut bst: stat = stat {
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
            eng_guest_make_mountpoint(
                guest,
                colon.offset(1 as ::core::ffi::c_int as isize),
                (stat(&raw mut spec as *mut ::core::ffi::c_char, &raw mut bst)
                    != 0 as ::core::ffi::c_int
                    || bst.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t)
                    as ::core::ffi::c_int,
            );
            k += 1;
        }
        let mut k_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while k_0 < nh {
            eng_guest_add_hide(guest, hides[k_0 as usize]);
            k_0 += 1;
        }
        if no_guest_binfmt == 0 {
            eng_guest_load_binfmt_dirs(guest);
        }
        let mut k_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while k_1 < nbf {
            let mut rc_0: ::core::ffi::c_int = if *binfmts[k_1 as usize].offset(0isize)
                as ::core::ffi::c_int
                == '@' as ::core::ffi::c_int
            {
                eng_guest_load_binfmt_file(
                    guest,
                    binfmts[k_1 as usize].offset(1 as ::core::ffi::c_int as isize),
                )
            } else {
                eng_guest_add_binfmt(guest, binfmts[k_1 as usize])
            };
            if rc_0 < 0 as ::core::ffi::c_int {
                fprintf(
                    stderr,
                    b"run: bad --binfmt %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                    binfmts[k_1 as usize],
                );
                return 2 as ::core::ffi::c_int;
            }
            k_1 += 1;
        }
        if strchr(*gargv.offset(0isize), '/' as ::core::ffi::c_int).is_null() {
            if guest_path_search(
                guest,
                *gargv.offset(0isize),
                environ,
                &raw mut cmd as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            ) != 0
            {
                fprintf(
                    stderr,
                    b"workflow-engine: %s: command not found\n\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    *gargv.offset(0isize),
                );
                return 127 as ::core::ffi::c_int;
            }
        } else if *(*gargv.offset(0isize)).offset(0isize) as ::core::ffi::c_int
            != '/' as ::core::ffi::c_int
        {
            snprintf(
                &raw mut cmd as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
                if !cwd.is_null() {
                    cwd
                } else {
                    b"\0".as_ptr() as *const ::core::ffi::c_char
                },
                *gargv.offset(0isize),
            );
        } else {
            snprintf(
                &raw mut cmd as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                *gargv.offset(0isize),
            );
        }
    }
    let mut se: *const ::core::ffi::c_char =
        getenv(b"WORKFLOW_ENGINE_SECCOMP\0".as_ptr() as *const ::core::ffi::c_char);
    if !se.is_null() && *se as ::core::ffi::c_int == '0' as ::core::ffi::c_int {
        seccomp = 0 as ::core::ffi::c_int;
    }
    static mut genv: [*mut ::core::ffi::c_char; 4096] =
        [::core::ptr::null_mut::<::core::ffi::c_char>(); 4096];
    let mut ne: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut e: *mut *mut ::core::ffi::c_char = environ;
    while !(*e).is_null() && ne < 4095 as ::core::ffi::c_int {
        if strncmp(
            *e,
            b"WORKFLOW_ENGINE_\0".as_ptr() as *const ::core::ffi::c_char,
            16 as size_t,
        ) != 0
        {
            let c2rust_fresh3 = ne;
            ne += 1;
            genv[c2rust_fresh3 as usize] = *e;
        }
        e = e.offset(1);
    }
    genv[ne as usize] = ::core::ptr::null_mut::<::core::ffi::c_char>();
    if !guest.is_null() && eng_log_enabled(eng_log_level::ENG_LOG_DEBUG) != 0 {
        let mut k_2: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while k_2 < (*guest).nbinds {
            eng_logf!(
                eng_log_level::ENG_LOG_DEBUG,
                b"bind %s -> %s (owner %u:%u)\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*(&raw mut (*guest).binds as *mut eng_bind).offset(k_2 as isize)).guest
                    as *mut ::core::ffi::c_char,
                &raw mut (*(&raw mut (*guest).binds as *mut eng_bind).offset(k_2 as isize)).host
                    as *mut ::core::ffi::c_char,
                (*guest).binds[k_2 as usize].uid,
                (*guest).binds[k_2 as usize].gid,
            );
            k_2 += 1;
        }
    }
    let mut cfg: eng_run_cfg = eng_run_cfg {
        argv: gargv,
        exe: if !root.is_null() {
            &raw mut cmd as *mut ::core::ffi::c_char
        } else {
            ::core::ptr::null_mut::<::core::ffi::c_char>()
        },
        envp: if !root.is_null() {
            &raw mut genv as *mut *mut ::core::ffi::c_char
        } else {
            environ
        },
        cwd: cwd,
        guest: guest,
        use_seccomp_fastpath: seccomp,
        uid: uid as uint32_t,
        gid: gid as uint32_t,
        test_app_filter: test_app_filter,
        wait_all: wait_all,
    };
    let mut rc_1: ::core::ffi::c_int = eng_tracer_run(&raw mut cfg);
    return if rc_1 < 0 as ::core::ffi::c_int {
        125 as ::core::ffi::c_int
    } else {
        rc_1
    };
}
unsafe extern "C" fn cmd_probe() -> ::core::ffi::c_int {
    printf(
        b"{\"arch\":\"%s\",\"pageSize\":%ld}\n\0".as_ptr() as *const ::core::ffi::c_char,
        ENG_ARCH_NAME.as_ptr(),
        sysconf(_SC_PAGESIZE),
    );
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args_os()
        .map(|arg| {
            ::std::ffi::CString::new(std::os::unix::ffi::OsStrExt::as_bytes(arg.as_os_str()))
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut ::core::ffi::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .chain(::core::iter::once(::core::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as ::core::ffi::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut ::core::ffi::c_char,
        ) as i32)
    }
}
