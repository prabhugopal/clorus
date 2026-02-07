/// Function runtime support for Clorus
///
/// Functions are first-class values that can capture their environment (closures).
/// They are represented as FunctionData structures with optional captured values.

use crate::value::Value;

/// Function data structure
/// Contains function pointer and captured environment
#[repr(C)]
pub struct FunctionData {
    /// Function pointer (to LLVM-generated code)
    pub func_ptr: *const u8,
    /// Arity (number of parameters)
    pub arity: i32,
    /// Number of captured environment values
    env_size: u32,
    // Following this struct in memory is an array of *mut Value (captured environment)
    // Array is allocated inline after the struct: [*mut Value; env_size]
}

impl FunctionData {
    /// Get the environment size
    pub fn env_size(&self) -> u32 {
        self.env_size
    }
}

/// Create a new function with captured environment
#[no_mangle]
pub extern "C" fn clorus_function_new(
    func_ptr: *const u8,
    arity: i32,
    env: *const *mut Value,
    env_size: u32,
) -> *mut Value {
    use std::alloc::{alloc, Layout};

    unsafe {
        // Calculate total size: FunctionData + array of env pointers
        let func_data_size = std::mem::size_of::<FunctionData>();
        let env_array_size = (env_size as usize) * std::mem::size_of::<*mut Value>();
        let total_size = func_data_size + env_array_size;

        // Allocate memory for function data + environment array
        let layout = Layout::from_size_align_unchecked(total_size, 8);
        let ptr = alloc(layout) as *mut FunctionData;

        // Initialize function data
        (*ptr).func_ptr = func_ptr;
        (*ptr).arity = arity;
        (*ptr).env_size = env_size;

        // Copy environment pointers (after the FunctionData struct)
        if env_size > 0 {
            let env_dest = ptr.offset(1) as *mut *mut Value;
            std::ptr::copy_nonoverlapping(env, env_dest, env_size as usize);

            // Retain each captured value
            for i in 0..env_size {
                let val = *env_dest.offset(i as isize);
                if !val.is_null() {
                    crate::value::clorus_retain(val);
                }
            }
        }

        // Wrap in Value
        Value::from_function(ptr)
    }
}

/// Call a function with arguments
#[no_mangle]
pub extern "C" fn clorus_function_call(
    func_val: *mut Value,
    args: *const *mut Value,
    arg_count: i32,
) -> *mut Value {
    unsafe {
        // Extract function data
        let func_data = (*func_val).as_function();

        // Check arity
        // If arity is negative, it's variadic (accepts any number of args)
        // For now, variadic functions are not fully supported in clorus_function_call
        // We'll just accept any arg count for variadic functions
        if (*func_data).arity >= 0 && (*func_data).arity != arg_count {
            eprintln!("Arity mismatch: expected {}, got {}", (*func_data).arity, arg_count);
            return Value::nil();
        }

        // Get function pointer
        let func_ptr = (*func_data).func_ptr;

        // Get environment pointer (stored after FunctionData struct)
        // Cast to *mut i8 (same as value_ptr_type in codegen)
        let env_ptr = if (*func_data).env_size > 0 {
            func_data.offset(1) as *mut i8
        } else {
            std::ptr::null_mut()
        };

        // Cast to appropriate function type based on arity
        // IMPORTANT: All closures take an additional environment parameter as the LAST argument
        match arg_count {
            0 => {
                let f: extern "C" fn(*mut i8) -> *mut Value = std::mem::transmute(func_ptr);
                f(env_ptr)
            }
            1 => {
                let f: extern "C" fn(*mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
                f(*args.offset(0), env_ptr)
            }
            2 => {
                let f: extern "C" fn(*mut Value, *mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
                f(*args.offset(0), *args.offset(1), env_ptr)
            }
            3 => {
                let f: extern "C" fn(*mut Value, *mut Value, *mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
                f(*args.offset(0), *args.offset(1), *args.offset(2), env_ptr)
            }
            4 => {
                let f: extern "C" fn(*mut Value, *mut Value, *mut Value, *mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
                f(*args.offset(0), *args.offset(1), *args.offset(2), *args.offset(3), env_ptr)
            }
            5 => {
                let f: extern "C" fn(*mut Value, *mut Value, *mut Value, *mut Value, *mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
                f(*args.offset(0), *args.offset(1), *args.offset(2), *args.offset(3), *args.offset(4), env_ptr)
            }
            6 => {
                let f: extern "C" fn(*mut Value, *mut Value, *mut Value, *mut Value, *mut Value, *mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
                f(*args.offset(0), *args.offset(1), *args.offset(2), *args.offset(3), *args.offset(4), *args.offset(5), env_ptr)
            }
            _ => {
                eprintln!("Function call with {} arguments not yet supported", arg_count);
                Value::nil()
            }
        }
    }
}

/// Call a function with a vector of arguments (convenience for apply)
#[no_mangle]
pub extern "C" fn clorus_call_with_vector(
    func_val: *mut Value,
    args_vector: *mut Value,
) -> *mut Value {
    unsafe {
        // Get vector count
        let count = crate::vector::clorus_vector_count(args_vector);

        // Extract arguments from vector into array
        let mut args_array: Vec<*mut Value> = Vec::with_capacity(count as usize);
        for i in 0..count {
            let arg = crate::vector::clorus_vector_nth(args_vector, i);
            args_array.push(arg);
        }

        // Call function with arguments
        clorus_function_call(func_val, args_array.as_ptr(), count as i32)
    }
}

/// Allocate environment array for closures
/// Returns a pointer to an array of `size` Value pointers
#[no_mangle]
pub extern "C" fn clorus_alloc_env(size: u32) -> *mut *mut Value {
    use std::alloc::{alloc, Layout};

    unsafe {
        if size == 0 {
            return std::ptr::null_mut();
        }

        // Allocate array of Value pointers
        let array_size = (size as usize) * std::mem::size_of::<*mut Value>();
        let layout = Layout::from_size_align_unchecked(array_size, 8);
        let ptr = alloc(layout) as *mut *mut Value;

        // Initialize to null
        for i in 0..size {
            *ptr.offset(i as isize) = std::ptr::null_mut();
        }

        ptr
    }
}
