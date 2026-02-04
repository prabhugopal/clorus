/// Modern FFI Analyzer using canonical type system
///
/// This module provides type-safe FFI analysis using the clorus-types
/// canonical type system instead of string-based types.

use clorus_types::{FfiFunction, FfiParam, FfiType, Safety, FunctionMetadata, TypeError};
use clorus_types::mapper::{TypeMapper, RustType};
use std::fs;
use std::path::Path;
use syn::{Item, ItemFn, FnArg, ReturnType, Type, parse_file};

pub struct FfiAnalyzer {
    /// Type mapper for Rust → FFI type conversion
    mapper: TypeMapper,
    /// Analyzed functions with canonical types
    pub functions: Vec<FfiFunction>,
}

impl FfiAnalyzer {
    pub fn new() -> Self {
        Self {
            mapper: TypeMapper::new(),
            functions: Vec::new(),
        }
    }

    /// Parse a Rust source file and extract public functions with canonical types
    pub fn parse_file(&mut self, path: &Path) -> Result<(), String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let ast = parse_file(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        for item in ast.items {
            if let Item::Fn(func) = item {
                // Only process public functions
                if self.is_public(&func) {
                    match self.extract_ffi_function(&func, path) {
                        Ok(Some(ffi_func)) => {
                            self.functions.push(ffi_func);
                        }
                        Ok(None) => {
                            // Function was skipped (async, extern "C", etc.)
                        }
                        Err(e) => {
                            eprintln!("Warning: Skipping function {} due to error: {}",
                                     func.sig.ident, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn is_public(&self, func: &ItemFn) -> bool {
        matches!(func.vis, syn::Visibility::Public(_))
    }

    /// Extract function information with canonical type system
    fn extract_ffi_function(&mut self, func: &ItemFn, source_file: &Path) -> Result<Option<FfiFunction>, TypeError> {
        // Skip async functions
        if func.sig.asyncness.is_some() {
            return Ok(None);
        }

        // Skip extern "C" functions
        if let Some(abi) = &func.sig.abi {
            if abi.name.as_ref().map(|n| n.value()) == Some("C".to_string()) {
                return Ok(None);
            }
        }

        let name = func.sig.ident.to_string();

        // Extract parameters with canonical types
        let mut params = Vec::new();
        for input in &func.sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                if let syn::Pat::Ident(ident) = &*pat_type.pat {
                    let param_name = ident.ident.to_string();

                    // Convert syn Type to our RustType
                    let rust_type = self.syn_type_to_rust_type(&pat_type.ty)?;

                    // Convert RustType to canonical FfiType
                    let ffi_type = self.mapper.from_rust(&rust_type)?;

                    params.push(FfiParam {
                        name: param_name,
                        ty: ffi_type,
                    });
                }
            }
        }

        // Extract return type
        let return_type = match &func.sig.output {
            ReturnType::Default => FfiType::Void,
            ReturnType::Type(_, ty) => {
                let rust_type = self.syn_type_to_rust_type(ty)?;
                self.mapper.from_rust(&rust_type)?
            }
        };

        // Determine safety level
        let safety = if func.sig.unsafety.is_some() {
            Safety::Unsafe
        } else {
            Safety::Safe
        };

        // Extract documentation
        let doc_comment = func.attrs.iter()
            .filter_map(|attr| {
                if attr.path().is_ident("doc") {
                    attr.meta.require_name_value().ok()
                        .and_then(|nv| match &nv.value {
                            syn::Expr::Lit(lit) => {
                                if let syn::Lit::Str(s) = &lit.lit {
                                    Some(s.value())
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let ffi_func = FfiFunction {
            name,
            params,
            return_type,
            safety,
            metadata: FunctionMetadata {
                doc_comment: if doc_comment.is_empty() { None } else { Some(doc_comment) },
                source_file: source_file.display().to_string(),
                line_number: 0, // proc_macro2::Span doesn't provide line info in stable Rust
            },
        };

        // Validate the function
        ffi_func.validate()?;

        Ok(Some(ffi_func))
    }

    /// Convert syn::Type to our RustType representation
    fn syn_type_to_rust_type(&self, ty: &Type) -> Result<RustType, TypeError> {
        match ty {
            Type::Path(type_path) => {
                let path = &type_path.path;

                // Handle simple types
                if path.segments.len() == 1 {
                    let segment = &path.segments[0];
                    let type_name = segment.ident.to_string();

                    // Check for generic types
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        // Handle Vec<T>
                        if type_name == "Vec" {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                let inner_rust = self.syn_type_to_rust_type(inner_ty)?;
                                return Ok(RustType::Vec(Box::new(inner_rust)));
                            }
                        }
                        // Handle Option<T>
                        else if type_name == "Option" {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                let inner_rust = self.syn_type_to_rust_type(inner_ty)?;
                                return Ok(RustType::Option(Box::new(inner_rust)));
                            }
                        }
                        // Handle Result<T, E>
                        else if type_name == "Result" {
                            let mut args_iter = args.args.iter();
                            if let (Some(syn::GenericArgument::Type(ok_ty)), Some(syn::GenericArgument::Type(err_ty))) =
                                (args_iter.next(), args_iter.next()) {
                                let ok_rust = self.syn_type_to_rust_type(ok_ty)?;
                                let err_rust = self.syn_type_to_rust_type(err_ty)?;
                                return Ok(RustType::Result {
                                    ok: Box::new(ok_rust),
                                    err: Box::new(err_rust),
                                });
                            }
                        }
                    }

                    // Simple named type - use from_string
                    RustType::from_string(&type_name)
                } else {
                    // Complex path - treat as named type using last segment
                    let last = path.segments.last().unwrap();
                    Ok(RustType::Named(last.ident.to_string()))
                }
            }

            Type::Ptr(type_ptr) => {
                // Raw pointer: *mut T or *const T
                let mutable = matches!(type_ptr.mutability, Some(_));
                let pointee = self.syn_type_to_rust_type(&type_ptr.elem)?;
                Ok(RustType::RawPointer {
                    mutable,
                    pointee: Box::new(pointee),
                })
            }

            Type::Tuple(type_tuple) => {
                // Unit type
                if type_tuple.elems.is_empty() {
                    Ok(RustType::Unit)
                } else {
                    Err(TypeError::UnsupportedType {
                        type_name: "tuple".to_string(),
                        reason: "Tuple types (except unit) are not supported in FFI".to_string(),
                    })
                }
            }

            _ => {
                Err(TypeError::UnsupportedType {
                    type_name: "complex_type".to_string(),
                    reason: "Complex type not supported in FFI".to_string(),
                })
            }
        }
    }

    /// Generate JSON metadata for Clorus runtime
    ///
    /// This generates a JSON file that the Clorus codegen can load
    /// to understand the FFI function signatures.
    pub fn generate_metadata_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.functions)
    }

    /// Get the type mapper (for accessing type registry)
    pub fn mapper(&self) -> &TypeMapper {
        &self.mapper
    }
}

impl Default for FfiAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_primitives() {
        let temp_file = "/tmp/test_primitives.rs";
        let mut file = std::fs::File::create(temp_file).unwrap();
        file.write_all(b"pub fn add(x: f64, y: f64) -> f64 { x + y }").unwrap();

        let mut analyzer = FfiAnalyzer::new();
        analyzer.parse_file(Path::new(temp_file)).unwrap();

        assert_eq!(analyzer.functions.len(), 1);
        let func = &analyzer.functions[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].ty, FfiType::F64);
        assert_eq!(func.params[1].ty, FfiType::F64);
        assert_eq!(func.return_type, FfiType::F64);
    }

    #[test]
    fn test_parse_pointer() {
        let temp_file = "/tmp/test_pointer.rs";
        let mut file = std::fs::File::create(temp_file).unwrap();
        file.write_all(b"pub fn create_context(width: f64) -> *mut u8 { std::ptr::null_mut() }").unwrap();

        let mut analyzer = FfiAnalyzer::new();
        analyzer.parse_file(Path::new(temp_file)).unwrap();

        assert_eq!(analyzer.functions.len(), 1);
        let func = &analyzer.functions[0];
        assert_eq!(func.name, "create_context");

        match &func.return_type {
            FfiType::OpaquePointer { type_id, type_name } => {
                assert!(type_id.is_valid());
                assert_eq!(type_name, "u8");
            }
            _ => panic!("Expected OpaquePointer return type"),
        }
    }

    #[test]
    fn test_parse_multi_arg_with_pointer() {
        let temp_file = "/tmp/test_multi_arg.rs";
        let mut file = std::fs::File::create(temp_file).unwrap();
        file.write_all(b"pub fn move_to(ctx: *mut u8, x: f64, y: f64) { }").unwrap();

        let mut analyzer = FfiAnalyzer::new();
        analyzer.parse_file(Path::new(temp_file)).unwrap();

        assert_eq!(analyzer.functions.len(), 1);
        let func = &analyzer.functions[0];
        assert_eq!(func.name, "move_to");
        assert_eq!(func.params.len(), 3);

        // First param should be opaque pointer
        match &func.params[0].ty {
            FfiType::OpaquePointer { .. } => {}
            _ => panic!("Expected OpaquePointer for ctx param"),
        }

        // Second and third should be f64
        assert_eq!(func.params[1].ty, FfiType::F64);
        assert_eq!(func.params[2].ty, FfiType::F64);
        assert_eq!(func.return_type, FfiType::Void);
    }
}
