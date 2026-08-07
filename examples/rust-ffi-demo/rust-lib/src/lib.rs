/// Example Rust library for testing automatic FFI generation
///
/// These functions will be automatically wrapped for Clorus

/// Simple arithmetic function
pub fn add(x: f64, y: f64) -> f64 {
    x + y
}

/// Multiply two numbers
pub fn multiply(a: f64, b: f64) -> f64 {
    a * b
}

/// Calculate factorial
pub fn factorial(n: f64) -> f64 {
    if n <= 1.0 {
        1.0
    } else {
        n * factorial(n - 1.0)
    }
}

/// String greeting function
pub fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

/// Convert string to uppercase
pub fn to_upper(s: String) -> String {
    s.to_uppercase()
}

/// Calculate the length of a string
pub fn string_length(s: String) -> f64 {
    s.len() as f64
}

/// Identity on bool -- regression coverage for FFI bool marshaling.
pub fn negate(b: bool) -> bool {
    !b
}

/// Named struct behind a pointer, used to test Option<*mut T>/Result<*mut T, E>.
pub struct Counter {
    pub value: i64,
}

/// Returns None for a negative start, Some(handle) otherwise.
pub fn make_counter(start: i64) -> Option<*mut Counter> {
    if start < 0 {
        None
    } else {
        Some(Box::into_raw(Box::new(Counter { value: start })))
    }
}

pub fn counter_value(c: *mut Counter) -> f64 {
    unsafe { (*c).value as f64 }
}

/// Ok(handle) unless refused is true, matching Coral's "Result/error
/// transport" gate: the Err message becomes a genuine Clorus exception.
pub fn make_counter_checked(start: i64, refused: bool) -> Result<*mut Counter, String> {
    if refused {
        Err(format!("counter creation refused for start={}", start))
    } else {
        Ok(Box::into_raw(Box::new(Counter { value: start })))
    }
}

// Private function - should NOT be wrapped
fn internal_helper(x: f64) -> f64 {
    x * 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2.0, 3.0), 5.0);
    }

    #[test]
    fn test_greet() {
        assert_eq!(greet("Alice".to_string()), "Hello, Alice!");
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(5.0), 120.0);
    }

    #[test]
    fn test_negate() {
        assert_eq!(negate(true), false);
        assert_eq!(negate(false), true);
    }

    #[test]
    fn test_make_counter() {
        assert!(make_counter(-1).is_none());
        let handle = make_counter(5).expect("non-negative start should succeed");
        assert_eq!(counter_value(handle), 5.0);
    }

    #[test]
    fn test_make_counter_checked() {
        assert!(make_counter_checked(0, true).is_err());
        let handle = make_counter_checked(3, false).expect("should succeed");
        assert_eq!(counter_value(handle), 3.0);
    }
}
