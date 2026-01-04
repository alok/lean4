use crate::LeanObject;

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
