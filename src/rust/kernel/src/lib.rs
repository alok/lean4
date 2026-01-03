#![allow(clippy::missing_safety_doc)]

use libc::{c_uchar, c_uint};
use std::mem;

#[repr(C)]
pub struct LeanObject {
    _private: [u8; 0],
}

extern "C" {
    fn lean_rs_is_scalar(o: *mut LeanObject) -> c_uchar;
    fn lean_rs_ctor_num_objs(o: *mut LeanObject) -> c_uint;
    fn lean_rs_ctor_get_uint64(o: *mut LeanObject, offset: c_uint) -> u64;
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

fn level_data_for(o: *mut LeanObject) -> Option<u64> {
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

#[no_mangle]
pub extern "C" fn lean_level_hash_rs(o: *mut LeanObject) -> u32 {
    match level_data_for(o) {
        None => 2221,
        Some(data) => level_data_hash(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_level_depth_rs(o: *mut LeanObject) -> u32 {
    match level_data_for(o) {
        None => 0,
        Some(data) => level_data_depth(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_level_has_mvar_rs(o: *mut LeanObject) -> c_uchar {
    match level_data_for(o) {
        None => 0,
        Some(data) => level_data_has_mvar(data),
    }
}

#[no_mangle]
pub extern "C" fn lean_level_has_param_rs(o: *mut LeanObject) -> c_uchar {
    match level_data_for(o) {
        None => 0,
        Some(data) => level_data_has_param(data),
    }
}

// Placeholder symbol to prove the Rust kernel staticlib is wired in.
#[no_mangle]
pub extern "C" fn lean_kernel_rs_ping() -> c_uchar {
    1
}
