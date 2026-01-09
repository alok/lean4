/*
Copyright (c) 2026.
Released under Apache 2.0 license as described in the file LICENSE.

FFI wrappers for Rust runtime code. These wrap inline functions so they can be
called from Rust via extern "C".
*/
#include <lean/lean.h>
#include "runtime/object.h"

namespace lean {

extern "C" LEAN_EXPORT lean_object * lean_ctor_get_ffi(b_lean_obj_arg o, unsigned idx) {
    return lean_ctor_get(o, idx);
}

extern "C" LEAN_EXPORT size_t lean_unbox_ffi(b_lean_obj_arg o) {
    return lean_unbox(o);
}

extern "C" LEAN_EXPORT uint8_t * lean_sarray_cptr_ffi(b_lean_obj_arg a) {
    return lean_sarray_cptr(a);
}

extern "C" LEAN_EXPORT lean_object * lean_io_result_mk_ok_ffi(lean_obj_arg a) {
    return lean_io_result_mk_ok(a);
}

extern "C" LEAN_EXPORT lean_object * lean_io_result_mk_error_ffi(lean_obj_arg e) {
    return lean_io_result_mk_error(e);
}

extern "C" LEAN_EXPORT char const * lean_string_cstr_ffi(b_lean_obj_arg o) {
    return lean_string_cstr(o);
}

extern "C" LEAN_EXPORT uint8_t lean_is_st_ffi(b_lean_obj_arg o) {
    return lean_is_st(o);
}

extern "C" LEAN_EXPORT uint8_t lean_is_exclusive_ffi(b_lean_obj_arg o) {
    return lean_is_exclusive(o);
}

extern "C" LEAN_EXPORT uint8_t lean_is_ctor_ffi(b_lean_obj_arg o) {
    return lean_is_ctor(o);
}

extern "C" LEAN_EXPORT uint8_t lean_is_array_ffi(b_lean_obj_arg o) {
    return lean_is_array(o);
}

extern "C" LEAN_EXPORT uint8_t lean_is_sarray_ffi(b_lean_obj_arg o) {
    return lean_is_sarray(o);
}

extern "C" LEAN_EXPORT uint8_t lean_is_string_ffi(b_lean_obj_arg o) {
    return lean_is_string(o);
}

extern "C" LEAN_EXPORT uint8_t lean_is_mpz_ffi(b_lean_obj_arg o) {
    return lean_is_mpz(o);
}

extern "C" LEAN_EXPORT void lean_inc_ref_ffi(b_lean_obj_arg o) {
    lean_inc_ref(o);
}

extern "C" LEAN_EXPORT void lean_dec_ref_ffi(b_lean_obj_arg o) {
    lean_dec_ref(o);
}

extern "C" LEAN_EXPORT unsigned lean_ctor_num_objs_ffi(b_lean_obj_arg o) {
    return lean_ctor_num_objs(o);
}

extern "C" LEAN_EXPORT lean_object * lean_ctor_get_core_ffi(b_lean_obj_arg o, unsigned idx) {
    return lean_ctor_get(o, idx);
}

extern "C" LEAN_EXPORT void lean_ctor_set_core_ffi(lean_object * o, unsigned idx, lean_object * v) {
    lean_ctor_set(o, idx, v);
}

extern "C" LEAN_EXPORT lean_object * lean_alloc_ctor_ffi(unsigned tag, unsigned num_objs, unsigned scalar_sz) {
    return lean_alloc_ctor(tag, num_objs, scalar_sz);
}

extern "C" LEAN_EXPORT lean_object * lean_alloc_array_ffi(size_t size, size_t capacity) {
    return lean_alloc_array(size, capacity);
}

extern "C" LEAN_EXPORT void lean_array_set_core_ffi(lean_object * o, size_t i, lean_object * v) {
    lean_array_set_core(o, i, v);
}

extern "C" LEAN_EXPORT uint8_t lean_ptr_tag_ffi(b_lean_obj_arg o) {
    return lean_ptr_tag(o);
}

extern "C" LEAN_EXPORT unsigned lean_ptr_other_ffi(b_lean_obj_arg o) {
    return lean_ptr_other(o);
}

extern "C" LEAN_EXPORT uint8_t lean_is_scalar_ffi(b_lean_obj_arg o) {
    return lean_is_scalar(o);
}

extern "C" LEAN_EXPORT size_t lean_object_header_size_ffi() {
    return sizeof(lean_object);
}

extern "C" LEAN_EXPORT uint8_t lean_mpz_tag_ffi() {
    return LeanMPZ;
}

extern "C" LEAN_EXPORT uint8_t lean_mpz_eq_ffi(b_lean_obj_arg a, b_lean_obj_arg b) {
    return mpz_value(a) == mpz_value(b);
}

extern "C" LEAN_EXPORT uint64_t lean_mpz_hash_ffi(b_lean_obj_arg a) {
    return mpz_value(a).hash();
}

}
