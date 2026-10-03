//! Bounded strict JSON parser used by image metadata validation.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    unsafe fn strtod(_: *const ::core::ffi::c_char, _: *mut *mut ::core::ffi::c_char) -> f64;
    unsafe fn malloc(_: usize) -> *mut ::core::ffi::c_void;
    unsafe fn calloc(_: usize, _: usize) -> *mut ::core::ffi::c_void;
    unsafe fn free(_: *mut ::core::ffi::c_void);
    unsafe fn memcmp(_: *const ::core::ffi::c_void, _: *const ::core::ffi::c_void, _: usize)
    -> i32;
    unsafe fn strcmp(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> i32;
    unsafe fn strchr(_: *const ::core::ffi::c_char, _: i32) -> *mut ::core::ffi::c_char;
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct eng_jtype(pub u32);
impl eng_jtype {
    pub const ENG_J_NULL: Self = Self(0);
    pub const ENG_J_BOOL: Self = Self(1);
    pub const ENG_J_NUM: Self = Self(2);
    pub const ENG_J_STR: Self = Self(3);
    pub const ENG_J_ARR: Self = Self(4);
    pub const ENG_J_OBJ: Self = Self(5);
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_json {
    pub t: eng_jtype,
    pub key: *mut ::core::ffi::c_char,
    pub s: *mut ::core::ffi::c_char,
    pub n: f64,
    pub b: i32,
    pub child: *mut eng_json,
    pub next: *mut eng_json,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct rd {
    pub p: *const ::core::ffi::c_char,
    pub end: *const ::core::ffi::c_char,
    pub depth: i32,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
unsafe extern "C" fn ws(mut r: *mut rd) {
    while (*r).p < (*r).end
        && (*(*r).p as i32 == ' ' as i32
            || *(*r).p as i32 == '\t' as i32
            || *(*r).p as i32 == '\n' as i32
            || *(*r).p as i32 == '\r' as i32)
    {
        (*r).p = (*r).p.offset(1);
    }
}
unsafe extern "C" fn node(mut t: eng_jtype) -> *mut eng_json {
    let mut n: *mut eng_json =
        calloc(1 as usize, ::core::mem::size_of::<eng_json>()) as *mut eng_json;
    if !n.is_null() {
        (*n).t = t;
    }
    return n;
}
unsafe extern "C" fn put_utf8(mut o: *mut *mut ::core::ffi::c_char, mut cp: u32) -> i32 {
    let mut q: *mut ::core::ffi::c_char = *o;
    if cp < 0x80 as u32 {
        let c2rust_fresh11 = q;
        q = q.offset(1);
        *c2rust_fresh11 = cp as ::core::ffi::c_char;
    } else if cp < 0x800 as u32 {
        let c2rust_fresh12 = q;
        q = q.offset(1);
        *c2rust_fresh12 = (0xc0 as u32 | cp >> 6 as i32) as ::core::ffi::c_char;
        let c2rust_fresh13 = q;
        q = q.offset(1);
        *c2rust_fresh13 = (0x80 as u32 | cp & 0x3f as u32) as ::core::ffi::c_char;
    } else if cp < 0x10000 as i32 as u32 {
        let c2rust_fresh14 = q;
        q = q.offset(1);
        *c2rust_fresh14 = (0xe0 as u32 | cp >> 12 as i32) as ::core::ffi::c_char;
        let c2rust_fresh15 = q;
        q = q.offset(1);
        *c2rust_fresh15 = (0x80 as u32 | cp >> 6 as i32 & 0x3f as u32) as ::core::ffi::c_char;
        let c2rust_fresh16 = q;
        q = q.offset(1);
        *c2rust_fresh16 = (0x80 as u32 | cp & 0x3f as u32) as ::core::ffi::c_char;
    } else {
        let c2rust_fresh17 = q;
        q = q.offset(1);
        *c2rust_fresh17 = (0xf0 as u32 | cp >> 18 as i32) as ::core::ffi::c_char;
        let c2rust_fresh18 = q;
        q = q.offset(1);
        *c2rust_fresh18 = (0x80 as u32 | cp >> 12 as i32 & 0x3f as u32) as ::core::ffi::c_char;
        let c2rust_fresh19 = q;
        q = q.offset(1);
        *c2rust_fresh19 = (0x80 as u32 | cp >> 6 as i32 & 0x3f as u32) as ::core::ffi::c_char;
        let c2rust_fresh20 = q;
        q = q.offset(1);
        *c2rust_fresh20 = (0x80 as u32 | cp & 0x3f as u32) as ::core::ffi::c_char;
    }
    *o = q;
    return 0 as i32;
}
unsafe extern "C" fn hex4(mut p: *const ::core::ffi::c_char, mut v: *mut u32) -> i32 {
    *v = 0 as u32;
    let mut i: i32 = 0 as i32;
    while i < 4 as i32 {
        let mut c: ::core::ffi::c_char = *p.offset(i as isize);
        *v <<= 4 as i32;
        if c as i32 >= '0' as i32 && c as i32 <= '9' as i32 {
            *v |= (c as i32 - '0' as i32) as u32;
        } else if c as i32 >= 'a' as i32 && c as i32 <= 'f' as i32 {
            *v |= (c as i32 - 'a' as i32 + 10 as i32) as u32;
        } else if c as i32 >= 'A' as i32 && c as i32 <= 'F' as i32 {
            *v |= (c as i32 - 'A' as i32 + 10 as i32) as u32;
        } else {
            return -1 as i32;
        }
        i += 1;
    }
    return 0 as i32;
}
unsafe extern "C" fn str(mut r: *mut rd) -> *mut ::core::ffi::c_char {
    if (*r).p >= (*r).end || *(*r).p as i32 != '"' as i32 {
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    (*r).p = (*r).p.offset(1);
    let mut s: *const ::core::ffi::c_char = (*r).p;
    let mut cap: usize = 0 as usize;
    while (*r).p < (*r).end && *(*r).p as i32 != '"' as i32 {
        if *(*r).p as i32 == '\\' as i32 {
            (*r).p = (*r).p.offset(1);
        }
        (*r).p = (*r).p.offset(1);
        cap = cap.wrapping_add(1);
    }
    if (*r).p >= (*r).end {
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    let mut out: *mut ::core::ffi::c_char =
        malloc(cap.wrapping_mul(4 as usize).wrapping_add(1 as usize)) as *mut ::core::ffi::c_char;
    let mut o: *mut ::core::ffi::c_char = out;
    if out.is_null() {
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    let mut q: *const ::core::ffi::c_char = s;
    while q < (*r).p {
        let mut c: u8 = *q as u8;
        if (c as i32) < 0x20 as i32 {
            free(out as *mut ::core::ffi::c_void);
            return ::core::ptr::null_mut::<::core::ffi::c_char>();
        }
        if c as i32 != '\\' as i32 {
            let c2rust_fresh2 = o;
            o = o.offset(1);
            *c2rust_fresh2 = c as ::core::ffi::c_char;
        } else {
            q = q.offset(1);
            match *q as i32 {
                34 => {
                    let c2rust_fresh3 = o;
                    o = o.offset(1);
                    *c2rust_fresh3 = '"' as ::core::ffi::c_char;
                }
                92 => {
                    let c2rust_fresh4 = o;
                    o = o.offset(1);
                    *c2rust_fresh4 = '\\' as ::core::ffi::c_char;
                }
                47 => {
                    let c2rust_fresh5 = o;
                    o = o.offset(1);
                    *c2rust_fresh5 = '/' as ::core::ffi::c_char;
                }
                98 => {
                    let c2rust_fresh6 = o;
                    o = o.offset(1);
                    *c2rust_fresh6 = '\u{8}' as ::core::ffi::c_char;
                }
                102 => {
                    let c2rust_fresh7 = o;
                    o = o.offset(1);
                    *c2rust_fresh7 = '\u{c}' as ::core::ffi::c_char;
                }
                110 => {
                    let c2rust_fresh8 = o;
                    o = o.offset(1);
                    *c2rust_fresh8 = '\n' as ::core::ffi::c_char;
                }
                114 => {
                    let c2rust_fresh9 = o;
                    o = o.offset(1);
                    *c2rust_fresh9 = '\r' as ::core::ffi::c_char;
                }
                116 => {
                    let c2rust_fresh10 = o;
                    o = o.offset(1);
                    *c2rust_fresh10 = '\t' as ::core::ffi::c_char;
                }
                117 => {
                    let mut cp: u32 = 0;
                    if (*r).p.offset_from(q) < 5isize
                        || hex4(q.offset(1 as i32 as isize), &raw mut cp) != 0
                    {
                        free(out as *mut ::core::ffi::c_void);
                        return ::core::ptr::null_mut::<::core::ffi::c_char>();
                    }
                    q = q.offset(4 as i32 as isize);
                    if cp >= 0xd800 as u32 && cp < 0xdc00 as u32 {
                        let mut lo: u32 = 0;
                        if (*r).p.offset_from(q) < 7isize
                            || *q.offset(1isize) as i32 != '\\' as i32
                            || *q.offset(2isize) as i32 != 'u' as i32
                            || hex4(q.offset(3 as i32 as isize), &raw mut lo) != 0
                            || lo < 0xdc00 as u32
                            || lo > 0xdfff as u32
                        {
                            free(out as *mut ::core::ffi::c_void);
                            return ::core::ptr::null_mut::<::core::ffi::c_char>();
                        }
                        cp = (0x10000 as i32 as u32)
                            .wrapping_add(cp.wrapping_sub(0xd800 as u32) << 10 as i32)
                            .wrapping_add(lo.wrapping_sub(0xdc00 as u32));
                        q = q.offset(6 as i32 as isize);
                    } else if cp >= 0xdc00 as u32 && cp < 0xe000 as u32 {
                        free(out as *mut ::core::ffi::c_void);
                        return ::core::ptr::null_mut::<::core::ffi::c_char>();
                    }
                    put_utf8(&raw mut o, cp);
                }
                _ => {
                    free(out as *mut ::core::ffi::c_void);
                    return ::core::ptr::null_mut::<::core::ffi::c_char>();
                }
            }
        }
        q = q.offset(1);
    }
    *o = 0 as ::core::ffi::c_char;
    (*r).p = (*r).p.offset(1);
    return out;
}
unsafe extern "C" fn container(mut r: *mut rd, mut obj: i32) -> *mut eng_json {
    (*r).depth += 1;
    if (*r).depth > 64 as i32 {
        return ::core::ptr::null_mut::<eng_json>();
    }
    let mut n: *mut eng_json = node(eng_jtype(
        (if obj != 0 {
            eng_jtype::ENG_J_OBJ.0 as i32
        } else {
            eng_jtype::ENG_J_ARR.0 as i32
        }) as u32,
    ));
    let mut last: *mut eng_json = ::core::ptr::null_mut::<eng_json>();
    if n.is_null() {
        return ::core::ptr::null_mut::<eng_json>();
    }
    (*r).p = (*r).p.offset(1);
    ws(r);
    if (*r).p < (*r).end && *(*r).p as i32 == if obj != 0 { '}' as i32 } else { ']' as i32 } {
        (*r).p = (*r).p.offset(1);
        (*r).depth -= 1;
        return n;
    }
    '_bad: {
        loop {
            ws(r);
            let mut key: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
            if obj != 0 {
                key = str(r);
                if key.is_null() {
                    break '_bad;
                }
                ws(r);
                if (*r).p >= (*r).end || *(*r).p as i32 != ':' as i32 {
                    free(key as *mut ::core::ffi::c_void);
                    break '_bad;
                } else {
                    (*r).p = (*r).p.offset(1);
                }
            }
            let mut v: *mut eng_json = value(r);
            if v.is_null() {
                free(key as *mut ::core::ffi::c_void);
                break '_bad;
            } else {
                (*v).key = key;
                if !last.is_null() {
                    (*last).next = v as *mut eng_json;
                } else {
                    (*n).child = v as *mut eng_json;
                }
                last = v;
                ws(r);
                if (*r).p < (*r).end && *(*r).p as i32 == ',' as i32 {
                    (*r).p = (*r).p.offset(1);
                } else {
                    if !((*r).p < (*r).end
                        && *(*r).p as i32 == if obj != 0 { '}' as i32 } else { ']' as i32 })
                    {
                        break '_bad;
                    }
                    (*r).p = (*r).p.offset(1);
                    break;
                }
            }
        }
        (*r).depth -= 1;
        return n;
    }
    eng_json_free(n);
    return ::core::ptr::null_mut::<eng_json>();
}
unsafe extern "C" fn value(mut r: *mut rd) -> *mut eng_json {
    ws(r);
    if (*r).p >= (*r).end {
        return ::core::ptr::null_mut::<eng_json>();
    }
    let mut c: ::core::ffi::c_char = *(*r).p;
    if c as i32 == '{' as i32 {
        return container(r, 1 as i32);
    }
    if c as i32 == '[' as i32 {
        return container(r, 0 as i32);
    }
    if c as i32 == '"' as i32 {
        let mut s: *mut ::core::ffi::c_char = str(r);
        if s.is_null() {
            return ::core::ptr::null_mut::<eng_json>();
        }
        let mut n: *mut eng_json = node(eng_jtype::ENG_J_STR);
        if n.is_null() {
            free(s as *mut ::core::ffi::c_void);
            return ::core::ptr::null_mut::<eng_json>();
        }
        (*n).s = s;
        return n;
    }
    if (*r).end.offset_from((*r).p) >= 4isize
        && memcmp(
            (*r).p as *const ::core::ffi::c_void,
            b"true\0".as_ptr() as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            4 as usize,
        ) == 0
    {
        (*r).p = (*r).p.offset(4 as i32 as isize);
        let mut n_0: *mut eng_json = node(eng_jtype::ENG_J_BOOL);
        if !n_0.is_null() {
            (*n_0).b = 1 as i32;
        }
        return n_0;
    }
    if (*r).end.offset_from((*r).p) >= 5isize
        && memcmp(
            (*r).p as *const ::core::ffi::c_void,
            b"false\0".as_ptr() as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            5 as usize,
        ) == 0
    {
        (*r).p = (*r).p.offset(5 as i32 as isize);
        return node(eng_jtype::ENG_J_BOOL);
    }
    if (*r).end.offset_from((*r).p) >= 4isize
        && memcmp(
            (*r).p as *const ::core::ffi::c_void,
            b"null\0".as_ptr() as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            4 as usize,
        ) == 0
    {
        (*r).p = (*r).p.offset(4 as i32 as isize);
        return node(eng_jtype::ENG_J_NULL);
    }
    if c as i32 == '-' as i32 || c as i32 >= '0' as i32 && c as i32 <= '9' as i32 {
        let mut buf: [::core::ffi::c_char; 64] = [0; 64];
        let mut k: usize = 0 as usize;
        while (*r).p < (*r).end
            && k < ::core::mem::size_of::<[::core::ffi::c_char; 64]>().wrapping_sub(1usize)
            && !strchr(
                b"-+.eE0123456789\0".as_ptr() as *const ::core::ffi::c_char,
                *(*r).p as i32,
            )
            .is_null()
        {
            let c2rust_fresh0 = (*r).p;
            (*r).p = (*r).p.offset(1);
            let c2rust_fresh1 = k;
            k = k.wrapping_add(1);
            buf[c2rust_fresh1] = *c2rust_fresh0;
        }
        buf[k] = 0 as ::core::ffi::c_char;
        let mut e: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
        let mut d: f64 = strtod(&raw mut buf as *mut ::core::ffi::c_char, &raw mut e);
        if *e as i32 != 0 || k == 0 {
            return ::core::ptr::null_mut::<eng_json>();
        }
        let mut n_1: *mut eng_json = node(eng_jtype::ENG_J_NUM);
        if !n_1.is_null() {
            (*n_1).n = d;
        }
        return n_1;
    }
    return ::core::ptr::null_mut::<eng_json>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_json_parse(
    mut text: *const ::core::ffi::c_char,
    mut len: usize,
) -> *mut eng_json {
    let mut r: rd = rd {
        p: text,
        end: text.offset(len as isize),
        depth: 0 as i32,
    };
    let mut v: *mut eng_json = value(&raw mut r);
    if v.is_null() {
        return ::core::ptr::null_mut::<eng_json>();
    }
    ws(&raw mut r);
    if r.p != r.end {
        eng_json_free(v);
        return ::core::ptr::null_mut::<eng_json>();
    }
    return v;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_json_free(mut n: *mut eng_json) {
    while !n.is_null() {
        let mut next: *mut eng_json = (*n).next;
        eng_json_free((*n).child);
        free((*n).key as *mut ::core::ffi::c_void);
        free((*n).s as *mut ::core::ffi::c_void);
        free(n as *mut ::core::ffi::c_void);
        n = next;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_json_get(
    mut obj: *const eng_json,
    mut key: *const ::core::ffi::c_char,
) -> *const eng_json {
    if obj.is_null() || (*obj).t.0 != eng_jtype::ENG_J_OBJ.0 {
        return ::core::ptr::null::<eng_json>();
    }
    let mut c: *const eng_json = (*obj).child;
    while !c.is_null() {
        if !(*c).key.is_null() && strcmp((*c).key, key) == 0 {
            return c;
        }
        c = (*c).next;
    }
    return ::core::ptr::null::<eng_json>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_json_str(
    mut obj: *const eng_json,
    mut key: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    let mut v: *const eng_json = eng_json_get(obj, key);
    return if !v.is_null() && (*v).t.0 == eng_jtype::ENG_J_STR.0 {
        (*v).s
    } else {
        ::core::ptr::null_mut::<::core::ffi::c_char>()
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_json_int(
    mut obj: *const eng_json,
    mut key: *const ::core::ffi::c_char,
    mut out: *mut i64,
) -> i32 {
    let mut v: *const eng_json = eng_json_get(obj, key);
    if v.is_null() || (*v).t.0 != eng_jtype::ENG_J_NUM.0 || (*v).n != (*v).n as i64 as f64 {
        return -1 as i32;
    }
    *out = (*v).n as i64;
    return 0 as i32;
}
