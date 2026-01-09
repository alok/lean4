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
