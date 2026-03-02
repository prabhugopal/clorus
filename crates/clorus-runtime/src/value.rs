/// Core Value type for Clorus runtime
///
/// All values in Clorus are represented as tagged pointers with reference counting.
/// This enables efficient persistent data structures with structural sharing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::fmt;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::time::{SystemTime, UNIX_EPOCH};
use std::cell::Cell;

static RELEASE_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct FreedInfo {
    seq: u64,
    tag: ValueTag,
    ts_ms: u128,
    backtrace: Option<String>,
}

static FREED_PTRS: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();
static FREED_INFO: OnceLock<Mutex<HashMap<usize, FreedInfo>>> = OnceLock::new();
static FREED_ALLOC_INFO: OnceLock<Mutex<HashMap<usize, AllocInfo>>> = OnceLock::new();
static ALLOC_INFO: OnceLock<Mutex<HashMap<usize, AllocInfo>>> = OnceLock::new();
static VALUE_RC_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

thread_local! {
    static VALUE_RC_LOCK_DEPTH: Cell<u32> = const { Cell::new(0) };
}

struct ValueRcSection {
    _guard: Option<std::sync::MutexGuard<'static, ()>>,
}

impl ValueRcSection {
    fn enter() -> Self {
        let nested = VALUE_RC_LOCK_DEPTH.with(|depth| {
            let current = depth.get();
            depth.set(current.saturating_add(1));
            current > 0
        });

        if nested {
            Self { _guard: None }
        } else {
            let guard = VALUE_RC_LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .ok();
            Self { _guard: guard }
        }
    }
}

impl Drop for ValueRcSection {
    fn drop(&mut self) {
        VALUE_RC_LOCK_DEPTH.with(|depth| {
            depth.set(depth.get().saturating_sub(1));
        });
    }
}

#[derive(Clone)]
struct AllocInfo {
    tag: ValueTag,
    ts_ms: u128,
    backtrace: Option<String>,
}

