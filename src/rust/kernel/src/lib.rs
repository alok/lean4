#![allow(clippy::missing_safety_doc)]

mod bitfield;
mod object;
mod ptr;

use libc::{c_char, c_uchar};
use static_assertions::const_assert;

#[repr(C)]
pub struct LeanObject {
    _private: [u8; 0],
}

#[allow(dead_code)]
mod ffi {
    use super::LeanObject;
    use libc::{c_char, c_uchar, c_uint};

    extern "C" {
        pub fn lean_rs_is_scalar(o: *mut LeanObject) -> c_uchar;
        pub fn lean_rs_ctor_num_objs(o: *mut LeanObject) -> c_uint;
        pub fn lean_rs_ctor_get_uint64(o: *mut LeanObject, offset: c_uint) -> u64;
        pub fn lean_rs_unbox(o: *mut LeanObject) -> usize;
        pub fn lean_expr_binder_info(o: *mut LeanObject) -> c_uchar;
        pub fn lean_uint64_mix_hash(a1: u64, a2: u64) -> u64;
        pub fn lean_internal_panic(msg: *const c_char) -> !;
        pub fn lean_rs_ctor_obj_cptr(o: *mut LeanObject) -> *mut *mut LeanObject;
        pub fn lean_rs_ctor_scalar_cptr(o: *mut LeanObject) -> *mut u8;
        pub fn lean_rs_array_size(o: *mut LeanObject) -> usize;
        pub fn lean_rs_array_cptr(o: *mut LeanObject) -> *mut *mut LeanObject;
        pub fn lean_rs_sarray_size(o: *mut LeanObject) -> usize;
        pub fn lean_rs_sarray_cptr(o: *mut LeanObject) -> *mut u8;
        pub fn lean_rs_string_size(o: *mut LeanObject) -> usize;
        pub fn lean_rs_string_len(o: *mut LeanObject) -> usize;
        pub fn lean_rs_string_cstr(o: *mut LeanObject) -> *const c_char;
    }
}

use bitfield as bf;
use object::LeanObj;
use ptr as lean_ptr;

const TOO_MANY_BVARS: &[u8] = b"too many bound variables\0";
const LEVEL_DEPTH_TOO_BIG: &[u8] = b"universe level depth is too big\0";

const LEVEL_HASH_SHIFT: u32 = 0;
const LEVEL_HASH_WIDTH: u32 = 32;
const LEVEL_HAS_MVAR_SHIFT: u32 = 32;
const LEVEL_HAS_PARAM_SHIFT: u32 = 33;
const LEVEL_DEPTH_SHIFT: u32 = 40;
const LEVEL_DEPTH_WIDTH: u32 = 24;

const EXPR_HASH_SHIFT: u32 = 0;
const EXPR_HASH_WIDTH: u32 = 32;
const EXPR_APPROX_DEPTH_SHIFT: u32 = 32;
const EXPR_APPROX_DEPTH_WIDTH: u32 = 8;
const EXPR_HAS_FVAR_SHIFT: u32 = 40;
const EXPR_HAS_EXPR_MVAR_SHIFT: u32 = 41;
const EXPR_HAS_LEVEL_MVAR_SHIFT: u32 = 42;
const EXPR_HAS_LEVEL_PARAM_SHIFT: u32 = 43;
const EXPR_BVAR_RANGE_SHIFT: u32 = 44;
const EXPR_BVAR_RANGE_WIDTH: u32 = 20;

const_assert!(LEVEL_HASH_SHIFT + LEVEL_HASH_WIDTH <= 64);
const_assert!(LEVEL_DEPTH_SHIFT + LEVEL_DEPTH_WIDTH <= 64);
const_assert!(EXPR_HASH_SHIFT + EXPR_HASH_WIDTH <= 64);
const_assert!(EXPR_APPROX_DEPTH_SHIFT + EXPR_APPROX_DEPTH_WIDTH <= 64);
const_assert!(EXPR_BVAR_RANGE_SHIFT + EXPR_BVAR_RANGE_WIDTH <= 64);

const EXPR_FLAGS_MASK: u64 =
    bf::mask::<4>() << EXPR_HAS_FVAR_SHIFT;

#[repr(transparent)]
#[derive(Copy, Clone)]
struct LevelData(u64);

