#![allow(clippy::missing_safety_doc)]

use libc::{c_uchar, c_void, memcmp};

#[repr(C)]
pub struct LeanObject {
    _private: [u8; 0],
}

#[allow(dead_code)]
mod ffi {
    use super::LeanObject;
    use libc::{c_uchar, size_t};

    extern "C" {
        pub fn lean_ctor_get_ffi(o: *mut LeanObject, idx: u32) -> *mut LeanObject;
        pub fn lean_unbox_ffi(o: *mut LeanObject) -> size_t;
        pub fn lean_sarray_cptr_ffi(a: *mut LeanObject) -> *mut c_uchar;
    }
}

#[no_mangle]
pub extern "C" fn lean_byteslice_beq_rs(a: *mut LeanObject, b: *mut LeanObject) -> c_uchar {
    unsafe {
        if a == b {
            return 1;
        }

        let bytearray_a = ffi::lean_ctor_get_ffi(a, 0);
        let start_a = ffi::lean_unbox_ffi(ffi::lean_ctor_get_ffi(a, 1));
        let end_a = ffi::lean_unbox_ffi(ffi::lean_ctor_get_ffi(a, 2));

        let bytearray_b = ffi::lean_ctor_get_ffi(b, 0);
        let start_b = ffi::lean_unbox_ffi(ffi::lean_ctor_get_ffi(b, 1));
        let end_b = ffi::lean_unbox_ffi(ffi::lean_ctor_get_ffi(b, 2));

        let size_a = end_a.wrapping_sub(start_a);
        let size_b = end_b.wrapping_sub(start_b);
        if size_a != size_b {
            return 0;
        }
        if size_a == 0 {
            return 1;
        }

        let ptr_a = ffi::lean_sarray_cptr_ffi(bytearray_a).add(start_a);
        let ptr_b = ffi::lean_sarray_cptr_ffi(bytearray_b).add(start_b);
        let cmp = memcmp(
            ptr_a as *const c_void,
            ptr_b as *const c_void,
            size_a as usize,
        );
        (cmp == 0) as c_uchar
    }
}

#[no_mangle]
pub extern "C" fn hash_str_rs(len: usize, str_ptr: *const u8, init_value: u64) -> u64 {
    const M: u64 = 0xc6a4a7935bd1e995;
    const R: u32 = 47;

    unsafe {
        let mut h = init_value ^ (len as u64).wrapping_mul(M);
        let nblocks = len / 8;
        for i in 0..nblocks {
            let k_ptr = str_ptr.add(i * 8) as *const u64;
            let mut k = std::ptr::read_unaligned(k_ptr);

            k = k.wrapping_mul(M);
            k ^= k >> R;
            k = k.wrapping_mul(M);

            h ^= k;
            h = h.wrapping_mul(M);
        }

        let data2 = str_ptr.add(nblocks * 8);
        let rem = len & 7;
        if rem >= 7 {
            h ^= (data2.add(6).read() as u64) << 48;
        }
        if rem >= 6 {
            h ^= (data2.add(5).read() as u64) << 40;
        }
        if rem >= 5 {
            h ^= (data2.add(4).read() as u64) << 32;
        }
        if rem >= 4 {
            h ^= (data2.add(3).read() as u64) << 24;
        }
        if rem >= 3 {
            h ^= (data2.add(2).read() as u64) << 16;
        }
        if rem >= 2 {
            h ^= (data2.add(1).read() as u64) << 8;
        }
        if rem >= 1 {
            h ^= data2.read() as u64;
            h = h.wrapping_mul(M);
        }

        h ^= h >> R;
        h = h.wrapping_mul(M);
        h ^= h >> R;
        h
    }
}

#[no_mangle]
pub extern "C" fn lean_utf8_strlen_rs(str_ptr: *const u8) -> usize {
    unsafe {
        let mut r = 0usize;
        let mut p = str_ptr;
        while *p != 0 {
            let sz = utf8_size(*p);
            r += 1;
            p = p.add(sz as usize);
        }
        r
    }
}

#[no_mangle]
pub extern "C" fn lean_utf8_n_strlen_rs(str_ptr: *const u8, sz: usize) -> usize {
    unsafe {
        let mut r = 0usize;
        let mut i = 0usize;
        while i < sz {
            let d = utf8_size(*str_ptr.add(i));
            r += 1;
            i = i.wrapping_add(d as usize);
        }
        r
    }
}

#[inline(always)]
fn utf8_size(c: u8) -> u32 {
    if (c & 0x80) == 0 {
        1
    } else if (c & 0xE0) == 0xC0 {
        2
    } else if (c & 0xF0) == 0xE0 {
        3
    } else if (c & 0xF8) == 0xF0 {
        4
    } else if (c & 0xFC) == 0xF8 {
        5
    } else if (c & 0xFE) == 0xFC {
        6
    } else {
        1
    }
}

#[inline(always)]
fn lean_box_usize(n: usize) -> *mut LeanObject {
    ((n << 1) | 1) as *mut LeanObject
}

#[no_mangle]
pub extern "C" fn lean_system_platform_nbits_rs() -> *mut LeanObject {
    let bits = std::mem::size_of::<*const u8>() * 8;
    lean_box_usize(bits)
}

#[no_mangle]
pub extern "C" fn lean_system_platform_windows_rs() -> c_uchar {
    cfg!(windows) as c_uchar
}

#[no_mangle]
pub extern "C" fn lean_system_platform_osx_rs() -> c_uchar {
    cfg!(target_os = "macos") as c_uchar
}

#[no_mangle]
pub extern "C" fn lean_system_platform_emscripten_rs() -> c_uchar {
    cfg!(target_os = "emscripten") as c_uchar
}

// Placeholder symbol to prove the Rust runtime staticlib is wired in.
#[no_mangle]
pub extern "C" fn lean_runtime_rs_ping() -> c_uchar {
    1
}
