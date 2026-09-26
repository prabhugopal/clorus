/// Type-aware arithmetic operations for Clorus
///
/// Implements promotion rules:
/// - Long + Long → Long
/// - Long + Double → Double
/// - Double + Double → Double
///
/// Supports bitwise operations on Long types only.

use crate::value::{Value, ValueTag};

// ============================================================================
// Addition
// ============================================================================

/// Add two values with type-aware promotion
#[no_mangle]
pub extern "C" fn clorus_add(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::nil();
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        match (a_tag, b_tag) {
            // Long + Long → Long
            (ValueTag::Long, ValueTag::Long) => {
                let result = (*a).as_long() + (*b).as_long();
                Value::long(result)
            }
            // Long + Double → Double
            (ValueTag::Long, ValueTag::Double) => {
                let result = (*a).as_long() as f64 + (*b).as_double();
                Value::double(result)
            }
            // Double + Long → Double
            (ValueTag::Double, ValueTag::Long) => {
                let result = (*a).as_double() + (*b).as_long() as f64;
                Value::double(result)
            }
            // Double + Double → Double
            (ValueTag::Double, ValueTag::Double) => {
                let result = (*a).as_double() + (*b).as_double();
                Value::double(result)
            }
            _ => {
                eprintln!("Type error: Cannot add {:?} and {:?}", a_tag, b_tag);
                Value::nil()
            }
        }
    }
}

// ============================================================================
// Subtraction
// ============================================================================

/// Subtract two values with type-aware promotion
#[no_mangle]
pub extern "C" fn clorus_sub(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::nil();
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        match (a_tag, b_tag) {
            // Long - Long → Long
            (ValueTag::Long, ValueTag::Long) => {
                let result = (*a).as_long() - (*b).as_long();
                Value::long(result)
            }
            // Long - Double → Double
            (ValueTag::Long, ValueTag::Double) => {
                let result = (*a).as_long() as f64 - (*b).as_double();
                Value::double(result)
            }
            // Double - Long → Double
            (ValueTag::Double, ValueTag::Long) => {
                let result = (*a).as_double() - (*b).as_long() as f64;
                Value::double(result)
            }
            // Double - Double → Double
            (ValueTag::Double, ValueTag::Double) => {
                let result = (*a).as_double() - (*b).as_double();
                Value::double(result)
            }
            _ => {
                eprintln!("Type error: Cannot subtract {:?} and {:?}", a_tag, b_tag);
                Value::nil()
            }
        }
    }
}

// ============================================================================
// Multiplication
// ============================================================================

/// Multiply two values with type-aware promotion
#[no_mangle]
pub extern "C" fn clorus_mul(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::nil();
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        match (a_tag, b_tag) {
            // Long * Long → Long
            (ValueTag::Long, ValueTag::Long) => {
                let result = (*a).as_long() * (*b).as_long();
                Value::long(result)
            }
            // Long * Double → Double
            (ValueTag::Long, ValueTag::Double) => {
                let result = (*a).as_long() as f64 * (*b).as_double();
                Value::double(result)
            }
            // Double * Long → Double
            (ValueTag::Double, ValueTag::Long) => {
                let result = (*a).as_double() * (*b).as_long() as f64;
                Value::double(result)
            }
            // Double * Double → Double
            (ValueTag::Double, ValueTag::Double) => {
                let result = (*a).as_double() * (*b).as_double();
                Value::double(result)
            }
            _ => {
                eprintln!("Type error: Cannot multiply {:?} and {:?}", a_tag, b_tag);
                Value::nil()
            }
        }
    }
}

// ============================================================================
// Division
// ============================================================================

