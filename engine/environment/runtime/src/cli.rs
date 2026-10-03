//! Compatibility CLI contract consumed by the Workspace Engine server.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    #[cfg(target_os = "linux")]
    #[link_name = "__errno_location"]
    unsafe fn errno() -> *mut i32;
    #[cfg(target_os = "android")]
    #[link_name = "__errno"]
    unsafe fn errno() -> *mut i32;
    #[cfg(target_os = "linux")]
    unsafe fn dirname(_: *mut ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    #[cfg(target_os = "android")]
    unsafe fn dirname(_: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    static mut stdout: *mut FILE;
    static mut stderr: *mut FILE;
    unsafe fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> i32;
    unsafe fn printf(_: *const ::core::ffi::c_char, ...) -> i32;
    unsafe fn snprintf(
        _: *mut ::core::ffi::c_char,
        _: usize,
        _: *const ::core::ffi::c_char,
        ...
    ) -> i32;
    unsafe fn strtol(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
        _: i32,
    ) -> i64;
    unsafe fn strtoul(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
        _: i32,
    ) -> u64;
    unsafe fn getenv(_: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn realpath(
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strcmp(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn strncmp(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn strchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strrchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
    unsafe fn strtok_r(
        _: *mut ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strerror(_: i32) -> *mut ::core::ffi::c_char;
    unsafe fn stat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
    unsafe fn lstat(_: *const ::core::ffi::c_char, _: *mut stat) -> i32;
    unsafe fn access(_: *const ::core::ffi::c_char, _: i32) -> i32;
    unsafe fn close(_: i32) -> i32;
    static mut environ: *mut *mut ::core::ffi::c_char;
    unsafe fn sysconf(_: i32) -> i64;
    unsafe fn getuid() -> u32;
    unsafe fn readlink(
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> isize;
    unsafe fn eng_guest_open(_: *const ::core::ffi::c_char) -> *mut eng_guest;
    unsafe fn eng_guest_close(_: *mut eng_guest);
    unsafe fn eng_guest_add_bind(
        _: *mut eng_guest,
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
    ) -> i32;
    unsafe fn eng_guest_default_binds(_: *mut eng_guest) -> i32;
    unsafe fn eng_guest_add_hide(_: *mut eng_guest, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn eng_guest_make_mountpoint(_: *mut eng_guest, _: *const ::core::ffi::c_char, _: i32);
    unsafe fn eng_guest_add_binfmt(_: *mut eng_guest, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn eng_guest_load_binfmt_file(_: *mut eng_guest, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn eng_guest_load_binfmt_dirs(_: *mut eng_guest) -> i32;
    unsafe fn eng_guest_to_host(
        _: *const eng_guest,
        _: *const ::core::ffi::c_char,
        _: *mut ::core::ffi::c_char,
        _: usize,
    ) -> i32;
    unsafe fn eng_install(_: *const eng_install_opts) -> i32;
    unsafe fn eng_clone(
        _: *const ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        _: i32,
    ) -> i32;
    unsafe fn eng_verify(_: *const ::core::ffi::c_char, _: i32) -> i32;
    unsafe fn eng_remove_tree(_: *const ::core::ffi::c_char) -> i32;
    unsafe fn eng_log_init(_: i32);
    unsafe fn eng_log_enabled(_: eng_log_level) -> i32;
    unsafe fn eng_tracer_run(_: *const eng_run_cfg) -> i32;
    unsafe fn eng_link_fsck(_: *mut eng_guest, _: i32, _: *mut ::core::ffi::c_void) -> i32;
    unsafe fn eng_instance_lock(_: *mut eng_guest, _: i32) -> i32;
}
#[cfg(target_os = "linux")]
#[repr(C)]
pub struct _IO_wide_data {
    _opaque: [u8; 0],
}
#[cfg(target_os = "linux")]
#[repr(C)]
pub struct _IO_codecvt {
    _opaque: [u8; 0],
}
#[cfg(target_os = "linux")]
#[repr(C)]
pub struct _IO_marker {
    _opaque: [u8; 0],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: i32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [i64; 3],
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __pad3: [i64; 3],
}
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub __pad1: u64,
    pub st_size: i64,
    pub st_blksize: i32,
    pub __pad2: i32,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __unused4: u32,
    pub __unused5: u32,
}
#[cfg(target_os = "linux")]
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: i32,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: i32,
    pub _flags2: [u8; 3],
    pub _short_backupbuf: [::core::ffi::c_char; 1],
    pub _old_offset: i64,
    pub _cur_column: u16,
    pub _vtable_offset: i8,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: i64,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut ::core::ffi::c_void,
    pub _prevchain: *mut *mut _IO_FILE,
    pub _mode: i32,
    pub _unused3: i32,
    pub _total_written: u64,
    pub _unused2: [::core::ffi::c_char; 8],
}
#[cfg(target_os = "linux")]
pub type _IO_lock_t = ();
#[cfg(target_os = "linux")]
pub type FILE = _IO_FILE;
#[cfg(target_os = "android")]
pub type FILE = __sFILE;
#[cfg(target_os = "linux")]
pub const _SC_ARG_MAX: u32 = 0;
#[cfg(target_os = "linux")]
pub const _SC_CHILD_MAX: u32 = 1;
#[cfg(target_os = "linux")]
pub const _SC_CLK_TCK: u32 = 2;
#[cfg(target_os = "linux")]
pub const _SC_NGROUPS_MAX: u32 = 3;
#[cfg(target_os = "linux")]
pub const _SC_OPEN_MAX: u32 = 4;
#[cfg(target_os = "linux")]
pub const _SC_STREAM_MAX: u32 = 5;
#[cfg(target_os = "linux")]
pub const _SC_TZNAME_MAX: u32 = 6;
#[cfg(target_os = "linux")]
pub const _SC_JOB_CONTROL: u32 = 7;
#[cfg(target_os = "linux")]
pub const _SC_SAVED_IDS: u32 = 8;
#[cfg(target_os = "linux")]
pub const _SC_REALTIME_SIGNALS: u32 = 9;
#[cfg(target_os = "linux")]
pub const _SC_PRIORITY_SCHEDULING: u32 = 10;
#[cfg(target_os = "linux")]
pub const _SC_TIMERS: u32 = 11;
#[cfg(target_os = "linux")]
pub const _SC_ASYNCHRONOUS_IO: u32 = 12;
#[cfg(target_os = "linux")]
pub const _SC_PRIORITIZED_IO: u32 = 13;
#[cfg(target_os = "linux")]
pub const _SC_SYNCHRONIZED_IO: u32 = 14;
#[cfg(target_os = "linux")]
pub const _SC_FSYNC: u32 = 15;
#[cfg(target_os = "linux")]
pub const _SC_MAPPED_FILES: u32 = 16;
#[cfg(target_os = "linux")]
pub const _SC_MEMLOCK: u32 = 17;
#[cfg(target_os = "linux")]
pub const _SC_MEMLOCK_RANGE: u32 = 18;
#[cfg(target_os = "linux")]
pub const _SC_MEMORY_PROTECTION: u32 = 19;
#[cfg(target_os = "linux")]
pub const _SC_MESSAGE_PASSING: u32 = 20;
#[cfg(target_os = "linux")]
pub const _SC_SEMAPHORES: u32 = 21;
#[cfg(target_os = "linux")]
pub const _SC_SHARED_MEMORY_OBJECTS: u32 = 22;
#[cfg(target_os = "linux")]
pub const _SC_AIO_LISTIO_MAX: u32 = 23;
#[cfg(target_os = "linux")]
pub const _SC_AIO_MAX: u32 = 24;
#[cfg(target_os = "linux")]
pub const _SC_AIO_PRIO_DELTA_MAX: u32 = 25;
#[cfg(target_os = "linux")]
pub const _SC_DELAYTIMER_MAX: u32 = 26;
#[cfg(target_os = "linux")]
pub const _SC_MQ_OPEN_MAX: u32 = 27;
#[cfg(target_os = "linux")]
pub const _SC_MQ_PRIO_MAX: u32 = 28;
#[cfg(target_os = "linux")]
pub const _SC_VERSION: u32 = 29;
#[cfg(target_os = "linux")]
pub const _SC_PAGESIZE: u32 = 30;
#[cfg(target_os = "android")]
pub const _SC_PAGESIZE: i32 = 0x27 as i32;
#[cfg(target_os = "linux")]
pub const _SC_RTSIG_MAX: u32 = 31;
#[cfg(target_os = "linux")]
pub const _SC_SEM_NSEMS_MAX: u32 = 32;
#[cfg(target_os = "linux")]
pub const _SC_SEM_VALUE_MAX: u32 = 33;
#[cfg(target_os = "linux")]
pub const _SC_SIGQUEUE_MAX: u32 = 34;
#[cfg(target_os = "linux")]
pub const _SC_TIMER_MAX: u32 = 35;
#[cfg(target_os = "linux")]
pub const _SC_BC_BASE_MAX: u32 = 36;
#[cfg(target_os = "linux")]
pub const _SC_BC_DIM_MAX: u32 = 37;
#[cfg(target_os = "linux")]
pub const _SC_BC_SCALE_MAX: u32 = 38;
#[cfg(target_os = "linux")]
pub const _SC_BC_STRING_MAX: u32 = 39;
#[cfg(target_os = "linux")]
pub const _SC_COLL_WEIGHTS_MAX: u32 = 40;
#[cfg(target_os = "linux")]
pub const _SC_EQUIV_CLASS_MAX: u32 = 41;
#[cfg(target_os = "linux")]
pub const _SC_EXPR_NEST_MAX: u32 = 42;
#[cfg(target_os = "linux")]
pub const _SC_LINE_MAX: u32 = 43;
#[cfg(target_os = "linux")]
pub const _SC_RE_DUP_MAX: u32 = 44;
#[cfg(target_os = "linux")]
pub const _SC_CHARCLASS_NAME_MAX: u32 = 45;
#[cfg(target_os = "linux")]
pub const _SC_2_VERSION: u32 = 46;
#[cfg(target_os = "linux")]
pub const _SC_2_C_BIND: u32 = 47;
#[cfg(target_os = "linux")]
pub const _SC_2_C_DEV: u32 = 48;
#[cfg(target_os = "linux")]
pub const _SC_2_FORT_DEV: u32 = 49;
#[cfg(target_os = "linux")]
pub const _SC_2_FORT_RUN: u32 = 50;
#[cfg(target_os = "linux")]
pub const _SC_2_SW_DEV: u32 = 51;
#[cfg(target_os = "linux")]
pub const _SC_2_LOCALEDEF: u32 = 52;
#[cfg(target_os = "linux")]
pub const _SC_PII: u32 = 53;
#[cfg(target_os = "linux")]
pub const _SC_PII_XTI: u32 = 54;
#[cfg(target_os = "linux")]
pub const _SC_PII_SOCKET: u32 = 55;
#[cfg(target_os = "linux")]
pub const _SC_PII_INTERNET: u32 = 56;
#[cfg(target_os = "linux")]
pub const _SC_PII_OSI: u32 = 57;
#[cfg(target_os = "linux")]
pub const _SC_POLL: u32 = 58;
#[cfg(target_os = "linux")]
pub const _SC_SELECT: u32 = 59;
#[cfg(target_os = "linux")]
pub const _SC_UIO_MAXIOV: u32 = 60;
#[cfg(target_os = "linux")]
pub const _SC_IOV_MAX: u32 = 60;
#[cfg(target_os = "linux")]
pub const _SC_PII_INTERNET_STREAM: u32 = 61;
#[cfg(target_os = "linux")]
pub const _SC_PII_INTERNET_DGRAM: u32 = 62;
#[cfg(target_os = "linux")]
pub const _SC_PII_OSI_COTS: u32 = 63;
#[cfg(target_os = "linux")]
pub const _SC_PII_OSI_CLTS: u32 = 64;
#[cfg(target_os = "linux")]
pub const _SC_PII_OSI_M: u32 = 65;
#[cfg(target_os = "linux")]
pub const _SC_T_IOV_MAX: u32 = 66;
#[cfg(target_os = "linux")]
pub const _SC_THREADS: u32 = 67;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_SAFE_FUNCTIONS: u32 = 68;
#[cfg(target_os = "linux")]
pub const _SC_GETGR_R_SIZE_MAX: u32 = 69;
#[cfg(target_os = "linux")]
pub const _SC_GETPW_R_SIZE_MAX: u32 = 70;
#[cfg(target_os = "linux")]
pub const _SC_LOGIN_NAME_MAX: u32 = 71;
#[cfg(target_os = "linux")]
pub const _SC_TTY_NAME_MAX: u32 = 72;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_DESTRUCTOR_ITERATIONS: u32 = 73;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_KEYS_MAX: u32 = 74;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_STACK_MIN: u32 = 75;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_THREADS_MAX: u32 = 76;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_ATTR_STACKADDR: u32 = 77;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_ATTR_STACKSIZE: u32 = 78;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_PRIORITY_SCHEDULING: u32 = 79;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_PRIO_INHERIT: u32 = 80;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_PRIO_PROTECT: u32 = 81;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_PROCESS_SHARED: u32 = 82;
#[cfg(target_os = "linux")]
pub const _SC_NPROCESSORS_CONF: u32 = 83;
#[cfg(target_os = "linux")]
pub const _SC_NPROCESSORS_ONLN: u32 = 84;
#[cfg(target_os = "linux")]
pub const _SC_PHYS_PAGES: u32 = 85;
#[cfg(target_os = "linux")]
pub const _SC_AVPHYS_PAGES: u32 = 86;
#[cfg(target_os = "linux")]
pub const _SC_ATEXIT_MAX: u32 = 87;
#[cfg(target_os = "linux")]
pub const _SC_PASS_MAX: u32 = 88;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_VERSION: u32 = 89;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_XCU_VERSION: u32 = 90;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_UNIX: u32 = 91;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_CRYPT: u32 = 92;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_ENH_I18N: u32 = 93;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_SHM: u32 = 94;
#[cfg(target_os = "linux")]
pub const _SC_2_CHAR_TERM: u32 = 95;
#[cfg(target_os = "linux")]
pub const _SC_2_C_VERSION: u32 = 96;
#[cfg(target_os = "linux")]
pub const _SC_2_UPE: u32 = 97;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_XPG2: u32 = 98;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_XPG3: u32 = 99;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_XPG4: u32 = 100;
#[cfg(target_os = "linux")]
pub const _SC_CHAR_BIT: u32 = 101;
#[cfg(target_os = "linux")]
pub const _SC_CHAR_MAX: u32 = 102;
#[cfg(target_os = "linux")]
pub const _SC_CHAR_MIN: u32 = 103;
#[cfg(target_os = "linux")]
pub const _SC_INT_MAX: u32 = 104;
#[cfg(target_os = "linux")]
pub const _SC_INT_MIN: u32 = 105;
#[cfg(target_os = "linux")]
pub const _SC_LONG_BIT: u32 = 106;
#[cfg(target_os = "linux")]
pub const _SC_WORD_BIT: u32 = 107;
#[cfg(target_os = "linux")]
pub const _SC_MB_LEN_MAX: u32 = 108;
#[cfg(target_os = "linux")]
pub const _SC_NZERO: u32 = 109;
#[cfg(target_os = "linux")]
pub const _SC_SSIZE_MAX: u32 = 110;
#[cfg(target_os = "linux")]
pub const _SC_SCHAR_MAX: u32 = 111;
#[cfg(target_os = "linux")]
pub const _SC_SCHAR_MIN: u32 = 112;
#[cfg(target_os = "linux")]
pub const _SC_SHRT_MAX: u32 = 113;
#[cfg(target_os = "linux")]
pub const _SC_SHRT_MIN: u32 = 114;
#[cfg(target_os = "linux")]
pub const _SC_UCHAR_MAX: u32 = 115;
#[cfg(target_os = "linux")]
pub const _SC_UINT_MAX: u32 = 116;
#[cfg(target_os = "linux")]
pub const _SC_ULONG_MAX: u32 = 117;
#[cfg(target_os = "linux")]
pub const _SC_USHRT_MAX: u32 = 118;
#[cfg(target_os = "linux")]
pub const _SC_NL_ARGMAX: u32 = 119;
#[cfg(target_os = "linux")]
pub const _SC_NL_LANGMAX: u32 = 120;
#[cfg(target_os = "linux")]
pub const _SC_NL_MSGMAX: u32 = 121;
#[cfg(target_os = "linux")]
pub const _SC_NL_NMAX: u32 = 122;
#[cfg(target_os = "linux")]
pub const _SC_NL_SETMAX: u32 = 123;
#[cfg(target_os = "linux")]
pub const _SC_NL_TEXTMAX: u32 = 124;
#[cfg(target_os = "linux")]
pub const _SC_XBS5_ILP32_OFF32: u32 = 125;
#[cfg(target_os = "linux")]
pub const _SC_XBS5_ILP32_OFFBIG: u32 = 126;
#[cfg(target_os = "linux")]
pub const _SC_XBS5_LP64_OFF64: u32 = 127;
#[cfg(target_os = "linux")]
pub const _SC_XBS5_LPBIG_OFFBIG: u32 = 128;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_LEGACY: u32 = 129;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_REALTIME: u32 = 130;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_REALTIME_THREADS: u32 = 131;
#[cfg(target_os = "linux")]
pub const _SC_ADVISORY_INFO: u32 = 132;
#[cfg(target_os = "linux")]
pub const _SC_BARRIERS: u32 = 133;
#[cfg(target_os = "linux")]
pub const _SC_BASE: u32 = 134;
#[cfg(target_os = "linux")]
pub const _SC_C_LANG_SUPPORT: u32 = 135;
#[cfg(target_os = "linux")]
pub const _SC_C_LANG_SUPPORT_R: u32 = 136;
#[cfg(target_os = "linux")]
pub const _SC_CLOCK_SELECTION: u32 = 137;
#[cfg(target_os = "linux")]
pub const _SC_CPUTIME: u32 = 138;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_CPUTIME: u32 = 139;
#[cfg(target_os = "linux")]
pub const _SC_DEVICE_IO: u32 = 140;
#[cfg(target_os = "linux")]
pub const _SC_DEVICE_SPECIFIC: u32 = 141;
#[cfg(target_os = "linux")]
pub const _SC_DEVICE_SPECIFIC_R: u32 = 142;
#[cfg(target_os = "linux")]
pub const _SC_FD_MGMT: u32 = 143;
#[cfg(target_os = "linux")]
pub const _SC_FIFO: u32 = 144;
#[cfg(target_os = "linux")]
pub const _SC_PIPE: u32 = 145;
#[cfg(target_os = "linux")]
pub const _SC_FILE_ATTRIBUTES: u32 = 146;
#[cfg(target_os = "linux")]
pub const _SC_FILE_LOCKING: u32 = 147;
#[cfg(target_os = "linux")]
pub const _SC_FILE_SYSTEM: u32 = 148;
#[cfg(target_os = "linux")]
pub const _SC_MONOTONIC_CLOCK: u32 = 149;
#[cfg(target_os = "linux")]
pub const _SC_MULTI_PROCESS: u32 = 150;
#[cfg(target_os = "linux")]
pub const _SC_SINGLE_PROCESS: u32 = 151;
#[cfg(target_os = "linux")]
pub const _SC_NETWORKING: u32 = 152;
#[cfg(target_os = "linux")]
pub const _SC_READER_WRITER_LOCKS: u32 = 153;
#[cfg(target_os = "linux")]
pub const _SC_SPIN_LOCKS: u32 = 154;
#[cfg(target_os = "linux")]
pub const _SC_REGEXP: u32 = 155;
#[cfg(target_os = "linux")]
pub const _SC_REGEX_VERSION: u32 = 156;
#[cfg(target_os = "linux")]
pub const _SC_SHELL: u32 = 157;
#[cfg(target_os = "linux")]
pub const _SC_SIGNALS: u32 = 158;
#[cfg(target_os = "linux")]
pub const _SC_SPAWN: u32 = 159;
#[cfg(target_os = "linux")]
pub const _SC_SPORADIC_SERVER: u32 = 160;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_SPORADIC_SERVER: u32 = 161;
#[cfg(target_os = "linux")]
pub const _SC_SYSTEM_DATABASE: u32 = 162;
#[cfg(target_os = "linux")]
pub const _SC_SYSTEM_DATABASE_R: u32 = 163;
#[cfg(target_os = "linux")]
pub const _SC_TIMEOUTS: u32 = 164;
#[cfg(target_os = "linux")]
pub const _SC_TYPED_MEMORY_OBJECTS: u32 = 165;
#[cfg(target_os = "linux")]
pub const _SC_USER_GROUPS: u32 = 166;
#[cfg(target_os = "linux")]
pub const _SC_USER_GROUPS_R: u32 = 167;
#[cfg(target_os = "linux")]
pub const _SC_2_PBS: u32 = 168;
#[cfg(target_os = "linux")]
pub const _SC_2_PBS_ACCOUNTING: u32 = 169;
#[cfg(target_os = "linux")]
pub const _SC_2_PBS_LOCATE: u32 = 170;
#[cfg(target_os = "linux")]
pub const _SC_2_PBS_MESSAGE: u32 = 171;
#[cfg(target_os = "linux")]
pub const _SC_2_PBS_TRACK: u32 = 172;
#[cfg(target_os = "linux")]
pub const _SC_SYMLOOP_MAX: u32 = 173;
#[cfg(target_os = "linux")]
pub const _SC_STREAMS: u32 = 174;
#[cfg(target_os = "linux")]
pub const _SC_2_PBS_CHECKPOINT: u32 = 175;
#[cfg(target_os = "linux")]
pub const _SC_V6_ILP32_OFF32: u32 = 176;
#[cfg(target_os = "linux")]
pub const _SC_V6_ILP32_OFFBIG: u32 = 177;
#[cfg(target_os = "linux")]
pub const _SC_V6_LP64_OFF64: u32 = 178;
#[cfg(target_os = "linux")]
pub const _SC_V6_LPBIG_OFFBIG: u32 = 179;
#[cfg(target_os = "linux")]
pub const _SC_HOST_NAME_MAX: u32 = 180;
#[cfg(target_os = "linux")]
pub const _SC_TRACE: u32 = 181;
#[cfg(target_os = "linux")]
pub const _SC_TRACE_EVENT_FILTER: u32 = 182;
#[cfg(target_os = "linux")]
pub const _SC_TRACE_INHERIT: u32 = 183;
#[cfg(target_os = "linux")]
pub const _SC_TRACE_LOG: u32 = 184;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL1_ICACHE_SIZE: u32 = 185;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL1_ICACHE_ASSOC: u32 = 186;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL1_ICACHE_LINESIZE: u32 = 187;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL1_DCACHE_SIZE: u32 = 188;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL1_DCACHE_ASSOC: u32 = 189;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL1_DCACHE_LINESIZE: u32 = 190;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL2_CACHE_SIZE: u32 = 191;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL2_CACHE_ASSOC: u32 = 192;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL2_CACHE_LINESIZE: u32 = 193;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL3_CACHE_SIZE: u32 = 194;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL3_CACHE_ASSOC: u32 = 195;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL3_CACHE_LINESIZE: u32 = 196;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL4_CACHE_SIZE: u32 = 197;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL4_CACHE_ASSOC: u32 = 198;
#[cfg(target_os = "linux")]
pub const _SC_LEVEL4_CACHE_LINESIZE: u32 = 199;
#[cfg(target_os = "linux")]
pub const _SC_IPV6: u32 = 235;
#[cfg(target_os = "linux")]
pub const _SC_RAW_SOCKETS: u32 = 236;
#[cfg(target_os = "linux")]
pub const _SC_V7_ILP32_OFF32: u32 = 237;
#[cfg(target_os = "linux")]
pub const _SC_V7_ILP32_OFFBIG: u32 = 238;
#[cfg(target_os = "linux")]
pub const _SC_V7_LP64_OFF64: u32 = 239;
#[cfg(target_os = "linux")]
pub const _SC_V7_LPBIG_OFFBIG: u32 = 240;
#[cfg(target_os = "linux")]
pub const _SC_SS_REPL_MAX: u32 = 241;
#[cfg(target_os = "linux")]
pub const _SC_TRACE_EVENT_NAME_MAX: u32 = 242;
#[cfg(target_os = "linux")]
pub const _SC_TRACE_NAME_MAX: u32 = 243;
#[cfg(target_os = "linux")]
pub const _SC_TRACE_SYS_MAX: u32 = 244;
#[cfg(target_os = "linux")]
pub const _SC_TRACE_USER_EVENT_MAX: u32 = 245;
#[cfg(target_os = "linux")]
pub const _SC_XOPEN_STREAMS: u32 = 246;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_ROBUST_PRIO_INHERIT: u32 = 247;
#[cfg(target_os = "linux")]
pub const _SC_THREAD_ROBUST_PRIO_PROTECT: u32 = 248;
#[cfg(target_os = "linux")]
pub const _SC_MINSIGSTKSZ: u32 = 249;
#[cfg(target_os = "linux")]
pub const _SC_SIGSTKSZ: u32 = 250;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_binfmt {
    pub name: [::core::ffi::c_char; 64],
    pub r#type: ::core::ffi::c_char,
    pub offset: u32,
    pub magic: [u8; 128],
    pub mask: [u8; 128],
    pub len: u32,
    pub ext: [::core::ffi::c_char; 64],
    pub interp: [::core::ffi::c_char; 4096],
    pub preserve_argv0: i32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_bind {
    pub guest: [::core::ffi::c_char; 4096],
    pub host: [::core::ffi::c_char; 4096],
    pub glen: usize,
    pub hlen: usize,
    pub uid: u32,
    pub gid: u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_guest {
    pub root: [::core::ffi::c_char; 4096],
    pub rootlen: usize,
    pub rootfd: i32,
    pub binds: [eng_bind; 32],
    pub nbinds: i32,
    pub hides: [[::core::ffi::c_char; 4096]; 16],
    pub nhides: i32,
    pub binfmt: [eng_binfmt; 32],
    pub nbinfmt: i32,
    pub loader: [::core::ffi::c_char; 4096],
    pub sockdir: [::core::ffi::c_char; 4096],
    pub no_filemap: i32,
    pub test_pagesz: u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_install_opts {
    pub image: *const ::core::ffi::c_char,
    pub sha256: *const ::core::ffi::c_char,
    pub index: *const ::core::ffi::c_char,
    pub target: *const ::core::ffi::c_char,
    pub profile: *const ::core::ffi::c_char,
    pub quiet: i32,
    pub keep_partial: i32,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_log_level(pub u32);
impl eng_log_level {
    pub const ENG_LOG_ERROR: Self = Self(0);
    pub const ENG_LOG_WARN: Self = Self(1);
    pub const ENG_LOG_INFO: Self = Self(2);
    pub const ENG_LOG_DEBUG: Self = Self(3);
    pub const ENG_LOG_TRACE: Self = Self(4);
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_run_cfg {
    pub argv: *mut *mut ::core::ffi::c_char,
    pub exe: *const ::core::ffi::c_char,
    pub envp: *mut *mut ::core::ffi::c_char,
    pub cwd: *const ::core::ffi::c_char,
    pub guest: *mut eng_guest,
    pub use_seccomp_fastpath: i32,
    pub uid: u32,
    pub gid: u32,
    pub test_app_filter: i32,
    pub wait_all: i32,
}
pub const ENOENT: i32 = 2 as i32;
pub const EAGAIN: i32 = 11 as i32;
pub const EWOULDBLOCK: i32 = EAGAIN;
#[cfg(target_os = "linux")]
pub const __S_IFMT: i32 = 0o170000 as i32;
pub const X_OK: i32 = 1 as i32;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ENG_MAX_BINDS: i32 = 32 as i32;
pub const ENG_MAX_HIDES: i32 = 16 as i32;
pub const ENG_MAX_BINFMT: i32 = 32 as i32;
#[cfg(target_arch = "x86_64")]
pub const ENG_ARCH_NAME: [::core::ffi::c_char; 7] =
    unsafe { ::core::mem::transmute::<[u8; 7], [::core::ffi::c_char; 7]>(*b"x86_64\0") };
#[cfg(target_arch = "aarch64")]
pub const ENG_ARCH_NAME: [::core::ffi::c_char; 6] =
    unsafe { ::core::mem::transmute::<[u8; 6], [::core::ffi::c_char; 6]>(*b"arm64\0") };
pub const S_IFREG: i32 = 0o100000 as i32;
pub const S_IFLNK: i32 = 0o120000 as i32;
pub const S_IFDIR: i32 = 0o40000 as i32;
pub const S_IFMT: i32 = 0o170000 as i32;
unsafe extern "C" fn usage() {
    fprintf(
        stderr,
        b"usage: workflow-engine run [--root DIR] [--cwd GUESTDIR] [--bind HOST:GUEST]...\n                           [--hide GUESTPATH]... [--loader FILE] [--user work|root]\n                           [--uid N] [--gid N] [--no-default-binds] [--no-filemap]\n                           [--wait-all] [-v] -- CMD [ARGS...]\n       workflow-engine probe\n       workflow-engine fsck --root DIR [--repair]\n       workflow-engine install --image FILE|- (--index image.json | --sha256 HEX) --target GEN\n                               [--profile workspace|base|any] [--quiet]\n       workflow-engine clone --from GEN --to GEN\n       workflow-engine verify --generation GEN\n       workflow-engine remove --generation GEN\n\0"
            .as_ptr() as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn default_loader(mut out: *mut ::core::ffi::c_char, mut cap: usize) -> i32 {
    let mut env: *const ::core::ffi::c_char =
        getenv(b"WORKFLOW_ENGINE_LOADER\0".as_ptr() as *const ::core::ffi::c_char);
    if !env.is_null() && *env as i32 != 0 {
        snprintf(
            out,
            cap,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            env,
        );
        return 0 as i32;
    }
    let mut self_0: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut n: isize = readlink(
        b"/proc/self/exe\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut self_0 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as usize),
    );
    if n <= 0 as isize {
        return -1 as i32;
    }
    self_0[n as usize] = 0 as ::core::ffi::c_char;
    let mut dir: *mut ::core::ffi::c_char = dirname(&raw mut self_0 as *mut ::core::ffi::c_char);
    snprintf(
        out,
        cap,
        b"%s/libworkflow-loader.so\0".as_ptr() as *const ::core::ffi::c_char,
        dir,
    );
    if access(out, X_OK) == 0 as i32 {
        return 0 as i32;
    }
    snprintf(
        out,
        cap,
        b"%s/loader\0".as_ptr() as *const ::core::ffi::c_char,
        dir,
    );
    return if access(out, X_OK) == 0 as i32 {
        0 as i32
    } else {
        -1 as i32
    };
}
unsafe extern "C" fn guest_path_search(
    mut g: *mut eng_guest,
    mut name: *const ::core::ffi::c_char,
    mut envp: *mut *mut ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: usize,
) -> i32 {
    let mut path: *const ::core::ffi::c_char =
        b"/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\0".as_ptr()
            as *const ::core::ffi::c_char;
    let mut e: *mut *mut ::core::ffi::c_char = envp;
    while !e.is_null() && !(*e).is_null() {
        if strncmp(
            *e,
            b"PATH=\0".as_ptr() as *const ::core::ffi::c_char,
            5 as usize,
        ) == 0
        {
            path = (*e).offset(5 as i32 as isize);
            break;
        } else {
            e = e.offset(1);
        }
    }
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut buf as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
        path,
    );
    let mut save: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut dir: *mut ::core::ffi::c_char = strtok_r(
        &raw mut buf as *mut ::core::ffi::c_char,
        b":\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut save,
    );
    while !dir.is_null() {
        if *dir.offset(0isize) as i32 == '/' as i32 {
            let mut gp: [::core::ffi::c_char; 4096] = [0; 4096];
            let mut hp: [::core::ffi::c_char; 4096] = [0; 4096];
            snprintf(
                &raw mut gp as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
                dir,
                name,
            );
            if eng_guest_to_host(
                g,
                &raw mut gp as *mut ::core::ffi::c_char,
                &raw mut hp as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            ) == 0
            {
                let mut st: stat = platform_empty_stat();
                if stat(&raw mut hp as *mut ::core::ffi::c_char, &raw mut st) == 0 as i32
                    && st.st_mode & S_IFMT as u32 == S_IFREG as u32
                {
                    snprintf(
                        out,
                        cap,
                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut gp as *mut ::core::ffi::c_char,
                    );
                    return 0 as i32;
                }
                if lstat(&raw mut hp as *mut ::core::ffi::c_char, &raw mut st) == 0 as i32
                    && st.st_mode & S_IFMT as u32 == S_IFLNK as u32
                {
                    snprintf(
                        out,
                        cap,
                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut gp as *mut ::core::ffi::c_char,
                    );
                    return 0 as i32;
                }
            }
        }
        dir = strtok_r(
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            b":\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut save,
        );
    }
    return -1 as i32;
}
unsafe extern "C" fn cmd_fsck(mut argc: i32, mut argv: *mut *mut ::core::ffi::c_char) -> i32 {
    let mut root: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut repair: i32 = 0 as i32;
    let mut i: i32 = 2 as i32;
    while i < argc {
        if strcmp(
            *argv.offset(i as isize),
            b"--root\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
            && (i + 1 as i32) < argc
        {
            i += 1;
            root = *argv.offset(i as isize);
        } else if strcmp(
            *argv.offset(i as isize),
            b"--repair\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        {
            repair = 1 as i32;
        } else {
            usage();
            return 2 as i32;
        }
        i += 1;
    }
    if root.is_null() {
        usage();
        return 2 as i32;
    }
    let mut g: *mut eng_guest = eng_guest_open(root);
    if g.is_null() {
        fprintf(
            stderr,
            b"fsck: cannot open %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            root,
            strerror(*errno()),
        );
        return 125 as i32;
    }
    let mut lk: i32 = eng_instance_lock(g, repair);
    if lk < 0 as i32 {
        fprintf(
            stderr,
            b"fsck: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            if lk == -EWOULDBLOCK {
                b"rootfs in use by a running engine\0".as_ptr() as *const ::core::ffi::c_char
            } else {
                strerror(-lk) as *const ::core::ffi::c_char
            },
        );
        return if lk == -EWOULDBLOCK {
            3 as i32
        } else {
            125 as i32
        };
    }
    let mut rc: i32 = eng_link_fsck(g, repair, stdout as *mut ::core::ffi::c_void);
    close(lk);
    return if rc < 0 as i32 {
        125 as i32
    } else if rc > 0 as i32 {
        1 as i32
    } else {
        0 as i32
    };
}
unsafe extern "C" fn cmd_lifecycle(mut argc: i32, mut argv: *mut *mut ::core::ffi::c_char) -> i32 {
    let mut o: eng_install_opts = eng_install_opts {
        image: ::core::ptr::null::<::core::ffi::c_char>(),
        sha256: ::core::ptr::null::<::core::ffi::c_char>(),
        index: ::core::ptr::null::<::core::ffi::c_char>(),
        target: ::core::ptr::null::<::core::ffi::c_char>(),
        profile: b"workspace\0".as_ptr() as *const ::core::ffi::c_char,
        quiet: 0,
        keep_partial: 0,
    };
    let mut from: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut r#gen: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut i: i32 = 2 as i32;
    while i < argc {
        let mut a: *const ::core::ffi::c_char = *argv.offset(i as isize);
        let mut more: i32 = ((i + 1 as i32) < argc) as i32;
        if strcmp(a, b"--image\0".as_ptr() as *const ::core::ffi::c_char) == 0 && more != 0 {
            i += 1;
            o.image = *argv.offset(i as isize);
        } else if strcmp(a, b"--sha256\0".as_ptr() as *const ::core::ffi::c_char) == 0 && more != 0
        {
            i += 1;
            o.sha256 = *argv.offset(i as isize);
        } else if strcmp(a, b"--index\0".as_ptr() as *const ::core::ffi::c_char) == 0 && more != 0 {
            i += 1;
            o.index = *argv.offset(i as isize);
        } else if (strcmp(a, b"--target\0".as_ptr() as *const ::core::ffi::c_char) == 0
            || strcmp(a, b"--to\0".as_ptr() as *const ::core::ffi::c_char) == 0)
            && more != 0
        {
            i += 1;
            o.target = *argv.offset(i as isize);
        } else if strcmp(a, b"--profile\0".as_ptr() as *const ::core::ffi::c_char) == 0 && more != 0
        {
            i += 1;
            o.profile = *argv.offset(i as isize);
        } else if strcmp(a, b"--from\0".as_ptr() as *const ::core::ffi::c_char) == 0 && more != 0 {
            i += 1;
            from = *argv.offset(i as isize);
        } else if strcmp(a, b"--generation\0".as_ptr() as *const ::core::ffi::c_char) == 0
            && more != 0
        {
            i += 1;
            r#gen = *argv.offset(i as isize);
        } else if strcmp(a, b"--quiet\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            o.quiet = 1 as i32;
        } else if strcmp(
            a,
            b"--keep-partial\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        {
            o.keep_partial = 1 as i32;
        } else {
            fprintf(
                stderr,
                b"%s: unknown option %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                *argv.offset(1isize),
                a,
            );
            usage();
            return 2 as i32;
        }
        i += 1;
    }
    if strcmp(
        *argv.offset(1isize),
        b"install\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        if o.image.is_null() || o.target.is_null() {
            usage();
            return 2 as i32;
        }
        return eng_install(&raw mut o);
    }
    if strcmp(
        *argv.offset(1isize),
        b"clone\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        if from.is_null() || o.target.is_null() {
            usage();
            return 2 as i32;
        }
        return eng_clone(from, o.target, o.quiet);
    }
    if r#gen.is_null() {
        usage();
        return 2 as i32;
    }
    if strcmp(
        *argv.offset(1isize),
        b"verify\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        return eng_verify(r#gen, o.quiet);
    }
    let mut root: [::core::ffi::c_char; 4096] = [0; 4096];
    snprintf(
        &raw mut root as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        b"%s/rootfs\0".as_ptr() as *const ::core::ffi::c_char,
        r#gen,
    );
    let mut g: *mut eng_guest = eng_guest_open(&raw mut root as *mut ::core::ffi::c_char);
    if !g.is_null() {
        let mut lk: i32 = eng_instance_lock(g, 1 as i32);
        eng_guest_close(g);
        if lk == -EWOULDBLOCK {
            fprintf(
                stderr,
                b"remove: generation in use by a running engine\n\0".as_ptr()
                    as *const ::core::ffi::c_char,
            );
            return 3 as i32;
        }
        if lk >= 0 as i32 {
            close(lk);
        }
    }
    let mut rc: i32 = eng_remove_tree(r#gen);
    if rc != 0 && rc != -ENOENT {
        fprintf(
            stderr,
            b"remove: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(-rc),
        );
        return 74 as i32;
    }
    return 0 as i32;
}
unsafe fn main_0(mut argc: i32, mut argv: *mut *mut ::core::ffi::c_char) -> i32 {
    if argc < 2 as i32 {
        usage();
        return 2 as i32;
    }
    if strcmp(
        *argv.offset(1isize),
        b"probe\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        eng_log_init(0 as i32);
        return cmd_probe();
    }
    if strcmp(
        *argv.offset(1isize),
        b"fsck\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        eng_log_init(0 as i32);
        return cmd_fsck(argc, argv);
    }
    if strcmp(
        *argv.offset(1isize),
        b"install\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
        || strcmp(
            *argv.offset(1isize),
            b"clone\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        || strcmp(
            *argv.offset(1isize),
            b"verify\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        || strcmp(
            *argv.offset(1isize),
            b"remove\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
    {
        eng_log_init(0 as i32);
        return cmd_lifecycle(argc, argv);
    }
    if strcmp(
        *argv.offset(1isize),
        b"run\0".as_ptr() as *const ::core::ffi::c_char,
    ) != 0
    {
        usage();
        return 2 as i32;
    }
    let mut root: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut cwd: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut loader: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut binds: [*const ::core::ffi::c_char; 32] =
        [::core::ptr::null::<::core::ffi::c_char>(); 32];
    let mut hides: [*const ::core::ffi::c_char; 16] =
        [::core::ptr::null::<::core::ffi::c_char>(); 16];
    let mut nb: i32 = 0 as i32;
    let mut nh: i32 = 0 as i32;
    let mut verbosity: i32 = 0 as i32;
    let mut default_binds: i32 = 1 as i32;
    let mut no_filemap: i32 = 0 as i32;
    let mut seccomp: i32 = 1 as i32;
    let mut test_pagesz: u32 = 0 as u32;
    let mut test_app_filter: i32 = 0 as i32;
    let mut wait_all: i32 = 0 as i32;
    let mut binfmts: [*const ::core::ffi::c_char; 32] =
        [::core::ptr::null::<::core::ffi::c_char>(); 32];
    let mut nbf: i32 = 0 as i32;
    let mut no_guest_binfmt: i32 = 0 as i32;
    let mut sockdir: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut uid: i64 = 1000 as i64;
    let mut gid: i64 = 1000 as i64;
    let mut i: i32 = 2 as i32;
    while i < argc {
        let mut a: *const ::core::ffi::c_char = *argv.offset(i as isize);
        if strcmp(a, b"--\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            i += 1;
            break;
        } else {
            if strcmp(a, b"--root\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as i32) < argc
            {
                i += 1;
                root = *argv.offset(i as isize);
            } else if strcmp(a, b"--cwd\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as i32) < argc
            {
                i += 1;
                cwd = *argv.offset(i as isize);
            } else if strcmp(a, b"--loader\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as i32) < argc
            {
                i += 1;
                loader = *argv.offset(i as isize);
            } else if strcmp(a, b"--bind\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as i32) < argc
                && nb < ENG_MAX_BINDS
            {
                i += 1;
                let c2rust_fresh0 = nb;
                nb += 1;
                binds[c2rust_fresh0 as usize] = *argv.offset(i as isize);
            } else if strcmp(a, b"--hide\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as i32) < argc
                && nh < ENG_MAX_HIDES
            {
                i += 1;
                let c2rust_fresh1 = nh;
                nh += 1;
                hides[c2rust_fresh1 as usize] = *argv.offset(i as isize);
            } else if strcmp(a, b"--uid\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as i32) < argc
            {
                i += 1;
                uid = strtol(
                    *argv.offset(i as isize),
                    ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                    10 as i32,
                );
            } else if strcmp(a, b"--gid\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as i32) < argc
            {
                i += 1;
                gid = strtol(
                    *argv.offset(i as isize),
                    ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                    10 as i32,
                );
            } else if strcmp(a, b"--user\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as i32) < argc
            {
                i += 1;
                let mut u: *const ::core::ffi::c_char = *argv.offset(i as isize);
                if strcmp(u, b"root\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                    gid = 0 as i64;
                    uid = gid;
                } else if strcmp(u, b"work\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                    gid = 1000 as i64;
                    uid = gid;
                } else {
                    fprintf(
                        stderr,
                        b"run: unknown user %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                        u,
                    );
                    return 2 as i32;
                }
            } else if strcmp(
                a,
                b"--no-default-binds\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                default_binds = 0 as i32;
            } else if strcmp(a, b"--binfmt\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as i32) < argc
                && nbf < ENG_MAX_BINFMT
            {
                i += 1;
                let c2rust_fresh2 = nbf;
                nbf += 1;
                binfmts[c2rust_fresh2 as usize] = *argv.offset(i as isize);
            } else if strcmp(
                a,
                b"--no-guest-binfmt\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                no_guest_binfmt = 1 as i32;
            } else if strcmp(a, b"--socket-dir\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as i32) < argc
            {
                i += 1;
                sockdir = *argv.offset(i as isize);
            } else if strcmp(a, b"--no-filemap\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                no_filemap = 1 as i32;
            } else if strcmp(
                a,
                b"--test-page-size\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
                && (i + 1 as i32) < argc
            {
                i += 1;
                test_pagesz = strtoul(
                    *argv.offset(i as isize),
                    ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                    0 as i32,
                ) as u32;
            } else if strcmp(a, b"--seccomp\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                seccomp = 1 as i32;
            } else if strcmp(a, b"--no-seccomp\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                seccomp = 0 as i32;
            } else if strcmp(
                a,
                b"--test-app-filter\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                test_app_filter = 1 as i32;
            } else if strcmp(a, b"--wait-all\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                wait_all = 1 as i32;
            } else if strcmp(a, b"-v\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                verbosity += 1;
            } else if strcmp(a, b"-vv\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                verbosity += 2 as i32;
            } else {
                fprintf(
                    stderr,
                    b"run: unknown option %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                    a,
                );
                usage();
                return 2 as i32;
            }
            i += 1;
        }
    }
    if i >= argc {
        fprintf(
            stderr,
            b"run: missing command\n\0".as_ptr() as *const ::core::ffi::c_char,
        );
        return 2 as i32;
    }
    eng_log_init(verbosity);
    let mut guest: *mut eng_guest = ::core::ptr::null_mut::<eng_guest>();
    let mut cmd: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut gargv: *mut *mut ::core::ffi::c_char = argv.offset(i as isize);
    if !root.is_null() {
        guest = eng_guest_open(root);
        if guest.is_null() {
            fprintf(
                stderr,
                b"run: cannot open guest root %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                root,
                strerror(*errno()),
            );
            return 125 as i32;
        }
        (*guest).no_filemap = no_filemap;
        (*guest).test_pagesz = test_pagesz;
        if !sockdir.is_null() {
            snprintf(
                &raw mut (*guest).sockdir as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                sockdir,
            );
        } else {
            let mut td: *const ::core::ffi::c_char =
                getenv(b"TMPDIR\0".as_ptr() as *const ::core::ffi::c_char);
            snprintf(
                &raw mut (*guest).sockdir as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s/wfs-%d\0".as_ptr() as *const ::core::ffi::c_char,
                if !td.is_null() && *td as i32 != 0 {
                    td
                } else {
                    b"/tmp\0".as_ptr() as *const ::core::ffi::c_char
                },
                getuid() as i32,
            );
        }
        if !loader.is_null() {
            snprintf(
                &raw mut (*guest).loader as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                loader,
            );
        } else if default_loader(
            &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
        ) != 0
        {
            fprintf(
                stderr,
                b"run: loader not found (set --loader or WORKFLOW_ENGINE_LOADER)\n\0".as_ptr()
                    as *const ::core::ffi::c_char,
            );
            return 125 as i32;
        }
        let mut real: [::core::ffi::c_char; 4096] = [0; 4096];
        if realpath(
            &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            &raw mut real as *mut ::core::ffi::c_char,
        )
        .is_null()
            || access(&raw mut real as *mut ::core::ffi::c_char, X_OK) != 0 as i32
        {
            fprintf(
                stderr,
                b"run: loader %s not executable\n\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            );
            return 125 as i32;
        }
        snprintf(
            &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut real as *mut ::core::ffi::c_char,
        );
        if eng_instance_lock(guest, 0 as i32) < 0 as i32 {
            fprintf(
                stderr,
                b"run: cannot register with %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                root,
                strerror(*errno()),
            );
            return 125 as i32;
        }
        if default_binds != 0 && eng_guest_default_binds(guest) != 0 {
            fprintf(
                stderr,
                b"run: default binds failed\n\0".as_ptr() as *const ::core::ffi::c_char,
            );
            return 125 as i32;
        }
        let mut k: i32 = 0 as i32;
        while k < nb {
            let mut spec: [::core::ffi::c_char; 8192] = [0; 8192];
            snprintf(
                &raw mut spec as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 8192]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                binds[k as usize],
            );
            let mut colon: *mut ::core::ffi::c_char =
                strrchr(&raw mut spec as *mut ::core::ffi::c_char, ':' as i32);
            if colon.is_null() {
                fprintf(
                    stderr,
                    b"run: --bind needs HOST:GUEST\n\0".as_ptr() as *const ::core::ffi::c_char,
                );
                return 2 as i32;
            }
            *colon = 0 as ::core::ffi::c_char;
            let mut rc: i32 = eng_guest_add_bind(
                guest,
                &raw mut spec as *mut ::core::ffi::c_char,
                colon.offset(1 as i32 as isize),
            );
            if rc != 0 {
                fprintf(
                    stderr,
                    b"run: bind %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                    binds[k as usize],
                    strerror(-rc),
                );
                return 125 as i32;
            }
            let mut bst: stat = platform_empty_stat();
            eng_guest_make_mountpoint(
                guest,
                colon.offset(1 as i32 as isize),
                (stat(&raw mut spec as *mut ::core::ffi::c_char, &raw mut bst) != 0 as i32
                    || bst.st_mode & S_IFMT as u32 == S_IFDIR as u32) as i32,
            );
            k += 1;
        }
        let mut k_0: i32 = 0 as i32;
        while k_0 < nh {
            eng_guest_add_hide(guest, hides[k_0 as usize]);
            k_0 += 1;
        }
        if no_guest_binfmt == 0 {
            eng_guest_load_binfmt_dirs(guest);
        }
        let mut k_1: i32 = 0 as i32;
        while k_1 < nbf {
            let mut rc_0: i32 = if *binfmts[k_1 as usize].offset(0isize) as i32 == '@' as i32 {
                eng_guest_load_binfmt_file(guest, binfmts[k_1 as usize].offset(1 as i32 as isize))
            } else {
                eng_guest_add_binfmt(guest, binfmts[k_1 as usize])
            };
            if rc_0 < 0 as i32 {
                fprintf(
                    stderr,
                    b"run: bad --binfmt %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                    binfmts[k_1 as usize],
                );
                return 2 as i32;
            }
            k_1 += 1;
        }
        if strchr(*gargv.offset(0isize), '/' as i32).is_null() {
            if guest_path_search(
                guest,
                *gargv.offset(0isize),
                environ,
                &raw mut cmd as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            ) != 0
            {
                fprintf(
                    stderr,
                    b"workflow-engine: %s: command not found\n\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    *gargv.offset(0isize),
                );
                return 127 as i32;
            }
        } else if *(*gargv.offset(0isize)).offset(0isize) as i32 != '/' as i32 {
            snprintf(
                &raw mut cmd as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s/%s\0".as_ptr() as *const ::core::ffi::c_char,
                if !cwd.is_null() {
                    cwd
                } else {
                    b"\0".as_ptr() as *const ::core::ffi::c_char
                },
                *gargv.offset(0isize),
            );
        } else {
            snprintf(
                &raw mut cmd as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                *gargv.offset(0isize),
            );
        }
    }
    let mut se: *const ::core::ffi::c_char =
        getenv(b"WORKFLOW_ENGINE_SECCOMP\0".as_ptr() as *const ::core::ffi::c_char);
    if !se.is_null() && *se as i32 == '0' as i32 {
        seccomp = 0 as i32;
    }
    static mut genv: [*mut ::core::ffi::c_char; 4096] =
        [::core::ptr::null_mut::<::core::ffi::c_char>(); 4096];
    let mut ne: i32 = 0 as i32;
    let mut e: *mut *mut ::core::ffi::c_char = environ;
    while !(*e).is_null() && ne < 4095 as i32 {
        if strncmp(
            *e,
            b"WORKFLOW_ENGINE_\0".as_ptr() as *const ::core::ffi::c_char,
            16 as usize,
        ) != 0
        {
            let c2rust_fresh3 = ne;
            ne += 1;
            genv[c2rust_fresh3 as usize] = *e;
        }
        e = e.offset(1);
    }
    genv[ne as usize] = ::core::ptr::null_mut::<::core::ffi::c_char>();
    if !guest.is_null() && eng_log_enabled(eng_log_level::ENG_LOG_DEBUG) != 0 {
        let mut k_2: i32 = 0 as i32;
        while k_2 < (*guest).nbinds {
            eng_logf!(
                eng_log_level::ENG_LOG_DEBUG,
                b"bind %s -> %s (owner %u:%u)\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*(&raw mut (*guest).binds as *mut eng_bind).offset(k_2 as isize)).guest
                    as *mut ::core::ffi::c_char,
                &raw mut (*(&raw mut (*guest).binds as *mut eng_bind).offset(k_2 as isize)).host
                    as *mut ::core::ffi::c_char,
                (*guest).binds[k_2 as usize].uid,
                (*guest).binds[k_2 as usize].gid,
            );
            k_2 += 1;
        }
    }
    let mut cfg: eng_run_cfg = eng_run_cfg {
        argv: gargv,
        exe: if !root.is_null() {
            &raw mut cmd as *mut ::core::ffi::c_char
        } else {
            ::core::ptr::null_mut::<::core::ffi::c_char>()
        },
        envp: if !root.is_null() {
            &raw mut genv as *mut *mut ::core::ffi::c_char
        } else {
            environ
        },
        cwd: cwd,
        guest: guest,
        use_seccomp_fastpath: seccomp,
        uid: uid as u32,
        gid: gid as u32,
        test_app_filter: test_app_filter,
        wait_all: wait_all,
    };
    let mut rc_1: i32 = eng_tracer_run(&raw mut cfg);
    return if rc_1 < 0 as i32 { 125 as i32 } else { rc_1 };
}
unsafe extern "C" fn cmd_probe() -> i32 {
    #[cfg(target_os = "linux")]
    printf(
        b"{\"arch\":\"%s\",\"pageSize\":%ld}\n\0".as_ptr() as *const ::core::ffi::c_char,
        ENG_ARCH_NAME.as_ptr(),
        sysconf(_SC_PAGESIZE as i32),
    );
    #[cfg(target_os = "android")]
    printf(
        b"{\"arch\":\"%s\",\"pageSize\":%ld}\n\0".as_ptr() as *const ::core::ffi::c_char,
        ENG_ARCH_NAME.as_ptr(),
        sysconf(_SC_PAGESIZE),
    );
    return 0 as i32;
}
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args_os()
        .map(|arg| {
            ::std::ffi::CString::new(std::os::unix::ffi::OsStrExt::as_bytes(arg.as_os_str()))
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut ::core::ffi::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .chain(::core::iter::once(::core::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as i32,
            args_ptrs.as_mut_ptr() as *mut *mut ::core::ffi::c_char,
        ) as i32)
    }
}
#[cfg(target_os = "linux")]
unsafe fn platform_empty_stat() -> stat {
    stat {
        st_dev: 0,
        st_ino: 0,
        st_nlink: 0,
        st_mode: 0,
        st_uid: 0,
        st_gid: 0,
        __pad0: 0,
        st_rdev: 0,
        st_size: 0,
        st_blksize: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __glibc_reserved: [0; 3],
    }
}
#[cfg(all(target_os = "android", target_arch = "x86_64"))]
unsafe fn platform_empty_stat() -> stat {
    stat {
        st_dev: 0,
        st_ino: 0,
        st_nlink: 0,
        st_mode: 0,
        st_uid: 0,
        st_gid: 0,
        __pad0: 0,
        st_rdev: 0,
        st_size: 0,
        st_blksize: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __pad3: [0; 3],
    }
}
#[cfg(target_arch = "aarch64")]
unsafe fn platform_empty_stat() -> stat {
    stat {
        st_dev: 0,
        st_ino: 0,
        st_mode: 0,
        st_nlink: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        __pad1: 0,
        st_size: 0,
        st_blksize: 0,
        __pad2: 0,
        st_blocks: 0,
        st_atim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        __unused4: 0,
        __unused5: 0,
    }
}
#[cfg(target_os = "android")]
#[repr(C)]
pub struct __sFILE {
    _opaque: [u8; 0],
}
