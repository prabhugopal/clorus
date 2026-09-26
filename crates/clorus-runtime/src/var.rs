/// Var (variable) support for dynamic bindings
/// Vars are first-class objects that hold a root value and metadata
use crate::value::{Value, ValueTag};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

thread_local! {
    /// Thread-local dynamic binding stack.
    /// Key is Var pointer address, value is stack of bound values (top = last).
    static DYNAMIC_BINDINGS: RefCell<HashMap<usize, Vec<*mut Value>>> = RefCell::new(HashMap::new());
}

#[inline]
fn var_error_logging_enabled() -> bool {
    std::env::var("CLORUS_LOG_CALL_ERRORS")
        .ok()
        .map(|v| v != "0")
        .unwrap_or(false)
}

/// Var structure
/// Holds a root binding and optional metadata
pub struct Var {
    /// Symbol name (e.g., "*out*")
    pub name: String,

    /// Root value (default binding)
    pub root: Arc<Mutex<*mut Value>>,

    /// Metadata map (e.g., {:dynamic true, :doc "..."})
    pub metadata: Arc<Mutex<HashMap<String, *mut Value>>>,
}

impl Var {
    /// Create a new Var with a root value
    pub fn new(name: String, root: *mut Value) -> Self {
        unsafe {
            if !root.is_null() {
                crate::value::clorus_retain(root);
            }
        }
        Var {
            name,
            root: Arc::new(Mutex::new(root)),
            metadata: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the root value
    pub fn get_root(&self) -> *mut Value {
        let root = self.root.lock().unwrap();
        *root
    }

    /// Set the root value (alter-var-root)
    pub fn set_root(&self, new_value: *mut Value) {
        let mut root = self.root.lock().unwrap();
        unsafe {
            if !new_value.is_null() {
                crate::value::clorus_retain(new_value);
            }
            if !(*root).is_null() {
                crate::value::clorus_release(*root);
            }
        }
        *root = new_value;
    }

    /// Check if this var is dynamic (has :dynamic metadata)
    pub fn is_dynamic(&self) -> bool {
        let meta = self.metadata.lock().unwrap();
        meta.contains_key("dynamic")
    }

    /// Get metadata value for a key
    pub fn get_meta(&self, key: &str) -> Option<*mut Value> {
        let meta = self.metadata.lock().unwrap();
        meta.get(key).copied()
    }

    /// Set metadata value for a key
    pub fn set_meta(&self, key: String, value: *mut Value) {
        let mut meta = self.metadata.lock().unwrap();
        unsafe {
            if !value.is_null() {
                crate::value::clorus_retain(value);
            }
        }
        if let Some(old) = meta.insert(key, value) {
            unsafe {
                if !old.is_null() {
                    crate::value::clorus_release(old);
                }
            }
        }
    }

    /// Replace all metadata with entries from `meta_map`.
    /// `meta_map` must be a Clorus HashMap value.
    pub fn replace_meta_from_map(&self, meta_map: *mut Value) {
        unsafe {
            // Metadata contract: only map or nil is valid.
            // Invalid metadata values keep existing metadata unchanged.
            let is_nil = meta_map.is_null() || (*meta_map).tag() == ValueTag::Nil;
            let is_map = !meta_map.is_null() && (*meta_map).tag() == ValueTag::HashMap;
            if !is_nil && !is_map {
                return;
            }

            // Clear old metadata and release old values.
            let mut meta = self.metadata.lock().unwrap();
            for v in meta.values() {
                if !(*v).is_null() {
                    crate::value::clorus_release(*v);
                }
            }
            meta.clear();

            if is_nil {
                return;
            }

            let map_ptr = (*meta_map).as_ptr() as *mut crate::map::ClorusHashMap;
            for (k, v) in (*map_ptr).entries_iter() {
                let key_str = if k.is_null() {
                    None
                } else {
                    match (*k).tag() {
                        ValueTag::Keyword => Some((*k).as_keyword().to_string()),
                        ValueTag::String => Some((*k).as_string().to_string()),
                        _ => None,
                    }
                };

                if let Some(key) = key_str {
                    if !v.is_null() {
                        crate::value::clorus_retain(v);
                    }
                    meta.insert(key, v);
                }
            }
        }
    }
}

impl Drop for Var {
    fn drop(&mut self) {
        if let Ok(root) = self.root.lock() {
            unsafe {
                if !(*root).is_null() {
                    crate::value::clorus_release(*root);
                }
            }
        }

        if let Ok(meta) = self.metadata.lock() {
            for v in meta.values() {
                unsafe {
                    if !(*v).is_null() {
                        crate::value::clorus_release(*v);
                    }
                }
            }
        }
    }
}

/// FFI functions for Var operations

/// Create a new Var
/// Args: name (String*), root_value (Value*)
/// Returns: Var* (as opaque pointer)
#[no_mangle]
pub extern "C" fn clorus_var_new(name_ptr: *mut Value, root_value: *mut Value) -> *mut Var {
    unsafe {
        if name_ptr.is_null() {
            return std::ptr::null_mut();
        }

        // Extract name from Value
        let name = if (*name_ptr).tag() == ValueTag::String {
            (*name_ptr).as_string().to_string()
        } else {
            "unnamed".to_string()
        };

        let var = Var::new(name, root_value);
        Box::into_raw(Box::new(var))
    }
}

/// Get Var's root value (deref the var)
/// Args: var (Var*)
/// Returns: Value*
#[no_mangle]
pub extern "C" fn clorus_var_get(var_ptr: *mut Var) -> *mut Value {
    if var_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let key = var_ptr as usize;
    let thread_bound = DYNAMIC_BINDINGS.with(|map_cell| {
        let map = map_cell.borrow();
        map.get(&key).and_then(|stack| stack.last().copied())
    });
    if let Some(v) = thread_bound {
        return v;
    }

    unsafe {
        let var = &*var_ptr;
        var.get_root()
    }
}

/// Set Var's root value (alter-var-root)
/// Args: var (Var*), new_value (Value*)
/// Returns: new_value (Value*)
#[no_mangle]
pub extern "C" fn clorus_var_set(var_ptr: *mut Var, new_value: *mut Value) -> *mut Value {
    if var_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let key = var_ptr as usize;
    let mut updated_thread_binding = false;
    DYNAMIC_BINDINGS.with(|map_cell| {
        let mut map = map_cell.borrow_mut();
        if let Some(stack) = map.get_mut(&key) {
            if let Some(slot) = stack.last_mut() {
                unsafe {
                    if !new_value.is_null() {
                        crate::value::clorus_retain(new_value);
                    }
                    if !(*slot).is_null() {
                        crate::value::clorus_release(*slot);
                    }
                }
                *slot = new_value;
                updated_thread_binding = true;
            }
        }
    });

    if !updated_thread_binding {
        unsafe {
            let var = &*var_ptr;
            var.set_root(new_value);
        }
    }

    new_value
}

/// Push a thread-local dynamic binding for this var.
#[no_mangle]
pub extern "C" fn clorus_var_push_binding(var_ptr: *mut Var, new_value: *mut Value) {
    if var_ptr.is_null() {
        return;
    }

    let value_to_bind = if new_value.is_null() {
        Value::nil()
    } else {
        new_value
    };

    unsafe {
        if !value_to_bind.is_null() {
            crate::value::clorus_retain(value_to_bind);
        }
    }

    let key = var_ptr as usize;
    DYNAMIC_BINDINGS.with(|map_cell| {
        let mut map = map_cell.borrow_mut();
        map.entry(key).or_default().push(value_to_bind);
    });
}

/// Pop a thread-local dynamic binding for this var.
#[no_mangle]
pub extern "C" fn clorus_var_pop_binding(var_ptr: *mut Var) {
    if var_ptr.is_null() {
        return;
    }

    let key = var_ptr as usize;
    let mut popped: *mut Value = std::ptr::null_mut();
    DYNAMIC_BINDINGS.with(|map_cell| {
        let mut map = map_cell.borrow_mut();
        let mut should_remove = false;
        if let Some(stack) = map.get_mut(&key) {
            popped = stack.pop().unwrap_or(std::ptr::null_mut());
            should_remove = stack.is_empty();
        }
        if should_remove {
            map.remove(&key);
        }
    });

    if !popped.is_null() {
        unsafe {
            crate::value::clorus_release(popped);
        }
    }
}

/// Check if Var is dynamic
/// Args: var (Var*)
/// Returns: bool (1 = dynamic, 0 = not dynamic)
#[no_mangle]
pub extern "C" fn clorus_var_is_dynamic(var_ptr: *mut Var) -> bool {
    unsafe {
        if var_ptr.is_null() {
            return false;
        }
        let var = &*var_ptr;
        var.is_dynamic()
    }
}

/// Set metadata on a Var
/// Args: var (Var*), key (String*), value (Value*)
/// Returns: void
#[no_mangle]
pub extern "C" fn clorus_var_set_meta(
    var_ptr: *mut Var,
    key_ptr: *mut Value,
    value: *mut Value,
) {
    unsafe {
        if var_ptr.is_null() || key_ptr.is_null() {
            return;
        }

        let var = &*var_ptr;

        // Extract key from Value
        let key = if (*key_ptr).tag() == ValueTag::String {
            (*key_ptr).as_string().to_string()
        } else {
            return; // Invalid key
        };

        var.set_meta(key, value);
    }
}

/// Get all metadata from a Var as a Clorus map.
/// Keys are returned as keywords (e.g. "doc" -> :doc).
#[no_mangle]
pub extern "C" fn clorus_var_meta(var_ptr: *mut Var) -> *mut Value {
    unsafe {
        if var_ptr.is_null() {
            return Value::nil();
        }

        let var = &*var_ptr;
        let map_empty_fn = crate::map::clorus_map_empty;
        let map_assoc_fn = crate::map::clorus_map_assoc;

        let mut out = map_empty_fn();

        // Every var carries :name in its metadata unconditionally, exactly
        // like real Clojure (`(:name (meta #'x))` always works, whether or
        // not `x` was ever declared with explicit metadata) -- the var's
        // own `name` field is authoritative, so this can't drift out of
        // sync the way a separately-tracked :name meta entry could.
        let name_key = Value::keyword("name");
        let name_val = Value::symbol(&var.name);
        out = map_assoc_fn(out, name_key, name_val);
        crate::value::clorus_release(name_key);
        crate::value::clorus_release(name_val);

        if let Ok(meta) = var.metadata.lock() {
            for (k, v) in meta.iter() {
                let key_val = Value::keyword(k);
                out = map_assoc_fn(out, key_val, *v);
                crate::value::clorus_release(key_val);
            }
        }

        out
    }
}

/// Generic metadata lookup.
/// Currently only Var metadata is supported.
#[no_mangle]
pub extern "C" fn clorus_meta(value_ptr: *mut Value) -> *mut Value {
    unsafe {
        if value_ptr.is_null() {
            return Value::nil();
        }

        if (*value_ptr).tag() == ValueTag::Var {
            let var_ptr = (*value_ptr).as_var();
            return clorus_var_meta(var_ptr);
        }
        crate::value::clorus_value_meta(value_ptr)
    }
}

/// Attach metadata to a value.
/// Currently metadata is applied only for Var values.
#[no_mangle]
pub extern "C" fn clorus_with_meta(value_ptr: *mut Value, meta_map: *mut Value) -> *mut Value {
    unsafe {
        if value_ptr.is_null() {
            return std::ptr::null_mut();
        }

        let valid_meta = meta_map.is_null()
            || (*meta_map).tag() == ValueTag::Nil
            || (*meta_map).tag() == ValueTag::HashMap;
        if !valid_meta {
            return crate::value::metadata_type_error("with-meta", meta_map);
        }

        if (*value_ptr).tag() == ValueTag::Var {
            let var_ptr = (*value_ptr).as_var();
            if !var_ptr.is_null() {
                (&*var_ptr).replace_meta_from_map(meta_map);
            }
            crate::value::clorus_retain(value_ptr);
            return value_ptr;
        }

        crate::value::clorus_value_with_meta(value_ptr, meta_map)
    }
}

/// Get Var's name as a String Value
/// Args: var (Var*)
/// Returns: String Value*
#[no_mangle]
pub extern "C" fn clorus_var_name(var_ptr: *mut Var) -> *mut Value {
    unsafe {
        if var_ptr.is_null() {
            return std::ptr::null_mut();
        }
        let var = &*var_ptr;

        // Create a new String Value from the name
        Value::string(&var.name)
    }
}

/// Free a Var
/// Args: var (Var*)
/// Returns: void
#[no_mangle]
pub extern "C" fn clorus_var_free(var_ptr: *mut Var) {
    unsafe {
        if !var_ptr.is_null() {
            let _ = Box::from_raw(var_ptr);
        }
    }
}

/// Wrap a Var* in a Value*
/// Args: var (Var*)
/// Returns: Value*
#[no_mangle]
pub extern "C" fn clorus_value_from_var(var_ptr: *mut Var) -> *mut Value {
    Value::from_var(var_ptr)
}

/// Extract Var* from a Value*
/// Args: value (Value*)
/// Returns: Var*
#[no_mangle]
pub extern "C" fn clorus_value_as_var(value_ptr: *mut Value) -> *mut Var {
    unsafe {
        if value_ptr.is_null() {
            return std::ptr::null_mut();
        }
        (*value_ptr).as_var()
    }
}

/// Auto-deref helper for symbol reads.
/// If `value_ptr` is a Var, returns its root value; otherwise returns `value_ptr`.
/// The returned value is retained for caller ownership.
#[no_mangle]
pub extern "C" fn clorus_deref_var_value(value_ptr: *mut Value) -> *mut Value {
    unsafe {
        if value_ptr.is_null() {
            return std::ptr::null_mut();
        }

        // Defensive guard against invalid pointers crossing FFI/JIT boundaries.
        // We frequently observe tiny sentinel-like addresses (e.g. 0x8/0x10) when a
        // bad caller passes non-Value data into this helper; dereferencing those
        // pointers causes process crashes.
        let addr = value_ptr as usize;
        if addr < 4096 || (addr & (std::mem::align_of::<Value>() - 1)) != 0 {
            if var_error_logging_enabled() {
                eprintln!(
                    "Attempted to deref invalid value pointer in clorus_deref_var_value: 0x{addr:x}"
                );
            }
            return std::ptr::null_mut();
        }

        let out = if (*value_ptr).tag() == ValueTag::Var {
            let var_ptr = (*value_ptr).as_var();
            clorus_var_get(var_ptr)
        } else {
            value_ptr
        };

        if !out.is_null() {
            crate::value::clorus_retain(out);
        }
        out
    }
}
