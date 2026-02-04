/// Canonical Type System for Clorus FFI
///
/// This is the SINGLE SOURCE OF TRUTH for all type representations.
/// All FFI operations must go through this type system.
///
/// Design Principles:
/// - Type-safe: No string-based types
/// - Explicit: No magic conversions
/// - Validated: All types are validated before use
/// - Documented: Every type has clear semantics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// Re-export mapper module
pub mod mapper;

// ============================================================================
// Core Type System
// ============================================================================

/// Unique identifier for types - ensures type safety for pointers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeId(u64);

impl TypeId {
    /// Create a new unique type ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Check if this type ID is valid (non-zero)
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Canonical FFI type representation
///
/// This represents all types that can cross the FFI boundary.
/// Each variant has clear semantics and memory layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FfiType {
    /// No value - ()
    Void,

    /// Boolean - true/false
    Bool,

    /// 64-bit signed integer
    I64,

    /// 64-bit floating point
    F64,

    /// UTF-8 string (owned)
    ///
    /// Memory: Clorus runtime manages the string
    /// Lifetime: Clorus GC handles cleanup
    String,

    /// Opaque pointer - cannot be dereferenced from Clorus
    ///
    /// Safety: TypeId ensures type safety
    /// Usage: Pass back to Rust functions only
    /// Memory: Rust code manages the allocation
    OpaquePointer {
        /// Unique ID for this pointer type
        type_id: TypeId,
        /// Human-readable type name (for error messages)
        type_name: String,
    },

    /// Structured type with named fields
    Struct {
        name: String,
        fields: Vec<StructField>,
    },

    /// Vector of elements (dynamic array)
    Vector {
        element_type: Box<FfiType>,
    },

    /// Optional value - Some(T) or None
    Option {
        inner: Box<FfiType>,
    },

    /// Result type - Ok(T) or Err(E)
    Result {
        ok_type: Box<FfiType>,
        err_type: Box<FfiType>,
    },

    /// Function pointer (for callbacks)
    Function {
        params: Vec<FfiType>,
        return_type: Box<FfiType>,
    },
}

impl FfiType {
    /// Get the size of this type in bytes
    ///
    /// Returns None for unsized types (String, Vec, etc.)
    pub fn size(&self) -> Option<usize> {
        match self {
            FfiType::Void => Some(0),
            FfiType::Bool => Some(1),
            FfiType::I64 => Some(8),
            FfiType::F64 => Some(8),
            FfiType::OpaquePointer { .. } => Some(8), // Pointer size on 64-bit
            FfiType::String => None, // Unsized
            FfiType::Vector { .. } => None, // Unsized
            FfiType::Struct { fields, .. } => {
                // Sum of all field sizes (simplified - no padding)
                fields.iter()
                    .map(|f| f.ty.size())
                    .collect::<Option<Vec<_>>>()
                    .map(|sizes| sizes.iter().sum())
            }
            FfiType::Option { inner } => inner.size(),
            FfiType::Result { ok_type, err_type } => {
                // Size is max of ok_type and err_type + discriminant
                match (ok_type.size(), err_type.size()) {
                    (Some(ok_size), Some(err_size)) => {
                        Some(ok_size.max(err_size) + 1) // +1 for discriminant
                    }
                    _ => None,
                }
            }
            FfiType::Function { .. } => Some(8), // Function pointer
        }
    }

    /// Check if this type is safe to pass across FFI boundary
    pub fn is_ffi_safe(&self) -> bool {
        match self {
            FfiType::Void | FfiType::Bool | FfiType::I64 | FfiType::F64 => true,
            FfiType::String => true, // Safe with proper conversion
            FfiType::OpaquePointer { .. } => true, // Safe - opaque only
            FfiType::Struct { fields, .. } => {
                // Struct is safe if all fields are safe
                fields.iter().all(|f| f.ty.is_ffi_safe())
            }
            FfiType::Vector { element_type } => element_type.is_ffi_safe(),
            FfiType::Option { inner } => inner.is_ffi_safe(),
            FfiType::Result { ok_type, err_type } => {
                ok_type.is_ffi_safe() && err_type.is_ffi_safe()
            }
            FfiType::Function { .. } => true, // Function pointers are safe
        }
    }

