use crate::arch::*;
pub type uint8_t = u8;
pub type uint16_t = u16;
pub type uint32_t = u32;
pub type uint64_t = u64;
pub type uintptr_t = usize;
pub type Elf64_Half = uint16_t;
pub type Elf64_Word = uint32_t;
pub type Elf64_Xword = uint64_t;
pub type Elf64_Addr = uint64_t;
pub type Elf64_Off = uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Elf64_Ehdr {
    pub e_ident: [::core::ffi::c_uchar; 16],
    pub e_type: Elf64_Half,
    pub e_machine: Elf64_Half,
    pub e_version: Elf64_Word,
    pub e_entry: Elf64_Addr,
    pub e_phoff: Elf64_Off,
    pub e_shoff: Elf64_Off,
    pub e_flags: Elf64_Word,
    pub e_ehsize: Elf64_Half,
    pub e_phentsize: Elf64_Half,
    pub e_phnum: Elf64_Half,
    pub e_shentsize: Elf64_Half,
    pub e_shnum: Elf64_Half,
    pub e_shstrndx: Elf64_Half,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Elf64_Phdr {
    pub p_type: Elf64_Word,
    pub p_flags: Elf64_Word,
    pub p_offset: Elf64_Off,
    pub p_vaddr: Elf64_Addr,
    pub p_paddr: Elf64_Addr,
    pub p_filesz: Elf64_Xword,
    pub p_memsz: Elf64_Xword,
    pub p_align: Elf64_Xword,
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_load_plan {
    pub magic: uint32_t,
    pub version: uint32_t,
    pub size: uint32_t,
    pub flags: uint32_t,
    pub exe_off: uint32_t,
    pub interp_off: uint32_t,
    pub execfn_off: uint32_t,
    pub argv_skip: uint32_t,
    pub n_prepend: uint32_t,
    pub prepend_off: uint32_t,
    pub uid: uint32_t,
    pub euid: uint32_t,
    pub gid: uint32_t,
    pub egid: uint32_t,
    pub pagesz: uint32_t,
    pub comm_off: uint32_t,
    pub reserved: [uint32_t; 2],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct img {
    pub bias: uintptr_t,
    pub entry: uintptr_t,
    pub phdr: uintptr_t,
    pub phnum: uint16_t,
    pub phent: uint16_t,
    pub is_dyn: ::core::ffi::c_int,
}
pub const EI_CLASS: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ELFCLASS64: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ET_EXEC: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const ET_DYN: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const EM_X86_64: ::core::ffi::c_int = 62 as ::core::ffi::c_int;
pub const PT_LOAD: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PT_PHDR: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const PF_X: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << 0 as ::core::ffi::c_int;
pub const PF_W: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << 1 as ::core::ffi::c_int;
pub const PF_R: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int;
pub const AT_NULL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const AT_PHDR: uint64_t = 3 as uint64_t;
pub const AT_PHENT: uint64_t = 4 as uint64_t;
pub const AT_PHNUM: uint64_t = 5 as uint64_t;
pub const AT_PAGESZ: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const AT_BASE: uint64_t = 7 as uint64_t;
pub const AT_ENTRY: uint64_t = 9 as uint64_t;
pub const AT_UID: uint64_t = 11 as uint64_t;
pub const AT_EUID: uint64_t = 12 as uint64_t;
pub const AT_GID: uint64_t = 13 as uint64_t;
pub const AT_EGID: uint64_t = 14 as uint64_t;
pub const AT_SECURE: uint64_t = 23 as uint64_t;
pub const AT_EXECFN: uint64_t = 31 as uint64_t;
pub const ENG_MARK_A: ::core::ffi::c_ulonglong = 0x574f524b464c4f57 as ::core::ffi::c_ulonglong;
pub const ENG_MARK_B: ::core::ffi::c_ulonglong = 0x4c4f414445523031 as ::core::ffi::c_ulonglong;
pub const ENG_MARK_OP_QUERY: ::core::ffi::c_uint = 1 as ::core::ffi::c_uint;
pub const ENG_MARK_OP_DONE: ::core::ffi::c_uint = 2 as ::core::ffi::c_uint;
pub const ENG_PLAN_MAGIC: ::core::ffi::c_uint = 0x4e414c50 as ::core::ffi::c_uint;
pub const ENG_PLAN_VERSION: ::core::ffi::c_uint = 2 as ::core::ffi::c_uint;
pub const ENG_SCRATCH_SIZE: ::core::ffi::c_uint = (4 as ::core::ffi::c_uint)
    .wrapping_mul(1024 as ::core::ffi::c_uint)
    .wrapping_mul(1024 as ::core::ffi::c_uint);
pub const ENG_PLAN_SECURE: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const ENG_PLAN_NO_FILEMAP: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const ENG_PLAN_PAGESZ: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;

unsafe extern "C" fn sys1(
    mut n: ::core::ffi::c_long,
    mut a: ::core::ffi::c_long,
) -> ::core::ffi::c_long {
    return sc6(
        n,
        a,
        0 as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
    );
}
unsafe extern "C" fn sys3(
    mut n: ::core::ffi::c_long,
    mut a: ::core::ffi::c_long,
    mut b: ::core::ffi::c_long,
    mut c: ::core::ffi::c_long,
) -> ::core::ffi::c_long {
    return sc6(
        n,
        a,
        b,
        c,
        0 as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
    );
}
unsafe extern "C" fn sys4(
    mut n: ::core::ffi::c_long,
    mut a: ::core::ffi::c_long,
    mut b: ::core::ffi::c_long,
    mut c: ::core::ffi::c_long,
    mut d: ::core::ffi::c_long,
) -> ::core::ffi::c_long {
    return sc6(
        n,
        a,
        b,
        c,
        d,
        0 as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
    );
}
pub const L_PROT_READ: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const L_PROT_WRITE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const L_PROT_EXEC: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const L_MAP_PRIVATE: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const L_MAP_FIXED: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const L_MAP_ANONYMOUS: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
pub const L_MAP_NORESERVE: ::core::ffi::c_int = 0x4000 as ::core::ffi::c_int;
pub const L_MAP_FIXED_NOREPLACE: ::core::ffi::c_int = 0x100000 as ::core::ffi::c_int;
pub const L_O_RDONLY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const L_O_CLOEXEC: ::core::ffi::c_int = 0o2000000 as ::core::ffi::c_int;
pub const L_AT_FDCWD: ::core::ffi::c_int = -100 as ::core::ffi::c_int;
pub const L_EACCES: ::core::ffi::c_int = 13 as ::core::ffi::c_int;
pub const L_EPERM: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const L_EINVAL: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
unsafe extern "C" fn is_err(mut r: ::core::ffi::c_long) -> ::core::ffi::c_int {
    return (r as ::core::ffi::c_ulong > -4096 as ::core::ffi::c_int as ::core::ffi::c_ulong)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn l_strlen(mut s: *const ::core::ffi::c_char) -> size_t {
    let mut n: size_t = 0 as size_t;
    while *s.offset(n as isize) != 0 {
        n = n.wrapping_add(1);
    }
    return n;
}
unsafe extern "C" fn l_memcpy(
    mut d: *mut ::core::ffi::c_void,
    mut s: *const ::core::ffi::c_void,
    mut n: size_t,
) {
    let mut dd: *mut ::core::ffi::c_uchar = d as *mut ::core::ffi::c_uchar;
    let mut ss: *const ::core::ffi::c_uchar = s as *const ::core::ffi::c_uchar;
    loop {
        let c2rust_fresh0 = n;
        n = n.wrapping_sub(1);
        if c2rust_fresh0 == 0 {
            break;
        }
        let c2rust_fresh1 = ss;
        ss = ss.offset(1);
        let c2rust_fresh2 = dd;
        dd = dd.offset(1);
        c2rust_fresh2.write_volatile(c2rust_fresh1.read_volatile());
    }
}
unsafe extern "C" fn l_memset(
    mut d: *mut ::core::ffi::c_void,
    mut c: ::core::ffi::c_int,
    mut n: size_t,
) {
    let mut dd: *mut ::core::ffi::c_uchar = d as *mut ::core::ffi::c_uchar;
    loop {
        let c2rust_fresh3 = n;
        n = n.wrapping_sub(1);
        if c2rust_fresh3 == 0 {
            break;
        }
        let c2rust_fresh4 = dd;
        dd = dd.offset(1);
        c2rust_fresh4.write_volatile(c as ::core::ffi::c_uchar);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(
    mut d: *mut ::core::ffi::c_void,
    mut s: *const ::core::ffi::c_void,
    mut n: size_t,
) -> *mut ::core::ffi::c_void {
    l_memcpy(d, s, n);
    return d;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(
    mut d: *mut ::core::ffi::c_void,
    mut c: ::core::ffi::c_int,
    mut n: size_t,
) -> *mut ::core::ffi::c_void {
    l_memset(d, c, n);
    return d;
}
unsafe extern "C" fn wr(mut s: *const ::core::ffi::c_char) {
    sys3(
        __NR_write as ::core::ffi::c_long,
        2 as ::core::ffi::c_long,
        s.expose_provenance() as ::core::ffi::c_long,
        l_strlen(s) as ::core::ffi::c_long,
    );
}
unsafe extern "C" fn die(
    mut what: *const ::core::ffi::c_char,
    mut arg: *const ::core::ffi::c_char,
    mut err: ::core::ffi::c_long,
) -> ! {
    let mut num: [::core::ffi::c_char; 24] = [0; 24];
    let mut i: ::core::ffi::c_int = 23 as ::core::ffi::c_int;
    num[i as usize] = 0 as ::core::ffi::c_char;
    let mut v: ::core::ffi::c_ulong = (if err < 0 as ::core::ffi::c_long {
        -err
    } else {
        err
    }) as ::core::ffi::c_ulong;
    loop {
        i -= 1;
        num[i as usize] = ('0' as ::core::ffi::c_ulong)
            .wrapping_add(v.wrapping_rem(10 as ::core::ffi::c_ulong))
            as ::core::ffi::c_char;
        v = v.wrapping_div(10 as ::core::ffi::c_ulong);
        if !(v != 0 && i > 0 as ::core::ffi::c_int) {
            break;
        }
    }
    wr(b"workflow-loader: \0".as_ptr() as *const ::core::ffi::c_char);
    wr(what);
    if !arg.is_null() {
        wr(b" \0".as_ptr() as *const ::core::ffi::c_char);
        wr(arg);
    }
    wr(b": errno \0".as_ptr() as *const ::core::ffi::c_char);
    wr((&raw mut num as *mut ::core::ffi::c_char).offset(i as isize));
    wr(b"\n\0".as_ptr() as *const ::core::ffi::c_char);
    sys1(
        __NR_exit_group as ::core::ffi::c_long,
        127 as ::core::ffi::c_long,
    );
    loop {}
}
#[repr(C, align(16))]
struct PlanBuffer([uint8_t; 65536]);
static mut plan_buf: PlanBuffer = PlanBuffer([0; 65536]);
static mut pgsz: uintptr_t = 0;
unsafe extern "C" fn pdown(mut x: uintptr_t) -> uintptr_t {
    return x & !pgsz.wrapping_sub(1 as ::core::ffi::c_int as uintptr_t);
}
unsafe extern "C" fn pup(mut x: uintptr_t) -> uintptr_t {
    return x
        .wrapping_add(pgsz)
        .wrapping_sub(1 as ::core::ffi::c_int as uintptr_t)
        & !pgsz.wrapping_sub(1 as ::core::ffi::c_int as uintptr_t);
}
unsafe extern "C" fn pread_full(
    mut fd: ::core::ffi::c_int,
    mut buf: *mut ::core::ffi::c_void,
    mut n: size_t,
    mut off: ::core::ffi::c_long,
) -> ::core::ffi::c_long {
    let mut done: size_t = 0 as size_t;
    while done < n {
        let mut r: ::core::ffi::c_long = sc6(
            __NR_pread64 as ::core::ffi::c_long,
            fd as ::core::ffi::c_long,
            (buf as *mut ::core::ffi::c_char)
                .offset(done as isize)
                .expose_provenance() as ::core::ffi::c_long,
            n.wrapping_sub(done) as ::core::ffi::c_long,
            off + done as ::core::ffi::c_long,
            0 as ::core::ffi::c_long,
            0 as ::core::ffi::c_long,
        );
        if is_err(r) != 0 {
            if r == -4 as ::core::ffi::c_long {
                continue;
            }
            return r;
        } else {
            if r == 0 as ::core::ffi::c_long {
                break;
            }
            done = done.wrapping_add(r as size_t);
        }
    }
    return done as ::core::ffi::c_long;
}
unsafe extern "C" fn prot_of(mut f: uint32_t) -> ::core::ffi::c_int {
    return if f & PF_R as uint32_t != 0 {
        L_PROT_READ
    } else {
        0 as ::core::ffi::c_int
    } | if f & PF_W as uint32_t != 0 {
        L_PROT_WRITE
    } else {
        0 as ::core::ffi::c_int
    } | if f & PF_X as uint32_t != 0 {
        L_PROT_EXEC
    } else {
        0 as ::core::ffi::c_int
    };
}
unsafe extern "C" fn map_seg_file(
    mut fd: ::core::ffi::c_int,
    mut p: *const Elf64_Phdr,
    mut bias: uintptr_t,
) -> ::core::ffi::c_long {
    let mut prot: ::core::ffi::c_int = prot_of((*p).p_flags);
    let mut va: uintptr_t = bias.wrapping_add((*p).p_vaddr as uintptr_t);
    let mut start: uintptr_t = pdown(va);
    let mut fend: uintptr_t = pup(va.wrapping_add((*p).p_filesz as uintptr_t));
    let mut mend: uintptr_t = pup(va.wrapping_add((*p).p_memsz as uintptr_t));
    if (*p).p_filesz != 0 {
        let mut r: ::core::ffi::c_long = sc6(
            __NR_mmap as ::core::ffi::c_long,
            start as ::core::ffi::c_long,
            fend.wrapping_sub(start) as ::core::ffi::c_long,
            prot as ::core::ffi::c_long,
            (L_MAP_PRIVATE | L_MAP_FIXED) as ::core::ffi::c_long,
            fd as ::core::ffi::c_long,
            pdown((*p).p_offset as uintptr_t) as ::core::ffi::c_long,
        );
        if is_err(r) != 0 {
            return r;
        }
    }
    if (*p).p_memsz > (*p).p_filesz {
        let mut zstart: uintptr_t = va.wrapping_add((*p).p_filesz as uintptr_t);
        let mut zend: uintptr_t = if (*p).p_filesz != 0 { fend } else { start };
        if zend > zstart && (*p).p_filesz != 0 {
            if prot & L_PROT_WRITE == 0 {
                sys3(
                    __NR_mprotect as ::core::ffi::c_long,
                    pdown(zstart) as ::core::ffi::c_long,
                    pgsz as ::core::ffi::c_long,
                    (prot | L_PROT_WRITE) as ::core::ffi::c_long,
                );
            }
            l_memset(
                ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(zstart as usize),
                0 as ::core::ffi::c_int,
                (zend as size_t).wrapping_sub(zstart as size_t),
            );
            if prot & L_PROT_WRITE == 0 {
                sys3(
                    __NR_mprotect as ::core::ffi::c_long,
                    pdown(zstart) as ::core::ffi::c_long,
                    pgsz as ::core::ffi::c_long,
                    prot as ::core::ffi::c_long,
                );
            }
        }
        if mend > zend {
            let mut r_0: ::core::ffi::c_long = sc6(
                __NR_mmap as ::core::ffi::c_long,
                zend as ::core::ffi::c_long,
                mend.wrapping_sub(zend) as ::core::ffi::c_long,
                prot as ::core::ffi::c_long,
                (L_MAP_PRIVATE | L_MAP_FIXED | L_MAP_ANONYMOUS) as ::core::ffi::c_long,
                -1 as ::core::ffi::c_long,
                0 as ::core::ffi::c_long,
            );
            if is_err(r_0) != 0 {
                return r_0;
            }
        }
    }
    return 0 as ::core::ffi::c_long;
}
static mut cuts: [uintptr_t; 514] = [0; 514];
unsafe extern "C" fn map_all_anon(
    mut fd: ::core::ffi::c_int,
    mut ph: *const Elf64_Phdr,
    mut n: ::core::ffi::c_int,
    mut bias: uintptr_t,
    mut lo: uintptr_t,
    mut hi: uintptr_t,
) -> ::core::ffi::c_long {
    let mut start: uintptr_t = pdown(bias.wrapping_add(lo));
    let mut end: uintptr_t = pup(bias.wrapping_add(hi));
    let mut r: ::core::ffi::c_long = sc6(
        __NR_mmap as ::core::ffi::c_long,
        start as ::core::ffi::c_long,
        end.wrapping_sub(start) as ::core::ffi::c_long,
        (L_PROT_READ | L_PROT_WRITE) as ::core::ffi::c_long,
        (L_MAP_PRIVATE | L_MAP_FIXED | L_MAP_ANONYMOUS) as ::core::ffi::c_long,
        -1 as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
    );
    if is_err(r) != 0 {
        return r;
    }
    let mut nc: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let c2rust_fresh5 = nc;
    nc += 1;
    cuts[c2rust_fresh5 as usize] = start;
    let c2rust_fresh6 = nc;
    nc += 1;
    cuts[c2rust_fresh6 as usize] = end;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < n {
        if (*ph.offset(i as isize)).p_type == PT_LOAD as Elf64_Word {
            let mut va: uintptr_t =
                bias.wrapping_add((*ph.offset(i as isize)).p_vaddr as uintptr_t);
            if (*ph.offset(i as isize)).p_filesz != 0 {
                r = pread_full(
                    fd,
                    ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_void>(va as usize),
                    (*ph.offset(i as isize)).p_filesz as size_t,
                    (*ph.offset(i as isize)).p_offset as ::core::ffi::c_long,
                );
                if is_err(r) != 0 {
                    return r;
                }
            }
            let c2rust_fresh7 = nc;
            nc += 1;
            cuts[c2rust_fresh7 as usize] = pdown(va);
            let c2rust_fresh8 = nc;
            nc += 1;
            cuts[c2rust_fresh8 as usize] =
                pup(va.wrapping_add((*ph.offset(i as isize)).p_memsz as uintptr_t));
        }
        i += 1;
    }
    let mut i_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while i_0 < nc {
        let mut j: ::core::ffi::c_int = i_0;
        while j > 0 as ::core::ffi::c_int
            && cuts[(j - 1 as ::core::ffi::c_int) as usize] > cuts[j as usize]
        {
            let mut x: uintptr_t = cuts[j as usize];
            cuts[j as usize] = cuts[(j - 1 as ::core::ffi::c_int) as usize];
            cuts[(j - 1 as ::core::ffi::c_int) as usize] = x;
            j -= 1;
        }
        i_0 += 1;
    }
    let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while (i_1 + 1 as ::core::ffi::c_int) < nc {
        let mut a: uintptr_t = cuts[i_1 as usize];
        let mut b: uintptr_t = cuts[(i_1 + 1 as ::core::ffi::c_int) as usize];
        if a < b {
            let mut prot: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while k < n {
                if (*ph.offset(k as isize)).p_type == PT_LOAD as Elf64_Word {
                    let mut s: uintptr_t =
                        pdown(bias.wrapping_add((*ph.offset(k as isize)).p_vaddr as uintptr_t));
                    let mut e: uintptr_t = pup(bias
                        .wrapping_add((*ph.offset(k as isize)).p_vaddr as uintptr_t)
                        .wrapping_add((*ph.offset(k as isize)).p_memsz as uintptr_t));
                    if s < b && e > a {
                        prot |= prot_of((*ph.offset(k as isize)).p_flags);
                    }
                }
                k += 1;
            }
            r = sys3(
                __NR_mprotect as ::core::ffi::c_long,
                a as ::core::ffi::c_long,
                b.wrapping_sub(a) as ::core::ffi::c_long,
                prot as ::core::ffi::c_long,
            );
            if is_err(r) != 0 {
                return r;
            }
        }
        i_1 += 1;
    }
    return 0 as ::core::ffi::c_long;
}
static mut phbuf: [[Elf64_Phdr; 256]; 2] = [[Elf64_Phdr {
    p_type: 0,
    p_flags: 0,
    p_offset: 0,
    p_vaddr: 0,
    p_paddr: 0,
    p_filesz: 0,
    p_memsz: 0,
    p_align: 0,
}; 256]; 2];
unsafe extern "C" fn map_image(
    mut path: *const ::core::ffi::c_char,
    mut out: *mut img,
    mut which: ::core::ffi::c_int,
    mut force_anon: ::core::ffi::c_int,
) {
    let mut fd: ::core::ffi::c_long = sys4(
        __NR_openat as ::core::ffi::c_long,
        L_AT_FDCWD as ::core::ffi::c_long,
        path.expose_provenance() as ::core::ffi::c_long,
        (L_O_RDONLY | L_O_CLOEXEC) as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
    );
    if is_err(fd) != 0 {
        die(b"open\0".as_ptr() as *const ::core::ffi::c_char, path, fd);
    }
    let mut eh: Elf64_Ehdr = Elf64_Ehdr {
        e_ident: [0; 16],
        e_type: 0,
        e_machine: 0,
        e_version: 0,
        e_entry: 0,
        e_phoff: 0,
        e_shoff: 0,
        e_flags: 0,
        e_ehsize: 0,
        e_phentsize: 0,
        e_phnum: 0,
        e_shentsize: 0,
        e_shnum: 0,
        e_shstrndx: 0,
    };
    let mut r: ::core::ffi::c_long = pread_full(
        fd as ::core::ffi::c_int,
        &raw mut eh as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<Elf64_Ehdr>(),
        0 as ::core::ffi::c_long,
    );
    if r != ::core::mem::size_of::<Elf64_Ehdr>() as ::core::ffi::c_long {
        die(
            b"read ELF header\0".as_ptr() as *const ::core::ffi::c_char,
            path,
            if is_err(r) != 0 {
                r
            } else {
                -(L_EINVAL as ::core::ffi::c_long)
            },
        );
    }
    if eh.e_ident[0usize] as ::core::ffi::c_int != 0x7f as ::core::ffi::c_int
        || eh.e_ident[1usize] as ::core::ffi::c_int != 'E' as ::core::ffi::c_int
        || eh.e_ident[2usize] as ::core::ffi::c_int != 'L' as ::core::ffi::c_int
        || eh.e_ident[3usize] as ::core::ffi::c_int != 'F' as ::core::ffi::c_int
        || eh.e_ident[EI_CLASS as usize] as ::core::ffi::c_int != ELFCLASS64
        || eh.e_machine as ::core::ffi::c_int != EM_SELF
        || eh.e_type as ::core::ffi::c_int != ET_DYN && eh.e_type as ::core::ffi::c_int != ET_EXEC
        || eh.e_phentsize as usize != ::core::mem::size_of::<Elf64_Phdr>()
        || eh.e_phnum as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        || eh.e_phnum as ::core::ffi::c_int > 256 as ::core::ffi::c_int
    {
        die(
            b"not a loadable ELF for this ABI\0".as_ptr() as *const ::core::ffi::c_char,
            path,
            -8 as ::core::ffi::c_long,
        );
    }
    let mut ph: *mut Elf64_Phdr = &raw mut *(&raw mut phbuf as *mut [Elf64_Phdr; 256])
        .offset(which as isize) as *mut Elf64_Phdr;
    r = pread_full(
        fd as ::core::ffi::c_int,
        ph as *mut ::core::ffi::c_void,
        (eh.e_phnum as size_t).wrapping_mul(::core::mem::size_of::<Elf64_Phdr>()),
        eh.e_phoff as ::core::ffi::c_long,
    );
    if r != (eh.e_phnum as usize).wrapping_mul(::core::mem::size_of::<Elf64_Phdr>())
        as ::core::ffi::c_long
    {
        die(
            b"read program headers\0".as_ptr() as *const ::core::ffi::c_char,
            path,
            -(L_EINVAL as ::core::ffi::c_long),
        );
    }
    let mut lo: uintptr_t = !(0 as ::core::ffi::c_int as uintptr_t);
    let mut hi: uintptr_t = 0 as uintptr_t;
    let mut align: uintptr_t = pgsz;
    let mut phdr_va: uintptr_t = 0 as uintptr_t;
    let mut have_phdr: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut congruent: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < eh.e_phnum as ::core::ffi::c_int {
        if (*ph.offset(i as isize)).p_type == PT_PHDR as Elf64_Word {
            phdr_va = (*ph.offset(i as isize)).p_vaddr as uintptr_t;
            have_phdr = 1 as ::core::ffi::c_int;
        }
        if (*ph.offset(i as isize)).p_type == PT_LOAD as Elf64_Word {
            if (*ph.offset(i as isize)).p_vaddr < lo as Elf64_Addr {
                lo = (*ph.offset(i as isize)).p_vaddr as uintptr_t;
            }
            if (*ph.offset(i as isize))
                .p_vaddr
                .wrapping_add((*ph.offset(i as isize)).p_memsz)
                > hi as Elf64_Addr
            {
                hi = (*ph.offset(i as isize))
                    .p_vaddr
                    .wrapping_add((*ph.offset(i as isize)).p_memsz)
                    as uintptr_t;
            }
            if (*ph.offset(i as isize)).p_align > align as Elf64_Xword
                && (*ph.offset(i as isize)).p_align
                    & (*ph.offset(i as isize))
                        .p_align
                        .wrapping_sub(1 as Elf64_Xword)
                    == 0 as Elf64_Xword
                && (*ph.offset(i as isize)).p_align <= 0x10000 as Elf64_Xword
            {
                align = (*ph.offset(i as isize)).p_align as uintptr_t;
            }
            if (*ph.offset(i as isize))
                .p_vaddr
                .wrapping_sub((*ph.offset(i as isize)).p_offset)
                & (pgsz as Elf64_Addr).wrapping_sub(1 as Elf64_Addr)
                != 0 as Elf64_Addr
            {
                congruent = 0 as ::core::ffi::c_int;
            }
        }
        i += 1;
    }
    if hi <= lo {
        die(
            b"no PT_LOAD\0".as_ptr() as *const ::core::ffi::c_char,
            path,
            -8 as ::core::ffi::c_long,
        );
    }
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < eh.e_phnum as ::core::ffi::c_int && congruent != 0 {
        if (*ph.offset(i_0 as isize)).p_type == PT_LOAD as Elf64_Word {
            let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while j < eh.e_phnum as ::core::ffi::c_int {
                if !(j == i_0 || (*ph.offset(j as isize)).p_type != PT_LOAD as Elf64_Word) {
                    let mut si: uintptr_t = pdown((*ph.offset(i_0 as isize)).p_vaddr as uintptr_t);
                    let mut ei: uintptr_t = pup(((*ph.offset(i_0 as isize)).p_vaddr as uintptr_t)
                        .wrapping_add((*ph.offset(i_0 as isize)).p_memsz as uintptr_t));
                    let mut sj: uintptr_t = pdown((*ph.offset(j as isize)).p_vaddr as uintptr_t);
                    let mut ej: uintptr_t = pup(((*ph.offset(j as isize)).p_vaddr as uintptr_t)
                        .wrapping_add((*ph.offset(j as isize)).p_memsz as uintptr_t));
                    if si < ej && sj < ei {
                        congruent = 0 as ::core::ffi::c_int;
                        break;
                    }
                }
                j += 1;
            }
        }
        i_0 += 1;
    }
    let mut anon: ::core::ffi::c_int = (force_anon != 0 || congruent == 0) as ::core::ffi::c_int;
    let mut attempt: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while attempt < 2 as ::core::ffi::c_int {
        let mut bias: uintptr_t = 0 as uintptr_t;
        let mut span: uintptr_t = pup(hi).wrapping_sub(pdown(lo));
        let mut base: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
        if eh.e_type as ::core::ffi::c_int == ET_DYN {
            base = sc6(
                __NR_mmap as ::core::ffi::c_long,
                0 as ::core::ffi::c_long,
                span.wrapping_add(align) as ::core::ffi::c_long,
                0 as ::core::ffi::c_long,
                (L_MAP_PRIVATE | L_MAP_ANONYMOUS) as ::core::ffi::c_long,
                -1 as ::core::ffi::c_long,
                0 as ::core::ffi::c_long,
            );
            if is_err(base) != 0 {
                die(
                    b"reserve address space\0".as_ptr() as *const ::core::ffi::c_char,
                    path,
                    base,
                );
            }
            let mut aligned: uintptr_t = (base as uintptr_t)
                .wrapping_add(align)
                .wrapping_sub(1 as ::core::ffi::c_int as uintptr_t)
                & !align.wrapping_sub(1 as ::core::ffi::c_int as uintptr_t);
            if aligned > base as uintptr_t {
                sys3(
                    __NR_munmap as ::core::ffi::c_long,
                    base,
                    aligned.wrapping_sub(base as uintptr_t) as ::core::ffi::c_long,
                    0 as ::core::ffi::c_long,
                );
            }
            let mut tail: uintptr_t = (base as uintptr_t)
                .wrapping_add(span)
                .wrapping_add(align)
                .wrapping_sub(aligned.wrapping_add(span));
            if tail != 0 {
                sys3(
                    __NR_munmap as ::core::ffi::c_long,
                    aligned.wrapping_add(span) as ::core::ffi::c_long,
                    tail as ::core::ffi::c_long,
                    0 as ::core::ffi::c_long,
                );
            }
            bias = aligned.wrapping_sub(pdown(lo));
        } else {
            let mut got: ::core::ffi::c_long = sc6(
                __NR_mmap as ::core::ffi::c_long,
                pdown(lo) as ::core::ffi::c_long,
                span as ::core::ffi::c_long,
                0 as ::core::ffi::c_long,
                (L_MAP_PRIVATE | L_MAP_ANONYMOUS | L_MAP_FIXED_NOREPLACE) as ::core::ffi::c_long,
                -1 as ::core::ffi::c_long,
                0 as ::core::ffi::c_long,
            );
            if is_err(got) != 0 || got as uintptr_t != pdown(lo) {
                if is_err(got) == 0 {
                    sys3(
                        __NR_munmap as ::core::ffi::c_long,
                        got,
                        span as ::core::ffi::c_long,
                        0 as ::core::ffi::c_long,
                    );
                }
                die(
                    b"fixed-address ELF overlaps an existing mapping\0".as_ptr()
                        as *const ::core::ffi::c_char,
                    path,
                    -12 as ::core::ffi::c_long,
                );
            }
        }
        let mut err: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
        if anon != 0 {
            err = map_all_anon(
                fd as ::core::ffi::c_int,
                ph,
                eh.e_phnum as ::core::ffi::c_int,
                bias,
                lo,
                hi,
            );
        } else {
            let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i_1 < eh.e_phnum as ::core::ffi::c_int && err == 0 {
                if (*ph.offset(i_1 as isize)).p_type == PT_LOAD as Elf64_Word {
                    err = map_seg_file(fd as ::core::ffi::c_int, ph.offset(i_1 as isize), bias);
                }
                i_1 += 1;
            }
        }
        if err == 0 {
            (*out).bias = bias;
            (*out).entry = bias.wrapping_add(eh.e_entry as uintptr_t);
            (*out).phnum = eh.e_phnum as uint16_t;
            (*out).phent = eh.e_phentsize as uint16_t;
            (*out).is_dyn = (eh.e_type as ::core::ffi::c_int == ET_DYN) as ::core::ffi::c_int;
            (*out).phdr = 0 as uintptr_t;
            if have_phdr != 0 {
                (*out).phdr = bias.wrapping_add(phdr_va);
            } else {
                let mut i_2: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while i_2 < eh.e_phnum as ::core::ffi::c_int {
                    if (*ph.offset(i_2 as isize)).p_type == PT_LOAD as Elf64_Word
                        && eh.e_phoff >= (*ph.offset(i_2 as isize)).p_offset
                        && eh.e_phoff
                            < (*ph.offset(i_2 as isize))
                                .p_offset
                                .wrapping_add((*ph.offset(i_2 as isize)).p_filesz)
                    {
                        (*out).phdr =
                            bias.wrapping_add((*ph.offset(i_2 as isize)).p_vaddr as uintptr_t)
                                .wrapping_add((eh.e_phoff as uintptr_t).wrapping_sub(
                                    (*ph.offset(i_2 as isize)).p_offset as uintptr_t,
                                ));
                        break;
                    } else {
                        i_2 += 1;
                    }
                }
            }
            sys1(__NR_close as ::core::ffi::c_long, fd);
            return;
        }
        sys3(
            __NR_munmap as ::core::ffi::c_long,
            bias.wrapping_add(pdown(lo)) as ::core::ffi::c_long,
            span as ::core::ffi::c_long,
            0 as ::core::ffi::c_long,
        );
        if anon != 0
            || err != -(L_EACCES as ::core::ffi::c_long) && err != -(L_EPERM as ::core::ffi::c_long)
        {
            die(
                b"map segment\0".as_ptr() as *const ::core::ffi::c_char,
                path,
                err,
            );
        }
        anon = 1 as ::core::ffi::c_int;
        attempt += 1;
    }
    die(
        b"map\0".as_ptr() as *const ::core::ffi::c_char,
        path,
        -(L_EINVAL as ::core::ffi::c_long),
    );
}
unsafe extern "C" fn streq(
    mut a: *const ::core::ffi::c_char,
    mut b: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    while *a as ::core::ffi::c_int != 0 && *a as ::core::ffi::c_int == *b as ::core::ffi::c_int {
        a = a.offset(1);
        b = b.offset(1);
    }
    return (*a as ::core::ffi::c_int == *b as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn loader_main(mut sp0: *mut uintptr_t) -> ! {
    let mut argc: ::core::ffi::c_long = *sp0.offset(0isize) as ::core::ffi::c_long;
    let mut argv: *mut *mut ::core::ffi::c_char =
        sp0.offset(1 as ::core::ffi::c_int as isize) as *mut *mut ::core::ffi::c_char;
    let mut envp: *mut *mut ::core::ffi::c_char = argv
        .offset(argc as isize)
        .offset(1 as ::core::ffi::c_int as isize);
    let mut envc: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
    while !(*envp.offset(envc as isize)).is_null() {
        envc += 1;
    }
    let mut auxv: *mut uint64_t =
        envp.offset(envc as isize)
            .offset(1 as ::core::ffi::c_int as isize) as *mut uint64_t;
    let mut nauxv: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
    pgsz = 4096 as ::core::ffi::c_int as uintptr_t;
    let mut a: *mut uint64_t = auxv;
    while *a.offset(0isize) != AT_NULL as uint64_t {
        if *a.offset(0isize) == AT_PAGESZ as uint64_t {
            pgsz = *a.offset(1isize) as uintptr_t;
        }
        a = a.offset(2 as ::core::ffi::c_int as isize);
        nauxv += 1;
    }
    let mut r: ::core::ffi::c_long = sc6(
        __NR_getpid as ::core::ffi::c_long,
        ENG_MARK_A as ::core::ffi::c_long,
        ENG_MARK_B as ::core::ffi::c_long,
        ENG_MARK_OP_QUERY as ::core::ffi::c_long,
        (&raw mut plan_buf as *mut uint8_t).expose_provenance() as ::core::ffi::c_long,
        ::core::mem::size_of::<[uint8_t; 65536]>() as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
    );
    let mut pl: *mut eng_load_plan = &raw mut plan_buf as *mut uint8_t as *mut eng_load_plan;
    if (*pl).magic != ENG_PLAN_MAGIC as uint32_t
        || (*pl).version != ENG_PLAN_VERSION as uint32_t
        || (*pl).size as usize > ::core::mem::size_of::<[uint8_t; 65536]>()
        || (*pl).exe_off == 0
        || (*pl).exe_off >= (*pl).size
    {
        die(
            b"no load plan from the engine (not started by workflow-engine?)\0".as_ptr()
                as *const ::core::ffi::c_char,
            ::core::ptr::null::<::core::ffi::c_char>(),
            if is_err(r) != 0 {
                r
            } else {
                -(L_EINVAL as ::core::ffi::c_long)
            },
        );
    }
    let mut base: *const ::core::ffi::c_char =
        &raw mut plan_buf as *mut uint8_t as *const ::core::ffi::c_char;
    let mut exe: *const ::core::ffi::c_char = base.offset((*pl).exe_off as isize);
    let mut interp: *const ::core::ffi::c_char = if (*pl).interp_off != 0 {
        base.offset((*pl).interp_off as isize)
    } else {
        ::core::ptr::null::<::core::ffi::c_char>()
    };
    let mut execfn: *const ::core::ffi::c_char = if (*pl).execfn_off != 0 {
        base.offset((*pl).execfn_off as isize)
    } else {
        exe
    };
    let mut force_anon: ::core::ffi::c_int =
        ((*pl).flags & ENG_PLAN_NO_FILEMAP as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
    if (*pl).flags & ENG_PLAN_PAGESZ as uint32_t != 0
        && (*pl).pagesz as uintptr_t > pgsz
        && (*pl).pagesz & (*pl).pagesz.wrapping_sub(1 as uint32_t) == 0 as uint32_t
    {
        pgsz = (*pl).pagesz as uintptr_t;
        force_anon = 1 as ::core::ffi::c_int;
    }
    let mut ex: img = img {
        bias: 0,
        entry: 0,
        phdr: 0,
        phnum: 0,
        phent: 0,
        is_dyn: 0,
    };
    let mut r#in: img = img {
        bias: 0,
        entry: 0,
        phdr: 0,
        phnum: 0,
        phent: 0,
        is_dyn: 0,
    };
    map_image(exe, &raw mut ex, 0 as ::core::ffi::c_int, force_anon);
    let mut jump: uintptr_t = ex.entry;
    let mut at_base: uintptr_t = 0 as uintptr_t;
    if !interp.is_null() {
        map_image(interp, &raw mut r#in, 1 as ::core::ffi::c_int, force_anon);
        jump = r#in.entry;
        at_base = r#in.bias;
    }
    let mut scratch: ::core::ffi::c_long = sc6(
        __NR_mmap as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
        ENG_SCRATCH_SIZE as ::core::ffi::c_long,
        (L_PROT_READ | L_PROT_WRITE) as ::core::ffi::c_long,
        (L_MAP_PRIVATE | L_MAP_ANONYMOUS | L_MAP_NORESERVE) as ::core::ffi::c_long,
        -1 as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
    );
    if is_err(scratch) != 0 {
        die(
            b"mmap scratch\0".as_ptr() as *const ::core::ffi::c_char,
            ::core::ptr::null::<::core::ffi::c_char>(),
            scratch,
        );
    }
    let mut skip: uint32_t = if (*pl).argv_skip > argc as uint32_t {
        argc as uint32_t
    } else {
        (*pl).argv_skip
    };
    let mut nargc: ::core::ffi::c_long =
        (*pl).n_prepend as ::core::ffi::c_long + argc - skip as ::core::ffi::c_long;
    let mut strbytes: size_t = l_strlen(execfn).wrapping_add(1 as size_t);
    let mut pp: *const ::core::ffi::c_char = if (*pl).prepend_off != 0 {
        base.offset((*pl).prepend_off as isize)
    } else {
        ::core::ptr::null::<::core::ffi::c_char>()
    };
    let mut q: *const ::core::ffi::c_char = pp;
    let mut i: uint32_t = 0 as uint32_t;
    while i < (*pl).n_prepend {
        let mut l: size_t = l_strlen(q).wrapping_add(1 as size_t);
        strbytes = strbytes.wrapping_add(l);
        q = q.offset(l as isize);
        i = i.wrapping_add(1);
    }
    let mut nwords: size_t = (1 as size_t)
        .wrapping_add(nargc as size_t)
        .wrapping_add(1 as size_t)
        .wrapping_add(envc as size_t)
        .wrapping_add(1 as size_t)
        .wrapping_add((2 as size_t).wrapping_mul((nauxv as size_t).wrapping_add(1 as size_t)));
    let mut len: size_t = nwords
        .wrapping_mul(8 as size_t)
        .wrapping_add(strbytes)
        .wrapping_add(15 as size_t)
        & !(15 as ::core::ffi::c_int as size_t);
    let mut final_sp: uintptr_t = (sp0.expose_provenance() as uintptr_t)
        .wrapping_sub(len as uintptr_t)
        & !(15 as ::core::ffi::c_int as uintptr_t);
    let mut blk: ::core::ffi::c_long = sc6(
        __NR_mmap as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
        len as ::core::ffi::c_long,
        (L_PROT_READ | L_PROT_WRITE) as ::core::ffi::c_long,
        (L_MAP_PRIVATE | L_MAP_ANONYMOUS) as ::core::ffi::c_long,
        -1 as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
    );
    if is_err(blk) != 0 {
        die(
            b"mmap stack image\0".as_ptr() as *const ::core::ffi::c_char,
            ::core::ptr::null::<::core::ffi::c_char>(),
            blk,
        );
    }
    let mut w: *mut uint64_t = ::core::ptr::with_exposed_provenance_mut::<uint64_t>(blk as usize);
    let mut sdst: *mut ::core::ffi::c_char =
        ::core::ptr::with_exposed_provenance_mut::<::core::ffi::c_char>(blk as usize)
            .offset(nwords.wrapping_mul(8 as size_t) as isize);
    let mut sfinal: uintptr_t = final_sp
        .wrapping_add((nwords as uintptr_t).wrapping_mul(8 as ::core::ffi::c_int as uintptr_t));
    let c2rust_fresh9 = w;
    w = w.offset(1);
    *c2rust_fresh9 = nargc as uint64_t;
    let mut q_0: *const ::core::ffi::c_char = pp;
    let mut i_0: uint32_t = 0 as uint32_t;
    while i_0 < (*pl).n_prepend {
        let mut l_0: size_t = l_strlen(q_0).wrapping_add(1 as size_t);
        l_memcpy(
            sdst as *mut ::core::ffi::c_void,
            q_0 as *const ::core::ffi::c_void,
            l_0,
        );
        let c2rust_fresh10 = w;
        w = w.offset(1);
        *c2rust_fresh10 = sfinal as uint64_t;
        sdst = sdst.offset(l_0 as isize);
        sfinal =
            (sfinal as ::core::ffi::c_ulong).wrapping_add(l_0 as ::core::ffi::c_ulong) as uintptr_t;
        q_0 = q_0.offset(l_0 as isize);
        i_0 = i_0.wrapping_add(1);
    }
    let mut i_1: ::core::ffi::c_long = skip as ::core::ffi::c_long;
    while i_1 < argc {
        let c2rust_fresh11 = w;
        w = w.offset(1);
        *c2rust_fresh11 = (*argv.offset(i_1 as isize)).expose_provenance() as uint64_t;
        i_1 += 1;
    }
    let c2rust_fresh12 = w;
    w = w.offset(1);
    *c2rust_fresh12 = 0 as uint64_t;
    let mut i_2: ::core::ffi::c_long = 0 as ::core::ffi::c_long;
    while i_2 < envc {
        let c2rust_fresh13 = w;
        w = w.offset(1);
        *c2rust_fresh13 = (*envp.offset(i_2 as isize)).expose_provenance() as uint64_t;
        i_2 += 1;
    }
    let c2rust_fresh14 = w;
    w = w.offset(1);
    *c2rust_fresh14 = 0 as uint64_t;
    let mut efl: size_t = l_strlen(execfn).wrapping_add(1 as size_t);
    l_memcpy(
        sdst as *mut ::core::ffi::c_void,
        execfn as *const ::core::ffi::c_void,
        efl,
    );
    let mut execfn_addr: uintptr_t = sfinal;
    let mut a_0: *mut uint64_t = auxv;
    while *a_0.offset(0isize) != AT_NULL as uint64_t {
        let mut v: uint64_t = *a_0.offset(1isize);
        match *a_0.offset(0isize) {
            AT_PHDR => {
                v = ex.phdr as uint64_t;
            }
            6 => {
                v = pgsz as uint64_t;
            }
            AT_PHENT => {
                v = ex.phent as uint64_t;
            }
            AT_PHNUM => {
                v = ex.phnum as uint64_t;
            }
            AT_ENTRY => {
                v = ex.entry as uint64_t;
            }
            AT_BASE => {
                v = at_base as uint64_t;
            }
            AT_EXECFN => {
                v = execfn_addr as uint64_t;
            }
            AT_SECURE => {
                v = (if (*pl).flags & ENG_PLAN_SECURE as uint32_t != 0 {
                    1 as ::core::ffi::c_int
                } else {
                    0 as ::core::ffi::c_int
                }) as uint64_t;
            }
            AT_UID => {
                v = (*pl).uid as uint64_t;
            }
            AT_EUID => {
                v = (*pl).euid as uint64_t;
            }
            AT_GID => {
                v = (*pl).gid as uint64_t;
            }
            AT_EGID => {
                v = (*pl).egid as uint64_t;
            }
            _ => {}
        }
        let c2rust_fresh15 = w;
        w = w.offset(1);
        *c2rust_fresh15 = *a_0.offset(0isize);
        let c2rust_fresh16 = w;
        w = w.offset(1);
        *c2rust_fresh16 = v;
        a_0 = a_0.offset(2 as ::core::ffi::c_int as isize);
    }
    let c2rust_fresh17 = w;
    w = w.offset(1);
    *c2rust_fresh17 = AT_NULL as uint64_t;
    let c2rust_fresh18 = w;
    w = w.offset(1);
    *c2rust_fresh18 = 0 as uint64_t;
    if (*pl).comm_off != 0 && (*pl).comm_off < (*pl).size {
        sc6(
            __NR_prctl as ::core::ffi::c_long,
            15 as ::core::ffi::c_long,
            base.offset((*pl).comm_off as isize).expose_provenance() as ::core::ffi::c_long,
            0 as ::core::ffi::c_long,
            0 as ::core::ffi::c_long,
            0 as ::core::ffi::c_long,
            0 as ::core::ffi::c_long,
        );
    }
    sc6(
        __NR_getpid as ::core::ffi::c_long,
        ENG_MARK_A as ::core::ffi::c_long,
        ENG_MARK_B as ::core::ffi::c_long,
        ENG_MARK_OP_DONE as ::core::ffi::c_long,
        scratch,
        ENG_SCRATCH_SIZE as ::core::ffi::c_long,
        0 as ::core::ffi::c_long,
    );
    wf_finish(
        final_sp,
        ::core::ptr::with_exposed_provenance::<::core::ffi::c_void>(blk as usize),
        len,
        jump,
    );
}
