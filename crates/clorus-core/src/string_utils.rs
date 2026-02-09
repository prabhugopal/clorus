/// String utility functions for Clorus that avoid allocations
/// These functions operate on strings without creating temporary substrings

use clorus_runtime::value::{Value, clorus_value_long, clorus_value_boolean};

/// Count the number of newlines in a string
/// Returns the count as a Clorus Long value
#[no_mangle]
pub extern "C" fn clorus_string_count_newlines(string_val: *mut Value) -> *mut Value {
    unsafe {
        // Get the string content using as_string()
        let rust_str = (*string_val).as_string();

        // Count newlines without allocating
        let count = rust_str.chars().filter(|&c| c == '\n').count() as i64;

        clorus_value_long(count)
    }
}

/// Find the index of the nth newline in a string (0-indexed)
/// Returns -1 if there aren't that many newlines
/// Args: string_val, n (Long value)
#[no_mangle]
pub extern "C" fn clorus_string_find_nth_newline(string_val: *mut Value, n_val: *mut Value) -> *mut Value {
    unsafe {
        // Get n
        let n = (*n_val).as_long() as usize;

        // Get the string content
        let rust_str = (*string_val).as_string();

        // Find the nth newline
        let mut count = 0;
        for (idx, ch) in rust_str.char_indices() {
            if ch == '\n' {
                if count == n {
                    return clorus_value_long(idx as i64);
                }
                count += 1;
            }
        }

        // Not found
        clorus_value_long(-1)
    }
}

/// Extract a line from a string by line number (0-indexed)
/// This creates ONE new string (the line) instead of splitting the entire text
/// Args: string_val, line_number (Long value)
#[no_mangle]
pub extern "C" fn clorus_string_get_line(string_val: *mut Value, line_num_val: *mut Value) -> *mut Value {
    unsafe {
        // Get line number
        let target_line = (*line_num_val).as_long();
        if target_line < 0 {
            return Value::string("");
        }

        // Get the string content
        let rust_str = (*string_val).as_string();

        // Find the line without allocating intermediate strings
        let mut current_line = 0i64;
        let mut line_start = 0;

        for (idx, ch) in rust_str.char_indices() {
            if ch == '\n' {
                if current_line == target_line {
                    // Found the line, extract it
                    let line = &rust_str[line_start..idx];
                    return Value::string(line);
                }
                current_line += 1;
                line_start = idx + 1;
            }
        }

        // Handle last line (no trailing newline)
        if current_line == target_line {
            let line = &rust_str[line_start..];
            return Value::string(line);
        }

        // Line not found
        Value::string("")
    }
}

/// Split a string into lines (returns a vector of strings)
/// This is more efficient than repeated subs calls
/// Args: string_val
#[no_mangle]
pub extern "C" fn clorus_string_split_lines(string_val: *mut Value) -> *mut Value {
    unsafe {
        // Get the string content
        let rust_str = (*string_val).as_string();

        // Split into lines
        let lines: Vec<&str> = rust_str.split('\n').collect();

        // Create vector of string Values
        let mut result = clorus_runtime::vector::clorus_vector_empty();
        for line in lines {
            let line_val = Value::string(line);
            result = clorus_runtime::vector::clorus_vector_conj(result, line_val);
        }

        result
    }
}

/// Check if character at position is a newline
/// Args: string_val, pos (Long value)
/// Returns true/false as Bool value
#[no_mangle]
pub extern "C" fn clorus_string_is_newline_at(string_val: *mut Value, pos_val: *mut Value) -> *mut Value {
    unsafe {
        // Get position
        let pos = (*pos_val).as_long();
        if pos < 0 {
            return clorus_value_boolean(false);
        }

        // Get the string content
        let rust_str = (*string_val).as_string();

        // Check if character at position is newline
        if let Some(ch) = rust_str.chars().nth(pos as usize) {
            clorus_value_boolean(ch == '\n')
        } else {
            clorus_value_boolean(false)
        }
    }
}
