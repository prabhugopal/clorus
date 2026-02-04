//! Clorus Code Generation - LLVM Backend
//!
//! This crate provides LLVM-based code generation for Clorus:
//! - Compile AST to LLVM IR
//! - JIT execution
//! - Variable and function management

pub mod codegen;
pub mod namespace_context;
pub mod ffi_codegen;

// Re-export main types
pub use codegen::CodeGen;
pub use namespace_context::NamespaceContext;
pub use ffi_codegen::{FfiDeclarations, FfiTypeMapper};
