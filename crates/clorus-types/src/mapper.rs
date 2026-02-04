/// Type Mapper - Bidirectional conversion between Rust, FFI, and Clorus types
///
/// This module provides the mapping layer between:
/// - Rust types (parsed from syn)
/// - Canonical FFI types (FfiType)
/// - Clorus runtime types
///
/// Design: Single source of truth - all mappings go through this module

use crate::{FfiType, TypeError, TypeRegistry};

/// Represents a Rust type parsed from function signatures
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustType {
    /// Unit type - ()
    Unit,
    /// Boolean - bool
    Bool,
    /// Signed integer types
    I32,
    I64,
    /// Floating point types
    F32,
    F64,
    /// String type
    String,
    /// Raw pointer - *mut T or *const T
    RawPointer {
        mutable: bool,
        pointee: Box<RustType>,
    },
    /// Named type (struct, enum, etc.)
    Named(String),
    /// Vector - Vec<T>
    Vec(Box<RustType>),
    /// Option - Option<T>
    Option(Box<RustType>),
    /// Result - Result<T, E>
    Result {
        ok: Box<RustType>,
        err: Box<RustType>,
    },
}

impl RustType {
    /// Parse a simple Rust type string into RustType
    ///
    /// This is a simple parser for basic types. For complex types,
    /// use syn-based parsing.
    pub fn from_string(s: &str) -> Result<Self, TypeError> {
        match s.trim() {
            "()" => Ok(RustType::Unit),
            "bool" => Ok(RustType::Bool),
            "i32" => Ok(RustType::I32),
            "i64" => Ok(RustType::I64),
            "f32" => Ok(RustType::F32),
            "f64" => Ok(RustType::F64),
            "String" => Ok(RustType::String),
            "*mut u8" => Ok(RustType::RawPointer {
                mutable: true,
                pointee: Box::new(RustType::Named("u8".to_string())),
            }),
            "*const u8" => Ok(RustType::RawPointer {
                mutable: false,
                pointee: Box::new(RustType::Named("u8".to_string())),
            }),
            other => {
                // Check if it's a pointer type
                if other.starts_with("*mut ") {
                    let pointee = other.trim_start_matches("*mut ").trim();
                    Ok(RustType::RawPointer {
                        mutable: true,
                        pointee: Box::new(RustType::Named(pointee.to_string())),
                    })
                } else if other.starts_with("*const ") {
                    let pointee = other.trim_start_matches("*const ").trim();
                    Ok(RustType::RawPointer {
                        mutable: false,
                        pointee: Box::new(RustType::Named(pointee.to_string())),
                    })
                } else if other.starts_with("Vec<") && other.ends_with('>') {
                    let inner = other.trim_start_matches("Vec<").trim_end_matches('>');
                    let inner_type = RustType::from_string(inner)?;
                    Ok(RustType::Vec(Box::new(inner_type)))
                } else if other.starts_with("Option<") && other.ends_with('>') {
                    let inner = other.trim_start_matches("Option<").trim_end_matches('>');
                    let inner_type = RustType::from_string(inner)?;
                    Ok(RustType::Option(Box::new(inner_type)))
                } else {
                    // Unknown type - treat as named type
                    Ok(RustType::Named(other.to_string()))
                }
            }
        }
    }
}

/// Type mapper - converts between Rust, FFI, and Clorus types
pub struct TypeMapper {
    /// Registry for pointer types
    registry: TypeRegistry,
}

impl TypeMapper {
    /// Create a new type mapper
    pub fn new() -> Self {
        Self {
            registry: TypeRegistry::new(),
        }
    }

    /// Map Rust type to canonical FFI type
    ///
    /// This is the core mapping function that converts Rust types
    /// to the canonical FFI representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use clorus_types::mapper::{TypeMapper, RustType};
    /// use clorus_types::FfiType;
    ///
    /// let mut mapper = TypeMapper::new();
    /// assert_eq!(
    ///     mapper.from_rust(&RustType::F64).unwrap(),
    ///     FfiType::F64
    /// );
    /// ```
    pub fn from_rust(&mut self, rust_ty: &RustType) -> Result<FfiType, TypeError> {
        match rust_ty {
            RustType::Unit => Ok(FfiType::Void),
            RustType::Bool => Ok(FfiType::Bool),
            RustType::I32 => {
                // For FFI, promote i32 to i64 for consistency
                // (we use i64 as the standard integer type)
                Ok(FfiType::I64)
            }
            RustType::I64 => Ok(FfiType::I64),
            RustType::F32 => {
                // For FFI, promote f32 to f64 for consistency
                Ok(FfiType::F64)
            }
            RustType::F64 => Ok(FfiType::F64),
            RustType::String => Ok(FfiType::String),

            RustType::RawPointer { mutable: _, pointee } => {
                // For opaque pointers, we need to register the type
                let type_name = match &**pointee {
                    RustType::Named(name) => name.clone(),
                    RustType::I32 | RustType::I64 | RustType::F32 | RustType::F64 => {
                        // Primitive pointer - use generic u8
                        "u8".to_string()
                    }
                    other => {
                        return Err(TypeError::UnsupportedType {
                            type_name: format!("{:?}", other),
                            reason: "Complex pointer types not supported".to_string(),
                        });
                    }
                };

                let type_id = self.registry.register(&type_name);

                Ok(FfiType::OpaquePointer {
                    type_id,
                    type_name,
                })
            }

            RustType::Vec(element) => {
                let element_ffi = self.from_rust(element)?;
                Ok(FfiType::Vector {
                    element_type: Box::new(element_ffi),
                })
            }

            RustType::Option(inner) => {
                let inner_ffi = self.from_rust(inner)?;
                Ok(FfiType::Option {
                    inner: Box::new(inner_ffi),
                })
            }

            RustType::Result { ok, err } => {
                let ok_ffi = self.from_rust(ok)?;
                let err_ffi = self.from_rust(err)?;
                Ok(FfiType::Result {
                    ok_type: Box::new(ok_ffi),
                    err_type: Box::new(err_ffi),
                })
            }

            RustType::Named(name) => {
                // Named types are treated as opaque handles
                let type_id = self.registry.register(name);
                Ok(FfiType::OpaquePointer {
                    type_id,
                    type_name: name.clone(),
                })
            }
        }
    }