/// Divide two values with type-aware promotion
/// Note: Long / Long → Double (for now, will be Ratio in future)
#[no_mangle]
pub extern "C" fn clorus_div(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::nil();
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        match (a_tag, b_tag) {
            // Long / Long → Double (future: Ratio for exact division)
            (ValueTag::Long, ValueTag::Long) => {
                let divisor = (*b).as_long();
                if divisor == 0 {
                    eprintln!("Division by zero");
                    return Value::nil();
                }
                let result = (*a).as_long() as f64 / divisor as f64;
                Value::double(result)
            }
            // Long / Double → Double
            (ValueTag::Long, ValueTag::Double) => {
                let divisor = (*b).as_double();
                if divisor == 0.0 {
                    eprintln!("Division by zero");
                    return Value::nil();
                }
                let result = (*a).as_long() as f64 / divisor;
                Value::double(result)
            }
            // Double / Long → Double
            (ValueTag::Double, ValueTag::Long) => {
                let divisor = (*b).as_long();
                if divisor == 0 {
                    eprintln!("Division by zero");
                    return Value::nil();
                }
                let result = (*a).as_double() / divisor as f64;
                Value::double(result)
            }
            // Double / Double → Double
            (ValueTag::Double, ValueTag::Double) => {
                let divisor = (*b).as_double();
                if divisor == 0.0 {
                    eprintln!("Division by zero");
                    return Value::nil();
                }
                let result = (*a).as_double() / divisor;
                Value::double(result)
            }
            _ => {
                eprintln!("Type error: Cannot divide {:?} and {:?}", a_tag, b_tag);
                Value::nil()
            }
        }
    }
}

// ============================================================================
// Modulo
// ============================================================================

/// Clojure-style `mod`: floored division remainder, sign always matches the
/// divisor (e.g. `(mod -7 3)` => 2, `(mod 7 -3)` => -2) -- distinct from
/// `rem`/Rust's `%`, whose sign matches the dividend instead. Zero when
/// exactly divisible, regardless of convention.
#[no_mangle]
pub extern "C" fn clorus_mod(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::nil();
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        match (a_tag, b_tag) {
            // Long % Long → Long
            (ValueTag::Long, ValueTag::Long) => {
                let divisor = (*b).as_long();
                if divisor == 0 {
                    eprintln!("Modulo by zero");
                    return Value::nil();
                }
                let r = (*a).as_long() % divisor;
                let result = if r != 0 && (r < 0) != (divisor < 0) { r + divisor } else { r };
                Value::long(result)
            }
            // Long % Double → Double
            (ValueTag::Long, ValueTag::Double) => {
                let divisor = (*b).as_double();
                let r = ((*a).as_long() as f64) % divisor;
                let result = if r != 0.0 && (r < 0.0) != (divisor < 0.0) { r + divisor } else { r };
                Value::double(result)
            }
            // Double % Long → Double
            (ValueTag::Double, ValueTag::Long) => {
                let divisor = (*b).as_long() as f64;
                let r = (*a).as_double() % divisor;
                let result = if r != 0.0 && (r < 0.0) != (divisor < 0.0) { r + divisor } else { r };
                Value::double(result)
            }
            // Double % Double → Double
            (ValueTag::Double, ValueTag::Double) => {
                let divisor = (*b).as_double();
                let r = (*a).as_double() % divisor;
                let result = if r != 0.0 && (r < 0.0) != (divisor < 0.0) { r + divisor } else { r };
                Value::double(result)
            }
            _ => {
                eprintln!("Type error: Cannot mod {:?} and {:?}", a_tag, b_tag);
                Value::nil()
            }
        }
    }
}

// ============================================================================
// Comparison
// ============================================================================

/// Less than comparison
#[no_mangle]
pub extern "C" fn clorus_lt(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::boolean(false);
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        let result = match (a_tag, b_tag) {
            (ValueTag::Long, ValueTag::Long) => (*a).as_long() < (*b).as_long(),
            (ValueTag::Long, ValueTag::Double) => ((*a).as_long() as f64) < (*b).as_double(),
            (ValueTag::Double, ValueTag::Long) => (*a).as_double() < ((*b).as_long() as f64),
            (ValueTag::Double, ValueTag::Double) => (*a).as_double() < (*b).as_double(),
            _ => {
                eprintln!("Type error: Cannot compare {:?} and {:?}", a_tag, b_tag);
                return Value::boolean(false);
            }
        };

        Value::boolean(result)
    }
}

