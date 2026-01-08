use crate::LeanObject;
use crate::object::LeanObj;
use crate::ptr;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BinderInfo {
    Default = 0,
    Implicit = 1,
    StrictImplicit = 2,
    InstImplicit = 3,
    Rec = 4,
}

pub struct LocalDecl<'a> {
    pub obj: LeanObj<'a>,
}

impl<'a> LocalDecl<'a> {
    pub fn index(&self) -> usize {
        unsafe { ptr::unbox_ptr(*self.obj.ctor_obj_ptr()) }
    }
    pub fn name(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(1) }
    }
    pub fn user_name(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(2) }
    }
    pub fn type_(&self) -> *mut LeanObject {
        unsafe { *self.obj.ctor_obj_ptr().add(3) }
    }
    pub fn value(&self) -> Option<*mut LeanObject> {
        unsafe {
            if crate::layout::header(self.obj.ptr).tag == 0 {
                None
            } else {
                Some(*self.obj.ctor_obj_ptr().add(4))
            }
        }
    }
    pub fn binder_info(&self) -> Option<BinderInfo> {
        unsafe {
            if crate::layout::header(self.obj.ptr).tag == 0 {
                Some(std::mem::transmute(self.obj.ctor_scalar_get_u8(0)))
            } else {
                None
            }
        }
    }
}

pub struct LocalCtx<'a> {
    pub obj: LeanObj<'a>,
}

#[allow(dead_code)]
mod ffi {
    use crate::LeanObject;
    extern "C" {
        pub fn lean_local_ctx_find(lctx: *mut LeanObject, name: *mut LeanObject) -> *mut LeanObject;
    }
}

impl<'a> LocalCtx<'a> {
    pub unsafe fn find(&self, n: *mut LeanObject) -> Option<LocalDecl<'a>> {
        let lctx = self.obj.ptr;
        crate::lean_inc(lctx);
        crate::lean_inc(n);
        let r = ffi::lean_local_ctx_find(lctx, n);
        if ptr::is_scalar_ptr(r) {
            None
        } else {
            let r_obj = LeanObj::new(r).unwrap();
            let decl_ptr = *r_obj.ctor_obj_ptr();
            Some(LocalDecl { obj: LeanObj::new(decl_ptr).unwrap() })
        }
    }
}
