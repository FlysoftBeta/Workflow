//! Reading and writing stopped tracee memory through the kernel API.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
unsafe extern "C" {
    unsafe fn __errno() -> *mut ::core::ffi::c_int;
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: size_t,
    ) -> *mut ::core::ffi::c_void;
    unsafe fn ptrace(__op: ::core::ffi::c_int, ...) -> ::core::ffi::c_long;
    unsafe fn process_vm_readv(
        __pid: pid_t,
        __local_iov: *const iovec,
        __local_iov_count: ::core::ffi::c_ulong,
        __remote_iov: *const iovec,
        __remote_iov_count: ::core::ffi::c_ulong,
        __flags: ::core::ffi::c_ulong,
    ) -> ssize_t;
    unsafe fn process_vm_writev(
        __pid: pid_t,
        __local_iov: *const iovec,
        __local_iov_count: ::core::ffi::c_ulong,
        __remote_iov: *const iovec,
        __remote_iov_count: ::core::ffi::c_ulong,
        __flags: ::core::ffi::c_ulong,
    ) -> ssize_t;
}
pub type size_t = usize;
pub type uintptr_t = usize;
pub type __kernel_ulong_t = ::core::ffi::c_ulong;
pub type __kernel_pid_t = ::core::ffi::c_int;
pub type __kernel_size_t = __kernel_ulong_t;
pub type __pid_t = __kernel_pid_t;
pub type pid_t = __pid_t;
pub type ssize_t = isize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct iovec {
    pub iov_base: *mut ::core::ffi::c_void,
    pub iov_len: __kernel_size_t,
}
pub const PTRACE_PEEKDATA: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PTRACE_POKEDATA: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_mem_read(
    mut tid: pid_t,
    mut addr: uintptr_t,
    mut buf: *mut ::core::ffi::c_void,
    mut n: size_t,
) -> ssize_t {
    let mut local: iovec = iovec {
        iov_base: buf,
        iov_len: n as __kernel_size_t,
    };
    let mut remote: iovec = iovec {
        iov_base: ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(addr as usize),
        iov_len: n as __kernel_size_t,
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
        *__errno() = 0 as ::core::ffi::c_int;
        let mut word: ::core::ffi::c_long = ptrace(
            PTRACE_PEEKDATA,
            tid,
            ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
                addr.wrapping_add(done as uintptr_t) as usize,
            ),
            0 as ::core::ffi::c_int,
        );
        if word == -1 as ::core::ffi::c_long && *__errno() != 0 {
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
        iov_len: n as __kernel_size_t,
    };
    let mut remote: iovec = iovec {
        iov_base: ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(addr as usize),
        iov_len: n as __kernel_size_t,
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
            *__errno() = 0 as ::core::ffi::c_int;
            word = ptrace(
                PTRACE_PEEKDATA,
                tid,
                ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
                    addr.wrapping_add(done as uintptr_t) as usize,
                ),
                0 as ::core::ffi::c_int,
            );
            if word == -1 as ::core::ffi::c_long && *__errno() != 0 {
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
            PTRACE_POKEDATA,
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
