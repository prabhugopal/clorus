/// I/O Functions - Clojure-style file operations
///
/// Provides simple, Clojure-like functions for file I/O:
/// - `slurp` - read entire file
/// - `spit` - write to file

use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::c_char;

/// slurp - Read entire file to string
///
/// Clojure equivalent: (slurp "file.txt")
///
/// Example:
/// ```clojure
/// (use clorus.core)
/// (def content (slurp "data.txt"))
/// (println content)
/// ```
///
/// Returns: String content of file (as C string pointer)
/// Returns NULL if file cannot be read
#[no_mangle]
pub extern "C" fn clorus_slurp(path: *const c_char) -> *mut c_char {
    unsafe {
        // Convert C string to Rust string
        if path.is_null() {
            eprintln!("Error: slurp received null path");
            return std::ptr::null_mut();
        }

        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: Invalid UTF-8 in path: {}", e);
                return std::ptr::null_mut();
            }
        };

        // Expand ~ to home directory
        let expanded_path = if path_str.starts_with("~/") {
            match std::env::var("HOME") {
                Ok(home) => format!("{}/{}", home, &path_str[2..]),
                Err(_) => path_str.to_string(),
            }
        } else {
            path_str.to_string()
        };

        // Read the file
        match fs::read_to_string(&expanded_path) {
            Ok(content) => {
                // Convert back to C string
                match CString::new(content) {
                    Ok(c_str) => c_str.into_raw(),
                    Err(e) => {
                        eprintln!("Error: File contains null bytes: {}", e);
                        std::ptr::null_mut()
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading {}: {}", expanded_path, e);
                std::ptr::null_mut()
            }
        }
    }
}

/// spit - Write string to file
///
/// Clojure equivalent: (spit "file.txt" "content")
///
/// Example:
/// ```clojure
/// (use clorus.core)
/// (spit "output.txt" "Hello, World!")
/// (spit "data.txt" (str "Count: " 42))
/// ```
///
/// Returns: 1 on success, 0 on failure
#[no_mangle]
pub extern "C" fn clorus_spit(path: *const c_char, content: *const c_char) -> i32 {
    unsafe {
        // Convert C strings to Rust strings
        if path.is_null() {
            eprintln!("Error: spit received null path");
            return 0;
        }

        if content.is_null() {
            eprintln!("Error: spit received null content");
            return 0;
        }

        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: Invalid UTF-8 in path: {}", e);
                return 0;
            }
        };

        let content_str = match CStr::from_ptr(content).to_str() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: Invalid UTF-8 in content: {}", e);
                return 0;
            }
        };

        // Expand ~ to home directory
        let expanded_path = if path_str.starts_with("~/") {
            match std::env::var("HOME") {
                Ok(home) => format!("{}/{}", home, &path_str[2..]),
                Err(_) => path_str.to_string(),
            }
        } else {
            path_str.to_string()
        };

        // Write the file
        match fs::write(&expanded_path, content_str) {
            Ok(_) => 1, // Success
            Err(e) => {
                eprintln!("Error writing {}: {}", expanded_path, e);
                0 // Failure
            }
        }
    }
}

/// Free string returned by slurp
///
/// Important: Must call this to free memory allocated by slurp
#[no_mangle]
pub extern "C" fn clorus_free_string(s: *mut c_char) {
    unsafe {
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_spit_and_slurp() {
        let path = CString::new("test_io.txt").unwrap();
        let content = CString::new("Hello from Clorus!").unwrap();

        // Test spit (write)
        let result = clorus_spit(path.as_ptr(), content.as_ptr());
        assert_eq!(result, 1, "spit should succeed");

        // Test slurp (read)
        let read_content = clorus_slurp(path.as_ptr());
        assert!(!read_content.is_null(), "slurp should return content");

        unsafe {
            let read_str = CStr::from_ptr(read_content).to_str().unwrap();
            assert_eq!(read_str, "Hello from Clorus!");
            clorus_free_string(read_content);
        }

        // Cleanup
        std::fs::remove_file("test_io.txt").ok();
    }

    #[test]
    fn test_slurp_nonexistent() {
        let path = CString::new("nonexistent_file_12345.txt").unwrap();
        let result = clorus_slurp(path.as_ptr());
        assert!(result.is_null(), "slurp should return NULL for nonexistent file");
    }

    #[test]
    fn test_spit_invalid_path() {
        let path = CString::new("/root/forbidden/file.txt").unwrap();
        let content = CString::new("content").unwrap();
        let result = clorus_spit(path.as_ptr(), content.as_ptr());
        assert_eq!(result, 0, "spit should fail for invalid path");
    }
}
