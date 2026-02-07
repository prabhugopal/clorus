/// Lexer/Tokenizer for Clojure-like syntax

/// Source code span tracking line, column, and position
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,   // Character position in input
    pub end: usize,
    pub line: usize,    // Line number (1-indexed)
    pub column: usize,  // Column number (1-indexed)
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Span { start, end, line, column }
    }

    pub fn dummy() -> Self {
        Span { start: 0, end: 0, line: 1, column: 1 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Delimiters
    LParen(Span),
    RParen(Span),
    LBracket(Span),
    RBracket(Span),
    LBrace(Span),
    RBrace(Span),

    // Reader macros
    ShorthandFnStart(Span),  // #(
    HashSetStart(Span),      // #{
    VarQuote(Span),          // #' (var quote)
    Meta(Span),              // ^ (metadata prefix)
    Quote(Span),             // '
    SyntaxQuote(Span),       // ` (backtick)
    Unquote(Span),           // ~
    UnquoteSplicing(Span),   // ~@
    Deref(Span),             // @ (dereference atom/ref)

    // Literals
    Long(i64, Span),
    Double(f64, Span),
    String(String, Span),
    Symbol(String, Span),
    Keyword(String, Span),
    Bool(bool, Span),
    Nil(Span),

    // End of input
    Eof(Span),
}

impl Token {
    pub fn span(&self) -> Span {
        match self {
            Token::LParen(s) | Token::RParen(s) | Token::LBracket(s) | Token::RBracket(s)
            | Token::LBrace(s) | Token::RBrace(s) | Token::ShorthandFnStart(s)
            | Token::HashSetStart(s) | Token::VarQuote(s) | Token::Meta(s) | Token::Quote(s)
            | Token::SyntaxQuote(s) | Token::Unquote(s) | Token::UnquoteSplicing(s)
            | Token::Deref(s) | Token::Nil(s) | Token::Eof(s) => *s,
            Token::Long(_, s) | Token::Double(_, s) | Token::String(_, s)
            | Token::Symbol(_, s) | Token::Keyword(_, s) | Token::Bool(_, s) => *s,
        }
    }
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,      // Current line (1-indexed)
    column: usize,    // Current column (1-indexed)
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    fn current_span(&self) -> Span {
        Span::new(self.position, self.position + 1, self.line, self.column)
    }

    fn make_span(&self, start_pos: usize, start_line: usize, start_col: usize) -> Span {
        Span::new(start_pos, self.position, start_line, start_col)
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
        if let Some(ch) = self.current_char() {
            self.position += 1;
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
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

    fn read_string(&mut self) -> Result<(String, Span), String> {
        let start_pos = self.position;
        let start_line = self.line;
        let start_col = self.column;

        let mut result = String::new();
        self.advance(); // skip opening "

        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance(); // skip closing "
                let span = self.make_span(start_pos, start_line, start_col);
                return Ok((result, span));
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
        let start_pos = self.position;
        let start_line = self.line;
        let start_col = self.column;

        let mut num_str = String::new();
        let mut has_decimal = false;

        // Handle negative numbers
        if self.current_char() == Some('-') {
            num_str.push('-');
            self.advance();
        }

        // Check for hexadecimal (0x...)
        if self.current_char() == Some('0') && self.peek_char(1) == Some('x') {
            self.advance(); // skip '0'
            self.advance(); // skip 'x'

            let mut hex_str = String::new();
            while let Some(ch) = self.current_char() {
                if ch.is_ascii_hexdigit() {
                    hex_str.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }

            let span = self.make_span(start_pos, start_line, start_col);

            if hex_str.is_empty() {
                return Token::Long(0, span);
            }

            // Parse hex string to i64
            if let Ok(value) = i64::from_str_radix(&hex_str, 16) {
                return Token::Long(value, span);
            } else {
                return Token::Long(0, span); // Fallback on overflow
            }
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

        let span = self.make_span(start_pos, start_line, start_col);

        // Parse as Long or Double based on presence of decimal point/exponent
        if has_decimal {
            let value = num_str.parse().unwrap_or(0.0);
            Token::Double(value, span)
        } else {
            let value = num_str.parse().unwrap_or(0);
            Token::Long(value, span)
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
            None => {
                let span = self.current_span();
                Ok(Token::Eof(span))
            }

            Some('(') => {
                let span = self.current_span();
                self.advance();
                Ok(Token::LParen(span))
            }
            Some(')') => {
                let span = self.current_span();
                self.advance();
                Ok(Token::RParen(span))
            }
            Some('[') => {
                let span = self.current_span();
                self.advance();
                Ok(Token::LBracket(span))
            }
            Some(']') => {
                let span = self.current_span();
                self.advance();
                Ok(Token::RBracket(span))
            }
            Some('{') => {
                let span = self.current_span();
                self.advance();
                Ok(Token::LBrace(span))
            }
            Some('}') => {
                let span = self.current_span();
                self.advance();
                Ok(Token::RBrace(span))
            }

            Some('#') => {
                // Check for reader macros starting with #
                if self.peek_char(1) == Some('(') {
                    // Shorthand function: #(...)
                    let start_pos = self.position;
                    let start_line = self.line;
                    let start_col = self.column;
                    self.advance(); // skip #
                    self.advance(); // skip (
                    let span = self.make_span(start_pos, start_line, start_col);
                    Ok(Token::ShorthandFnStart(span))
                } else if self.peek_char(1) == Some('{') {
                    // Hash set: #{...}
                    let start_pos = self.position;
                    let start_line = self.line;
                    let start_col = self.column;
                    self.advance(); // skip #
                    self.advance(); // skip {
                    let span = self.make_span(start_pos, start_line, start_col);
                    Ok(Token::HashSetStart(span))
                } else if self.peek_char(1) == Some('\'') {
                    // Var quote: #'symbol
                    let start_pos = self.position;
                    let start_line = self.line;
                    let start_col = self.column;
                    self.advance(); // skip #
                    self.advance(); // skip '
                    let span = self.make_span(start_pos, start_line, start_col);
                    Ok(Token::VarQuote(span))
                } else {
                    // Other # forms not yet supported
                    return Err(format!("Unsupported reader macro: #{:?}", self.peek_char(1)));
                }
            }

            Some('\'') => {
                // Quote: 'x or '(...)
                let span = self.current_span();
                self.advance();
                Ok(Token::Quote(span))
            }

            Some('`') => {
                // Syntax-quote: `x or `(...)
                let span = self.current_span();
                self.advance();
                Ok(Token::SyntaxQuote(span))
            }

            Some('~') => {
                // Check for unquote-splicing ~@ or plain unquote ~
                if self.peek_char(1) == Some('@') {
                    let start_pos = self.position;
                    let start_line = self.line;
                    let start_col = self.column;
                    self.advance(); // skip ~
                    self.advance(); // skip @
                    let span = self.make_span(start_pos, start_line, start_col);
                    Ok(Token::UnquoteSplicing(span))
                } else {
                    let span = self.current_span();
                    self.advance();
                    Ok(Token::Unquote(span))
                }
            }

            Some('@') => {
                // Deref: @my-atom
                let span = self.current_span();
                self.advance();
                Ok(Token::Deref(span))
            }

            Some('^') => {
                // Metadata: ^:dynamic or ^{:doc "..."}
                let span = self.current_span();
                self.advance();
                Ok(Token::Meta(span))
            }

            Some('"') => {
                let (s, span) = self.read_string()?;
                Ok(Token::String(s, span))
            }

            Some(':') => {
                let start_pos = self.position;
                let start_line = self.line;
                let start_col = self.column;
                self.advance();
                let keyword = self.read_symbol();
                let span = self.make_span(start_pos, start_line, start_col);
                Ok(Token::Keyword(keyword, span))
            }

            Some(ch) if ch.is_ascii_digit() || (ch == '-' && self.peek_char(1).map_or(false, |c| c.is_ascii_digit())) => {
                Ok(self.read_number())
            }

            Some(_) => {
                let start_pos = self.position;
                let start_line = self.line;
                let start_col = self.column;
                let sym = self.read_symbol();
                let span = self.make_span(start_pos, start_line, start_col);
                match sym.as_str() {
                    "true" => Ok(Token::Bool(true, span)),
                    "false" => Ok(Token::Bool(false, span)),
                    "nil" => Ok(Token::Nil(span)),
                    _ => Ok(Token::Symbol(sym, span)),
                }
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token, Token::Eof(_));
            tokens.push(token);
            if is_eof {
                break;
            }
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
        assert_eq!(tokens.len(), 7);
        assert!(matches!(tokens[0], Token::LParen(_)));
        assert!(matches!(tokens[1], Token::RParen(_)));
        assert!(matches!(tokens[2], Token::LBracket(_)));
        assert!(matches!(tokens[3], Token::RBracket(_)));
        assert!(matches!(tokens[4], Token::LBrace(_)));
        assert!(matches!(tokens[5], Token::RBrace(_)));
        assert!(matches!(tokens[6], Token::Eof(_)));
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 -10");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0], Token::Long(42, _)));
        assert!(matches!(tokens[1], Token::Double(v, _) if (v - 3.14).abs() < 0.001));
        assert!(matches!(tokens[2], Token::Long(-10, _)));
    }

    #[test]
    fn test_symbols() {
        let mut lexer = Lexer::new("defn + my-var?");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0], Token::Symbol(s, _) if s == "defn"));
        assert!(matches!(&tokens[1], Token::Symbol(s, _) if s == "+"));
        assert!(matches!(&tokens[2], Token::Symbol(s, _) if s == "my-var?"));
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new(":name :type");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0], Token::Keyword(k, _) if k == "name"));
        assert!(matches!(&tokens[1], Token::Keyword(k, _) if k == "type"));
    }

    #[test]
    fn test_strings() {
        let mut lexer = Lexer::new(r#""hello" "world\n""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0], Token::String(s, _) if s == "hello"));
        assert!(matches!(&tokens[1], Token::String(s, _) if s == "world\n"));
    }
}
