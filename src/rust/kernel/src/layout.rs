use crate::LeanObject;
use memoffset::offset_of;
use static_assertions::const_assert;
use std::mem;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct LeanObjectHeader {
    pub rc: i32,
    pub cs_sz: u16,
    pub other: u8,
    pub tag: u8,
}

#[repr(C)]
pub struct LeanArrayObject {
    pub header: LeanObjectHeader,
    pub size: usize,
    pub capacity: usize,
    pub data: [*mut LeanObject; 0],
}

#[repr(C)]
pub struct LeanCtorObject {
    pub header: LeanObjectHeader,
    pub objs: [*mut LeanObject; 0],
}

#[repr(C)]
pub struct LeanSArrayObject {
    pub header: LeanObjectHeader,
    pub size: usize,
    pub capacity: usize,
    pub data: [u8; 0],
}

#[repr(C)]
pub struct LeanStringObject {
    pub header: LeanObjectHeader,
    pub size: usize,
    pub capacity: usize,
    pub length: usize,
    pub data: [u8; 0],
}

const_assert!(mem::size_of::<LeanObjectHeader>() == mem::size_of::<i32>() + mem::size_of::<u16>() + 2);
const_assert!(offset_of!(LeanArrayObject, size) == mem::size_of::<LeanObjectHeader>());
const_assert!(offset_of!(LeanArrayObject, capacity) == mem::size_of::<LeanObjectHeader>() + mem::size_of::<usize>());
const_assert!(offset_of!(LeanSArrayObject, size) == mem::size_of::<LeanObjectHeader>());
const_assert!(offset_of!(LeanSArrayObject, capacity) == mem::size_of::<LeanObjectHeader>() + mem::size_of::<usize>());
const_assert!(offset_of!(LeanStringObject, size) == mem::size_of::<LeanObjectHeader>());
const_assert!(offset_of!(LeanStringObject, capacity) == mem::size_of::<LeanObjectHeader>() + mem::size_of::<usize>());
const_assert!(offset_of!(LeanStringObject, length) == mem::size_of::<LeanObjectHeader>() + 2 * mem::size_of::<usize>());
const_assert!(offset_of!(LeanStringObject, data) == mem::size_of::<LeanObjectHeader>() + 3 * mem::size_of::<usize>());
const_assert!(offset_of!(LeanCtorObject, objs) == mem::size_of::<LeanObjectHeader>());

#[inline(always)]
pub unsafe fn array_obj<'a>(ptr: *mut LeanObject) -> &'a LeanArrayObject {
    &*(ptr as *const LeanArrayObject)
}

#[inline(always)]
pub unsafe fn ctor_obj<'a>(ptr: *mut LeanObject) -> &'a LeanCtorObject {
    &*(ptr as *const LeanCtorObject)
}

#[inline(always)]
pub unsafe fn sarray_obj<'a>(ptr: *mut LeanObject) -> &'a LeanSArrayObject {
    &*(ptr as *const LeanSArrayObject)
}

#[inline(always)]
pub unsafe fn string_obj<'a>(ptr: *mut LeanObject) -> &'a LeanStringObject {
    &*(ptr as *const LeanStringObject)
}
