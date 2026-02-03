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

/// Modulo operation (remainder)
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
                let result = (*a).as_long() % divisor;
                Value::long(result)
            }
            // Long % Double → Double
            (ValueTag::Long, ValueTag::Double) => {
                let result = ((*a).as_long() as f64) % (*b).as_double();
                Value::double(result)
            }
            // Double % Long → Double
            (ValueTag::Double, ValueTag::Long) => {
                let result = (*a).as_double() % ((*b).as_long() as f64);
                Value::double(result)
            }
            // Double % Double → Double
            (ValueTag::Double, ValueTag::Double) => {
                let result = (*a).as_double() % (*b).as_double();
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
