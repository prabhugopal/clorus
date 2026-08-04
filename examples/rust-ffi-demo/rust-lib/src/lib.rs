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
}
