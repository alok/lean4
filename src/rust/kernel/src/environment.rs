use crate::LeanObject;
use crate::object::LeanObj;
use crate::declaration::ConstantInfo;
use crate::ptr;

pub struct Environment<'a> {
    pub obj: LeanObj<'a>,
}

#[allow(dead_code)]
mod ffi {
    use crate::LeanObject;
    extern "C" {
        pub fn lean_environment_add(env: *mut LeanObject, info: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_environment_find(env: *mut LeanObject, name: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_environment_mark_quot_init(env: *mut LeanObject) -> *mut LeanObject;
        pub fn lean_environment_quot_init(env: *mut LeanObject) -> u8;
    }
}

impl<'a> Environment<'a> {
    pub unsafe fn find(&self, n: *mut LeanObject) -> Option<ConstantInfo<'a>> {
        let env = self.obj.ptr;
        crate::lean_inc(env);
        crate::lean_inc(n);
        let r = ffi::lean_environment_find(env, n);
        if ptr::is_scalar_ptr(r) {
            None
        } else {
            let r_obj = LeanObj::new(r).unwrap();
            let info_ptr = *r_obj.ctor_obj_ptr();
            Some(ConstantInfo { obj: LeanObj::new(info_ptr).unwrap() })
        }
    }

    pub unsafe fn is_quot_initialized(&self) -> bool {
        let env = self.obj.ptr;
        crate::lean_inc(env);
        ffi::lean_environment_quot_init(env) != 0
    }
}
