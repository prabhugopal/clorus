/// Type-safe FFI codegen using canonical type system
///
/// This module generates LLVM IR for FFI function calls using
/// the canonical FfiFunction type system.

use clorus_types::{FfiFunction, FfiType, FfiParam};
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{FloatValue, PointerValue, BasicMetadataValueEnum};
use inkwell::types::{BasicMetadataTypeEnum, AnyTypeEnum};
use inkwell::AddressSpace;

/// Helper for generating LLVM types from canonical FFI types
pub struct FfiTypeMapper<'ctx> {
    context: &'ctx Context,
}

impl<'ctx> FfiTypeMapper<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { context }
    }

    /// Convert FfiType to LLVM type for function parameters
    pub fn to_llvm_param_type(&self, ffi_type: &FfiType) -> Result<BasicMetadataTypeEnum<'ctx>, String> {
        match ffi_type {
            FfiType::Void => Err("Void cannot be a parameter type".to_string()),
            FfiType::Bool => Ok(self.context.bool_type().into()),
            FfiType::I64 => Ok(self.context.i64_type().into()),
            FfiType::F64 => Ok(self.context.f64_type().into()),
            FfiType::String => {
                // String is passed as i8*
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::OpaquePointer { .. } => {
                // All opaque pointers are i8* in LLVM
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::Vector { .. } => {
                // Vectors are passed as pointers to runtime structures
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::Option { .. } => {
                // Options are passed as pointers to runtime structures
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::Result { .. } => {
                // Results are passed as pointers to runtime structures
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::Struct { .. } => {
                // Structs are passed as pointers
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::Function { .. } => {
                // Function pointers
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
        }
    }

    /// Convert FfiType to LLVM return type
    pub fn to_llvm_return_type(&self, ffi_type: &FfiType) -> Result<AnyTypeEnum<'ctx>, String> {
        match ffi_type {
            FfiType::Void => Ok(self.context.void_type().into()),
            FfiType::Bool => Ok(self.context.bool_type().into()),
            FfiType::I64 => Ok(self.context.i64_type().into()),
            FfiType::F64 => Ok(self.context.f64_type().into()),
            FfiType::String => {
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::OpaquePointer { .. } => {
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::Vector { .. } => {
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::Option { .. } => {
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::Result { .. } => {
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::Struct { .. } => {
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            FfiType::Function { .. } => {
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
        }
    }
}

/// FFI function declarations for LLVM module
pub struct FfiDeclarations<'ctx> {
    type_mapper: FfiTypeMapper<'ctx>,
}

impl<'ctx> FfiDeclarations<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self {
            type_mapper: FfiTypeMapper::new(context),
        }
    }

    /// Declare a single FFI function in the LLVM module
    pub fn declare_function(&self, module: &Module<'ctx>, func: &FfiFunction) -> Result<(), String> {
        // Convert parameters to LLVM types
        let mut param_types = Vec::new();
        for param in &func.params {
            let llvm_type = self.type_mapper.to_llvm_param_type(&param.ty)?;
            param_types.push(llvm_type);
        }

        // Convert return type
        let return_type = self.type_mapper.to_llvm_return_type(&func.return_type)?;

        // Create function type
        let fn_type = match return_type {
            AnyTypeEnum::VoidType(void_ty) => void_ty.fn_type(&param_types, false),
            AnyTypeEnum::IntType(int_ty) => int_ty.fn_type(&param_types, false),
            AnyTypeEnum::FloatType(float_ty) => float_ty.fn_type(&param_types, false),
            AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.fn_type(&param_types, false),
            _ => return Err(format!("Unsupported return type: {:?}", return_type)),
        };

        // Declare function with clorus_ prefix
        let ffi_name = format!("clorus_{}", func.name);
        module.add_function(&ffi_name, fn_type, None);

        Ok(())
    }

    /// Declare all functions from a library
    pub fn declare_all(&self, module: &Module<'ctx>, functions: &[FfiFunction]) -> Result<(), String> {
        for func in functions {
            self.declare_function(module, func)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clorus_types::{FfiType, FfiFunction, FfiParam, Safety, FunctionMetadata, TypeId};

    #[test]
    fn test_type_mapping_primitives() {
        let context = Context::create();
        let mapper = FfiTypeMapper::new(&context);

        // Test primitive types
        assert!(mapper.to_llvm_param_type(&FfiType::F64).is_ok());
        assert!(mapper.to_llvm_param_type(&FfiType::I64).is_ok());
        assert!(mapper.to_llvm_param_type(&FfiType::Bool).is_ok());
        assert!(mapper.to_llvm_param_type(&FfiType::String).is_ok());
    }

    #[test]
    fn test_type_mapping_pointers() {
        let context = Context::create();
        let mapper = FfiTypeMapper::new(&context);

        let pointer_type = FfiType::OpaquePointer {
            type_id: TypeId::new(1),
            type_name: "Context".to_string(),
        };

        assert!(mapper.to_llvm_param_type(&pointer_type).is_ok());
        assert!(mapper.to_llvm_return_type(&pointer_type).is_ok());
    }

    #[test]
    fn test_declare_function() {
        let context = Context::create();
        let module = context.create_module("test");

        let func = FfiFunction {
            name: "move_to".to_string(),
            params: vec![
                FfiParam {
                    name: "ctx".to_string(),
                    ty: FfiType::OpaquePointer {
                        type_id: TypeId::new(1),
                        type_name: "Context".to_string(),
                    },
                },
                FfiParam {
                    name: "x".to_string(),
                    ty: FfiType::F64,
                },
                FfiParam {
                    name: "y".to_string(),
                    ty: FfiType::F64,
                },
            ],
            return_type: FfiType::Void,
            safety: Safety::Safe,
            metadata: FunctionMetadata {
                doc_comment: None,
                source_file: "test.rs".to_string(),
                line_number: 0,
            },
        };

        let declarations = FfiDeclarations::new(&context);
        assert!(declarations.declare_function(&module, &func).is_ok());

        // Verify function was declared
        assert!(module.get_function("clorus_move_to").is_some());
    }

    #[test]
    fn test_declare_function_with_pointer_return() {
        let context = Context::create();
        let module = context.create_module("test");

        let func = FfiFunction {
            name: "create_context".to_string(),
            params: vec![
                FfiParam {
                    name: "width".to_string(),
                    ty: FfiType::F64,
                },
                FfiParam {
                    name: "height".to_string(),
                    ty: FfiType::F64,
                },
            ],
            return_type: FfiType::OpaquePointer {
                type_id: TypeId::new(1),
                type_name: "Context".to_string(),
            },
            safety: Safety::Safe,
            metadata: FunctionMetadata {
                doc_comment: None,
                source_file: "test.rs".to_string(),
                line_number: 0,
            },
        };

        let declarations = FfiDeclarations::new(&context);
        assert!(declarations.declare_function(&module, &func).is_ok());

        // Verify function was declared
        let llvm_func = module.get_function("clorus_create_context");
        assert!(llvm_func.is_some());
    }
}
