#![allow(clippy::missing_safety_doc)]

use libc::c_uchar;

// Placeholder symbol to prove the Rust runtime staticlib is wired in.
#[no_mangle]
pub extern "C" fn lean_runtime_rs_ping() -> c_uchar {
    1
}
