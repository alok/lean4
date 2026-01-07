use crate::LeanObject;
use crate::object::LeanObj;
use crate::environment::Environment;
use crate::local_ctx::{LocalCtx, BinderInfo};
use crate::declaration::{DefinitionSafety, ConstantInfo, ConstantInfoKind};
use crate::ptr;
use std::collections::HashMap;

pub struct TypeChecker<'a> {
    pub env: Environment<'a>,
    pub lctx: LocalCtx<'a>,
    pub safety: DefinitionSafety,
    pub infer_cache: [HashMap<*mut LeanObject, *mut LeanObject>; 2],
    pub whnf_core_cache: HashMap<*mut LeanObject, *mut LeanObject>,
    pub whnf_cache: HashMap<*mut LeanObject, *mut LeanObject>,
}

#[allow(dead_code)]
mod ffi {
    use crate::LeanObject;
    extern "C" {
        pub fn lean_expr_get_kind(e: *mut LeanObject) -> u8;
        pub fn lean_expr_get_app_fn(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_app_arg(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_binding_domain(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_binding_body(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_let_value(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_let_body(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_const_name(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_const_levels(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_fvar_name(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_sort_level(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_lit_value(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_proj_idx(e: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_get_proj_expr(e: *mut LeanObject) -> *mut LeanObject;
        
        pub fn lean_mk_sort(l: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_level_mk_succ(l: *mut LeanObject) -> *mut LeanObject;
        
        pub fn lean_expr_instantiate1(e: *mut LeanObject, subst: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_expr_instantiate_rev(e: *mut LeanObject, n: usize, subst: *const *mut LeanObject) -> *mut LeanObject;
    }
}

// Expression kinds (tags) from lib.rs/layout.rs
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

impl<'a> TypeChecker<'a> {
    pub fn new(env: Environment<'a>, lctx: LocalCtx<'a>, safety: DefinitionSafety) -> Self {
        Self {
            env,
            lctx,
            safety,
            infer_cache: [HashMap::new(), HashMap::new()],
            whnf_core_cache: HashMap::new(),
            whnf_cache: HashMap::new(),
        }
    }

    pub unsafe fn infer_type(&mut self, e: *mut LeanObject) -> *mut LeanObject {
        self.infer_type_core(e, true)
    }

    pub unsafe fn infer_type_core(&mut self, e: *mut LeanObject, infer_only: bool) -> *mut LeanObject {
        let idx = if infer_only { 1 } else { 0 };
        if let Some(&r) = self.infer_cache[idx].get(&e) {
            crate::lean_inc(r);
            return r;
        }

        let kind = ffi::lean_expr_get_kind(e);
        let r = match kind {
            EXPR_LIT_TAG => {
                // Simplified: lit_type implementation needed
                std::ptr::null_mut()
            }
            EXPR_MDATA_TAG => {
                let obj = LeanObj::new(e).unwrap();
                let inner = *obj.ctor_obj_ptr().add(1);
                self.infer_type_core(inner, infer_only)
            }
            EXPR_FVAR_TAG => {
                let name = ffi::lean_expr_get_fvar_name(e);
                if let Some(decl) = self.lctx.find(name) {
                    let t = decl.type_();
                    crate::lean_inc(t);
                    t
                } else {
                    panic!("unknown free variable");
                }
            }
            EXPR_SORT_TAG => {
                let level = ffi::lean_expr_get_sort_level(e);
                let succ = ffi::lean_level_mk_succ(level);
                ffi::lean_mk_sort(succ)
            }
            EXPR_CONST_TAG => {
                let name = ffi::lean_expr_get_const_name(e);
                if let Some(info) = self.env.find(name) {
                    let t = info.obj.ctor_obj_ptr().add(2); // type is field 2 of ConstantVal
                    let t_ptr = *t;
                    // Need to handle universe instantiation
                    crate::lean_inc(t_ptr);
                    t_ptr
                } else {
                    panic!("unknown constant");
                }
            }
            _ => {
                // Placeholder for other kinds
                std::ptr::null_mut()
            }
        };

        if !r.is_null() {
            crate::lean_inc(r);
            self.infer_cache[idx].insert(e, r);
        }
        r
    }

    pub unsafe fn whnf_core(&mut self, e: *mut LeanObject) -> *mut LeanObject {
        let kind = ffi::lean_expr_get_kind(e);
        match kind {
            EXPR_BVAR_TAG | EXPR_SORT_TAG | EXPR_MVAR_TAG | EXPR_PI_TAG | 
            EXPR_CONST_TAG | EXPR_LAM_TAG | EXPR_LIT_TAG => {
                crate::lean_inc(e);
                return e;
            }
            EXPR_MDATA_TAG => {
                let obj = LeanObj::new(e).unwrap();
                let inner = *obj.ctor_obj_ptr().add(1);
                return self.whnf_core(inner);
            }
            _ => {}
        }

        // Cache and logic implementation follows...
        crate::lean_inc(e);
        e
    }
}

// Add missing tags if needed
const EXPR_PI_TAG: u8 = EXPR_FORALL_TAG;