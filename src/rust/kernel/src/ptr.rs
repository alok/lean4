use crate::LeanObject;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct LeanPtr(pub usize);

impl LeanPtr {
    #[inline(always)]
    pub fn from_raw(ptr: *mut LeanObject) -> Self {
        Self(ptr as usize)
    }

    #[inline(always)]
    pub fn to_raw(self) -> *mut LeanObject {
        self.0 as *mut LeanObject
    }

    #[inline(always)]
    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub fn is_scalar(self) -> bool {
        is_scalar_bits(self.0)
    }
}

#[inline(always)]
pub const fn is_scalar_bits(bits: usize) -> bool {
    (bits & 1) != 0
}

#[inline(always)]
#[allow(dead_code)]
pub const fn box_bits(bits: usize) -> usize {
    (bits << 1) | 1
}

#[inline(always)]
pub const fn unbox_bits(bits: usize) -> usize {
    bits >> 1
}

#[inline(always)]
pub fn is_scalar_ptr(ptr: *mut LeanObject) -> bool {
    is_scalar_bits(ptr as usize)
}

#[inline(always)]
pub fn unbox_ptr(ptr: *mut LeanObject) -> usize {
    unbox_bits(ptr as usize)
}
