/// Var (variable) support for dynamic bindings
/// Vars are first-class objects that hold a root value and metadata
use crate::value::{Value, ValueTag};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
        meta.insert(key, value);
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
    unsafe {
        if var_ptr.is_null() {
            return std::ptr::null_mut();
        }
        let var = &*var_ptr;
        var.get_root()
    }
}

/// Set Var's root value (alter-var-root)
/// Args: var (Var*), new_value (Value*)
/// Returns: new_value (Value*)
#[no_mangle]
pub extern "C" fn clorus_var_set(var_ptr: *mut Var, new_value: *mut Value) -> *mut Value {
    unsafe {
        if var_ptr.is_null() {
            return std::ptr::null_mut();
        }
        let var = &*var_ptr;
        var.set_root(new_value);
        new_value
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