/// Type tags for different kinds of values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueTag {
    Long = 0,      // i64 integer (NEW: primary integer type)
    Double = 1,    // f64 floating point (RENAMED from Number)
    List = 2,      // Shifted by 1
    Vector = 3,
    HashMap = 4,
    String = 5,
    Keyword = 6,
    Symbol = 7,
    Bool = 8,
    Nil = 9,
    HashSet = 10,
    Atom = 11,
    Ref = 12,
    Agent = 13,
    Channel = 14,
    Function = 15,
    Var = 16,  // Var for dynamic bindings (shifted by 1)
    MultiArityFunction = 17,  // Multi-arity function with runtime dispatch
    OpaquePointer = 18,  // FFI opaque pointer (window, etc.)
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
    /// Inline Long (i64) value
    long: i64,
    /// Inline Double (f64) value
    double: f64,
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
    /// Create a Long (i64) value
    pub fn long(n: i64) -> *mut Self {
        let val = Box::new(Value {
            header: Header::new(ValueTag::Long),
            data: ValueData { long: n },
        });
        let ptr = Box::into_raw(val);
        record_alloc(ptr, ValueTag::Long);
        ptr
    }

    /// Create a Double (f64) value
    pub fn double(n: f64) -> *mut Self {
        let val = Box::new(Value {
            header: Header::new(ValueTag::Double),
            data: ValueData { double: n },
        });
        let ptr = Box::into_raw(val);
        record_alloc(ptr, ValueTag::Double);
        ptr
    }

    /// Create a boolean value
    pub fn boolean(b: bool) -> *mut Self {
        let val = Box::new(Value {
            header: Header::new(ValueTag::Bool),
            data: ValueData {
                boolean: if b { 1.0 } else { 0.0 },
            },
        });
        let ptr = Box::into_raw(val);
        record_alloc(ptr, ValueTag::Bool);
        ptr
    }

    /// Create a nil value
    pub fn nil() -> *mut Self {
        let val = Box::new(Value {
            header: Header::new(ValueTag::Nil),
            data: ValueData { long: 0 },
        });
        let ptr = Box::into_raw(val);
        record_alloc(ptr, ValueTag::Nil);
        ptr
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
        let ptr = Box::into_raw(val);
        record_alloc(ptr, ValueTag::String);
        ptr
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
        let ptr = Box::into_raw(val);
        record_alloc(ptr, ValueTag::Keyword);
        ptr
    }

    /// Create a value wrapping a heap pointer
    pub fn from_ptr(tag: ValueTag, ptr: *mut u8) -> *mut Self {
        let val = Box::new(Value {
            header: Header::new(tag),
            data: ValueData { ptr },
        });
        let out = Box::into_raw(val);
        record_alloc(out, tag);
        out
    }

    /// Get the Long value (unsafe - caller must ensure tag is Long)
    pub unsafe fn as_long(&self) -> i64 {
        self.data.long
    }

    /// Get the Double value (unsafe - caller must ensure tag is Double)
    pub unsafe fn as_double(&self) -> f64 {
        self.data.double
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

    /// Create an opaque pointer value (for FFI opaque pointers like window handles)
    pub fn opaque_pointer(ptr: *mut u8) -> *mut Self {
        Self::from_ptr(ValueTag::OpaquePointer, ptr)
    }

    /// Get the opaque pointer value (unsafe - caller must ensure tag is OpaquePointer)
    pub unsafe fn as_opaque_pointer(&self) -> *mut u8 {
        self.data.ptr
    }

    /// Create a function value from FunctionData pointer
    pub fn from_function(func_data: *mut crate::function::FunctionData) -> *mut Self {
        Self::from_ptr(ValueTag::Function, func_data as *mut u8)
    }

    /// Get the function data (unsafe - caller must ensure tag is Function)
    pub unsafe fn as_function(&self) -> *mut crate::function::FunctionData {
        self.data.ptr as *mut crate::function::FunctionData
    }

    /// Create a multi-arity function value from MultiArityFunctionData pointer
    pub fn from_multi_arity_function(func_data: *mut crate::function::MultiArityFunctionData) -> *mut Self {
        Self::from_ptr(ValueTag::MultiArityFunction, func_data as *mut u8)
    }

    /// Get the multi-arity function data (unsafe - caller must ensure tag is MultiArityFunction)
    pub unsafe fn as_multi_arity_function(&self) -> *mut crate::function::MultiArityFunctionData {
        self.data.ptr as *mut crate::function::MultiArityFunctionData
    }

    /// Create a var value from Var pointer
    pub fn from_var(var_ptr: *mut crate::var::Var) -> *mut Self {
        Self::from_ptr(ValueTag::Var, var_ptr as *mut u8)
    }

    /// Get the var pointer (unsafe - caller must ensure tag is Var)
    pub unsafe fn as_var(&self) -> *mut crate::var::Var {
        self.data.ptr as *mut crate::var::Var
    }

    /// Get tag
    #[inline]
    pub fn tag(&self) -> ValueTag {
        self.header.tag()
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
                ValueTag::Long => write!(f, "Long({})", self.as_long()),
                ValueTag::Double => write!(f, "Double({})", self.as_double()),
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
    let _rc_section = ValueRcSection::enter();
    unsafe {
        (*val).header.retain();
    }

    // Optional debug tracing for a specific pointer or tag.
    // Use CLORUS_DEBUG_PTR (hex like 0x1234 or decimal) and/or CLORUS_DEBUG_TAG (e.g., "Vector").
    if std::env::var("CLORUS_DEBUG_PTR").is_ok() || std::env::var("CLORUS_DEBUG_TAG").is_ok() {
        if should_debug_value(val) {
            eprintln!("[clorus] retain ptr={:p} tag={:?} rc={}", val, unsafe { (*val).header.tag() }, unsafe { (*val).header.refcount() });
            if std::env::var("CLORUS_DEBUG_RELEASE_BT").is_ok() {
                let bt_now = std::backtrace::Backtrace::force_capture().to_string();
                eprintln!("[clorus] retain backtrace:\n{}", bt_now);
            }
        }
    }
}

/// Decrement reference count and free if zero
#[no_mangle]
pub extern "C" fn clorus_release(val: *mut Value) {
    if val.is_null() {
        return;
    }

    if std::env::var("CLORUS_SAFE_VALUE").is_ok() {
        // Debug safety valve: skip all deallocation to avoid UAF/double-free crashes.
        // This leaks memory but keeps the process alive for debugging.
        return;
    }

    // Serialize retain/release critical sections while allowing re-entrant
    // release() calls on the same thread during recursive deallocation.
    let _rc_section = ValueRcSection::enter();

    let debug_release = std::env::var("CLORUS_DEBUG_RELEASE").is_ok();
    let ptr = val as usize;

    // Always guard against releasing an already-freed Value pointer.
    // This avoids allocator corruption on accidental duplicate release calls.
    if let Ok(set) = FREED_PTRS.get_or_init(|| Mutex::new(HashSet::new())).lock() {
        if set.contains(&ptr) {
            if debug_release {
                let info = FREED_INFO
                    .get_or_init(|| Mutex::new(HashMap::new()))
                    .lock()
                    .ok()
                    .and_then(|m| m.get(&ptr).cloned());
                let alloc = ALLOC_INFO
                    .get_or_init(|| Mutex::new(HashMap::new()))
                    .lock()
                    .ok()
                    .and_then(|m| m.get(&ptr).cloned())
                    .or_else(|| {
                        FREED_ALLOC_INFO
                            .get_or_init(|| Mutex::new(HashMap::new()))
                            .lock()
                            .ok()
                            .and_then(|m| m.get(&ptr).cloned())
                    });
                if let Some(info) = info {
                    eprintln!(
                        "[clorus] double free detected: {:p} (first free seq={} tag={:?} ts={}ms)",
                        val, info.seq, info.tag, info.ts_ms
                    );
                    if let Some(bt) = info.backtrace {
                        eprintln!("[clorus] first free backtrace:\n{}", bt);
                    }
                } else {
                    eprintln!("[clorus] double free detected: {:p}", val);
                }
                if let Some(alloc) = alloc {
                    if let Some(bt) = alloc.backtrace {
                        eprintln!(
                            "[clorus] allocation backtrace (tag={:?} ts={}ms):\n{}",
                            alloc.tag, alloc.ts_ms, bt
                        );
                    }
                }
                if std::env::var("CLORUS_DEBUG_RELEASE_BT").is_ok() {
                    let bt_now = std::backtrace::Backtrace::force_capture().to_string();
                    eprintln!("[clorus] double free current backtrace:\n{}", bt_now);
                }
            }
            return;
        }
    }

    if debug_release {
        if (std::env::var("CLORUS_DEBUG_PTR").is_ok() || std::env::var("CLORUS_DEBUG_TAG").is_ok())
            && should_debug_value(val)
            && std::env::var("CLORUS_DEBUG_RELEASE_BT").is_ok()
        {
            let bt_now = std::backtrace::Backtrace::force_capture().to_string();
            eprintln!("[clorus] release call backtrace:\n{}", bt_now);
        }
        if std::env::var("CLORUS_GUARD_RELEASE").is_ok() {
            let rc_before = unsafe { (*val).header.refcount() };
            if rc_before == 0 {
                eprintln!(
                    "[clorus] release on refcount=0 ptr={:p} tag={:?}",
                    val,
                    unsafe { (*val).header.tag() }
                );
                if std::env::var("CLORUS_DEBUG_RELEASE_BT").is_ok() {
                    let bt_now = std::backtrace::Backtrace::force_capture().to_string();
                    eprintln!("[clorus] release on refcount=0 backtrace:\n{}", bt_now);
                }
                return;
            }
        }

        let seq = RELEASE_SEQ.fetch_add(1, Ordering::Relaxed);
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|d| d.as_millis()).unwrap_or(0);
        // Read tag/refcount after double-free guard to avoid deref on freed memory.
        let tag = unsafe { (*val).header.tag() };
        let rc_before = unsafe { (*val).header.refcount() };
        eprintln!(
            "[clorus] release seq={} ts={} ptr={:p} tag={:?} rc_before={}",
            seq, ts, val, tag, rc_before
        );
    }

    // Optional debug tracing for a specific pointer or tag.
    // Use CLORUS_DEBUG_PTR (hex like 0x1234 or decimal) and/or CLORUS_DEBUG_TAG (e.g., "Vector").
    if std::env::var("CLORUS_DEBUG_PTR").is_ok() || std::env::var("CLORUS_DEBUG_TAG").is_ok() {
        if should_debug_value(val) {
            eprintln!("[clorus] release (pre) ptr={:p} tag={:?} rc={}", val, unsafe { (*val).header.tag() }, unsafe { (*val).header.refcount() });
            if std::env::var("CLORUS_DEBUG_RELEASE_BT").is_ok() {
                let bt_now = std::backtrace::Backtrace::force_capture().to_string();
                eprintln!("[clorus] release (pre) backtrace:\n{}", bt_now);
            }
        }
    }

    unsafe {
        if (*val).header.tag() == ValueTag::Keyword {
            // Keywords are interned and live for program lifetime.
            return;
        }
        if (*val).header.release() {
            // Last reference - deallocate
            if debug_release {
                let seq = RELEASE_SEQ.fetch_add(1, Ordering::Relaxed);
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let rc_after = (*val).header.refcount();
                let tag = (*val).header.tag();
                eprintln!(
                    "[clorus] release -> deallocate ptr={:p} tag={:?} rc_after={}",
                    val, tag, rc_after
                );
                let bt = if std::env::var("CLORUS_DEBUG_RELEASE_BT").is_ok() {
                    Some(std::backtrace::Backtrace::force_capture().to_string())
                } else {
                    None
                };
                if let Ok(mut info_map) = FREED_INFO.get_or_init(|| Mutex::new(HashMap::new())).lock() {
                    let info = FreedInfo { seq, tag, ts_ms: ts, backtrace: bt };
                    info_map.insert(val as usize, info);
                }
            }
            if let Ok(mut set) = FREED_PTRS.get_or_init(|| Mutex::new(HashSet::new())).lock() {
                set.insert(val as usize);
            }
            if std::env::var("CLORUS_DEBUG_PTR").is_ok() || std::env::var("CLORUS_DEBUG_TAG").is_ok() {
                if should_debug_value(val) {
                    eprintln!("[clorus] deallocate ptr={:p} tag={:?}", val, (*val).header.tag());
                    if std::env::var("CLORUS_DEBUG_RELEASE_BT").is_ok() {
                        let bt_now = std::backtrace::Backtrace::force_capture().to_string();
                        eprintln!("[clorus] deallocate backtrace:\n{}", bt_now);
                    }
                }
            }
            let debug_alloc = std::env::var("CLORUS_DEBUG_ALLOC_BT").is_ok();
            if let Ok(mut allocs) = ALLOC_INFO.get_or_init(|| Mutex::new(HashMap::new())).lock() {
                if debug_alloc {
                    if let Some(info) = allocs.remove(&(val as usize)) {
                        if let Ok(mut freed_allocs) =
                            FREED_ALLOC_INFO.get_or_init(|| Mutex::new(HashMap::new())).lock()
                        {
                            freed_allocs.insert(val as usize, info);
                        }
                    }
                } else {
                    allocs.remove(&(val as usize));
                }
            }
            deallocate_value(val);
        }
    }
}

