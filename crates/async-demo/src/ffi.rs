// Auto-generated FFI wrappers by clorus-ffi-gen
use super::*;
// Auto-generated FFI wrappers
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// Note: Async functions can't be called directly from C FFI
// Use the blocking wrappers instead

#[no_mangle]
pub extern "C" fn clorus_hello_blocking() -> *const c_char {
    let result = hello_blocking();
    CString::new(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn clorus_countdown_blocking(n: f64) -> f64 {
    countdown_blocking(n)
}


