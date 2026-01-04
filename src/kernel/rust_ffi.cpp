/*
Copyright (c) 2026.
Released under Apache 2.0 license as described in the file LICENSE.
*/
#include <lean/lean.h>

extern "C" uint8_t lean_rs_is_scalar(lean_object * o) {
    return lean_is_scalar(o);
}

extern "C" uint32_t lean_rs_ctor_num_objs(lean_object * o) {
    return lean_ctor_num_objs(o);
}

extern "C" uint64_t lean_rs_ctor_get_uint64(lean_object * o, uint32_t offset) {
    return lean_ctor_get_uint64(o, offset);
}

extern "C" size_t lean_rs_unbox(lean_object * o) {
    return lean_unbox(o);
}

extern "C" uint32_t lean_rs_obj_tag(lean_object * o) {
    return lean_obj_tag(o);
}

extern "C" lean_object * lean_rs_ctor_get(lean_object * o, uint32_t i) {
    return lean_ctor_get(o, i);
}

extern "C" lean_object ** lean_rs_ctor_obj_cptr(lean_object * o) {
    return lean_ctor_obj_cptr(o);
}

extern "C" uint8_t * lean_rs_ctor_scalar_cptr(lean_object * o) {
    return lean_ctor_scalar_cptr(o);
}

extern "C" size_t lean_rs_array_size(lean_object * o) {
    return lean_array_size(o);
}

extern "C" lean_object ** lean_rs_array_cptr(lean_object * o) {
    return lean_array_cptr(o);
}

extern "C" size_t lean_rs_sarray_size(lean_object * o) {
    return lean_sarray_size(o);
}

extern "C" size_t lean_rs_sarray_elem_size(lean_object * o) {
    return lean_sarray_elem_size(o);
}

extern "C" uint8_t * lean_rs_sarray_cptr(lean_object * o) {
    return lean_sarray_cptr(o);
}

extern "C" size_t lean_rs_string_size(lean_object * o) {
    return lean_string_size(o);
}

extern "C" size_t lean_rs_string_len(lean_object * o) {
    return lean_string_len(o);
}

extern "C" char const * lean_rs_string_cstr(lean_object * o) {
    return lean_string_cstr(o);
}