impl LevelData {
    #[inline(always)]
    fn hash(self) -> u32 {
        bf::get::<LEVEL_HASH_SHIFT, LEVEL_HASH_WIDTH>(self.0) as u32
    }

    #[inline(always)]
    fn depth(self) -> u32 {
        bf::get::<LEVEL_DEPTH_SHIFT, LEVEL_DEPTH_WIDTH>(self.0) as u32
    }

    #[inline(always)]
    fn has_mvar(self) -> c_uchar {
        bf::get_bit::<LEVEL_HAS_MVAR_SHIFT>(self.0) as c_uchar
    }

    #[inline(always)]
    fn has_param(self) -> c_uchar {
        bf::get_bit::<LEVEL_HAS_PARAM_SHIFT>(self.0) as c_uchar
    }

    #[inline(always)]
    const fn pack(hash: u32, depth: u32, has_mvar: bool, has_param: bool) -> u64 {
        let mut v = 0u64;
        v = bf::set::<LEVEL_HASH_SHIFT, LEVEL_HASH_WIDTH>(v, hash as u64);
        v = bf::set::<LEVEL_HAS_MVAR_SHIFT, 1>(v, has_mvar as u64);
        v = bf::set::<LEVEL_HAS_PARAM_SHIFT, 1>(v, has_param as u64);
        bf::set::<LEVEL_DEPTH_SHIFT, LEVEL_DEPTH_WIDTH>(v, depth as u64)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
struct ExprData(u64);

impl ExprData {
    #[inline(always)]
    fn hash(self) -> u32 {
        bf::get::<EXPR_HASH_SHIFT, EXPR_HASH_WIDTH>(self.0) as u32
    }

    #[inline(always)]
    fn approx_depth(self) -> u32 {
        bf::get::<EXPR_APPROX_DEPTH_SHIFT, EXPR_APPROX_DEPTH_WIDTH>(self.0) as u32
    }

    #[inline(always)]
    fn loose_bvar_range(self) -> u32 {
        bf::get::<EXPR_BVAR_RANGE_SHIFT, EXPR_BVAR_RANGE_WIDTH>(self.0) as u32
    }

    #[inline(always)]
    fn has_fvar(self) -> c_uchar {
        bf::get_bit::<EXPR_HAS_FVAR_SHIFT>(self.0) as c_uchar
    }

    #[inline(always)]
    fn has_expr_mvar(self) -> c_uchar {
        bf::get_bit::<EXPR_HAS_EXPR_MVAR_SHIFT>(self.0) as c_uchar
    }

    #[inline(always)]
    fn has_level_mvar(self) -> c_uchar {
        bf::get_bit::<EXPR_HAS_LEVEL_MVAR_SHIFT>(self.0) as c_uchar
    }

    #[inline(always)]
    fn has_level_param(self) -> c_uchar {
        bf::get_bit::<EXPR_HAS_LEVEL_PARAM_SHIFT>(self.0) as c_uchar
    }

    #[inline(always)]
    fn flags(self) -> u64 {
        self.0 & EXPR_FLAGS_MASK
    }

    #[inline(always)]
    const fn pack_with_flags(hash: u32, range: u32, depth: u32, flags: u64) -> u64 {
        let mut v = 0u64;
        v = bf::set::<EXPR_HASH_SHIFT, EXPR_HASH_WIDTH>(v, hash as u64);
        v = bf::set::<EXPR_APPROX_DEPTH_SHIFT, EXPR_APPROX_DEPTH_WIDTH>(v, depth as u64);
        v = bf::set::<EXPR_BVAR_RANGE_SHIFT, EXPR_BVAR_RANGE_WIDTH>(v, range as u64);
        v | (flags & EXPR_FLAGS_MASK)
    }
}

fn data_for(o: *mut LeanObject) -> Option<u64> {
    let obj = unsafe { LeanObj::new(o)? };
    if obj.is_scalar() {
        None
    } else {
        Some(obj.ctor_data_u64())
    }
}


#[no_mangle]
pub extern "C" fn lean_level_hash_rs(o: *mut LeanObject) -> u32 {
    match data_for(o) {
        None => 2221,
        Some(data) => LevelData(data).hash(),
    }
}

#[no_mangle]
pub extern "C" fn lean_level_depth_rs(o: *mut LeanObject) -> u32 {
    match data_for(o) {
        None => 0,
        Some(data) => LevelData(data).depth(),
    }
}

#[no_mangle]
pub extern "C" fn lean_level_has_mvar_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => LevelData(data).has_mvar(),
    }
}

