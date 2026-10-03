//! Raw Linux syscall ABI; no bionic/glibc startup, allocator or dynamic linker.
#[cfg(target_arch = "x86_64")]
mod numbers {
    pub const __NR_write: i32 = 1;
    pub const __NR_close: i32 = 3;
    pub const __NR_mmap: i32 = 9;
    pub const __NR_mprotect: i32 = 10;
    pub const __NR_munmap: i32 = 11;
    pub const __NR_pread64: i32 = 17;
    pub const __NR_getpid: i32 = 39;
    pub const __NR_prctl: i32 = 157;
    pub const __NR_exit_group: i32 = 231;
    pub const __NR_openat: i32 = 257;
    pub const EM_SELF: i32 = 62;
}
#[cfg(target_arch = "aarch64")]
mod numbers {
    pub const __NR_write: i32 = 64;
    pub const __NR_close: i32 = 57;
    pub const __NR_mmap: i32 = 222;
    pub const __NR_mprotect: i32 = 226;
    pub const __NR_munmap: i32 = 215;
    pub const __NR_pread64: i32 = 67;
    pub const __NR_getpid: i32 = 172;
    pub const __NR_prctl: i32 = 167;
    pub const __NR_exit_group: i32 = 94;
    pub const __NR_openat: i32 = 56;
    pub const EM_SELF: i32 = 183;
}
pub use numbers::*;
#[inline(always)]
pub unsafe fn sc6(n: i64, a: i64, b: i64, c: i64, d: i64, e: i64, f: i64) -> i64 {
    let result: i64;
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!("syscall", inlateout("rax") n=>result,
        in("rdi") a, in("rsi") b, in("rdx") c, in("r10") d,
        in("r8") e, in("r9") f, lateout("rcx") _, lateout("r11") _, options(nostack));
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("svc #0", in("x8") n, inlateout("x0") a=>result,
        in("x1") b, in("x2") c, in("x3") d, in("x4") e, in("x5") f, options(nostack));
    result
}
#[inline(always)]
pub unsafe fn wf_finish(sp: usize, block: *const core::ffi::c_void, len: usize, entry: usize) -> ! {
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!("jmp wf_finish",in("rdi") sp,in("rsi") block,in("rdx") len,in("rcx") entry,options(noreturn));
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("b wf_finish",in("x0") sp,in("x1") block,in("x2") len,in("x3") entry,options(noreturn));
}
