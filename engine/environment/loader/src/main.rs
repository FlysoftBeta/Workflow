//! Freestanding ELF loader: every guest exec is a real kernel exec into this file.
#![no_std]
#![no_main]
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(dead_code, unused_mut, unused_assignments, unused_variables)]
#![allow(static_mut_refs, unsafe_op_in_unsafe_fn)]
mod arch;
mod elf;
#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(include_str!("x86_64.S"), options(att_syntax));
#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(include_str!("aarch64.S"));
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe {
        arch::sc6(arch::__NR_exit_group as i64, 127, 0, 0, 0, 0, 0);
    }
    loop {
        core::hint::spin_loop()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}
// Volatile loops prevent LLVM from lowering these freestanding definitions back
// to libc calls to themselves.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(s: *const core::ffi::c_char) -> usize {
    let mut n = 0;
    while s.add(n).read_volatile() != 0 {
        n += 1;
    }
    n
}
