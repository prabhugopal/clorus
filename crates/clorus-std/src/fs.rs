/// File system operations wrapper for Clorus
///
/// Provides simple, Clojure-style file I/O:
/// - (fs/read path) - Read file to string
/// - (fs/write path content) - Write string to file
/// - (fs/exists? path) - Check if path exists
/// - (fs/remove path) - Delete file

use std::fs;
use std::path::Path;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// String representation for return values
// null pointer = error/nil
// non-null = success with data

/// Read entire file to string
///
/// Returns: String content or null on error
#[no_mangle]
pub extern "C" fn clorus_fs_read(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };

        match fs::read_to_string(path_str) {
            Ok(content) => {
                match CString::new(content) {
                    Ok(c_str) => c_str.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                }
            }
            Err(_) => std::ptr::null_mut(),
        }
    }
}

/// Write string to file (overwrites existing)
///
/// Returns: 1 on success, 0 on error
#[no_mangle]
pub extern "C" fn clorus_fs_write(
    path: *const c_char,
    content: *const c_char,
) -> i32 {
    if path.is_null() || content.is_null() {
        return 0;
    }

    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        let content_str = match CStr::from_ptr(content).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        match fs::write(path_str, content_str) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

/// Append string to file
///
/// Returns: 1 on success, 0 on error
#[no_mangle]
pub extern "C" fn clorus_fs_append(
    path: *const c_char,
    content: *const c_char,
) -> i32 {
    if path.is_null() || content.is_null() {
        return 0;
    }

    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        let content_str = match CStr::from_ptr(content).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        use std::fs::OpenOptions;
        use std::io::Write;

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(path_str)
        {
            Ok(mut file) => match file.write_all(content_str.as_bytes()) {
                Ok(_) => 1,
                Err(_) => 0,
            },
            Err(_) => 0,
        }
    }
}

/// Check if path exists
///
/// Returns: 1 if exists, 0 if not
#[no_mangle]
pub extern "C" fn clorus_fs_exists(path: *const c_char) -> i32 {
    if path.is_null() {
        return 0;
    }

    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        if Path::new(path_str).exists() { 1 } else { 0 }
    }
}

/// Check if path is a file
///
/// Returns: 1 if file, 0 if not
#[no_mangle]
pub extern "C" fn clorus_fs_is_file(path: *const c_char) -> i32 {
    if path.is_null() {
        return 0;
    }

    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        if Path::new(path_str).is_file() { 1 } else { 0 }
    }
}

/// Check if path is a directory
///
/// Returns: 1 if directory, 0 if not
#[no_mangle]
pub extern "C" fn clorus_fs_is_dir(path: *const c_char) -> i32 {
    if path.is_null() {
        return 0;
    }

    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        if Path::new(path_str).is_dir() { 1 } else { 0 }
    }
}

/// Delete a file or empty directory
///
/// Returns: 1 on success, 0 on error
#[no_mangle]
pub extern "C" fn clorus_fs_remove(path: *const c_char) -> i32 {
    if path.is_null() {
        return 0;
    }

    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        let path_obj = Path::new(path_str);

        if path_obj.is_file() {
            match fs::remove_file(path_str) {
                Ok(_) => 1,
                Err(_) => 0,
            }
        } else if path_obj.is_dir() {
            match fs::remove_dir(path_str) {
                Ok(_) => 1,
                Err(_) => 0,
            }
        } else {
            0
        }
    }
}

/// Copy a file
///
/// Returns: 1 on success, 0 on error
#[no_mangle]
pub extern "C" fn clorus_fs_copy(
    src: *const c_char,
    dst: *const c_char,
) -> i32 {
    if src.is_null() || dst.is_null() {
        return 0;
    }

    unsafe {
        let src_str = match CStr::from_ptr(src).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        let dst_str = match CStr::from_ptr(dst).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        match fs::copy(src_str, dst_str) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

/// Rename/move a file
///
/// Returns: 1 on success, 0 on error
#[no_mangle]
pub extern "C" fn clorus_fs_rename(
    old: *const c_char,
    new: *const c_char,
) -> i32 {
    if old.is_null() || new.is_null() {
        return 0;
    }

    unsafe {
        let old_str = match CStr::from_ptr(old).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        let new_str = match CStr::from_ptr(new).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        match fs::rename(old_str, new_str) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

/// Create a directory
///
/// Returns: 1 on success, 0 on error
#[no_mangle]
pub extern "C" fn clorus_fs_create_dir(path: *const c_char) -> i32 {
    if path.is_null() {
        return 0;
    }

    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        match fs::create_dir(path_str) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

/// Create a directory and all parent directories
///
/// Returns: 1 on success, 0 on error
#[no_mangle]
pub extern "C" fn clorus_fs_create_dir_all(path: *const c_char) -> i32 {
    if path.is_null() {
        return 0;
    }

    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        match fs::create_dir_all(path_str) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

/// Free a string returned by fs operations
#[no_mangle]
pub extern "C" fn clorus_fs_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_fs_write_read() {
        let path = CString::new("/tmp/clorus_test.txt").unwrap();
        let content = CString::new("Hello from Clorus!").unwrap();

        // Write
        let result = clorus_fs_write(path.as_ptr(), content.as_ptr());
        assert_eq!(result, 1);

        // Read
        let read_ptr = clorus_fs_read(path.as_ptr());
        assert!(!read_ptr.is_null());

        unsafe {
            let read_content = CStr::from_ptr(read_ptr).to_string_lossy();
            assert_eq!(read_content, "Hello from Clorus!");
            clorus_fs_free_string(read_ptr);
        }

        // Clean up
        clorus_fs_remove(path.as_ptr());
    }

    #[test]
    fn test_fs_exists() {
        let path = CString::new("/tmp/clorus_exists_test.txt").unwrap();

        // Should not exist initially
        assert_eq!(clorus_fs_exists(path.as_ptr()), 0);

        // Create file
        let content = CString::new("test").unwrap();
        clorus_fs_write(path.as_ptr(), content.as_ptr());

        // Should exist now
        assert_eq!(clorus_fs_exists(path.as_ptr()), 1);

        // Is a file
        assert_eq!(clorus_fs_is_file(path.as_ptr()), 1);
        assert_eq!(clorus_fs_is_dir(path.as_ptr()), 0);

        // Clean up
        clorus_fs_remove(path.as_ptr());
    }

    #[test]
    fn test_fs_append() {
        let path = CString::new("/tmp/clorus_append_test.txt").unwrap();
        let content1 = CString::new("Line 1\n").unwrap();
        let content2 = CString::new("Line 2\n").unwrap();

        // Write first line
        clorus_fs_write(path.as_ptr(), content1.as_ptr());

        // Append second line
        clorus_fs_append(path.as_ptr(), content2.as_ptr());

        // Read and verify
        let read_ptr = clorus_fs_read(path.as_ptr());
        unsafe {
            let content = CStr::from_ptr(read_ptr).to_string_lossy();
            assert_eq!(content, "Line 1\nLine 2\n");
            clorus_fs_free_string(read_ptr);
        }

        // Clean up
        clorus_fs_remove(path.as_ptr());
    }
}
