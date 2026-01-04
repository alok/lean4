use crate::ffi;
use crate::layout;
use crate::LeanObject;
use crate::ptr;
use libc::c_uint;
use std::marker::PhantomData;
use std::mem;
use std::slice;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct LeanArrayView<'a> {
    data: *const *mut LeanObject,
    len: usize,
    _marker: PhantomData<&'a [*mut LeanObject]>,
}

#[allow(dead_code)]
impl<'a> LeanArrayView<'a> {
    #[inline(always)]
    pub const unsafe fn from_raw(data: *const *mut LeanObject, len: usize) -> Self {
        Self {
            data,
            len,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub const fn len(self) -> usize {
        self.len
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub unsafe fn as_slice(self) -> &'a [*mut LeanObject] {
        slice::from_raw_parts(self.data, self.len)
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct LeanSArrayView<'a> {
    data: *const u8,
    len: usize,
    elem_size: usize,
    _marker: PhantomData<&'a [u8]>,
}

#[allow(dead_code)]
impl<'a> LeanSArrayView<'a> {
    #[inline(always)]
    pub const unsafe fn from_raw(data: *const u8, len: usize, elem_size: usize) -> Self {
        Self {
            data,
            len,
            elem_size,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub const fn len(self) -> usize {
        self.len
    }

    #[inline(always)]
    pub const fn elem_size(self) -> usize {
        self.elem_size
    }

    #[inline(always)]
    pub const fn byte_len(self) -> usize {
        self.len.saturating_mul(self.elem_size)
    }

    #[inline(always)]
    pub unsafe fn as_bytes(self) -> &'a [u8] {
        slice::from_raw_parts(self.data, self.byte_len())
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct LeanStringView<'a> {
    data: *const u8,
    byte_len: usize,
    utf8_len: usize,
    _marker: PhantomData<&'a [u8]>,
}

#[allow(dead_code)]
impl<'a> LeanStringView<'a> {
    #[inline(always)]
    pub const unsafe fn from_raw(data: *const u8, byte_len: usize, utf8_len: usize) -> Self {
        Self {
            data,
            byte_len,
            utf8_len,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub const fn byte_len(self) -> usize {
        self.byte_len
    }

    #[inline(always)]
    pub const fn utf8_len(self) -> usize {
        self.utf8_len
    }

    #[inline(always)]
    pub const fn content_len(self) -> usize {
        self.byte_len.saturating_sub(1)
    }

    #[inline(always)]
    pub unsafe fn as_bytes(self) -> &'a [u8] {
        slice::from_raw_parts(self.data, self.byte_len)
    }

    #[inline(always)]
    pub unsafe fn as_content_bytes(self) -> &'a [u8] {
        slice::from_raw_parts(self.data, self.content_len())
    }
}

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
        unsafe { layout::array_obj(self.ptr).size }
    }

    #[inline(always)]
    pub fn array_cptr(self) -> *mut *mut LeanObject {
        unsafe { layout::array_obj(self.ptr).data.as_ptr() as *mut *mut LeanObject }
    }

    #[inline(always)]
    pub unsafe fn array_slice(self) -> &'a [*mut LeanObject] {
        self.array_view().as_slice()
    }

    #[inline(always)]
    pub unsafe fn array_view(self) -> LeanArrayView<'a> {
        LeanArrayView::from_raw(self.array_cptr() as *const *mut LeanObject, self.array_size())
    }

    #[inline(always)]
    pub fn sarray_size(self) -> usize {
        unsafe { layout::sarray_obj(self.ptr).size }
    }

    #[inline(always)]
    pub fn sarray_elem_size(self) -> usize {
        unsafe { ffi::lean_rs_sarray_elem_size(self.ptr) }
    }

    #[inline(always)]
    pub fn sarray_cptr(self) -> *mut u8 {
        unsafe { layout::sarray_obj(self.ptr).data.as_ptr() as *mut u8 }
    }

    #[inline(always)]
    pub unsafe fn sarray_slice(self) -> &'a [u8] {
        self.sarray_view().as_bytes()
    }

    #[inline(always)]
    pub unsafe fn sarray_view(self) -> LeanSArrayView<'a> {
        LeanSArrayView::from_raw(
            self.sarray_cptr() as *const u8,
            self.sarray_size(),
            self.sarray_elem_size(),
        )
    }

    #[inline(always)]
    pub fn string_size(self) -> usize {
        unsafe { layout::string_obj(self.ptr).size }
    }

    #[inline(always)]
    pub fn string_len(self) -> usize {
        unsafe { layout::string_obj(self.ptr).length }
    }

    #[inline(always)]
    pub fn string_cstr(self) -> *const u8 {
        unsafe { layout::string_obj(self.ptr).data.as_ptr() as *const u8 }
    }

    #[inline(always)]
    pub unsafe fn string_bytes(self) -> &'a [u8] {
        self.string_view().as_bytes()
    }

    #[inline(always)]
    pub unsafe fn string_view(self) -> LeanStringView<'a> {
        LeanStringView::from_raw(self.string_cstr(), self.string_size(), self.string_len())
    }
}
