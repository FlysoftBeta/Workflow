//! Register sets and syscall register access for the selected kernel ABI.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
unsafe extern "C" {
    unsafe fn ptrace(__op: ::core::ffi::c_int, ...) -> ::core::ffi::c_long;
}
pub type uint64_t = u64;
pub type __kernel_ulong_t = ::core::ffi::c_ulong;
pub type __kernel_pid_t = ::core::ffi::c_int;
pub type __kernel_size_t = __kernel_ulong_t;
pub type __pid_t = __kernel_pid_t;
pub type pid_t = __pid_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct user_regs_struct {
    pub regs: [uint64_t; 31],
    pub sp: uint64_t,
    pub pc: uint64_t,
    pub pstate: uint64_t,
}
pub type eng_regs = user_regs_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct iovec {
    pub iov_base: *mut ::core::ffi::c_void,
    pub iov_len: __kernel_size_t,
}
pub const NT_ARM_SYSTEM_CALL: ::core::ffi::c_int = 0x404 as ::core::ffi::c_int;
pub const NT_PRSTATUS: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PTRACE_GETREGSET: ::core::ffi::c_int = 0x4204 as ::core::ffi::c_int;
pub const PTRACE_SETREGSET: ::core::ffi::c_int = 0x4205 as ::core::ffi::c_int;
pub const PTRACE_GET_SYSCALL_INFO: ::core::ffi::c_int = 0x420e as ::core::ffi::c_int;
pub const __NR_faccessat: ::core::ffi::c_long = 48 as ::core::ffi::c_long;
pub const __NR_openat: ::core::ffi::c_long = 56 as ::core::ffi::c_long;
pub const __NR_newfstatat: ::core::ffi::c_long = 79 as ::core::ffi::c_long;
pub const __NR_getpid: ::core::ffi::c_int = 172 as ::core::ffi::c_int;
pub const __NR_execve: ::core::ffi::c_long = 221 as ::core::ffi::c_long;
pub const __NR_execveat: ::core::ffi::c_long = 281 as ::core::ffi::c_long;
pub const __NR_statx: ::core::ffi::c_long = 291 as ::core::ffi::c_long;
pub const __NR_rseq: ::core::ffi::c_long = 293 as ::core::ffi::c_long;
pub const __NR_clone3: ::core::ffi::c_long = 435 as ::core::ffi::c_long;
pub const __NR_close_range: ::core::ffi::c_long = 436 as ::core::ffi::c_long;
pub const __NR_openat2: ::core::ffi::c_long = 437 as ::core::ffi::c_long;
pub const __NR_faccessat2: ::core::ffi::c_long = 439 as ::core::ffi::c_long;
pub const __NR_fchmodat2: ::core::ffi::c_long = 452 as ::core::ffi::c_long;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_regs_get(mut tid: pid_t, mut r: *mut eng_regs) -> ::core::ffi::c_int {
    let mut io: iovec = iovec {
        iov_base: r as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<eng_regs>() as __kernel_size_t,
    };
    return if ptrace(
        PTRACE_GETREGSET,
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
        iov_len: ::core::mem::size_of::<eng_regs>() as __kernel_size_t,
    };
    return if ptrace(
        PTRACE_SETREGSET,
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
    return (*r).regs[8usize] as ::core::ffi::c_long;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_arg(mut r: *const eng_regs, mut i: ::core::ffi::c_int) -> uint64_t {
    return (*r).regs[i as usize];
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_arg(
    mut r: *mut eng_regs,
    mut i: ::core::ffi::c_int,
    mut v: uint64_t,
) {
    (*r).regs[i as usize] = v;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_ret(mut r: *const eng_regs) -> uint64_t {
    return (*r).regs[0usize];
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_ret(mut r: *mut eng_regs, mut v: uint64_t) {
    (*r).regs[0usize] = v;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_pc(mut r: *const eng_regs) -> uint64_t {
    return (*r).pc;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_pc(mut r: *mut eng_regs, mut v: uint64_t) {
    (*r).pc = v;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sp(mut r: *const eng_regs) -> uint64_t {
    return (*r).sp;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_set_sp(mut r: *mut eng_regs, mut v: uint64_t) {
    (*r).sp = v;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_void(
    mut tid: pid_t,
    mut r: *mut eng_regs,
) -> ::core::ffi::c_int {
    let mut nr: ::core::ffi::c_int = __NR_getpid;
    let mut io: iovec = iovec {
        iov_base: &raw mut nr as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<::core::ffi::c_int>() as __kernel_size_t,
    };
    return if ptrace(
        PTRACE_SETREGSET,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
            NT_ARM_SYSTEM_CALL as usize,
        ),
        &raw mut io,
    ) == 0 as ::core::ffi::c_long
    {
        0 as ::core::ffi::c_int
    } else {
        -1 as ::core::ffi::c_int
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_restore_args(mut cur: *mut eng_regs, mut orig: *const eng_regs) {
    let mut i: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while i <= 5 as ::core::ffi::c_int {
        (*cur).regs[i as usize] = (*orig).regs[i as usize];
        i += 1;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_set(
    mut tid: pid_t,
    mut r: *mut eng_regs,
    mut nr: ::core::ffi::c_long,
) -> ::core::ffi::c_int {
    let mut n: ::core::ffi::c_int = nr as ::core::ffi::c_int;
    let mut io: iovec = iovec {
        iov_base: &raw mut n as *mut ::core::ffi::c_void,
        iov_len: ::core::mem::size_of::<::core::ffi::c_int>() as __kernel_size_t,
    };
    return if ptrace(
        PTRACE_SETREGSET,
        tid,
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
            NT_ARM_SYSTEM_CALL as usize,
        ),
        &raw mut io,
    ) == 0 as ::core::ffi::c_long
    {
        0 as ::core::ffi::c_int
    } else {
        -1 as ::core::ffi::c_int
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_insn_len() -> ::core::ffi::c_int {
    return 4 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_syscall_info_op(mut tid: pid_t) -> ::core::ffi::c_int {
    let mut info: [::core::ffi::c_uchar; 96] = [0; 96];
    let mut n: ::core::ffi::c_long = ptrace(
        PTRACE_GET_SYSCALL_INFO,
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
        172 => return b"getpid\0".as_ptr() as *const ::core::ffi::c_char,
        _ => return b"?\0".as_ptr() as *const ::core::ffi::c_char,
    };
}
