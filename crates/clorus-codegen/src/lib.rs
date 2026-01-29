//! Clorus Code Generation - LLVM Backend
//!
//! This crate provides LLVM-based code generation for Clorus:
//! - Compile AST to LLVM IR
//! - JIT execution
//! - Variable and function management

pub mod codegen;
pub mod namespace_context;

// Re-export main types
pub use codegen::CodeGen;
pub use namespace_context::NamespaceContext;
