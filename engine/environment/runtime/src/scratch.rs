//! Per-thread tracee scratch slots for translated arguments.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    unsafe fn strlen(_: *const ::core::ffi::c_char) -> usize;
    unsafe fn eng_mem_write(_: i32, _: usize, _: *const ::core::ffi::c_void, _: usize) -> isize;
    unsafe fn eng_sp(_: *const eng_regs) -> u64;
}
#[repr(C)]
pub struct eng_tracer {
    _opaque: [u8; 0],
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
pub const ENG_SLOT_SIZE: u32 = (16 as u32).wrapping_mul(1024 as u32);
#[cfg(target_arch = "x86_64")]
pub const RED_ZONE: i32 = 128 as i32;
#[cfg(target_arch = "aarch64")]
pub const RED_ZONE: i32 = 0 as i32;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_scratch_alloc(
    mut t: *mut eng_task,
    mut r: *const eng_regs,
    mut n: usize,
) -> u64 {
    n = n.wrapping_add(15 as usize) & !(15 as i32 as usize);
    let mut mm: *mut eng_mm = (*t).mm;
    if !mm.is_null() && (*mm).base != 0 && n <= ENG_SLOT_SIZE as usize {
        if (*t).slot < 0 as i32 {
            let mut i: u32 = 0 as u32;
            while i < (*mm).nslots {
                if (*mm).used[i as usize] == 0 {
                    (*mm).used[i as usize] = 1 as u8;
                    (*t).slot = i as i32;
                    break;
                } else {
                    i = i.wrapping_add(1);
                }
            }
        }
        if (*t).slot >= 0 as i32
            && ((*t).slot_off as usize).wrapping_add(n) <= ENG_SLOT_SIZE as usize
        {
            let mut a: u64 = (*mm)
                .base
                .wrapping_add(((*t).slot as u64).wrapping_mul(ENG_SLOT_SIZE as u64))
                .wrapping_add((*t).slot_off as u64);
            (*t).slot_off = (*t).slot_off.wrapping_add(n as u32);
            return a;
        }
    }
    if (*t).stack_scratch == 0 {
        (*t).stack_scratch = eng_sp(r)
            .wrapping_sub(RED_ZONE as u64)
            .wrapping_sub(64 as u64)
            & !(15 as i32 as u64);
    }
    (*t).stack_scratch = (*t).stack_scratch.wrapping_sub(n as u64) & !(15 as i32 as u64);
    static mut warned: i32 = 0;
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
) -> u64 {
    let mut n: usize = strlen(s).wrapping_add(1 as usize);
    let mut a: u64 = eng_scratch_alloc(t, r, n);
    if a == 0 {
        return 0 as u64;
    }
    if eng_mem_write((*t).tid, a as usize, s as *const ::core::ffi::c_void, n) != n as isize {
        return 0 as u64;
    }
    return a;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_scratch_release(mut t: *mut eng_task) {
    if !(*t).mm.is_null() && (*t).slot >= 0 as i32 && ((*t).slot as u32) < (*(*t).mm).nslots {
        (*(*t).mm).used[(*t).slot as usize] = 0 as u8;
    }
    (*t).slot = -1 as i32;
    (*t).slot_off = 0 as u32;
}
