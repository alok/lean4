#![allow(clippy::missing_safety_doc)]

use libc::{c_char, c_uchar, c_uint};
use std::mem;

#[repr(C)]
pub struct LeanObject {
    _private: [u8; 0],
}

extern "C" {
    fn lean_rs_is_scalar(o: *mut LeanObject) -> c_uchar;
    fn lean_rs_ctor_num_objs(o: *mut LeanObject) -> c_uint;
    fn lean_rs_ctor_get_uint64(o: *mut LeanObject, offset: c_uint) -> u64;
    fn lean_rs_unbox(o: *mut LeanObject) -> usize;
    fn lean_uint64_mix_hash(a1: u64, a2: u64) -> u64;
    fn lean_internal_panic(msg: *const c_char) -> !;
}

const TOO_MANY_BVARS: &[u8] = b"too many bound variables\0";

fn data_for(o: *mut LeanObject) -> Option<u64> {
    if o.is_null() {
        return None;
    }
    let is_scalar = unsafe { lean_rs_is_scalar(o) } != 0;
    if is_scalar {
        return None;
    }
    let num_objs = unsafe { lean_rs_ctor_num_objs(o) } as usize;
    let offset = (num_objs * mem::size_of::<*mut LeanObject>()) as c_uint;
    Some(unsafe { lean_rs_ctor_get_uint64(o, offset) })
}

fn level_data_hash(data: u64) -> u32 {
    data as u32
}

fn level_data_depth(data: u64) -> u32 {
    (data >> 40) as u32
}

fn level_data_has_mvar(data: u64) -> c_uchar {
    ((data >> 32) & 1) as c_uchar
}

fn level_data_has_param(data: u64) -> c_uchar {
    ((data >> 33) & 1) as c_uchar
}

fn expr_data_hash(data: u64) -> u64 {
    data as u32 as u64
}

fn expr_data_approx_depth(data: u64) -> u32 {
    ((data >> 32) & 255) as u32
}

fn expr_data_loose_bvar_range(data: u64) -> u32 {
    (data >> 44) as u32
}

fn expr_data_has_fvar(data: u64) -> c_uchar {
    ((data >> 40) & 1) as c_uchar
}

fn expr_data_has_expr_mvar(data: u64) -> c_uchar {
    ((data >> 41) & 1) as c_uchar
}

fn expr_data_has_level_mvar(data: u64) -> c_uchar {
    ((data >> 42) & 1) as c_uchar
}

fn expr_data_has_level_param(data: u64) -> c_uchar {
    ((data >> 43) & 1) as c_uchar
}

#[no_mangle]
pub extern "C" fn lean_level_hash_rs(o: *mut LeanObject) -> u32 {
    match data_for(o) {
        None => 2221,
        Some(data) => level_data_hash(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_level_depth_rs(o: *mut LeanObject) -> u32 {
    match data_for(o) {
        None => 0,
        Some(data) => level_data_depth(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_level_has_mvar_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => level_data_has_mvar(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_level_has_param_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => level_data_has_param(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_hash_rs(o: *mut LeanObject) -> u64 {
    match data_for(o) {
        None => 0,
        Some(data) => expr_data_hash(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_has_fvar_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => expr_data_has_fvar(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_has_expr_mvar_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => expr_data_has_expr_mvar(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_has_level_mvar_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => expr_data_has_level_mvar(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_has_level_param_rs(o: *mut LeanObject) -> c_uchar {
    match data_for(o) {
        None => 0,
        Some(data) => expr_data_has_level_param(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_loose_bvar_range_rs(o: *mut LeanObject) -> u32 {
    match data_for(o) {
        None => 0,
        Some(data) => expr_data_loose_bvar_range(data),
    }
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
    let is_scalar = unsafe { lean_rs_is_scalar(bvar_range) } != 0;
    if !is_scalar {
        unsafe { lean_internal_panic(TOO_MANY_BVARS.as_ptr() as *const c_char) };
    }
    let range = unsafe { lean_rs_unbox(bvar_range) };
    if range > 1_048_575 {
        unsafe { lean_internal_panic(TOO_MANY_BVARS.as_ptr() as *const c_char) };
    }
    let r = range as u32;
    let h = hash as u32;
    (h as u64)
        | ((approx_depth as u64) << 32)
        | ((has_fvar as u64) << 40)
        | ((has_expr_mvar as u64) << 41)
        | ((has_level_mvar as u64) << 42)
        | ((has_level_param as u64) << 43)
        | ((r as u64) << 44)
}

#[no_mangle]
pub extern "C" fn lean_expr_mk_app_data_rs(f_data: u64, a_data: u64) -> u64 {
    let mut depth = std::cmp::max(expr_data_approx_depth(f_data), expr_data_approx_depth(a_data)) + 1;
    if depth > 255 {
        depth = 255;
    }
    let range = std::cmp::max(expr_data_loose_bvar_range(f_data), expr_data_loose_bvar_range(a_data));
    let h = unsafe { lean_uint64_mix_hash(f_data, a_data) } as u32;
    ((f_data | a_data) & (15u64 << 40))
        | (h as u64)
        | ((depth as u64) << 32)
        | ((range as u64) << 44)
}

// Placeholder symbol to prove the Rust kernel staticlib is wired in.
#[no_mangle]
pub extern "C" fn lean_kernel_rs_ping() -> c_uchar {
    1
}
