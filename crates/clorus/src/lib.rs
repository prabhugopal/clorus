//! Clorus - Clojure-inspired Systems Programming Language
//!
//! This is the main library crate that re-exports all functionality.
//!
//! ## Architecture
//!
//! - `clorus-syntax` - Lexer, Parser, AST (no dependencies)
//! - `clorus-codegen` - LLVM code generation
//! - `clorus` - This crate (re-exports everything)

// Module system
pub mod module_loader;
pub mod namespace;

// Re-export syntax
pub use clorus_syntax::{ast, lexer, parser, parse, parse_and_expand, expand_macros, Expr, Lexer, Parser, Token, RequireSpec, RustImport};

// Re-export codegen
pub use clorus_codegen::{codegen, CodeGen};
pub use clorus_codegen::codegen::{RustLibrary, RustFunction, RustParam};

// Re-export module system
pub use module_loader::ModuleLoader;
pub use namespace::NamespaceContext;

// Re-export inkwell for convenience
pub use inkwell;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end_to_end() {
        let exprs = parse("(+ 1 2)").unwrap();
        assert_eq!(exprs.len(), 1);
    }
}
