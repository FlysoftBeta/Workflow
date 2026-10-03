//! Workspace container runtime. The initial Rust port preserves the frozen syscall ABI.
//! All engine logic is Rust. Native dependencies are limited to OS and zstd APIs;
//! internal modules retain C-compatible layouts and calling conventions.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(
    dead_code,
    unused_mut,
    unused_assignments,
    unused_variables,
    unused_unsafe
)]
#![allow(static_mut_refs, unsafe_op_in_unsafe_fn, clashing_extern_declarations)]
#![allow(unpredictable_function_pointer_comparisons)]
extern crate zstd_sys;

macro_rules! eng_logf {
    ($level:expr, $fmt:expr $(,$arg:expr)* $(,)?) => {{
        let level = $level.0 as u32;
        if crate::logging::enabled(level) {
            crate::logging::dprintf(2, c"[engine:%c:%d] ".as_ptr(), b"EWIDT"[level as usize] as i32, libc::getpid());
            crate::logging::dprintf(2, $fmt $(,$arg)*);
            crate::logging::dprintf(2, c"\n".as_ptr());
        }
    }};
}
#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "android", target_arch = "x86_64"),
    all(target_os = "android", target_arch = "aarch64"),
)))]
compile_error!("workflow-runtime supports Linux x86_64 and Android x86_64/aarch64");

mod arch;
mod cli;
mod exec;
mod guest;
mod ident;
mod install;
mod json;
mod logging;
mod mem;
mod meta;
mod path;
mod scratch;
mod sha256;
mod sys;
mod sysinv;
mod tracer;

fn main() {
    cli::main();
}
