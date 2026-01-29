/// Core Value type for Clorus runtime
///
/// All values in Clorus are represented as tagged pointers with reference counting.
/// This enables efficient persistent data structures with structural sharing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::fmt;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Type tags for different kinds of values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueTag {
    Number = 0,
    List = 1,
    Vector = 2,
    HashMap = 3,
    String = 4,
    Keyword = 5,
    Symbol = 6,
    Bool = 7,
    Nil = 8,
    HashSet = 9,
    Atom = 10,
    Ref = 11,
    Agent = 12,
    Channel = 13,
}

/// Header for all heap-allocated values
///
/// Contains reference count and type tag.
/// Reference count uses atomic operations for thread safety.
#[repr(C)]
pub struct Header {
    /// Reference count - number of pointers to this value
    refcount: AtomicU64,
    /// Type tag identifying what kind of value this is
    tag: ValueTag,
}

impl Header {
    pub fn new(tag: ValueTag) -> Self {
        Header {
            refcount: AtomicU64::new(1), // Start with refcount = 1
            tag,
        }
    }

    #[inline]
    pub fn retain(&self) {
        self.refcount.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn release(&self) -> bool {
        // Returns true if this was the last reference
        self.refcount.fetch_sub(1, Ordering::Relaxed) == 1
    }

    #[inline]
    pub fn refcount(&self) -> u64 {
        self.refcount.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn tag(&self) -> ValueTag {
        self.tag
    }
}

/// Value representation
///
/// Small values (numbers, bools, nil) are stored inline.
/// Large values (lists, vectors, maps) are stored as pointers to heap.
#[repr(C)]
pub union ValueData {
    /// Inline number value
    number: f64,
    /// Inline boolean (0.0 = false, 1.0 = true)
    boolean: f64,
    /// Pointer to heap-allocated data
    ptr: *mut u8,
}

/// A Clorus value
///
/// This is the type that LLVM code manipulates.
/// It's designed to be FFI-safe and passed by value.
#[repr(C)]
pub struct Value {
    /// Value header (refcount + tag)
    header: Header,
    /// Value data (inline or pointer)
    data: ValueData,
}

impl Value {
    /// Create a number value
    pub fn number(n: f64) -> *mut Self {
        let val = Box::new(Value {
            header: Header::new(ValueTag::Number),
            data: ValueData { number: n },
        });
        Box::into_raw(val)
    }

    /// Create a boolean value
    pub fn boolean(b: bool) -> *mut Self {
        let val = Box::new(Value {
            header: Header::new(ValueTag::Bool),
            data: ValueData {
                boolean: if b { 1.0 } else { 0.0 },
            },
        });
        Box::into_raw(val)
    }

    /// Create a nil value
    pub fn nil() -> *mut Self {
        let val = Box::new(Value {
            header: Header::new(ValueTag::Nil),
            data: ValueData { number: 0.0 },
        });
        Box::into_raw(val)
    }

    /// Create a string value
    pub fn string(s: &str) -> *mut Self {
        let boxed_string = Box::new(s.to_string());
        let val = Box::new(Value {
            header: Header::new(ValueTag::String),
            data: ValueData {
                ptr: Box::into_raw(boxed_string) as *mut u8,
            },
        });
        Box::into_raw(val)
    }

    /// Create a keyword value
    /// Note: Keywords should be interned via keyword::intern_keyword for efficiency
    pub fn keyword(s: &str) -> *mut Self {
        let boxed_string = Box::new(s.to_string());
        let val = Box::new(Value {
            header: Header::new(ValueTag::Keyword),
            data: ValueData {
                ptr: Box::into_raw(boxed_string) as *mut u8,
            },
        });
        Box::into_raw(val)
    }

    /// Create a value wrapping a heap pointer
    pub fn from_ptr(tag: ValueTag, ptr: *mut u8) -> *mut Self {
        let val = Box::new(Value {
            header: Header::new(tag),
            data: ValueData { ptr },
        });
        Box::into_raw(val)
    }

    /// Get the number value (unsafe - caller must ensure tag is Number)
    pub unsafe fn as_number(&self) -> f64 {
        self.data.number
    }

    /// Get the boolean value (unsafe - caller must ensure tag is Bool)
    pub unsafe fn as_bool(&self) -> bool {
        self.data.boolean != 0.0
    }

    /// Get the string value (unsafe - caller must ensure tag is String)
    pub unsafe fn as_string(&self) -> &str {
        let ptr = self.data.ptr as *const String;
        &*ptr
    }

    /// Get the keyword value (unsafe - caller must ensure tag is Keyword)
    pub unsafe fn as_keyword(&self) -> &str {
        let ptr = self.data.ptr as *const String;
        &*ptr
    }

    /// Get the pointer value (unsafe - caller must ensure tag is not Number/Bool/Nil)
    pub unsafe fn as_ptr(&self) -> *mut u8 {
        self.data.ptr
    }

    /// Get the header
    #[inline]
    pub fn header(&self) -> &Header {
        &self.header
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            match self.header.tag() {
                ValueTag::Number => write!(f, "Number({})", self.as_number()),
                ValueTag::Bool => write!(f, "Bool({})", self.as_bool()),
                ValueTag::Nil => write!(f, "Nil"),
                ValueTag::String => write!(f, "String(\"{}\")", self.as_string()),
                ValueTag::Keyword => write!(f, "Keyword(:{})", self.as_keyword()),
                _ => write!(
                    f,
                    "{:?}({:p})",
                    self.header.tag(),
                    self.as_ptr()
                ),
            }
        }
    }
}

// FFI functions for LLVM-generated code

/// Increment reference count
#[no_mangle]
pub extern "C" fn clorus_retain(val: *mut Value) {
    if val.is_null() {
        return;
    }
    unsafe {
        (*val).header.retain();
    }
}

/// Decrement reference count and free if zero
#[no_mangle]
pub extern "C" fn clorus_release(val: *mut Value) {
    if val.is_null() {
        return;
    }

    unsafe {
        if (*val).header.release() {
            // Last reference - deallocate
            deallocate_value(val);
        }
    }
}

/// Get reference count (for debugging)
#[no_mangle]
pub extern "C" fn clorus_refcount(val: *const Value) -> u64 {
    if val.is_null() {
        return 0;
    }
    unsafe { (*val).header.refcount() }
}

/// Create a Value from f64 (for LLVM codegen)
#[no_mangle]
pub extern "C" fn clorus_value_number(n: f64) -> *mut Value {
    Value::number(n)
}

/// Create a nil value (for LLVM codegen)
#[no_mangle]
pub extern "C" fn clorus_value_nil() -> *mut Value {
    Value::nil()
}

/// Create a bool value (for LLVM codegen)
#[no_mangle]
pub extern "C" fn clorus_value_bool(b: f64) -> *mut Value {
    Value::boolean(b != 0.0)
}

/// Extract f64 from Value (for LLVM codegen)
/// Returns 0.0 if the value is not a number
#[no_mangle]
pub extern "C" fn clorus_value_as_number(val: *mut Value) -> f64 {
    if val.is_null() {
        return 0.0;
    }
    unsafe {
        if (*val).header.tag() == ValueTag::Number {
            (*val).as_number()
        } else {
            0.0
        }
    }
}

/// Create a Value from a C string (for LLVM codegen)
/// Takes ownership of the string - the caller is responsible for freeing the input c_char*
#[no_mangle]
pub extern "C" fn clorus_value_string(ptr: *const c_char) -> *mut Value {
    if ptr.is_null() {
        return Value::nil();
    }
    unsafe {
        let c_str = CStr::from_ptr(ptr);
        match c_str.to_str() {
            Ok(s) => Value::string(s),
            Err(_) => Value::nil(), // Return nil on invalid UTF-8
        }
    }
}

/// Extract a C string from Value (for LLVM codegen)
/// Returns a newly allocated C string that the caller must free with clorus_free_cstring
/// Returns null if the value is not a string
#[no_mangle]
pub extern "C" fn clorus_value_as_cstring(val: *mut Value) -> *mut c_char {
    if val.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        if (*val).header.tag() == ValueTag::String {
            let s = (*val).as_string();
            match CString::new(s) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        } else {
            std::ptr::null_mut()
        }
    }
}

