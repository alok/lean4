/*
Copyright (c) 2026.
Released under Apache 2.0 license as described in the file LICENSE.

FFI wrappers for Rust kernel code. These wrap inline functions and
template implementations so they can be called from Rust via extern "C".
*/
#include <lean/lean.h>
#include "util/kvmap.h"

namespace lean {

// Wrapper for lean_nat_eq (inline in lean.h)
extern "C" LEAN_EXPORT uint8_t lean_nat_eq_ffi(b_lean_obj_arg a, b_lean_obj_arg b) {
    return lean_nat_eq(a, b);
}

// Wrapper for lean_string_eq (inline in lean.h)
extern "C" LEAN_EXPORT uint8_t lean_string_eq_ffi(b_lean_obj_arg a, b_lean_obj_arg b) {
    return lean_string_eq(a, b);
}

// kvmap equality - compares two kvmaps (list of name-datavalue pairs)
extern "C" LEAN_EXPORT uint8_t lean_kvmap_eq_ffi(b_lean_obj_arg a, b_lean_obj_arg b) {
    kvmap const & m1 = static_cast<kvmap const &>(TO_REF(object_ref, a));
    kvmap const & m2 = static_cast<kvmap const &>(TO_REF(object_ref, b));
    return m1 == m2;
}

// Array access wrappers (inline functions in lean.h)
extern "C" LEAN_EXPORT size_t lean_array_size_ffi(b_lean_obj_arg a) {
    return lean_array_size(a);
}

extern "C" LEAN_EXPORT lean_object* lean_array_get_core_ffi(b_lean_obj_arg a, size_t i) {
    return lean_array_get_core(a, i);
}

// Array element pointer for range operations
extern "C" LEAN_EXPORT lean_object** lean_array_cptr_ffi(b_lean_obj_arg a) {
    return lean_array_cptr(a);
}

// List cons - create a cons cell (inline in lean.h)
extern "C" LEAN_EXPORT lean_object* lean_list_cons(lean_object* head, lean_object* tail) {
    lean_object* r = lean_alloc_ctor(1, 2, 0);
    lean_ctor_set(r, 0, head);
    lean_ctor_set(r, 1, tail);
    return r;
}

// Empty array with capacity (inline in lean.h)
extern "C" LEAN_EXPORT lean_object* lean_mk_empty_array_with_capacity(b_lean_obj_arg capacity) {
    size_t cap = lean_usize_of_nat(capacity);
    return lean_alloc_array(0, cap);
}

}
