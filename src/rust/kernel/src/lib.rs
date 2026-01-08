#![allow(clippy::missing_safety_doc)]

mod bitfield;
mod declaration;
mod environment;
mod layout;
mod local_ctx;
mod object;
mod ptr;
mod type_checker;

use libc::{c_char, c_uchar};
use static_assertions::const_assert;
use std::collections::{HashMap, HashSet};
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
        pub fn lean_big_usize_to_nat(n: usize) -> *mut LeanObject;
        pub fn lean_nat_big_add(a1: *mut LeanObject, a2: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_nat_big_sub(a1: *mut LeanObject, a2: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_mk_bvar(idx: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_mk_app(f: *mut LeanObject, a: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_mk_lambda(
            n: *mut LeanObject,
            t: *mut LeanObject,
            b: *mut LeanObject,
            bi: c_uchar,
        ) -> *mut LeanObject;
        pub fn lean_expr_mk_forall(
            n: *mut LeanObject,
            t: *mut LeanObject,
            b: *mut LeanObject,
            bi: c_uchar,
        ) -> *mut LeanObject;
        pub fn lean_expr_mk_let(
            n: *mut LeanObject,
            t: *mut LeanObject,
            v: *mut LeanObject,
            b: *mut LeanObject,
            nondep: c_uchar,
        ) -> *mut LeanObject;
        pub fn lean_expr_mk_mdata(m: *mut LeanObject, e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_mk_proj(
            n: *mut LeanObject,
            idx: *mut LeanObject,
            e: *mut LeanObject,
        ) -> *mut LeanObject;
        pub fn lean_expr_mk_sort(l: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_mk_const(n: *mut LeanObject, ls: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_apply_1(f: *mut LeanObject, a: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_alloc_ctor_export(tag: u32, num_objs: u32, scalar_sz: u32) -> *mut LeanObject;
        pub fn lean_ctor_set_export(o: *mut LeanObject, idx: u32, v: *mut LeanObject);
        pub fn lean_mk_empty_array_with_capacity(capacity: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_array_push(a: *mut LeanObject, v: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_list_cons(head: *mut LeanObject, tail: *mut LeanObject) -> *mut LeanObject;
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
const MAX_SMALL_NAT: usize = usize::MAX >> 1;
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

struct ExprCache {
    map: HashMap<(usize, u32), *mut LeanObject>,
}

impl ExprCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    unsafe fn get(&self, key: (usize, u32)) -> Option<*mut LeanObject> {
        self.map.get(&key).copied()
    }

    unsafe fn insert(&mut self, key: (usize, u32), value: *mut LeanObject) {
        use std::collections::hash_map::Entry;
        match self.map.entry(key) {
            Entry::Vacant(v) => {
                lean_inc(value);
                v.insert(value);
            }
            Entry::Occupied(_) => {}
        }
    }
}

impl Drop for ExprCache {
    fn drop(&mut self) {
        for &value in self.map.values() {
            unsafe {
                lean_dec(value);
            }
        }
    }
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
unsafe fn is_shared(o: *mut LeanObject) -> bool {
    if lean_ptr::is_scalar_ptr(o) {
        return false;
    }
    layout::header(o).rc > 1
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
unsafe fn nat_box(n: usize) -> *mut LeanObject {
    (lean_ptr::box_bits(n)) as *mut LeanObject
}

#[inline(always)]
unsafe fn nat_add(a: *mut LeanObject, b: *mut LeanObject) -> *mut LeanObject {
    if lean_ptr::is_scalar_ptr(a) && lean_ptr::is_scalar_ptr(b) {
        let a_val = lean_ptr::unbox_ptr(a);
        let b_val = lean_ptr::unbox_ptr(b);
        let sum = a_val + b_val;
        if sum <= MAX_SMALL_NAT {
            nat_box(sum)
        } else {
            ffi::lean_big_usize_to_nat(sum)
        }
    } else {
        ffi::lean_nat_big_add(a, b)
    }
}

#[inline(always)]
unsafe fn nat_sub(a: *mut LeanObject, b: *mut LeanObject) -> *mut LeanObject {
    if lean_ptr::is_scalar_ptr(a) && lean_ptr::is_scalar_ptr(b) {
        let a_val = lean_ptr::unbox_ptr(a);
        let b_val = lean_ptr::unbox_ptr(b);
        if a_val < b_val {
            nat_box(0)
        } else {
            nat_box(a_val - b_val)
        }
    } else {
        ffi::lean_nat_big_sub(a, b)
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

unsafe fn lower_loose_bvars_go(
    e: *mut LeanObject,
    s: u32,
    d: u32,
    offset: u32,
    cache: &mut ExprCache,
) -> *mut LeanObject {
    if e.is_null() || lean_ptr::is_scalar_ptr(e) {
        return e;
    }
    let key = (e as usize, offset);
    if !is_likely_unshared(e) {
        if let Some(cached) = cache.get(key) {
            lean_inc(cached);
            return cached;
        }
    }
    let s1 = s.wrapping_add(offset);
    if s1 < s {
        lean_inc(e);
        if !is_likely_unshared(e) {
            cache.insert(key, e);
        }
        return e;
    }
    let range = expr_loose_bvar_range(e);
    if s1 >= range {
        lean_inc(e);
        if !is_likely_unshared(e) {
            cache.insert(key, e);
        }
        return e;
    }
    let tag = layout::header(e).tag;
    let result = match tag {
        EXPR_BVAR_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let idx_obj = *obj.ctor_obj_ptr();
            if lean_ptr::is_scalar_ptr(idx_obj) {
                let idx_val = lean_ptr::unbox_ptr(idx_obj);
                if idx_val >= s1 as usize {
                    let new_idx = idx_val - d as usize;
                    let new_idx_obj = nat_box(new_idx);
                    ffi::lean_expr_mk_bvar(new_idx_obj)
                } else {
                    lean_inc(e);
                    e
                }
            } else {
                let d_obj = nat_box(d as usize);
                let new_idx_obj = nat_sub(idx_obj, d_obj);
                ffi::lean_expr_mk_bvar(new_idx_obj)
            }
        }
        EXPR_APP_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let objs = obj.ctor_obj_ptr();
            let f = *objs;
            let a = *objs.add(1);
            let f_new = lower_loose_bvars_go(f, s, d, offset, cache);
            let a_new = lower_loose_bvars_go(a, s, d, offset, cache);
            if f_new == f && a_new == a {
                lean_dec(f_new);
                lean_dec(a_new);
                lean_inc(e);
                e
            } else {
                ffi::lean_expr_mk_app(f_new, a_new)
            }
        }
        EXPR_LAM_TAG | EXPR_FORALL_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let objs = obj.ctor_obj_ptr();
            let name = *objs;
            let domain = *objs.add(1);
            let body = *objs.add(2);
            let domain_new = lower_loose_bvars_go(domain, s, d, offset, cache);
            let body_new = lower_loose_bvars_go(body, s, d, offset.wrapping_add(1), cache);
            if domain_new == domain && body_new == body {
                lean_dec(domain_new);
                lean_dec(body_new);
                lean_inc(e);
                e
            } else {
                lean_inc(name);
                let bi = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                if tag == EXPR_LAM_TAG {
                    ffi::lean_expr_mk_lambda(name, domain_new, body_new, bi)
                } else {
                    ffi::lean_expr_mk_forall(name, domain_new, body_new, bi)
                }
            }
        }
        EXPR_LET_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let objs = obj.ctor_obj_ptr();
            let name = *objs;
            let ty = *objs.add(1);
            let val = *objs.add(2);
            let body = *objs.add(3);
            let ty_new = lower_loose_bvars_go(ty, s, d, offset, cache);
            let val_new = lower_loose_bvars_go(val, s, d, offset, cache);
            let body_new = lower_loose_bvars_go(body, s, d, offset.wrapping_add(1), cache);
            if ty_new == ty && val_new == val && body_new == body {
                lean_dec(ty_new);
                lean_dec(val_new);
                lean_dec(body_new);
                lean_inc(e);
                e
            } else {
                lean_inc(name);
                let nondep = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                ffi::lean_expr_mk_let(name, ty_new, val_new, body_new, nondep)
            }
        }
        EXPR_MDATA_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let objs = obj.ctor_obj_ptr();
            let mdata = *objs;
            let expr = *objs.add(1);
            let expr_new = lower_loose_bvars_go(expr, s, d, offset, cache);
            if expr_new == expr {
                lean_dec(expr_new);
                lean_inc(e);
                e
            } else {
                lean_inc(mdata);
                ffi::lean_expr_mk_mdata(mdata, expr_new)
            }
        }
        EXPR_PROJ_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let objs = obj.ctor_obj_ptr();
            let name = *objs;
            let idx = *objs.add(1);
            let expr = *objs.add(2);
            let expr_new = lower_loose_bvars_go(expr, s, d, offset, cache);
            if expr_new == expr {
                lean_dec(expr_new);
                lean_inc(e);
                e
            } else {
                lean_inc(name);
                lean_inc(idx);
                ffi::lean_expr_mk_proj(name, idx, expr_new)
            }
        }
        _ => {
            lean_inc(e);
            e
        }
    };
    if !is_likely_unshared(e) {
        cache.insert(key, result);
    }
    result
}

#[no_mangle]
pub extern "C" fn lean_expr_lower_loose_bvars_rs(
    e: *mut LeanObject,
    s: *mut LeanObject,
    d: *mut LeanObject,
) -> *mut LeanObject {
    if e.is_null() {
        return e;
    }
    if !lean_ptr::is_scalar_ptr(s) || !lean_ptr::is_scalar_ptr(d) {
        unsafe { lean_inc(e) };
        return e;
    }
    let s_val = lean_ptr::unbox_ptr(s);
    let d_val = lean_ptr::unbox_ptr(d);
    if s_val < d_val {
        unsafe { lean_inc(e) };
        return e;
    }
    let s_u = s_val as u32;
    let d_u = d_val as u32;
    if d_u == 0 {
        unsafe { lean_inc(e) };
        return e;
    }
    unsafe {
        if s_u >= expr_loose_bvar_range(e) {
            lean_inc(e);
            return e;
        }
        let mut cache = ExprCache::new();
        lower_loose_bvars_go(e, s_u, d_u, 0, &mut cache)
    }
}

unsafe fn lift_loose_bvars_go(
    e: *mut LeanObject,
    s: u32,
    d: u32,
    offset: u32,
    cache: &mut ExprCache,
) -> *mut LeanObject {
    if e.is_null() || lean_ptr::is_scalar_ptr(e) {
        return e;
    }
    let key = (e as usize, offset);
    if !is_likely_unshared(e) {
        if let Some(cached) = cache.get(key) {
            lean_inc(cached);
            return cached;
        }
    }
    let s1 = s.wrapping_add(offset);
    if s1 < s {
        lean_inc(e);
        if !is_likely_unshared(e) {
            cache.insert(key, e);
        }
        return e;
    }
    let range = expr_loose_bvar_range(e);
    if s1 >= range {
        lean_inc(e);
        if !is_likely_unshared(e) {
            cache.insert(key, e);
        }
        return e;
    }
    let tag = layout::header(e).tag;
    let result = match tag {
        EXPR_BVAR_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let idx_obj = *obj.ctor_obj_ptr();
            if lean_ptr::is_scalar_ptr(idx_obj) {
                let idx_val = lean_ptr::unbox_ptr(idx_obj);
                if idx_val >= s1 as usize {
                    let d_obj = nat_box(d as usize);
                    let idx_obj_box = nat_box(idx_val);
                    let new_idx_obj = nat_add(idx_obj_box, d_obj);
                    ffi::lean_expr_mk_bvar(new_idx_obj)
                } else {
                    lean_inc(e);
                    e
                }
            } else {
                let d_obj = nat_box(d as usize);
                let new_idx_obj = nat_add(idx_obj, d_obj);
                ffi::lean_expr_mk_bvar(new_idx_obj)
            }
        }
        EXPR_APP_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let objs = obj.ctor_obj_ptr();
            let f = *objs;
            let a = *objs.add(1);
            let f_new = lift_loose_bvars_go(f, s, d, offset, cache);
            let a_new = lift_loose_bvars_go(a, s, d, offset, cache);
            if f_new == f && a_new == a {
                lean_dec(f_new);
                lean_dec(a_new);
                lean_inc(e);
                e
            } else {
                ffi::lean_expr_mk_app(f_new, a_new)
            }
        }
        EXPR_LAM_TAG | EXPR_FORALL_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let objs = obj.ctor_obj_ptr();
            let name = *objs;
            let domain = *objs.add(1);
            let body = *objs.add(2);
            let domain_new = lift_loose_bvars_go(domain, s, d, offset, cache);
            let body_new = lift_loose_bvars_go(body, s, d, offset.wrapping_add(1), cache);
            if domain_new == domain && body_new == body {
                lean_dec(domain_new);
                lean_dec(body_new);
                lean_inc(e);
                e
            } else {
                lean_inc(name);
                let bi = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                if tag == EXPR_LAM_TAG {
                    ffi::lean_expr_mk_lambda(name, domain_new, body_new, bi)
                } else {
                    ffi::lean_expr_mk_forall(name, domain_new, body_new, bi)
                }
            }
        }
        EXPR_LET_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let objs = obj.ctor_obj_ptr();
            let name = *objs;
            let ty = *objs.add(1);
            let val = *objs.add(2);
            let body = *objs.add(3);
            let ty_new = lift_loose_bvars_go(ty, s, d, offset, cache);
            let val_new = lift_loose_bvars_go(val, s, d, offset, cache);
            let body_new = lift_loose_bvars_go(body, s, d, offset.wrapping_add(1), cache);
            if ty_new == ty && val_new == val && body_new == body {
                lean_dec(ty_new);
                lean_dec(val_new);
                lean_dec(body_new);
                lean_inc(e);
                e
            } else {
                lean_inc(name);
                let nondep = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                ffi::lean_expr_mk_let(name, ty_new, val_new, body_new, nondep)
            }
        }
        EXPR_MDATA_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let objs = obj.ctor_obj_ptr();
            let mdata = *objs;
            let expr = *objs.add(1);
            let expr_new = lift_loose_bvars_go(expr, s, d, offset, cache);
            if expr_new == expr {
                lean_dec(expr_new);
                lean_inc(e);
                e
            } else {
                lean_inc(mdata);
                ffi::lean_expr_mk_mdata(mdata, expr_new)
            }
        }
        EXPR_PROJ_TAG => {
            let obj = LeanObj::new(e).unwrap();
            let objs = obj.ctor_obj_ptr();
            let name = *objs;
            let idx = *objs.add(1);
            let expr = *objs.add(2);
            let expr_new = lift_loose_bvars_go(expr, s, d, offset, cache);
            if expr_new == expr {
                lean_dec(expr_new);
                lean_inc(e);
                e
            } else {
                lean_inc(name);
                lean_inc(idx);
                ffi::lean_expr_mk_proj(name, idx, expr_new)
            }
        }
        _ => {
            lean_inc(e);
            e
        }
    };
    if !is_likely_unshared(e) {
        cache.insert(key, result);
    }
    result
}

#[no_mangle]
pub extern "C" fn lean_expr_lift_loose_bvars_rs(
    e: *mut LeanObject,
    s: *mut LeanObject,
    d: *mut LeanObject,
) -> *mut LeanObject {
    if e.is_null() {
        return e;
    }
    if !lean_ptr::is_scalar_ptr(s) || !lean_ptr::is_scalar_ptr(d) {
        unsafe { lean_inc(e) };
        return e;
    }
    let s_val = lean_ptr::unbox_ptr(s);
    let d_val = lean_ptr::unbox_ptr(d);
    let s_u = s_val as u32;
    let d_u = d_val as u32;
    if d_u == 0 {
        unsafe { lean_inc(e) };
        return e;
    }
    unsafe {
        if s_u >= expr_loose_bvar_range(e) {
            lean_inc(e);
            return e;
        }
        let mut cache = ExprCache::new();
        lift_loose_bvars_go(e, s_u, d_u, 0, &mut cache)
    }
}

#[no_mangle]
pub extern "C" fn lean_find_expr_rs(p: *mut LeanObject, e: *mut LeanObject) -> *mut LeanObject {
    unsafe {
        if e.is_null() {
            return nat_box(0);
        }
        let mut found: *mut LeanObject = std::ptr::null_mut();
        let mut cache: HashSet<*mut LeanObject> = HashSet::new();
        fn visit(
            p: *mut LeanObject,
            e: *mut LeanObject,
            found: &mut *mut LeanObject,
            cache: &mut HashSet<*mut LeanObject>,
        ) {
            unsafe {
                if found.is_null() == false {
                    return;
                }
                if e.is_null() || lean_ptr::is_scalar_ptr(e) {
                    return;
                }
                let tag = layout::header(e).tag;
                match tag {
                    EXPR_CONST_TAG | EXPR_BVAR_TAG | EXPR_SORT_TAG => {
                        lean_inc(p);
                        lean_inc(e);
                        let r = ffi::lean_apply_1(p, e);
                        if lean_ptr::unbox_ptr(r) != 0 {
                            *found = e;
                        }
                        return;
                    }
                    _ => {}
                }
                if !is_likely_unshared(e) {
                    if cache.contains(&e) {
                        return;
                    }
                    cache.insert(e);
                }
                lean_inc(p);
                lean_inc(e);
                let r = ffi::lean_apply_1(p, e);
                if lean_ptr::unbox_ptr(r) != 0 {
                    *found = e;
                    return;
                }
                match tag {
                    EXPR_LIT_TAG | EXPR_MVAR_TAG | EXPR_FVAR_TAG => {}
                    EXPR_MDATA_TAG => {
                        let obj = LeanObj::new(e).unwrap();
                        let expr = *obj.ctor_obj_ptr().add(1);
                        visit(p, expr, found, cache);
                    }
                    EXPR_PROJ_TAG => {
                        let obj = LeanObj::new(e).unwrap();
                        let expr = *obj.ctor_obj_ptr().add(2);
                        visit(p, expr, found, cache);
                    }
                    EXPR_APP_TAG => {
                        let obj = LeanObj::new(e).unwrap();
                        let objs = obj.ctor_obj_ptr();
                        let f = *objs;
                        let a = *objs.add(1);
                        visit(p, f, found, cache);
                        visit(p, a, found, cache);
                    }
                    EXPR_LAM_TAG | EXPR_FORALL_TAG => {
                        let obj = LeanObj::new(e).unwrap();
                        let objs = obj.ctor_obj_ptr();
                        let domain = *objs.add(1);
                        let body = *objs.add(2);
                        visit(p, domain, found, cache);
                        visit(p, body, found, cache);
                    }
                    EXPR_LET_TAG => {
                        let obj = LeanObj::new(e).unwrap();
                        let objs = obj.ctor_obj_ptr();
                        let ty = *objs.add(1);
                        let val = *objs.add(2);
                        let body = *objs.add(3);
                        visit(p, ty, found, cache);
                        visit(p, val, found, cache);
                        visit(p, body, found, cache);
                    }
                    _ => {}
                }
            }
        }
        visit(p, e, &mut found, &mut cache);
        if found.is_null() {
            nat_box(0)
        } else {
            lean_inc(found);
            let r = ffi::lean_alloc_ctor_export(1, 1, 0);
            ffi::lean_ctor_set_export(r, 0, found);
            r
        }
    }
}

#[no_mangle]
pub extern "C" fn lean_find_ext_expr_rs(p: *mut LeanObject, e: *mut LeanObject) -> *mut LeanObject {
    unsafe {
        if e.is_null() {
            return nat_box(0);
        }
        let mut found: *mut LeanObject = std::ptr::null_mut();
        let mut cache: HashSet<*mut LeanObject> = HashSet::new();
        fn visit_app_fn(
            p: *mut LeanObject,
            e: *mut LeanObject,
            found: &mut *mut LeanObject,
            cache: &mut HashSet<*mut LeanObject>,
        ) {
            unsafe {
                if found.is_null() == false {
                    return;
                }
                if e.is_null() || lean_ptr::is_scalar_ptr(e) {
                    return;
                }
                if layout::header(e).tag == EXPR_APP_TAG {
                    let obj = LeanObj::new(e).unwrap();
                    let objs = obj.ctor_obj_ptr();
                    let f = *objs;
                    let a = *objs.add(1);
                    visit_app_fn(p, f, found, cache);
                    visit(p, a, found, cache);
                } else {
                    visit(p, e, found, cache);
                }
            }
        }
        fn visit(
            p: *mut LeanObject,
            e: *mut LeanObject,
            found: &mut *mut LeanObject,
            cache: &mut HashSet<*mut LeanObject>,
        ) {
            unsafe {
                if found.is_null() == false {
                    return;
                }
                if e.is_null() || lean_ptr::is_scalar_ptr(e) {
                    return;
                }
                let tag = layout::header(e).tag;
                match tag {
                    EXPR_CONST_TAG | EXPR_BVAR_TAG | EXPR_SORT_TAG => {
                        lean_inc(p);
                        lean_inc(e);
                        let r = ffi::lean_apply_1(p, e);
                        match lean_ptr::unbox_ptr(r) {
                            0 => {
                                *found = e;
                                return;
                            }
                            1 => {}
                            2 => return,
                            _ => return,
                        }
                        return;
                    }
                    _ => {}
                }
                if !is_likely_unshared(e) {
                    if cache.contains(&e) {
                        return;
                    }
                    cache.insert(e);
                }
                lean_inc(p);
                lean_inc(e);
                let r = ffi::lean_apply_1(p, e);
                match lean_ptr::unbox_ptr(r) {
                    0 => {
                        *found = e;
                        return;
                    }
                    1 => {}
                    2 => return,
                    _ => return,
                }
                match tag {
                    EXPR_LIT_TAG | EXPR_MVAR_TAG | EXPR_FVAR_TAG => {}
                    EXPR_MDATA_TAG => {
                        let obj = LeanObj::new(e).unwrap();
                        let expr = *obj.ctor_obj_ptr().add(1);
                        visit(p, expr, found, cache);
                    }
                    EXPR_PROJ_TAG => {
                        let obj = LeanObj::new(e).unwrap();
                        let expr = *obj.ctor_obj_ptr().add(2);
                        visit(p, expr, found, cache);
                    }
                    EXPR_APP_TAG => {
                        let obj = LeanObj::new(e).unwrap();
                        let objs = obj.ctor_obj_ptr();
                        let f = *objs;
                        let a = *objs.add(1);
                        visit_app_fn(p, f, found, cache);
                        visit(p, a, found, cache);
                    }
                    EXPR_LAM_TAG | EXPR_FORALL_TAG => {
                        let obj = LeanObj::new(e).unwrap();
                        let objs = obj.ctor_obj_ptr();
                        let domain = *objs.add(1);
                        let body = *objs.add(2);
                        visit(p, domain, found, cache);
                        visit(p, body, found, cache);
                    }
                    EXPR_LET_TAG => {
                        let obj = LeanObj::new(e).unwrap();
                        let objs = obj.ctor_obj_ptr();
                        let ty = *objs.add(1);
                        let val = *objs.add(2);
                        let body = *objs.add(3);
                        visit(p, ty, found, cache);
                        visit(p, val, found, cache);
                        visit(p, body, found, cache);
                    }
                    _ => {}
                }
            }
        }
        visit(p, e, &mut found, &mut cache);
        if found.is_null() {
            nat_box(0)
        } else {
            lean_inc(found);
            let r = ffi::lean_alloc_ctor_export(1, 1, 0);
            ffi::lean_ctor_set_export(r, 0, found);
            r
        }
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

// Expression equality tags for literals
const LIT_NAT_TAG: u8 = 0;
const LIT_STRING_TAG: u8 = 1;

// FFI for equality comparisons
// Note: lean_nat_eq, lean_string_eq, lean_kvmap_eq are inline functions in C++ headers
// so we use _ffi suffixed wrappers from rust_ffi.cpp
#[allow(dead_code)]
mod eq_ffi {
    use super::LeanObject;
    use libc::c_uchar;

    extern "C" {
        pub fn lean_name_eq(n1: *mut LeanObject, n2: *mut LeanObject) -> c_uchar;
        pub fn lean_level_eq(l1: *mut LeanObject, l2: *mut LeanObject) -> c_uchar;
        // FFI wrappers from rust_ffi.cpp for inline functions
        #[link_name = "lean_nat_eq_ffi"]
        pub fn lean_nat_eq(n1: *mut LeanObject, n2: *mut LeanObject) -> c_uchar;
        #[link_name = "lean_string_eq_ffi"]
        pub fn lean_string_eq(s1: *mut LeanObject, s2: *mut LeanObject) -> c_uchar;
        #[link_name = "lean_kvmap_eq_ffi"]
        pub fn lean_kvmap_eq(m1: *mut LeanObject, m2: *mut LeanObject) -> c_uchar;
    }
}

/// Compare two levels lists for equality
unsafe fn levels_eq(ls1: *mut LeanObject, ls2: *mut LeanObject) -> bool {
    let mut l1 = ls1;
    let mut l2 = ls2;
    loop {
        let is_nil1 = lean_ptr::is_scalar_ptr(l1);
        let is_nil2 = lean_ptr::is_scalar_ptr(l2);
        if is_nil1 && is_nil2 {
            return true;
        }
        if is_nil1 || is_nil2 {
            return false;
        }
        // Both are cons cells
        let obj1 = LeanObj::new(l1).unwrap();
        let obj2 = LeanObj::new(l2).unwrap();
        let head1 = *obj1.ctor_obj_ptr();
        let head2 = *obj2.ctor_obj_ptr();
        if eq_ffi::lean_level_eq(head1, head2) == 0 {
            return false;
        }
        l1 = *obj1.ctor_obj_ptr().add(1);
        l2 = *obj2.ctor_obj_ptr().add(1);
    }
}

/// Compare two literals for equality
unsafe fn literal_eq(a: *mut LeanObject, b: *mut LeanObject) -> bool {
    let obj_a = LeanObj::new(a).unwrap();
    let obj_b = LeanObj::new(b).unwrap();
    let tag_a = layout::header(a).tag;
    let tag_b = layout::header(b).tag;
    if tag_a != tag_b {
        return false;
    }
    let val_a = *obj_a.ctor_obj_ptr();
    let val_b = *obj_b.ctor_obj_ptr();
    match tag_a {
        LIT_NAT_TAG => eq_ffi::lean_nat_eq(val_a, val_b) != 0,
        LIT_STRING_TAG => eq_ffi::lean_string_eq(val_a, val_b) != 0,
        _ => false,
    }
}

/// Expression equality implementation
/// When compare_binder_info is true, also compares binder names and info for lambda/pi/let
struct ExprEq {
    cache: HashSet<(*mut LeanObject, *mut LeanObject)>,
    compare_binder_info: bool,
}

impl ExprEq {
    fn new(compare_binder_info: bool) -> Self {
        Self {
            cache: HashSet::new(),
            compare_binder_info,
        }
    }

    unsafe fn check_cache(&mut self, a: *mut LeanObject, b: *mut LeanObject) -> bool {
        if !is_shared(a) || !is_shared(b) {
            return false;
        }
        let key = (a, b);
        if self.cache.contains(&key) {
            return true;
        }
        self.cache.insert(key);
        false
    }

    unsafe fn apply(&mut self, a: *mut LeanObject, b: *mut LeanObject) -> bool {
        // Same pointer = equal
        if a == b {
            return true;
        }

        // Handle null/scalar
        if a.is_null() || b.is_null() {
            return false;
        }
        let is_scalar_a = lean_ptr::is_scalar_ptr(a);
        let is_scalar_b = lean_ptr::is_scalar_ptr(b);
        if is_scalar_a || is_scalar_b {
            return is_scalar_a && is_scalar_b && a == b;
        }

        // Get data for hash comparison
        let data_a = data_for(a);
        let data_b = data_for(b);

        // Hash mismatch = not equal
        if let (Some(da), Some(db)) = (data_a, data_b) {
            if ExprData(da).hash() != ExprData(db).hash() {
                return false;
            }
        }

        // Tag (kind) must match
        let tag_a = layout::header(a).tag;
        let tag_b = layout::header(b).tag;
        if tag_a != tag_b {
            return false;
        }

        let obj_a = LeanObj::new(a).unwrap();
        let obj_b = LeanObj::new(b).unwrap();

        // Handle atomic cases first
        match tag_a {
            EXPR_BVAR_TAG => {
                let idx_a = *obj_a.ctor_obj_ptr();
                let idx_b = *obj_b.ctor_obj_ptr();
                return eq_ffi::lean_nat_eq(idx_a, idx_b) != 0;
            }
            EXPR_LIT_TAG => {
                let lit_a = *obj_a.ctor_obj_ptr();
                let lit_b = *obj_b.ctor_obj_ptr();
                return literal_eq(lit_a, lit_b);
            }
            EXPR_MVAR_TAG | EXPR_FVAR_TAG => {
                let name_a = *obj_a.ctor_obj_ptr();
                let name_b = *obj_b.ctor_obj_ptr();
                return eq_ffi::lean_name_eq(name_a, name_b) != 0;
            }
            EXPR_SORT_TAG => {
                let level_a = *obj_a.ctor_obj_ptr();
                let level_b = *obj_b.ctor_obj_ptr();
                return eq_ffi::lean_level_eq(level_a, level_b) != 0;
            }
            _ => {}
        }

        // Check cache for compound expressions
        if self.check_cache(a, b) {
            return true;
        }

        match tag_a {
            EXPR_MDATA_TAG => {
                let expr_a = *obj_a.ctor_obj_ptr().add(1);
                let expr_b = *obj_b.ctor_obj_ptr().add(1);
                if !self.apply(expr_a, expr_b) {
                    return false;
                }
                let mdata_a = *obj_a.ctor_obj_ptr();
                let mdata_b = *obj_b.ctor_obj_ptr();
                eq_ffi::lean_kvmap_eq(mdata_a, mdata_b) != 0
            }
            EXPR_PROJ_TAG => {
                let expr_a = *obj_a.ctor_obj_ptr().add(2);
                let expr_b = *obj_b.ctor_obj_ptr().add(2);
                if !self.apply(expr_a, expr_b) {
                    return false;
                }
                let name_a = *obj_a.ctor_obj_ptr();
                let name_b = *obj_b.ctor_obj_ptr();
                if eq_ffi::lean_name_eq(name_a, name_b) == 0 {
                    return false;
                }
                let idx_a = *obj_a.ctor_obj_ptr().add(1);
                let idx_b = *obj_b.ctor_obj_ptr().add(1);
                eq_ffi::lean_nat_eq(idx_a, idx_b) != 0
            }
            EXPR_CONST_TAG => {
                let name_a = *obj_a.ctor_obj_ptr();
                let name_b = *obj_b.ctor_obj_ptr();
                if eq_ffi::lean_name_eq(name_a, name_b) == 0 {
                    return false;
                }
                let levels_a = *obj_a.ctor_obj_ptr().add(1);
                let levels_b = *obj_b.ctor_obj_ptr().add(1);
                levels_eq(levels_a, levels_b)
            }
            EXPR_APP_TAG => {
                // Compare args first, then function (often more efficient)
                let arg_a = *obj_a.ctor_obj_ptr().add(1);
                let arg_b = *obj_b.ctor_obj_ptr().add(1);
                if !self.apply(arg_a, arg_b) {
                    return false;
                }
                let fn_a = *obj_a.ctor_obj_ptr();
                let fn_b = *obj_b.ctor_obj_ptr();
                self.apply(fn_a, fn_b)
            }
            EXPR_LAM_TAG | EXPR_FORALL_TAG => {
                let domain_a = *obj_a.ctor_obj_ptr().add(1);
                let domain_b = *obj_b.ctor_obj_ptr().add(1);
                if !self.apply(domain_a, domain_b) {
                    return false;
                }
                let body_a = *obj_a.ctor_obj_ptr().add(2);
                let body_b = *obj_b.ctor_obj_ptr().add(2);
                if !self.apply(body_a, body_b) {
                    return false;
                }
                if self.compare_binder_info {
                    let name_a = *obj_a.ctor_obj_ptr();
                    let name_b = *obj_b.ctor_obj_ptr();
                    if eq_ffi::lean_name_eq(name_a, name_b) == 0 {
                        return false;
                    }
                    let bi_a = obj_a.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                    let bi_b = obj_b.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                    if bi_a != bi_b {
                        return false;
                    }
                }
                true
            }
            EXPR_LET_TAG => {
                let ty_a = *obj_a.ctor_obj_ptr().add(1);
                let ty_b = *obj_b.ctor_obj_ptr().add(1);
                if !self.apply(ty_a, ty_b) {
                    return false;
                }
                let val_a = *obj_a.ctor_obj_ptr().add(2);
                let val_b = *obj_b.ctor_obj_ptr().add(2);
                if !self.apply(val_a, val_b) {
                    return false;
                }
                let body_a = *obj_a.ctor_obj_ptr().add(3);
                let body_b = *obj_b.ctor_obj_ptr().add(3);
                if !self.apply(body_a, body_b) {
                    return false;
                }
                // Compare nondep flag
                let nondep_a = obj_a.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                let nondep_b = obj_b.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                if nondep_a != nondep_b {
                    return false;
                }
                if self.compare_binder_info {
                    let name_a = *obj_a.ctor_obj_ptr();
                    let name_b = *obj_b.ctor_obj_ptr();
                    if eq_ffi::lean_name_eq(name_a, name_b) == 0 {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}

/// Expression equality without binder info comparison (lean_expr_eqv)
#[no_mangle]
pub extern "C" fn lean_expr_eqv_rs(a: *mut LeanObject, b: *mut LeanObject) -> c_uchar {
    unsafe {
        let mut eq = ExprEq::new(false);
        if eq.apply(a, b) { 1 } else { 0 }
    }
}

/// Expression equality with binder info comparison (lean_expr_equal)
#[no_mangle]
pub extern "C" fn lean_expr_equal_rs(a: *mut LeanObject, b: *mut LeanObject) -> c_uchar {
    unsafe {
        let mut eq = ExprEq::new(true);
        if eq.apply(a, b) { 1 } else { 0 }
    }
}

// ==========================================================================
// Expression replacement (replace_fn.cpp)
// ==========================================================================

/// Expression replacement implementation
/// Applies a callback function `f: Expr → Option Expr` to each subexpression.
/// If `f(e)` returns `Some(new_e)`, uses `new_e`; otherwise recurses into children.
struct ReplaceExpr {
    /// Cache for shared subexpressions: (original_ptr) -> result_ptr
    cache: HashMap<*mut LeanObject, *mut LeanObject>,
    /// The callback function (borrowed reference, we inc_ref on each call)
    callback: *mut LeanObject,
}

impl ReplaceExpr {
    fn new(callback: *mut LeanObject) -> Self {
        Self {
            cache: HashMap::new(),
            callback,
        }
    }

    /// Apply replacement to expression, returning the (possibly new) result.
    /// The returned pointer has its own reference count.
    unsafe fn apply(&mut self, e: *mut LeanObject) -> *mut LeanObject {
        // Check if e is shared and already in cache
        let shared = is_shared(e);
        if shared {
            if let Some(&cached) = self.cache.get(&e) {
                // Return cached result with incremented refcount
                lean_inc(cached);
                return cached;
            }
        }

        // Call the callback function: f(e)
        // We need to inc both f and e since lean_apply_1 consumes them
        lean_inc(self.callback);
        lean_inc(e);
        let result = ffi::lean_apply_1(self.callback, e);

        // Check if result is Some(new_e) or None
        // Option is: None = scalar 0, Some(x) = ctor 1 with field 0 = x
        if !lean_ptr::is_scalar_ptr(result) {
            // Result is Some(new_e)
            let obj = LeanObj::new(result).unwrap();
            let new_e = *obj.ctor_obj_ptr();
            lean_inc(new_e);
            // Drop the Option wrapper
            lean_dec(result);

            // Cache the result if shared
            if shared {
                lean_inc(new_e);
                self.cache.insert(e, new_e);
            }
            return new_e;
        }
        // result is None (scalar), so we recurse into children

        let obj = match LeanObj::new(e) {
            Some(o) => o,
            None => {
                // e is scalar (shouldn't happen for exprs, but be safe)
                lean_inc(e);
                return e;
            }
        };

        let tag = layout::header(e).tag;
        let result = match tag {
            // Leaf expressions - just return the original
            EXPR_BVAR_TAG | EXPR_FVAR_TAG | EXPR_MVAR_TAG |
            EXPR_SORT_TAG | EXPR_CONST_TAG | EXPR_LIT_TAG => {
                lean_inc(e);
                e
            }

            EXPR_APP_TAG => {
                let fn_ptr = *obj.ctor_obj_ptr();
                let arg_ptr = *obj.ctor_obj_ptr().add(1);

                let new_fn = self.apply(fn_ptr);
                let new_arg = self.apply(arg_ptr);

                // Check if anything changed (pointer equality)
                if new_fn == fn_ptr && new_arg == arg_ptr {
                    // Nothing changed, return original
                    lean_dec(new_fn);
                    lean_dec(new_arg);
                    lean_inc(e);
                    e
                } else {
                    // Create new app expression (consumes new_fn and new_arg)
                    ffi::lean_expr_mk_app(new_fn, new_arg)
                }
            }

            EXPR_LAM_TAG | EXPR_FORALL_TAG => {
                let name = *obj.ctor_obj_ptr();
                let domain = *obj.ctor_obj_ptr().add(1);
                let body = *obj.ctor_obj_ptr().add(2);
                let bi = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);

                let new_domain = self.apply(domain);
                let new_body = self.apply(body);

                if new_domain == domain && new_body == body {
                    lean_dec(new_domain);
                    lean_dec(new_body);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    if tag == EXPR_LAM_TAG {
                        ffi::lean_expr_mk_lambda(name, new_domain, new_body, bi)
                    } else {
                        ffi::lean_expr_mk_forall(name, new_domain, new_body, bi)
                    }
                }
            }

            EXPR_LET_TAG => {
                let name = *obj.ctor_obj_ptr();
                let ty = *obj.ctor_obj_ptr().add(1);
                let val = *obj.ctor_obj_ptr().add(2);
                let body = *obj.ctor_obj_ptr().add(3);
                let nondep = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);

                let new_ty = self.apply(ty);
                let new_val = self.apply(val);
                let new_body = self.apply(body);

                if new_ty == ty && new_val == val && new_body == body {
                    lean_dec(new_ty);
                    lean_dec(new_val);
                    lean_dec(new_body);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    ffi::lean_expr_mk_let(name, new_ty, new_val, new_body, nondep)
                }
            }

            EXPR_MDATA_TAG => {
                let mdata = *obj.ctor_obj_ptr();
                let expr = *obj.ctor_obj_ptr().add(1);

                let new_expr = self.apply(expr);

                if new_expr == expr {
                    lean_dec(new_expr);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(mdata);
                    ffi::lean_expr_mk_mdata(mdata, new_expr)
                }
            }

            EXPR_PROJ_TAG => {
                let name = *obj.ctor_obj_ptr();
                let idx = *obj.ctor_obj_ptr().add(1);
                let expr = *obj.ctor_obj_ptr().add(2);

                let new_expr = self.apply(expr);

                if new_expr == expr {
                    lean_dec(new_expr);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    lean_inc(idx);
                    ffi::lean_expr_mk_proj(name, idx, new_expr)
                }
            }

            _ => {
                // Unknown expression kind, return as-is
                lean_inc(e);
                e
            }
        };

        // Cache result if shared
        if shared {
            lean_inc(result);
            self.cache.insert(e, result);
        }

        result
    }
}

/// Replace expressions using a callback function
/// f: Expr → Option Expr
/// Returns a new expression with the callback applied
#[no_mangle]
pub extern "C" fn lean_replace_expr_rs(
    f: *mut LeanObject,
    e: *mut LeanObject,
) -> *mut LeanObject {
    unsafe {
        let mut replacer = ReplaceExpr::new(f);
        replacer.apply(e)
    }
}

// ==========================================================================
// Expression abstraction (abstract.cpp)
// ==========================================================================

// FFI for array access
mod array_ffi {
    use super::LeanObject;
    use libc::c_ulong;

    extern "C" {
        #[link_name = "lean_array_size_ffi"]
        pub fn lean_array_size(a: *mut LeanObject) -> c_ulong;
        #[link_name = "lean_array_get_core_ffi"]
        pub fn lean_array_get_core(a: *mut LeanObject, i: c_ulong) -> *mut LeanObject;
        #[link_name = "lean_array_cptr_ffi"]
        pub fn lean_array_cptr(a: *mut LeanObject) -> *mut *mut LeanObject;
    }
}

/// Check if expression has fvar flag
#[inline]
unsafe fn expr_has_fvar(e: *mut LeanObject) -> bool {
    match data_for(e) {
        None => false,
        Some(data) => ExprData(data).has_fvar() != 0,
    }
}

/// Check if expression has mvar flag (expr_mvar or level_mvar)
#[inline]
unsafe fn expr_has_mvar(e: *mut LeanObject) -> bool {
    match data_for(e) {
        None => false,
        Some(data) => ExprData(data).has_expr_mvar() != 0 || ExprData(data).has_level_mvar() != 0,
    }
}

/// Expression abstraction implementation
struct ExprAbstract {
    /// Cache for shared subexpressions: (original_ptr, offset) -> result_ptr
    cache: HashMap<(*mut LeanObject, usize), *mut LeanObject>,
    /// Array of fvar/mvar expressions to abstract
    subst: *mut LeanObject,
    /// Number of elements in subst to use
    n: usize,
}

impl ExprAbstract {
    fn new(subst: *mut LeanObject, n: usize) -> Self {
        Self {
            cache: HashMap::new(),
            subst,
            n,
        }
    }

    /// Apply abstraction to expression with given offset
    unsafe fn apply(&mut self, e: *mut LeanObject, offset: usize) -> *mut LeanObject {
        // Fast path: no fvar/mvar in expression
        if !expr_has_fvar(e) && !expr_has_mvar(e) {
            lean_inc(e);
            return e;
        }

        // Check cache for shared expressions
        let shared = is_shared(e);
        if shared {
            if let Some(&cached) = self.cache.get(&(e, offset)) {
                lean_inc(cached);
                return cached;
            }
        }

        let obj = match LeanObj::new(e) {
            Some(o) => o,
            None => {
                lean_inc(e);
                return e;
            }
        };

        let tag = layout::header(e).tag;
        let result = match tag {
            // Check if this is an fvar/mvar to abstract
            EXPR_FVAR_TAG | EXPR_MVAR_TAG => {
                let name = *obj.ctor_obj_ptr();
                // Search for matching variable in subst array
                let mut i = self.n;
                while i > 0 {
                    i -= 1;
                    let v = array_ffi::lean_array_get_core(self.subst, i as libc::c_ulong);
                    if !lean_ptr::is_scalar_ptr(v) {
                        let v_obj = LeanObj::new(v).unwrap();
                        let v_tag = layout::header(v).tag;
                        let v_name = *v_obj.ctor_obj_ptr();
                        // Match fvar with fvar, mvar with mvar
                        let matches = if tag == EXPR_FVAR_TAG && v_tag == EXPR_FVAR_TAG {
                            ffi::lean_name_eq(name, v_name) != 0
                        } else if tag == EXPR_MVAR_TAG && v_tag == EXPR_MVAR_TAG {
                            ffi::lean_name_eq(name, v_name) != 0
                        } else {
                            false
                        };
                        if matches {
                            // Create bvar with index: offset + n - i - 1
                            let bvar_idx = offset + self.n - i - 1;
                            let idx_obj = if bvar_idx <= MAX_SMALL_NAT {
                                nat_box(bvar_idx)
                            } else {
                                ffi::lean_big_usize_to_nat(bvar_idx)
                            };
                            return ffi::lean_expr_mk_bvar(idx_obj);
                        }
                    }
                }
                // No match found, return original
                lean_inc(e);
                e
            }

            // Leaf expressions without fvar/mvar - return as-is
            EXPR_BVAR_TAG | EXPR_SORT_TAG | EXPR_CONST_TAG | EXPR_LIT_TAG => {
                lean_inc(e);
                e
            }

            EXPR_APP_TAG => {
                let fn_ptr = *obj.ctor_obj_ptr();
                let arg_ptr = *obj.ctor_obj_ptr().add(1);

                let new_fn = self.apply(fn_ptr, offset);
                let new_arg = self.apply(arg_ptr, offset);

                if new_fn == fn_ptr && new_arg == arg_ptr {
                    lean_dec(new_fn);
                    lean_dec(new_arg);
                    lean_inc(e);
                    e
                } else {
                    ffi::lean_expr_mk_app(new_fn, new_arg)
                }
            }

            EXPR_LAM_TAG | EXPR_FORALL_TAG => {
                let name = *obj.ctor_obj_ptr();
                let domain = *obj.ctor_obj_ptr().add(1);
                let body = *obj.ctor_obj_ptr().add(2);
                let bi = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);

                let new_domain = self.apply(domain, offset);
                let new_body = self.apply(body, offset + 1);

                if new_domain == domain && new_body == body {
                    lean_dec(new_domain);
                    lean_dec(new_body);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    if tag == EXPR_LAM_TAG {
                        ffi::lean_expr_mk_lambda(name, new_domain, new_body, bi)
                    } else {
                        ffi::lean_expr_mk_forall(name, new_domain, new_body, bi)
                    }
                }
            }

            EXPR_LET_TAG => {
                let name = *obj.ctor_obj_ptr();
                let ty = *obj.ctor_obj_ptr().add(1);
                let val = *obj.ctor_obj_ptr().add(2);
                let body = *obj.ctor_obj_ptr().add(3);
                let nondep = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);

                let new_ty = self.apply(ty, offset);
                let new_val = self.apply(val, offset);
                let new_body = self.apply(body, offset + 1);

                if new_ty == ty && new_val == val && new_body == body {
                    lean_dec(new_ty);
                    lean_dec(new_val);
                    lean_dec(new_body);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    ffi::lean_expr_mk_let(name, new_ty, new_val, new_body, nondep)
                }
            }

            EXPR_MDATA_TAG => {
                let mdata = *obj.ctor_obj_ptr();
                let expr = *obj.ctor_obj_ptr().add(1);

                let new_expr = self.apply(expr, offset);

                if new_expr == expr {
                    lean_dec(new_expr);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(mdata);
                    ffi::lean_expr_mk_mdata(mdata, new_expr)
                }
            }

            EXPR_PROJ_TAG => {
                let name = *obj.ctor_obj_ptr();
                let idx = *obj.ctor_obj_ptr().add(1);
                let expr = *obj.ctor_obj_ptr().add(2);

                let new_expr = self.apply(expr, offset);

                if new_expr == expr {
                    lean_dec(new_expr);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    lean_inc(idx);
                    ffi::lean_expr_mk_proj(name, idx, new_expr)
                }
            }

            _ => {
                lean_inc(e);
                e
            }
        };

        // Cache result if shared
        if shared {
            lean_inc(result);
            self.cache.insert((e, offset), result);
        }

        result
    }
}

/// Abstract expression: replace fvars/mvars from subst array with bound variables
#[no_mangle]
pub extern "C" fn lean_expr_abstract_rs(
    e: *mut LeanObject,
    subst: *mut LeanObject,
) -> *mut LeanObject {
    unsafe {
        let n = array_ffi::lean_array_size(subst) as usize;
        let mut abs = ExprAbstract::new(subst, n);
        abs.apply(e, 0)
    }
}

/// Abstract expression with range: use first min(n, array_size) elements from subst
#[no_mangle]
pub extern "C" fn lean_expr_abstract_range_rs(
    e: *mut LeanObject,
    n_obj: *mut LeanObject,
    subst: *mut LeanObject,
) -> *mut LeanObject {
    unsafe {
        let array_size = array_ffi::lean_array_size(subst) as usize;
        let n = if lean_ptr::is_scalar_ptr(n_obj) {
            let n_val = lean_ptr::unbox_ptr(n_obj);
            n_val.min(array_size)
        } else {
            // n is big nat, use full array
            array_size
        };
        let mut abs = ExprAbstract::new(subst, n);
        abs.apply(e, 0)
    }
}

// ==========================================================================
// Expression instantiation (instantiate.cpp)
// ==========================================================================

/// Expression instantiation implementation
/// Replaces bound variables with expressions from a substitution array
struct ExprInstantiate<'a> {
    /// Cache for shared subexpressions: (original_ptr, offset) -> result_ptr
    cache: HashMap<(*mut LeanObject, usize), *mut LeanObject>,
    /// Pointer to substitution array elements
    subst: *const *mut LeanObject,
    /// Number of elements in substitution
    n: usize,
    /// Whether to reverse the substitution order
    reverse: bool,
    /// Phantom lifetime
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> ExprInstantiate<'a> {
    fn new(subst: *const *mut LeanObject, n: usize, reverse: bool) -> Self {
        Self {
            cache: HashMap::new(),
            subst,
            n,
            reverse,
            _marker: std::marker::PhantomData,
        }
    }

    /// Apply instantiation to expression with given offset
    unsafe fn apply(&mut self, e: *mut LeanObject, offset: usize) -> *mut LeanObject {
        // Fast path: check loose bvar range
        let range = expr_loose_bvar_range(e) as usize;
        if offset >= range {
            lean_inc(e);
            return e;
        }

        // Check cache for shared expressions
        let shared = is_shared(e);
        if shared {
            if let Some(&cached) = self.cache.get(&(e, offset)) {
                lean_inc(cached);
                return cached;
            }
        }

        let obj = match LeanObj::new(e) {
            Some(o) => o,
            None => {
                lean_inc(e);
                return e;
            }
        };

        let tag = layout::header(e).tag;
        let result = match tag {
            EXPR_BVAR_TAG => {
                let idx_obj = *obj.ctor_obj_ptr();
                if lean_ptr::is_scalar_ptr(idx_obj) {
                    let vidx = lean_ptr::unbox_ptr(idx_obj);
                    if vidx >= offset {
                        let h = offset.wrapping_add(self.n);
                        // Check for overflow or if vidx is in substitution range
                        if h < offset || vidx < h {
                            // Substitute with lifted expression
                            let subst_idx = if self.reverse {
                                self.n - (vidx - offset) - 1
                            } else {
                                vidx - offset
                            };
                            let v = *self.subst.add(subst_idx);
                            // Lift loose bvars in substitution by offset
                            if offset == 0 {
                                lean_inc(v);
                                v
                            } else {
                                let offset_obj = nat_box(offset);
                                lean_expr_lift_loose_bvars_rs(v, nat_box(0), offset_obj)
                            }
                        } else {
                            // Shift bvar index down by n
                            let new_idx = vidx - self.n;
                            ffi::lean_expr_mk_bvar(nat_box(new_idx))
                        }
                    } else {
                        lean_inc(e);
                        e
                    }
                } else {
                    // Big nat index - just return as is (rare case)
                    lean_inc(e);
                    e
                }
            }

            // Leaf expressions - return unchanged
            EXPR_FVAR_TAG | EXPR_MVAR_TAG | EXPR_SORT_TAG | EXPR_CONST_TAG | EXPR_LIT_TAG => {
                lean_inc(e);
                e
            }

            EXPR_APP_TAG => {
                let fn_ptr = *obj.ctor_obj_ptr();
                let arg_ptr = *obj.ctor_obj_ptr().add(1);

                let new_fn = self.apply(fn_ptr, offset);
                let new_arg = self.apply(arg_ptr, offset);

                if new_fn == fn_ptr && new_arg == arg_ptr {
                    lean_dec(new_fn);
                    lean_dec(new_arg);
                    lean_inc(e);
                    e
                } else {
                    ffi::lean_expr_mk_app(new_fn, new_arg)
                }
            }

            EXPR_LAM_TAG | EXPR_FORALL_TAG => {
                let name = *obj.ctor_obj_ptr();
                let domain = *obj.ctor_obj_ptr().add(1);
                let body = *obj.ctor_obj_ptr().add(2);
                let bi = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);

                let new_domain = self.apply(domain, offset);
                let new_body = self.apply(body, offset + 1);

                if new_domain == domain && new_body == body {
                    lean_dec(new_domain);
                    lean_dec(new_body);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    if tag == EXPR_LAM_TAG {
                        ffi::lean_expr_mk_lambda(name, new_domain, new_body, bi)
                    } else {
                        ffi::lean_expr_mk_forall(name, new_domain, new_body, bi)
                    }
                }
            }

            EXPR_LET_TAG => {
                let name = *obj.ctor_obj_ptr();
                let ty = *obj.ctor_obj_ptr().add(1);
                let val = *obj.ctor_obj_ptr().add(2);
                let body = *obj.ctor_obj_ptr().add(3);
                let nondep = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);

                let new_ty = self.apply(ty, offset);
                let new_val = self.apply(val, offset);
                let new_body = self.apply(body, offset + 1);

                if new_ty == ty && new_val == val && new_body == body {
                    lean_dec(new_ty);
                    lean_dec(new_val);
                    lean_dec(new_body);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    ffi::lean_expr_mk_let(name, new_ty, new_val, new_body, nondep)
                }
            }

            EXPR_MDATA_TAG => {
                let mdata = *obj.ctor_obj_ptr();
                let expr = *obj.ctor_obj_ptr().add(1);

                let new_expr = self.apply(expr, offset);

                if new_expr == expr {
                    lean_dec(new_expr);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(mdata);
                    ffi::lean_expr_mk_mdata(mdata, new_expr)
                }
            }

            EXPR_PROJ_TAG => {
                let name = *obj.ctor_obj_ptr();
                let idx = *obj.ctor_obj_ptr().add(1);
                let expr = *obj.ctor_obj_ptr().add(2);

                let new_expr = self.apply(expr, offset);

                if new_expr == expr {
                    lean_dec(new_expr);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    lean_inc(idx);
                    ffi::lean_expr_mk_proj(name, idx, new_expr)
                }
            }

            _ => {
                lean_inc(e);
                e
            }
        };

        // Cache result if shared
        if shared {
            lean_inc(result);
            self.cache.insert((e, offset), result);
        }

        result
    }
}

/// Check if expression has loose bound variables
#[inline]
unsafe fn expr_has_loose_bvars(e: *mut LeanObject) -> bool {
    expr_loose_bvar_range(e) > 0
}

/// Instantiate single bound variable (lean_expr_instantiate1)
#[no_mangle]
pub extern "C" fn lean_expr_instantiate1_rs(
    a: *mut LeanObject,
    e: *mut LeanObject,
) -> *mut LeanObject {
    unsafe {
        if !expr_has_loose_bvars(a) {
            lean_inc(a);
            return a;
        }
        let subst: [*mut LeanObject; 1] = [e];
        let mut inst = ExprInstantiate::new(subst.as_ptr(), 1, false);
        inst.apply(a, 0)
    }
}

/// Instantiate with array of expressions
#[no_mangle]
pub extern "C" fn lean_expr_instantiate_rs(
    a: *mut LeanObject,
    subst: *mut LeanObject,
) -> *mut LeanObject {
    unsafe {
        if !expr_has_loose_bvars(a) {
            lean_inc(a);
            return a;
        }
        let n = array_ffi::lean_array_size(subst) as usize;
        if n == 0 {
            lean_inc(a);
            return a;
        }
        let cptr = array_ffi::lean_array_cptr(subst);
        let mut inst = ExprInstantiate::new(cptr, n, false);
        inst.apply(a, 0)
    }
}

/// Instantiate with range from array
#[no_mangle]
pub extern "C" fn lean_expr_instantiate_range_rs(
    a: *mut LeanObject,
    begin_obj: *mut LeanObject,
    end_obj: *mut LeanObject,
    subst: *mut LeanObject,
) -> *mut LeanObject {
    unsafe {
        if !lean_ptr::is_scalar_ptr(begin_obj) || !lean_ptr::is_scalar_ptr(end_obj) {
            ffi::lean_internal_panic(b"invalid range for Expr.instantiateRange\0".as_ptr() as *const c_char);
        }
        let sz = array_ffi::lean_array_size(subst) as usize;
        let b = lean_ptr::unbox_ptr(begin_obj);
        let e = lean_ptr::unbox_ptr(end_obj);
        if b > e || e > sz {
            ffi::lean_internal_panic(b"invalid range for Expr.instantiateRange\0".as_ptr() as *const c_char);
        }
        if !expr_has_loose_bvars(a) || e == b {
            lean_inc(a);
            return a;
        }
        let cptr = array_ffi::lean_array_cptr(subst).add(b);
        let mut inst = ExprInstantiate::new(cptr, e - b, false);
        inst.apply(a, 0)
    }
}

/// Instantiate with array in reverse order
#[no_mangle]
pub extern "C" fn lean_expr_instantiate_rev_rs(
    a: *mut LeanObject,
    subst: *mut LeanObject,
) -> *mut LeanObject {
    unsafe {
        if !expr_has_loose_bvars(a) {
            lean_inc(a);
            return a;
        }
        let n = array_ffi::lean_array_size(subst) as usize;
        if n == 0 {
            lean_inc(a);
            return a;
        }
        let cptr = array_ffi::lean_array_cptr(subst);
        let mut inst = ExprInstantiate::new(cptr, n, true);
        inst.apply(a, 0)
    }
}

/// Instantiate with range from array in reverse order
#[no_mangle]
pub extern "C" fn lean_expr_instantiate_rev_range_rs(
    a: *mut LeanObject,
    begin_obj: *mut LeanObject,
    end_obj: *mut LeanObject,
    subst: *mut LeanObject,
) -> *mut LeanObject {
    unsafe {
        if !lean_ptr::is_scalar_ptr(begin_obj) || !lean_ptr::is_scalar_ptr(end_obj) {
            ffi::lean_internal_panic(b"invalid range for Expr.instantiateRevRange\0".as_ptr() as *const c_char);
        }
        let sz = array_ffi::lean_array_size(subst) as usize;
        let b = lean_ptr::unbox_ptr(begin_obj);
        let e = lean_ptr::unbox_ptr(end_obj);
        if b > e || e > sz {
            ffi::lean_internal_panic(b"invalid range for Expr.instantiateRevRange\0".as_ptr() as *const c_char);
        }
        if !expr_has_loose_bvars(a) || e == b {
            lean_inc(a);
            return a;
        }
        let cptr = array_ffi::lean_array_cptr(subst).add(b);
        let mut inst = ExprInstantiate::new(cptr, e - b, true);
        inst.apply(a, 0)
    }
}

// Placeholder symbol to prove the Rust kernel staticlib is wired in.
#[no_mangle]
pub extern "C" fn lean_kernel_rs_ping() -> c_uchar {
    1
}

// ==========================================================================
// Metavariable instantiation (instantiate_mvars.cpp)
// ==========================================================================

// FFI callbacks into Lean for metavariable context operations
mod mvar_ffi {
    use super::LeanObject;

    extern "C" {
        pub fn lean_get_lmvar_assignment(mctx: *mut LeanObject, mid: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_assign_lmvar(mctx: *mut LeanObject, mid: *mut LeanObject, val: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_get_mvar_assignment(mctx: *mut LeanObject, mid: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_assign_mvar(mctx: *mut LeanObject, mid: *mut LeanObject, val: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_get_delayed_mvar_assignment(mctx: *mut LeanObject, mid: *mut LeanObject) -> *mut LeanObject;
    }
}

// Level kind constants
const LEVEL_ZERO_TAG: u8 = 0;
const LEVEL_SUCC_TAG: u8 = 1;
const LEVEL_MAX_TAG: u8 = 2;
const LEVEL_IMAX_TAG: u8 = 3;
const LEVEL_PARAM_TAG: u8 = 4;
const LEVEL_MVAR_TAG: u8 = 5;

/// Level instantiation for metavariables
struct InstantiateLevelMVars {
    cache: HashMap<*mut LeanObject, *mut LeanObject>,
    saved: Vec<*mut LeanObject>,
}

impl InstantiateLevelMVars {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            saved: Vec::new(),
        }
    }

    /// Check if level has metavariables using the cached level data.
    #[inline(never)]
    unsafe fn level_has_mvar(l: *mut LeanObject) -> bool {
        if l.is_null() || lean_ptr::is_scalar_ptr(l) {
            return false;
        }
        let obj = match LeanObj::new(l) {
            Some(o) => o,
            None => return false,
        };
        let data = LevelData(obj.ctor_data_u64());
        data.has_mvar() != 0
    }

    /// Cache a result if shared
    #[inline]
    unsafe fn cache_result(&mut self, l: *mut LeanObject, r: *mut LeanObject, shared: bool) {
        if shared {
            lean_inc(r);
            self.cache.insert(l, r);
        }
    }

    /// Visit a level and instantiate metavariables
    unsafe fn visit(&mut self, mctx: &mut *mut LeanObject, l: *mut LeanObject) -> *mut LeanObject {
        if l.is_null() || lean_ptr::is_scalar_ptr(l) {
            lean_inc(l);
            return l;
        }
        self.visit_nonscalar(mctx, l)
    }

    #[inline(never)]
    unsafe fn visit_nonscalar(&mut self, mctx: &mut *mut LeanObject, l: *mut LeanObject) -> *mut LeanObject {
        if !Self::level_has_mvar(l) {
            lean_inc(l);
            return l;
        }

        let shared = is_shared(l);
        if shared {
            if let Some(&cached) = self.cache.get(&l) {
                lean_inc(cached);
                return cached;
            }
        }

        let obj = match LeanObj::new(l) {
            Some(o) => o,
            None => {
                lean_inc(l);
                return l;
            }
        };

        let tag = layout::header(l).tag;
        let result = match tag {
            LEVEL_SUCC_TAG => {
                let inner = *obj.ctor_obj_ptr();
                let new_inner = self.visit(mctx, inner);
                if new_inner == inner {
                    lean_dec(new_inner);
                    lean_inc(l);
                    l
                } else {
                    level_ffi::lean_level_mk_succ(new_inner)
                }
            }

            LEVEL_MAX_TAG | LEVEL_IMAX_TAG => {
                let lhs = *obj.ctor_obj_ptr();
                let rhs = *obj.ctor_obj_ptr().add(1);
                let new_lhs = self.visit(mctx, lhs);
                let new_rhs = self.visit(mctx, rhs);
                if new_lhs == lhs && new_rhs == rhs {
                    lean_dec(new_lhs);
                    lean_dec(new_rhs);
                    lean_inc(l);
                    l
                } else if tag == LEVEL_MAX_TAG {
                    level_ffi::lean_level_mk_max(new_lhs, new_rhs)
                } else {
                    level_ffi::lean_level_mk_imax(new_lhs, new_rhs)
                }
            }

            LEVEL_MVAR_TAG => {
                let mid = *obj.ctor_obj_ptr();
                lean_inc(mid);
                lean_inc(*mctx);
                let r = mvar_ffi::lean_get_lmvar_assignment(*mctx, mid);

                if lean_ptr::is_scalar_ptr(r) {
                    // None - not assigned
                    lean_inc(l);
                    l
                } else {
                    // Some(val) - extract from Option.some ctor
                    let r_obj = LeanObj::new(r).unwrap();
                    let a = *r_obj.ctor_obj_ptr();  // Field 0 of Option.some
                    lean_inc(a);
                    lean_dec(r);

                    if !Self::level_has_mvar(a) {
                        a
                    } else {
                        let a_new = self.visit(mctx, a);
                        if a != a_new {
                            // Save 'a' to prevent garbage collection
                            lean_inc(a);
                            self.saved.push(a);
                            // Assign the new value - inc a_new BEFORE the call since assign consumes it
                            lean_inc(mid);
                            lean_inc(a_new);
                            let old_mctx = *mctx;
                            *mctx = mvar_ffi::lean_assign_lmvar(old_mctx, mid, a_new);
                        }
                        lean_dec(a);
                        // a_new still has its refcount from visit
                        a_new
                    }
                }
            }

            // Zero and Param don't have mvars
            _ => {
                lean_inc(l);
                l
            }
        };

        self.cache_result(l, result, shared);
        result
    }
}

// FFI for level construction
mod level_ffi {
    use super::LeanObject;

    extern "C" {
        pub fn lean_level_mk_succ(l: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_level_mk_max(l1: *mut LeanObject, l2: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_level_mk_imax(l1: *mut LeanObject, l2: *mut LeanObject) -> *mut LeanObject;
    }
}

/// Instantiate level metavariables (lean_instantiate_level_mvars)
#[no_mangle]
pub extern "C" fn lean_instantiate_level_mvars_rs(
    mctx: *mut LeanObject,
    l: *mut LeanObject,
) -> *mut LeanObject {
    unsafe {
        let mut mctx_mut = mctx;
        let mut inst = InstantiateLevelMVars::new();
        let l_new = inst.visit(&mut mctx_mut, l);

        // Construct result pair (mctx, l_new)
        let result = ffi::lean_alloc_ctor_export(0, 2, 0);
        ffi::lean_ctor_set_export(result, 0, mctx_mut);
        ffi::lean_ctor_set_export(result, 1, l_new);
        result
    }
}

// ==========================================================================
// Expression metavariable instantiation
// ==========================================================================

/// Expression metavariable instantiation
/// This is more complex than level mvar instantiation because it handles:
/// - Regular mvar assignments
/// - Delayed mvar assignments with fvar substitution
/// - Beta reduction for assigned mvars applied to arguments
struct InstantiateExprMVars {
    /// Cache for shared subexpressions
    cache: HashMap<*mut LeanObject, *mut LeanObject>,
    /// Saved expressions to prevent GC (since cache may reference subterms)
    saved: Vec<*mut LeanObject>,
    /// Already normalized mvar names (to avoid infinite loops)
    already_normalized: HashSet<*mut LeanObject>,
    /// Level instantiation helper
    level_inst: InstantiateLevelMVars,
}

impl InstantiateExprMVars {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            saved: Vec::new(),
            already_normalized: HashSet::new(),
            level_inst: InstantiateLevelMVars::new(),
        }
    }

    /// Visit a level and instantiate metavariables
    #[inline]
    unsafe fn visit_level(&mut self, mctx: &mut *mut LeanObject, l: *mut LeanObject) -> *mut LeanObject {
        self.level_inst.visit(mctx, l)
    }

    /// Visit a list of levels and instantiate metavariables
    unsafe fn visit_levels(&mut self, mctx: &mut *mut LeanObject, ls: *mut LeanObject) -> *mut LeanObject {
        if lean_ptr::is_scalar_ptr(ls) {
            // Empty list
            return ls;
        }

        // Build new list by processing each element
        let mut result_vec: Vec<*mut LeanObject> = Vec::new();
        let mut curr = ls;
        let mut changed = false;

        while !lean_ptr::is_scalar_ptr(curr) {
            let obj = LeanObj::new(curr).unwrap();
            let head = *obj.ctor_obj_ptr();
            let tail = *obj.ctor_obj_ptr().add(1);

            let new_head = self.visit_level(mctx, head);
            if new_head != head {
                changed = true;
            }
            result_vec.push(new_head);
            curr = tail;
        }

        if !changed {
            // Return original, cleanup allocated heads
            for h in result_vec {
                lean_dec(h);
            }
            lean_inc(ls);
            return ls;
        }

        // Build new list from back to front
        let mut result = nat_box(0); // nil
        for h in result_vec.into_iter().rev() {
            result = ffi::lean_list_cons(h, result);
        }
        result
    }

    /// Get expression mvar assignment, visiting it if needed
    unsafe fn get_assignment(&mut self, mctx: &mut *mut LeanObject, mid: *mut LeanObject) -> Option<*mut LeanObject> {
        lean_inc(mid);
        lean_inc(*mctx);
        let r = mvar_ffi::lean_get_mvar_assignment(*mctx, mid);

        if lean_ptr::is_scalar_ptr(r) {
            // None - not assigned
            return None;
        }

        // Some(val) - extract the value from the ctor
        let r_obj = LeanObj::new(r).unwrap();
        let a = *r_obj.ctor_obj_ptr();  // Field 0 of Option.some
        lean_inc(a);
        lean_dec(r);

        // Check if already normalized or no mvars
        if !Self::expr_has_any_mvar(a) || self.already_normalized.contains(&mid) {
            return Some(a);
        }

        // Mark as being normalized
        lean_inc(mid);
        self.already_normalized.insert(mid);

        let a_new = self.visit(mctx, a);
        if a != a_new {
            // Save 'a' to prevent GC
            lean_inc(a);
            self.saved.push(a);
            // Assign the normalized value - assign consumes its args
            lean_inc(mid);
            lean_inc(a_new);  // One for assign
            let old_mctx = *mctx;
            *mctx = mvar_ffi::lean_assign_mvar(old_mctx, mid, a_new);
        }
        lean_dec(a);
        // a_new already has proper refcount from visit
        Some(a_new)
    }

    /// Check if expression has any metavariables (expr or level)
    #[inline]
    unsafe fn expr_has_any_mvar(e: *mut LeanObject) -> bool {
        match data_for(e) {
            None => false,
            Some(data) => {
                let d = ExprData(data);
                d.has_expr_mvar() != 0 || d.has_level_mvar() != 0
            }
        }
    }

    /// Visit standalone mvar
    unsafe fn visit_mvar(&mut self, mctx: &mut *mut LeanObject, e: *mut LeanObject) -> *mut LeanObject {
        let obj = LeanObj::new(e).unwrap();
        let mid = *obj.ctor_obj_ptr();

        match self.get_assignment(mctx, mid) {
            Some(r) => r,
            None => {
                lean_inc(e);
                e
            }
        }
    }

    /// Make application from function and reversed args
    unsafe fn mk_rev_app(f: *mut LeanObject, args: &[*mut LeanObject]) -> *mut LeanObject {
        let mut result = f;
        for i in (0..args.len()).rev() {
            lean_inc(args[i]);
            result = ffi::lean_expr_mk_app(result, args[i]);
        }
        result
    }

    /// Apply beta reduction
    /// f is a lambda, args are the arguments in reverse order
    unsafe fn apply_beta(f: *mut LeanObject, num_args: usize, args: &[*mut LeanObject]) -> *mut LeanObject {
        if num_args == 0 {
            lean_inc(f);
            return f;
        }
        Self::apply_beta_rec(f, 0, num_args, args)
    }

    /// Recursive beta reduction helper
    unsafe fn apply_beta_rec(e: *mut LeanObject, i: usize, num_args: usize, args: &[*mut LeanObject]) -> *mut LeanObject {
        let tag = layout::header(e).tag;

        if tag == EXPR_LAM_TAG {
            let obj = LeanObj::new(e).unwrap();
            let body = *obj.ctor_obj_ptr().add(2);
            if i + 1 < num_args {
                return Self::apply_beta_rec(body, i + 1, num_args, args);
            } else {
                // Instantiate body with all args
                return Self::instantiate_n(body, num_args, args);
            }
        }

        if tag == EXPR_LET_TAG && i < num_args {
            // zeta reduce: substitute let value into body
            let obj = LeanObj::new(e).unwrap();
            let val = *obj.ctor_obj_ptr().add(2);
            let body = *obj.ctor_obj_ptr().add(3);
            let expanded = lean_expr_instantiate1_rs(body, val);
            let result = Self::apply_beta_rec(expanded, i, num_args, args);
            lean_dec(expanded);
            return result;
        }

        // Not a lambda or let - instantiate and apply remaining args
        let n = num_args - i;
        let instantiated = Self::instantiate_n(e, i, &args[n..]);
        Self::mk_rev_app(instantiated, &args[..n])
    }

    /// Instantiate first n bvars with reversed args
    unsafe fn instantiate_n(e: *mut LeanObject, n: usize, rev_args: &[*mut LeanObject]) -> *mut LeanObject {
        if n == 0 {
            lean_inc(e);
            return e;
        }

        // Create array for instantiation
        let mut arr = ffi::lean_mk_empty_array_with_capacity(nat_box(n));
        for i in 0..n {
            lean_inc(rev_args[n - 1 - i]);
            arr = ffi::lean_array_push(arr, rev_args[n - 1 - i]);
        }

        let result = lean_expr_instantiate_rs(e, arr);
        lean_dec(arr);
        result
    }

    /// Visit application - handles mvar head specially
    unsafe fn visit_app(&mut self, mctx: &mut *mut LeanObject, e: *mut LeanObject) -> *mut LeanObject {
        // Get the head function
        let mut curr = e;
        let mut args: Vec<*mut LeanObject> = Vec::new();

        // Collect args in reverse order (innermost first)
        while layout::header(curr).tag == EXPR_APP_TAG {
            let obj = LeanObj::new(curr).unwrap();
            let arg = *obj.ctor_obj_ptr().add(1);
            args.push(arg);
            curr = *obj.ctor_obj_ptr();
        }

        // curr is now the head function
        if layout::header(curr).tag != EXPR_MVAR_TAG {
            // Non-mvar head: just visit all subexpressions
            return self.visit_app_default(mctx, e);
        }

        // mvar head case
        let obj = LeanObj::new(curr).unwrap();
        let mid = *obj.ctor_obj_ptr();

        // Check for regular assignment first
        if let Some(f_new) = self.get_assignment(mctx, mid) {
            // Visit args and beta reduce
            let mut visited_args: Vec<*mut LeanObject> = Vec::new();
            for arg in &args {
                visited_args.push(self.visit(mctx, *arg));
            }
            let result = Self::apply_beta(f_new, visited_args.len(), &visited_args);
            lean_dec(f_new);
            for a in visited_args {
                lean_dec(a);
            }
            return result;
        }

        // Check for delayed assignment
        lean_inc(mid);
        lean_inc(*mctx);
        let d = mvar_ffi::lean_get_delayed_mvar_assignment(*mctx, mid);

        if lean_ptr::is_scalar_ptr(d) {
            // Not delayed assigned - visit args and reconstruct
            return self.visit_mvar_app_args(mctx, e, curr, &args);
        }

        // d is Some(DelayedAssignment)
        // DelayedAssignment is (fvars: Array Expr, mvarIdPending: Name)
        let dobj = LeanObj::new(d).unwrap();
        let delayed = *dobj.ctor_obj_ptr();
        lean_inc(delayed);
        lean_dec(d);

        let da_obj = LeanObj::new(delayed).unwrap();
        let fvars = *da_obj.ctor_obj_ptr();           // Array Expr
        let mid_pending = *da_obj.ctor_obj_ptr().add(1); // Name

        let fvars_size = array_ffi::lean_array_size(fvars) as usize;

        // Check if we have enough args
        if fvars_size > args.len() {
            // Not enough args - just visit mvar app args
            lean_dec(delayed);
            return self.visit_mvar_app_args(mctx, e, curr, &args);
        }

        // Get the pending mvar assignment
        if let Some(val) = self.get_assignment(mctx, mid_pending) {
            if !Self::expr_has_any_mvar(val) {
                // Apply delayed substitution
                let mut visited_args: Vec<*mut LeanObject> = Vec::new();
                for arg in &args {
                    visited_args.push(self.visit(mctx, *arg));
                }

                // Replace fvars in val with the last fvars_size args
                let val_subst = Self::replace_fvars(val, fvars, &visited_args[args.len() - fvars_size..]);
                lean_dec(val);

                // Apply remaining args
                let result = Self::mk_rev_app(val_subst, &visited_args[..args.len() - fvars_size]);

                for a in visited_args {
                    lean_dec(a);
                }
                lean_dec(delayed);
                return result;
            }
            lean_dec(val);
        }

        lean_dec(delayed);
        self.visit_mvar_app_args(mctx, e, curr, &args)
    }

    /// Replace fvars in expression with substitution values
    unsafe fn replace_fvars(e: *mut LeanObject, fvars: *mut LeanObject, rev_args: &[*mut LeanObject]) -> *mut LeanObject {
        let sz = array_ffi::lean_array_size(fvars) as usize;
        if sz == 0 {
            lean_inc(e);
            return e;
        }

        // Use replace function - create a callback that replaces fvars
        // For now, use a simpler approach: iterate and substitute one at a time
        let mut result = e;
        lean_inc(result);

        for i in 0..sz {
            let fvar = array_ffi::lean_array_get_core(fvars, i as libc::c_ulong);
            let subst_val = rev_args[sz - i - 1];

            // Create simple substitution
            let new_result = Self::subst_fvar(result, fvar, subst_val, 0);
            lean_dec(result);
            result = new_result;
        }

        result
    }

    /// Substitute a single fvar with a value, lifting bvars as needed
    unsafe fn subst_fvar(e: *mut LeanObject, fvar: *mut LeanObject, val: *mut LeanObject, offset: usize) -> *mut LeanObject {
        if !expr_has_fvar(e) {
            lean_inc(e);
            return e;
        }

        let obj = match LeanObj::new(e) {
            Some(o) => o,
            None => {
                lean_inc(e);
                return e;
            }
        };

        let tag = layout::header(e).tag;
        match tag {
            EXPR_FVAR_TAG => {
                let fid = *obj.ctor_obj_ptr();
                let fvar_obj = LeanObj::new(fvar).unwrap();
                let target_fid = *fvar_obj.ctor_obj_ptr();

                if ffi::lean_name_eq(fid, target_fid) != 0 {
                    // Match - return lifted value
                    if offset == 0 {
                        lean_inc(val);
                        val
                    } else {
                        lean_expr_lift_loose_bvars_rs(val, nat_box(0), nat_box(offset))
                    }
                } else {
                    lean_inc(e);
                    e
                }
            }

            EXPR_APP_TAG => {
                let fn_ptr = *obj.ctor_obj_ptr();
                let arg_ptr = *obj.ctor_obj_ptr().add(1);
                let new_fn = Self::subst_fvar(fn_ptr, fvar, val, offset);
                let new_arg = Self::subst_fvar(arg_ptr, fvar, val, offset);
                if new_fn == fn_ptr && new_arg == arg_ptr {
                    lean_dec(new_fn);
                    lean_dec(new_arg);
                    lean_inc(e);
                    e
                } else {
                    ffi::lean_expr_mk_app(new_fn, new_arg)
                }
            }

            EXPR_LAM_TAG | EXPR_FORALL_TAG => {
                let name = *obj.ctor_obj_ptr();
                let domain = *obj.ctor_obj_ptr().add(1);
                let body = *obj.ctor_obj_ptr().add(2);
                let bi = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                let new_domain = Self::subst_fvar(domain, fvar, val, offset);
                let new_body = Self::subst_fvar(body, fvar, val, offset + 1);
                if new_domain == domain && new_body == body {
                    lean_dec(new_domain);
                    lean_dec(new_body);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    if tag == EXPR_LAM_TAG {
                        ffi::lean_expr_mk_lambda(name, new_domain, new_body, bi)
                    } else {
                        ffi::lean_expr_mk_forall(name, new_domain, new_body, bi)
                    }
                }
            }

            EXPR_LET_TAG => {
                let name = *obj.ctor_obj_ptr();
                let ty = *obj.ctor_obj_ptr().add(1);
                let value = *obj.ctor_obj_ptr().add(2);
                let body = *obj.ctor_obj_ptr().add(3);
                let nondep = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                let new_ty = Self::subst_fvar(ty, fvar, val, offset);
                let new_val = Self::subst_fvar(value, fvar, val, offset);
                let new_body = Self::subst_fvar(body, fvar, val, offset + 1);
                if new_ty == ty && new_val == value && new_body == body {
                    lean_dec(new_ty);
                    lean_dec(new_val);
                    lean_dec(new_body);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    ffi::lean_expr_mk_let(name, new_ty, new_val, new_body, nondep)
                }
            }

            EXPR_MDATA_TAG => {
                let mdata = *obj.ctor_obj_ptr();
                let expr = *obj.ctor_obj_ptr().add(1);
                let new_expr = Self::subst_fvar(expr, fvar, val, offset);
                if new_expr == expr {
                    lean_dec(new_expr);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(mdata);
                    ffi::lean_expr_mk_mdata(mdata, new_expr)
                }
            }

            EXPR_PROJ_TAG => {
                let name = *obj.ctor_obj_ptr();
                let idx = *obj.ctor_obj_ptr().add(1);
                let expr = *obj.ctor_obj_ptr().add(2);
                let new_expr = Self::subst_fvar(expr, fvar, val, offset);
                if new_expr == expr {
                    lean_dec(new_expr);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    lean_inc(idx);
                    ffi::lean_expr_mk_proj(name, idx, new_expr)
                }
            }

            _ => {
                lean_inc(e);
                e
            }
        }
    }

    /// Visit app with mvar head - just visit args
    unsafe fn visit_mvar_app_args(&mut self, mctx: &mut *mut LeanObject, _e: *mut LeanObject, head: *mut LeanObject, args: &[*mut LeanObject]) -> *mut LeanObject {
        let mut visited_args: Vec<*mut LeanObject> = Vec::new();
        for arg in args {
            visited_args.push(self.visit(mctx, *arg));
        }
        lean_inc(head);
        let result = Self::mk_rev_app(head, &visited_args);
        for a in visited_args {
            lean_dec(a);
        }
        result
    }

    /// Visit application - default case (non-mvar head)
    unsafe fn visit_app_default(&mut self, mctx: &mut *mut LeanObject, e: *mut LeanObject) -> *mut LeanObject {
        let obj = LeanObj::new(e).unwrap();
        let fn_ptr = *obj.ctor_obj_ptr();
        let arg_ptr = *obj.ctor_obj_ptr().add(1);

        let new_fn = self.visit(mctx, fn_ptr);
        let new_arg = self.visit(mctx, arg_ptr);

        if new_fn == fn_ptr && new_arg == arg_ptr {
            lean_dec(new_fn);
            lean_dec(new_arg);
            lean_inc(e);
            e
        } else {
            ffi::lean_expr_mk_app(new_fn, new_arg)
        }
    }

    /// Main visit function
    unsafe fn visit(&mut self, mctx: &mut *mut LeanObject, e: *mut LeanObject) -> *mut LeanObject {
        if !Self::expr_has_any_mvar(e) {
            lean_inc(e);
            return e;
        }

        let shared = is_shared(e);
        if shared {
            if let Some(&cached) = self.cache.get(&e) {
                lean_inc(cached);
                return cached;
            }
        }

        let obj = match LeanObj::new(e) {
            Some(o) => o,
            None => {
                lean_inc(e);
                return e;
            }
        };

        let tag = layout::header(e).tag;
        let result = match tag {
            EXPR_BVAR_TAG | EXPR_LIT_TAG | EXPR_FVAR_TAG => {
                lean_inc(e);
                e
            }

            EXPR_SORT_TAG => {
                let level = *obj.ctor_obj_ptr();
                let new_level = self.visit_level(mctx, level);
                if new_level == level {
                    lean_dec(new_level);
                    lean_inc(e);
                    e
                } else {
                    ffi::lean_expr_mk_sort(new_level)
                }
            }

            EXPR_CONST_TAG => {
                let name = *obj.ctor_obj_ptr();
                let levels = *obj.ctor_obj_ptr().add(1);
                let new_levels = self.visit_levels(mctx, levels);
                if new_levels == levels {
                    lean_dec(new_levels);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    ffi::lean_expr_mk_const(name, new_levels)
                }
            }

            EXPR_MVAR_TAG => self.visit_mvar(mctx, e),

            EXPR_MDATA_TAG => {
                let mdata = *obj.ctor_obj_ptr();
                let expr = *obj.ctor_obj_ptr().add(1);
                let new_expr = self.visit(mctx, expr);
                if new_expr == expr {
                    lean_dec(new_expr);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(mdata);
                    ffi::lean_expr_mk_mdata(mdata, new_expr)
                }
            }

            EXPR_PROJ_TAG => {
                let name = *obj.ctor_obj_ptr();
                let idx = *obj.ctor_obj_ptr().add(1);
                let expr = *obj.ctor_obj_ptr().add(2);
                let new_expr = self.visit(mctx, expr);
                if new_expr == expr {
                    lean_dec(new_expr);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    lean_inc(idx);
                    ffi::lean_expr_mk_proj(name, idx, new_expr)
                }
            }

            EXPR_APP_TAG => self.visit_app(mctx, e),

            EXPR_LAM_TAG | EXPR_FORALL_TAG => {
                let name = *obj.ctor_obj_ptr();
                let domain = *obj.ctor_obj_ptr().add(1);
                let body = *obj.ctor_obj_ptr().add(2);
                let bi = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                let new_domain = self.visit(mctx, domain);
                let new_body = self.visit(mctx, body);
                if new_domain == domain && new_body == body {
                    lean_dec(new_domain);
                    lean_dec(new_body);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    if tag == EXPR_LAM_TAG {
                        ffi::lean_expr_mk_lambda(name, new_domain, new_body, bi)
                    } else {
                        ffi::lean_expr_mk_forall(name, new_domain, new_body, bi)
                    }
                }
            }

            EXPR_LET_TAG => {
                let name = *obj.ctor_obj_ptr();
                let ty = *obj.ctor_obj_ptr().add(1);
                let value = *obj.ctor_obj_ptr().add(2);
                let body = *obj.ctor_obj_ptr().add(3);
                let nondep = obj.ctor_scalar_get_u8(EXPR_DATA_BYTES);
                let new_ty = self.visit(mctx, ty);
                let new_val = self.visit(mctx, value);
                let new_body = self.visit(mctx, body);
                if new_ty == ty && new_val == value && new_body == body {
                    lean_dec(new_ty);
                    lean_dec(new_val);
                    lean_dec(new_body);
                    lean_inc(e);
                    e
                } else {
                    lean_inc(name);
                    ffi::lean_expr_mk_let(name, new_ty, new_val, new_body, nondep)
                }
            }

            _ => {
                lean_inc(e);
                e
            }
        };

        if shared {
            lean_inc(result);
            self.cache.insert(e, result);
        }

        result
    }
}

/// Instantiate expression metavariables
#[no_mangle]
pub extern "C" fn lean_instantiate_expr_mvars_rs(
    mctx: *mut LeanObject,
    e: *mut LeanObject,
) -> *mut LeanObject {
    unsafe {
        let mut mctx_mut = mctx;
        let mut inst = InstantiateExprMVars::new();
        let e_new = inst.visit(&mut mctx_mut, e);

        // Construct result pair (mctx, e_new)
        let result = ffi::lean_alloc_ctor_export(0, 2, 0);
        ffi::lean_ctor_set_export(result, 0, mctx_mut);
        ffi::lean_ctor_set_export(result, 1, e_new);
        result
    }
}
