//! Clorus Syntax - Lexer, Parser, and AST
//!
//! This crate provides the frontend for the Clorus language:
//! - Lexical analysis (tokenization)
//! - Syntax parsing (S-expressions)
//! - Abstract Syntax Tree representation
//!
//! This crate has no external dependencies and can be used standalone
//! for syntax highlighting, linting, or other tooling.

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod macros;

// Re-export main types
pub use ast::{Expr, RequireSpec, RustImport, Pattern, MapPatternKey};
pub use lexer::{Lexer, Token};
pub use parser::{parse_str, Parser};
pub use macros::{expand_macros, expand_macros_sequence};

/// Convenience function to parse source code
pub fn parse(source: &str) -> Result<Vec<Expr>, String> {
    parse_str(source)
}

/// Convenience function to parse and expand macros
pub fn parse_and_expand(source: &str) -> Result<Vec<Expr>, String> {
    let exprs = parse_str(source)?;
    Ok(expand_macros_sequence(&exprs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integration() {
        let exprs = parse("(+ 1 2)").unwrap();
        assert_eq!(exprs.len(), 1);
    }
}