    /// Get a human-readable name for this type
    pub fn display_name(&self) -> String {
        match self {
            FfiType::Void => "()".to_string(),
            FfiType::Bool => "bool".to_string(),
            FfiType::I64 => "i64".to_string(),
            FfiType::F64 => "f64".to_string(),
            FfiType::String => "String".to_string(),
            FfiType::OpaquePointer { type_name, .. } => format!("*{}", type_name),
            FfiType::Struct { name, .. } => name.clone(),
            FfiType::Vector { element_type } => {
                format!("Vec<{}>", element_type.display_name())
            }
            FfiType::Option { inner } => {
                format!("Option<{}>", inner.display_name())
            }
            FfiType::Result { ok_type, err_type } => {
                format!("Result<{}, {}>", ok_type.display_name(), err_type.display_name())
            }
            FfiType::Function { params, return_type } => {
                let param_names: Vec<String> = params.iter()
                    .map(|p| p.display_name())
                    .collect();
                format!("fn({}) -> {}", param_names.join(", "), return_type.display_name())
            }
        }
    }
}

impl fmt::Display for FfiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Field in a struct
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub ty: FfiType,
    /// Offset from start of struct in bytes
    pub offset: usize,
}

// ============================================================================
// Function Signatures
// ============================================================================

/// Complete function signature - validated and ready for codegen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfiFunction {
    /// Function name (Rust side)
    pub name: String,
    /// Parameters with names and types
    pub params: Vec<FfiParam>,
    /// Return type
    pub return_type: FfiType,
    /// Safety level
    pub safety: Safety,
    /// Additional metadata
    pub metadata: FunctionMetadata,
}

impl FfiFunction {
    /// Validate this function signature
    ///
    /// Ensures all types are FFI-safe and function is callable
    pub fn validate(&self) -> Result<(), TypeError> {
        // Check all parameters are FFI-safe
        for param in &self.params {
            if !param.ty.is_ffi_safe() {
                return Err(TypeError::UnsafeType {
                    type_name: param.ty.display_name(),
                    param_name: Some(param.name.clone()),
                    reason: "Type is not safe to pass across FFI boundary".to_string(),
                });
            }
        }

        // Check return type is FFI-safe
        if !self.return_type.is_ffi_safe() {
            return Err(TypeError::UnsafeType {
                type_name: self.return_type.display_name(),
                param_name: None,
                reason: "Return type is not safe to pass across FFI boundary".to_string(),
            });
        }

        Ok(())
    }

    /// Get the arity (number of parameters)
    pub fn arity(&self) -> usize {
        self.params.len()
    }

    /// Check if this function takes no parameters
    pub fn is_nullary(&self) -> bool {
        self.params.is_empty()
    }

    /// Check if this function returns void
    pub fn returns_void(&self) -> bool {
        self.return_type == FfiType::Void
    }
}

/// Function parameter
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfiParam {
    pub name: String,
    pub ty: FfiType,
}

/// Safety level of function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Safety {
    /// Safe - no unsafe operations, memory-safe
    Safe,
    /// Unsafe - contains unsafe operations, caller must ensure safety
    Unsafe,
}

/// Function metadata (for documentation, debugging)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetadata {
    /// Doc comment (if any)
    pub doc_comment: Option<String>,
    /// Source file location
    pub source_file: String,
    /// Line number
    pub line_number: usize,
}

// ============================================================================
// Type Errors
// ============================================================================

/// Type-related errors
#[derive(Debug, thiserror::Error)]
pub enum TypeError {
    #[error("Unsafe type: {type_name} - {reason}")]
    UnsafeType {
        type_name: String,
        param_name: Option<String>,
        reason: String,
    },

    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        expected: String,
        found: String,
    },

    #[error("Unsupported type: {type_name}\n  Reason: {reason}")]
    UnsupportedType {
        type_name: String,
        reason: String,
    },

    #[error("Invalid pointer type: {message}")]
    InvalidPointer {
        message: String,
    },
}

impl TypeError {
    /// Create an unsupported type error with helpful suggestion
    pub fn unsupported_with_suggestion(type_name: String, reason: String, suggestion: String) -> Self {
        Self::UnsupportedType {
            type_name: format!("{}\n  Suggestion: {}", type_name, suggestion),
            reason,
        }
    }