/// Less than or equal comparison
#[no_mangle]
pub extern "C" fn clorus_lte(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::boolean(false);
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        let result = match (a_tag, b_tag) {
            (ValueTag::Long, ValueTag::Long) => (*a).as_long() <= (*b).as_long(),
            (ValueTag::Long, ValueTag::Double) => ((*a).as_long() as f64) <= (*b).as_double(),
            (ValueTag::Double, ValueTag::Long) => (*a).as_double() <= ((*b).as_long() as f64),
            (ValueTag::Double, ValueTag::Double) => (*a).as_double() <= (*b).as_double(),
            _ => {
                eprintln!("Type error: Cannot compare {:?} and {:?}", a_tag, b_tag);
                return Value::boolean(false);
            }
        };

        Value::boolean(result)
    }
}

/// Greater than comparison
#[no_mangle]
pub extern "C" fn clorus_gt(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::boolean(false);
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        let result = match (a_tag, b_tag) {
            (ValueTag::Long, ValueTag::Long) => (*a).as_long() > (*b).as_long(),
            (ValueTag::Long, ValueTag::Double) => ((*a).as_long() as f64) > (*b).as_double(),
            (ValueTag::Double, ValueTag::Long) => (*a).as_double() > ((*b).as_long() as f64),
            (ValueTag::Double, ValueTag::Double) => (*a).as_double() > (*b).as_double(),
            _ => {
                eprintln!("Type error: Cannot compare {:?} and {:?}", a_tag, b_tag);
                return Value::boolean(false);
            }
        };

        Value::boolean(result)
    }
}

/// Greater than or equal comparison
#[no_mangle]
pub extern "C" fn clorus_gte(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::boolean(false);
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        let result = match (a_tag, b_tag) {
            (ValueTag::Long, ValueTag::Long) => (*a).as_long() >= (*b).as_long(),
            (ValueTag::Long, ValueTag::Double) => ((*a).as_long() as f64) >= (*b).as_double(),
            (ValueTag::Double, ValueTag::Long) => (*a).as_double() >= ((*b).as_long() as f64),
            (ValueTag::Double, ValueTag::Double) => (*a).as_double() >= (*b).as_double(),
            _ => {
                eprintln!("Type error: Cannot compare {:?} and {:?}", a_tag, b_tag);
                return Value::boolean(false);
            }
        };

        Value::boolean(result)
    }
}

// ============================================================================
// Variadic wrappers, for use when +/-/*//=/<// etc. are captured as
// first-class function values (e.g. `(reduce + coll)`, `(def f +)`) rather
// than applied directly. Direct call sites like `(+ 1 2 3)` are folded at
// compile time instead (see compile_add et al. in clorus-codegen's
// arithmetic.rs) and never reach these; these give a runtime-callable
// function value the same variadic semantics, since clorus_function_call
// can't unroll a compile-time loop. Called via the variadic calling
// convention (see clorus_function_call/call_variadic_function): all
// arguments arrive pre-collected into a single Vector Value, plus an
// (unused, these ops never close over anything) environment pointer.
// ============================================================================

/// Left-fold a Vector of Values with Clojure's `(op a b c ...)` semantics:
/// zero args -> `zero_args()`, one arg -> `one_arg(x)`, two or more ->
/// pairwise `op` folded left-to-right. Shared by +/-/*// so each only needs
/// to supply its own identity/unary-case/pairwise-op, not reimplement the
/// arg-count dispatch.
unsafe fn fold_variadic_arithmetic(
    args_vec: *mut Value,
    zero_args: fn() -> *mut Value,
    one_arg: fn(*mut Value) -> *mut Value,
    op: extern "C" fn(*mut Value, *mut Value) -> *mut Value,
) -> *mut Value {
    let count = crate::vector::clorus_vector_count(args_vec);
    if count == 0 {
        return zero_args();
    }
    let first = crate::vector::clorus_vector_nth(args_vec, 0);
    if count == 1 {
        return one_arg(first);
    }
    let mut acc = first;
    for i in 1..count {
        acc = op(acc, crate::vector::clorus_vector_nth(args_vec, i));
    }
    acc
}

