/// Function runtime support for Clorus
///
/// Functions are first-class values that can capture their environment (closures).
/// They are represented as FunctionData structures with optional captured values.

use crate::value::Value;

#[inline]
fn call_error_logging_enabled() -> bool {
    std::env::var("CLORUS_LOG_CALL_ERRORS")
        .ok()
        .map(|v| v != "0")
        .unwrap_or(false)
}

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

/// Multi-arity function variant
#[repr(C)]
pub struct ArityVariant {
    /// Number of parameters for this arity
    pub arity: i32,
    /// Function pointer for this arity
    pub func_ptr: *const u8,
}

/// Multi-arity function data structure
/// Stores multiple arity variants that share the same environment
#[repr(C)]
pub struct MultiArityFunctionData {
    /// Number of arity variants
    pub arity_count: u32,
    /// Number of captured environment values (shared across all arities)
    pub env_size: u32,
    // Following this struct in memory:
    // 1. Array of ArityVariant: [ArityVariant; arity_count]
    // 2. Array of captured values: [*mut Value; env_size]
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

unsafe fn build_rest_vector_from_args(
    args: *const *mut Value,
    start: i32,
    arg_count: i32,
) -> *mut Value {
    let mut rest_vec = crate::vector::clorus_vector_empty();
    for i in start..arg_count {
        let arg = *args.offset(i as isize);
        rest_vec = crate::vector::clorus_vector_conj(rest_vec, arg);
    }
    rest_vec
}

unsafe fn call_non_variadic_function(
    func_ptr: *const u8,
    args: *const *mut Value,
    arg_count: i32,
    env_ptr: *mut i8,
) -> *mut Value {
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
            if call_error_logging_enabled() {
                eprintln!("Function call with {} arguments not yet supported", arg_count);
            }
            Value::nil()
        }
    }
}