    /// Get the type registry (for accessing TypeIds)
    pub fn registry(&self) -> &TypeRegistry {
        &self.registry
    }

    /// Get mutable access to registry
    pub fn registry_mut(&mut self) -> &mut TypeRegistry {
        &mut self.registry
    }
}

impl Default for TypeMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_type_from_string() {
        assert_eq!(RustType::from_string("f64").unwrap(), RustType::F64);
        assert_eq!(RustType::from_string("String").unwrap(), RustType::String);
        assert_eq!(RustType::from_string("()").unwrap(), RustType::Unit);
        assert_eq!(RustType::from_string("bool").unwrap(), RustType::Bool);

        // Pointer types
        match RustType::from_string("*mut Context").unwrap() {
            RustType::RawPointer { mutable, pointee } => {
                assert!(mutable);
                assert_eq!(*pointee, RustType::Named("Context".to_string()));
            }
            _ => panic!("Expected RawPointer"),
        }
    }

    #[test]
    fn test_mapper_primitives() {
        let mut mapper = TypeMapper::new();

        assert_eq!(
            mapper.from_rust(&RustType::F64).unwrap(),
            FfiType::F64
        );
        assert_eq!(
            mapper.from_rust(&RustType::String).unwrap(),
            FfiType::String
        );
        assert_eq!(
            mapper.from_rust(&RustType::Bool).unwrap(),
            FfiType::Bool
        );
    }

    #[test]
    fn test_mapper_pointers() {
        let mut mapper = TypeMapper::new();

        let rust_ty = RustType::RawPointer {
            mutable: true,
            pointee: Box::new(RustType::Named("Context".to_string())),
        };

        match mapper.from_rust(&rust_ty).unwrap() {
            FfiType::OpaquePointer { type_id, type_name } => {
                assert!(type_id.is_valid());
                assert_eq!(type_name, "Context");
            }
            _ => panic!("Expected OpaquePointer"),
        }
    }

    #[test]
    fn test_mapper_vectors() {
        let mut mapper = TypeMapper::new();

        let rust_ty = RustType::Vec(Box::new(RustType::F64));

        match mapper.from_rust(&rust_ty).unwrap() {
            FfiType::Vector { element_type } => {
                assert_eq!(*element_type, FfiType::F64);
            }
            _ => panic!("Expected Vector"),
        }
    }

    #[test]
    fn test_mapper_option() {
        let mut mapper = TypeMapper::new();

        let rust_ty = RustType::Option(Box::new(RustType::String));

        match mapper.from_rust(&rust_ty).unwrap() {
            FfiType::Option { inner } => {
                assert_eq!(*inner, FfiType::String);
            }
            _ => panic!("Expected Option"),
        }
    }

    #[test]
    fn test_type_id_consistency() {
        let mut mapper = TypeMapper::new();

        // Register same type twice - should get same TypeId
        let ctx1 = RustType::RawPointer {
            mutable: true,
            pointee: Box::new(RustType::Named("Context".to_string())),
        };

        let ctx2 = RustType::RawPointer {
            mutable: true,
            pointee: Box::new(RustType::Named("Context".to_string())),
        };

        let ffi1 = mapper.from_rust(&ctx1).unwrap();
        let ffi2 = mapper.from_rust(&ctx2).unwrap();

        match (ffi1, ffi2) {
            (
                FfiType::OpaquePointer { type_id: id1, .. },
                FfiType::OpaquePointer { type_id: id2, .. },
            ) => {
                assert_eq!(id1, id2, "Same type should get same TypeId");
            }
            _ => panic!("Expected OpaquePointer"),
        }
    }
}