#[no_mangle]
pub extern "C" fn clorus_add_variadic(args: *mut Value, _env: *mut i8) -> *mut Value {
    unsafe { fold_variadic_arithmetic(args, || Value::long(0), |x| x, clorus_add) }
}

#[no_mangle]
pub extern "C" fn clorus_mul_variadic(args: *mut Value, _env: *mut i8) -> *mut Value {
    unsafe { fold_variadic_arithmetic(args, || Value::long(1), |x| x, clorus_mul) }
}

#[no_mangle]
pub extern "C" fn clorus_sub_variadic(args: *mut Value, _env: *mut i8) -> *mut Value {
    unsafe {
        fold_variadic_arithmetic(
            args,
            || {
                eprintln!("Arity error: - requires at least 1 argument");
                Value::nil()
            },
            |x| clorus_sub(Value::long(0), x),
            clorus_sub,
        )
    }
}

#[no_mangle]
pub extern "C" fn clorus_div_variadic(args: *mut Value, _env: *mut i8) -> *mut Value {
    unsafe {
        fold_variadic_arithmetic(
            args,
            || {
                eprintln!("Arity error: / requires at least 1 argument");
                Value::nil()
            },
            |x| clorus_div(Value::long(1), x),
            clorus_div,
        )
    }
}

/// Chain a Vector of Values with Clojure's `(cmp a b c ...)` semantics: true
/// iff every consecutive pair satisfies `pairwise` (e.g. `(< 1 2 3)` checks
/// 1<2 and 2<3). Zero or one arg is vacuously true, matching real Clojure.
/// Shared by =/</>/<=/>= so each only supplies its own pairwise comparison.
unsafe fn chain_compare(
    args_vec: *mut Value,
    pairwise: unsafe extern "C" fn(*mut Value, *mut Value) -> *mut Value,
) -> *mut Value {
    let count = crate::vector::clorus_vector_count(args_vec);
    if count <= 1 {
        return Value::boolean(true);
    }
    let mut prev = crate::vector::clorus_vector_nth(args_vec, 0);
    for i in 1..count {
        let cur = crate::vector::clorus_vector_nth(args_vec, i);
        if crate::value::clorus_is_truthy(pairwise(prev, cur)) == 0 {
            return Value::boolean(false);
        }
        prev = cur;
    }
    Value::boolean(true)
}

#[no_mangle]
pub extern "C" fn clorus_eq_variadic(args: *mut Value, _env: *mut i8) -> *mut Value {
    unsafe { chain_compare(args, crate::value::clorus_eq) }
}

#[no_mangle]
pub extern "C" fn clorus_lt_variadic(args: *mut Value, _env: *mut i8) -> *mut Value {
    unsafe { chain_compare(args, clorus_lt) }
}

#[no_mangle]
pub extern "C" fn clorus_gt_variadic(args: *mut Value, _env: *mut i8) -> *mut Value {
    unsafe { chain_compare(args, clorus_gt) }
}

#[no_mangle]
pub extern "C" fn clorus_lte_variadic(args: *mut Value, _env: *mut i8) -> *mut Value {
    unsafe { chain_compare(args, clorus_lte) }
}

#[no_mangle]
pub extern "C" fn clorus_gte_variadic(args: *mut Value, _env: *mut i8) -> *mut Value {
    unsafe { chain_compare(args, clorus_gte) }
}

// ============================================================================
// Bitwise Operations (Long only)
// ============================================================================

/// Bitwise AND (Long only)
#[no_mangle]
pub extern "C" fn clorus_bit_and(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::nil();
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        if a_tag == ValueTag::Long && b_tag == ValueTag::Long {
            let result = (*a).as_long() & (*b).as_long();
            Value::long(result)
        } else {
            eprintln!("Type error: Bitwise operations require Long integers");
            Value::nil()
        }
    }
}

