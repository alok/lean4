use crate::ffi;
use crate::LeanObject;
use libc::c_uint;
use std::marker::PhantomData;
use std::mem;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct LeanObj<'a> {
    ptr: *mut LeanObject,
    _marker: PhantomData<&'a LeanObject>,
}

impl<'a> LeanObj<'a> {
    #[inline(always)]
    pub unsafe fn new(ptr: *mut LeanObject) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self {
                ptr,
                _marker: PhantomData,
            })
        }
    }

    #[inline(always)]
    pub fn is_scalar(self) -> bool {
        unsafe { ffi::lean_rs_is_scalar(self.ptr) != 0 }
    }

    #[inline(always)]
    pub fn ctor_num_objs(self) -> usize {
        unsafe { ffi::lean_rs_ctor_num_objs(self.ptr) as usize }
    }

    #[inline(always)]
    pub fn ctor_data_u64(self) -> u64 {
        let num_objs = self.ctor_num_objs();
        let offset = (num_objs * mem::size_of::<*mut LeanObject>()) as c_uint;
        unsafe { ffi::lean_rs_ctor_get_uint64(self.ptr, offset) }
    }
}