    /// Get a user-friendly error message with suggestions
    pub fn detailed_message(&self) -> String {
        match self {
            TypeError::UnsupportedType { type_name, reason } => {
                format!(
                    "Unsupported FFI type: {}\n\n\
                     Reason: {}\n\n\
                     Supported types:\n\
                     - Primitives: bool, i64, f64\n\
                     - String\n\
                     - Opaque pointers: *mut T, *const T\n\
                     - Vec<T> (coming soon)\n\
                     - Option<T> (coming soon)\n\
                     - Result<T, E> (coming soon)",
                    type_name, reason
                )
            }
            _ => self.to_string(),
        }
    }
}

// ============================================================================
// Type Registry (for pointer types)
// ============================================================================

/// Registry of all known types
///
/// This tracks type IDs for pointer types to ensure type safety
pub struct TypeRegistry {
    next_id: u64,
    types: HashMap<String, TypeId>,
    reverse: HashMap<TypeId, String>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            next_id: 1, // Start from 1 (0 is reserved for invalid)
            types: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// Register a new type and get its TypeId
    ///
    /// If the type is already registered, returns the existing ID
    pub fn register(&mut self, type_name: impl Into<String>) -> TypeId {
        let type_name = type_name.into();

        if let Some(&id) = self.types.get(&type_name) {
            return id;
        }

        let id = TypeId::new(self.next_id);
        self.next_id += 1;

        self.types.insert(type_name.clone(), id);
        self.reverse.insert(id, type_name);

        id
    }

    /// Get the type name for a TypeId
    pub fn get_name(&self, id: TypeId) -> Option<&str> {
        self.reverse.get(&id).map(|s| s.as_str())
    }

    /// Get the TypeId for a type name
    pub fn get_id(&self, type_name: &str) -> Option<TypeId> {
        self.types.get(type_name).copied()
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_sizes() {
        assert_eq!(FfiType::Void.size(), Some(0));
        assert_eq!(FfiType::Bool.size(), Some(1));
        assert_eq!(FfiType::I64.size(), Some(8));
        assert_eq!(FfiType::F64.size(), Some(8));
        assert_eq!(FfiType::String.size(), None); // Unsized
    }

    #[test]
    fn test_ffi_safety() {
        assert!(FfiType::F64.is_ffi_safe());
        assert!(FfiType::String.is_ffi_safe());
        assert!(FfiType::OpaquePointer {
            type_id: TypeId::new(1),
            type_name: "Context".to_string(),
        }.is_ffi_safe());
    }

    #[test]
    fn test_type_registry() {
        let mut registry = TypeRegistry::new();

        let ctx_id = registry.register("Context");
        assert!(ctx_id.is_valid());

        // Registering again should return same ID
        let ctx_id2 = registry.register("Context");
        assert_eq!(ctx_id, ctx_id2);

        // Different type should get different ID
        let window_id = registry.register("Window");
        assert_ne!(ctx_id, window_id);

        // Can look up by name
        assert_eq!(registry.get_id("Context"), Some(ctx_id));
        assert_eq!(registry.get_name(ctx_id), Some("Context"));
    }

    #[test]
    fn test_function_validation() {
        let func = FfiFunction {
            name: "test".to_string(),
            params: vec![
                FfiParam {
                    name: "x".to_string(),
                    ty: FfiType::F64,
                },
            ],
            return_type: FfiType::F64,
            safety: Safety::Safe,
            metadata: FunctionMetadata {
                doc_comment: None,
                source_file: "test.rs".to_string(),
                line_number: 1,
            },
        };

        assert!(func.validate().is_ok());
        assert_eq!(func.arity(), 1);
        assert!(!func.is_nullary());
        assert!(!func.returns_void());
    }

    #[test]
    fn test_display_names() {
        assert_eq!(FfiType::F64.display_name(), "f64");
        assert_eq!(FfiType::String.display_name(), "String");

        let ptr = FfiType::OpaquePointer {
            type_id: TypeId::new(1),
            type_name: "Context".to_string(),
        };
        assert_eq!(ptr.display_name(), "*Context");

        let vec = FfiType::Vector {
            element_type: Box::new(FfiType::F64),
        };
        assert_eq!(vec.display_name(), "Vec<f64>");
    }
}
