//! Streaming SHA-256 for the image format digest contract.
//! Platform ABI specialization of the frozen runtime, ported to Rust.
//! These are maintained Rust sources; the build does not compile or execute the C reference.
unsafe extern "C" {
    unsafe fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    unsafe fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type size_t = usize;
pub type uint8_t = u8;
pub type uint32_t = u32;
pub type uint64_t = u64;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_sha256 {
    pub h: [uint32_t; 8],
    pub len: uint64_t,
    pub buf: [uint8_t; 64],
    pub n: size_t,
}
static mut K: [uint32_t; 64] = [
    0x428a2f98 as uint32_t,
    0x71374491 as uint32_t,
    0xb5c0fbcf as uint32_t,
    0xe9b5dba5 as uint32_t,
    0x3956c25b as uint32_t,
    0x59f111f1 as uint32_t,
    0x923f82a4 as uint32_t,
    0xab1c5ed5 as uint32_t,
    0xd807aa98 as uint32_t,
    0x12835b01 as uint32_t,
    0x243185be as uint32_t,
    0x550c7dc3 as uint32_t,
    0x72be5d74 as uint32_t,
    0x80deb1fe as uint32_t,
    0x9bdc06a7 as uint32_t,
    0xc19bf174 as uint32_t,
    0xe49b69c1 as uint32_t,
    0xefbe4786 as uint32_t,
    0xfc19dc6 as uint32_t,
    0x240ca1cc as uint32_t,
    0x2de92c6f as uint32_t,
    0x4a7484aa as uint32_t,
    0x5cb0a9dc as uint32_t,
    0x76f988da as uint32_t,
    0x983e5152 as uint32_t,
    0xa831c66d as uint32_t,
    0xb00327c8 as uint32_t,
    0xbf597fc7 as uint32_t,
    0xc6e00bf3 as uint32_t,
    0xd5a79147 as uint32_t,
    0x6ca6351 as uint32_t,
    0x14292967 as uint32_t,
    0x27b70a85 as uint32_t,
    0x2e1b2138 as uint32_t,
    0x4d2c6dfc as uint32_t,
    0x53380d13 as uint32_t,
    0x650a7354 as uint32_t,
    0x766a0abb as uint32_t,
    0x81c2c92e as uint32_t,
    0x92722c85 as uint32_t,
    0xa2bfe8a1 as uint32_t,
    0xa81a664b as uint32_t,
    0xc24b8b70 as uint32_t,
    0xc76c51a3 as uint32_t,
    0xd192e819 as uint32_t,
    0xd6990624 as uint32_t,
    0xf40e3585 as uint32_t,
    0x106aa070 as uint32_t,
    0x19a4c116 as uint32_t,
    0x1e376c08 as uint32_t,
    0x2748774c as uint32_t,
    0x34b0bcb5 as uint32_t,
    0x391c0cb3 as uint32_t,
    0x4ed8aa4a as uint32_t,
    0x5b9cca4f as uint32_t,
    0x682e6ff3 as uint32_t,
    0x748f82ee as uint32_t,
    0x78a5636f as uint32_t,
    0x84c87814 as uint32_t,
    0x8cc70208 as uint32_t,
    0x90befffa as uint32_t,
    0xa4506ceb as uint32_t,
    0xbef9a3f7 as uint32_t,
    0xc67178f2 as uint32_t,
];
unsafe extern "C" fn block(mut c: *mut eng_sha256, mut p: *const uint8_t) {
    let mut w: [uint32_t; 64] = [0; 64];
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 16 as ::core::ffi::c_int {
        w[i as usize] = (*p.offset((4 as ::core::ffi::c_int * i) as isize) as uint32_t)
            << 24 as ::core::ffi::c_int
            | (*p.offset((4 as ::core::ffi::c_int * i + 1 as ::core::ffi::c_int) as isize)
                as uint32_t)
                << 16 as ::core::ffi::c_int
            | (*p.offset((4 as ::core::ffi::c_int * i + 2 as ::core::ffi::c_int) as isize)
                as uint32_t)
                << 8 as ::core::ffi::c_int
            | *p.offset((4 as ::core::ffi::c_int * i + 3 as ::core::ffi::c_int) as isize)
                as uint32_t;
        i += 1;
    }
    let mut i_0: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
    while i_0 < 64 as ::core::ffi::c_int {
        let mut s0: uint32_t = (w[(i_0 - 15 as ::core::ffi::c_int) as usize]
            >> 7 as ::core::ffi::c_int
            | w[(i_0 - 15 as ::core::ffi::c_int) as usize]
                << 32 as ::core::ffi::c_int - 7 as ::core::ffi::c_int)
            ^ (w[(i_0 - 15 as ::core::ffi::c_int) as usize] >> 18 as ::core::ffi::c_int
                | w[(i_0 - 15 as ::core::ffi::c_int) as usize]
                    << 32 as ::core::ffi::c_int - 18 as ::core::ffi::c_int)
            ^ w[(i_0 - 15 as ::core::ffi::c_int) as usize] >> 3 as ::core::ffi::c_int;
        let mut s1: uint32_t = (w[(i_0 - 2 as ::core::ffi::c_int) as usize]
            >> 17 as ::core::ffi::c_int
            | w[(i_0 - 2 as ::core::ffi::c_int) as usize]
                << 32 as ::core::ffi::c_int - 17 as ::core::ffi::c_int)
            ^ (w[(i_0 - 2 as ::core::ffi::c_int) as usize] >> 19 as ::core::ffi::c_int
                | w[(i_0 - 2 as ::core::ffi::c_int) as usize]
                    << 32 as ::core::ffi::c_int - 19 as ::core::ffi::c_int)
            ^ w[(i_0 - 2 as ::core::ffi::c_int) as usize] >> 10 as ::core::ffi::c_int;
        w[i_0 as usize] = w[(i_0 - 16 as ::core::ffi::c_int) as usize]
            .wrapping_add(s0)
            .wrapping_add(w[(i_0 - 7 as ::core::ffi::c_int) as usize])
            .wrapping_add(s1);
        i_0 += 1;
    }
    let mut a: uint32_t = (*c).h[0usize];
    let mut b: uint32_t = (*c).h[1usize];
    let mut cc: uint32_t = (*c).h[2usize];
    let mut d: uint32_t = (*c).h[3usize];
    let mut e: uint32_t = (*c).h[4usize];
    let mut f: uint32_t = (*c).h[5usize];
    let mut g: uint32_t = (*c).h[6usize];
    let mut h: uint32_t = (*c).h[7usize];
    let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_1 < 64 as ::core::ffi::c_int {
        let mut S1: uint32_t = (e >> 6 as ::core::ffi::c_int
            | e << 32 as ::core::ffi::c_int - 6 as ::core::ffi::c_int)
            ^ (e >> 11 as ::core::ffi::c_int
                | e << 32 as ::core::ffi::c_int - 11 as ::core::ffi::c_int)
            ^ (e >> 25 as ::core::ffi::c_int
                | e << 32 as ::core::ffi::c_int - 25 as ::core::ffi::c_int);
        let mut ch: uint32_t = e & f ^ !e & g;
        let mut t1: uint32_t = h
            .wrapping_add(S1)
            .wrapping_add(ch)
            .wrapping_add(K[i_1 as usize])
            .wrapping_add(w[i_1 as usize]);
        let mut S0: uint32_t = (a >> 2 as ::core::ffi::c_int
            | a << 32 as ::core::ffi::c_int - 2 as ::core::ffi::c_int)
            ^ (a >> 13 as ::core::ffi::c_int
                | a << 32 as ::core::ffi::c_int - 13 as ::core::ffi::c_int)
            ^ (a >> 22 as ::core::ffi::c_int
                | a << 32 as ::core::ffi::c_int - 22 as ::core::ffi::c_int);
        let mut mj: uint32_t = a & b ^ a & cc ^ b & cc;
        let mut t2: uint32_t = S0.wrapping_add(mj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = cc;
        cc = b;
        b = a;
        a = t1.wrapping_add(t2);
        i_1 += 1;
    }
    (*c).h[0usize] = (*c).h[0usize].wrapping_add(a);
    (*c).h[1usize] = (*c).h[1usize].wrapping_add(b);
    (*c).h[2usize] = (*c).h[2usize].wrapping_add(cc);
    (*c).h[3usize] = (*c).h[3usize].wrapping_add(d);
    (*c).h[4usize] = (*c).h[4usize].wrapping_add(e);
    (*c).h[5usize] = (*c).h[5usize].wrapping_add(f);
    (*c).h[6usize] = (*c).h[6usize].wrapping_add(g);
    (*c).h[7usize] = (*c).h[7usize].wrapping_add(h);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sha256_init(mut c: *mut eng_sha256) {
    static mut H0: [uint32_t; 8] = [
        0x6a09e667 as uint32_t,
        0xbb67ae85 as uint32_t,
        0x3c6ef372 as uint32_t,
        0xa54ff53a as uint32_t,
        0x510e527f as uint32_t,
        0x9b05688c as uint32_t,
        0x1f83d9ab as uint32_t,
        0x5be0cd19 as uint32_t,
    ];
    memcpy(
        &raw mut (*c).h as *mut uint32_t as *mut ::core::ffi::c_void,
        &raw const H0 as *const uint32_t as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[uint32_t; 8]>(),
    );
    (*c).len = 0 as uint64_t;
    (*c).n = 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sha256_update(
    mut c: *mut eng_sha256,
    mut data: *const ::core::ffi::c_void,
    mut n: size_t,
) {
    let mut p: *const uint8_t = data as *const uint8_t;
    (*c).len =
        ((*c).len as ::core::ffi::c_ulong).wrapping_add(n as ::core::ffi::c_ulong) as uint64_t;
    if (*c).n != 0 {
        let mut take: size_t = if (64 as size_t).wrapping_sub((*c).n) < n {
            (64 as size_t).wrapping_sub((*c).n)
        } else {
            n
        };
        memcpy(
            (&raw mut (*c).buf as *mut uint8_t).offset((*c).n as isize) as *mut ::core::ffi::c_void,
            p as *const ::core::ffi::c_void,
            take,
        );
        (*c).n = (*c).n.wrapping_add(take);
        p = p.offset(take as isize);
        n = n.wrapping_sub(take);
        if (*c).n == 64 as size_t {
            block(c, &raw mut (*c).buf as *mut uint8_t);
            (*c).n = 0 as size_t;
        }
    }
    while n >= 64 as size_t {
        block(c, p);
        p = p.offset(64 as ::core::ffi::c_int as isize);
        n = n.wrapping_sub(64 as size_t);
    }
    if n != 0 {
        memcpy(
            &raw mut (*c).buf as *mut uint8_t as *mut ::core::ffi::c_void,
            p as *const ::core::ffi::c_void,
            n,
        );
        (*c).n = n;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sha256_final(mut c: *mut eng_sha256, mut out: *mut uint8_t) {
    let mut bits: uint64_t = (*c).len.wrapping_mul(8 as uint64_t);
    let mut pad: uint8_t = 0x80 as uint8_t;
    eng_sha256_update(c, &raw mut pad as *const ::core::ffi::c_void, 1 as size_t);
    let mut z: uint8_t = 0 as uint8_t;
    while (*c).n != 56 as size_t {
        eng_sha256_update(c, &raw mut z as *const ::core::ffi::c_void, 1 as size_t);
    }
    let mut lb: [uint8_t; 8] = [0; 8];
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 8 as ::core::ffi::c_int {
        lb[i as usize] =
            (bits >> 56 as ::core::ffi::c_int - 8 as ::core::ffi::c_int * i) as uint8_t;
        i += 1;
    }
    eng_sha256_update(
        c,
        &raw mut lb as *mut uint8_t as *const ::core::ffi::c_void,
        8 as size_t,
    );
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < 8 as ::core::ffi::c_int {
        *out.offset((4 as ::core::ffi::c_int * i_0) as isize) =
            ((*c).h[i_0 as usize] >> 24 as ::core::ffi::c_int) as uint8_t;
        *out.offset((4 as ::core::ffi::c_int * i_0 + 1 as ::core::ffi::c_int) as isize) =
            ((*c).h[i_0 as usize] >> 16 as ::core::ffi::c_int) as uint8_t;
        *out.offset((4 as ::core::ffi::c_int * i_0 + 2 as ::core::ffi::c_int) as isize) =
            ((*c).h[i_0 as usize] >> 8 as ::core::ffi::c_int) as uint8_t;
        *out.offset((4 as ::core::ffi::c_int * i_0 + 3 as ::core::ffi::c_int) as isize) =
            (*c).h[i_0 as usize] as uint8_t;
        i_0 += 1;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sha256_hex(mut d: *const uint8_t, mut out: *mut ::core::ffi::c_char) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 32 as ::core::ffi::c_int {
        snprintf(
            out.offset((2 as ::core::ffi::c_int * i) as isize),
            3 as size_t,
            b"%02x\0".as_ptr() as *const ::core::ffi::c_char,
            *d.offset(i as isize) as ::core::ffi::c_int,
        );
        i += 1;
    }
}