fn should_debug_value(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }

    // Filter by pointer if provided.
    if let Ok(ptr_str) = std::env::var("CLORUS_DEBUG_PTR") {
        if let Some(target) = parse_ptr(&ptr_str) {
            if val as usize != target {
                return false;
            }
        }
    }

    // Filter by tag if provided.
    if let Ok(tag_str) = std::env::var("CLORUS_DEBUG_TAG") {
        let tag = unsafe { (*val).header.tag() };
        if !tag_matches(&tag_str, tag) {
            return false;
        }
    }

    true
}

fn parse_ptr(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<usize>().ok()
    }
}

fn tag_matches(s: &str, tag: ValueTag) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return true;
    }
    // Accept either exact enum Debug name (e.g., "Vector") or lowercase.
    let tag_name = format!("{:?}", tag);
    tag_name == s || tag_name.eq_ignore_ascii_case(s)
}

fn record_alloc(val: *mut Value, tag: ValueTag) {
    if val.is_null() {
        return;
    }
    let debug_alloc_bt = std::env::var("CLORUS_DEBUG_ALLOC_BT").is_ok();

    // If allocator reuses an address, clear stale "freed" tracking first.
    // Do this unconditionally so runtime duplicate-release guard remains accurate.
    if let Ok(mut freed) = FREED_PTRS.get_or_init(|| Mutex::new(HashSet::new())).lock() {
        freed.remove(&(val as usize));
    }
    if let Ok(mut freed_info) = FREED_INFO.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        freed_info.remove(&(val as usize));
    }
    if let Ok(mut freed_allocs) = FREED_ALLOC_INFO.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        freed_allocs.remove(&(val as usize));
    }

    if !debug_alloc_bt {
        return;
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let bt = Some(std::backtrace::Backtrace::force_capture().to_string());
    if let Ok(mut allocs) = ALLOC_INFO.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        allocs.insert(
            val as usize,
            AllocInfo {
                tag,
                ts_ms: ts,
                backtrace: bt,
            },
        );
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

/// Create a Value from i64 Long (for LLVM codegen)
#[no_mangle]
pub extern "C" fn clorus_value_long(n: i64) -> *mut Value {
    Value::long(n)
}

/// Create a Value from f64 Double (for LLVM codegen)
#[no_mangle]
pub extern "C" fn clorus_value_double(n: f64) -> *mut Value {
    Value::double(n)
}

/// Create a boolean Value (for LLVM codegen)
#[no_mangle]
pub extern "C" fn clorus_value_boolean(b: bool) -> *mut Value {
    Value::boolean(b)
}

/// Create an opaque pointer Value (for LLVM codegen FFI pointers)
#[no_mangle]
pub extern "C" fn clorus_value_opaque_pointer(ptr: *mut u8) -> *mut Value {
    Value::opaque_pointer(ptr)
}

/// Extract opaque pointer from Value (for LLVM codegen FFI calls)
#[no_mangle]
pub extern "C" fn clorus_extract_opaque_pointer(val: *mut Value) -> *mut u8 {
    if val.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        (*val).as_opaque_pointer()
    }
}

