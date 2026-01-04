use crate::ffi;
use crate::LeanObject;
use crate::ptr;
use libc::c_uint;
use std::marker::PhantomData;
use std::mem;
use std::slice;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct LeanObj<'a> {
    ptr: *mut LeanObject,
    _marker: PhantomData<&'a LeanObject>,
}

#[allow(dead_code)]
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
        ptr::is_scalar_ptr(self.ptr)
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

    #[inline(always)]
    pub fn ctor_obj_ptr(self) -> *mut *mut LeanObject {
        unsafe { ffi::lean_rs_ctor_obj_cptr(self.ptr) }
    }

    #[inline(always)]
    pub fn ctor_scalar_ptr(self) -> *mut u8 {
        unsafe { ffi::lean_rs_ctor_scalar_cptr(self.ptr) }
    }

    #[inline(always)]
    pub unsafe fn ctor_obj_slice(self) -> &'a [*mut LeanObject] {
        let len = self.ctor_num_objs();
        slice::from_raw_parts(self.ctor_obj_ptr() as *const *mut LeanObject, len)
    }

    #[inline(always)]
    pub unsafe fn ctor_scalar_slice(self, len: usize) -> &'a [u8] {
        slice::from_raw_parts(self.ctor_scalar_ptr() as *const u8, len)
    }

    #[inline(always)]
    pub fn array_size(self) -> usize {
        unsafe { ffi::lean_rs_array_size(self.ptr) }
    }

    #[inline(always)]
    pub fn array_cptr(self) -> *mut *mut LeanObject {
        unsafe { ffi::lean_rs_array_cptr(self.ptr) }
    }

    #[inline(always)]
    pub unsafe fn array_slice(self) -> &'a [*mut LeanObject] {
        let len = self.array_size();
        slice::from_raw_parts(self.array_cptr() as *const *mut LeanObject, len)
    }

    #[inline(always)]
    pub fn sarray_size(self) -> usize {
        unsafe { ffi::lean_rs_sarray_size(self.ptr) }
    }

    #[inline(always)]
    pub fn sarray_cptr(self) -> *mut u8 {
        unsafe { ffi::lean_rs_sarray_cptr(self.ptr) }
    }

    #[inline(always)]
    pub unsafe fn sarray_slice(self) -> &'a [u8] {
        let len = self.sarray_size();
        slice::from_raw_parts(self.sarray_cptr() as *const u8, len)
    }

    #[inline(always)]
    pub fn string_size(self) -> usize {
        unsafe { ffi::lean_rs_string_size(self.ptr) }
    }

    #[inline(always)]
    pub fn string_len(self) -> usize {
        unsafe { ffi::lean_rs_string_len(self.ptr) }
    }

    #[inline(always)]
    pub fn string_cstr(self) -> *const u8 {
        unsafe { ffi::lean_rs_string_cstr(self.ptr) as *const u8 }
    }

    #[inline(always)]
    pub unsafe fn string_bytes(self) -> &'a [u8] {
        let len = self.string_size();
        slice::from_raw_parts(self.string_cstr(), len)
    }
}
