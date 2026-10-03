//! Register sets and syscall register access for the selected kernel ABI.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
unsafe extern "C" {
    unsafe fn ptrace(__request: __ptrace_request, ...) -> ::core::ffi::c_long;
}
pub type __pid_t = ::core::ffi::c_int;
pub type uint64_t = u64;
pub type pid_t = __pid_t;
pub type size_t = usize;
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
pub struct iovec {
    pub iov_base: *mut ::core::ffi::c_void,
    pub iov_len: size_t,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct __ptrace_request(pub ::core::ffi::c_uint);
impl __ptrace_request {
    pub const PTRACE_TRACEME: Self = Self(0);
    pub const PTRACE_PEEKTEXT: Self = Self(1);
    pub const PTRACE_PEEKDATA: Self = Self(2);
    pub const PTRACE_PEEKUSER: Self = Self(3);
    pub const PTRACE_POKETEXT: Self = Self(4);
    pub const PTRACE_POKEDATA: Self = Self(5);
    pub const PTRACE_POKEUSER: Self = Self(6);
    pub const PTRACE_CONT: Self = Self(7);
    pub const PTRACE_KILL: Self = Self(8);
    pub const PTRACE_SINGLESTEP: Self = Self(9);
    pub const PTRACE_GETREGS: Self = Self(12);
    pub const PTRACE_SETREGS: Self = Self(13);
    pub const PTRACE_GETFPREGS: Self = Self(14);
    pub const PTRACE_SETFPREGS: Self = Self(15);
    pub const PTRACE_ATTACH: Self = Self(16);
    pub const PTRACE_DETACH: Self = Self(17);
    pub const PTRACE_GETFPXREGS: Self = Self(18);
    pub const PTRACE_SETFPXREGS: Self = Self(19);
    pub const PTRACE_SYSCALL: Self = Self(24);
    pub const PTRACE_GET_THREAD_AREA: Self = Self(25);
    pub const PTRACE_SET_THREAD_AREA: Self = Self(26);
    pub const PTRACE_ARCH_PRCTL: Self = Self(30);
    pub const PTRACE_SYSEMU: Self = Self(31);
    pub const PTRACE_SYSEMU_SINGLESTEP: Self = Self(32);
    pub const PTRACE_SINGLEBLOCK: Self = Self(33);
    pub const PTRACE_SETOPTIONS: Self = Self(16896);
    pub const PTRACE_GETEVENTMSG: Self = Self(16897);
    pub const PTRACE_GETSIGINFO: Self = Self(16898);
    pub const PTRACE_SETSIGINFO: Self = Self(16899);
    pub const PTRACE_GETREGSET: Self = Self(16900);
    pub const PTRACE_SETREGSET: Self = Self(16901);
    pub const PTRACE_SEIZE: Self = Self(16902);
    pub const PTRACE_INTERRUPT: Self = Self(16903);
    pub const PTRACE_LISTEN: Self = Self(16904);
    pub const PTRACE_PEEKSIGINFO: Self = Self(16905);
    pub const PTRACE_GETSIGMASK: Self = Self(16906);
    pub const PTRACE_SETSIGMASK: Self = Self(16907);
    pub const PTRACE_SECCOMP_GET_FILTER: Self = Self(16908);
    pub const PTRACE_SECCOMP_GET_METADATA: Self = Self(16909);
    pub const PTRACE_GET_SYSCALL_INFO: Self = Self(16910);
    pub const PTRACE_GET_RSEQ_CONFIGURATION: Self = Self(16911);
    pub const PTRACE_SET_SYSCALL_USER_DISPATCH_CONFIG: Self = Self(16912);
    pub const PTRACE_GET_SYSCALL_USER_DISPATCH_CONFIG: Self = Self(16913);
}
pub const NT_PRSTATUS: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const __NR_getpid: ::core::ffi::c_int = 39 as ::core::ffi::c_int;
pub const __NR_execve: ::core::ffi::c_long = 59 as ::core::ffi::c_long;
pub const __NR_openat: ::core::ffi::c_long = 257 as ::core::ffi::c_long;
pub const __NR_newfstatat: ::core::ffi::c_long = 262 as ::core::ffi::c_long;
pub const __NR_faccessat: ::core::ffi::c_long = 269 as ::core::ffi::c_long;
pub const __NR_execveat: ::core::ffi::c_long = 322 as ::core::ffi::c_long;
pub const __NR_statx: ::core::ffi::c_long = 332 as ::core::ffi::c_long;
pub const __NR_rseq: ::core::ffi::c_long = 334 as ::core::ffi::c_long;
pub const __NR_clone3: ::core::ffi::c_long = 435 as ::core::ffi::c_long;
pub const __NR_close_range: ::core::ffi::c_long = 436 as ::core::ffi::c_long;
pub const __NR_openat2: ::core::ffi::c_long = 437 as ::core::ffi::c_long;
pub const __NR_faccessat2: ::core::ffi::c_long = 439 as ::core::ffi::c_long;
pub const __NR_fchmodat2: ::core::ffi::c_long = 452 as ::core::ffi::c_long;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_regs_get(mut tid: pid_t, mut r: *mut eng_regs) -> ::core::ffi::c_int {
    let mut io: iovec = iovec {
        iov_base: r as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<eng_regs>(),
    };
    return if ptrace(
        __ptrace_request::PTRACE_GETREGSET,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(NT_PRSTATUS as usize),
        &raw mut io,
    ) == 0 as ::core::ffi::c_long
    {
        0 as ::core::ffi::c_int
    } else {
        -1 as ::core::ffi::c_int
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_regs_set(
    mut tid: pid_t,
    mut r: *const eng_regs,
) -> ::core::ffi::c_int {
    let mut io: iovec = iovec {
        iov_base: r as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<eng_regs>(),
    };
    return if ptrace(
        __ptrace_request::PTRACE_SETREGSET,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(NT_PRSTATUS as usize),
        &raw mut io,
    ) == 0 as ::core::ffi::c_long
    {
        0 as ::core::ffi::c_int
    } else {
        -1 as ::core::ffi::c_int
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sysno(mut r: *const eng_regs) -> ::core::ffi::c_long {
    return (*r).orig_rax as ::core::ffi::c_long;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_arg(mut r: *const eng_regs, mut i: ::core::ffi::c_int) -> uint64_t {
    match i {
        0 => return (*r).rdi as uint64_t,
        1 => return (*r).rsi as uint64_t,
        2 => return (*r).rdx as uint64_t,
        3 => return (*r).r10 as uint64_t,
        4 => return (*r).r8 as uint64_t,
        _ => return (*r).r9 as uint64_t,
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_arg(
    mut r: *mut eng_regs,
    mut i: ::core::ffi::c_int,
    mut v: uint64_t,
) {
    match i {
        0 => {
            (*r).rdi = v as ::core::ffi::c_ulonglong;
        }
        1 => {
            (*r).rsi = v as ::core::ffi::c_ulonglong;
        }
        2 => {
            (*r).rdx = v as ::core::ffi::c_ulonglong;
        }
        3 => {
            (*r).r10 = v as ::core::ffi::c_ulonglong;
        }
        4 => {
            (*r).r8 = v as ::core::ffi::c_ulonglong;
        }
        _ => {
            (*r).r9 = v as ::core::ffi::c_ulonglong;
        }
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ret(mut r: *const eng_regs) -> uint64_t {
    return (*r).rax as uint64_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_ret(mut r: *mut eng_regs, mut v: uint64_t) {
    (*r).rax = v as ::core::ffi::c_ulonglong;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_pc(mut r: *const eng_regs) -> uint64_t {
    return (*r).rip as uint64_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_pc(mut r: *mut eng_regs, mut v: uint64_t) {
    (*r).rip = v as ::core::ffi::c_ulonglong;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sp(mut r: *const eng_regs) -> uint64_t {
    return (*r).rsp as uint64_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_sp(mut r: *mut eng_regs, mut v: uint64_t) {
    (*r).rsp = v as ::core::ffi::c_ulonglong;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_void(
    mut tid: pid_t,
    mut r: *mut eng_regs,
) -> ::core::ffi::c_int {
    (*r).orig_rax = __NR_getpid as ::core::ffi::c_ulong as ::core::ffi::c_ulonglong;
    return eng_regs_set(tid, r);
}
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_set(
    mut tid: pid_t,
    mut r: *mut eng_regs,
    mut nr: ::core::ffi::c_long,
) -> ::core::ffi::c_int {
    (*r).orig_rax = nr as ::core::ffi::c_ulong as ::core::ffi::c_ulonglong;
    return eng_regs_set(tid, r);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_insn_len() -> ::core::ffi::c_int {
    return 2 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_info_op(mut tid: pid_t) -> ::core::ffi::c_int {
    let mut info: [::core::ffi::c_uchar; 96] = [0; 96];
    let mut n: ::core::ffi::c_long = ptrace(
        __ptrace_request::PTRACE_GET_SYSCALL_INFO,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(::core::mem::size_of::<
            [::core::ffi::c_uchar; 96],
        >() as usize),
        &raw mut info as *mut ::core::ffi::c_uchar,
    );
    if n < 0 as ::core::ffi::c_long {
        return -1 as ::core::ffi::c_int;
    }
    return info[0usize] as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sysname(mut nr: ::core::ffi::c_long) -> *const ::core::ffi::c_char {
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