/// Check if a Value is truthy (for if/when/etc.)
/// Returns 1 for truthy, 0 for falsy
/// Falsy values: nil, false, 0 (as Long or Double)
/// Everything else is truthy
#[no_mangle]
pub extern "C" fn clorus_is_truthy(val: *mut Value) -> i32 {
    if val.is_null() {
        return 0; // nil is falsy
    }
    unsafe {
        match (*val).header().tag() {
            ValueTag::Nil => 0, // nil is falsy
            ValueTag::Bool => {
                if (*val).as_bool() { 1 } else { 0 }
            }
            ValueTag::Long => {
                let n = (*val).as_long();
                if n == 0 { 0 } else { 1 }
            }
            ValueTag::Double => {
                let n = (*val).as_double();
                if n == 0.0 { 0 } else { 1 }
            }
            _ => 1, // Everything else is truthy (strings, vectors, functions, etc.)
        }
    }
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

/// Extract i64 from Value (for LLVM codegen)
/// Returns 0 if the value is not a Long
#[no_mangle]
pub extern "C" fn clorus_value_as_long(val: *mut Value) -> i64 {
    if val.is_null() {
        return 0;
    }
    unsafe {
        if (*val).header.tag() == ValueTag::Long {
            (*val).as_long()
        } else {
            0
        }
    }
}

/// Extract f64 from Value (for LLVM codegen)
/// Returns 0.0 if the value is not a Double
#[no_mangle]
pub extern "C" fn clorus_value_as_double(val: *mut Value) -> f64 {
    if val.is_null() {
        return 0.0;
    }
    unsafe {
        if (*val).header.tag() == ValueTag::Double {
            (*val).as_double()
        } else {
            0.0
        }
    }
}

/// Extract numeric value as f64 (handles both Long and Double)
/// Returns 0.0 if the value is neither Long nor Double
#[no_mangle]
pub extern "C" fn clorus_value_as_number(val: *mut Value) -> f64 {
    if val.is_null() {
        return 0.0;
    }
    unsafe {
        match (*val).header.tag() {
            ValueTag::Long => (*val).as_long() as f64,
            ValueTag::Double => (*val).as_double(),
            _ => 0.0,
        }
    }
}

/// Extract bool from Value (for REPL display)
/// Returns false if the value is not a boolean
#[no_mangle]
pub extern "C" fn clorus_value_as_bool(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        if (*val).header.tag() == ValueTag::Bool {
            (*val).as_bool()
        } else {
            false
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
        ValueTag::Long | ValueTag::Double | ValueTag::Bool | ValueTag::Nil => {
            // Just free the Value itself
            drop(Box::from_raw(val));
        }
        ValueTag::List => {
            // Release persistent list wrapper and free Value.
            // NOTE: ValueTag::List stores `*mut PersistentList`, not `*mut ListNode`.
            // Dropping PersistentList will release the head node chain via Drop.
            let ptr = (*val).as_ptr() as *mut crate::list::PersistentList;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
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
        ValueTag::Function => {
            // Release function data
            let func_data = (*val).as_function();
            if !func_data.is_null() {
                // Release captured environment values
                let env_size = (*func_data).env_size();
                if env_size > 0 {
                    let env_ptr = func_data.offset(1) as *const *mut Value;
                    for i in 0..env_size {
                        let env_val = *env_ptr.offset(i as isize);
                        if !env_val.is_null() {
                            clorus_release(env_val);
                        }
                    }
                }
                // Free the function data
                let func_data_size = std::mem::size_of::<crate::function::FunctionData>();
                let env_data_size = (env_size as usize) * std::mem::size_of::<*mut Value>();
                let total_size = func_data_size + env_data_size;
                let layout = std::alloc::Layout::from_size_align_unchecked(total_size, 8);
                std::alloc::dealloc(func_data as *mut u8, layout);
            }
            drop(Box::from_raw(val));
        }
        ValueTag::MultiArityFunction => {
            // Release multi-arity function data
            let multi_func_data = (*val).as_multi_arity_function();
            if !multi_func_data.is_null() {
                let arity_count = (*multi_func_data).arity_count;
                let env_size = (*multi_func_data).env_size;

                // Release captured environment values (after arity variants)
                if env_size > 0 {
                    let variants_ptr = multi_func_data.offset(1) as *const crate::function::ArityVariant;
                    let env_ptr = variants_ptr.offset(arity_count as isize) as *const *mut Value;
                    for i in 0..env_size {
                        let env_val = *env_ptr.offset(i as isize);
                        if !env_val.is_null() {
                            clorus_release(env_val);
                        }
                    }
                }

                // Free the multi-arity function data
                let header_size = std::mem::size_of::<crate::function::MultiArityFunctionData>();
                let variants_size = (arity_count as usize) * std::mem::size_of::<crate::function::ArityVariant>();
                let env_data_size = (env_size as usize) * std::mem::size_of::<*mut Value>();
                let total_size = header_size + variants_size + env_data_size;
                let layout = std::alloc::Layout::from_size_align_unchecked(total_size, 8);
                std::alloc::dealloc(multi_func_data as *mut u8, layout);
            }
            drop(Box::from_raw(val));
        }
        ValueTag::Var => {
            // Release var and free Value
            let var_ptr = (*val).as_var();
            if !var_ptr.is_null() {
                crate::var::clorus_var_free(var_ptr);
            }
            drop(Box::from_raw(val));
        }
        ValueTag::OpaquePointer => {
            // Opaque pointers are managed by FFI layer
            // Just free the Value wrapper, not the pointer itself
            drop(Box::from_raw(val));
        }
    }
}

// ============================================================================
// Type Predicates
// ============================================================================

/// Check if value is a Long (i64)
#[no_mangle]
pub extern "C" fn clorus_is_long(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::Long
    }
}

/// Check if value is a Double (f64)
#[no_mangle]
pub extern "C" fn clorus_is_double(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::Double
    }
}