/// Free a C string returned by clorus_value_as_cstring
#[no_mangle]
pub extern "C" fn clorus_free_cstring(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Check if a value is nil
#[no_mangle]
pub extern "C" fn clorus_value_is_nil(val: *mut Value) -> i32 {
    if val.is_null() {
        return 1; // null pointer is considered nil
    }

    unsafe {
        if (*val).header.tag() == ValueTag::Nil {
            1
        } else {
            0
        }
    }
}

/// Deallocate a value and its contents
unsafe fn deallocate_value(val: *mut Value) {
    match (*val).header.tag() {
        ValueTag::Number | ValueTag::Bool | ValueTag::Nil => {
            // Just free the Value itself
            drop(Box::from_raw(val));
        }
        ValueTag::List => {
            // Release list and free Value
            let ptr = (*val).as_ptr() as *mut crate::list::ListNode;
            if !ptr.is_null() {
                crate::list::release_list(ptr);
            }
            drop(Box::from_raw(val));
        }
        ValueTag::Vector => {
            // Release vector and free Value
            let ptr = (*val).as_ptr() as *mut crate::vector::PersistentVector;
            if !ptr.is_null() {
                crate::vector::release_vector(ptr);
            }
            drop(Box::from_raw(val));
        }
        ValueTag::String => {
            // Free the String and the Value
            let ptr = (*val).as_ptr() as *mut String;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            drop(Box::from_raw(val));
        }
        ValueTag::HashMap => {
            // Release hash map and free Value
            let ptr = (*val).as_ptr() as *mut crate::map::ClorusHashMap;
            if !ptr.is_null() {
                crate::map::release_map(ptr);
            }
            drop(Box::from_raw(val));
        }
        ValueTag::HashSet => {
            // Release hash set and free Value
            let ptr = (*val).as_ptr() as *mut crate::set::ClorusHashSet;
            if !ptr.is_null() {
                crate::set::release_set(ptr);
            }
            drop(Box::from_raw(val));
        }
        ValueTag::Atom => {
            // Release atom and free Value
            let ptr = (*val).as_ptr() as *mut crate::atom::ClorusAtom;
            if !ptr.is_null() {
                crate::atom::release_atom(ptr);
            }
            drop(Box::from_raw(val));
        }
        ValueTag::Ref => {
            // Release ref and free Value
            let ptr = (*val).as_ptr() as *mut crate::ref_type::ClorusRef;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            drop(Box::from_raw(val));
        }
        ValueTag::Agent => {
            // Release agent and free Value
            let ptr = (*val).as_ptr() as *mut crate::agent::ClorusAgent;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            drop(Box::from_raw(val));
        }
        ValueTag::Channel => {
            // Release channel and free Value
            let ptr = (*val).as_ptr() as *mut crate::channel::ClorusChannel;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
            drop(Box::from_raw(val));
        }
        ValueTag::Keyword => {
            // Keywords are interned and never deallocated
            // They live for the program lifetime
            // Just drop the Value wrapper (the String stays alive in the intern table)
            // NOTE: This assumes keywords are always created via intern_keyword
            drop(Box::from_raw(val));
        }
        ValueTag::Symbol => {
            // TODO: Implement when we add symbol type
            // For now, just free the Value
            drop(Box::from_raw(val));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_value() {
        let val = Value::number(42.0);
        unsafe {
            assert_eq!((*val).header.tag(), ValueTag::Number);
            assert_eq!((*val).as_number(), 42.0);
            assert_eq!((*val).header.refcount(), 1);
        }
        // Clean up
        unsafe { drop(Box::from_raw(val)); }
    }

    #[test]
    fn test_refcounting() {
        let val = Value::number(3.14);

        // Initial refcount
        assert_eq!(unsafe { (*val).header.refcount() }, 1);

        // Retain
        clorus_retain(val);
        assert_eq!(unsafe { (*val).header.refcount() }, 2);

        clorus_retain(val);
        assert_eq!(unsafe { (*val).header.refcount() }, 3);

        // Release
        clorus_release(val);
        assert_eq!(unsafe { (*val).header.refcount() }, 2);

        clorus_release(val);
        assert_eq!(unsafe { (*val).header.refcount() }, 1);

        // Final release (will deallocate)
        clorus_release(val);
        // Value is now freed - can't check refcount
    }

    #[test]
    fn test_boolean() {
        let val_true = Value::boolean(true);
        let val_false = Value::boolean(false);

        unsafe {
            assert_eq!((*val_true).header.tag(), ValueTag::Bool);
            assert_eq!((*val_true).as_bool(), true);

            assert_eq!((*val_false).header.tag(), ValueTag::Bool);
            assert_eq!((*val_false).as_bool(), false);

            // Clean up
            drop(Box::from_raw(val_true));
            drop(Box::from_raw(val_false));
        }
    }

    #[test]
    fn test_string_value() {
        let val = Value::string("Hello, Clorus!");
        unsafe {
            assert_eq!((*val).header.tag(), ValueTag::String);
            assert_eq!((*val).as_string(), "Hello, Clorus!");
            assert_eq!((*val).header.refcount(), 1);
        }
        // Clean up - will call deallocate_value which handles String
        clorus_release(val);
    }

    #[test]
    fn test_string_ffi() {
        use std::ffi::CString;

        let c_str = CString::new("Test string").unwrap();
        let val = clorus_value_string(c_str.as_ptr());

        unsafe {
            assert_eq!((*val).header.tag(), ValueTag::String);
            assert_eq!((*val).as_string(), "Test string");
        }

        // Extract as C string
        let extracted = clorus_value_as_cstring(val);
        assert!(!extracted.is_null());

        unsafe {
            let extracted_str = CStr::from_ptr(extracted);
            assert_eq!(extracted_str.to_str().unwrap(), "Test string");
        }

        // Clean up
        clorus_free_cstring(extracted);
        clorus_release(val);
    }

    #[test]
    fn test_string_refcounting() {
        let val = Value::string("Refcounted string");

        // Initial refcount
        assert_eq!(unsafe { (*val).header.refcount() }, 1);

        // Retain
        clorus_retain(val);
        assert_eq!(unsafe { (*val).header.refcount() }, 2);

        // Release
        clorus_release(val);
        assert_eq!(unsafe { (*val).header.refcount() }, 1);

        // Final release (will deallocate)
        clorus_release(val);
    }
}
