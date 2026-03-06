/// Interface file (.clri or .clorus-ffi) parser
/// Preferred extension: .clri, Legacy: .clorus-ffi
use clorus_syntax::{Lexer, Token};
use clorus_syntax::lexer::Span;
use std::fs;
use std::path::Path;

/// Represents a parsed interface file
#[derive(Debug, Clone)]
pub struct InterfaceFile {
    pub name: String,
    pub functions: Vec<InterfaceFunction>,
}

/// Represents a function declaration in an interface file
#[derive(Debug, Clone)]
pub struct InterfaceFunction {
    pub name: String,
    pub params: Vec<InterfaceParam>,
    pub return_type: String,
    pub doc: Option<String>,
    pub rust_symbol: Option<String>,
}

/// Represents a parameter in a function declaration
#[derive(Debug, Clone)]
pub struct InterfaceParam {
    pub name: String,
    pub type_name: String,
}

struct InterfaceTokenParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl InterfaceTokenParser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> Token {
        self.tokens
            .get(self.pos)
            .cloned()
            .unwrap_or(Token::Eof(Span::dummy()))
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn expect_lparen(&mut self) -> Result<(), String> {
        match self.current() {
            Token::LParen(_) => {
                self.advance();
                Ok(())
            }
            _ => Err(self.expected_token_error("'('")),
        }
    }

    fn expect_rparen(&mut self) -> Result<(), String> {
        match self.current() {
            Token::RParen(_) => {
                self.advance();
                Ok(())
            }
            _ => Err(self.expected_token_error("')'")),
        }
    }

    fn expect_lbracket(&mut self) -> Result<(), String> {
        match self.current() {
            Token::LBracket(_) => {
                self.advance();
                Ok(())
            }
            _ => Err(self.expected_token_error("'['")),
        }
    }

    fn expect_rbracket(&mut self) -> Result<(), String> {
        match self.current() {
            Token::RBracket(_) => {
                self.advance();
                Ok(())
            }
            _ => Err(self.expected_token_error("']'")),
        }
    }

    fn expect_symbol(&mut self) -> Result<String, String> {
        match self.current() {
            Token::Symbol(s, _) => {
                let out = s;
                self.advance();
                Ok(out)
            }
            _ => Err(self.expected_token_error("symbol")),
        }
    }

    fn expect_keyword(&mut self) -> Result<String, String> {
        match self.current() {
            Token::Keyword(k, _) => {
                let out = k;
                self.advance();
                Ok(out)
            }
            _ => Err(self.expected_token_error("keyword")),
        }
    }

    fn expect_interface_type(&mut self) -> Result<String, String> {
        let keyword = self.expect_keyword()?;

        // Accept split pointer syntax in interface files:
        //   :*const u8  /  :*mut u8
        // and keep supporting compact keyword form:
        //   :*const-u8  /  :*mut-u8
        let raw_type = if keyword == "*const" || keyword == "*mut" {
            match self.current() {
                Token::Symbol(sym, _) => {
                    let pointee = sym;
                    self.advance();
                    format!("{} {}", keyword, pointee)
                }
                _ => {
                    return Err(format!(
                        "Expected pointee type after :{} (e.g. :{}-u8 or :{} u8)",
                        keyword, keyword, keyword
                    ));
                }
            }
        } else {
            keyword
        };

        Ok(map_type_keyword(&raw_type))
    }

    fn expected_token_error(&self, expected: &str) -> String {
        let current = self.current();
        let span = current.span();
        let (line, column) = (span.line, span.column);

        format!(
            "Expected {} at line {}, column {}, found {}",
            expected,
            line,
            column,
            Self::token_kind_name(&current)
        )
    }

