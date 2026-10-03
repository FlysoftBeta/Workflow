//! Exhaustive, default-deny syscall inventory. Numbering is an ABI, not a host-libc detail.
use core::ffi::{CStr, c_char, c_long};
#[derive(Clone, Copy)]
struct Entry {
    name: Option<&'static CStr>,
    class: u32,
}
include!("syscalls.rs");
#[cfg(target_arch = "x86_64")]
const SELECTED: &[Entry] = &X86_64;
#[cfg(target_arch = "aarch64")]
const SELECTED: &[Entry] = &AARCH64;
const CLASSES: [&CStr; 11] = [
    c"UNKNOWN", c"PASS", c"PATH", c"FD", c"ID", c"META", c"EXEC", c"PROC", c"SOCK", c"ENOSYS",
    c"EPERM",
];
fn lookup(nr: c_long) -> Option<Entry> {
    usize::try_from(nr)
        .ok()
        .and_then(|n| SELECTED.get(n))
        .copied()
}
#[unsafe(no_mangle)]
pub extern "C" fn eng_sysinv_name(nr: c_long) -> *const c_char {
    lookup(nr)
        .and_then(|e| e.name)
        .map_or(core::ptr::null(), CStr::as_ptr)
}
#[unsafe(no_mangle)]
pub extern "C" fn eng_sysinv_class(nr: c_long) -> u32 {
    lookup(nr).map_or(0, |e| e.class)
}
#[unsafe(no_mangle)]
pub extern "C" fn eng_sysinv_max() -> c_long {
    (SELECTED.len() - 1) as c_long
}
#[unsafe(no_mangle)]
pub extern "C" fn eng_sysinv_class_name(class: u32) -> *const c_char {
    CLASSES
        .get(class as usize)
        .copied()
        .unwrap_or(c"INVALID")
        .as_ptr()
}
#[unsafe(no_mangle)]
pub extern "C" fn eng_sysinv_abi() -> *const c_char {
    #[cfg(target_arch = "x86_64")]
    {
        c"x86_64".as_ptr()
    }
    #[cfg(target_arch = "aarch64")]
    {
        c"aarch64".as_ptr()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_abi_entry_has_its_declared_policy_and_no_unknown_passes() {
        let policies: std::collections::HashMap<_, _> = include_str!("../inventory/syscalls.tsv")
            .lines()
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
            .map(|s| {
                let mut f = s.split('\t');
                (f.next().unwrap(), f.next().unwrap())
            })
            .collect();
        for table in [&X86_64, &AARCH64] {
            for entry in table {
                match entry.name {
                    Some(name) => assert_eq!(
                        policies[name.to_str().unwrap()],
                        CLASSES[entry.class as usize].to_str().unwrap()
                    ),
                    None => assert_eq!(entry.class, 0),
                }
            }
        }
        for n in [-1, 472, 0x40000000, i64::MAX] {
            assert_eq!(eng_sysinv_class(n), 0);
            assert!(eng_sysinv_name(n).is_null());
        }
    }
}
