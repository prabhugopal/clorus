/// I/O support for Clorus runtime
///
/// Provides print, println, and other I/O functions

use crate::value::{Value, ValueTag};
use crate::vector::PersistentVector;
use std::io::{self, Write};

/// Convert a Value* to a Rust String for display
unsafe fn value_to_display_string(val: *mut Value) -> String {
    if val.is_null() {
        return "nil".to_string();
    }

    match (*val).header().tag() {
        ValueTag::String => (*val).as_string().to_string(),
        ValueTag::Long => format!("{}", (*val).as_long()),
        ValueTag::Double => {
            let num = (*val).as_double();
            if num.fract() == 0.0 && num.is_finite() {
                format!("{:.0}", num)
            } else {
                format!("{}", num)
            }
        }
        ValueTag::Keyword => format!(":{}", (*val).as_keyword()),
        ValueTag::Symbol => (*val).as_string().to_string(),
        ValueTag::Bool => {
            if (*val).as_bool() {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        ValueTag::Nil => "nil".to_string(),
        ValueTag::Vector => {
            let vec_ptr = (*val).as_ptr() as *mut PersistentVector;
            let count = (*vec_ptr).count();
            let mut parts = Vec::new();
            for i in 0..count {
                let elem = PersistentVector::nth(vec_ptr, i);
                parts.push(value_to_display_string(elem));
                crate::value::clorus_release(elem);
            }
            format!("[{}]", parts.join(" "))
        }
        ValueTag::HashMap => format!("{{...}}"),
        _ => format!("#<{:?}>", (*val).header().tag()),
    }
}

/// Print a value to stdout without a newline
/// Returns nil
#[no_mangle]
pub extern "C" fn clorus_print(val: *mut Value) -> *mut Value {
    unsafe {
        let s = value_to_display_string(val);
        print!("{}", s);
        io::stdout().flush().unwrap_or(());
        Value::nil()
    }
}

/// Print a value to stdout with a newline
/// Returns nil
#[no_mangle]
pub extern "C" fn clorus_println(val: *mut Value) -> *mut Value {
    unsafe {
        let s = value_to_display_string(val);
        println!("{}", s);
        Value::nil()
    }
}

/// Print multiple values with spaces between them and a newline (variadic)
/// Takes a vector of values
#[no_mangle]
pub extern "C" fn clorus_println_variadic(args: *mut Value) -> *mut Value {
    unsafe {
        if args.is_null() {
            println!();
            return Value::nil();
        }

        // Get vector count
        let count = crate::collections::clorus_count(args);

        for i in 0..count {
            if i > 0 {
                print!(" ");
            }
            let idx_fn = std::mem::transmute::<_, extern "C" fn(*mut Value, i64) -> *mut Value>(
                crate::collections::clorus_nth as *const ()
            );
            let val = idx_fn(args, i as i64);
            let s = value_to_display_string(val);
            print!("{}", s);
        }
        println!();
        Value::nil()
    }
}

/// Print multiple values with spaces between them, no newline
/// Takes a vector of values
#[no_mangle]
pub extern "C" fn clorus_pr(args: *mut Value) -> *mut Value {
    unsafe {
        if args.is_null() {
            return Value::nil();
        }

        // Get vector count
        let count = crate::collections::clorus_count(args);

        for i in 0..count {
            if i > 0 {
                print!(" ");
            }

            // Get element at index i
            let elem = crate::collections::clorus_nth(args, i);

            // Print readable representation (with quotes for strings, etc.)
            let s_val = crate::string::clorus_pr_str(elem);
            if !s_val.is_null() {
                // Extract string from Value*
                let rust_str = value_to_display_string(s_val);
                print!("{}", rust_str);
                crate::value::clorus_release(s_val);
            }
        }

        io::stdout().flush().unwrap_or(());
        Value::nil()
    }
}

/// Print multiple values with spaces between them, with newline
/// Takes a vector of values
#[no_mangle]
pub extern "C" fn clorus_prn(args: *mut Value) -> *mut Value {
    clorus_pr(args);
    println!();
    Value::nil()
}

/// Read a line from stdin
/// Returns a string Value
#[no_mangle]
pub extern "C" fn clorus_read_line() -> *mut Value {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            // Remove trailing newline
            if input.ends_with('\n') {
                input.pop();
                if input.ends_with('\r') {
                    input.pop();
                }
            }

            // Create a C string
            let c_str = std::ffi::CString::new(input).unwrap_or_default();
            crate::string::clorus_string(c_str.as_ptr())
        }
        Err(_) => Value::nil(),
    }
}

/// Flush stdout
#[no_mangle]
pub extern "C" fn clorus_flush() -> *mut Value {
    io::stdout().flush().unwrap_or(());
    Value::nil()
}

/// Print to stderr with newline
#[no_mangle]
pub extern "C" fn clorus_eprintln(val: *mut Value) -> *mut Value {
    unsafe {
        let s = value_to_display_string(val);
        eprintln!("{}", s);
        Value::nil()
    }
}

/// Print a single value to stdout with "=> " prefix and newline
/// Used by compiler for printing results
#[no_mangle]
pub extern "C" fn clorus_print_value(val: *mut Value) -> *mut Value {
    unsafe {
        let s = value_to_display_string(val);
        println!("=> {}", s);
        Value::nil()
    }
}

// Helper to get module symbol (for dynamic linking)
fn module_symbol<T>(_name: &str) -> Option<T> {
    None // Placeholder - in full implementation would use libloading
}
