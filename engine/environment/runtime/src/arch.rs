//! Register sets and syscall register access for the selected kernel ABI.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    #[cfg(target_os = "linux")]
    unsafe fn ptrace(_: u32, ...) -> i64;
    #[cfg(target_os = "android")]
    unsafe fn ptrace(_: i32, ...) -> i64;
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
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct iovec {
    pub iov_base: *mut ::core::ffi::c_void,
    pub iov_len: usize,
}
#[cfg(target_os = "android")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct iovec {
    pub iov_base: *mut ::core::ffi::c_void,
    pub iov_len: u64,
}
#[cfg(target_os = "linux")]
pub const PTRACE_TRACEME: u32 = 0;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKTEXT: u32 = 1;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKDATA: u32 = 2;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKUSER: u32 = 3;
#[cfg(target_os = "linux")]
pub const PTRACE_POKETEXT: u32 = 4;
#[cfg(target_os = "linux")]
pub const PTRACE_POKEDATA: u32 = 5;
#[cfg(target_os = "linux")]
pub const PTRACE_POKEUSER: u32 = 6;
#[cfg(target_os = "linux")]
pub const PTRACE_CONT: u32 = 7;
#[cfg(target_os = "linux")]
pub const PTRACE_KILL: u32 = 8;
#[cfg(target_os = "linux")]
pub const PTRACE_SINGLESTEP: u32 = 9;
#[cfg(target_os = "linux")]
pub const PTRACE_GETREGS: u32 = 12;
#[cfg(target_os = "linux")]
pub const PTRACE_SETREGS: u32 = 13;
#[cfg(target_os = "linux")]
pub const PTRACE_GETFPREGS: u32 = 14;
#[cfg(target_os = "linux")]
pub const PTRACE_SETFPREGS: u32 = 15;
#[cfg(target_os = "linux")]
pub const PTRACE_ATTACH: u32 = 16;
#[cfg(target_os = "linux")]
pub const PTRACE_DETACH: u32 = 17;
#[cfg(target_os = "linux")]
pub const PTRACE_GETFPXREGS: u32 = 18;
#[cfg(target_os = "linux")]
pub const PTRACE_SETFPXREGS: u32 = 19;
#[cfg(target_os = "linux")]
pub const PTRACE_SYSCALL: u32 = 24;
#[cfg(target_os = "linux")]
pub const PTRACE_GET_THREAD_AREA: u32 = 25;
#[cfg(target_os = "linux")]
pub const PTRACE_SET_THREAD_AREA: u32 = 26;
#[cfg(target_os = "linux")]
pub const PTRACE_ARCH_PRCTL: u32 = 30;
#[cfg(target_os = "linux")]
pub const PTRACE_SYSEMU: u32 = 31;
#[cfg(target_os = "linux")]
pub const PTRACE_SYSEMU_SINGLESTEP: u32 = 32;
#[cfg(target_os = "linux")]
pub const PTRACE_SINGLEBLOCK: u32 = 33;
#[cfg(target_os = "linux")]
pub const PTRACE_SETOPTIONS: u32 = 16896;
#[cfg(target_os = "linux")]
pub const PTRACE_GETEVENTMSG: u32 = 16897;
#[cfg(target_os = "linux")]
pub const PTRACE_GETSIGINFO: u32 = 16898;
#[cfg(target_os = "linux")]
pub const PTRACE_SETSIGINFO: u32 = 16899;
#[cfg(target_os = "linux")]
pub const PTRACE_GETREGSET: u32 = 16900;
#[cfg(target_os = "android")]
pub const PTRACE_GETREGSET: i32 = 0x4204 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_SETREGSET: u32 = 16901;
#[cfg(target_os = "android")]
pub const PTRACE_SETREGSET: i32 = 0x4205 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_SEIZE: u32 = 16902;
#[cfg(target_os = "linux")]
pub const PTRACE_INTERRUPT: u32 = 16903;
#[cfg(target_os = "linux")]
pub const PTRACE_LISTEN: u32 = 16904;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKSIGINFO: u32 = 16905;
#[cfg(target_os = "linux")]
pub const PTRACE_GETSIGMASK: u32 = 16906;
#[cfg(target_os = "linux")]
pub const PTRACE_SETSIGMASK: u32 = 16907;
#[cfg(target_os = "linux")]
pub const PTRACE_SECCOMP_GET_FILTER: u32 = 16908;
#[cfg(target_os = "linux")]
pub const PTRACE_SECCOMP_GET_METADATA: u32 = 16909;
#[cfg(target_os = "linux")]
pub const PTRACE_GET_SYSCALL_INFO: u32 = 16910;
#[cfg(target_os = "android")]
pub const PTRACE_GET_SYSCALL_INFO: i32 = 0x420e as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_GET_RSEQ_CONFIGURATION: u32 = 16911;
#[cfg(target_os = "linux")]
pub const PTRACE_SET_SYSCALL_USER_DISPATCH_CONFIG: u32 = 16912;
#[cfg(target_os = "linux")]
pub const PTRACE_GET_SYSCALL_USER_DISPATCH_CONFIG: u32 = 16913;
pub const NT_PRSTATUS: i32 = 1 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_getpid: i32 = 39 as i32;
#[cfg(target_arch = "aarch64")]
pub const __NR_getpid: i32 = 172 as i32;
#[cfg(target_arch = "x86_64")]
pub const __NR_execve: i64 = 59 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_execve: i64 = 221 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_openat: i64 = 257 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_openat: i64 = 56 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_newfstatat: i64 = 262 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_newfstatat: i64 = 79 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_faccessat: i64 = 269 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_faccessat: i64 = 48 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_execveat: i64 = 322 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_execveat: i64 = 281 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_statx: i64 = 332 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_statx: i64 = 291 as i64;
#[cfg(target_arch = "x86_64")]
pub const __NR_rseq: i64 = 334 as i64;
#[cfg(target_arch = "aarch64")]
pub const __NR_rseq: i64 = 293 as i64;
pub const __NR_clone3: i64 = 435 as i64;
pub const __NR_close_range: i64 = 436 as i64;
pub const __NR_openat2: i64 = 437 as i64;
pub const __NR_faccessat2: i64 = 439 as i64;
pub const __NR_fchmodat2: i64 = 452 as i64;
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_regs_get(mut tid: i32, mut r: *mut eng_regs) -> i32 {
    let mut io: iovec = iovec {
        iov_base: r as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<eng_regs>(),
    };
    return if ptrace(
        PTRACE_GETREGSET,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(NT_PRSTATUS as usize),
        &raw mut io,
    ) == 0 as i64
    {
        0 as i32
    } else {
        -1 as i32
    };
}
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_regs_get(mut tid: i32, mut r: *mut eng_regs) -> i32 {
    let mut io: iovec = iovec {
        iov_base: r as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<eng_regs>() as u64,
    };
    return if ptrace(
        PTRACE_GETREGSET,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(NT_PRSTATUS as usize),
        &raw mut io,
    ) == 0 as i64
    {
        0 as i32
    } else {
        -1 as i32
    };
}
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_regs_set(mut tid: i32, mut r: *const eng_regs) -> i32 {
    let mut io: iovec = iovec {
        iov_base: r as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<eng_regs>(),
    };
    return if ptrace(
        PTRACE_SETREGSET,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(NT_PRSTATUS as usize),
        &raw mut io,
    ) == 0 as i64
    {
        0 as i32
    } else {
        -1 as i32
    };
}
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_regs_set(mut tid: i32, mut r: *const eng_regs) -> i32 {
    let mut io: iovec = iovec {
        iov_base: r as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<eng_regs>() as u64,
    };
    return if ptrace(
        PTRACE_SETREGSET,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(NT_PRSTATUS as usize),
        &raw mut io,
    ) == 0 as i64
    {
        0 as i32
    } else {
        -1 as i32
    };
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sysno(mut r: *const eng_regs) -> i64 {
    return (*r).orig_rax as i64;
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sysno(mut r: *const eng_regs) -> i64 {
    return (*r).regs[8usize] as i64;
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_arg(mut r: *const eng_regs, mut i: i32) -> u64 {
    match i {
        0 => return (*r).rdi as u64,
        1 => return (*r).rsi as u64,
        2 => return (*r).rdx as u64,
        3 => return (*r).r10 as u64,
        4 => return (*r).r8 as u64,
        _ => return (*r).r9 as u64,
    };
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_arg(mut r: *const eng_regs, mut i: i32) -> u64 {
    return (*r).regs[i as usize];
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_arg(mut r: *mut eng_regs, mut i: i32, mut v: u64) {
    match i {
        0 => {
            (*r).rdi = v as u64;
        }
        1 => {
            (*r).rsi = v as u64;
        }
        2 => {
            (*r).rdx = v as u64;
        }
        3 => {
            (*r).r10 = v as u64;
        }
        4 => {
            (*r).r8 = v as u64;
        }
        _ => {
            (*r).r9 = v as u64;
        }
    };
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_arg(mut r: *mut eng_regs, mut i: i32, mut v: u64) {
    (*r).regs[i as usize] = v;
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ret(mut r: *const eng_regs) -> u64 {
    return (*r).rax as u64;
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ret(mut r: *const eng_regs) -> u64 {
    return (*r).regs[0usize];
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_ret(mut r: *mut eng_regs, mut v: u64) {
    (*r).rax = v as u64;
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_ret(mut r: *mut eng_regs, mut v: u64) {
    (*r).regs[0usize] = v;
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_pc(mut r: *const eng_regs) -> u64 {
    return (*r).rip as u64;
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_pc(mut r: *const eng_regs) -> u64 {
    return (*r).pc;
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_pc(mut r: *mut eng_regs, mut v: u64) {
    (*r).rip = v as u64;
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_pc(mut r: *mut eng_regs, mut v: u64) {
    (*r).pc = v;
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sp(mut r: *const eng_regs) -> u64 {
    return (*r).rsp as u64;
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sp(mut r: *const eng_regs) -> u64 {
    return (*r).sp;
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_sp(mut r: *mut eng_regs, mut v: u64) {
    (*r).rsp = v as u64;
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_sp(mut r: *mut eng_regs, mut v: u64) {
    (*r).sp = v;
}
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_void(mut tid: i32, mut r: *mut eng_regs) -> i32 {
    (*r).orig_rax = __NR_getpid as u64 as u64;
    return eng_regs_set(tid, r);
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_void(mut tid: i32, mut r: *mut eng_regs) -> i32 {
    (*r).orig_rax = __NR_getpid as u64;
    return eng_regs_set(tid, r);
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_void(mut tid: i32, mut r: *mut eng_regs) -> i32 {
    let mut nr: i32 = __NR_getpid;
    let mut io: iovec = iovec {
        iov_base: &raw mut nr as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<i32>() as u64,
    };
    return if ptrace(
        PTRACE_SETREGSET,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
            NT_ARM_SYSTEM_CALL as usize,
        ),
        &raw mut io,
    ) == 0 as i64
    {
        0 as i32
    } else {
        -1 as i32
    };
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_restore_args(mut cur: *mut eng_regs, mut orig: *const eng_regs) {
    (*cur).rdi = (*orig).rdi;
    (*cur).rsi = (*orig).rsi;
    (*cur).rdx = (*orig).rdx;
    (*cur).r10 = (*orig).r10;
    (*cur).r8 = (*orig).r8;
    (*cur).r9 = (*orig).r9;
    (*cur).orig_rax = (*orig).orig_rax;
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_restore_args(mut cur: *mut eng_regs, mut orig: *const eng_regs) {
    let mut i: i32 = 1 as i32;
    while i <= 5 as i32 {
        (*cur).regs[i as usize] = (*orig).regs[i as usize];
        i += 1;
    }
}
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_set(mut tid: i32, mut r: *mut eng_regs, mut nr: i64) -> i32 {
    (*r).orig_rax = nr as u64 as u64;
    return eng_regs_set(tid, r);
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_set(mut tid: i32, mut r: *mut eng_regs, mut nr: i64) -> i32 {
    (*r).orig_rax = nr as u64;
    return eng_regs_set(tid, r);
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_set(mut tid: i32, mut r: *mut eng_regs, mut nr: i64) -> i32 {
    let mut n: i32 = nr as i32;
    let mut io: iovec = iovec {
        iov_base: &raw mut n as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<i32>() as u64,
    };
    return if ptrace(
        PTRACE_SETREGSET,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
            NT_ARM_SYSTEM_CALL as usize,
        ),
        &raw mut io,
    ) == 0 as i64
    {
        0 as i32
    } else {
        -1 as i32
    };
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_insn_len() -> i32 {
    return 2 as i32;
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_insn_len() -> i32 {
    return 4 as i32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_info_op(mut tid: i32) -> i32 {
    let mut info: [u8; 96] = [0; 96];
    let mut n: i64 = ptrace(
        PTRACE_GET_SYSCALL_INFO,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(::core::mem::size_of::<
            [u8; 96],
        >() as usize),
        &raw mut info as *mut u8,
    );
    if n < 0 as i64 {
        return -1 as i32;
    }
    return info[0usize] as i32;
}
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sysname(mut nr: i64) -> *const ::core::ffi::c_char {
    match nr {
        __NR_openat => return b"openat\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_openat2 => return b"openat2\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_execve => return b"execve\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_execveat => return b"execveat\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_statx => return b"statx\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_newfstatat => return b"newfstatat\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_faccessat => return b"faccessat\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_faccessat2 => return b"faccessat2\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_clone3 => return b"clone3\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_close_range => {
            return b"close_range\0".as_ptr() as *const ::core::ffi::c_char;
        }
        __NR_fchmodat2 => return b"fchmodat2\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_rseq => return b"rseq\0".as_ptr() as *const ::core::ffi::c_char,
        39 => return b"getpid\0".as_ptr() as *const ::core::ffi::c_char,
        _ => return b"?\0".as_ptr() as *const ::core::ffi::c_char,
    };
}
#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sysname(mut nr: i64) -> *const ::core::ffi::c_char {
    match nr {
        __NR_openat => return b"openat\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_openat2 => return b"openat2\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_execve => return b"execve\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_execveat => return b"execveat\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_statx => return b"statx\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_newfstatat => return b"newfstatat\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_faccessat => return b"faccessat\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_faccessat2 => return b"faccessat2\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_clone3 => return b"clone3\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_close_range => {
            return b"close_range\0".as_ptr() as *const ::core::ffi::c_char;
        }
        __NR_fchmodat2 => return b"fchmodat2\0".as_ptr() as *const ::core::ffi::c_char,
        __NR_rseq => return b"rseq\0".as_ptr() as *const ::core::ffi::c_char,
        172 => return b"getpid\0".as_ptr() as *const ::core::ffi::c_char,
        _ => return b"?\0".as_ptr() as *const ::core::ffi::c_char,
    };
}
#[cfg(target_arch = "aarch64")]
pub const NT_ARM_SYSTEM_CALL: i32 = 0x404 as i32;
