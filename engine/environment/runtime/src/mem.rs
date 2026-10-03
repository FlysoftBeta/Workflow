//! Reading and writing stopped tracee memory through the kernel API.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    #[cfg(target_os = "linux")]
    #[link_name = "__errno_location"]
    unsafe fn errno() -> *mut i32;
    #[cfg(target_os = "android")]
    #[link_name = "__errno"]
    unsafe fn errno() -> *mut i32;
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: usize,
    ) -> *mut ::core::ffi::c_void;
    #[cfg(target_os = "linux")]
    unsafe fn ptrace(_: u32, ...) -> i64;
    #[cfg(target_os = "android")]
    unsafe fn ptrace(_: i32, ...) -> i64;
    unsafe fn process_vm_readv(
        _: i32,
        _: *const iovec,
        _: u64,
        _: *const iovec,
        _: u64,
        _: u64,
    ) -> isize;
    unsafe fn process_vm_writev(
        _: i32,
        _: *const iovec,
        _: u64,
        _: *const iovec,
        _: u64,
        _: u64,
    ) -> isize;
}
#[cfg(target_os = "linux")]
pub const PTRACE_TRACEME: u32 = 0;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKTEXT: u32 = 1;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKDATA: u32 = 2;
#[cfg(target_os = "android")]
pub const PTRACE_PEEKDATA: i32 = 2 as i32;
#[cfg(target_os = "linux")]
pub const PTRACE_PEEKUSER: u32 = 3;
#[cfg(target_os = "linux")]
pub const PTRACE_POKETEXT: u32 = 4;
#[cfg(target_os = "linux")]
pub const PTRACE_POKEDATA: u32 = 5;
#[cfg(target_os = "android")]
pub const PTRACE_POKEDATA: i32 = 5 as i32;
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
#[cfg(target_os = "linux")]
pub const PTRACE_SETREGSET: u32 = 16901;
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
#[cfg(target_os = "linux")]
pub const PTRACE_GET_RSEQ_CONFIGURATION: u32 = 16911;
#[cfg(target_os = "linux")]
pub const PTRACE_SET_SYSCALL_USER_DISPATCH_CONFIG: u32 = 16912;
#[cfg(target_os = "linux")]
pub const PTRACE_GET_SYSCALL_USER_DISPATCH_CONFIG: u32 = 16913;
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_mem_read(
    mut tid: i32,
    mut addr: usize,
    mut buf: *mut ::core::ffi::c_void,
    mut n: usize,
) -> isize {
    let mut local: iovec = iovec {
        iov_base: buf,
        #[cfg(target_os = "linux")]
        iov_len: n,
        #[cfg(target_os = "android")]
        iov_len: n as u64,
    };
    let mut remote: iovec = iovec {
        iov_base: ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(addr as usize),
        #[cfg(target_os = "linux")]
        iov_len: n,
        #[cfg(target_os = "android")]
        iov_len: n as u64,
    };
    let mut r: isize = process_vm_readv(
        tid,
        &raw mut local,
        1 as u64,
        &raw mut remote,
        1 as u64,
        0 as u64,
    );
    if r >= 0 as isize {
        return r;
    }
    let mut done: usize = 0 as usize;
    while done < n {
        *errno() = 0 as i32;
        let mut word: i64 = ptrace(
            PTRACE_PEEKDATA,
            tid,
            ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
                addr.wrapping_add(done as usize) as usize,
            ),
            0 as i32,
        );
        if word == -1 as i64 && *errno() != 0 {
            return if done != 0 {
                done as isize
            } else {
                -1 as isize
            };
        }
        let mut chunk: usize = if n.wrapping_sub(done) < ::core::mem::size_of::<i64>() {
            n.wrapping_sub(done)
        } else {
            ::core::mem::size_of::<i64>()
        };
        memcpy(
            (buf as *mut ::core::ffi::c_char).offset(done as isize) as *mut ::core::ffi::c_void,
            &raw mut word as *const ::core::ffi::c_void,
            chunk,
        );
        done = done.wrapping_add(chunk);
    }
    return done as isize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_mem_write(
    mut tid: i32,
    mut addr: usize,
    mut buf: *const ::core::ffi::c_void,
    mut n: usize,
) -> isize {
    let mut local: iovec = iovec {
        iov_base: buf as *mut ::core::ffi::c_void,
        #[cfg(target_os = "linux")]
        iov_len: n,
        #[cfg(target_os = "android")]
        iov_len: n as u64,
    };
    let mut remote: iovec = iovec {
        iov_base: ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(addr as usize),
        #[cfg(target_os = "linux")]
        iov_len: n,
        #[cfg(target_os = "android")]
        iov_len: n as u64,
    };
    let mut r: isize = process_vm_writev(
        tid,
        &raw mut local,
        1 as u64,
        &raw mut remote,
        1 as u64,
        0 as u64,
    );
    if r >= 0 as isize {
        return r;
    }
    let mut done: usize = 0 as usize;
    while done < n {
        let mut chunk: usize = if n.wrapping_sub(done) < ::core::mem::size_of::<i64>() {
            n.wrapping_sub(done)
        } else {
            ::core::mem::size_of::<i64>()
        };
        let mut word: i64 = 0 as i64;
        if chunk < ::core::mem::size_of::<i64>() {
            *errno() = 0 as i32;
            word = ptrace(
                PTRACE_PEEKDATA,
                tid,
                ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(
                    addr.wrapping_add(done as usize) as usize,
                ),
                0 as i32,
            );
            if word == -1 as i64 && *errno() != 0 {
                return if done != 0 {
                    done as isize
                } else {
                    -1 as isize
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
                addr.wrapping_add(done as usize) as usize,
            ),
            ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(word as usize),
        ) != 0 as i64
        {
            return if done != 0 {
                done as isize
            } else {
                -1 as isize
            };
        }
        done = done.wrapping_add(chunk);
    }
    return done as isize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_mem_read_cstr(
    mut tid: i32,
    mut addr: usize,
    mut buf: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> isize {
    if cap == 0 as usize {
        return -1 as isize;
    }
    let mut got: usize = 0 as usize;
    while got < cap.wrapping_sub(1 as usize) {
        let mut chunk: [::core::ffi::c_char; 256] = [0; 256];
        let mut want: usize = cap.wrapping_sub(1 as usize).wrapping_sub(got);
        if want > ::core::mem::size_of::<[::core::ffi::c_char; 256]>() {
            want = ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as usize;
        }
        let mut r: isize = eng_mem_read(
            tid,
            addr.wrapping_add(got as usize),
            &raw mut chunk as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            want,
        );
        if r <= 0 as isize {
            if got == 0 as usize {
                return -1 as isize;
            }
            break;
        } else {
            let mut i: isize = 0 as isize;
            while i < r {
                if chunk[i as usize] as i32 == '\0' as i32 {
                    memcpy(
                        buf.offset(got as isize) as *mut ::core::ffi::c_void,
                        &raw mut chunk as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                        i as usize,
                    );
                    *buf.offset(got.wrapping_add(i as usize) as isize) =
                        '\0' as ::core::ffi::c_char;
                    return got.wrapping_add(i as usize) as isize;
                }
                i += 1;
            }
            memcpy(
                buf.offset(got as isize) as *mut ::core::ffi::c_void,
                &raw mut chunk as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
                r as usize,
            );
            got = got.wrapping_add(r as usize);
        }
    }
    *buf.offset(got as isize) = '\0' as ::core::ffi::c_char;
    return got as isize;
}
