//! ELF and shebang planning; real exec into the freestanding Rust loader.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    unsafe fn close(_: i32) -> i32;
    unsafe fn read(_: i32, _: *mut ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn pread(_: i32, _: *mut ::core::ffi::c_void, _: usize, _: i64) -> isize;
    unsafe fn eng_tracer_guest(_: *mut eng_tracer) -> *mut eng_guest;
    #[cfg(target_os = "linux")]
    #[link_name = "__errno_location"]
    unsafe fn errno() -> *mut i32;
    #[cfg(target_os = "android")]
    #[link_name = "__errno"]
    unsafe fn errno() -> *mut i32;
    unsafe fn open(_: *const ::core::ffi::c_char, _: i32, ...) -> i32;
    unsafe fn snprintf(
        _: *mut ::core::ffi::c_char,
        _: usize,
        _: *const ::core::ffi::c_char,
        ...
    ) -> i32;
    unsafe fn strtoul(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
        _: i32,
    ) -> u64;
    unsafe fn calloc(_: usize, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn free(_: *mut ::core::ffi::c_void);
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: usize,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memmove(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: usize,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn memset(_: *mut ::core::ffi::c_void, _: i32, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn memcmp(_: *const ::core::ffi::c_void, _: *const ::core::ffi::c_void, _: usize)
    -> i32;
    unsafe fn memchr(_: *const ::core::ffi::c_void, _: i32, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn strcmp(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn strdup(_: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn strrchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strstr(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strlen(_: *const ::core::ffi::c_char) -> usize;
    unsafe fn fstat(_: i32, _: *mut stat) -> i32;
    unsafe fn lstat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
    unsafe fn eng_guest_to_host(
        _: *const eng_guest,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn eng_ident_setid(
        _: *const eng_task,
        _: u32,
        _: u32,
        _: u32,
        _: *mut u32,
        _: *mut u32,
    ) -> i32;
    unsafe fn eng_meta_get(
        _: *mut eng_guest,
        _: *const ::core::ffi::c_char,
        _: i32,
        _: *const stat,
        _: *mut eng_meta,
    ) -> i32;
    unsafe fn eng_meta_is_placeholder(_: *const eng_meta, _: u32) -> i32;
    unsafe fn eng_meta_may_exec(
        _: *mut eng_task,
        _: *const ::core::ffi::c_char,
        _: *const stat,
    ) -> i32;
    unsafe fn eng_resolve(
        _: *mut eng_task,
        _: i32,
        _: *const ::core::ffi::c_char,
        _: i32,
        _: *mut eng_resolved,
    ) -> i32;
    unsafe fn eng_task_fd_path(
        _: *mut eng_task,
        _: i32,
        _: i32,
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
pub struct strlist {
    pub s: [*mut ::core::ffi::c_char; 16],
    pub n: i32,
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_load_plan {
    pub magic: u32,
    pub version: u32,
    pub size: u32,
    pub flags: u32,
    pub exe_off: u32,
    pub interp_off: u32,
    pub execfn_off: u32,
    pub argv_skip: u32,
    pub n_prepend: u32,
    pub prepend_off: u32,
    pub uid: u32,
    pub euid: u32,
    pub gid: u32,
    pub egid: u32,
    pub pagesz: u32,
    pub comm_off: u32,
    pub reserved: [u32; 2],
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Elf64_Ehdr {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Elf64_Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}
pub const ELFMAG: [::core::ffi::c_char; 5] =
    unsafe { ::core::mem::transmute::<[u8; 5], [::core::ffi::c_char; 5]>(*b"\x7FELF\0") };
pub const SELFMAG: i32 = 4 as i32;
pub const EI_CLASS: i32 = 4 as i32;
pub const ELFCLASS64: i32 = 2 as i32;
pub const EI_DATA: i32 = 5 as i32;
pub const ELFDATA2LSB: i32 = 1 as i32;
pub const ET_EXEC: i32 = 2 as i32;
pub const ET_DYN: i32 = 3 as i32;
#[cfg(target_arch = "x86_64")]
pub const EM_X86_64: i32 = 62 as i32;
pub const PT_LOAD: i32 = 1 as i32;
pub const PT_INTERP: i32 = 3 as i32;
pub const ENOENT: i32 = 2 as i32;
pub const E2BIG: i32 = 7 as i32;
pub const ENOEXEC: i32 = 8 as i32;
pub const ENOMEM: i32 = 12 as i32;
pub const EACCES: i32 = 13 as i32;
pub const ENOTDIR: i32 = 20 as i32;
pub const ENAMETOOLONG: i32 = 36 as i32;
pub const ELOOP: i32 = 40 as i32;
pub const O_RDONLY: i32 = 0 as i32;
#[cfg(target_os = "linux")]
pub const __O_CLOEXEC: i32 = 0o2000000 as i32;
#[cfg(target_os = "linux")]
pub const O_CLOEXEC: i32 = __O_CLOEXEC;
#[cfg(target_os = "android")]
pub const O_CLOEXEC: i32 = 0o2000000 as i32;
#[cfg(target_os = "linux")]
pub const __S_IFMT: i32 = 0o170000 as i32;
pub const AT_FDCWD: i32 = -100 as i32;
pub const AT_SYMLINK_NOFOLLOW: i32 = 0x100 as i32;
pub const AT_EMPTY_PATH: i32 = 0x1000 as i32;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ENG_PLAN_MAGIC: u32 = 0x4e414c50 as u32;
pub const ENG_PLAN_VERSION: u32 = 2 as u32;
pub const ENG_PLAN_MAX: u32 = (64 as u32).wrapping_mul(1024 as u32);
pub const ENG_PLAN_SECURE: u32 = 0x1 as u32;
pub const ENG_PLAN_NO_FILEMAP: u32 = 0x2 as u32;
pub const ENG_PLAN_PAGESZ: u32 = 0x4 as u32;
pub const ENG_RES_FOLLOW: i32 = 0x1 as i32;
#[cfg(target_arch = "x86_64")]
pub const EM_SELF: i32 = EM_X86_64;
#[cfg(target_arch = "aarch64")]
pub const EM_SELF: i32 = EM_AARCH64;
pub const MAX_SCRIPT_DEPTH: i32 = 5 as i32;
pub const MAX_PREPEND: i32 = 16 as i32;
pub const S_IFREG: i32 = 0o100000 as i32;
pub const S_IFLNK: i32 = 0o120000 as i32;
pub const S_IFMT: i32 = 0o170000 as i32;
unsafe extern "C" fn sl_free(mut l: *mut strlist) {
    let mut i: i32 = 0 as i32;
    while i < (*l).n {
        free((*l).s[i as usize] as *mut ::core::ffi::c_void);
        i += 1;
    }
    (*l).n = 0 as i32;
}
unsafe extern "C" fn sl_push_front(mut l: *mut strlist, mut s: *const ::core::ffi::c_char) -> i32 {
    if (*l).n >= MAX_PREPEND {
        return -E2BIG;
    }
    memmove(
        (&raw mut (*l).s as *mut *mut ::core::ffi::c_char).offset(1isize)
            as *mut ::core::ffi::c_void,
        (&raw mut (*l).s as *mut *mut ::core::ffi::c_char).offset(0isize)
            as *const ::core::ffi::c_void,
        ::core::mem::size_of::<*mut ::core::ffi::c_char>().wrapping_mul((*l).n as usize),
    );
    (*l).s[0usize] = strdup(s);
    (*l).n += 1;
    return 0 as i32;
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
            .wrapping_mul(((*l).n - 1 as i32) as usize),
    );
    (*l).n -= 1;
}
unsafe extern "C" fn parse_shebang(
    mut buf: *const ::core::ffi::c_char,
    mut n: usize,
    mut interp: *mut ::core::ffi::c_char,
    mut icap: usize,
    mut arg: *mut ::core::ffi::c_char,
    mut acap: usize,
    mut has_arg: *mut i32,
) -> i32 {
    let mut end: *const ::core::ffi::c_char =
        memchr(buf as *const ::core::ffi::c_void, '\n' as i32, n) as *const ::core::ffi::c_char;
    let mut truncated: i32 = end.is_null() as i32;
    if end.is_null() {
        end = buf.offset(n as isize);
    }
    let mut p: *const ::core::ffi::c_char = buf.offset(2 as i32 as isize);
    while p < end && (*p as i32 == ' ' as i32 || *p as i32 == '\t' as i32) {
        p = p.offset(1);
    }
    let mut e: *const ::core::ffi::c_char = end;
    while e > p
        && (*e.offset(-1isize) as i32 == ' ' as i32
            || *e.offset(-1isize) as i32 == '\t' as i32
            || *e.offset(-1isize) as i32 == '\r' as i32
            || *e.offset(-1isize) as i32 == 0 as i32)
    {
        e = e.offset(-1);
    }
    if p >= e {
        return -ENOEXEC;
    }
    let mut ie: *const ::core::ffi::c_char = p;
    while ie < e && *ie as i32 != ' ' as i32 && *ie as i32 != '\t' as i32 && *ie as i32 != 0 {
        ie = ie.offset(1);
    }
    if truncated != 0 && ie == e {
        return -ENOEXEC;
    }
    let mut il: usize = ie.offset_from(p) as usize;
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
    while a < e && (*a as i32 == ' ' as i32 || *a as i32 == '\t' as i32) {
        a = a.offset(1);
    }
    *has_arg = (a < e) as i32;
    if *has_arg != 0 {
        let mut al: usize = e.offset_from(a) as usize;
        if al >= acap {
            al = acap.wrapping_sub(1 as usize);
        }
        memcpy(
            arg as *mut ::core::ffi::c_void,
            a as *const ::core::ffi::c_void,
            al,
        );
        *arg.offset(al as isize) = 0 as ::core::ffi::c_char;
    }
    return 0 as i32;
}
unsafe extern "C" fn elf_interp(
    mut fd: i32,
    mut eh: *const Elf64_Ehdr,
    mut interp: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    if (*eh).e_phentsize as usize != ::core::mem::size_of::<Elf64_Phdr>()
        || (*eh).e_phnum as i32 == 0 as i32
        || (*eh).e_phnum as i32 > 256 as i32
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
    let mut sz: usize = ((*eh).e_phnum as usize).wrapping_mul(::core::mem::size_of::<Elf64_Phdr>());
    if pread(
        fd,
        &raw mut ph as *mut Elf64_Phdr as *mut ::core::ffi::c_void,
        sz,
        (*eh).e_phoff as i64,
    ) != sz as isize
    {
        return -ENOEXEC;
    }
    let mut loads: i32 = 0 as i32;
    let mut i: i32 = 0 as i32;
    while i < (*eh).e_phnum as i32 {
        if ph[i as usize].p_type == PT_LOAD as u32 {
            loads += 1;
        }
        if ph[i as usize].p_type != PT_INTERP as u32 {
            i += 1;
        } else {
            if ph[i as usize].p_filesz < 2 as u64 || ph[i as usize].p_filesz >= cap as u64 {
                return -ENOEXEC;
            }
            if pread(
                fd,
                interp as *mut ::core::ffi::c_void,
                ph[i as usize].p_filesz as usize,
                ph[i as usize].p_offset as i64,
            ) != ph[i as usize].p_filesz as isize
            {
                return -ENOEXEC;
            }
            *interp.offset(ph[i as usize].p_filesz as isize) = 0 as ::core::ffi::c_char;
            if *interp.offset(ph[i as usize].p_filesz.wrapping_sub(1 as u64) as isize) as i32
                != 0 as i32
            {
                return -ENOEXEC;
            }
            return 1 as i32;
        }
    }
    return if loads != 0 { 0 as i32 } else { -ENOEXEC };
}
unsafe extern "C" fn open_candidate(
    mut t: *mut eng_task,
    mut res: *const eng_resolved,
    mut st: *mut stat,
) -> i32 {
    let mut fd: i32 = open(
        &raw const (*res).host as *const ::core::ffi::c_char,
        O_RDONLY | O_CLOEXEC,
    );
    if fd < 0 as i32 {
        return -*errno();
    }
    if fstat(fd, st) != 0 as i32 {
        let mut e: i32 = *errno();
        close(fd);
        return -e;
    }
    if !((*st).st_mode & S_IFMT as u32 == S_IFREG as u32) {
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
        0 as i32,
        st,
        &raw mut vm,
    );
    if eng_meta_is_placeholder(&raw mut vm, (*st).st_mode) != 0 {
        close(fd);
        return -EACCES;
    }
    let mut rc: i32 =
        eng_meta_may_exec(t, &raw const (*res).host as *const ::core::ffi::c_char, st);
    if rc != 0 {
        close(fd);
        return rc;
    }
    return fd;
}
unsafe extern "C" fn match_binfmt(
    mut g: *const eng_guest,
    mut buf: *const u8,
    mut n: usize,
    mut filename: *const ::core::ffi::c_char,
) -> *const eng_binfmt {
    let mut i: i32 = (*g).nbinfmt - 1 as i32;
    while i >= 0 as i32 {
        let mut b: *const eng_binfmt =
            (&raw const (*g).binfmt as *const eng_binfmt).offset(i as isize);
        if (*b).r#type as i32 == 'E' as i32 {
            let mut base: *const ::core::ffi::c_char = strrchr(filename, '/' as i32);
            base = if !base.is_null() {
                base.offset(1 as i32 as isize)
            } else {
                filename
            };
            let mut dot: *const ::core::ffi::c_char = strrchr(base, '.' as i32);
            if !dot.is_null()
                && strcmp(
                    dot.offset(1 as i32 as isize),
                    &raw const (*b).ext as *const ::core::ffi::c_char,
                ) == 0
            {
                return b;
            }
        } else if (*b).offset.wrapping_add((*b).len) as usize <= n {
            let mut k: u32 = 0 as u32;
            while k < (*b).len
                && *buf.offset((*b).offset.wrapping_add(k) as isize) as i32
                    & (*b).mask[k as usize] as i32
                    == (*b).magic[k as usize] as i32
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
    mut skip: u32,
    mut pre: *const strlist,
    mut flags: u32,
) -> i32 {
    let mut need: usize = ::core::mem::size_of::<eng_load_plan>()
        .wrapping_add(strlen(exe_host))
        .wrapping_add(1 as usize)
        .wrapping_add(if !interp_host.is_null() {
            strlen(interp_host).wrapping_add(1 as usize)
        } else {
            0 as usize
        })
        .wrapping_add(strlen(execfn))
        .wrapping_add(1 as usize)
        .wrapping_add(strlen(comm))
        .wrapping_add(1 as usize);
    let mut i: i32 = 0 as i32;
    while i < (*pre).n {
        need = need.wrapping_add(strlen((*pre).s[i as usize]).wrapping_add(1 as usize));
        i += 1;
    }
    if need > ENG_PLAN_MAX as usize {
        return -E2BIG;
    }
    let mut buf: *mut ::core::ffi::c_char = calloc(1 as usize, need) as *mut ::core::ffi::c_char;
    if buf.is_null() {
        return -ENOMEM;
    }
    let mut pl: *mut eng_load_plan = buf as *mut eng_load_plan;
    let mut off: usize = ::core::mem::size_of::<eng_load_plan>();
    let mut _l: usize = strlen(exe_host).wrapping_add(1 as usize);
    memcpy(
        buf.offset(off as isize) as *mut ::core::ffi::c_void,
        exe_host as *const ::core::ffi::c_void,
        _l,
    );
    (*pl).exe_off = off as u32;
    off = off.wrapping_add(_l);
    if !interp_host.is_null() {
        let mut _l_0: usize = strlen(interp_host).wrapping_add(1 as usize);
        memcpy(
            buf.offset(off as isize) as *mut ::core::ffi::c_void,
            interp_host as *const ::core::ffi::c_void,
            _l_0,
        );
        (*pl).interp_off = off as u32;
        off = off.wrapping_add(_l_0);
    }
    let mut _l_1: usize = strlen(execfn).wrapping_add(1 as usize);
    memcpy(
        buf.offset(off as isize) as *mut ::core::ffi::c_void,
        execfn as *const ::core::ffi::c_void,
        _l_1,
    );
    (*pl).execfn_off = off as u32;
    off = off.wrapping_add(_l_1);
    let mut _l_2: usize = strlen(comm).wrapping_add(1 as usize);
    memcpy(
        buf.offset(off as isize) as *mut ::core::ffi::c_void,
        comm as *const ::core::ffi::c_void,
        _l_2,
    );
    (*pl).comm_off = off as u32;
    off = off.wrapping_add(_l_2);
    if (*pre).n != 0 {
        (*pl).prepend_off = off as u32;
        let mut i_0: i32 = 0 as i32;
        while i_0 < (*pre).n {
            let mut l: usize = strlen((*pre).s[i_0 as usize]).wrapping_add(1 as usize);
            memcpy(
                buf.offset(off as isize) as *mut ::core::ffi::c_void,
                (*pre).s[i_0 as usize] as *const ::core::ffi::c_void,
                l,
            );
            off = off.wrapping_add(l);
            i_0 += 1;
        }
    }
    (*pl).magic = ENG_PLAN_MAGIC as u32;
    (*pl).version = ENG_PLAN_VERSION as u32;
    (*pl).size = off as u32;
    (*pl).flags = flags;
    (*pl).argv_skip = skip;
    (*pl).n_prepend = (*pre).n as u32;
    (*pl).uid = (*t).cr.ruid;
    (*pl).euid = (*t).cr.euid;
    (*pl).gid = (*t).cr.rgid;
    (*pl).egid = (*t).cr.egid;
    free((*t).plan);
    (*t).plan = buf as *mut ::core::ffi::c_void;
    (*t).plan_len = off as u32;
    return 0 as i32;
}
unsafe extern "C" fn prepare(
    mut t: *mut eng_task,
    mut dirfd: i32,
    mut path: *const ::core::ffi::c_char,
    mut at_flags: i32,
) -> i32 {
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
    let mut rc: i32 = 0;
    if at_flags & AT_EMPTY_PATH != 0 && *path.offset(0isize) as i32 == 0 as i32 {
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
            0 as i32,
            dirfd,
            &raw mut gp as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        );
        memset(
            &raw mut res as *mut ::core::ffi::c_void,
            0 as i32,
            ::core::mem::size_of::<eng_resolved>(),
        );
        if rc == 0 as i32 {
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
        if *path.offset(0isize) as i32 == '/' as i32 || dirfd == AT_FDCWD {
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
        let mut fl: i32 = if at_flags & AT_SYMLINK_NOFOLLOW != 0 {
            0 as i32
        } else {
            ENG_RES_FOLLOW
        };
        rc = eng_resolve(t, dirfd, path, fl, &raw mut res);
        if rc != 0 {
            return rc;
        }
        let mut lst: stat = platform_empty_stat();
        if at_flags & AT_SYMLINK_NOFOLLOW != 0
            && lstat(&raw mut res.host as *mut ::core::ffi::c_char, &raw mut lst) == 0 as i32
            && lst.st_mode & S_IFMT as u32 == S_IFLNK as u32
        {
            return -ELOOP;
        }
    }
    let mut comm: [::core::ffi::c_char; 16] = [0; 16];
    let mut src: *const ::core::ffi::c_char =
        if at_flags & AT_EMPTY_PATH != 0 && *path.offset(0isize) as i32 == 0 as i32 {
            &raw mut res.guest as *mut ::core::ffi::c_char as *const ::core::ffi::c_char
        } else {
            path
        };
    let mut b: *const ::core::ffi::c_char = strrchr(src, '/' as i32);
    b = if !b.is_null() && *b.offset(1isize) as i32 != 0 {
        b.offset(1 as i32 as isize)
    } else {
        src
    };
    snprintf(
        &raw mut comm as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 16]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        b,
    );
    let mut path_inaccessible: i32 = 0 as i32;
    if dirfd != AT_FDCWD && *path.offset(0isize) as i32 != '/' as i32 {
        let mut fi: [::core::ffi::c_char; 64] = [0; 64];
        let mut buf: [::core::ffi::c_char; 512] = [0; 512];
        snprintf(
            &raw mut fi as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>(),
            b"/proc/%d/fdinfo/%d\0".as_ptr() as *const ::core::ffi::c_char,
            (*t).tid,
            dirfd,
        );
        let mut ff: i32 = open(
            &raw mut fi as *mut ::core::ffi::c_char,
            O_RDONLY | O_CLOEXEC,
        );
        if ff >= 0 as i32 {
            let mut k: isize = read(
                ff,
                &raw mut buf as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_char; 512]>().wrapping_sub(1 as usize),
            );
            close(ff);
            if k > 0 as isize {
                buf[k as usize] = 0 as ::core::ffi::c_char;
                let mut fl_0: *mut ::core::ffi::c_char = strstr(
                    &raw mut buf as *mut ::core::ffi::c_char,
                    b"flags:\0".as_ptr() as *const ::core::ffi::c_char,
                );
                if !fl_0.is_null()
                    && strtoul(
                        fl_0.offset(6 as i32 as isize),
                        ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                        8 as i32,
                    ) & O_CLOEXEC as u64
                        != 0
                {
                    path_inaccessible = 1 as i32;
                }
            }
        }
    }
    let mut pre: strlist = strlist {
        s: [::core::ptr::null_mut::<::core::ffi::c_char>(); 16],
        n: 0 as i32,
    };
    let mut skip: u32 = 0 as u32;
    let mut filename: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut filename as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut execfn as *mut ::core::ffi::c_char,
    );
    let mut depth: i32 = 0 as i32;
    loop {
        let mut st: stat = platform_empty_stat();
        let mut fd: i32 = open_candidate(t, &raw mut res, &raw mut st);
        if fd < 0 as i32 {
            sl_free(&raw mut pre);
            return fd;
        }
        let mut buf_0: [u8; 256] = [0; 256];
        let mut n: isize = pread(
            fd,
            &raw mut buf_0 as *mut u8 as *mut ::core::ffi::c_void,
            ::core::mem::size_of::<[u8; 256]>(),
            0 as i64,
        );
        if n < 0 as isize {
            let mut e: i32 = *errno();
            close(fd);
            sl_free(&raw mut pre);
            return -e;
        }
        let mut bf: *const eng_binfmt = match_binfmt(
            g,
            &raw mut buf_0 as *mut u8,
            n as usize,
            &raw mut filename as *mut ::core::ffi::c_char,
        );
        if !bf.is_null() {
            close(fd);
            if depth >= MAX_SCRIPT_DEPTH {
                sl_free(&raw mut pre);
                return -ELOOP;
            }
            if depth == 0 as i32 && path_inaccessible != 0 {
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
        } else if n >= 2 as isize
            && buf_0[0usize] as i32 == '#' as i32
            && buf_0[1usize] as i32 == '!' as i32
        {
            close(fd);
            if depth >= MAX_SCRIPT_DEPTH {
                sl_free(&raw mut pre);
                return -ELOOP;
            }
            if depth == 0 as i32 && path_inaccessible != 0 {
                sl_free(&raw mut pre);
                return -ENOENT;
            }
            let mut interp: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut arg: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut has_arg: i32 = 0 as i32;
            rc = parse_shebang(
                &raw mut buf_0 as *mut u8 as *const ::core::ffi::c_char,
                n as usize,
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
            if n >= ::core::mem::size_of::<Elf64_Ehdr>() as isize
                && memcmp(
                    &raw mut buf_0 as *mut u8 as *const ::core::ffi::c_void,
                    ELFMAG.as_ptr() as *const ::core::ffi::c_void,
                    SELFMAG as usize,
                ) == 0
            {
                let mut eh: *const Elf64_Ehdr = &raw mut buf_0 as *mut u8 as *const Elf64_Ehdr;
                let eh_value = eh.read_unaligned();
                let eh = &eh_value as *const Elf64_Ehdr;
                if (*eh).e_ident[EI_CLASS as usize] as i32 != ELFCLASS64
                    || (*eh).e_ident[EI_DATA as usize] as i32 != ELFDATA2LSB
                    || (*eh).e_machine as i32 != EM_SELF
                    || (*eh).e_type as i32 != ET_EXEC && (*eh).e_type as i32 != ET_DYN
                {
                    close(fd);
                    sl_free(&raw mut pre);
                    return -ENOEXEC;
                }
                let mut interp_0: [::core::ffi::c_char; 4096] = [0; 4096];
                let mut hi: i32 = elf_interp(
                    fd,
                    eh,
                    &raw mut interp_0 as *mut ::core::ffi::c_char,
                    ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                );
                close(fd);
                if hi < 0 as i32 {
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
                    let mut ist: stat = platform_empty_stat();
                    let mut ifd: i32 = open_candidate(t, &raw mut ires, &raw mut ist);
                    if ifd < 0 as i32 {
                        sl_free(&raw mut pre);
                        return ifd;
                    }
                    close(ifd);
                }
                let mut flags: u32 = if (*g).no_filemap != 0 {
                    ENG_PLAN_NO_FILEMAP as u32
                } else {
                    0 as u32
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
                    0 as i32,
                    &raw mut st,
                    &raw mut em,
                );
                let mut neu: u32 = 0;
                let mut neg: u32 = 0;
                (*t).plan_setid =
                    eng_ident_setid(t, em.mode, em.uid, em.gid, &raw mut neu, &raw mut neg);
                (*t).plan_euid = neu;
                (*t).plan_egid = neg;
                if neu != (*t).cr.ruid || neg != (*t).cr.rgid {
                    flags = (flags as u32 | ENG_PLAN_SECURE) as u32;
                }
                if (*g).test_pagesz != 0 {
                    flags = (flags as u32 | ENG_PLAN_PAGESZ) as u32;
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
                if rc == 0 as i32 {
                    let mut pl: *mut eng_load_plan = (*t).plan as *mut eng_load_plan;
                    (*pl).pagesz = (*g).test_pagesz as u32;
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
                return 0 as i32;
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
    mut dirfd: i32,
    mut path: *const ::core::ffi::c_char,
    mut at_flags: i32,
) -> i32 {
    return prepare(t, dirfd, path, at_flags);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_exec_prepare_initial(
    mut t: *mut eng_task,
    mut path: *const ::core::ffi::c_char,
) -> i32 {
    return prepare(t, AT_FDCWD, path, 0 as i32);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_exec_discard(mut t: *mut eng_task) {
    free((*t).plan);
    (*t).plan = NULL;
    (*t).plan_len = 0 as u32;
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
pub type Elf64_Ehdr = elf64_hdr;
#[cfg(target_os = "android")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct elf64_hdr {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}
#[cfg(target_os = "android")]
pub type Elf64_Phdr = elf64_phdr;
#[cfg(target_os = "android")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct elf64_phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}
#[cfg(target_arch = "aarch64")]
pub const EM_AARCH64: i32 = 183 as i32;
