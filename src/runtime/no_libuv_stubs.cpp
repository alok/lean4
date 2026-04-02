#include <stdint.h>

#include <lean/lean.h>

extern "C" {

static lean_obj_res lean_no_libuv_unsupported(const char * fn) {
    lean_object * msg = lean_mk_string(fn);
    return lean_mk_io_error_unsupported_operation(0, msg);
}

lean_obj_res lean_uv_tcp_wait_readable(lean_obj_arg) {
    return lean_no_libuv_unsupported("libuv is disabled: lean_uv_tcp_wait_readable");
}
lean_obj_res lean_uv_tcp_cancel_recv(lean_obj_arg) {
    return lean_no_libuv_unsupported("libuv is disabled: lean_uv_tcp_cancel_recv");
}
lean_obj_res lean_uv_tcp_try_accept(lean_obj_arg) {
    return lean_no_libuv_unsupported("libuv is disabled: lean_uv_tcp_try_accept");
}
lean_obj_res lean_uv_udp_wait_readable(lean_obj_arg) {
    return lean_no_libuv_unsupported("libuv is disabled: lean_uv_udp_wait_readable");
}
lean_obj_res lean_uv_udp_cancel_recv(lean_obj_arg) {
    return lean_no_libuv_unsupported("libuv is disabled: lean_uv_udp_cancel_recv");
}
}
