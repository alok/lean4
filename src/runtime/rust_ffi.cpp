/*
Copyright (c) 2026.
Released under Apache 2.0 license as described in the file LICENSE.

FFI wrappers for Rust runtime code. These wrap inline functions so they can be
called from Rust via extern "C".
*/
#include <lean/lean.h>

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

}
