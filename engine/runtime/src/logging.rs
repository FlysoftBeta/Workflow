use std::sync::atomic::{AtomicU32, Ordering};
static LEVEL: AtomicU32 = AtomicU32::new(1);
pub fn enabled(level: u32) -> bool {
    level <= LEVEL.load(Ordering::Relaxed)
}
#[unsafe(no_mangle)]
pub extern "C" fn eng_log_init(verbosity: i32) {
    let environment = std::env::var("WORKFLOW_ENGINE_LOG")
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(-1);
    let level = verbosity.max(environment);
    LEVEL.store(
        if level < 0 { 1 } else { level.min(4) } as u32,
        Ordering::Relaxed,
    );
}
#[unsafe(no_mangle)]
pub extern "C" fn eng_log_enabled(level: u32) -> i32 {
    enabled(level) as i32
}

unsafe extern "C" {
    pub fn dprintf(fd: i32, format: *const core::ffi::c_char, ...) -> i32;
}
