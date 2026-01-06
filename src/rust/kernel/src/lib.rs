#![allow(clippy::missing_safety_doc)]

mod bitfield;
mod layout;
mod object;
mod ptr;

use libc::{c_char, c_uchar};
use static_assertions::const_assert;
use std::collections::HashSet;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

#[repr(C)]
pub struct LeanObject {
    _private: [u8; 0],
}

#[allow(dead_code)]
mod ffi {
    use libc::{c_char, c_uchar};
    use super::LeanObject;

    extern "C" {
        pub fn lean_uint64_mix_hash(a1: u64, a2: u64) -> u64;
        pub fn lean_internal_panic(msg: *const c_char) -> !;
        pub fn lean_dec_ref_cold(o: *mut LeanObject);
        pub fn lean_mark_persistent(o: *mut LeanObject);
        pub fn lean_mk_string(s: *const c_char) -> *mut LeanObject;
        pub fn lean_name_eq(n1: *mut LeanObject, n2: *mut LeanObject) -> c_uchar;
        pub fn l_Lean_Name_mkStr1(s: *mut LeanObject) -> *mut LeanObject;
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
const EXPR_DATA_BYTES: usize = std::mem::size_of::<u64>();
const EXPR_BVAR_TAG: u8 = 0;
const EXPR_FVAR_TAG: u8 = 1;
const EXPR_MVAR_TAG: u8 = 2;
const EXPR_SORT_TAG: u8 = 3;
const EXPR_CONST_TAG: u8 = 4;
const EXPR_APP_TAG: u8 = 5;
const EXPR_LAM_TAG: u8 = 6;
const EXPR_FORALL_TAG: u8 = 7;
const EXPR_LET_TAG: u8 = 8;
const EXPR_LIT_TAG: u8 = 9;
const EXPR_MDATA_TAG: u8 = 10;
const EXPR_PROJ_TAG: u8 = 11;

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

#[derive(Copy, Clone)]
struct TypeAnnotationNames {
    opt_param: usize,
    auto_param: usize,
    out_param: usize,
    semi_out_param: usize,
}

const OPT_PARAM_NAME: &[u8] = b"optParam\0";
const AUTO_PARAM_NAME: &[u8] = b"autoParam\0";
const OUT_PARAM_NAME: &[u8] = b"outParam\0";
const SEMI_OUT_PARAM_NAME: &[u8] = b"semiOutParam\0";

fn type_annotation_names() -> TypeAnnotationNames {
    static NAMES: OnceLock<TypeAnnotationNames> = OnceLock::new();
    *NAMES.get_or_init(|| unsafe {
        TypeAnnotationNames {
            opt_param: mk_name(OPT_PARAM_NAME) as usize,
            auto_param: mk_name(AUTO_PARAM_NAME) as usize,
            out_param: mk_name(OUT_PARAM_NAME) as usize,
            semi_out_param: mk_name(SEMI_OUT_PARAM_NAME) as usize,
        }
    })
}

unsafe fn mk_name(bytes: &[u8]) -> *mut LeanObject {
    let s = ffi::lean_mk_string(bytes.as_ptr() as *const c_char);
    let name = ffi::l_Lean_Name_mkStr1(s);
    ffi::lean_mark_persistent(name);
    name
}

#[inline(always)]
unsafe fn lean_inc_ref_n(o: *mut LeanObject, n: i32) {
    let header = o as *mut layout::LeanObjectHeader;
    let rc = (*header).rc;
    if rc > 0 {
        (*header).rc = rc + n;
    } else if rc != 0 {
        let atomic = &*(std::ptr::addr_of!((*header).rc) as *const AtomicI32);
        atomic.fetch_sub(n, Ordering::Relaxed);
    }
}

#[inline(always)]
unsafe fn lean_inc(o: *mut LeanObject) {
    if !lean_ptr::is_scalar_ptr(o) {
        lean_inc_ref_n(o, 1);
    }
}

#[inline(always)]
unsafe fn lean_dec(o: *mut LeanObject) {
    if lean_ptr::is_scalar_ptr(o) {
        return;
    }
    let header = o as *mut layout::LeanObjectHeader;
    let rc = (*header).rc;
    if rc > 1 {
        (*header).rc = rc - 1;
    } else if rc != 0 {
        ffi::lean_dec_ref_cold(o);
    }
}

#[inline(always)]
unsafe fn is_likely_unshared(o: *mut LeanObject) -> bool {
    if lean_ptr::is_scalar_ptr(o) {
        return true;
    }
    let rc = layout::header(o).rc;
    rc == 1 || rc == -1
}

#[inline(always)]
unsafe fn expr_is_const_name(e: *mut LeanObject, name: *mut LeanObject) -> bool {
    if e.is_null() || lean_ptr::is_scalar_ptr(e) {
        return false;
    }
    if layout::header(e).tag != EXPR_CONST_TAG {
        return false;
    }
    let obj = LeanObj::new(e).unwrap();
    let name_ptr = *obj.ctor_obj_ptr();
    ffi::lean_name_eq(name_ptr, name) != 0
}

#[inline(always)]
unsafe fn expr_loose_bvar_range(o: *mut LeanObject) -> u32 {
    match data_for(o) {
        None => 0,
        Some(data) => ExprData(data).loose_bvar_range(),
    }
}

#[inline(always)]
unsafe fn match_app1_const_arg(
    e: *mut LeanObject,
    name: *mut LeanObject,
) -> Option<*mut LeanObject> {
    if e.is_null() || lean_ptr::is_scalar_ptr(e) {
        return None;
    }
    if layout::header(e).tag != EXPR_APP_TAG {
        return None;
    }
    let e_obj = LeanObj::new(e).unwrap();
    let objs = e_obj.ctor_obj_ptr();
    let f = *objs;
    if !expr_is_const_name(f, name) {
        return None;
    }
    Some(*objs.add(1))
}

#[inline(always)]
unsafe fn match_app2_const_arg1(
    e: *mut LeanObject,
    name: *mut LeanObject,
) -> Option<*mut LeanObject> {
    if e.is_null() || lean_ptr::is_scalar_ptr(e) {
        return None;
    }
    if layout::header(e).tag != EXPR_APP_TAG {
        return None;
    }
    let e_obj = LeanObj::new(e).unwrap();
    let objs = e_obj.ctor_obj_ptr();
    let f = *objs;
    if f.is_null() || lean_ptr::is_scalar_ptr(f) {
        return None;
    }
    if layout::header(f).tag != EXPR_APP_TAG {
        return None;
    }
    let f_obj = LeanObj::new(f).unwrap();
    let f_objs = f_obj.ctor_obj_ptr();
    let g = *f_objs;
    if !expr_is_const_name(g, name) {
        return None;
    }
    Some(*f_objs.add(1))
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
    if o.is_null() {
        return 0;
    }
    if lean_ptr::is_scalar_ptr(o) {
        return 0;
    }
    unsafe {
        let header = layout::header(o);
        if header.tag != EXPR_LAM_TAG && header.tag != EXPR_FORALL_TAG {
            return 0;
        }
        let obj = LeanObj::new(o).unwrap();
        obj.ctor_scalar_get_u8(EXPR_DATA_BYTES) as c_uchar
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_is_have_rs(o: *mut LeanObject) -> c_uchar {
    if o.is_null() {
        return 0;
    }
    if lean_ptr::is_scalar_ptr(o) {
        return 0;
    }
    unsafe {
        let header = layout::header(o);
        if header.tag != EXPR_LET_TAG {
            return 0;
        }
        let obj = LeanObj::new(o).unwrap();
        obj.ctor_scalar_get_u8(EXPR_DATA_BYTES) as c_uchar
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_consume_type_annotations_rs(mut e: *mut LeanObject) -> *mut LeanObject {
    if e.is_null() {
        return e;
    }
    let names = type_annotation_names();
    let opt_param = names.opt_param as *mut LeanObject;
    let auto_param = names.auto_param as *mut LeanObject;
    let out_param = names.out_param as *mut LeanObject;
    let semi_out_param = names.semi_out_param as *mut LeanObject;
    loop {
        unsafe {
            if let Some(arg) = match_app2_const_arg1(e, opt_param) {
                lean_inc(arg);
                lean_dec(e);
                e = arg;
                continue;
            }
            if let Some(arg) = match_app2_const_arg1(e, auto_param) {
                lean_inc(arg);
                lean_dec(e);
                e = arg;
                continue;
            }
            if let Some(arg) = match_app1_const_arg(e, out_param) {
                lean_inc(arg);
                lean_dec(e);
                e = arg;
                continue;
            }
            if let Some(arg) = match_app1_const_arg(e, semi_out_param) {
                lean_inc(arg);
                lean_dec(e);
                e = arg;
                continue;
            }
            return e;
        }
    }
}

#[no_mangle]
pub extern "C" fn lean_expr_has_loose_bvar_rs(e: *mut LeanObject, i: *mut LeanObject) -> c_uchar {
    if e.is_null() || !lean_ptr::is_scalar_ptr(i) {
        return 0;
    }
    let i_val = lean_ptr::unbox_ptr(i);
    if i_val > u32::MAX as usize {
        return 0;
    }
    let target = i_val as u32;
    unsafe {
        if expr_loose_bvar_range(e) == 0 {
            return 0;
        }
        let mut stack: Vec<(*mut LeanObject, u32)> = Vec::new();
        let mut cache: HashSet<(usize, u32)> = HashSet::new();
        stack.push((e, 0));
        while let Some((node, offset)) = stack.pop() {
            if node.is_null() || lean_ptr::is_scalar_ptr(node) {
                continue;
            }
            let n_i = match target.checked_add(offset) {
                Some(v) => v,
                None => continue,
            };
            let tag = layout::header(node).tag;
            if tag == EXPR_BVAR_TAG || tag == EXPR_CONST_TAG || tag == EXPR_SORT_TAG {
                let range = expr_loose_bvar_range(node);
                if n_i >= range {
                    continue;
                }
                if tag == EXPR_BVAR_TAG {
                    let obj = LeanObj::new(node).unwrap();
                    let idx_obj = *obj.ctor_obj_ptr();
                    if lean_ptr::is_scalar_ptr(idx_obj) {
                        let idx = lean_ptr::unbox_ptr(idx_obj);
                        if idx <= u32::MAX as usize && idx as u32 == n_i {
                            return 1;
                        }
                    }
                }
                continue;
            }
            if !is_likely_unshared(node) {
                let key = (node as usize, offset);
                if !cache.insert(key) {
                    continue;
                }
            }
            let range = expr_loose_bvar_range(node);
            if n_i >= range {
                continue;
            }
            match tag {
                EXPR_MDATA_TAG => {
                    let obj = LeanObj::new(node).unwrap();
                    let expr = *obj.ctor_obj_ptr().add(1);
                    stack.push((expr, offset));
                }
                EXPR_PROJ_TAG => {
                    let obj = LeanObj::new(node).unwrap();
                    let expr = *obj.ctor_obj_ptr().add(2);
                    stack.push((expr, offset));
                }
                EXPR_APP_TAG => {
                    let obj = LeanObj::new(node).unwrap();
                    let objs = obj.ctor_obj_ptr();
                    let f = *objs;
                    let a = *objs.add(1);
                    stack.push((a, offset));
                    stack.push((f, offset));
                }
                EXPR_LAM_TAG | EXPR_FORALL_TAG => {
                    let obj = LeanObj::new(node).unwrap();
                    let objs = obj.ctor_obj_ptr();
                    let domain = *objs.add(1);
                    let body = *objs.add(2);
                    stack.push((body, offset + 1));
                    stack.push((domain, offset));
                }
                EXPR_LET_TAG => {
                    let obj = LeanObj::new(node).unwrap();
                    let objs = obj.ctor_obj_ptr();
                    let ty = *objs.add(1);
                    let val = *objs.add(2);
                    let body = *objs.add(3);
                    stack.push((body, offset + 1));
                    stack.push((val, offset));
                    stack.push((ty, offset));
                }
                EXPR_FVAR_TAG | EXPR_MVAR_TAG | EXPR_LIT_TAG => {}
                _ => {}
            }
        }
    }
    0
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
