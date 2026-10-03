//! ELF and shebang planning; real exec into the freestanding Rust loader.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
#[repr(C)]
pub struct eng_tracer {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    unsafe fn close(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    unsafe fn read(
        __fd: ::core::ffi::c_int,
        __buf: *mut ::core::ffi::c_void,
        __nbytes: size_t,
    ) -> ssize_t;
    unsafe fn pread(
        __fd: ::core::ffi::c_int,
        __buf: *mut ::core::ffi::c_void,
        __nbytes: size_t,
        __offset: __off_t,
    ) -> ssize_t;
    unsafe fn eng_tracer_guest(tr: *mut eng_tracer) -> *mut eng_guest;
    unsafe fn __errno_location() -> *mut ::core::ffi::c_int;
    unsafe fn open(
        __file: *const ::core::ffi::c_char,
        __oflag: ::core::ffi::c_int,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn strtoul(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulong;
    unsafe fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    unsafe fn free(__ptr: *mut ::core::ffi::c_void);
    unsafe fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memmove(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn memchr(
        __s: *const ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn strdup(__s: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strstr(
        __haystack: *const ::core::ffi::c_char,
        __needle: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    unsafe fn fstat(__fd: ::core::ffi::c_int, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn lstat(__file: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn eng_guest_to_host(
        g: *const eng_guest,
        guest: *const ::core::ffi::c_char,
        host: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_ident_setid(
        t: *const eng_task,
        mode: uint32_t,
        uid: uint32_t,
        gid: uint32_t,
        neuid: *mut uint32_t,
        negid: *mut uint32_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_meta_get(
        g: *mut eng_guest,
        host: *const ::core::ffi::c_char,
        nofollow: ::core::ffi::c_int,
        st: *const stat,
        m: *mut eng_meta,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_meta_is_placeholder(m: *const eng_meta, host_mode: mode_t) -> ::core::ffi::c_int;
    unsafe fn eng_meta_may_exec(
        t: *mut eng_task,
        host: *const ::core::ffi::c_char,
        st: *const stat,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_resolve(
        t: *mut eng_task,
        dirfd: ::core::ffi::c_int,
        path: *const ::core::ffi::c_char,
        flags: ::core::ffi::c_int,
        out: *mut eng_resolved,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_task_fd_path(
        t: *mut eng_task,
        pid: pid_t,
        fd: ::core::ffi::c_int,
        out: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
}
pub type __dev_t = ::core::ffi::c_ulong;
pub type __uid_t = ::core::ffi::c_uint;
pub type __gid_t = ::core::ffi::c_uint;
pub type __ino_t = ::core::ffi::c_ulong;
pub type __mode_t = ::core::ffi::c_uint;
pub type __nlink_t = ::core::ffi::c_ulong;
pub type __off_t = ::core::ffi::c_long;
pub type __pid_t = ::core::ffi::c_int;
pub type __time_t = ::core::ffi::c_long;
pub type __blksize_t = ::core::ffi::c_long;
pub type __blkcnt_t = ::core::ffi::c_long;
pub type __syscall_slong_t = ::core::ffi::c_long;
pub type pid_t = __pid_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __time_t,
    pub tv_nsec: __syscall_slong_t,
}
pub type size_t = usize;
pub type ssize_t = isize;
pub type off_t = __off_t;
pub type uint8_t = u8;
pub type uint16_t = u16;
pub type uint32_t = u32;
pub type uint64_t = u64;
pub type mode_t = __mode_t;
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
pub struct strlist {
    pub s: [*mut ::core::ffi::c_char; 16],
    pub n: ::core::ffi::c_int,
}
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
pub struct eng_meta {
    pub uid: uint32_t,
    pub gid: uint32_t,
    pub mode: uint32_t,
    pub nlink: uint32_t,
    pub major: uint32_t,
    pub minor: uint32_t,
    pub present: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_load_plan {
    pub magic: uint32_t,
    pub version: uint32_t,
    pub size: uint32_t,
    pub flags: uint32_t,
    pub exe_off: uint32_t,
    pub interp_off: uint32_t,
    pub execfn_off: uint32_t,
    pub argv_skip: uint32_t,
    pub n_prepend: uint32_t,
    pub prepend_off: uint32_t,
    pub uid: uint32_t,
    pub euid: uint32_t,
    pub gid: uint32_t,
    pub egid: uint32_t,
    pub pagesz: uint32_t,
    pub comm_off: uint32_t,
    pub reserved: [uint32_t; 2],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Elf64_Ehdr {
    pub e_ident: [::core::ffi::c_uchar; 16],
    pub e_type: Elf64_Half,
    pub e_machine: Elf64_Half,
    pub e_version: Elf64_Word,
    pub e_entry: Elf64_Addr,
    pub e_phoff: Elf64_Off,
    pub e_shoff: Elf64_Off,
    pub e_flags: Elf64_Word,
    pub e_ehsize: Elf64_Half,
    pub e_phentsize: Elf64_Half,
    pub e_phnum: Elf64_Half,
    pub e_shentsize: Elf64_Half,
    pub e_shnum: Elf64_Half,
    pub e_shstrndx: Elf64_Half,
}
pub type Elf64_Half = uint16_t;
pub type Elf64_Word = uint32_t;
pub type Elf64_Off = uint64_t;
pub type Elf64_Addr = uint64_t;
pub type Elf64_Xword = uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Elf64_Phdr {
    pub p_type: Elf64_Word,
    pub p_flags: Elf64_Word,
    pub p_offset: Elf64_Off,
    pub p_vaddr: Elf64_Addr,
    pub p_paddr: Elf64_Addr,
    pub p_filesz: Elf64_Xword,
    pub p_memsz: Elf64_Xword,
    pub p_align: Elf64_Xword,
}
pub const ELFMAG: [::core::ffi::c_char; 5] =
    unsafe { ::core::mem::transmute::<[u8; 5], [::core::ffi::c_char; 5]>(*b"\x7FELF\0") };
pub const SELFMAG: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const EI_CLASS: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ELFCLASS64: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const EI_DATA: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const ELFDATA2LSB: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ET_EXEC: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ET_DYN: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const EM_X86_64: ::core::ffi::c_int = 62 as ::core::ffi::c_int;
pub const PT_LOAD: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PT_INTERP: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const ENOENT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const E2BIG: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const ENOEXEC: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const ENOMEM: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const EACCES: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const ENOTDIR: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const ENAMETOOLONG: ::core::ffi::c_int = 36 as ::core::ffi::c_int;
pub const ELOOP: ::core::ffi::c_int = 40 as ::core::ffi::c_int;
pub const O_RDONLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const __O_CLOEXEC: ::core::ffi::c_int = 0o2000000 as ::core::ffi::c_int;
pub const O_CLOEXEC: ::core::ffi::c_int = __O_CLOEXEC;
pub const __S_IFMT: ::core::ffi::c_int = 0o170000 as ::core::ffi::c_int;
pub const AT_FDCWD: ::core::ffi::c_int = -100 as ::core::ffi::c_int;
pub const AT_SYMLINK_NOFOLLOW: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const AT_EMPTY_PATH: ::core::ffi::c_int = 0x1000 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ENG_PLAN_MAGIC: ::core::ffi::c_uint = 0x4e414c50 as ::core::ffi::c_uint;
pub const ENG_PLAN_VERSION: ::core::ffi::c_uint = 2 as ::core::ffi::c_uint;
pub const ENG_PLAN_MAX: ::core::ffi::c_uint =
    (64 as ::core::ffi::c_uint).wrapping_mul(1024 as ::core::ffi::c_uint);
pub const ENG_PLAN_SECURE: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const ENG_PLAN_NO_FILEMAP: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const ENG_PLAN_PAGESZ: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const ENG_RES_FOLLOW: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const EM_SELF: ::core::ffi::c_int = EM_X86_64;
pub const MAX_SCRIPT_DEPTH: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const MAX_PREPEND: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
unsafe extern "C" fn sl_free(mut l: *mut strlist) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*l).n {
        free((*l).s[i as usize] as *mut ::core::ffi::c_void);
        i += 1;
    }
    (*l).n = 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn sl_push_front(
    mut l: *mut strlist,
    mut s: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if (*l).n >= MAX_PREPEND {
        return -E2BIG;
    }
    memmove(
        (&raw mut (*l).s as *mut *mut ::core::ffi::c_char).offset(1isize)
            as *mut ::core::ffi::c_void,
        (&raw mut (*l).s as *mut *mut ::core::ffi::c_char).offset(0isize)
            as *const ::core::ffi::c_void,
        ::core::mem::size_of::<*mut ::core::ffi::c_char>().wrapping_mul((*l).n as size_t),
    );
    (*l).s[0usize] = strdup(s);
    (*l).n += 1;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn sl_pop_front(mut l: *mut strlist) {
    if (*l).n == 0 {
        return;
    }
    free((*l).s[0usize] as *mut ::core::ffi::c_void);
    memmove(
        (&raw mut (*l).s as *mut *mut ::core::ffi::c_char).offset(0isize)
            as *mut ::core::ffi::c_void,
        (&raw mut (*l).s as *mut *mut ::core::ffi::c_char).offset(1isize)
            as *const ::core::ffi::c_void,
        ::core::mem::size_of::<*mut ::core::ffi::c_char>()
            .wrapping_mul(((*l).n - 1 as ::core::ffi::c_int) as size_t),
    );
    (*l).n -= 1;
}
unsafe extern "C" fn parse_shebang(
    mut buf: *const ::core::ffi::c_char,
    mut n: size_t,
    mut interp: *mut ::core::ffi::c_char,
    mut icap: size_t,
    mut arg: *mut ::core::ffi::c_char,
    mut acap: size_t,
    mut has_arg: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut end: *const ::core::ffi::c_char = memchr(
        buf as *const ::core::ffi::c_void,
        '\n' as ::core::ffi::c_int,
        n,
    ) as *const ::core::ffi::c_void
        as *const ::core::ffi::c_char;
    let mut truncated: ::core::ffi::c_int = end.is_null() as ::core::ffi::c_int;
    if end.is_null() {
        end = buf.offset(n as isize);
    }
    let mut p: *const ::core::ffi::c_char = buf.offset(2 as ::core::ffi::c_int as isize);
    while p < end
        && (*p as ::core::ffi::c_int == ' ' as ::core::ffi::c_int
            || *p as ::core::ffi::c_int == '\t' as ::core::ffi::c_int)
    {
        p = p.offset(1);
    }
    let mut e: *const ::core::ffi::c_char = end;
    while e > p
        && (*e.offset(-1isize) as ::core::ffi::c_int == ' ' as ::core::ffi::c_int
            || *e.offset(-1isize) as ::core::ffi::c_int == '\t' as ::core::ffi::c_int
            || *e.offset(-1isize) as ::core::ffi::c_int == '\r' as ::core::ffi::c_int
            || *e.offset(-1isize) as ::core::ffi::c_int == 0 as ::core::ffi::c_int)
    {
        e = e.offset(-1);
    }
    if p >= e {
        return -ENOEXEC;
    }
    let mut ie: *const ::core::ffi::c_char = p;
    while ie < e
        && *ie as ::core::ffi::c_int != ' ' as ::core::ffi::c_int
        && *ie as ::core::ffi::c_int != '\t' as ::core::ffi::c_int
        && *ie as ::core::ffi::c_int != 0
    {
        ie = ie.offset(1);
    }
    if truncated != 0 && ie == e {
        return -ENOEXEC;
    }
    let mut il: size_t = ie.offset_from(p) as size_t;
    if il >= icap {
        return -ENAMETOOLONG;
    }
    memcpy(
        interp as *mut ::core::ffi::c_void,
        p as *const ::core::ffi::c_void,
        il,
    );
    *interp.offset(il as isize) = 0 as ::core::ffi::c_char;
    let mut a: *const ::core::ffi::c_char = ie;
    while a < e
        && (*a as ::core::ffi::c_int == ' ' as ::core::ffi::c_int
            || *a as ::core::ffi::c_int == '\t' as ::core::ffi::c_int)
    {
        a = a.offset(1);
    }
    *has_arg = (a < e) as ::core::ffi::c_int;
    if *has_arg != 0 {
        let mut al: size_t = e.offset_from(a) as size_t;
        if al >= acap {
            al = acap.wrapping_sub(1 as size_t);
        }
        memcpy(
            arg as *mut ::core::ffi::c_void,
            a as *const ::core::ffi::c_void,
            al,
        );
        *arg.offset(al as isize) = 0 as ::core::ffi::c_char;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn elf_interp(
    mut fd: ::core::ffi::c_int,
    mut eh: *const Elf64_Ehdr,
    mut interp: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    if (*eh).e_phentsize as usize != ::core::mem::size_of::<Elf64_Phdr>()
        || (*eh).e_phnum as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        || (*eh).e_phnum as ::core::ffi::c_int > 256 as ::core::ffi::c_int
    {
        return -ENOEXEC;
    }
    let mut ph: [Elf64_Phdr; 256] = [Elf64_Phdr {
        p_type: 0,
        p_flags: 0,
        p_offset: 0,
        p_vaddr: 0,
        p_paddr: 0,
        p_filesz: 0,
        p_memsz: 0,
        p_align: 0,
    }; 256];
    let mut sz: size_t =
        ((*eh).e_phnum as size_t).wrapping_mul(::core::mem::size_of::<Elf64_Phdr>());
    if pread(
        fd,
        &raw mut ph as *mut Elf64_Phdr as *mut ::core::ffi::c_void,
        sz,
        (*eh).e_phoff as __off_t,
    ) != sz as ssize_t
    {
        return -ENOEXEC;
    }
    let mut loads: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*eh).e_phnum as ::core::ffi::c_int {
        if ph[i as usize].p_type == PT_LOAD as Elf64_Word {
            loads += 1;
        }
        if ph[i as usize].p_type != PT_INTERP as Elf64_Word {
            i += 1;
        } else {
            if ph[i as usize].p_filesz < 2 as Elf64_Xword
                || ph[i as usize].p_filesz >= cap as Elf64_Xword
            {
                return -ENOEXEC;
            }
            if pread(
                fd,
                interp as *mut ::core::ffi::c_void,
                ph[i as usize].p_filesz as size_t,
                ph[i as usize].p_offset as __off_t,
            ) != ph[i as usize].p_filesz as ssize_t
            {
                return -ENOEXEC;
            }
            *interp.offset(ph[i as usize].p_filesz as isize) = 0 as ::core::ffi::c_char;
            if *interp.offset(ph[i as usize].p_filesz.wrapping_sub(1 as Elf64_Xword) as isize)
                as ::core::ffi::c_int
                != 0 as ::core::ffi::c_int
            {
                return -ENOEXEC;
            }
            return 1 as ::core::ffi::c_int;
        }
    }
    return if loads != 0 {
        0 as ::core::ffi::c_int
    } else {
        -ENOEXEC
    };
}
unsafe extern "C" fn open_candidate(
    mut t: *mut eng_task,
    mut res: *const eng_resolved,
    mut st: *mut stat,
) -> ::core::ffi::c_int {
    let mut fd: ::core::ffi::c_int = open(
        &raw const (*res).host as *const ::core::ffi::c_char,
        O_RDONLY | O_CLOEXEC,
    );
    if fd < 0 as ::core::ffi::c_int {
        return -*__errno_location();
    }
    if fstat(fd, st) != 0 as ::core::ffi::c_int {
        let mut e: ::core::ffi::c_int = *__errno_location();
        close(fd);
        return -e;
    }
    if !((*st).st_mode & __S_IFMT as __mode_t == 0o100000 as __mode_t) {
        close(fd);
        return -EACCES;
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
        &raw const (*res).host as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
        st,
        &raw mut vm,
    );
    if eng_meta_is_placeholder(&raw mut vm, (*st).st_mode) != 0 {
        close(fd);
        return -EACCES;
    }
    let mut rc: ::core::ffi::c_int =
        eng_meta_may_exec(t, &raw const (*res).host as *const ::core::ffi::c_char, st);
    if rc != 0 {
        close(fd);
        return rc;
    }
    return fd;
}
unsafe extern "C" fn match_binfmt(
    mut g: *const eng_guest,
    mut buf: *const ::core::ffi::c_uchar,
    mut n: size_t,
    mut filename: *const ::core::ffi::c_char,
) -> *const eng_binfmt {
    let mut i: ::core::ffi::c_int = (*g).nbinfmt - 1 as ::core::ffi::c_int;
    while i >= 0 as ::core::ffi::c_int {
        let mut b: *const eng_binfmt =
            (&raw const (*g).binfmt as *const eng_binfmt).offset(i as isize);
        if (*b).r#type as ::core::ffi::c_int == 'E' as ::core::ffi::c_int {
            let mut base: *const ::core::ffi::c_char =
                strrchr(filename, '/' as ::core::ffi::c_int) as *const ::core::ffi::c_char;
            base = if !base.is_null() {
                base.offset(1 as ::core::ffi::c_int as isize)
            } else {
                filename
            };
            let mut dot: *const ::core::ffi::c_char =
                strrchr(base, '.' as ::core::ffi::c_int) as *const ::core::ffi::c_char;
            if !dot.is_null()
                && strcmp(
                    dot.offset(1 as ::core::ffi::c_int as isize),
                    &raw const (*b).ext as *const ::core::ffi::c_char,
                ) == 0
            {
                return b;
            }
        } else if (*b).offset.wrapping_add((*b).len) as size_t <= n {
            let mut k: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
            while k < (*b).len
                && *buf.offset((*b).offset.wrapping_add(k) as isize) as ::core::ffi::c_int
                    & (*b).mask[k as usize] as ::core::ffi::c_int
                    == (*b).magic[k as usize] as ::core::ffi::c_int
            {
                k = k.wrapping_add(1);
            }
            if k == (*b).len {
                return b;
            }
        }
        i -= 1;
    }
    return ::core::ptr::null::<eng_binfmt>();
}
unsafe extern "C" fn build_plan(
    mut t: *mut eng_task,
    mut exe_host: *const ::core::ffi::c_char,
    mut interp_host: *const ::core::ffi::c_char,
    mut execfn: *const ::core::ffi::c_char,
    mut comm: *const ::core::ffi::c_char,
    mut skip: uint32_t,
    mut pre: *const strlist,
    mut flags: uint32_t,
) -> ::core::ffi::c_int {
    let mut need: size_t = ::core::mem::size_of::<eng_load_plan>()
        .wrapping_add(strlen(exe_host))
        .wrapping_add(1 as size_t)
        .wrapping_add(if !interp_host.is_null() {
            strlen(interp_host).wrapping_add(1 as size_t)
        } else {
            0 as size_t
        })
        .wrapping_add(strlen(execfn))
        .wrapping_add(1 as size_t)
        .wrapping_add(strlen(comm))
        .wrapping_add(1 as size_t);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*pre).n {
        need = need.wrapping_add(strlen((*pre).s[i as usize]).wrapping_add(1 as size_t));
        i += 1;
    }
    if need > ENG_PLAN_MAX as size_t {
        return -E2BIG;
    }
    let mut buf: *mut ::core::ffi::c_char = calloc(1 as size_t, need) as *mut ::core::ffi::c_char;
    if buf.is_null() {
        return -ENOMEM;
    }
    let mut pl: *mut eng_load_plan = buf as *mut eng_load_plan;
    let mut off: size_t = ::core::mem::size_of::<eng_load_plan>();
    let mut _l: size_t = strlen(exe_host).wrapping_add(1 as size_t);
    memcpy(
        buf.offset(off as isize) as *mut ::core::ffi::c_void,
        exe_host as *const ::core::ffi::c_void,
        _l,
    );
    (*pl).exe_off = off as uint32_t;
    off = off.wrapping_add(_l);
    if !interp_host.is_null() {
        let mut _l_0: size_t = strlen(interp_host).wrapping_add(1 as size_t);
        memcpy(
            buf.offset(off as isize) as *mut ::core::ffi::c_void,
            interp_host as *const ::core::ffi::c_void,
            _l_0,
        );
        (*pl).interp_off = off as uint32_t;
        off = off.wrapping_add(_l_0);
    }
    let mut _l_1: size_t = strlen(execfn).wrapping_add(1 as size_t);
    memcpy(
        buf.offset(off as isize) as *mut ::core::ffi::c_void,
        execfn as *const ::core::ffi::c_void,
        _l_1,
    );
    (*pl).execfn_off = off as uint32_t;
    off = off.wrapping_add(_l_1);
    let mut _l_2: size_t = strlen(comm).wrapping_add(1 as size_t);
    memcpy(
        buf.offset(off as isize) as *mut ::core::ffi::c_void,
        comm as *const ::core::ffi::c_void,
        _l_2,
    );
    (*pl).comm_off = off as uint32_t;
    off = off.wrapping_add(_l_2);
    if (*pre).n != 0 {
        (*pl).prepend_off = off as uint32_t;
        let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i_0 < (*pre).n {
            let mut l: size_t = strlen((*pre).s[i_0 as usize]).wrapping_add(1 as size_t);
            memcpy(
                buf.offset(off as isize) as *mut ::core::ffi::c_void,
                (*pre).s[i_0 as usize] as *const ::core::ffi::c_void,
                l,
            );
            off = off.wrapping_add(l);
            i_0 += 1;
        }
    }
    (*pl).magic = ENG_PLAN_MAGIC as uint32_t;
    (*pl).version = ENG_PLAN_VERSION as uint32_t;
    (*pl).size = off as uint32_t;
    (*pl).flags = flags;
    (*pl).argv_skip = skip;
    (*pl).n_prepend = (*pre).n as uint32_t;
    (*pl).uid = (*t).cr.ruid;
    (*pl).euid = (*t).cr.euid;
    (*pl).gid = (*t).cr.rgid;
    (*pl).egid = (*t).cr.egid;
    free((*t).plan);
    (*t).plan = buf as *mut ::core::ffi::c_void;
    (*t).plan_len = off as uint32_t;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn prepare(
    mut t: *mut eng_task,
    mut dirfd: ::core::ffi::c_int,
    mut path: *const ::core::ffi::c_char,
    mut at_flags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut g: *const eng_guest = eng_tracer_guest((*t).tr);
    let mut execfn: [::core::ffi::c_char; 4096] = [0; 4096];
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
    let mut rc: ::core::ffi::c_int = 0;
    if at_flags & AT_EMPTY_PATH != 0
        && *path.offset(0isize) as ::core::ffi::c_int == 0 as ::core::ffi::c_int
    {
        if dirfd == AT_FDCWD {
            return -ENOENT;
        }
        let mut gp: [::core::ffi::c_char; 4096] = [0; 4096];
        snprintf(
            &raw mut execfn as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"/dev/fd/%d\0".as_ptr() as *const ::core::ffi::c_char,
            dirfd,
        );
        rc = eng_task_fd_path(
            t,
            0 as pid_t,
            dirfd,
            &raw mut gp as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        );
        memset(
            &raw mut res as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<eng_resolved>(),
        );
        if rc == 0 as ::core::ffi::c_int {
            snprintf(
                &raw mut res.guest as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut gp as *mut ::core::ffi::c_char,
            );
            eng_guest_to_host(
                g,
                &raw mut gp as *mut ::core::ffi::c_char,
                &raw mut res.host as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            );
        } else if rc == -ENOENT {
            snprintf(
                &raw mut res.host as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"/proc/%d/fd/%d\0".as_ptr() as *const ::core::ffi::c_char,
                (*t).tid,
                dirfd,
            );
            snprintf(
                &raw mut res.guest as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut execfn as *mut ::core::ffi::c_char,
            );
        } else {
            return rc;
        }
    } else {
        if *path.offset(0isize) == 0 {
            return -ENOENT;
        }
        if *path.offset(0isize) as ::core::ffi::c_int == '/' as ::core::ffi::c_int
            || dirfd == AT_FDCWD
        {
            snprintf(
                &raw mut execfn as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                path,
            );
        } else {
            snprintf(
                &raw mut execfn as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"/dev/fd/%d/%s\0".as_ptr() as *const ::core::ffi::c_char,
                dirfd,
                path,
            );
        }
        let mut fl: ::core::ffi::c_int = if at_flags & AT_SYMLINK_NOFOLLOW != 0 {
            0 as ::core::ffi::c_int
        } else {
            ENG_RES_FOLLOW
        };
        rc = eng_resolve(t, dirfd, path, fl, &raw mut res);
        if rc != 0 {
            return rc;
        }
        let mut lst: stat = stat {
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
        if at_flags & AT_SYMLINK_NOFOLLOW != 0
            && lstat(&raw mut res.host as *mut ::core::ffi::c_char, &raw mut lst)
                == 0 as ::core::ffi::c_int
            && lst.st_mode & __S_IFMT as __mode_t == 0o120000 as __mode_t
        {
            return -ELOOP;
        }
    }
    let mut comm: [::core::ffi::c_char; 16] = [0; 16];
    let mut src: *const ::core::ffi::c_char = if at_flags & AT_EMPTY_PATH != 0
        && *path.offset(0isize) as ::core::ffi::c_int == 0 as ::core::ffi::c_int
    {
        &raw mut res.guest as *mut ::core::ffi::c_char as *const ::core::ffi::c_char
    } else {
        path
    };
    let mut b: *const ::core::ffi::c_char =
        strrchr(src, '/' as ::core::ffi::c_int) as *const ::core::ffi::c_char;
    b = if !b.is_null() && *b.offset(1isize) as ::core::ffi::c_int != 0 {
        b.offset(1 as ::core::ffi::c_int as isize)
    } else {
        src
    };
    snprintf(
        &raw mut comm as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 16]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        b,
    );
    let mut path_inaccessible: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if dirfd != AT_FDCWD && *path.offset(0isize) as ::core::ffi::c_int != '/' as ::core::ffi::c_int
    {
        let mut fi: [::core::ffi::c_char; 64] = [0; 64];
        let mut buf: [::core::ffi::c_char; 512] = [0; 512];
        snprintf(
            &raw mut fi as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            b"/proc/%d/fdinfo/%d\0".as_ptr() as *const ::core::ffi::c_char,
            (*t).tid,
            dirfd,
        );
        let mut ff: ::core::ffi::c_int = open(
            &raw mut fi as *mut ::core::ffi::c_char,
            O_RDONLY | O_CLOEXEC,
        );
        if ff >= 0 as ::core::ffi::c_int {
            let mut k: ssize_t = read(
                ff,
                &raw mut buf as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_char; 512]>().wrapping_sub(1 as size_t),
            );
            close(ff);
            if k > 0 as ssize_t {
                buf[k as usize] = 0 as ::core::ffi::c_char;
                let mut fl_0: *mut ::core::ffi::c_char = strstr(
                    &raw mut buf as *mut ::core::ffi::c_char,
                    b"flags:\0".as_ptr() as *const ::core::ffi::c_char,
                );
                if !fl_0.is_null()
                    && strtoul(
                        fl_0.offset(6 as ::core::ffi::c_int as isize),
                        ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                        8 as ::core::ffi::c_int,
                    ) & O_CLOEXEC as ::core::ffi::c_ulong
                        != 0
                {
                    path_inaccessible = 1 as ::core::ffi::c_int;
                }
            }
        }
    }
    let mut pre: strlist = strlist {
        s: [::core::ptr::null_mut::<::core::ffi::c_char>(); 16],
        n: 0 as ::core::ffi::c_int,
    };
    let mut skip: uint32_t = 0 as uint32_t;
    let mut filename: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut filename as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut execfn as *mut ::core::ffi::c_char,
    );
    let mut depth: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    loop {
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
        let mut fd: ::core::ffi::c_int = open_candidate(t, &raw mut res, &raw mut st);
        if fd < 0 as ::core::ffi::c_int {
            sl_free(&raw mut pre);
            return fd;
        }
        let mut buf_0: [::core::ffi::c_uchar; 256] = [0; 256];
        let mut n: ssize_t = pread(
            fd,
            &raw mut buf_0 as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<[::core::ffi::c_uchar; 256]>(),
            0 as __off_t,
        );
        if n < 0 as ssize_t {
            let mut e: ::core::ffi::c_int = *__errno_location();
            close(fd);
            sl_free(&raw mut pre);
            return -e;
        }
        let mut bf: *const eng_binfmt = match_binfmt(
            g,
            &raw mut buf_0 as *mut ::core::ffi::c_uchar,
            n as size_t,
            &raw mut filename as *mut ::core::ffi::c_char,
        );
        if !bf.is_null() {
            close(fd);
            if depth >= MAX_SCRIPT_DEPTH {
                sl_free(&raw mut pre);
                return -ELOOP;
            }
            if depth == 0 as ::core::ffi::c_int && path_inaccessible != 0 {
                sl_free(&raw mut pre);
                return -ENOENT;
            }
            if (*bf).preserve_argv0 == 0 {
                if pre.n != 0 {
                    sl_pop_front(&raw mut pre);
                } else {
                    skip = skip.wrapping_add(1);
                }
            }
            rc = sl_push_front(&raw mut pre, &raw mut filename as *mut ::core::ffi::c_char);
            if rc != 0 || {
                rc = sl_push_front(
                    &raw mut pre,
                    &raw const (*bf).interp as *const ::core::ffi::c_char,
                );
                rc != 0
            } {
                sl_free(&raw mut pre);
                return rc;
            }
            snprintf(
                &raw mut filename as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw const (*bf).interp as *const ::core::ffi::c_char,
            );
            rc = eng_resolve(
                t,
                AT_FDCWD,
                &raw const (*bf).interp as *const ::core::ffi::c_char,
                ENG_RES_FOLLOW,
                &raw mut res,
            );
            if rc != 0 {
                sl_free(&raw mut pre);
                return if rc == -ENOTDIR { -ENOENT } else { rc };
            }
        } else if n >= 2 as ssize_t
            && buf_0[0usize] as ::core::ffi::c_int == '#' as ::core::ffi::c_int
            && buf_0[1usize] as ::core::ffi::c_int == '!' as ::core::ffi::c_int
        {
            close(fd);
            if depth >= MAX_SCRIPT_DEPTH {
                sl_free(&raw mut pre);
                return -ELOOP;
            }
            if depth == 0 as ::core::ffi::c_int && path_inaccessible != 0 {
                sl_free(&raw mut pre);
                return -ENOENT;
            }
            let mut interp: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut arg: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut has_arg: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            rc = parse_shebang(
                &raw mut buf_0 as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_char,
                n as size_t,
                &raw mut interp as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                &raw mut arg as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                &raw mut has_arg,
            );
            if rc != 0 {
                sl_free(&raw mut pre);
                return rc;
            }
            if pre.n != 0 {
                sl_pop_front(&raw mut pre);
            } else {
                skip = skip.wrapping_add(1);
            }
            rc = sl_push_front(&raw mut pre, &raw mut filename as *mut ::core::ffi::c_char);
            if rc != 0
                || has_arg != 0 && {
                    rc = sl_push_front(&raw mut pre, &raw mut arg as *mut ::core::ffi::c_char);
                    rc != 0
                }
                || {
                    rc = sl_push_front(&raw mut pre, &raw mut interp as *mut ::core::ffi::c_char);
                    rc != 0
                }
            {
                sl_free(&raw mut pre);
                return rc;
            }
            snprintf(
                &raw mut filename as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut interp as *mut ::core::ffi::c_char,
            );
            rc = eng_resolve(
                t,
                AT_FDCWD,
                &raw mut interp as *mut ::core::ffi::c_char,
                ENG_RES_FOLLOW,
                &raw mut res,
            );
            if rc != 0 {
                sl_free(&raw mut pre);
                return rc;
            }
        } else {
            if n >= ::core::mem::size_of::<Elf64_Ehdr>() as ssize_t
                && memcmp(
                    &raw mut buf_0 as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
                    ELFMAG.as_ptr() as *const ::core::ffi::c_void,
                    SELFMAG as size_t,
                ) == 0
            {
                let mut eh: *const Elf64_Ehdr =
                    &raw mut buf_0 as *mut ::core::ffi::c_uchar as *const Elf64_Ehdr;
                let eh_value = eh.read_unaligned();
                let eh = &eh_value as *const Elf64_Ehdr;
                if (*eh).e_ident[EI_CLASS as usize] as ::core::ffi::c_int != ELFCLASS64
                    || (*eh).e_ident[EI_DATA as usize] as ::core::ffi::c_int != ELFDATA2LSB
                    || (*eh).e_machine as ::core::ffi::c_int != EM_SELF
                    || (*eh).e_type as ::core::ffi::c_int != ET_EXEC
                        && (*eh).e_type as ::core::ffi::c_int != ET_DYN
                {
                    close(fd);
                    sl_free(&raw mut pre);
                    return -ENOEXEC;
                }
                let mut interp_0: [::core::ffi::c_char; 4096] = [0; 4096];
                let mut hi: ::core::ffi::c_int = elf_interp(
                    fd,
                    eh,
                    &raw mut interp_0 as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                );
                close(fd);
                if hi < 0 as ::core::ffi::c_int {
                    sl_free(&raw mut pre);
                    return hi;
                }
                let mut ires: eng_resolved = eng_resolved {
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
                if hi != 0 {
                    rc = eng_resolve(
                        t,
                        AT_FDCWD,
                        &raw mut interp_0 as *mut ::core::ffi::c_char,
                        ENG_RES_FOLLOW,
                        &raw mut ires,
                    );
                    if rc != 0 {
                        sl_free(&raw mut pre);
                        return if rc == -ENOTDIR { -ENOENT } else { rc };
                    }
                    let mut ist: stat = stat {
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
                    let mut ifd: ::core::ffi::c_int =
                        open_candidate(t, &raw mut ires, &raw mut ist);
                    if ifd < 0 as ::core::ffi::c_int {
                        sl_free(&raw mut pre);
                        return ifd;
                    }
                    close(ifd);
                }
                let mut flags: uint32_t = if (*g).no_filemap != 0 {
                    ENG_PLAN_NO_FILEMAP as uint32_t
                } else {
                    0 as uint32_t
                };
                let mut em: eng_meta = eng_meta {
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
                    &raw mut res.host as *mut ::core::ffi::c_char,
                    0 as ::core::ffi::c_int,
                    &raw mut st,
                    &raw mut em,
                );
                let mut neu: uint32_t = 0;
                let mut neg: uint32_t = 0;
                (*t).plan_setid =
                    eng_ident_setid(t, em.mode, em.uid, em.gid, &raw mut neu, &raw mut neg);
                (*t).plan_euid = neu;
                (*t).plan_egid = neg;
                if neu != (*t).cr.ruid || neg != (*t).cr.rgid {
                    flags = (flags as ::core::ffi::c_uint | ENG_PLAN_SECURE) as uint32_t;
                }
                if (*g).test_pagesz != 0 {
                    flags = (flags as ::core::ffi::c_uint | ENG_PLAN_PAGESZ) as uint32_t;
                }
                rc = build_plan(
                    t,
                    &raw mut res.host as *mut ::core::ffi::c_char,
                    if hi != 0 {
                        &raw mut ires.host as *mut ::core::ffi::c_char
                    } else {
                        ::core::ptr::null_mut::<::core::ffi::c_char>()
                    },
                    &raw mut execfn as *mut ::core::ffi::c_char,
                    &raw mut comm as *mut ::core::ffi::c_char,
                    skip,
                    &raw mut pre,
                    flags,
                );
                if rc == 0 as ::core::ffi::c_int {
                    let mut pl: *mut eng_load_plan = (*t).plan as *mut eng_load_plan;
                    (*pl).pagesz = (*g).test_pagesz as uint32_t;
                    (*pl).euid = neu;
                    (*pl).egid = neg;
                }
                sl_free(&raw mut pre);
                if rc != 0 {
                    return rc;
                }
                snprintf(
                    &raw mut (*t).plan_exe as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                    b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                    &raw mut res.guest as *mut ::core::ffi::c_char,
                );
                return 0 as ::core::ffi::c_int;
            }
            close(fd);
            sl_free(&raw mut pre);
            return -ENOEXEC;
        }
        depth += 1;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_exec_prepare(
    mut t: *mut eng_task,
    mut dirfd: ::core::ffi::c_int,
    mut path: *const ::core::ffi::c_char,
    mut at_flags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return prepare(t, dirfd, path, at_flags);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_exec_prepare_initial(
    mut t: *mut eng_task,
    mut path: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    return prepare(t, AT_FDCWD, path, 0 as ::core::ffi::c_int);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_exec_discard(mut t: *mut eng_task) {
    free((*t).plan);
    (*t).plan = NULL;
    (*t).plan_len = 0 as uint32_t;
}
