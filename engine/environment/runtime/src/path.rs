//! Component-by-component guest path resolution, including proc magic links.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    unsafe fn readlink(
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> isize;
    unsafe fn eng_task_find(_: *mut eng_tracer, _: i32) -> *mut eng_task;
    unsafe fn eng_tracer_guest(_: *mut eng_tracer) -> *mut eng_guest;
    #[cfg(target_os = "linux")]
    #[link_name = "__errno_location"]
    unsafe fn errno() -> *mut i32;
    #[cfg(target_os = "android")]
    #[link_name = "__errno"]
    unsafe fn errno() -> *mut i32;
    unsafe fn snprintf(
        _: *mut ::core::ffi::c_char,
        _: usize,
        _: *const ::core::ffi::c_char,
        ...
    ) -> i32;
    unsafe fn atoi(_: *const ::core::ffi::c_char) -> i32;
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: usize,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memset(_: *mut ::core::ffi::c_void, _: i32, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn strcpy(
        _: *mut ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strcmp(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn strncmp(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn strchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strrchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(_: *const ::core::ffi::c_char) -> usize;
    unsafe fn stat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
    unsafe fn lstat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
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
    unsafe fn eng_meta_get(
        _: *mut eng_guest,
        _: *const ::core::ffi::c_char,
        _: i32,
        _: *const stat,
        _: *mut eng_meta,
    ) -> i32;
    unsafe fn eng_meta_permission(_: *mut eng_task, _: *const eng_meta, _: i32, _: i32) -> i32;
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
pub struct eng_meta {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub nlink: u32,
    pub major: u32,
    pub minor: u32,
    pub present: i32,
}
pub const NAME_MAX: i32 = 255 as i32;
pub const PATH_MAX: i32 = 4096 as i32;
pub const X_OK: i32 = 1 as i32;
pub const ENG_RES_FOLLOW: i32 = 0x1 as i32;
pub const ENG_RES_MISSING_OK: i32 = 0x2 as i32;
pub const ENG_RES_DIR_ONLY: i32 = 0x4 as i32;
pub const ENOENT: i32 = 2 as i32;
pub const EBADF: i32 = 9 as i32;
pub const EACCES: i32 = 13 as i32;
pub const ENOTDIR: i32 = 20 as i32;
pub const ENAMETOOLONG: i32 = 36 as i32;
pub const ELOOP: i32 = 40 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFMT: i32 = 0o170000 as i32;
pub const AT_FDCWD: i32 = -100 as i32;
pub const ENG_CAP_DAC_OVERRIDE: i32 = 1 as i32;
pub const ENG_CAP_DAC_READ_SEARCH: i32 = 2 as i32;
pub const MAX_LINKS: i32 = 40 as i32;
pub const S_IFLNK: i32 = 0o120000 as i32;
pub const S_IFDIR: i32 = 0o40000 as i32;
pub const S_IFMT: i32 = 0o170000 as i32;
unsafe extern "C" fn read_host_link(
    mut hostpath: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut n: isize = readlink(hostpath, out, cap.wrapping_sub(1 as usize));
    if n < 0 as isize {
        return -*errno();
    }
    *out.offset(n) = 0 as ::core::ffi::c_char;
    return 0 as i32;
}
unsafe extern "C" fn link_text_to_guest(
    mut g: *const eng_guest,
    mut text: *mut ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut n: usize = strlen(text);
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
            ::core::mem::size_of::<[::core::ffi::c_char; 11]>().wrapping_sub(1 as usize),
        ) as isize) = 0 as ::core::ffi::c_char;
    }
    if *text.offset(0isize) as i32 != '/' as i32 {
        snprintf(
            out,
            cap,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            text,
        );
        return -ENOENT;
    }
    if eng_host_to_guest(g, text, out, cap) != 0 as i32 {
        snprintf(
            out,
            cap,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            text,
        );
        return -ENOENT;
    }
    return 0 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_task_cwd(
    mut t: *mut eng_task,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut p: [::core::ffi::c_char; 64] = [0; 64];
    let mut text: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
        b"/proc/%d/cwd\0".as_ptr() as *const ::core::ffi::c_char,
        (*t).tid,
    );
    let mut rc: i32 = read_host_link(
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
    mut pid: i32,
    mut fd: i32,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut p: [::core::ffi::c_char; 64] = [0; 64];
    let mut text: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut p as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
        b"/proc/%d/fd/%d\0".as_ptr() as *const ::core::ffi::c_char,
        if pid != 0 { pid } else { (*t).tid },
        fd,
    );
    let mut rc: i32 = read_host_link(
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
) -> i32 {
    if (*t).cr.cap_eff as u64
        & ((1 as u64) << ENG_CAP_DAC_OVERRIDE | (1 as u64) << ENG_CAP_DAC_READ_SEARCH)
        != 0
    {
        return 0 as i32;
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
        return 0 as i32;
    }
    let mut st: stat = platform_empty_stat();
    if stat(&raw mut host as *mut ::core::ffi::c_char, &raw mut st) != 0 as i32
        || !(st.st_mode & S_IFMT as u32 == S_IFDIR as u32)
    {
        return 0 as i32;
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
        0 as i32,
        &raw mut st,
        &raw mut m,
    );
    return eng_meta_permission(t, &raw mut m, X_OK, 0 as i32);
}
unsafe extern "C" fn is_num(mut s: *const ::core::ffi::c_char, mut n: usize) -> i32 {
    if n == 0 {
        return 0 as i32;
    }
    let mut i: usize = 0 as usize;
    while i < n {
        if (*s.offset(i as isize) as i32) < '0' as i32 || *s.offset(i as isize) as i32 > '9' as i32
        {
            return 0 as i32;
        }
        i = i.wrapping_add(1);
    }
    return 1 as i32;
}
unsafe extern "C" fn proc_magic(
    mut cand: *const ::core::ffi::c_char,
    mut kind: *mut ::core::ffi::c_char,
    mut pid: *mut i32,
    mut fd: *mut i32,
) -> i32 {
    if strncmp(
        cand,
        b"/proc/\0".as_ptr() as *const ::core::ffi::c_char,
        6 as usize,
    ) != 0
    {
        return 0 as i32;
    }
    let mut p: *const ::core::ffi::c_char = cand.offset(6 as i32 as isize);
    let mut s: *const ::core::ffi::c_char = strchr(p, '/' as i32);
    if s.is_null() || is_num(p, s.offset_from(p) as usize) == 0 {
        return 0 as i32;
    }
    *pid = atoi(p);
    p = s.offset(1 as i32 as isize);
    if strncmp(
        p,
        b"task/\0".as_ptr() as *const ::core::ffi::c_char,
        5 as usize,
    ) == 0
    {
        let mut q: *const ::core::ffi::c_char = p.offset(5 as i32 as isize);
        let mut e: *const ::core::ffi::c_char = strchr(q, '/' as i32);
        if e.is_null() || is_num(q, e.offset_from(q) as usize) == 0 {
            return 0 as i32;
        }
        *pid = atoi(q);
        p = e.offset(1 as i32 as isize);
    }
    if strcmp(p, b"cwd\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
        *kind = 'c' as ::core::ffi::c_char;
        return 1 as i32;
    }
    if strcmp(p, b"root\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
        *kind = 'r' as ::core::ffi::c_char;
        return 1 as i32;
    }
    if strcmp(p, b"exe\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
        *kind = 'e' as ::core::ffi::c_char;
        return 1 as i32;
    }
    if strncmp(
        p,
        b"fd/\0".as_ptr() as *const ::core::ffi::c_char,
        3 as usize,
    ) == 0
        && is_num(
            p.offset(3 as i32 as isize),
            strlen(p.offset(3 as i32 as isize)),
        ) != 0
    {
        *kind = 'f' as ::core::ffi::c_char;
        *fd = atoi(p.offset(3 as i32 as isize));
        return 1 as i32;
    }
    return 0 as i32;
}
unsafe extern "C" fn magic_target(
    mut t: *mut eng_task,
    mut kind: ::core::ffi::c_char,
    mut pid: i32,
    mut fd: i32,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut g: *const eng_guest = eng_tracer_guest((*t).tr);
    let mut p: [::core::ffi::c_char; 64] = [0; 64];
    let mut text: [::core::ffi::c_char; 4096] = [0; 4096];
    match kind as i32 {
        114 => {
            snprintf(out, cap, b"/\0".as_ptr() as *const ::core::ffi::c_char);
            return 0 as i32;
        }
        101 => {
            let mut o: *mut eng_task = eng_task_find((*t).tr, pid);
            if !o.is_null() && !(*o).proc.is_null() && (*(*o).proc).exe[0usize] as i32 != 0 {
                snprintf(
                    out,
                    cap,
                    b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                    &raw mut (*(*o).proc).exe as *mut ::core::ffi::c_char,
                );
                return 0 as i32;
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
    let mut rc: i32 = read_host_link(
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
    mut dirfd: i32,
    mut path: *const ::core::ffi::c_char,
    mut flags: i32,
    mut out: *mut eng_resolved,
) -> i32 {
    let mut g: *const eng_guest = eng_tracer_guest((*t).tr);
    memset(out as *mut ::core::ffi::c_void, 0 as i32, 8200 as usize);
    (*out).magic_text[0usize] = 0 as ::core::ffi::c_char;
    // The initial cwd/root already exists. Dot-only paths visit no component.
    (*out).exists = 1;
    (*out).stub = 0 as i32;
    (*out).stub_id[0usize] = 0 as ::core::ffi::c_char;
    (*out).entry[0usize] = 0 as ::core::ffi::c_char;
    if *path.offset(0isize) == 0 {
        return -ENOENT;
    }
    let mut plen: usize = strlen(path);
    if plen >= PATH_MAX as usize {
        return -ENAMETOOLONG;
    }
    let mut cur: [::core::ffi::c_char; 4096] = [0; 4096];
    if *path.offset(0isize) as i32 == '/' as i32 {
        strcpy(
            &raw mut cur as *mut ::core::ffi::c_char,
            b"/\0".as_ptr() as *const ::core::ffi::c_char,
        );
    } else {
        let mut rc: i32 = if dirfd == AT_FDCWD {
            eng_task_cwd(
                t,
                &raw mut cur as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            )
        } else {
            eng_task_fd_path(
                t,
                0 as i32,
                dirfd,
                &raw mut cur as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            )
        };
        if rc == -ENOENT && dirfd != AT_FDCWD {
            let mut n: i32 = snprintf(
                &raw mut (*out).host as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"/proc/%d/fd/%d/%s\0".as_ptr() as *const ::core::ffi::c_char,
                (*t).tid,
                dirfd,
                path,
            );
            if n <= 0 as i32 || n as usize >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
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
            (*out).verbatim = 1 as i32;
            return 0 as i32;
        }
        if rc != 0 {
            return rc;
        }
    }
    let mut trailing: i32 = (plen > 0 as usize
        && *path.offset(plen.wrapping_sub(1 as usize) as isize) as i32 == '/' as i32)
        as i32;
    let mut todo: [::core::ffi::c_char; 8194] = [0; 8194];
    snprintf(
        &raw mut todo as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        path,
    );
    let mut pos: usize = 0 as usize;
    let mut links: i32 = 0 as i32;
    '_done: {
        loop {
            while todo[pos] as i32 == '/' as i32 {
                pos = pos.wrapping_add(1);
            }
            if todo[pos] == 0 {
                break;
            }
            let mut start: usize = pos;
            while todo[pos] as i32 != 0 && todo[pos] as i32 != '/' as i32 {
                pos = pos.wrapping_add(1);
            }
            let mut clen: usize = pos.wrapping_sub(start);
            let mut after: usize = pos;
            while todo[after] as i32 == '/' as i32 {
                after = after.wrapping_add(1);
            }
            let mut last: i32 = (todo[after] as i32 == 0 as i32) as i32;
            if clen >= (NAME_MAX + 1 as i32) as usize {
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
                5 as usize,
            ) != 0
                && strncmp(
                    &raw mut cur as *mut ::core::ffi::c_char,
                    b"/sys\0".as_ptr() as *const ::core::ffi::c_char,
                    4 as usize,
                ) != 0
            {
                let mut prc: i32 = may_search(t, &raw mut cur as *mut ::core::ffi::c_char);
                if prc != 0 {
                    return prc;
                }
            }
            if strcmp(
                &raw mut comp as *mut ::core::ffi::c_char,
                b"..\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                let mut sl: *mut ::core::ffi::c_char =
                    strrchr(&raw mut cur as *mut ::core::ffi::c_char, '/' as i32);
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
                if comp[0usize] as i32 == 's' as i32 {
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
                pos = 0 as usize;
            } else {
                let mut cand: [::core::ffi::c_char; 4096] = [0; 4096];
                let mut n_0: i32 = snprintf(
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
                if n_0 <= 0 as i32
                    || n_0 as usize >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
                {
                    return -ENAMETOOLONG;
                }
                if eng_guest_is_hidden(g, &raw mut cand as *mut ::core::ffi::c_char) != 0 {
                    if last != 0 && flags & ENG_RES_MISSING_OK != 0 {
                        return -EACCES;
                    }
                    return -ENOENT;
                }
                let mut follow_here: i32 =
                    (last == 0 || flags & ENG_RES_FOLLOW != 0 || trailing != 0) as i32;
                let mut kind: ::core::ffi::c_char = 0;
                let mut mpid: i32 = 0;
                let mut mfd: i32 = -1 as i32;
                if proc_magic(
                    &raw mut cand as *mut ::core::ffi::c_char,
                    &raw mut kind,
                    &raw mut mpid,
                    &raw mut mfd,
                ) != 0
                {
                    let mut tgt_0: [::core::ffi::c_char; 4096] = [0; 4096];
                    let mut rc_0: i32 = magic_target(
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
                        (*out).magic = 1 as i32;
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
                        (*out).exists = 1 as i32;
                        return 0 as i32;
                    }
                    if rc_0 == -ENOENT {
                        let mut h: [::core::ffi::c_char; 4096] = [0; 4096];
                        eng_guest_to_host(
                            g,
                            &raw mut cand as *mut ::core::ffi::c_char,
                            &raw mut h as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                        );
                        let mut m: i32 = snprintf(
                            &raw mut (*out).host as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                            &raw mut h as *mut ::core::ffi::c_char,
                            (&raw mut todo as *mut ::core::ffi::c_char).offset(pos as isize),
                        );
                        if m <= 0 as i32
                            || m as usize >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>()
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
                        (*out).verbatim = 1 as i32;
                        (*out).exists = 1 as i32;
                        return 0 as i32;
                    }
                    links += 1;
                    if links > MAX_LINKS {
                        return -ELOOP;
                    }
                    let mut rest_0: [::core::ffi::c_char; 8194] = [0; 8194];
                    let mut m_0: i32 = snprintf(
                        &raw mut rest_0 as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
                        b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut tgt_0 as *mut ::core::ffi::c_char,
                        (&raw mut todo as *mut ::core::ffi::c_char).offset(pos as isize),
                    );
                    if m_0 <= 0 as i32
                        || m_0 as usize >= ::core::mem::size_of::<[::core::ffi::c_char; 8194]>()
                    {
                        return -ENAMETOOLONG;
                    }
                    snprintf(
                        &raw mut todo as *mut ::core::ffi::c_char,
                        ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut rest_0 as *mut ::core::ffi::c_char,
                    );
                    pos = 0 as usize;
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
                    let mut st: stat = platform_empty_stat();
                    if lstat(&raw mut host as *mut ::core::ffi::c_char, &raw mut st) != 0 as i32 {
                        let mut e: i32 = *errno();
                        if e == ENOENT && last != 0 && flags & ENG_RES_MISSING_OK != 0 {
                            snprintf(
                                &raw mut cur as *mut ::core::ffi::c_char,
                                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                                &raw mut cand as *mut ::core::ffi::c_char,
                            );
                            (*out).exists = 0 as i32;
                            break '_done;
                        } else {
                            return -e;
                        }
                    } else {
                        if st.st_mode & S_IFMT as u32 == S_IFLNK as u32 {
                            let mut tgt_1: [::core::ffi::c_char; 4096] = [0; 4096];
                            let mut rc_1: i32 = read_host_link(
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
                                (*out).stub = 1 as i32;
                                (*out).exists = 1 as i32;
                                if trailing != 0 {
                                    return -ENOTDIR;
                                }
                                return 0 as i32;
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
                                let mut m_1: i32 = snprintf(
                                    &raw mut rest_1 as *mut ::core::ffi::c_char,
                                    ::core::mem::size_of::<[::core::ffi::c_char; 8194]>(),
                                    b"%s%s\0".as_ptr() as *const ::core::ffi::c_char,
                                    &raw mut tgt_1 as *mut ::core::ffi::c_char,
                                    (&raw mut todo as *mut ::core::ffi::c_char)
                                        .offset(pos as isize),
                                );
                                if m_1 <= 0 as i32
                                    || m_1 as usize
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
                                pos = 0 as usize;
                                if tgt_1[0usize] as i32 == '/' as i32 {
                                    strcpy(
                                        &raw mut cur as *mut ::core::ffi::c_char,
                                        b"/\0".as_ptr() as *const ::core::ffi::c_char,
                                    );
                                }
                                continue;
                            }
                        }
                        if last == 0 && !(st.st_mode & S_IFMT as u32 == S_IFDIR as u32) {
                            return -ENOTDIR;
                        }
                        snprintf(
                            &raw mut cur as *mut ::core::ffi::c_char,
                            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                            &raw mut cand as *mut ::core::ffi::c_char,
                        );
                        (*out).exists = 1 as i32;
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
            (*out).exists = 1 as i32;
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
        let mut hl: usize = strlen(&raw mut (*out).host as *mut ::core::ffi::c_char);
        if hl.wrapping_add(1 as usize) >= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>() {
            return -ENAMETOOLONG;
        }
        if (*out).host[hl.wrapping_sub(1 as usize)] as i32 != '/' as i32 {
            (*out).host[hl] = '/' as ::core::ffi::c_char;
            (*out).host[hl.wrapping_add(1 as usize)] = 0 as ::core::ffi::c_char;
        }
    }
    if flags & ENG_RES_DIR_ONLY != 0 && (*out).exists != 0 {
        let mut st_0: stat = platform_empty_stat();
        if stat(
            &raw mut (*out).host as *mut ::core::ffi::c_char,
            &raw mut st_0,
        ) == 0 as i32
            && !(st_0.st_mode & S_IFMT as u32 == S_IFDIR as u32)
        {
            return -ENOTDIR;
        }
    }
    return 0 as i32;
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