/// Check if value is a number (Long or Double)
#[no_mangle]
pub extern "C" fn clorus_is_number(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        let tag = (*val).header().tag();
        tag == ValueTag::Long || tag == ValueTag::Double
    }
}

/// Check if value is a vector
#[no_mangle]
pub extern "C" fn clorus_is_vector(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::Vector
    }
}

/// Check if value is a list
#[no_mangle]
pub extern "C" fn clorus_is_list(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::List
    }
}

/// Check if value is a map
#[no_mangle]
pub extern "C" fn clorus_is_map(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::HashMap
    }
}

/// Check if value is a set
#[no_mangle]
pub extern "C" fn clorus_is_set(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::HashSet
    }
}

/// Check if value is a symbol
#[no_mangle]
pub extern "C" fn clorus_is_symbol(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::Symbol
    }
}

/// Check if value is nil
#[no_mangle]
pub extern "C" fn clorus_is_nil(val: *mut Value) -> bool {
    if val.is_null() {
        return true;
    }
    unsafe {
        (*val).header().tag() == ValueTag::Nil
    }
}

/// Check if value is a boolean
#[no_mangle]
pub extern "C" fn clorus_is_bool(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::Bool
    }
}

/// Check if value is a seq (list only - not vector)
/// In Clojure, seq? returns true only for lists/seqs, not vectors
#[no_mangle]
pub extern "C" fn clorus_is_seq(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::List
    }
}