    fn token_kind_name(token: &Token) -> &'static str {
        match token {
            Token::LParen(_) => "LParen",
            Token::RParen(_) => "RParen",
            Token::LBracket(_) => "LBracket",
            Token::RBracket(_) => "RBracket",
            Token::LBrace(_) => "LBrace",
            Token::RBrace(_) => "RBrace",
            Token::Symbol(_, _) => "Symbol",
            Token::Keyword(_, _) => "Keyword",
            Token::String(_, _) => "String",
            Token::Long(_, _) => "Long",
            Token::Double(_, _) => "Double",
            Token::Bool(_, _) => "Bool",
            Token::Nil(_) => "Nil",
            Token::Regex(_, _) => "Regex",
            Token::ShorthandFnStart(_) => "ShorthandFnStart",
            Token::HashSetStart(_) => "HashSetStart",
            Token::ReaderDiscard(_) => "ReaderDiscard",
            Token::VarQuote(_) => "VarQuote",
            Token::Meta(_) => "Meta",
            Token::Quote(_) => "Quote",
            Token::SyntaxQuote(_) => "SyntaxQuote",
            Token::Unquote(_) => "Unquote",
            Token::UnquoteSplicing(_) => "UnquoteSplicing",
            Token::Deref(_) => "Deref",
            Token::Eof(_) => "Eof",
        }
    }

    fn parse_interface(&mut self) -> Result<InterfaceFile, String> {
        self.expect_lparen()?;

        let head = self.expect_symbol()?;
        if head != "interface" {
            return Err("Interface file must start with (interface ...)".to_string());
        }

        let name = self.expect_symbol()?;
        let mut functions = Vec::new();

        while !matches!(self.current(), Token::RParen(_)) {
            functions.push(self.parse_function()?);
        }

        self.expect_rparen()?;
        Ok(InterfaceFile { name, functions })
    }

    fn parse_function(&mut self) -> Result<InterfaceFunction, String> {
        self.expect_lparen()?;
        let head = self.expect_symbol()?;
        if head != "fn" && head != "defn" {
            return Err("Function declaration must start with (fn ...) or (defn ...)".to_string());
        }

        let name = self.expect_symbol()?;
        let params = self.parse_params()?;
        let return_type = self.expect_interface_type()?;
        let mut doc = None;
        let mut rust_symbol = None;

        while !matches!(self.current(), Token::RParen(_)) {
            match self.current() {
                Token::String(s, _) => {
                    if doc.is_some() {
                        return Err("Only one doc string is allowed in interface function".to_string());
                    }
                    doc = Some(s);
                    self.advance();
                }
                Token::Keyword(ref k, _) if k == "rust" => {
                    self.advance();
                    match self.current() {
                        Token::String(s, _) => {
                            rust_symbol = Some(s);
                            self.advance();
                        }
                        _ => {
                            return Err("Expected string after :rust in interface function".to_string());
                        }
                    }
                }
                _ => return Err("Unexpected token in interface function declaration".to_string()),
            }
        }

        self.expect_rparen()?;

        Ok(InterfaceFunction {
            name,
            params,
            return_type,
            doc,
            rust_symbol,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<InterfaceParam>, String> {
        self.expect_lbracket()?;
        let mut params = Vec::new();

        while !matches!(self.current(), Token::RBracket(_)) {
            let name = self.expect_symbol()?;
            let type_name = self.expect_interface_type()?;
            params.push(InterfaceParam { name, type_name });
        }

        self.expect_rbracket()?;
        Ok(params)
    }
}

/// Parse a .clri or .clorus-ffi interface file
pub fn parse_interface_file(path: &Path) -> Result<InterfaceFile, String> {

    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read interface file {}: {}", path.display(), e))?;


    // Interface files use a simplified grammar (fn/defn with explicit return type).
    // The main Clorus parser treats fn/defn as special forms, so use a custom token parser here.
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()
        .map_err(|e| format!("Failed to tokenize interface file {}: {}", path.display(), e))?;

    let mut parser = InterfaceTokenParser::new(tokens);
    parser
        .parse_interface()
        .map_err(|e| format!("Failed to parse interface file {}: {}", path.display(), e))
}

/// Parse interface from a Call expression
// Old Expr-based interface parsing removed in favor of the token parser above.

/// Map Clorus type keywords to Rust type strings
fn map_type_keyword(keyword: &str) -> String {
    match keyword {
        "f64" => "f64".to_string(),
        "i32" => "i32".to_string(),
        "i64" => "i64".to_string(),
        "i8" => "i8".to_string(),
        "i16" => "i16".to_string(),
        "u8" => "u8".to_string(),
        "u16" => "u16".to_string(),
        "u32" => "u32".to_string(),
        "u64" => "u64".to_string(),
        "isize" => "isize".to_string(),
        "usize" => "usize".to_string(),
        "f32" => "f32".to_string(),
        "bool" => "bool".to_string(),
        "string" => "String".to_string(),
        "unit" => "()".to_string(),
        "*mut u8" => "*mut u8".to_string(),
        "*const u8" => "*const u8".to_string(),
        "*mut-u8" => "*mut u8".to_string(),
        "*const-u8" => "*const u8".to_string(),
        other => other.to_string(), // For custom types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_interface() {
        let source = r#"
(interface example
  (defn add [x :f64 y :f64] :f64
    "Add two numbers")

  (defn greet [name :string] :string
    "Greet someone"))
"#;

        // Write to temp file
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(source.as_bytes()).unwrap();

        let result = parse_interface_file(file.path());
        assert!(result.is_ok());

        let interface = result.unwrap();
        assert_eq!(interface.name, "example");
        assert_eq!(interface.functions.len(), 2);

        let add_fn = &interface.functions[0];
        assert_eq!(add_fn.name, "add");
        assert_eq!(add_fn.params.len(), 2);
        assert_eq!(add_fn.params[0].name, "x");
        assert_eq!(add_fn.params[0].type_name, "f64");
        assert_eq!(add_fn.return_type, "f64");
        assert_eq!(add_fn.doc, Some("Add two numbers".to_string()));
        assert_eq!(add_fn.rust_symbol, None);
    }

    #[test]
    fn test_parse_interface_with_rust_symbol_override() {
        let source = r#"
(interface demo
  (fn new-point [x :f64 y :f64] :unit :rust "Point::new"))
"#;

        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(source.as_bytes()).unwrap();

        let interface = parse_interface_file(file.path()).expect("parse interface");
        assert_eq!(interface.functions.len(), 1);
        let f = &interface.functions[0];
        assert_eq!(f.name, "new-point");
        assert_eq!(f.rust_symbol.as_deref(), Some("Point::new"));
    }

    #[test]
    fn test_parse_interface_reports_location_for_malformed_param_type() {
        let source = r#"
(interface bad
  (fn ptr-roundtrip [p :*mut] :i32))
"#;

        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(source.as_bytes()).unwrap();

        let err = parse_interface_file(file.path()).expect_err("expected parse failure");
        assert!(err.contains("Failed to parse interface file"));
        assert!(err.contains("Expected pointee type after :*mut"));
    }

    #[test]
    fn test_parse_interface_supports_pointer_keywords_without_spaces() {
        let source = r#"
(interface ptrs
  (fn const-ptr [p :*const-u8] :*const-u8 :rust "id_const_ptr")
  (fn mut-ptr [p :*mut-u8] :*mut-u8 :rust "id_mut_ptr"))
"#;

        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(source.as_bytes()).unwrap();

        let interface = parse_interface_file(file.path()).expect("parse interface");
        assert_eq!(interface.functions.len(), 2);

        let const_ptr = &interface.functions[0];
        assert_eq!(const_ptr.params[0].type_name, "*const u8");
        assert_eq!(const_ptr.return_type, "*const u8");

        let mut_ptr = &interface.functions[1];
        assert_eq!(mut_ptr.params[0].type_name, "*mut u8");
        assert_eq!(mut_ptr.return_type, "*mut u8");
    }

    #[test]
    fn test_parse_interface_supports_pointer_keywords_with_split_form() {
        let source = r#"
(interface ptrs
  (fn const-ptr [p :*const u8] :*const u8 :rust "id_const_ptr")
  (fn mut-ptr [p :*mut u8] :*mut u8 :rust "id_mut_ptr"))
"#;

        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(source.as_bytes()).unwrap();

        let interface = parse_interface_file(file.path()).expect("parse interface");
        assert_eq!(interface.functions.len(), 2);

        let const_ptr = &interface.functions[0];
        assert_eq!(const_ptr.params[0].type_name, "*const u8");
        assert_eq!(const_ptr.return_type, "*const u8");

        let mut_ptr = &interface.functions[1];
        assert_eq!(mut_ptr.params[0].type_name, "*mut u8");
        assert_eq!(mut_ptr.return_type, "*mut u8");
    }
}
