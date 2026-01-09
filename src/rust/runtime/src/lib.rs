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

// Placeholder symbol to prove the Rust runtime staticlib is wired in.
#[no_mangle]
pub extern "C" fn lean_runtime_rs_ping() -> c_uchar {
    1
}
