//! Streaming SHA-256 for the image format digest contract.
//! Shared runtime logic; target ABI differences are selected explicitly with cfg.
unsafe extern "C" {
    unsafe fn snprintf(
        _: *mut ::core::ffi::c_char,
        _: usize,
        _: *const ::core::ffi::c_char,
        ...
    ) -> i32;
    unsafe fn memcpy(
        _: *mut ::core::ffi::c_void,
        _: *const ::core::ffi::c_void,
        _: usize,
    ) -> *mut ::core::ffi::c_void;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct eng_sha256 {
    pub h: [u32; 8],
    pub len: u64,
    pub buf: [u8; 64],
    pub n: usize,
}
static mut K: [u32; 64] = [
    0x428a2f98 as u32,
    0x71374491 as u32,
    0xb5c0fbcf as u32,
    0xe9b5dba5 as u32,
    0x3956c25b as u32,
    0x59f111f1 as u32,
    0x923f82a4 as u32,
    0xab1c5ed5 as u32,
    0xd807aa98 as u32,
    0x12835b01 as u32,
    0x243185be as u32,
    0x550c7dc3 as u32,
    0x72be5d74 as u32,
    0x80deb1fe as u32,
    0x9bdc06a7 as u32,
    0xc19bf174 as u32,
    0xe49b69c1 as u32,
    0xefbe4786 as u32,
    0xfc19dc6 as u32,
    0x240ca1cc as u32,
    0x2de92c6f as u32,
    0x4a7484aa as u32,
    0x5cb0a9dc as u32,
    0x76f988da as u32,
    0x983e5152 as u32,
    0xa831c66d as u32,
    0xb00327c8 as u32,
    0xbf597fc7 as u32,
    0xc6e00bf3 as u32,
    0xd5a79147 as u32,
    0x6ca6351 as u32,
    0x14292967 as u32,
    0x27b70a85 as u32,
    0x2e1b2138 as u32,
    0x4d2c6dfc as u32,
    0x53380d13 as u32,
    0x650a7354 as u32,
    0x766a0abb as u32,
    0x81c2c92e as u32,
    0x92722c85 as u32,
    0xa2bfe8a1 as u32,
    0xa81a664b as u32,
    0xc24b8b70 as u32,
    0xc76c51a3 as u32,
    0xd192e819 as u32,
    0xd6990624 as u32,
    0xf40e3585 as u32,
    0x106aa070 as u32,
    0x19a4c116 as u32,
    0x1e376c08 as u32,
    0x2748774c as u32,
    0x34b0bcb5 as u32,
    0x391c0cb3 as u32,
    0x4ed8aa4a as u32,
    0x5b9cca4f as u32,
    0x682e6ff3 as u32,
    0x748f82ee as u32,
    0x78a5636f as u32,
    0x84c87814 as u32,
    0x8cc70208 as u32,
    0x90befffa as u32,
    0xa4506ceb as u32,
    0xbef9a3f7 as u32,
    0xc67178f2 as u32,
];
unsafe extern "C" fn block(mut c: *mut eng_sha256, mut p: *const u8) {
    let mut w: [u32; 64] = [0; 64];
    let mut i: i32 = 0 as i32;
    while i < 16 as i32 {
        w[i as usize] = (*p.offset((4 as i32 * i) as isize) as u32) << 24 as i32
            | (*p.offset((4 as i32 * i + 1 as i32) as isize) as u32) << 16 as i32
            | (*p.offset((4 as i32 * i + 2 as i32) as isize) as u32) << 8 as i32
            | *p.offset((4 as i32 * i + 3 as i32) as isize) as u32;
        i += 1;
    }
    let mut i_0: i32 = 16 as i32;
    while i_0 < 64 as i32 {
        let mut s0: u32 = (w[(i_0 - 15 as i32) as usize] >> 7 as i32
            | w[(i_0 - 15 as i32) as usize] << 32 as i32 - 7 as i32)
            ^ (w[(i_0 - 15 as i32) as usize] >> 18 as i32
                | w[(i_0 - 15 as i32) as usize] << 32 as i32 - 18 as i32)
            ^ w[(i_0 - 15 as i32) as usize] >> 3 as i32;
        let mut s1: u32 = (w[(i_0 - 2 as i32) as usize] >> 17 as i32
            | w[(i_0 - 2 as i32) as usize] << 32 as i32 - 17 as i32)
            ^ (w[(i_0 - 2 as i32) as usize] >> 19 as i32
                | w[(i_0 - 2 as i32) as usize] << 32 as i32 - 19 as i32)
            ^ w[(i_0 - 2 as i32) as usize] >> 10 as i32;
        w[i_0 as usize] = w[(i_0 - 16 as i32) as usize]
            .wrapping_add(s0)
            .wrapping_add(w[(i_0 - 7 as i32) as usize])
            .wrapping_add(s1);
        i_0 += 1;
    }
    let mut a: u32 = (*c).h[0usize];
    let mut b: u32 = (*c).h[1usize];
    let mut cc: u32 = (*c).h[2usize];
    let mut d: u32 = (*c).h[3usize];
    let mut e: u32 = (*c).h[4usize];
    let mut f: u32 = (*c).h[5usize];
    let mut g: u32 = (*c).h[6usize];
    let mut h: u32 = (*c).h[7usize];
    let mut i_1: i32 = 0 as i32;
    while i_1 < 64 as i32 {
        let mut S1: u32 = (e >> 6 as i32 | e << 32 as i32 - 6 as i32)
            ^ (e >> 11 as i32 | e << 32 as i32 - 11 as i32)
            ^ (e >> 25 as i32 | e << 32 as i32 - 25 as i32);
        let mut ch: u32 = e & f ^ !e & g;
        let mut t1: u32 = h
            .wrapping_add(S1)
            .wrapping_add(ch)
            .wrapping_add(K[i_1 as usize])
            .wrapping_add(w[i_1 as usize]);
        let mut S0: u32 = (a >> 2 as i32 | a << 32 as i32 - 2 as i32)
            ^ (a >> 13 as i32 | a << 32 as i32 - 13 as i32)
            ^ (a >> 22 as i32 | a << 32 as i32 - 22 as i32);
        let mut mj: u32 = a & b ^ a & cc ^ b & cc;
        let mut t2: u32 = S0.wrapping_add(mj);
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
    static mut H0: [u32; 8] = [
        0x6a09e667 as u32,
        0xbb67ae85 as u32,
        0x3c6ef372 as u32,
        0xa54ff53a as u32,
        0x510e527f as u32,
        0x9b05688c as u32,
        0x1f83d9ab as u32,
        0x5be0cd19 as u32,
    ];
    memcpy(
        &raw mut (*c).h as *mut u32 as *mut ::core::ffi::c_void,
        &raw const H0 as *const u32 as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[u32; 8]>(),
    );
    (*c).len = 0 as u64;
    (*c).n = 0 as usize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sha256_update(
    mut c: *mut eng_sha256,
    mut data: *const ::core::ffi::c_void,
    mut n: usize,
) {
    let mut p: *const u8 = data as *const u8;
    (*c).len = ((*c).len as u64).wrapping_add(n as u64) as u64;
    if (*c).n != 0 {
        let mut take: usize = if (64 as usize).wrapping_sub((*c).n) < n {
            (64 as usize).wrapping_sub((*c).n)
        } else {
            n
        };
        memcpy(
            (&raw mut (*c).buf as *mut u8).offset((*c).n as isize) as *mut ::core::ffi::c_void,
            p as *const ::core::ffi::c_void,
            take,
        );
        (*c).n = (*c).n.wrapping_add(take);
        p = p.offset(take as isize);
        n = n.wrapping_sub(take);
        if (*c).n == 64 as usize {
            block(c, &raw mut (*c).buf as *mut u8);
            (*c).n = 0 as usize;
        }
    }
    while n >= 64 as usize {
        block(c, p);
        p = p.offset(64 as i32 as isize);
        n = n.wrapping_sub(64 as usize);
    }
    if n != 0 {
        memcpy(
            &raw mut (*c).buf as *mut u8 as *mut ::core::ffi::c_void,
            p as *const ::core::ffi::c_void,
            n,
        );
        (*c).n = n;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sha256_final(mut c: *mut eng_sha256, mut out: *mut u8) {
    let mut bits: u64 = (*c).len.wrapping_mul(8 as u64);
    let mut pad: u8 = 0x80 as u8;
    eng_sha256_update(c, &raw mut pad as *const ::core::ffi::c_void, 1 as usize);
    let mut z: u8 = 0 as u8;
    while (*c).n != 56 as usize {
        eng_sha256_update(c, &raw mut z as *const ::core::ffi::c_void, 1 as usize);
    }
    let mut lb: [u8; 8] = [0; 8];
    let mut i: i32 = 0 as i32;
    while i < 8 as i32 {
        lb[i as usize] = (bits >> 56 as i32 - 8 as i32 * i) as u8;
        i += 1;
    }
    eng_sha256_update(
        c,
        &raw mut lb as *mut u8 as *const ::core::ffi::c_void,
        8 as usize,
    );
    let mut i_0: i32 = 0 as i32;
    while i_0 < 8 as i32 {
        *out.offset((4 as i32 * i_0) as isize) = ((*c).h[i_0 as usize] >> 24 as i32) as u8;
        *out.offset((4 as i32 * i_0 + 1 as i32) as isize) =
            ((*c).h[i_0 as usize] >> 16 as i32) as u8;
        *out.offset((4 as i32 * i_0 + 2 as i32) as isize) =
            ((*c).h[i_0 as usize] >> 8 as i32) as u8;
        *out.offset((4 as i32 * i_0 + 3 as i32) as isize) = (*c).h[i_0 as usize] as u8;
        i_0 += 1;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eng_sha256_hex(mut d: *const u8, mut out: *mut ::core::ffi::c_char) {
    let mut i: i32 = 0 as i32;
    while i < 32 as i32 {
        snprintf(
            out.offset((2 as i32 * i) as isize),
            3 as usize,
            b"%02x\0".as_ptr() as *const ::core::ffi::c_char,
            *d.offset(i as isize) as i32,
        );
        i += 1;
    }
}