/// Bitwise OR (Long only)
#[no_mangle]
pub extern "C" fn clorus_bit_or(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::nil();
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        if a_tag == ValueTag::Long && b_tag == ValueTag::Long {
            let result = (*a).as_long() | (*b).as_long();
            Value::long(result)
        } else {
            eprintln!("Type error: Bitwise operations require Long integers");
            Value::nil()
        }
    }
}

/// Bitwise XOR (Long only)
#[no_mangle]
pub extern "C" fn clorus_bit_xor(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::nil();
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        if a_tag == ValueTag::Long && b_tag == ValueTag::Long {
            let result = (*a).as_long() ^ (*b).as_long();
            Value::long(result)
        } else {
            eprintln!("Type error: Bitwise operations require Long integers");
            Value::nil()
        }
    }
}

/// Bitwise NOT (Long only)
#[no_mangle]
pub extern "C" fn clorus_bit_not(a: *mut Value) -> *mut Value {
    if a.is_null() {
        return Value::nil();
    }

    unsafe {
        if (*a).header().tag() == ValueTag::Long {
            let result = !(*a).as_long();
            Value::long(result)
        } else {
            eprintln!("Type error: Bitwise operations require Long integers");
            Value::nil()
        }
    }
}

/// Bitwise left shift (Long only)
#[no_mangle]
pub extern "C" fn clorus_bit_shift_left(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::nil();
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        if a_tag == ValueTag::Long && b_tag == ValueTag::Long {
            let shift = (*b).as_long();
            if shift < 0 || shift >= 64 {
                eprintln!("Shift amount out of range: {}", shift);
                return Value::nil();
            }
            let result = (*a).as_long() << shift;
            Value::long(result)
        } else {
            eprintln!("Type error: Bitwise operations require Long integers");
            Value::nil()
        }
    }
}

/// Bitwise right shift (Long only)
#[no_mangle]
pub extern "C" fn clorus_bit_shift_right(a: *mut Value, b: *mut Value) -> *mut Value {
    if a.is_null() || b.is_null() {
        return Value::nil();
    }

    unsafe {
        let a_tag = (*a).header().tag();
        let b_tag = (*b).header().tag();

        if a_tag == ValueTag::Long && b_tag == ValueTag::Long {
            let shift = (*b).as_long();
            if shift < 0 || shift >= 64 {
                eprintln!("Shift amount out of range: {}", shift);
                return Value::nil();
            }
            let result = (*a).as_long() >> shift;
            Value::long(result)
        } else {
            eprintln!("Type error: Bitwise operations require Long integers");
            Value::nil()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_addition() {
        let a = Value::long(42);
        let b = Value::long(10);
        let result = clorus_add(a, b);

        unsafe {
            assert_eq!((*result).header().tag(), ValueTag::Long);
            assert_eq!((*result).as_long(), 52);
        }

        unsafe {
            crate::value::clorus_release(a);
            crate::value::clorus_release(b);
            crate::value::clorus_release(result);
        }
    }

    #[test]
    fn test_mixed_addition() {
        let a = Value::long(42);
        let b = Value::double(3.14);
        let result = clorus_add(a, b);

        unsafe {
            assert_eq!((*result).header().tag(), ValueTag::Double);
            assert!(((*result).as_double() - 45.14).abs() < 0.0001);
        }

        unsafe {
            crate::value::clorus_release(a);
            crate::value::clorus_release(b);
            crate::value::clorus_release(result);
        }
    }

    #[test]
    fn test_bitwise_and() {
        let a = Value::long(0b1100);
        let b = Value::long(0b1010);
        let result = clorus_bit_and(a, b);

        unsafe {
            assert_eq!((*result).header().tag(), ValueTag::Long);
            assert_eq!((*result).as_long(), 0b1000);
        }

        unsafe {
            crate::value::clorus_release(a);
            crate::value::clorus_release(b);
            crate::value::clorus_release(result);
        }
    }
}
