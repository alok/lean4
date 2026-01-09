#![allow(dead_code)]

/// Safe mirror of tagged-pointer operations used in the Rust kernel.
/// This crate is intentionally `safe` Rust for Aeneas translation.
pub const fn is_scalar_bits(bits: usize) -> bool {
    (bits & 1) != 0
}

pub const fn box_bits(bits: usize) -> usize {
    (bits << 1) | 1
}

pub const fn unbox_bits(bits: usize) -> usize {
    bits >> 1
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct LeanPtr(pub usize);

impl LeanPtr {
    #[inline(always)]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub const fn is_scalar(self) -> bool {
        is_scalar_bits(self.0)
    }

    #[inline(always)]
    pub const fn unbox_scalar(self) -> usize {
        unbox_bits(self.0)
    }
}

pub fn roundtrip_box_unbox(x: usize) -> usize {
    unbox_bits(box_bits(x))
}

pub fn box_sets_scalar_bit(x: usize) -> bool {
    is_scalar_bits(box_bits(x))
}

