//! Per-thread tracee scratch slots for translated arguments.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
#[repr(C)]
pub struct eng_tracer {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    unsafe fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    unsafe fn eng_mem_write(
        tid: pid_t,
        addr: uintptr_t,
        buf: *const ::core::ffi::c_void,
        n: size_t,
    ) -> ssize_t;
    unsafe fn eng_sp(r: *const eng_regs) -> uint64_t;
}
pub type __pid_t = ::core::ffi::c_int;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type uint64_t = u64;
pub type uintptr_t = usize;
pub type size_t = usize;
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
pub type pid_t = __pid_t;
pub type ssize_t = isize;
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
pub const ENG_SLOT_SIZE: ::core::ffi::c_uint =
    (16 as ::core::ffi::c_uint).wrapping_mul(1024 as ::core::ffi::c_uint);
pub const RED_ZONE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_scratch_alloc(
    mut t: *mut eng_task,
    mut r: *const eng_regs,
    mut n: size_t,
) -> uint64_t {
    n = n.wrapping_add(15 as size_t) & !(15 as ::core::ffi::c_int as size_t);
    let mut mm: *mut eng_mm = (*t).mm;
    if !mm.is_null() && (*mm).base != 0 && n <= ENG_SLOT_SIZE as size_t {
        if (*t).slot < 0 as ::core::ffi::c_int {
            let mut i: uint32_t = 0 as uint32_t;
            while i < (*mm).nslots {
                if (*mm).used[i as usize] == 0 {
                    (*mm).used[i as usize] = 1 as uint8_t;
                    (*t).slot = i as ::core::ffi::c_int;
                    break;
                } else {
                    i = i.wrapping_add(1);
                }
            }
        }
        if (*t).slot >= 0 as ::core::ffi::c_int
            && ((*t).slot_off as size_t).wrapping_add(n) <= ENG_SLOT_SIZE as size_t
        {
            let mut a: uint64_t = (*mm)
                .base
                .wrapping_add(((*t).slot as uint64_t).wrapping_mul(ENG_SLOT_SIZE as uint64_t))
                .wrapping_add((*t).slot_off as uint64_t);
            (*t).slot_off = (*t).slot_off.wrapping_add(n as uint32_t);
            return a;
        }
    }
    if (*t).stack_scratch == 0 {
        (*t).stack_scratch = eng_sp(r)
            .wrapping_sub(RED_ZONE as uint64_t)
            .wrapping_sub(64 as uint64_t)
            & !(15 as ::core::ffi::c_int as uint64_t);
    }
    (*t).stack_scratch =
        (*t).stack_scratch.wrapping_sub(n as uint64_t) & !(15 as ::core::ffi::c_int as uint64_t);
    static mut warned: ::core::ffi::c_int = 0;
    let c2rust_fresh0 = warned;
    warned += 1;
    if c2rust_fresh0 == 0 {
        eng_logf!(
            eng_log_level::ENG_LOG_DEBUG,
            b"scratch: using stack fallback for tid %d\0".as_ptr() as *const ::core::ffi::c_char,
            (*t).tid,
        );
    }
    return (*t).stack_scratch;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_scratch_put_str(
    mut t: *mut eng_task,
    mut r: *const eng_regs,
    mut s: *const ::core::ffi::c_char,
) -> uint64_t {
    let mut n: size_t = strlen(s).wrapping_add(1 as size_t);
    let mut a: uint64_t = eng_scratch_alloc(t, r, n);
    if a == 0 {
        return 0 as uint64_t;
    }
    if eng_mem_write((*t).tid, a as uintptr_t, s as *const ::core::ffi::c_void, n) != n as ssize_t {
        return 0 as uint64_t;
    }
    return a;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_scratch_release(mut t: *mut eng_task) {
    if !(*t).mm.is_null()
        && (*t).slot >= 0 as ::core::ffi::c_int
        && ((*t).slot as uint32_t) < (*(*t).mm).nslots
    {
        (*(*t).mm).used[(*t).slot as usize] = 0 as uint8_t;
    }
    (*t).slot = -1 as ::core::ffi::c_int;
    (*t).slot_off = 0 as uint32_t;
}