/// Check if value is a collection (vector, list, map, or set)
#[no_mangle]
pub extern "C" fn clorus_is_coll(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        let tag = (*val).header().tag();
        tag == ValueTag::Vector || tag == ValueTag::List ||
        tag == ValueTag::HashMap || tag == ValueTag::HashSet
    }
}

/// Check if value is an atom
#[no_mangle]
pub extern "C" fn clorus_is_atom(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::Atom
    }
}

/// Check if value is a ref
#[no_mangle]
pub extern "C" fn clorus_is_ref(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::Ref
    }
}

/// Check if value is an agent
#[no_mangle]
pub extern "C" fn clorus_is_agent(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::Agent
    }
}

/// Check if value is a channel
#[no_mangle]
pub extern "C" fn clorus_is_channel(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        (*val).header().tag() == ValueTag::Channel
    }
}

/// Check if value is a function
#[no_mangle]
pub extern "C" fn clorus_is_fn(val: *mut Value) -> bool {
    if val.is_null() {
        return false;
    }
    unsafe {
        let tag = (*val).header().tag();
        tag == ValueTag::Function || tag == ValueTag::MultiArityFunction
    }
}

#[inline]
fn bool_to_i32(b: bool) -> i32 {
    if b { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn clorus_is_number_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_number(val)) }
#[no_mangle]
pub extern "C" fn clorus_is_vector_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_vector(val)) }
#[no_mangle]
pub extern "C" fn clorus_is_list_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_list(val)) }
#[no_mangle]
pub extern "C" fn clorus_is_map_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_map(val)) }
#[no_mangle]
pub extern "C" fn clorus_is_set_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_set(val)) }
#[no_mangle]
pub extern "C" fn clorus_is_symbol_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_symbol(val)) }
#[no_mangle]
pub extern "C" fn clorus_is_nil_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_nil(val)) }
#[no_mangle]
pub extern "C" fn clorus_is_bool_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_bool(val)) }
#[no_mangle]
pub extern "C" fn clorus_is_seq_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_seq(val)) }
#[no_mangle]
pub extern "C" fn clorus_is_coll_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_coll(val)) }
#[no_mangle]
pub extern "C" fn clorus_is_fn_i32(val: *mut Value) -> i32 { bool_to_i32(clorus_is_fn(val)) }

