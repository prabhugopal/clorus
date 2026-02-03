/// Lexer/Tokenizer for Clojure-like syntax

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    // Reader macros
    ShorthandFnStart,  // #(
    HashSetStart,      // #{
    VarQuote,          // #' (var quote)
    Meta,              // ^ (metadata prefix)
    Quote,             // '
    SyntaxQuote,       // ` (backtick)
    Unquote,           // ~
    UnquoteSplicing,   // ~@
    Deref,             // @ (dereference atom/ref)

    // Literals
    Long(i64),
    Double(f64),
    String(String),
    Symbol(String),
    Keyword(String),
    Bool(bool),
    Nil,

    // End of input
    Eof,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
        }
    }

    fn current_char(&self) -> Option<char> {
        if self.position < self.input.len() {
            Some(self.input[self.position])
        } else {
            None
        }
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        let pos = self.position + offset;
        if pos < self.input.len() {
            Some(self.input[pos])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() || ch == ',' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        // Skip ; comments until end of line
        if self.current_char() == Some(';') {
            while let Some(ch) = self.current_char() {
                self.advance();
                if ch == '\n' {
                    break;
                }
            }
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        let mut result = String::new();
        self.advance(); // skip opening "

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance(); // skip closing "
                return Ok(result);
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.current_char() {
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        _ => {
                            result.push('\\');
                            result.push(escaped);
                        }
                    }
                    self.advance();
                }
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err("Unterminated string".to_string())
    }

    fn read_number(&mut self) -> Token {
        let mut num_str = String::new();
        let mut has_decimal = false;

        // Handle negative numbers
        if self.current_char() == Some('-') {
            num_str.push('-');
            self.advance();
        }

        // Read digits and decimal point
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' {
                has_decimal = true;
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check for scientific notation (e.g., 1e6, 3.14e-2)
        if let Some('e') | Some('E') = self.current_char() {
            has_decimal = true;  // Scientific notation is always a double
            num_str.push('e');
            self.advance();

            // Handle optional +/- after 'e'
            if let Some('+') | Some('-') = self.current_char() {
                num_str.push(self.current_char().unwrap());
                self.advance();
            }

            // Read exponent digits
            while let Some(ch) = self.current_char() {
                if ch.is_ascii_digit() {
                    num_str.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Parse as Long or Double based on presence of decimal point/exponent
        if has_decimal {
            let value = num_str.parse().unwrap_or(0.0);
            Token::Double(value)
        } else {
            let value = num_str.parse().unwrap_or(0);
            Token::Long(value)
        }
    }

    fn read_symbol(&mut self) -> String {
        let mut result = String::new();

        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric()
                || ch == '-'
                || ch == '_'
                || ch == '?'
                || ch == '!'
                || ch == '+'
                || ch == '*'
                || ch == '/'
                || ch == '<'
                || ch == '>'
                || ch == '='
                || ch == '.'
                || ch == '%'
                || ch == '&'
                || ch == '#'  // Allow # for gensyms (auto-gensym like result#, x#)
            {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        result
    }

    pub fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace();

        // Skip comments
        if self.current_char() == Some(';') {
            self.skip_comment();
            return self.next_token();
        }

        match self.current_char() {
            None => Ok(Token::Eof),

            Some('(') => {
                self.advance();
                Ok(Token::LParen)
            }
            Some(')') => {
                self.advance();
                Ok(Token::RParen)
            }
            Some('[') => {
                self.advance();
                Ok(Token::LBracket)
            }
            Some(']') => {
                self.advance();
                Ok(Token::RBracket)
            }
            Some('{') => {
                self.advance();
                Ok(Token::LBrace)
            }
            Some('}') => {
                self.advance();
                Ok(Token::RBrace)
            }

            Some('#') => {
                // Check for reader macros starting with #
                if self.peek_char(1) == Some('(') {
                    // Shorthand function: #(...)
                    self.advance(); // skip #
                    self.advance(); // skip (
                    Ok(Token::ShorthandFnStart)
                } else if self.peek_char(1) == Some('{') {
                    // Hash set: #{...}
                    self.advance(); // skip #
                    self.advance(); // skip {
                    Ok(Token::HashSetStart)
                } else if self.peek_char(1) == Some('\'') {
                    // Var quote: #'symbol
                    self.advance(); // skip #
                    self.advance(); // skip '
                    Ok(Token::VarQuote)
                } else {
                    // Other # forms not yet supported
                    return Err(format!("Unsupported reader macro: #{:?}", self.peek_char(1)));
                }
            }

            Some('\'') => {
                // Quote: 'x or '(...)
                self.advance();
                Ok(Token::Quote)
            }

            Some('`') => {
                // Syntax-quote: `x or `(...)
                self.advance();
                Ok(Token::SyntaxQuote)
            }

            Some('~') => {
                // Check for unquote-splicing ~@ or plain unquote ~
                if self.peek_char(1) == Some('@') {
                    self.advance(); // skip ~
                    self.advance(); // skip @
                    Ok(Token::UnquoteSplicing)
                } else {
                    self.advance();
                    Ok(Token::Unquote)
                }
            }

            Some('@') => {
                // Deref: @my-atom
                self.advance();
                Ok(Token::Deref)
            }

            Some('^') => {
                // Metadata: ^:dynamic or ^{:doc "..."}
                self.advance();
                Ok(Token::Meta)
            }

            Some('"') => {
                let s = self.read_string()?;
                Ok(Token::String(s))
            }

            Some(':') => {
                self.advance();
                let keyword = self.read_symbol();
                Ok(Token::Keyword(keyword))
            }

            Some(ch) if ch.is_ascii_digit() || (ch == '-' && self.peek_char(1).map_or(false, |c| c.is_ascii_digit())) => {
                Ok(self.read_number())
            }

            Some(_) => {
                let sym = self.read_symbol();
                match sym.as_str() {
                    "true" => Ok(Token::Bool(true)),
                    "false" => Ok(Token::Bool(false)),
                    "nil" => Ok(Token::Nil),
                    _ => Ok(Token::Symbol(sym)),
                }
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            if token == Token::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("() [] {}");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens, vec![
            Token::LParen, Token::RParen,
            Token::LBracket, Token::RBracket,
            Token::LBrace, Token::RBrace,
            Token::Eof,
        ]);
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 -10");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Long(42));
        assert_eq!(tokens[1], Token::Double(3.14));
        assert_eq!(tokens[2], Token::Long(-10));
    }

    #[test]
    fn test_symbols() {
        let mut lexer = Lexer::new("defn + my-var?");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Symbol("defn".to_string()));
        assert_eq!(tokens[1], Token::Symbol("+".to_string()));
        assert_eq!(tokens[2], Token::Symbol("my-var?".to_string()));
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new(":name :type");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], Token::Keyword("name".to_string()));
        assert_eq!(tokens[1], Token::Keyword("type".to_string()));
    }

    #[test]
    fn test_strings() {
        let mut lexer = Lexer::new(r#""hello" "world\n""#);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], Token::String("hello".to_string()));
        assert_eq!(tokens[1], Token::String("world\n".to_string()));
    }
}
