//! Component-by-component guest path resolution, including proc magic links.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
#[repr(C)]
pub struct eng_tracer {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    unsafe fn atoi(__s: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    unsafe fn eng_task_find(tr: *mut eng_tracer, tid: pid_t) -> *mut eng_task;
    unsafe fn eng_tracer_guest(tr: *mut eng_tracer) -> *mut eng_guest;
    unsafe fn __errno() -> *mut ::core::ffi::c_int;
    unsafe fn snprintf(
        __buf: *mut ::core::ffi::c_char,
        __size: size_t,
        __fmt: *const ::core::ffi::c_char,
        ...
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
    unsafe fn strcpy(
        __dst: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strncmp(
        __lhs: *const ::core::ffi::c_char,
        __rhs: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn lstat(__path: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn stat(__path: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn readlink(
        __path: *const ::core::ffi::c_char,
        __buf: *mut ::core::ffi::c_char,
        __buf_size: size_t,
    ) -> ssize_t;
    unsafe fn eng_guest_to_host(
        g: *const eng_guest,
        guest: *const ::core::ffi::c_char,
        host: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_host_to_guest(
        g: *const eng_guest,
        host: *const ::core::ffi::c_char,
        guest: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_guest_is_hidden(
        g: *const eng_guest,
        guest: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_meta_get(
        g: *mut eng_guest,
        host: *const ::core::ffi::c_char,
        nofollow: ::core::ffi::c_int,
        st: *const stat,
        m: *mut eng_meta,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_meta_permission(
        t: *mut eng_task,
        m: *const eng_meta,
        mask: ::core::ffi::c_int,
        use_real: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_link_is_stub_text(
        text: *const ::core::ffi::c_char,
        id: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_link_object_path(
        g: *mut eng_guest,
        id: *const ::core::ffi::c_char,
        out: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type int64_t = i64;
pub type uint64_t = u64;
pub type __kernel_long_t = ::core::ffi::c_long;
pub type __kernel_ulong_t = ::core::ffi::c_ulong;
pub type __kernel_ino_t = __kernel_ulong_t;
pub type __kernel_mode_t = ::core::ffi::c_uint;
pub type __kernel_pid_t = ::core::ffi::c_int;
pub type __kernel_uid32_t = ::core::ffi::c_uint;
pub type __kernel_gid32_t = ::core::ffi::c_uint;
pub type __kernel_time_t = __kernel_long_t;
pub type __gid_t = __kernel_gid32_t;
pub type gid_t = __gid_t;
pub type __uid_t = __kernel_uid32_t;
pub type uid_t = __uid_t;
pub type __pid_t = __kernel_pid_t;
pub type pid_t = __pid_t;
pub type __mode_t = __kernel_mode_t;
pub type mode_t = __mode_t;
pub type __ino_t = __kernel_ino_t;
pub type ino_t = __ino_t;
pub type dev_t = uint64_t;
pub type __time_t = __kernel_time_t;
pub type time_t = __time_t;
pub type off_t = int64_t;
pub type ssize_t = isize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: time_t,
    pub tv_nsec: ::core::ffi::c_long,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct user_regs_struct {
    pub r15: ::core::ffi::c_ulong,
    pub r14: ::core::ffi::c_ulong,
    pub r13: ::core::ffi::c_ulong,
    pub r12: ::core::ffi::c_ulong,
    pub rbp: ::core::ffi::c_ulong,
    pub rbx: ::core::ffi::c_ulong,
    pub r11: ::core::ffi::c_ulong,
    pub r10: ::core::ffi::c_ulong,
    pub r9: ::core::ffi::c_ulong,
    pub r8: ::core::ffi::c_ulong,
    pub rax: ::core::ffi::c_ulong,
    pub rcx: ::core::ffi::c_ulong,
    pub rdx: ::core::ffi::c_ulong,
    pub rsi: ::core::ffi::c_ulong,
    pub rdi: ::core::ffi::c_ulong,
    pub orig_rax: ::core::ffi::c_ulong,
    pub rip: ::core::ffi::c_ulong,
    pub cs: ::core::ffi::c_ulong,
    pub eflags: ::core::ffi::c_ulong,
    pub rsp: ::core::ffi::c_ulong,
    pub ss: ::core::ffi::c_ulong,
    pub fs_base: ::core::ffi::c_ulong,
    pub gs_base: ::core::ffi::c_ulong,
    pub ds: ::core::ffi::c_ulong,
    pub es: ::core::ffi::c_ulong,
    pub fs: ::core::ffi::c_ulong,
    pub gs: ::core::ffi::c_ulong,
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
pub struct eng_resolved {
    pub guest: [::core::ffi::c_char; 4096],
    pub host: [::core::ffi::c_char; 4096],
    pub verbatim: ::core::ffi::c_int,
    pub magic: ::core::ffi::c_int,
    pub magic_text: [::core::ffi::c_char; 4096],
    pub exists: ::core::ffi::c_int,
    pub stub: ::core::ffi::c_int,
    pub stub_id: [::core::ffi::c_char; 64],
    pub entry: [::core::ffi::c_char; 4096],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: dev_t,
    pub st_ino: ino_t,
    pub st_nlink: ::core::ffi::c_ulong,
    pub st_mode: mode_t,
    pub st_uid: uid_t,
    pub st_gid: gid_t,
    pub __pad0: ::core::ffi::c_uint,
    pub st_rdev: dev_t,
    pub st_size: off_t,
    pub st_blksize: ::core::ffi::c_long,
    pub st_blocks: ::core::ffi::c_long,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __pad3: [::core::ffi::c_long; 3],
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
pub const NAME_MAX: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const PATH_MAX: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
pub const ENG_RES_FOLLOW: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const ENG_RES_MISSING_OK: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const ENG_RES_DIR_ONLY: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const ENOENT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const EBADF: ::core::ffi::c_int = 9 as ::core::ffi::c_int;
pub const EACCES: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const ENOTDIR: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const ENAMETOOLONG: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const ELOOP: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const AT_FDCWD: ::core::ffi::c_int = -100 as ::core::ffi::c_int;
pub const S_IFMT: ::core::ffi::c_int = 0o170000 as ::core::ffi::c_int;
pub const S_IFLNK: ::core::ffi::c_int = 0o120000 as ::core::ffi::c_int;
pub const S_IFDIR: ::core::ffi::c_int = 0o40000 as ::core::ffi::c_int;
pub const X_OK: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ENG_CAP_DAC_OVERRIDE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ENG_CAP_DAC_READ_SEARCH: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const MAX_LINKS: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
unsafe extern "C" fn read_host_link(
    mut hostpath: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut n: ssize_t = readlink(hostpath, out, cap.wrapping_sub(1 as size_t));
    if n < 0 as ssize_t {
        return -*__errno();
    }
    *out.offset(n) = 0 as ::core::ffi::c_char;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn link_text_to_guest(
    mut g: *const eng_guest,
    mut text: *mut ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut n: size_t = strlen(text);
    static mut del: [::core::ffi::c_char; 11] =
        unsafe { ::core::mem::transmute::<[u8; 11], [::core::ffi::c_char; 11]>(*b" (deleted)\0") };
    if n > ::core::mem::size_of::<[::core::ffi::c_char; 11]>().wrapping_sub(1usize)
        && strcmp(
            text.offset(n as isize).offset(
                -(::core::mem::size_of::<[::core::ffi::c_char; 11]>().wrapping_sub(1usize)
                    as isize),
            ),
            &raw const del as *const ::core::ffi::c_char,
        ) == 0
    {
        *text.offset(n.wrapping_sub(
            ::core::mem::size_of::<[::core::ffi::c_char; 11]>().wrapping_sub(1 as size_t),
        ) as isize) = 0 as ::core::ffi::c_char;
    }
    if *text.offset(0isize) as ::core::ffi::c_int != '/' as ::core::ffi::c_int {
        snprintf(
            out,
            cap,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            text,
        );
        return -ENOENT;
    }
    if eng_host_to_guest(g, text, out, cap) != 0 as ::core::ffi::c_int {
        snprintf(
            out,
            cap,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            text,
        );
        return -ENOENT;
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_task_cwd(
    mut t: *mut eng_task,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut p: [::core::ffi::c_char; 64] = [0; 64];
    let mut text: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
        b"/proc/%d/cwd\0".as_ptr() as *const ::core::ffi::c_char,
        (*t).tid,
    );
    let mut rc: ::core::ffi::c_int = read_host_link(
        &raw mut p as *mut ::core::ffi::c_char,
        &raw mut text as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    if rc != 0 {
        return rc;
    }
    return link_text_to_guest(
        eng_tracer_guest((*t).tr),
        &raw mut text as *mut ::core::ffi::c_char,
        out,
        cap,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_task_fd_path(
    mut t: *mut eng_task,
    mut pid: pid_t,
    mut fd: ::core::ffi::c_int,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut p: [::core::ffi::c_char; 64] = [0; 64];
    let mut text: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
        b"/proc/%d/fd/%d\0".as_ptr() as *const ::core::ffi::c_char,
        if pid != 0 { pid } else { (*t).tid },
        fd,
    );
    let mut rc: ::core::ffi::c_int = read_host_link(
        &raw mut p as *mut ::core::ffi::c_char,
        &raw mut text as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    if rc != 0 {
        return if rc == -ENOENT { -EBADF } else { rc };
    }
    return link_text_to_guest(
        eng_tracer_guest((*t).tr),
        &raw mut text as *mut ::core::ffi::c_char,
        out,
        cap,
    );
}
unsafe extern "C" fn may_search(
    mut t: *mut eng_task,
    mut guest_dir: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if (*t).cr.cap_eff as ::core::ffi::c_ulonglong
        & ((1 as ::core::ffi::c_ulonglong) << ENG_CAP_DAC_OVERRIDE
            | (1 as ::core::ffi::c_ulonglong) << ENG_CAP_DAC_READ_SEARCH)
        != 0
    {
        return 0 as ::core::ffi::c_int;
    }
    let mut g: *const eng_guest = eng_tracer_guest((*t).tr);
    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
    if eng_guest_to_host(
        g,
        guest_dir,
        &raw mut host as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return 0 as ::core::ffi::c_int;
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
        __pad3: [0; 3],
    };
    if stat(&raw mut host as *mut ::core::ffi::c_char, &raw mut st) != 0 as ::core::ffi::c_int
        || !(st.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t)
    {
        return 0 as ::core::ffi::c_int;
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
        g as *mut eng_guest,
        &raw mut host as *mut ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
        &raw mut st,
        &raw mut m,
    );
    return eng_meta_permission(t, &raw mut m, X_OK, 0 as ::core::ffi::c_int);
}
unsafe extern "C" fn is_num(
    mut s: *const ::core::ffi::c_char,
    mut n: size_t,
) -> ::core::ffi::c_int {
    if n == 0 {
        return 0 as ::core::ffi::c_int;
    }
    let mut i: size_t = 0 as size_t;
    while i < n {
        if (*s.offset(i as isize) as ::core::ffi::c_int) < '0' as ::core::ffi::c_int
            || *s.offset(i as isize) as ::core::ffi::c_int > '9' as ::core::ffi::c_int
        {
            return 0 as ::core::ffi::c_int;
        }
        i = i.wrapping_add(1);
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn proc_magic(
    mut cand: *const ::core::ffi::c_char,
    mut kind: *mut ::core::ffi::c_char,
    mut pid: *mut pid_t,
    mut fd: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if strncmp(
        cand,
        b"/proc/\0".as_ptr() as *const ::core::ffi::c_char,
        6 as size_t,
    ) != 0
    {
        return 0 as ::core::ffi::c_int;
    }
    let mut p: *const ::core::ffi::c_char = cand.offset(6 as ::core::ffi::c_int as isize);
    let mut s: *const ::core::ffi::c_char = strchr(p, '/' as ::core::ffi::c_int);
    if s.is_null() || is_num(p, s.offset_from(p) as size_t) == 0 {
        return 0 as ::core::ffi::c_int;
    }
    *pid = atoi(p);
    p = s.offset(1 as ::core::ffi::c_int as isize);
    if strncmp(
        p,
        b"task/\0".as_ptr() as *const ::core::ffi::c_char,
        5 as size_t,
    ) == 0
    {
        let mut q: *const ::core::ffi::c_char = p.offset(5 as ::core::ffi::c_int as isize);
        let mut e: *const ::core::ffi::c_char = strchr(q, '/' as ::core::ffi::c_int);
        if e.is_null() || is_num(q, e.offset_from(q) as size_t) == 0 {
            return 0 as ::core::ffi::c_int;
        }
        *pid = atoi(q);
        p = e.offset(1 as ::core::ffi::c_int as isize);
    }
    if strcmp(p, b"cwd\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
        *kind = 'c' as ::core::ffi::c_char;
        return 1 as ::core::ffi::c_int;
    }
    if strcmp(p, b"root\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
        *kind = 'r' as ::core::ffi::c_char;
        return 1 as ::core::ffi::c_int;
    }
    if strcmp(p, b"exe\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
        *kind = 'e' as ::core::ffi::c_char;
        return 1 as ::core::ffi::c_int;
    }
    if strncmp(
        p,
        b"fd/\0".as_ptr() as *const ::core::ffi::c_char,
        3 as size_t,
    ) == 0
        && is_num(
            p.offset(3 as ::core::ffi::c_int as isize),
            strlen(p.offset(3 as ::core::ffi::c_int as isize)),
        ) != 0
    {
        *kind = 'f' as ::core::ffi::c_char;
        *fd = atoi(p.offset(3 as ::core::ffi::c_int as isize));
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn magic_target(
    mut t: *mut eng_task,
    mut kind: ::core::ffi::c_char,
    mut pid: pid_t,
    mut fd: ::core::ffi::c_int,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut g: *const eng_guest = eng_tracer_guest((*t).tr);
    let mut p: [::core::ffi::c_char; 64] = [0; 64];
    let mut text: [::core::ffi::c_char; 4096] = [0; 4096];
    match kind as ::core::ffi::c_int {
        114 => {
            snprintf(out, cap, b"/\0".as_ptr() as *const ::core::ffi::c_char);
            return 0 as ::core::ffi::c_int;
        }
        101 => {
            let mut o: *mut eng_task = eng_task_find((*t).tr, pid);
            if !o.is_null()
                && !(*o).proc.is_null()
                && (*(*o).proc).exe[0usize] as ::core::ffi::c_int != 0
            {
                snprintf(
                    out,
                    cap,
                    b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                    &raw mut (*(*o).proc).exe as *mut ::core::ffi::c_char,
                );
                return 0 as ::core::ffi::c_int;
            }
            snprintf(
                &raw mut p as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                b"/proc/%d/exe\0".as_ptr() as *const ::core::ffi::c_char,
                pid,
            );
        }
        99 => {
            snprintf(
                &raw mut p as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                b"/proc/%d/cwd\0".as_ptr() as *const ::core::ffi::c_char,
                pid,
            );
        }
        _ => {
            snprintf(
                &raw mut p as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                b"/proc/%d/fd/%d\0".as_ptr() as *const ::core::ffi::c_char,
                pid,
                fd,
            );
        }
    }
    let mut rc: ::core::ffi::c_int = read_host_link(
        &raw mut p as *mut ::core::ffi::c_char,
        &raw mut text as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    );
    if rc != 0 {
        return rc;
    }
    return link_text_to_guest(g, &raw mut text as *mut ::core::ffi::c_char, out, cap);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_resolve(
    mut t: *mut eng_task,
    mut dirfd: ::core::ffi::c_int,
    mut path: *const ::core::ffi::c_char,
    mut flags: ::core::ffi::c_int,
    mut out: *mut eng_resolved,
) -> ::core::ffi::c_int {
    let mut g: *const eng_guest = eng_tracer_guest((*t).tr);
    memset(
        out as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        8200 as size_t,
    );
    (*out).magic_text[0usize] = 0 as ::core::ffi::c_char;
    // The initial cwd/root already exists. Dot-only paths visit no component.
    (*out).exists = 1;
    (*out).stub = 0 as ::core::ffi::c_int;
    (*out).stub_id[0usize] = 0 as ::core::ffi::c_char;
    (*out).entry[0usize] = 0 as ::core::ffi::c_char;
    if *path.offset(0isize) == 0 {
        return -ENOENT;
    }
    let mut plen: size_t = strlen(path);
    if plen >= PATH_MAX as size_t {
        return -ENAMETOOLONG;
    }
    let mut cur: [::core::ffi::c_char; 4096] = [0; 4096];
    if *path.offset(0isize) as ::core::ffi::c_int == '/' as ::core::ffi::c_int {
        strcpy(
            &raw mut cur as *mut ::core::ffi::c_char,
            b"/\0".as_ptr() as *const ::core::ffi::c_char,
        );
    } else {
        let mut rc: ::core::ffi::c_int = if dirfd == AT_FDCWD {
            eng_task_cwd(
                t,
                &raw mut cur as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            )
        } else {
            eng_task_fd_path(
                t,
                0 as pid_t,
                dirfd,
                &raw mut cur as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            )
        };
        if rc == -ENOENT && dirfd != AT_FDCWD {
            let mut n: ::core::ffi::c_int = snprintf(
                &raw mut (*out).host as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"/proc/%d/fd/%d/%s\0".as_ptr() as *const ::core::ffi::c_char,
                (*t).tid,
                dirfd,
                path,
            );
            if n <= 0 as ::core::ffi::c_int
                || n as size_t >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
            {
                return -ENAMETOOLONG;
            }
            snprintf(
                &raw mut (*out).guest as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                path,
            );
            snprintf(
                &raw mut (*out).entry as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*out).host as *mut ::core::ffi::c_char,
            );
            (*out).verbatim = 1 as ::core::ffi::c_int;
            return 0 as ::core::ffi::c_int;
        }
        if rc != 0 {
            return rc;
        }
    }
    let mut trailing: ::core::ffi::c_int = (plen > 0 as size_t
        && *path.offset(plen.wrapping_sub(1 as size_t) as isize) as ::core::ffi::c_int
            == '/' as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    let mut todo: [::core::ffi::c_char; 8194] = [0; 8194];
    snprintf(
        &raw mut todo as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        path,
    );
    let mut pos: size_t = 0 as size_t;
    let mut links: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    '_done: {
        loop {
            while todo[pos] as ::core::ffi::c_int == '/' as ::core::ffi::c_int {
                pos = pos.wrapping_add(1);
            }
            if todo[pos] == 0 {
                break;
            }
            let mut start: size_t = pos;
            while todo[pos] as ::core::ffi::c_int != 0
                && todo[pos] as ::core::ffi::c_int != '/' as ::core::ffi::c_int
            {
                pos = pos.wrapping_add(1);
            }
            let mut clen: size_t = pos.wrapping_sub(start);
            let mut after: size_t = pos;
            while todo[after] as ::core::ffi::c_int == '/' as ::core::ffi::c_int {
                after = after.wrapping_add(1);
            }
            let mut last: ::core::ffi::c_int = (todo[after] as ::core::ffi::c_int
                == 0 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            if clen >= (NAME_MAX + 1 as ::core::ffi::c_int) as size_t {
                return -ENAMETOOLONG;
            }
            let mut comp: [::core::ffi::c_char; 256] = [0; 256];
            memcpy(
                &raw mut comp as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                (&raw mut todo as *mut ::core::ffi::c_char).offset(start as isize)
                    as *const ::core::ffi::c_void,
                clen,
            );
            comp[clen] = 0 as ::core::ffi::c_char;
            if strcmp(
                &raw mut comp as *mut ::core::ffi::c_char,
                b".\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                continue;
            }
            if strncmp(
                &raw mut cur as *mut ::core::ffi::c_char,
                b"/proc\0".as_ptr() as *const ::core::ffi::c_char,
                5 as size_t,
            ) != 0
                && strncmp(
                    &raw mut cur as *mut ::core::ffi::c_char,
                    b"/sys\0".as_ptr() as *const ::core::ffi::c_char,
                    4 as size_t,
                ) != 0
            {
                let mut prc: ::core::ffi::c_int =
                    may_search(t, &raw mut cur as *mut ::core::ffi::c_char);
                if prc != 0 {
                    return prc;
                }
            }
            if strcmp(
                &raw mut comp as *mut ::core::ffi::c_char,
                b"..\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                let mut sl: *mut ::core::ffi::c_char = strrchr(
                    &raw mut cur as *mut ::core::ffi::c_char,
                    '/' as ::core::ffi::c_int,
                );
                if !sl.is_null() && sl != &raw mut cur as *mut ::core::ffi::c_char {
                    *sl = 0 as ::core::ffi::c_char;
                } else {
                    strcpy(
                        &raw mut cur as *mut ::core::ffi::c_char,
                        b"/\0".as_ptr() as *const ::core::ffi::c_char,
                    );
                }
            } else if strcmp(
                &raw mut cur as *mut ::core::ffi::c_char,
                b"/proc\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
                && (strcmp(
                    &raw mut comp as *mut ::core::ffi::c_char,
                    b"self\0".as_ptr() as *const ::core::ffi::c_char,
                ) == 0
                    || strcmp(
                        &raw mut comp as *mut ::core::ffi::c_char,
                        b"thread-self\0".as_ptr() as *const ::core::ffi::c_char,
                    ) == 0)
            {
                let mut tgt: [::core::ffi::c_char; 64] = [0; 64];
                if comp[0usize] as ::core::ffi::c_int == 's' as ::core::ffi::c_int {
                    snprintf(
                        &raw mut tgt as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                        b"%d\0".as_ptr() as *const ::core::ffi::c_char,
                        (*t).tgid,
                    );
                } else {
                    snprintf(
                        &raw mut tgt as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                        b"%d/task/%d\0".as_ptr() as *const ::core::ffi::c_char,
                        (*t).tgid,
                        (*t).tid,
                    );
                }
                let mut rest: [::core::ffi::c_char; 8194] = [0; 8194];
                snprintf(
                    &raw mut rest as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
                    b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                    &raw mut tgt as *mut ::core::ffi::c_char,
                    (&raw mut todo as *mut ::core::ffi::c_char).offset(pos as isize),
                );
                snprintf(
                    &raw mut todo as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
                    b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                    &raw mut rest as *mut ::core::ffi::c_char,
                );
                pos = 0 as size_t;
            } else {
                let mut cand: [::core::ffi::c_char; 4096] = [0; 4096];
                let mut n_0: ::core::ffi::c_int = snprintf(
                    &raw mut cand as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                    b"%s%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                    &raw mut cur as *mut ::core::ffi::c_char,
                    if strcmp(
                        &raw mut cur as *mut ::core::ffi::c_char,
                        b"/\0".as_ptr() as *const ::core::ffi::c_char,
                    ) != 0
                    {
                        b"/\0".as_ptr() as *const ::core::ffi::c_char
                    } else {
                        b"\0".as_ptr() as *const ::core::ffi::c_char
                    },
                    &raw mut comp as *mut ::core::ffi::c_char,
                );
                if n_0 <= 0 as ::core::ffi::c_int
                    || n_0 as size_t >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
                {
                    return -ENAMETOOLONG;
                }
                if eng_guest_is_hidden(g, &raw mut cand as *mut ::core::ffi::c_char) != 0 {
                    if last != 0 && flags & ENG_RES_MISSING_OK != 0 {
                        return -EACCES;
                    }
                    return -ENOENT;
                }
                let mut follow_here: ::core::ffi::c_int =
                    (last == 0 || flags & ENG_RES_FOLLOW != 0 || trailing != 0)
                        as ::core::ffi::c_int;
                let mut kind: ::core::ffi::c_char = 0;
                let mut mpid: pid_t = 0;
                let mut mfd: ::core::ffi::c_int = -1 as ::core::ffi::c_int;
                if proc_magic(
                    &raw mut cand as *mut ::core::ffi::c_char,
                    &raw mut kind,
                    &raw mut mpid,
                    &raw mut mfd,
                ) != 0
                {
                    let mut tgt_0: [::core::ffi::c_char; 4096] = [0; 4096];
                    let mut rc_0: ::core::ffi::c_int = magic_target(
                        t,
                        kind,
                        mpid,
                        mfd,
                        &raw mut tgt_0 as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                    );
                    if rc_0 != 0 && rc_0 != -ENOENT {
                        return rc_0;
                    }
                    if follow_here == 0 {
                        snprintf(
                            &raw mut (*out).magic_text as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                            &raw mut tgt_0 as *mut ::core::ffi::c_char,
                        );
                        (*out).magic = 1 as ::core::ffi::c_int;
                        snprintf(
                            &raw mut (*out).guest as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                            &raw mut cand as *mut ::core::ffi::c_char,
                        );
                        eng_guest_to_host(
                            g,
                            &raw mut cand as *mut ::core::ffi::c_char,
                            &raw mut (*out).host as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                        );
                        snprintf(
                            &raw mut (*out).entry as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                            &raw mut (*out).host as *mut ::core::ffi::c_char,
                        );
                        (*out).exists = 1 as ::core::ffi::c_int;
                        return 0 as ::core::ffi::c_int;
                    }
                    if rc_0 == -ENOENT {
                        let mut h: [::core::ffi::c_char; 4096] = [0; 4096];
                        eng_guest_to_host(
                            g,
                            &raw mut cand as *mut ::core::ffi::c_char,
                            &raw mut h as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                        );
                        let mut m: ::core::ffi::c_int = snprintf(
                            &raw mut (*out).host as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                            &raw mut h as *mut ::core::ffi::c_char,
                            (&raw mut todo as *mut ::core::ffi::c_char).offset(pos as isize),
                        );
                        if m <= 0 as ::core::ffi::c_int
                            || m as size_t >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
                        {
                            return -ENAMETOOLONG;
                        }
                        snprintf(
                            &raw mut (*out).guest as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                            &raw mut cand as *mut ::core::ffi::c_char,
                        );
                        snprintf(
                            &raw mut (*out).entry as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                            &raw mut (*out).host as *mut ::core::ffi::c_char,
                        );
                        (*out).verbatim = 1 as ::core::ffi::c_int;
                        (*out).exists = 1 as ::core::ffi::c_int;
                        return 0 as ::core::ffi::c_int;
                    }
                    links += 1;
                    if links > MAX_LINKS {
                        return -ELOOP;
                    }
                    let mut rest_0: [::core::ffi::c_char; 8194] = [0; 8194];
                    let mut m_0: ::core::ffi::c_int = snprintf(
                        &raw mut rest_0 as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
                        b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut tgt_0 as *mut ::core::ffi::c_char,
                        (&raw mut todo as *mut ::core::ffi::c_char).offset(pos as isize),
                    );
                    if m_0 <= 0 as ::core::ffi::c_int
                        || m_0 as size_t >= ::core::mem::size_of::<[::core::ffi::c_char; 8194]>()
                    {
                        return -ENAMETOOLONG;
                    }
                    snprintf(
                        &raw mut todo as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut rest_0 as *mut ::core::ffi::c_char,
                    );
                    pos = 0 as size_t;
                    strcpy(
                        &raw mut cur as *mut ::core::ffi::c_char,
                        b"/\0".as_ptr() as *const ::core::ffi::c_char,
                    );
                } else {
                    let mut host: [::core::ffi::c_char; 4096] = [0; 4096];
                    if eng_guest_to_host(
                        g,
                        &raw mut cand as *mut ::core::ffi::c_char,
                        &raw mut host as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                    ) != 0
                    {
                        return -ENAMETOOLONG;
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
                        __pad3: [0; 3],
                    };
                    if lstat(&raw mut host as *mut ::core::ffi::c_char, &raw mut st)
                        != 0 as ::core::ffi::c_int
                    {
                        let mut e: ::core::ffi::c_int = *__errno();
                        if e == ENOENT && last != 0 && flags & ENG_RES_MISSING_OK != 0 {
                            snprintf(
                                &raw mut cur as *mut ::core::ffi::c_char,
                                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                                &raw mut cand as *mut ::core::ffi::c_char,
                            );
                            (*out).exists = 0 as ::core::ffi::c_int;
                            break '_done;
                        } else {
                            return -e;
                        }
                    } else {
                        if st.st_mode & S_IFMT as mode_t == S_IFLNK as mode_t {
                            let mut tgt_1: [::core::ffi::c_char; 4096] = [0; 4096];
                            let mut rc_1: ::core::ffi::c_int = read_host_link(
                                &raw mut host as *mut ::core::ffi::c_char,
                                &raw mut tgt_1 as *mut ::core::ffi::c_char,
                                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            );
                            if rc_1 != 0 {
                                return rc_1;
                            }
                            let mut id: [::core::ffi::c_char; 64] = [0; 64];
                            if eng_link_is_stub_text(
                                &raw mut tgt_1 as *mut ::core::ffi::c_char,
                                &raw mut id as *mut ::core::ffi::c_char,
                                ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                            ) != 0
                            {
                                if last == 0 {
                                    return -ENOTDIR;
                                }
                                snprintf(
                                    &raw mut (*out).guest as *mut ::core::ffi::c_char,
                                    ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                                    b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                                    &raw mut cand as *mut ::core::ffi::c_char,
                                );
                                snprintf(
                                    &raw mut (*out).entry as *mut ::core::ffi::c_char,
                                    ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                                    b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                                    &raw mut host as *mut ::core::ffi::c_char,
                                );
                                if eng_link_object_path(
                                    g as *mut eng_guest,
                                    &raw mut id as *mut ::core::ffi::c_char,
                                    &raw mut (*out).host as *mut ::core::ffi::c_char,
                                    ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                                ) != 0
                                {
                                    return -ENAMETOOLONG;
                                }
                                snprintf(
                                    &raw mut (*out).stub_id as *mut ::core::ffi::c_char,
                                    ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
                                    b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                                    &raw mut id as *mut ::core::ffi::c_char,
                                );
                                (*out).stub = 1 as ::core::ffi::c_int;
                                (*out).exists = 1 as ::core::ffi::c_int;
                                if trailing != 0 {
                                    return -ENOTDIR;
                                }
                                return 0 as ::core::ffi::c_int;
                            }
                            if follow_here != 0 {
                                links += 1;
                                if links > MAX_LINKS {
                                    return -ELOOP;
                                }
                                if tgt_1[0usize] == 0 {
                                    return -ENOENT;
                                }
                                let mut rest_1: [::core::ffi::c_char; 8194] = [0; 8194];
                                let mut m_1: ::core::ffi::c_int = snprintf(
                                    &raw mut rest_1 as *mut ::core::ffi::c_char,
                                    ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
                                    b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                                    &raw mut tgt_1 as *mut ::core::ffi::c_char,
                                    (&raw mut todo as *mut ::core::ffi::c_char)
                                        .offset(pos as isize),
                                );
                                if m_1 <= 0 as ::core::ffi::c_int
                                    || m_1 as size_t
                                        >= ::core::mem::size_of::<[::core::ffi::c_char; 8194]>()
                                {
                                    return -ENAMETOOLONG;
                                }
                                snprintf(
                                    &raw mut todo as *mut ::core::ffi::c_char,
                                    ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
                                    b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                                    &raw mut rest_1 as *mut ::core::ffi::c_char,
                                );
                                pos = 0 as size_t;
                                if tgt_1[0usize] as ::core::ffi::c_int == '/' as ::core::ffi::c_int
                                {
                                    strcpy(
                                        &raw mut cur as *mut ::core::ffi::c_char,
                                        b"/\0".as_ptr() as *const ::core::ffi::c_char,
                                    );
                                }
                                continue;
                            }
                        }
                        if last == 0 && !(st.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t) {
                            return -ENOTDIR;
                        }
                        snprintf(
                            &raw mut cur as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                            &raw mut cand as *mut ::core::ffi::c_char,
                        );
                        (*out).exists = 1 as ::core::ffi::c_int;
                        if last != 0 {
                            break;
                        }
                    }
                }
            }
        }
        if strcmp(
            &raw mut cur as *mut ::core::ffi::c_char,
            b"/\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        {
            (*out).exists = 1 as ::core::ffi::c_int;
        }
    }
    snprintf(
        &raw mut (*out).guest as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut cur as *mut ::core::ffi::c_char,
    );
    if eng_guest_to_host(
        g,
        &raw mut cur as *mut ::core::ffi::c_char,
        &raw mut (*out).host as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
    ) != 0
    {
        return -ENAMETOOLONG;
    }
    snprintf(
        &raw mut (*out).entry as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut (*out).host as *mut ::core::ffi::c_char,
    );
    if trailing != 0 {
        let mut hl: size_t = strlen(&raw mut (*out).host as *mut ::core::ffi::c_char);
        if hl.wrapping_add(1 as size_t) >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>() {
            return -ENAMETOOLONG;
        }
        if (*out).host[hl.wrapping_sub(1 as size_t)] as ::core::ffi::c_int
            != '/' as ::core::ffi::c_int
        {
            (*out).host[hl] = '/' as ::core::ffi::c_char;
            (*out).host[hl.wrapping_add(1 as size_t)] = 0 as ::core::ffi::c_char;
        }
    }
    if flags & ENG_RES_DIR_ONLY != 0 && (*out).exists != 0 {
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
            __pad3: [0; 3],
        };
        if stat(
            &raw mut (*out).host as *mut ::core::ffi::c_char,
            &raw mut st_0,
        ) == 0 as ::core::ffi::c_int
            && !(st_0.st_mode & S_IFMT as mode_t == S_IFDIR as mode_t)
        {
            return -ENOTDIR;
        }
    }
    return 0 as ::core::ffi::c_int;
}
