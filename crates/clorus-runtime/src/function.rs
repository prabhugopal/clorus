/// Function runtime support for Clorus
///
/// Functions are first-class values that can capture their environment (closures).
/// They are represented as FunctionData structures with optional captured values.

use crate::value::Value;

/// Calling convention used by a function value.
///
/// `Legacy` remains available for native runtime callbacks which expose a
/// fixed Rust/C ABI. Compiler-generated Clorus functions use `CallFrame`, a
/// single stable ABI which carries the complete argument array and count.
/// Keeping the distinction explicit prevents an unsafe transmute from
/// treating a native callback as generated code (or vice versa).
const CALL_CONVENTION_LEGACY: u8 = 0;
const CALL_CONVENTION_FRAME: u8 = 1;

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
    /// ABI expected by `func_ptr`; see the calling-convention constants above.
    call_convention: u8,
    /// Explicit padding keeps the trailing captured-value array naturally
    /// aligned on every supported target.
    _padding: [u8; 3],
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
    /// ABI shared by every function pointer in the variants array.
    call_convention: u8,
    // The variants contain pointers and therefore require 8-byte alignment.
    // Keep this header exactly 16 bytes so `ptr.offset(1)` is aligned even
    // though the header itself has no pointer field.
    _padding: [u8; 7],
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
    function_new_with_convention(func_ptr, arity, env, env_size, CALL_CONVENTION_LEGACY)
}

/// Create a compiler-generated function value using the uniform call-frame
/// ABI: `fn(args: *const *mut Value, arg_count: i32, env: *mut i8) -> Value`.
///
/// This is deliberately separate from `clorus_function_new`: embedders can
/// continue to register small, typed native callbacks through the legacy API,
/// while generated language functions are no longer limited by a handwritten
/// set of host signatures.
#[no_mangle]
pub extern "C" fn clorus_function_new_call_frame(
    func_ptr: *const u8,
    arity: i32,
    env: *const *mut Value,
    env_size: u32,
) -> *mut Value {
    function_new_with_convention(func_ptr, arity, env, env_size, CALL_CONVENTION_FRAME)
}

fn function_new_with_convention(
    func_ptr: *const u8,
    arity: i32,
    env: *const *mut Value,
    env_size: u32,
    call_convention: u8,
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
        (*ptr).call_convention = call_convention;
        (*ptr)._padding = [0; 3];
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

type CallFrameFunction = unsafe extern "C" fn(*const *mut Value, i32, *mut i8) -> *mut Value;

unsafe fn call_frame_function(
    func_ptr: *const u8,
    args: *const *mut Value,
    arg_count: i32,
    env_ptr: *mut i8,
) -> *mut Value {
    let function: CallFrameFunction = std::mem::transmute(func_ptr);
    function(args, arg_count, env_ptr)
}

/// Collect the tail of a call frame for a compiler-generated variadic
/// function. Zero trailing arguments use nil, matching direct variadic calls
/// and keeping the representation independent of the caller (JIT or AOT).
#[no_mangle]
pub extern "C" fn clorus_call_frame_rest(
    args: *const *mut Value,
    start: i32,
    arg_count: i32,
) -> *mut Value {
    unsafe {
        if start < 0 || arg_count < start || (arg_count > 0 && args.is_null()) {
            return Value::nil();
        }
        if start == arg_count {
            return Value::nil();
        }

        let mut rest_vec = crate::vector::clorus_vector_empty();
        for i in start..arg_count {
            rest_vec = crate::vector::clorus_vector_conj(rest_vec, *args.offset(i as isize));
        }
        rest_vec
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
        if arg_count < 0 {
            if call_error_logging_enabled() {
                eprintln!("Function call received a negative argument count: {}", arg_count);
            }
            return Value::nil();
        }
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

        if encoded_arity < 0 && arg_count < -encoded_arity - 1 {
            if call_error_logging_enabled() {
                eprintln!(
                    "Arity mismatch: variadic function expected at least {}, got {}",
                    -encoded_arity - 1,
                    arg_count
                );
            }
            return Value::nil();
        }

        if (*func_data).call_convention == CALL_CONVENTION_FRAME {
            return call_frame_function(func_ptr, args, arg_count, env_ptr);
        }

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
        // Nil-safe: real Clojure's `(apply f nil)` calls f with no args, and
        // the raw clorus_vector_count/clorus_vector_nth assume their input
        // is always an actual PersistentVector, crashing on nil (see
        // clorus_vector_rest's doc comment for why nil -- not an empty
        // vector -- is now a normal, common "no args" representation).
        let count = crate::collections::clorus_count(args_vector) as u64;

        // Extract arguments from vector into array
        let mut args_array: Vec<*mut Value> = Vec::with_capacity(count as usize);
        for i in 0..count {
            let arg = crate::collections::clorus_nth(args_vector, i as i64);
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
    multi_arity_function_new_with_convention(
        arities,
        arity_count,
        env,
        env_size,
        CALL_CONVENTION_LEGACY,
    )
}

/// Create a multi-arity compiler-generated closure whose variants all use the
/// uniform call-frame ABI documented by `clorus_function_new_call_frame`.
#[no_mangle]
pub extern "C" fn clorus_multi_arity_function_new_call_frame(
    arities: *const ArityVariant,
    arity_count: u32,
    env: *const *mut Value,
    env_size: u32,
) -> *mut Value {
    multi_arity_function_new_with_convention(
        arities,
        arity_count,
        env,
        env_size,
        CALL_CONVENTION_FRAME,
    )
}

fn multi_arity_function_new_with_convention(
    arities: *const ArityVariant,
    arity_count: u32,
    env: *const *mut Value,
    env_size: u32,
    call_convention: u8,
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
        (*ptr).call_convention = call_convention;
        (*ptr)._padding = [0; 7];

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
        if arg_count < 0 {
            if call_error_logging_enabled() {
                eprintln!("Multi-arity function received a negative argument count: {}", arg_count);
            }
            return Value::nil();
        }
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
        if (*multi_func_data).call_convention == CALL_CONVENTION_FRAME {
            call_frame_function(func_ptr, args, arg_count, env_ptr)
        } else if encoded_arity >= 0 {
            call_non_variadic_function(func_ptr, args, arg_count, env_ptr)
        } else {
            let fixed_count = -encoded_arity - 1;
            call_variadic_function(func_ptr, args, arg_count, fixed_count, env_ptr)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn count_call_frame(
        _args: *const *mut Value,
        arg_count: i32,
        _env: *mut i8,
    ) -> *mut Value {
        Value::long(arg_count as i64)
    }

    #[test]
    fn call_frame_accepts_arbitrary_fixed_arity() {
        unsafe {
            let function = clorus_function_new_call_frame(
                count_call_frame as *const u8,
                7,
                std::ptr::null(),
                0,
            );
            let args = (0..7).map(Value::long).collect::<Vec<_>>();
            let result = clorus_function_call(function, args.as_ptr(), args.len() as i32);

            assert_eq!((*result).as_long(), 7);

            crate::value::clorus_release(result);
            crate::value::clorus_release(function);
            for arg in args {
                crate::value::clorus_release(arg);
            }
        }
    }
}
