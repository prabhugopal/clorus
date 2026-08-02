// Auto-generated FFI wrappers by clorus-ffi-gen
use super::*;
// Auto-generated FFI wrappers
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn clorus_add(x: f64, y: f64) -> f64 {
    let x_rust = x;
        let y_rust = y;
    let result = add(x_rust, y_rust);
    result
}

#[no_mangle]
pub extern "C" fn clorus_multiply(a: f64, b: f64) -> f64 {
    let a_rust = a;
        let b_rust = b;
    let result = multiply(a_rust, b_rust);
    result
}

#[no_mangle]
pub extern "C" fn clorus_factorial(n: f64) -> f64 {
    let n_rust = n;
    let result = factorial(n_rust);
    result
}

#[no_mangle]
pub extern "C" fn clorus_greet(name: *const c_char) -> *const c_char {
    let name_rust = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
    let result = greet(name_rust);
    unsafe { CString::new(result).unwrap().into_raw() }
}

#[no_mangle]
pub extern "C" fn clorus_to_upper(s: *const c_char) -> *const c_char {
    let s_rust = unsafe { CStr::from_ptr(s).to_string_lossy().to_string() };
    let result = to_upper(s_rust);
    unsafe { CString::new(result).unwrap().into_raw() }
}

#[no_mangle]
pub extern "C" fn clorus_string_length(s: *const c_char) -> f64 {
    let s_rust = unsafe { CStr::from_ptr(s).to_string_lossy().to_string() };
    let result = string_length(s_rust);
    result
}