#[no_mangle]
pub extern "C" fn lean_level_has_param_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => LevelData(data).has_param(),
    }
}

#[no_mangle]
pub extern "C" fn lean_level_mk_data_rs(
    hash: u64,
    depth: *mut LeanObject,
    has_mvar: c_uchar,
    has_param: c_uchar,
) -> u64 {
    let is_scalar = lean_ptr::is_scalar_ptr(depth);
    if !is_scalar {
        unsafe { ffi::lean_internal_panic(LEVEL_DEPTH_TOO_BIG.as_ptr() as *const c_char) };
    }
    let d = lean_ptr::unbox_ptr(depth);
    if d > 16_777_215 {
        unsafe { ffi::lean_internal_panic(LEVEL_DEPTH_TOO_BIG.as_ptr() as *const c_char) };
    }
    LevelData::pack(hash as u32, d as u32, has_mvar != 0, has_param != 0)
}

#[no_mangle]
pub extern "C" fn lean_expr_hash_rs(o: *mut LeanObject) -> u64 {
    match data_for(o) {
        None => 0,
        Some(data) => ExprData(data).hash() as u64,
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_has_fvar_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => ExprData(data).has_fvar(),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_has_expr_mvar_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => ExprData(data).has_expr_mvar(),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_has_level_mvar_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => ExprData(data).has_level_mvar(),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_has_level_param_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => ExprData(data).has_level_param(),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_loose_bvar_range_rs(o: *mut LeanObject) -> u32 {
    match data_for(o) {
        None => 0,
        Some(data) => ExprData(data).loose_bvar_range(),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_binder_info_rs(o: *mut LeanObject) -> c_uchar {
    unsafe { ffi::lean_expr_binder_info(o) }
}

#[no_mangle]
pub extern "C" fn lean_expr_mk_data_rs(
    hash: u64,
    bvar_range: *mut LeanObject,
    mut approx_depth: u32,
    has_fvar: c_uchar,
    has_expr_mvar: c_uchar,
    has_level_mvar: c_uchar,
    has_level_param: c_uchar,
) -> u64 {
    if approx_depth > 255 {
        approx_depth = 255;
    }
    let is_scalar = lean_ptr::is_scalar_ptr(bvar_range);
    if !is_scalar {
        unsafe { ffi::lean_internal_panic(TOO_MANY_BVARS.as_ptr() as *const c_char) };
    }
    let range = lean_ptr::unbox_ptr(bvar_range);
    if range > 1_048_575 {
        unsafe { ffi::lean_internal_panic(TOO_MANY_BVARS.as_ptr() as *const c_char) };
    }
    let flags = ((has_fvar != 0) as u64) << EXPR_HAS_FVAR_SHIFT
        | ((has_expr_mvar != 0) as u64) << EXPR_HAS_EXPR_MVAR_SHIFT
        | ((has_level_mvar != 0) as u64) << EXPR_HAS_LEVEL_MVAR_SHIFT
        | ((has_level_param != 0) as u64) << EXPR_HAS_LEVEL_PARAM_SHIFT;
    ExprData::pack_with_flags(hash as u32, range as u32, approx_depth, flags)
}

#[no_mangle]
pub extern "C" fn lean_expr_mk_app_data_rs(f_data: u64, a_data: u64) -> u64 {
    let f = ExprData(f_data);
    let a = ExprData(a_data);
    let mut depth = std::cmp::max(f.approx_depth(), a.approx_depth()) + 1;
    if depth > 255 {
        depth = 255;
    }
    let range = std::cmp::max(f.loose_bvar_range(), a.loose_bvar_range());
    let h = unsafe { ffi::lean_uint64_mix_hash(f_data, a_data) } as u32;
    let flags = f.flags() | a.flags();
    ExprData::pack_with_flags(h, range, depth, flags)
}

// Placeholder symbol to prove the Rust kernel staticlib is wired in.
#[no_mangle]
pub extern "C" fn lean_kernel_rs_ping() -> c_uchar {
    1
}