/// Compare two values for equality
/// Returns 1 (true) if equal, 0 (false) if not equal
/// Handles all value types properly:
/// - Keywords: pointer equality (since they're interned)
/// - Strings: string content comparison
/// - Numbers (Long/Double): numeric comparison
/// - Booleans: boolean comparison
/// - Nil: both must be nil
/// - Other types: pointer equality for now
#[no_mangle]
pub extern "C" fn clorus_equals(left: *mut Value, right: *mut Value) -> bool {
    // Handle null pointers
    if left.is_null() && right.is_null() {
        return true; // Both nil
    }
    if left.is_null() || right.is_null() {
        return false; // One is nil, other is not
    }

    unsafe {
        let left_tag = (*left).header().tag();
        let right_tag = (*right).header().tag();

        // Different types are not equal (except numeric types)
        if left_tag != right_tag {
            // Allow comparison between Long and Double
            match (left_tag, right_tag) {
                (ValueTag::Long, ValueTag::Double) | (ValueTag::Double, ValueTag::Long) => {
                    let left_num = if left_tag == ValueTag::Long {
                        (*left).as_long() as f64
                    } else {
                        (*left).as_double()
                    };
                    let right_num = if right_tag == ValueTag::Long {
                        (*right).as_long() as f64
                    } else {
                        (*right).as_double()
                    };
                    return left_num == right_num;
                }
                _ => return false,
            }
        }

        // Same type - compare based on type
        match left_tag {
            ValueTag::Nil => true, // Both are nil

            ValueTag::Bool => (*left).as_bool() == (*right).as_bool(),

            ValueTag::Long => (*left).as_long() == (*right).as_long(),

            ValueTag::Double => {
                let left_val = (*left).as_double();
                let right_val = (*right).as_double();
                // Handle NaN comparison
                if left_val.is_nan() && right_val.is_nan() {
                    return true; // NaN == NaN for our purposes
                }
                left_val == right_val
            }

            ValueTag::Keyword => {
                // Keywords are interned, so we can use pointer equality
                // But to be safe, also compare the strings
                let left_ptr = (*left).as_ptr();
                let right_ptr = (*right).as_ptr();
                if left_ptr == right_ptr {
                    return true; // Same interned keyword
                }
                // Fall back to string comparison
                let left_str = (*left).as_keyword();
                let right_str = (*right).as_keyword();
                left_str == right_str
            }

            ValueTag::String => {
                let left_str = (*left).as_string();
                let right_str = (*right).as_string();
                left_str == right_str
            }

            ValueTag::Symbol => {
                // TODO: Implement proper symbol comparison
                // For now, use pointer equality
                (*left).as_ptr() == (*right).as_ptr()
            }

            ValueTag::Vector => {
                let left_ptr = (*left).as_ptr() as *mut crate::vector::PersistentVector;
                let right_ptr = (*right).as_ptr() as *mut crate::vector::PersistentVector;
                let left_count = (*left_ptr).count();
                let right_count = (*right_ptr).count();
                if left_count != right_count {
                    return false;
                }
                for i in 0..left_count {
                    let left_elem = crate::vector::PersistentVector::nth(left_ptr, i);
                    let right_elem = crate::vector::PersistentVector::nth(right_ptr, i);
                    let eq = clorus_equals(left_elem, right_elem);
                    if !left_elem.is_null() {
                        crate::value::clorus_release(left_elem);
                    }
                    if !right_elem.is_null() {
                        crate::value::clorus_release(right_elem);
                    }
                    if !eq {
                        return false;
                    }
                }
                true
            }
            ValueTag::List => {
                let left_ptr = (*left).as_ptr() as *mut crate::list::PersistentList;
                let right_ptr = (*right).as_ptr() as *mut crate::list::PersistentList;
                crate::list::list_equals(left_ptr, right_ptr)
            }
            ValueTag::HashMap => {
                let left_ptr = (*left).as_ptr() as *mut crate::map::ClorusHashMap;
                let right_ptr = (*right).as_ptr() as *mut crate::map::ClorusHashMap;
                if (*left_ptr).count() != (*right_ptr).count() {
                    return false;
                }
                for (key, value) in (*left_ptr).entries_iter() {
                    match (*right_ptr).get_entry(*key) {
                        Some(other_val) => {
                            if !clorus_equals(*value, other_val) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            }
            ValueTag::HashSet => {
                let left_ptr = (*left).as_ptr() as *mut crate::set::ClorusHashSet;
                let right_ptr = (*right).as_ptr() as *mut crate::set::ClorusHashSet;
                if (*left_ptr).count() != (*right_ptr).count() {
                    return false;
                }
                for value in (*left_ptr).values().iter() {
                    if !(*right_ptr).contains(*value) {
                        return false;
                    }
                }
                true
            }

            ValueTag::Atom | ValueTag::Ref | ValueTag::Agent | ValueTag::Channel |
            ValueTag::Function | ValueTag::MultiArityFunction | ValueTag::Var |
            ValueTag::OpaquePointer => (*left).as_ptr() == (*right).as_ptr(),
        }
    }
}

/// Create a symbol value from a C string pointer
/// Used for map destructuring with :syms
/// Note: Currently symbols are not fully implemented, so this creates a keyword instead
#[no_mangle]
pub extern "C" fn clorus_symbol(name_ptr: *const std::os::raw::c_char) -> *mut Value {
    // For now, symbols behave like keywords
    // TODO: Implement proper Symbol type when needed
    crate::keyword::clorus_keyword(name_ptr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_value() {
        let val = Value::long(42);
        unsafe {
            assert_eq!((*val).header.tag(), ValueTag::Long);
            assert_eq!((*val).as_long(), 42);
            assert_eq!((*val).header.refcount(), 1);
        }
        // Clean up
        unsafe { drop(Box::from_raw(val)); }
    }

    #[test]
    fn test_double_value() {
        let val = Value::double(3.14);
        unsafe {
            assert_eq!((*val).header.tag(), ValueTag::Double);
            assert_eq!((*val).as_double(), 3.14);
            assert_eq!((*val).header.refcount(), 1);
        }
        // Clean up
        unsafe { drop(Box::from_raw(val)); }
    }

    #[test]
    fn test_number_value() {
        // Backward compatibility test - still works with Double
        let val = Value::double(42.0);
        unsafe {
            assert_eq!((*val).header.tag(), ValueTag::Double);
            assert_eq!((*val).as_double(), 42.0);
            assert_eq!((*val).header.refcount(), 1);
        }
        // Clean up
        unsafe { drop(Box::from_raw(val)); }
    }

    #[test]
    fn test_refcounting() {
        let val = Value::double(3.14);

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

    #[test]
    fn test_vector_structural_equals_and_hash() {
        unsafe {
            let vec1 = crate::vector::clorus_vector_empty();
            let vec2 = crate::vector::clorus_vector_empty();

            let v1 = Value::long(1);
            let v2 = Value::long(2);

            let vec1 = crate::vector::clorus_vector_conj(vec1, v1);
            let vec1 = crate::vector::clorus_vector_conj(vec1, v2);

            let v1b = Value::long(1);
            let v2b = Value::long(2);

            let vec2 = crate::vector::clorus_vector_conj(vec2, v1b);
            let vec2 = crate::vector::clorus_vector_conj(vec2, v2b);

            assert!(clorus_equals(vec1, vec2));
            assert_eq!(crate::hash::clorus_hash(vec1), crate::hash::clorus_hash(vec2));

            clorus_release(vec1);
            clorus_release(vec2);
            clorus_release(v1);
            clorus_release(v2);
            clorus_release(v1b);
            clorus_release(v2b);
        }
    }

    #[test]
    fn test_map_structural_equals_and_hash() {
        unsafe {
            let map1 = crate::map::clorus_map_empty();
            let map2 = crate::map::clorus_map_empty();

            let k1 = Value::string("a");
            let v1 = Value::long(1);
            let k2 = Value::string("b");
            let v2 = Value::long(2);

            let map1 = crate::map::clorus_map_assoc(map1, k1, v1);
            let map1 = crate::map::clorus_map_assoc(map1, k2, v2);

            let k1b = Value::string("a");
            let v1b = Value::long(1);
            let k2b = Value::string("b");
            let v2b = Value::long(2);

            let map2 = crate::map::clorus_map_assoc(map2, k1b, v1b);
            let map2 = crate::map::clorus_map_assoc(map2, k2b, v2b);

            assert!(clorus_equals(map1, map2));
            assert_eq!(crate::hash::clorus_hash(map1), crate::hash::clorus_hash(map2));

            clorus_release(map1);
            clorus_release(map2);
            clorus_release(k1);
            clorus_release(v1);
            clorus_release(k2);
            clorus_release(v2);
            clorus_release(k1b);
            clorus_release(v1b);
            clorus_release(k2b);
            clorus_release(v2b);
        }
    }
}
