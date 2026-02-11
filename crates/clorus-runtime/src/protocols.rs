/// Protocol dispatch runtime for Clorus
/// Tracks which types implement which protocols for automatic method dispatch

use std::collections::HashMap;
use std::sync::RwLock;
use std::ffi::{CStr, c_char};
use once_cell::sync::Lazy;

/// Global protocol registry
/// Maps (TypeName, ProtocolName, MethodName) -> FunctionPointer
static PROTOCOL_REGISTRY: Lazy<RwLock<ProtocolRegistry>> = Lazy::new(|| {
    RwLock::new(ProtocolRegistry::new())
});

/// Protocol registry for type -> protocol -> method lookups
pub struct ProtocolRegistry {
    /// Maps (type_name, protocol_name, method_name) -> function pointer
    methods: HashMap<(String, String, String), usize>,
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        ProtocolRegistry {
            methods: HashMap::new(),
        }
    }

    /// Register a protocol method implementation
    pub fn register_method(
        &mut self,
        type_name: String,
        protocol_name: String,
        method_name: String,
        fn_ptr: usize,
    ) {
        let key = (type_name, protocol_name, method_name);
        self.methods.insert(key, fn_ptr);
    }

    /// Lookup a protocol method for a given type
    pub fn lookup_method(
        &self,
        type_name: &str,
        protocol_name: &str,
        method_name: &str,
    ) -> Option<usize> {
        let key = (type_name.to_string(), protocol_name.to_string(), method_name.to_string());
        self.methods.get(&key).copied()
    }
}

/// Register a protocol method implementation
/// Called during compilation of deftype/extend-type
#[no_mangle]
pub extern "C" fn clorus_register_protocol_method(
    type_name: *const c_char,
    protocol_name: *const c_char,
    method_name: *const c_char,
    fn_ptr: usize,
) {
    unsafe {
        if type_name.is_null() || protocol_name.is_null() || method_name.is_null() {
            eprintln!("ERROR: null pointer in clorus_register_protocol_method");
            return;
        }

        let type_str = match CStr::from_ptr(type_name).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                eprintln!("ERROR: invalid UTF-8 in type_name");
                return;
            }
        };

        let protocol_str = match CStr::from_ptr(protocol_name).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                eprintln!("ERROR: invalid UTF-8 in protocol_name");
                return;
            }
        };

        let method_str = match CStr::from_ptr(method_name).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                eprintln!("ERROR: invalid UTF-8 in method_name");
                return;
            }
        };

        match PROTOCOL_REGISTRY.write() {
            Ok(mut registry) => {
                registry.register_method(type_str, protocol_str, method_str, fn_ptr);
            }
            Err(_) => {
                eprintln!("ERROR: failed to acquire write lock on protocol registry");
            }
        }
    }
}

/// Lookup a protocol method for a given type
/// Returns function pointer or 0 if not found
#[no_mangle]
pub extern "C" fn clorus_lookup_protocol_method(
    type_name: *const c_char,
    protocol_name: *const c_char,
    method_name: *const c_char,
) -> usize {
    unsafe {
        if type_name.is_null() || protocol_name.is_null() || method_name.is_null() {
            return 0;
        }

        let type_str = match CStr::from_ptr(type_name).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        let protocol_str = match CStr::from_ptr(protocol_name).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        let method_str = match CStr::from_ptr(method_name).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        match PROTOCOL_REGISTRY.read() {
            Ok(registry) => {
                let result = registry.lookup_method(type_str, protocol_str, method_str).unwrap_or(0);
                if result == 0 {
                    eprintln!("[PROTOCOL] Lookup FAILED: type='{}', protocol='{}', method='{}'", type_str, protocol_str, method_str);
                }
                result
            }
            Err(_) => 0,
        }
    }
}
