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
