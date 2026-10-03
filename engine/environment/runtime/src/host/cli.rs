//! Compatibility CLI contract consumed by the Workspace Engine server.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
#[repr(C)]
pub struct _IO_wide_data {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct _IO_codecvt {
    _opaque: [u8; 0],
}
#[repr(C)]
pub struct _IO_marker {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    unsafe fn __errno_location() -> *mut ::core::ffi::c_int;
    unsafe fn dirname(__path: *mut ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    static mut stdout: *mut FILE;
    static mut stderr: *mut FILE;
    unsafe fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    unsafe fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn strtol(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    unsafe fn strtoul(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulong;
    unsafe fn getenv(__name: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    unsafe fn realpath(
        __name: *const ::core::ffi::c_char,
        __resolved: *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn strchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strtok_r(
        __s: *mut ::core::ffi::c_char,
        __delim: *const ::core::ffi::c_char,
        __save_ptr: *mut *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    unsafe fn strerror(__errnum: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
    unsafe fn stat(__file: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn lstat(__file: *const ::core::ffi::c_char, __buf: *mut stat) -> ::core::ffi::c_int;
    unsafe fn access(
        __name: *const ::core::ffi::c_char,
        __type: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn close(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    static mut environ: *mut *mut ::core::ffi::c_char;
    unsafe fn sysconf(__name: ::core::ffi::c_int) -> ::core::ffi::c_long;
    unsafe fn getuid() -> __uid_t;
    unsafe fn readlink(
        __path: *const ::core::ffi::c_char,
        __buf: *mut ::core::ffi::c_char,
        __len: size_t,
    ) -> ssize_t;
    unsafe fn eng_guest_open(root: *const ::core::ffi::c_char) -> *mut eng_guest;
    unsafe fn eng_guest_close(g: *mut eng_guest);
    unsafe fn eng_guest_add_bind(
        g: *mut eng_guest,
        host: *const ::core::ffi::c_char,
        guest: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_guest_default_binds(g: *mut eng_guest) -> ::core::ffi::c_int;
    unsafe fn eng_guest_add_hide(
        g: *mut eng_guest,
        guest: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_guest_make_mountpoint(
        g: *mut eng_guest,
        guest: *const ::core::ffi::c_char,
        is_dir: ::core::ffi::c_int,
    );
    unsafe fn eng_guest_add_binfmt(
        g: *mut eng_guest,
        rule: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_guest_load_binfmt_file(
        g: *mut eng_guest,
        host_path: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_guest_load_binfmt_dirs(g: *mut eng_guest) -> ::core::ffi::c_int;
    unsafe fn eng_guest_to_host(
        g: *const eng_guest,
        guest: *const ::core::ffi::c_char,
        host: *mut ::core::ffi::c_char,
        cap: size_t,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_install(o: *const eng_install_opts) -> ::core::ffi::c_int;
    unsafe fn eng_clone(
        src_generation: *const ::core::ffi::c_char,
        dst_generation: *const ::core::ffi::c_char,
        quiet: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_verify(
        generation: *const ::core::ffi::c_char,
        quiet: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_remove_tree(path: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    unsafe fn eng_log_init(verbosity: ::core::ffi::c_int);
    unsafe fn eng_log_enabled(lv: eng_log_level) -> ::core::ffi::c_int;
    unsafe fn eng_tracer_run(cfg: *const eng_run_cfg) -> ::core::ffi::c_int;
    unsafe fn eng_link_fsck(
        g: *mut eng_guest,
        repair: ::core::ffi::c_int,
        out: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    unsafe fn eng_instance_lock(
        g: *mut eng_guest,
        exclusive: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
}
pub type __uint64_t = u64;
pub type __dev_t = ::core::ffi::c_ulong;
pub type __uid_t = ::core::ffi::c_uint;
pub type __gid_t = ::core::ffi::c_uint;
pub type __ino_t = ::core::ffi::c_ulong;
pub type __mode_t = ::core::ffi::c_uint;
pub type __nlink_t = ::core::ffi::c_ulong;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
pub type __time_t = ::core::ffi::c_long;
pub type __blksize_t = ::core::ffi::c_long;
pub type __blkcnt_t = ::core::ffi::c_long;
pub type __syscall_slong_t = ::core::ffi::c_long;
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __time_t,
    pub tv_nsec: __syscall_slong_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: __dev_t,
    pub st_ino: __ino_t,
    pub st_nlink: __nlink_t,
    pub st_mode: __mode_t,
    pub st_uid: __uid_t,
    pub st_gid: __gid_t,
    pub __pad0: ::core::ffi::c_int,
    pub st_rdev: __dev_t,
    pub st_size: __off_t,
    pub st_blksize: __blksize_t,
    pub st_blocks: __blkcnt_t,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [__syscall_slong_t; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
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
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: [u8; 3],
    pub _short_backupbuf: [::core::ffi::c_char; 1],
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut ::core::ffi::c_void,
    pub _prevchain: *mut *mut _IO_FILE,
    pub _mode: ::core::ffi::c_int,
    pub _unused3: ::core::ffi::c_int,
    pub _total_written: __uint64_t,
    pub _unused2: [::core::ffi::c_char; 8],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
pub type ssize_t = isize;
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct C2Rust_Unnamed(pub ::core::ffi::c_uint);
impl C2Rust_Unnamed {
    pub const _SC_ARG_MAX: Self = Self(0);
    pub const _SC_CHILD_MAX: Self = Self(1);
    pub const _SC_CLK_TCK: Self = Self(2);
    pub const _SC_NGROUPS_MAX: Self = Self(3);
    pub const _SC_OPEN_MAX: Self = Self(4);
    pub const _SC_STREAM_MAX: Self = Self(5);
    pub const _SC_TZNAME_MAX: Self = Self(6);
    pub const _SC_JOB_CONTROL: Self = Self(7);
    pub const _SC_SAVED_IDS: Self = Self(8);
    pub const _SC_REALTIME_SIGNALS: Self = Self(9);
    pub const _SC_PRIORITY_SCHEDULING: Self = Self(10);
    pub const _SC_TIMERS: Self = Self(11);
    pub const _SC_ASYNCHRONOUS_IO: Self = Self(12);
    pub const _SC_PRIORITIZED_IO: Self = Self(13);
    pub const _SC_SYNCHRONIZED_IO: Self = Self(14);
    pub const _SC_FSYNC: Self = Self(15);
    pub const _SC_MAPPED_FILES: Self = Self(16);
    pub const _SC_MEMLOCK: Self = Self(17);
    pub const _SC_MEMLOCK_RANGE: Self = Self(18);
    pub const _SC_MEMORY_PROTECTION: Self = Self(19);
    pub const _SC_MESSAGE_PASSING: Self = Self(20);
    pub const _SC_SEMAPHORES: Self = Self(21);
    pub const _SC_SHARED_MEMORY_OBJECTS: Self = Self(22);
    pub const _SC_AIO_LISTIO_MAX: Self = Self(23);
    pub const _SC_AIO_MAX: Self = Self(24);
    pub const _SC_AIO_PRIO_DELTA_MAX: Self = Self(25);
    pub const _SC_DELAYTIMER_MAX: Self = Self(26);
    pub const _SC_MQ_OPEN_MAX: Self = Self(27);
    pub const _SC_MQ_PRIO_MAX: Self = Self(28);
    pub const _SC_VERSION: Self = Self(29);
    pub const _SC_PAGESIZE: Self = Self(30);
    pub const _SC_RTSIG_MAX: Self = Self(31);
    pub const _SC_SEM_NSEMS_MAX: Self = Self(32);
    pub const _SC_SEM_VALUE_MAX: Self = Self(33);
    pub const _SC_SIGQUEUE_MAX: Self = Self(34);
    pub const _SC_TIMER_MAX: Self = Self(35);
    pub const _SC_BC_BASE_MAX: Self = Self(36);
    pub const _SC_BC_DIM_MAX: Self = Self(37);
    pub const _SC_BC_SCALE_MAX: Self = Self(38);
    pub const _SC_BC_STRING_MAX: Self = Self(39);
    pub const _SC_COLL_WEIGHTS_MAX: Self = Self(40);
    pub const _SC_EQUIV_CLASS_MAX: Self = Self(41);
    pub const _SC_EXPR_NEST_MAX: Self = Self(42);
    pub const _SC_LINE_MAX: Self = Self(43);
    pub const _SC_RE_DUP_MAX: Self = Self(44);
    pub const _SC_CHARCLASS_NAME_MAX: Self = Self(45);
    pub const _SC_2_VERSION: Self = Self(46);
    pub const _SC_2_C_BIND: Self = Self(47);
    pub const _SC_2_C_DEV: Self = Self(48);
    pub const _SC_2_FORT_DEV: Self = Self(49);
    pub const _SC_2_FORT_RUN: Self = Self(50);
    pub const _SC_2_SW_DEV: Self = Self(51);
    pub const _SC_2_LOCALEDEF: Self = Self(52);
    pub const _SC_PII: Self = Self(53);
    pub const _SC_PII_XTI: Self = Self(54);
    pub const _SC_PII_SOCKET: Self = Self(55);
    pub const _SC_PII_INTERNET: Self = Self(56);
    pub const _SC_PII_OSI: Self = Self(57);
    pub const _SC_POLL: Self = Self(58);
    pub const _SC_SELECT: Self = Self(59);
    pub const _SC_UIO_MAXIOV: Self = Self(60);
    pub const _SC_IOV_MAX: Self = Self(60);
    pub const _SC_PII_INTERNET_STREAM: Self = Self(61);
    pub const _SC_PII_INTERNET_DGRAM: Self = Self(62);
    pub const _SC_PII_OSI_COTS: Self = Self(63);
    pub const _SC_PII_OSI_CLTS: Self = Self(64);
    pub const _SC_PII_OSI_M: Self = Self(65);
    pub const _SC_T_IOV_MAX: Self = Self(66);
    pub const _SC_THREADS: Self = Self(67);
    pub const _SC_THREAD_SAFE_FUNCTIONS: Self = Self(68);
    pub const _SC_GETGR_R_SIZE_MAX: Self = Self(69);
    pub const _SC_GETPW_R_SIZE_MAX: Self = Self(70);
    pub const _SC_LOGIN_NAME_MAX: Self = Self(71);
    pub const _SC_TTY_NAME_MAX: Self = Self(72);
    pub const _SC_THREAD_DESTRUCTOR_ITERATIONS: Self = Self(73);
    pub const _SC_THREAD_KEYS_MAX: Self = Self(74);
    pub const _SC_THREAD_STACK_MIN: Self = Self(75);
    pub const _SC_THREAD_THREADS_MAX: Self = Self(76);
    pub const _SC_THREAD_ATTR_STACKADDR: Self = Self(77);
    pub const _SC_THREAD_ATTR_STACKSIZE: Self = Self(78);
    pub const _SC_THREAD_PRIORITY_SCHEDULING: Self = Self(79);
    pub const _SC_THREAD_PRIO_INHERIT: Self = Self(80);
    pub const _SC_THREAD_PRIO_PROTECT: Self = Self(81);
    pub const _SC_THREAD_PROCESS_SHARED: Self = Self(82);
    pub const _SC_NPROCESSORS_CONF: Self = Self(83);
    pub const _SC_NPROCESSORS_ONLN: Self = Self(84);
    pub const _SC_PHYS_PAGES: Self = Self(85);
    pub const _SC_AVPHYS_PAGES: Self = Self(86);
    pub const _SC_ATEXIT_MAX: Self = Self(87);
    pub const _SC_PASS_MAX: Self = Self(88);
    pub const _SC_XOPEN_VERSION: Self = Self(89);
    pub const _SC_XOPEN_XCU_VERSION: Self = Self(90);
    pub const _SC_XOPEN_UNIX: Self = Self(91);
    pub const _SC_XOPEN_CRYPT: Self = Self(92);
    pub const _SC_XOPEN_ENH_I18N: Self = Self(93);
    pub const _SC_XOPEN_SHM: Self = Self(94);
    pub const _SC_2_CHAR_TERM: Self = Self(95);
    pub const _SC_2_C_VERSION: Self = Self(96);
    pub const _SC_2_UPE: Self = Self(97);
    pub const _SC_XOPEN_XPG2: Self = Self(98);
    pub const _SC_XOPEN_XPG3: Self = Self(99);
    pub const _SC_XOPEN_XPG4: Self = Self(100);
    pub const _SC_CHAR_BIT: Self = Self(101);
    pub const _SC_CHAR_MAX: Self = Self(102);
    pub const _SC_CHAR_MIN: Self = Self(103);
    pub const _SC_INT_MAX: Self = Self(104);
    pub const _SC_INT_MIN: Self = Self(105);
    pub const _SC_LONG_BIT: Self = Self(106);
    pub const _SC_WORD_BIT: Self = Self(107);
    pub const _SC_MB_LEN_MAX: Self = Self(108);
    pub const _SC_NZERO: Self = Self(109);
    pub const _SC_SSIZE_MAX: Self = Self(110);
    pub const _SC_SCHAR_MAX: Self = Self(111);
    pub const _SC_SCHAR_MIN: Self = Self(112);
    pub const _SC_SHRT_MAX: Self = Self(113);
    pub const _SC_SHRT_MIN: Self = Self(114);
    pub const _SC_UCHAR_MAX: Self = Self(115);
    pub const _SC_UINT_MAX: Self = Self(116);
    pub const _SC_ULONG_MAX: Self = Self(117);
    pub const _SC_USHRT_MAX: Self = Self(118);
    pub const _SC_NL_ARGMAX: Self = Self(119);
    pub const _SC_NL_LANGMAX: Self = Self(120);
    pub const _SC_NL_MSGMAX: Self = Self(121);
    pub const _SC_NL_NMAX: Self = Self(122);
    pub const _SC_NL_SETMAX: Self = Self(123);
    pub const _SC_NL_TEXTMAX: Self = Self(124);
    pub const _SC_XBS5_ILP32_OFF32: Self = Self(125);
    pub const _SC_XBS5_ILP32_OFFBIG: Self = Self(126);
    pub const _SC_XBS5_LP64_OFF64: Self = Self(127);
    pub const _SC_XBS5_LPBIG_OFFBIG: Self = Self(128);
    pub const _SC_XOPEN_LEGACY: Self = Self(129);
    pub const _SC_XOPEN_REALTIME: Self = Self(130);
    pub const _SC_XOPEN_REALTIME_THREADS: Self = Self(131);
    pub const _SC_ADVISORY_INFO: Self = Self(132);
    pub const _SC_BARRIERS: Self = Self(133);
    pub const _SC_BASE: Self = Self(134);
    pub const _SC_C_LANG_SUPPORT: Self = Self(135);
    pub const _SC_C_LANG_SUPPORT_R: Self = Self(136);
    pub const _SC_CLOCK_SELECTION: Self = Self(137);
    pub const _SC_CPUTIME: Self = Self(138);
    pub const _SC_THREAD_CPUTIME: Self = Self(139);
    pub const _SC_DEVICE_IO: Self = Self(140);
    pub const _SC_DEVICE_SPECIFIC: Self = Self(141);
    pub const _SC_DEVICE_SPECIFIC_R: Self = Self(142);
    pub const _SC_FD_MGMT: Self = Self(143);
    pub const _SC_FIFO: Self = Self(144);
    pub const _SC_PIPE: Self = Self(145);
    pub const _SC_FILE_ATTRIBUTES: Self = Self(146);
    pub const _SC_FILE_LOCKING: Self = Self(147);
    pub const _SC_FILE_SYSTEM: Self = Self(148);
    pub const _SC_MONOTONIC_CLOCK: Self = Self(149);
    pub const _SC_MULTI_PROCESS: Self = Self(150);
    pub const _SC_SINGLE_PROCESS: Self = Self(151);
    pub const _SC_NETWORKING: Self = Self(152);
    pub const _SC_READER_WRITER_LOCKS: Self = Self(153);
    pub const _SC_SPIN_LOCKS: Self = Self(154);
    pub const _SC_REGEXP: Self = Self(155);
    pub const _SC_REGEX_VERSION: Self = Self(156);
    pub const _SC_SHELL: Self = Self(157);
    pub const _SC_SIGNALS: Self = Self(158);
    pub const _SC_SPAWN: Self = Self(159);
    pub const _SC_SPORADIC_SERVER: Self = Self(160);
    pub const _SC_THREAD_SPORADIC_SERVER: Self = Self(161);
    pub const _SC_SYSTEM_DATABASE: Self = Self(162);
    pub const _SC_SYSTEM_DATABASE_R: Self = Self(163);
    pub const _SC_TIMEOUTS: Self = Self(164);
    pub const _SC_TYPED_MEMORY_OBJECTS: Self = Self(165);
    pub const _SC_USER_GROUPS: Self = Self(166);
    pub const _SC_USER_GROUPS_R: Self = Self(167);
    pub const _SC_2_PBS: Self = Self(168);
    pub const _SC_2_PBS_ACCOUNTING: Self = Self(169);
    pub const _SC_2_PBS_LOCATE: Self = Self(170);
    pub const _SC_2_PBS_MESSAGE: Self = Self(171);
    pub const _SC_2_PBS_TRACK: Self = Self(172);
    pub const _SC_SYMLOOP_MAX: Self = Self(173);
    pub const _SC_STREAMS: Self = Self(174);
    pub const _SC_2_PBS_CHECKPOINT: Self = Self(175);
    pub const _SC_V6_ILP32_OFF32: Self = Self(176);
    pub const _SC_V6_ILP32_OFFBIG: Self = Self(177);
    pub const _SC_V6_LP64_OFF64: Self = Self(178);
    pub const _SC_V6_LPBIG_OFFBIG: Self = Self(179);
    pub const _SC_HOST_NAME_MAX: Self = Self(180);
    pub const _SC_TRACE: Self = Self(181);
    pub const _SC_TRACE_EVENT_FILTER: Self = Self(182);
    pub const _SC_TRACE_INHERIT: Self = Self(183);
    pub const _SC_TRACE_LOG: Self = Self(184);
    pub const _SC_LEVEL1_ICACHE_SIZE: Self = Self(185);
    pub const _SC_LEVEL1_ICACHE_ASSOC: Self = Self(186);
    pub const _SC_LEVEL1_ICACHE_LINESIZE: Self = Self(187);
    pub const _SC_LEVEL1_DCACHE_SIZE: Self = Self(188);
    pub const _SC_LEVEL1_DCACHE_ASSOC: Self = Self(189);
    pub const _SC_LEVEL1_DCACHE_LINESIZE: Self = Self(190);
    pub const _SC_LEVEL2_CACHE_SIZE: Self = Self(191);
    pub const _SC_LEVEL2_CACHE_ASSOC: Self = Self(192);
    pub const _SC_LEVEL2_CACHE_LINESIZE: Self = Self(193);
    pub const _SC_LEVEL3_CACHE_SIZE: Self = Self(194);
    pub const _SC_LEVEL3_CACHE_ASSOC: Self = Self(195);
    pub const _SC_LEVEL3_CACHE_LINESIZE: Self = Self(196);
    pub const _SC_LEVEL4_CACHE_SIZE: Self = Self(197);
    pub const _SC_LEVEL4_CACHE_ASSOC: Self = Self(198);
    pub const _SC_LEVEL4_CACHE_LINESIZE: Self = Self(199);
    pub const _SC_IPV6: Self = Self(235);
    pub const _SC_RAW_SOCKETS: Self = Self(236);
    pub const _SC_V7_ILP32_OFF32: Self = Self(237);
    pub const _SC_V7_ILP32_OFFBIG: Self = Self(238);
    pub const _SC_V7_LP64_OFF64: Self = Self(239);
    pub const _SC_V7_LPBIG_OFFBIG: Self = Self(240);
    pub const _SC_SS_REPL_MAX: Self = Self(241);
    pub const _SC_TRACE_EVENT_NAME_MAX: Self = Self(242);
    pub const _SC_TRACE_NAME_MAX: Self = Self(243);
    pub const _SC_TRACE_SYS_MAX: Self = Self(244);
    pub const _SC_TRACE_USER_EVENT_MAX: Self = Self(245);
    pub const _SC_XOPEN_STREAMS: Self = Self(246);
    pub const _SC_THREAD_ROBUST_PRIO_INHERIT: Self = Self(247);
    pub const _SC_THREAD_ROBUST_PRIO_PROTECT: Self = Self(248);
    pub const _SC_MINSIGSTKSZ: Self = Self(249);
    pub const _SC_SIGSTKSZ: Self = Self(250);
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_binfmt {
    pub name: [::core::ffi::c_char; 64],
    pub r#type: ::core::ffi::c_char,
    pub offset: ::core::ffi::c_uint,
    pub magic: [::core::ffi::c_uchar; 128],
    pub mask: [::core::ffi::c_uchar; 128],
    pub len: ::core::ffi::c_uint,
    pub ext: [::core::ffi::c_char; 64],
    pub interp: [::core::ffi::c_char; 4096],
    pub preserve_argv0: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_bind {
    pub guest: [::core::ffi::c_char; 4096],
    pub host: [::core::ffi::c_char; 4096],
    pub glen: size_t,
    pub hlen: size_t,
    pub uid: ::core::ffi::c_uint,
    pub gid: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_guest {
    pub root: [::core::ffi::c_char; 4096],
    pub rootlen: size_t,
    pub rootfd: ::core::ffi::c_int,
    pub binds: [eng_bind; 32],
    pub nbinds: ::core::ffi::c_int,
    pub hides: [[::core::ffi::c_char; 4096]; 16],
    pub nhides: ::core::ffi::c_int,
    pub binfmt: [eng_binfmt; 32],
    pub nbinfmt: ::core::ffi::c_int,
    pub loader: [::core::ffi::c_char; 4096],
    pub sockdir: [::core::ffi::c_char; 4096],
    pub no_filemap: ::core::ffi::c_int,
    pub test_pagesz: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_install_opts {
    pub image: *const ::core::ffi::c_char,
    pub sha256: *const ::core::ffi::c_char,
    pub index: *const ::core::ffi::c_char,
    pub target: *const ::core::ffi::c_char,
    pub profile: *const ::core::ffi::c_char,
    pub quiet: ::core::ffi::c_int,
    pub keep_partial: ::core::ffi::c_int,
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_log_level(pub ::core::ffi::c_uint);
impl eng_log_level {
    pub const ENG_LOG_ERROR: Self = Self(0);
    pub const ENG_LOG_WARN: Self = Self(1);
    pub const ENG_LOG_INFO: Self = Self(2);
    pub const ENG_LOG_DEBUG: Self = Self(3);
    pub const ENG_LOG_TRACE: Self = Self(4);
}
pub type uint32_t = u32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_run_cfg {
    pub argv: *mut *mut ::core::ffi::c_char,
    pub exe: *const ::core::ffi::c_char,
    pub envp: *mut *mut ::core::ffi::c_char,
    pub cwd: *const ::core::ffi::c_char,
    pub guest: *mut eng_guest,
    pub use_seccomp_fastpath: ::core::ffi::c_int,
    pub uid: uint32_t,
    pub gid: uint32_t,
    pub test_app_filter: ::core::ffi::c_int,
    pub wait_all: ::core::ffi::c_int,
}
pub const ENOENT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const EAGAIN: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const EWOULDBLOCK: ::core::ffi::c_int = EAGAIN;
pub const __S_IFMT: ::core::ffi::c_int = 0o170000 as ::core::ffi::c_int;
pub const X_OK: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ENG_MAX_BINDS: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const ENG_MAX_HIDES: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const ENG_MAX_BINFMT: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const ENG_ARCH_NAME: [::core::ffi::c_char; 7] =
    unsafe { ::core::mem::transmute::<[u8; 7], [::core::ffi::c_char; 7]>(*b"x86_64\0") };
unsafe extern "C" fn usage() {
    fprintf(
        stderr,
        b"usage: workflow-engine run [--root DIR] [--cwd GUESTDIR] [--bind HOST:GUEST]...\n                           [--hide GUESTPATH]... [--loader FILE] [--user work|root]\n                           [--uid N] [--gid N] [--no-default-binds] [--no-filemap]\n                           [--wait-all] [-v] -- CMD [ARGS...]\n       workflow-engine probe\n       workflow-engine fsck --root DIR [--repair]\n       workflow-engine install --image FILE|- (--index image.json | --sha256 HEX) --target GEN\n                               [--profile workspace|base|any] [--quiet]\n       workflow-engine clone --from GEN --to GEN\n       workflow-engine verify --generation GEN\n       workflow-engine remove --generation GEN\n\0"
            .as_ptr() as *const ::core::ffi::c_char,
    );
}
unsafe extern "C" fn default_loader(
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut env: *const ::core::ffi::c_char =
        getenv(b"WORKFLOW_ENGINE_LOADER\0".as_ptr() as *const ::core::ffi::c_char);
    if !env.is_null() && *env as ::core::ffi::c_int != 0 {
        snprintf(
            out,
            cap,
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            env,
        );
        return 0 as ::core::ffi::c_int;
    }
    let mut self_0: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut n: ssize_t = readlink(
        b"/proc/self/exe\0".as_ptr() as *const ::core::ffi::c_char,
        &raw mut self_0 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 4096]>().wrapping_sub(1 as size_t),
    );
    if n <= 0 as ssize_t {
        return -1 as ::core::ffi::c_int;
    }
    self_0[n as usize] = 0 as ::core::ffi::c_char;
    let mut dir: *mut ::core::ffi::c_char = dirname(&raw mut self_0 as *mut ::core::ffi::c_char);
    snprintf(
        out,
        cap,
        b"%s/libworkflow-loader.so\0".as_ptr() as *const ::core::ffi::c_char,
        dir,
    );
    if access(out, X_OK) == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    snprintf(
        out,
        cap,
        b"%s/loader\0".as_ptr() as *const ::core::ffi::c_char,
        dir,
    );
    return if access(out, X_OK) == 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_int
    } else {
        -1 as ::core::ffi::c_int
    };
}
unsafe extern "C" fn guest_path_search(
    mut g: *mut eng_guest,
    mut name: *const ::core::ffi::c_char,
    mut envp: *mut *mut ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_char,
    mut cap: size_t,
) -> ::core::ffi::c_int {
    let mut path: *const ::core::ffi::c_char =
        b"/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\0".as_ptr()
            as *const ::core::ffi::c_char;
    let mut e: *mut *mut ::core::ffi::c_char = envp;
    while !e.is_null() && !(*e).is_null() {
        if strncmp(
            *e,
            b"PATH=\0".as_ptr() as *const ::core::ffi::c_char,
            5 as size_t,
        ) == 0
        {
            path = (*e).offset(5 as ::core::ffi::c_int as isize);
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
        if *dir.offset(0isize) as ::core::ffi::c_int == '/' as ::core::ffi::c_int {
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
                let mut st: stat = stat {
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
                };
                if stat(&raw mut hp as *mut ::core::ffi::c_char, &raw mut st)
                    == 0 as ::core::ffi::c_int
                    && st.st_mode & __S_IFMT as __mode_t == 0o100000 as __mode_t
                {
                    snprintf(
                        out,
                        cap,
                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut gp as *mut ::core::ffi::c_char,
                    );
                    return 0 as ::core::ffi::c_int;
                }
                if lstat(&raw mut hp as *mut ::core::ffi::c_char, &raw mut st)
                    == 0 as ::core::ffi::c_int
                    && st.st_mode & __S_IFMT as __mode_t == 0o120000 as __mode_t
                {
                    snprintf(
                        out,
                        cap,
                        b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                        &raw mut gp as *mut ::core::ffi::c_char,
                    );
                    return 0 as ::core::ffi::c_int;
                }
            }
        }
        dir = strtok_r(
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            b":\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut save,
        );
    }
    return -1 as ::core::ffi::c_int;
}
unsafe extern "C" fn cmd_fsck(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut root: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut repair: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    while i < argc {
        if strcmp(
            *argv.offset(i as isize),
            b"--root\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
            && (i + 1 as ::core::ffi::c_int) < argc
        {
            i += 1;
            root = *argv.offset(i as isize);
        } else if strcmp(
            *argv.offset(i as isize),
            b"--repair\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        {
            repair = 1 as ::core::ffi::c_int;
        } else {
            usage();
            return 2 as ::core::ffi::c_int;
        }
        i += 1;
    }
    if root.is_null() {
        usage();
        return 2 as ::core::ffi::c_int;
    }
    let mut g: *mut eng_guest = eng_guest_open(root);
    if g.is_null() {
        fprintf(
            stderr,
            b"fsck: cannot open %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            root,
            strerror(*__errno_location()),
        );
        return 125 as ::core::ffi::c_int;
    }
    let mut lk: ::core::ffi::c_int = eng_instance_lock(g, repair);
    if lk < 0 as ::core::ffi::c_int {
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
            3 as ::core::ffi::c_int
        } else {
            125 as ::core::ffi::c_int
        };
    }
    let mut rc: ::core::ffi::c_int = eng_link_fsck(g, repair, stdout as *mut ::core::ffi::c_void);
    close(lk);
    return if rc < 0 as ::core::ffi::c_int {
        125 as ::core::ffi::c_int
    } else if rc > 0 as ::core::ffi::c_int {
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
}
unsafe extern "C" fn cmd_lifecycle(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
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
    let mut i: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    while i < argc {
        let mut a: *const ::core::ffi::c_char = *argv.offset(i as isize);
        let mut more: ::core::ffi::c_int =
            ((i + 1 as ::core::ffi::c_int) < argc) as ::core::ffi::c_int;
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
            o.quiet = 1 as ::core::ffi::c_int;
        } else if strcmp(
            a,
            b"--keep-partial\0".as_ptr() as *const ::core::ffi::c_char,
        ) == 0
        {
            o.keep_partial = 1 as ::core::ffi::c_int;
        } else {
            fprintf(
                stderr,
                b"%s: unknown option %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                *argv.offset(1isize),
                a,
            );
            usage();
            return 2 as ::core::ffi::c_int;
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
            return 2 as ::core::ffi::c_int;
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
            return 2 as ::core::ffi::c_int;
        }
        return eng_clone(from, o.target, o.quiet);
    }
    if r#gen.is_null() {
        usage();
        return 2 as ::core::ffi::c_int;
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
        let mut lk: ::core::ffi::c_int = eng_instance_lock(g, 1 as ::core::ffi::c_int);
        eng_guest_close(g);
        if lk == -EWOULDBLOCK {
            fprintf(
                stderr,
                b"remove: generation in use by a running engine\n\0".as_ptr()
                    as *const ::core::ffi::c_char,
            );
            return 3 as ::core::ffi::c_int;
        }
        if lk >= 0 as ::core::ffi::c_int {
            close(lk);
        }
    }
    let mut rc: ::core::ffi::c_int = eng_remove_tree(r#gen);
    if rc != 0 && rc != -ENOENT {
        fprintf(
            stderr,
            b"remove: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
            strerror(-rc),
        );
        return 74 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if argc < 2 as ::core::ffi::c_int {
        usage();
        return 2 as ::core::ffi::c_int;
    }
    if strcmp(
        *argv.offset(1isize),
        b"probe\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        eng_log_init(0 as ::core::ffi::c_int);
        return cmd_probe();
    }
    if strcmp(
        *argv.offset(1isize),
        b"fsck\0".as_ptr() as *const ::core::ffi::c_char,
    ) == 0
    {
        eng_log_init(0 as ::core::ffi::c_int);
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
        eng_log_init(0 as ::core::ffi::c_int);
        return cmd_lifecycle(argc, argv);
    }
    if strcmp(
        *argv.offset(1isize),
        b"run\0".as_ptr() as *const ::core::ffi::c_char,
    ) != 0
    {
        usage();
        return 2 as ::core::ffi::c_int;
    }
    let mut root: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut cwd: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut loader: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut binds: [*const ::core::ffi::c_char; 32] =
        [::core::ptr::null::<::core::ffi::c_char>(); 32];
    let mut hides: [*const ::core::ffi::c_char; 16] =
        [::core::ptr::null::<::core::ffi::c_char>(); 16];
    let mut nb: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut nh: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut verbosity: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut default_binds: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut no_filemap: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut seccomp: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut test_pagesz: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut test_app_filter: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut wait_all: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut binfmts: [*const ::core::ffi::c_char; 32] =
        [::core::ptr::null::<::core::ffi::c_char>(); 32];
    let mut nbf: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut no_guest_binfmt: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut sockdir: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut uid: ::core::ffi::c_long = 1000 as ::core::ffi::c_long;
    let mut gid: ::core::ffi::c_long = 1000 as ::core::ffi::c_long;
    let mut i: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    while i < argc {
        let mut a: *const ::core::ffi::c_char = *argv.offset(i as isize);
        if strcmp(a, b"--\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
            i += 1;
            break;
        } else {
            if strcmp(a, b"--root\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                root = *argv.offset(i as isize);
            } else if strcmp(a, b"--cwd\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                cwd = *argv.offset(i as isize);
            } else if strcmp(a, b"--loader\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                loader = *argv.offset(i as isize);
            } else if strcmp(a, b"--bind\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
                && nb < ENG_MAX_BINDS
            {
                i += 1;
                let c2rust_fresh0 = nb;
                nb += 1;
                binds[c2rust_fresh0 as usize] = *argv.offset(i as isize);
            } else if strcmp(a, b"--hide\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
                && nh < ENG_MAX_HIDES
            {
                i += 1;
                let c2rust_fresh1 = nh;
                nh += 1;
                hides[c2rust_fresh1 as usize] = *argv.offset(i as isize);
            } else if strcmp(a, b"--uid\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                uid = strtol(
                    *argv.offset(i as isize),
                    ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                    10 as ::core::ffi::c_int,
                );
            } else if strcmp(a, b"--gid\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                gid = strtol(
                    *argv.offset(i as isize),
                    ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                    10 as ::core::ffi::c_int,
                );
            } else if strcmp(a, b"--user\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                let mut u: *const ::core::ffi::c_char = *argv.offset(i as isize);
                if strcmp(u, b"root\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                    gid = 0 as ::core::ffi::c_long;
                    uid = gid;
                } else if strcmp(u, b"work\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                    gid = 1000 as ::core::ffi::c_long;
                    uid = gid;
                } else {
                    fprintf(
                        stderr,
                        b"run: unknown user %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                        u,
                    );
                    return 2 as ::core::ffi::c_int;
                }
            } else if strcmp(
                a,
                b"--no-default-binds\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                default_binds = 0 as ::core::ffi::c_int;
            } else if strcmp(a, b"--binfmt\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
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
                no_guest_binfmt = 1 as ::core::ffi::c_int;
            } else if strcmp(a, b"--socket-dir\0".as_ptr() as *const ::core::ffi::c_char) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                sockdir = *argv.offset(i as isize);
            } else if strcmp(a, b"--no-filemap\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                no_filemap = 1 as ::core::ffi::c_int;
            } else if strcmp(
                a,
                b"--test-page-size\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
                && (i + 1 as ::core::ffi::c_int) < argc
            {
                i += 1;
                test_pagesz = strtoul(
                    *argv.offset(i as isize),
                    ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                    0 as ::core::ffi::c_int,
                ) as ::core::ffi::c_uint;
            } else if strcmp(a, b"--seccomp\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                seccomp = 1 as ::core::ffi::c_int;
            } else if strcmp(a, b"--no-seccomp\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                seccomp = 0 as ::core::ffi::c_int;
            } else if strcmp(
                a,
                b"--test-app-filter\0".as_ptr() as *const ::core::ffi::c_char,
            ) == 0
            {
                test_app_filter = 1 as ::core::ffi::c_int;
            } else if strcmp(a, b"--wait-all\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                wait_all = 1 as ::core::ffi::c_int;
            } else if strcmp(a, b"-v\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                verbosity += 1;
            } else if strcmp(a, b"-vv\0".as_ptr() as *const ::core::ffi::c_char) == 0 {
                verbosity += 2 as ::core::ffi::c_int;
            } else {
                fprintf(
                    stderr,
                    b"run: unknown option %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                    a,
                );
                usage();
                return 2 as ::core::ffi::c_int;
            }
            i += 1;
        }
    }
    if i >= argc {
        fprintf(
            stderr,
            b"run: missing command\n\0".as_ptr() as *const ::core::ffi::c_char,
        );
        return 2 as ::core::ffi::c_int;
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
                strerror(*__errno_location()),
            );
            return 125 as ::core::ffi::c_int;
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
                if !td.is_null() && *td as ::core::ffi::c_int != 0 {
                    td
                } else {
                    b"/tmp\0".as_ptr() as *const ::core::ffi::c_char
                },
                getuid() as ::core::ffi::c_int,
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
            return 125 as ::core::ffi::c_int;
        }
        let mut real: [::core::ffi::c_char; 4096] = [0; 4096];
        if realpath(
            &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            &raw mut real as *mut ::core::ffi::c_char,
        )
        .is_null()
            || access(&raw mut real as *mut ::core::ffi::c_char, X_OK) != 0 as ::core::ffi::c_int
        {
            fprintf(
                stderr,
                b"run: loader %s not executable\n\0".as_ptr() as *const ::core::ffi::c_char,
                &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            );
            return 125 as ::core::ffi::c_int;
        }
        snprintf(
            &raw mut (*guest).loader as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 4096]>(),
            b"%s\0".as_ptr() as *const ::core::ffi::c_char,
            &raw mut real as *mut ::core::ffi::c_char,
        );
        if eng_instance_lock(guest, 0 as ::core::ffi::c_int) < 0 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"run: cannot register with %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                root,
                strerror(*__errno_location()),
            );
            return 125 as ::core::ffi::c_int;
        }
        if default_binds != 0 && eng_guest_default_binds(guest) != 0 {
            fprintf(
                stderr,
                b"run: default binds failed\n\0".as_ptr() as *const ::core::ffi::c_char,
            );
            return 125 as ::core::ffi::c_int;
        }
        let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while k < nb {
            let mut spec: [::core::ffi::c_char; 8192] = [0; 8192];
            snprintf(
                &raw mut spec as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 8192]>(),
                b"%s\0".as_ptr() as *const ::core::ffi::c_char,
                binds[k as usize],
            );
            let mut colon: *mut ::core::ffi::c_char = strrchr(
                &raw mut spec as *mut ::core::ffi::c_char,
                ':' as ::core::ffi::c_int,
            );
            if colon.is_null() {
                fprintf(
                    stderr,
                    b"run: --bind needs HOST:GUEST\n\0".as_ptr() as *const ::core::ffi::c_char,
                );
                return 2 as ::core::ffi::c_int;
            }
            *colon = 0 as ::core::ffi::c_char;
            let mut rc: ::core::ffi::c_int = eng_guest_add_bind(
                guest,
                &raw mut spec as *mut ::core::ffi::c_char,
                colon.offset(1 as ::core::ffi::c_int as isize),
            );
            if rc != 0 {
                fprintf(
                    stderr,
                    b"run: bind %s: %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                    binds[k as usize],
                    strerror(-rc),
                );
                return 125 as ::core::ffi::c_int;
            }
            let mut bst: stat = stat {
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
            };
            eng_guest_make_mountpoint(
                guest,
                colon.offset(1 as ::core::ffi::c_int as isize),
                (stat(&raw mut spec as *mut ::core::ffi::c_char, &raw mut bst)
                    != 0 as ::core::ffi::c_int
                    || bst.st_mode & __S_IFMT as __mode_t == 0o40000 as __mode_t)
                    as ::core::ffi::c_int,
            );
            k += 1;
        }
        let mut k_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while k_0 < nh {
            eng_guest_add_hide(guest, hides[k_0 as usize]);
            k_0 += 1;
        }
        if no_guest_binfmt == 0 {
            eng_guest_load_binfmt_dirs(guest);
        }
        let mut k_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while k_1 < nbf {
            let mut rc_0: ::core::ffi::c_int = if *binfmts[k_1 as usize].offset(0isize)
                as ::core::ffi::c_int
                == '@' as ::core::ffi::c_int
            {
                eng_guest_load_binfmt_file(
                    guest,
                    binfmts[k_1 as usize].offset(1 as ::core::ffi::c_int as isize),
                )
            } else {
                eng_guest_add_binfmt(guest, binfmts[k_1 as usize])
            };
            if rc_0 < 0 as ::core::ffi::c_int {
                fprintf(
                    stderr,
                    b"run: bad --binfmt %s\n\0".as_ptr() as *const ::core::ffi::c_char,
                    binfmts[k_1 as usize],
                );
                return 2 as ::core::ffi::c_int;
            }
            k_1 += 1;
        }
        if strchr(*gargv.offset(0isize), '/' as ::core::ffi::c_int).is_null() {
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
                return 127 as ::core::ffi::c_int;
            }
        } else if *(*gargv.offset(0isize)).offset(0isize) as ::core::ffi::c_int
            != '/' as ::core::ffi::c_int
        {
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
    if !se.is_null() && *se as ::core::ffi::c_int == '0' as ::core::ffi::c_int {
        seccomp = 0 as ::core::ffi::c_int;
    }
    static mut genv: [*mut ::core::ffi::c_char; 4096] =
        [::core::ptr::null_mut::<::core::ffi::c_char>(); 4096];
    let mut ne: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut e: *mut *mut ::core::ffi::c_char = environ;
    while !(*e).is_null() && ne < 4095 as ::core::ffi::c_int {
        if strncmp(
            *e,
            b"WORKFLOW_ENGINE_\0".as_ptr() as *const ::core::ffi::c_char,
            16 as size_t,
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
        let mut k_2: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
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
        uid: uid as uint32_t,
        gid: gid as uint32_t,
        test_app_filter: test_app_filter,
        wait_all: wait_all,
    };
    let mut rc_1: ::core::ffi::c_int = eng_tracer_run(&raw mut cfg);
    return if rc_1 < 0 as ::core::ffi::c_int {
        125 as ::core::ffi::c_int
    } else {
        rc_1
    };
}
unsafe extern "C" fn cmd_probe() -> ::core::ffi::c_int {
    printf(
        b"{\"arch\":\"%s\",\"pageSize\":%ld}\n\0".as_ptr() as *const ::core::ffi::c_char,
        ENG_ARCH_NAME.as_ptr(),
        sysconf(C2Rust_Unnamed::_SC_PAGESIZE.0 as ::core::ffi::c_int),
    );
    return 0 as ::core::ffi::c_int;
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
            (args_ptrs.len() - 1) as ::core::ffi::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut ::core::ffi::c_char,
        ) as i32)
    }
}