unsafe fn call_variadic_function(
    func_ptr: *const u8,
    args: *const *mut Value,
    arg_count: i32,
    fixed_count: i32,
    env_ptr: *mut i8,
) -> *mut Value {
    if arg_count < fixed_count {
        if call_error_logging_enabled() {
            eprintln!(
                "Arity mismatch: variadic function expected at least {}, got {}",
                fixed_count, arg_count
            );
        }
        return Value::nil();
    }

    let rest_vec = build_rest_vector_from_args(args, fixed_count, arg_count);
    match fixed_count {
        0 => {
            let f: extern "C" fn(*mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
            f(rest_vec, env_ptr)
        }
        1 => {
            let f: extern "C" fn(*mut Value, *mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
            f(*args.offset(0), rest_vec, env_ptr)
        }
        2 => {
            let f: extern "C" fn(*mut Value, *mut Value, *mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
            f(*args.offset(0), *args.offset(1), rest_vec, env_ptr)
        }
        3 => {
            let f: extern "C" fn(*mut Value, *mut Value, *mut Value, *mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
            f(*args.offset(0), *args.offset(1), *args.offset(2), rest_vec, env_ptr)
        }
        4 => {
            let f: extern "C" fn(*mut Value, *mut Value, *mut Value, *mut Value, *mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
            f(*args.offset(0), *args.offset(1), *args.offset(2), *args.offset(3), rest_vec, env_ptr)
        }
        5 => {
            let f: extern "C" fn(*mut Value, *mut Value, *mut Value, *mut Value, *mut Value, *mut Value, *mut i8) -> *mut Value = std::mem::transmute(func_ptr);
            f(*args.offset(0), *args.offset(1), *args.offset(2), *args.offset(3), *args.offset(4), rest_vec, env_ptr)
        }
        _ => {
            if call_error_logging_enabled() {
                eprintln!(
                    "Variadic function with {} fixed arguments is not yet supported",
                    fixed_count
                );
            }
            Value::nil()
        }
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
        if func_val.is_null() {
            if call_error_logging_enabled() {
                eprintln!("Attempted to call null as function");
            }
            return Value::nil();
        }

        // Be defensive at the runtime boundary: call sites should pass function values.
        // If a Var is passed, auto-deref it here to avoid UB in mixed call paths.
        let mut target = func_val;
        if (*target).tag() == crate::value::ValueTag::Var {
            let deref = crate::var::clorus_var_get((*target).as_var());
            if deref.is_null() {
                if call_error_logging_enabled() {
                    eprintln!("Attempted to call unresolved var as function");
                }
                return Value::nil();
            }
            target = deref;
        }

        let target_tag = (*target).tag();
        if target_tag != crate::value::ValueTag::Function
            && target_tag != crate::value::ValueTag::MultiArityFunction
        {
            if call_error_logging_enabled() {
                eprintln!("Attempted to call non-function value tag {:?}", target_tag);
            }
            return Value::nil();
        }

        // Check if this is a multi-arity function
        if target_tag == crate::value::ValueTag::MultiArityFunction {
            // Dispatch to multi-arity function handler
            return clorus_multi_arity_function_call(target, args, arg_count);
        }

        // Extract function data (regular single-arity function)
        let func_data = (*target).as_function();

        // Check arity
        // Non-variadic: arity >= 0 and exact match required.
        // Variadic: arity < 0 encodes fixed prefix as (-arity - 1).
        let encoded_arity = (*func_data).arity;
        if encoded_arity >= 0 && encoded_arity != arg_count {
            if call_error_logging_enabled() {
                eprintln!("Arity mismatch: expected {}, got {}", (*func_data).arity, arg_count);
            }
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

        if encoded_arity >= 0 {
            call_non_variadic_function(func_ptr, args, arg_count, env_ptr)
        } else {
            let fixed_count = -encoded_arity - 1;
            call_variadic_function(func_ptr, args, arg_count, fixed_count, env_ptr)
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

/// Create a new multi-arity function
/// arities: array of (arity, func_ptr) pairs
/// arity_count: number of arity variants
/// env: captured environment (shared across all arities)
/// env_size: number of captured values
#[no_mangle]
pub extern "C" fn clorus_multi_arity_function_new(
    arities: *const ArityVariant,
    arity_count: u32,
    env: *const *mut Value,
    env_size: u32,
) -> *mut Value {
    use std::alloc::{alloc, Layout};

    unsafe {
        // Calculate total size
        let header_size = std::mem::size_of::<MultiArityFunctionData>();
        let variants_size = (arity_count as usize) * std::mem::size_of::<ArityVariant>();
        let env_array_size = (env_size as usize) * std::mem::size_of::<*mut Value>();
        let total_size = header_size + variants_size + env_array_size;

        // Allocate memory
        let layout = Layout::from_size_align_unchecked(total_size, 8);
        let ptr = alloc(layout) as *mut MultiArityFunctionData;

        // Initialize header
        (*ptr).arity_count = arity_count;
        (*ptr).env_size = env_size;

        // Copy arity variants (after header)
        let variants_dest = ptr.offset(1) as *mut ArityVariant;
        std::ptr::copy_nonoverlapping(arities, variants_dest, arity_count as usize);

        // Copy environment (after variants)
        if env_size > 0 {
            let env_dest = variants_dest.offset(arity_count as isize) as *mut *mut Value;
            std::ptr::copy_nonoverlapping(env, env_dest, env_size as usize);

            // Retain each captured value
            for i in 0..env_size {
                let val = *env_dest.offset(i as isize);
                if !val.is_null() {
                    crate::value::clorus_retain(val);
                }
            }
        }

        // Wrap in Value as multi-arity function
        Value::from_multi_arity_function(ptr)
    }
}

/// Call a multi-arity function with runtime dispatch
#[no_mangle]
pub extern "C" fn clorus_multi_arity_function_call(
    func_val: *mut Value,
    args: *const *mut Value,
    arg_count: i32,
) -> *mut Value {
    unsafe {
        // Extract multi-arity function data
        let multi_func_data = (*func_val).as_multi_arity_function();

        // Get arity variants array
        let variants = multi_func_data.offset(1) as *const ArityVariant;

        // Find matching arity variant (prefer exact arity over variadic)
        let mut matched_variant: *const ArityVariant = std::ptr::null();
        for i in 0..(*multi_func_data).arity_count {
            let variant = variants.offset(i as isize);
            if (*variant).arity >= 0 && (*variant).arity == arg_count {
                matched_variant = variant;
                break;
            }
        }
        if matched_variant.is_null() {
            for i in 0..(*multi_func_data).arity_count {
                let variant = variants.offset(i as isize);
                if (*variant).arity < 0 {
                    let fixed_count = -(*variant).arity - 1;
                    if arg_count >= fixed_count {
                        matched_variant = variant;
                        break;
                    }
                }
            }
        }

        if matched_variant.is_null() {
            if call_error_logging_enabled() {
                eprintln!("Multi-arity function: No matching arity for {} arguments", arg_count);
                eprintln!("Available arities: ");
                for i in 0..(*multi_func_data).arity_count {
                    let variant = &*variants.offset(i as isize);
                    eprintln!("  - {}", variant.arity);
                }
            }
            return Value::nil();
        }

        // Get environment pointer (after variants array)
        let env_ptr = if (*multi_func_data).env_size > 0 {
            variants.offset((*multi_func_data).arity_count as isize) as *mut i8
        } else {
            std::ptr::null_mut()
        };

        let func_ptr = (*matched_variant).func_ptr;
        let encoded_arity = (*matched_variant).arity;
        if encoded_arity >= 0 {
            call_non_variadic_function(func_ptr, args, arg_count, env_ptr)
        } else {
            let fixed_count = -encoded_arity - 1;
            call_variadic_function(func_ptr, args, arg_count, fixed_count, env_ptr)
        }
    }
}
