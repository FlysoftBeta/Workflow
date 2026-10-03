//! Reading and writing stopped tracee memory through the kernel API.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
unsafe extern "C" {
    unsafe fn __errno_location() -> *mut ::core::ffi::c_int;
    unsafe fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn ptrace(__request: __ptrace_request, ...) -> ::core::ffi::c_long;
    unsafe fn process_vm_readv(
        __pid: pid_t,
        __lvec: *const iovec,
        __liovcnt: ::core::ffi::c_ulong,
        __rvec: *const iovec,
        __riovcnt: ::core::ffi::c_ulong,
        __flags: ::core::ffi::c_ulong,
    ) -> ssize_t;
    unsafe fn process_vm_writev(
        __pid: pid_t,
        __lvec: *const iovec,
        __liovcnt: ::core::ffi::c_ulong,
        __rvec: *const iovec,
        __riovcnt: ::core::ffi::c_ulong,
        __flags: ::core::ffi::c_ulong,
    ) -> ssize_t;
}
pub type size_t = usize;
pub type __pid_t = ::core::ffi::c_int;
pub type uintptr_t = usize;
pub type pid_t = __pid_t;
pub type ssize_t = isize;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct iovec {
    pub iov_base: *mut ::core::ffi::c_void,
    pub iov_len: size_t,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_mem_read(
    mut tid: pid_t,
    mut addr: uintptr_t,
    mut buf: *mut ::core::ffi::c_void,
    mut n: size_t,
) -> ssize_t {
    let mut local: iovec = iovec {
        iov_base: buf,
        iov_len: n,
    };
    let mut remote: iovec = iovec {
        iov_base: ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(addr as usize),
        iov_len: n,
    };
    let mut r: ssize_t = process_vm_readv(
        tid,
        &raw mut local,
        1 as ::core::ffi::c_ulong,
        &raw mut remote,
        1 as ::core::ffi::c_ulong,
        0 as ::core::ffi::c_ulong,
    );
    if r >= 0 as ssize_t {
        return r;
    }
    let mut done: size_t = 0 as size_t;
    while done < n {
        *__errno_location() = 0 as ::core::ffi::c_int;
        let mut word: ::core::ffi::c_long = ptrace(
            __ptrace_request::PTRACE_PEEKDATA,
            tid,
            ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
                addr.wrapping_add(done as uintptr_t) as usize,
            ),
            0 as ::core::ffi::c_int,
        );
        if word == -1 as ::core::ffi::c_long && *__errno_location() != 0 {
            return if done != 0 {
                done as ssize_t
            } else {
                -1 as ssize_t
            };
        }
        let mut chunk: size_t =
            if n.wrapping_sub(done) < ::core::mem::size_of::<::core::ffi::c_long>() {
                n.wrapping_sub(done)
            } else {
                ::core::mem::size_of::<::core::ffi::c_long>()
            };
        memcpy(
            (buf as *mut ::core::ffi::c_char).offset(done as isize) as *mut ::core::ffi::c_void,
            &raw mut word as *const ::core::ffi::c_void,
            chunk,
        );
        done = done.wrapping_add(chunk);
    }
    return done as ssize_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_mem_write(
    mut tid: pid_t,
    mut addr: uintptr_t,
    mut buf: *const ::core::ffi::c_void,
    mut n: size_t,
) -> ssize_t {
    let mut local: iovec = iovec {
        iov_base: buf as *mut ::core::ffi::c_void,
        iov_len: n,
    };
    let mut remote: iovec = iovec {
        iov_base: ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(addr as usize),
        iov_len: n,
    };
    let mut r: ssize_t = process_vm_writev(
        tid,
        &raw mut local,
        1 as ::core::ffi::c_ulong,
        &raw mut remote,
        1 as ::core::ffi::c_ulong,
        0 as ::core::ffi::c_ulong,
    );
    if r >= 0 as ssize_t {
        return r;
    }
    let mut done: size_t = 0 as size_t;
    while done < n {
        let mut chunk: size_t =
            if n.wrapping_sub(done) < ::core::mem::size_of::<::core::ffi::c_long>() {
                n.wrapping_sub(done)
            } else {
                ::core::mem::size_of::<::core::ffi::c_long>()
            };
        let mut word: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
        if chunk < ::core::mem::size_of::<::core::ffi::c_long>() {
            *__errno_location() = 0 as ::core::ffi::c_int;
            word = ptrace(
                __ptrace_request::PTRACE_PEEKDATA,
                tid,
                ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
                    addr.wrapping_add(done as uintptr_t) as usize,
                ),
                0 as ::core::ffi::c_int,
            );
            if word == -1 as ::core::ffi::c_long && *__errno_location() != 0 {
                return if done != 0 {
                    done as ssize_t
                } else {
                    -1 as ssize_t
                };
            }
        }
        memcpy(
            &raw mut word as *mut ::core::ffi::c_void,
            (buf as *const ::core::ffi::c_char).offset(done as isize) as *const ::core::ffi::c_void,
            chunk,
        );
        if ptrace(
            __ptrace_request::PTRACE_POKEDATA,
            tid,
            ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
                addr.wrapping_add(done as uintptr_t) as usize,
            ),
            ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(word as usize),
        ) != 0 as ::core::ffi::c_long
        {
            return if done != 0 {
                done as ssize_t
            } else {
                -1 as ssize_t
            };
        }
        done = done.wrapping_add(chunk);
    }
    return done as ssize_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_mem_read_cstr(
    mut tid: pid_t,
    mut addr: uintptr_t,
    mut buf: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ssize_t {
    if cap == 0 as size_t {
        return -1 as ssize_t;
    }
    let mut got: size_t = 0 as size_t;
    while got < cap.wrapping_sub(1 as size_t) {
        let mut chunk: [::core::ffi::c_char; 256] = [0; 256];
        let mut want: size_t = cap.wrapping_sub(1 as size_t).wrapping_sub(got);
        if want > ::core::mem::size_of::<[::core::ffi::c_char; 256]>() {
            want = ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as size_t;
        }
        let mut r: ssize_t = eng_mem_read(
            tid,
            addr.wrapping_add(got as uintptr_t),
            &raw mut chunk as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            want,
        );
        if r <= 0 as ssize_t {
            if got == 0 as size_t {
                return -1 as ssize_t;
            }
            break;
        } else {
            let mut i: ssize_t = 0 as ssize_t;
            while i < r {
                if chunk[i as usize] as ::core::ffi::c_int == '\0' as ::core::ffi::c_int {
                    memcpy(
                        buf.offset(got as isize) as *mut ::core::ffi::c_void,
                        &raw mut chunk as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                        i as size_t,
                    );
                    *buf.offset(got.wrapping_add(i as size_t) as isize) =
                        '\0' as ::core::ffi::c_char;
                    return got.wrapping_add(i as size_t) as ssize_t;
                }
                i += 1;
            }
            memcpy(
                buf.offset(got as isize) as *mut ::core::ffi::c_void,
                &raw mut chunk as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                r as size_t,
            );
            got = got.wrapping_add(r as size_t);
        }
    }
    *buf.offset(got as isize) = '\0' as ::core::ffi::c_char;
    return got as ssize_t;
}
