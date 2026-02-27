/// Parser for S-expressions
use crate::ast::{Expr, ProtocolMethodImpl};
use crate::lexer::{Lexer, Token, Span};
use std::cell::Cell;

thread_local! {
    static PARSE_DEPTH: Cell<usize> = Cell::new(0);
}

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    eof_token: Token,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
            eof_token: Token::Eof(Span::dummy()),
        }
    }

    fn current_token(&self) -> &Token {
        if self.position < self.tokens.len() {
            &self.tokens[self.position]
        } else {
            &self.eof_token
        }
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if std::mem::discriminant(self.current_token()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            let span = self.current_token().span();
            Err(format!(
                "Expected {:?}, found {:?} at line {}, column {}",
                expected,
                self.current_token(),
                span.line,
                span.column
            ))
        }
    }

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        match self.current_token() {
            Token::Long(n, _) => {
                let num = *n;
                self.advance();
                Ok(Expr::Long(num))
            }

            Token::Double(n, _) => {
                let num = *n;
                self.advance();
                Ok(Expr::Double(num))
            }

            Token::String(s, _) => {
                let string = s.clone();
                self.advance();
                Ok(Expr::String(string))
            }

            Token::Symbol(s, _) => {
                let symbol = s.clone();
                self.advance();
                Ok(Expr::Symbol(symbol))
            }

            Token::Keyword(k, _) => {
                let keyword = k.clone();
                self.advance();
                Ok(Expr::Keyword(keyword))
            }

            Token::Bool(b, _) => {
                let bool_val = *b;
                self.advance();
                Ok(Expr::Bool(bool_val))
            }

            Token::Nil(_) => {
                self.advance();
                Ok(Expr::Nil)
            }

            Token::LParen(_) => self.parse_list_or_special(),
            Token::LBracket(_) => self.parse_vector(),
            Token::LBrace(_) => self.parse_map(),

            Token::HashSetStart(_) => {
                self.advance(); // Consume the HashSetStart token
                self.parse_set()
            }

            Token::Quote(_) => {
                self.advance(); // Consume the ' token
                let quoted_expr = self.parse_expr()?;
                Ok(Expr::Quote {
                    expr: Box::new(quoted_expr),
                })
            }

            Token::SyntaxQuote(_) => {
                self.advance(); // Consume the ` token
                let quoted_expr = self.parse_expr()?;
                Ok(Expr::SyntaxQuote {
                    expr: Box::new(quoted_expr),
                })
            }

            Token::Unquote(_) => {
                self.advance(); // Consume the ~ token
                let unquoted_expr = self.parse_expr()?;
                Ok(Expr::Unquote {
                    expr: Box::new(unquoted_expr),
                })
            }

            Token::UnquoteSplicing(_) => {
                self.advance(); // Consume the ~@ token
                let spliced_expr = self.parse_expr()?;
                Ok(Expr::UnquoteSplicing {
                    expr: Box::new(spliced_expr),
                })
            }

            Token::Deref(_) => {
                self.advance(); // Consume the @ token
                let deref_expr = self.parse_expr()?;
                Ok(Expr::Deref {
                    expr: Box::new(deref_expr),
                })
            }

            Token::ShorthandFnStart(_) => {
                self.advance(); // CRITICAL: Consume the ShorthandFnStart token!
                self.parse_shorthand_fn()
            }

            Token::VarQuote(_) => {
                self.advance(); // Consume the #' token
                let var_symbol = self.parse_expr()?;
                // Extract name from symbol
                let name = match var_symbol {
                    Expr::Symbol(s) => s,
                    _ => return Err("Var quote #' requires a symbol".to_string()),
                };
                Ok(Expr::Var { name })
            }

            Token::Meta(_) => {
                self.advance(); // Consume the ^ token
                // Parse metadata and the expression it applies to
                // For now, just skip metadata and parse the next expression
                self.parse_expr()
            }

            Token::RParen(_) | Token::RBracket(_) | Token::RBrace(_) => {
                let span = self.current_token().span();
                Err(format!(
                    "Unexpected closing delimiter: {:?} at line {}, column {}",
                    self.current_token(),
                    span.line,
                    span.column
                ))
            }

            Token::Eof(_) => {
                let span = self.current_token().span();
                Err(format!(
                    "Unexpected end of input at line {}, column {}",
                    span.line,
                    span.column
                ))
            }
        }
    }

    fn parse_list_or_special(&mut self) -> Result<Expr, String> {
        self.expect(Token::LParen(Span::dummy()))?;

        // Check for special forms: ns, require, let, def, defn, defmacro, fn, if, do, quote, use, loop, recur, try, throw, declare
        if let Token::Symbol(sym, _) = self.current_token() {
            match sym.as_str() {
                "ns" => return self.parse_ns(),
                "require" => return self.parse_require(),
                "declare" => return self.parse_declare(),
                "let" => return self.parse_let(),
                "letfn" => return self.parse_letfn(),
                "def" => return self.parse_def(),
                "defn" => return self.parse_defn(),
                "defmacro" => return self.parse_defmacro(),
                "defrecord" => return self.parse_defrecord(),
                "deftype" => return self.parse_deftype(),
                "defprotocol" => return self.parse_defprotocol(),
                "extend-type" => return self.parse_extend_type(),
                "defmulti" => return self.parse_defmulti(),
                "defmethod" => return self.parse_defmethod(),
                "fn" => return self.parse_fn(),
                "if" => return self.parse_if(),
                "do" => return self.parse_do(),
                "dosync" => return self.parse_dosync(),
                "quote" => return self.parse_quote(),
                "use" => return self.parse_use(),
                "loop" => return self.parse_loop(),
                "recur" => return self.parse_recur(),
                "try" => return self.parse_try(),
                "throw" => return self.parse_throw(),
                _ => {}
            }
        }

        // Check if this is a function call or operator call
        let mut elements = Vec::new();
        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed list".to_string());
            }
            elements.push(self.parse_expr()?);
        }

        self.expect(Token::RParen(Span::dummy()))?;

        // If first element is a symbol, it might be a function call
        if !elements.is_empty() {
            if let Expr::Symbol(func_name) = &elements[0] {
                // Known operators stay as List
                let operators = ["+", "-", "*", "/", "<", ">", "="];
                if !operators.contains(&func_name.as_str()) {
                    // It's a function call
                    return Ok(Expr::Call {
                        func: func_name.clone(),
                        args: elements[1..].to_vec(),
                    });
                }
            }
        }

        Ok(Expr::List(elements))
    }

    fn parse_defn(&mut self) -> Result<Expr, String> {
        // Already saw 'defn', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "defn" {
                self.advance();
            }
        }

        // Get function name
        let name = match self.current_token() {
            Token::Symbol(s, _) => s.clone(),
            _ => return Err("defn requires a function name".to_string()),
        };
        self.advance();

        // Check for optional docstring
        // (defn foo "docstring" [...] body) or (defn foo "docstring" ([] ...) ([x] ...))
        let _docstring = if let Token::String(_, _) = self.current_token() {
            // Consume the docstring but don't use it yet (could be added to AST later)
            let docstring = match self.current_token() {
                Token::String(s, _) => Some(s.clone()),
                _ => None,
            };
            self.advance();
            docstring
        } else {
            None
        };

        // Check if this is single-arity or multi-arity
        // Single: (defn foo [x y] body) or (defn foo "doc" [x y] body)
        // Multi:  (defn foo ([] body1) ([x] body2)) or (defn foo "doc" ([] ...) ([x] ...))
        if matches!(self.current_token(), Token::LBracket(_)) {
            // Single arity
            self.parse_single_arity_defn(name)
        } else if matches!(self.current_token(), Token::LParen(_)) {
            // Multi arity
            self.parse_multi_arity_defn(name)
        } else {
            Err("defn requires either parameter vector [...] or arity clauses (...)".to_string())
        }
    }

    fn parse_single_arity_defn(&mut self, name: String) -> Result<Expr, String> {
        // Expect parameter vector: [x y z]
        if !matches!(self.current_token(), Token::LBracket(_)) {
            return Err("defn requires a parameter vector [...]".to_string());
        }
        self.advance(); // consume [

        // Parse parameters (now supports destructuring)
        let mut params = Vec::new();
        let mut rest_param = None;

        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed parameter vector in defn".to_string());
            }

            // Check for & symbol (rest parameter marker)
            if let Token::Symbol(s, _) = self.current_token() {
                if s == "&" {
                    self.advance(); // consume &

                    // Next symbol should be the rest parameter name
                    let rest_name = match self.current_token() {
                        Token::Symbol(s, _) => s.clone(),
                        _ => return Err("Rest parameter after & must be a symbol".to_string()),
                    };
                    self.advance();

                    rest_param = Some(rest_name);

                    // After rest parameter, there should be no more parameters
                    if !matches!(self.current_token(), Token::RBracket(_)) {
                        return Err("Rest parameter must be the last parameter".to_string());
                    }
                    break;
                }
            }

            // Parse parameter pattern (symbol, vector, map, etc.)
            let param = self.parse_pattern()?;
            params.push(param);
        }

        self.expect(Token::RBracket(Span::dummy()))?; // consume ]

        // Parse body (single expression for now)
        let body = self.parse_expr()?;

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Defn {
            name,
            params,
            rest_param,
            body: Box::new(body),
        })
    }

    fn parse_multi_arity_defn(&mut self, name: String) -> Result<Expr, String> {
        use crate::ast::FunctionArity;

        let mut arities = Vec::new();

        // Parse each arity clause: ([] body) ([x] body) ([x y] body)
        while matches!(self.current_token(), Token::LParen(_)) {
            self.advance(); // consume (

            // Parse parameter vector
            if !matches!(self.current_token(), Token::LBracket(_)) {
                return Err("Each arity clause must start with parameter vector [...]".to_string());
            }
            self.advance(); // consume [

            let mut params = Vec::new();
            let mut rest_param = None;

            while !matches!(self.current_token(), Token::RBracket(_)) {
                if self.current_token() == &self.eof_token {
                    return Err("Unclosed parameter vector in arity clause".to_string());
                }

                // Check for & symbol
                if let Token::Symbol(s, _) = self.current_token() {
                    if s == "&" {
                        self.advance(); // consume &

                        let rest_name = match self.current_token() {
                            Token::Symbol(s, _) => s.clone(),
                            _ => return Err("Rest parameter after & must be a symbol".to_string()),
                        };
                        self.advance();

                        rest_param = Some(rest_name);

                        if !matches!(self.current_token(), Token::RBracket(_)) {
                            return Err("Rest parameter must be the last parameter".to_string());
                        }
                        break;
                    }
                }

                // Parse parameter pattern (symbol, vector, map, etc.)
                let param = self.parse_pattern()?;
                params.push(param);
            }

            self.expect(Token::RBracket(Span::dummy()))?; // consume ]

            // Parse body
            let body = self.parse_expr()?;

            self.expect(Token::RParen(Span::dummy()))?; // consume ) closing arity clause

            arities.push(FunctionArity {
                params,
                rest_param,
                body: Box::new(body),
            });
        }

        self.expect(Token::RParen(Span::dummy()))?; // consume ) closing defn

        if arities.is_empty() {
            return Err("Multi-arity defn must have at least one arity clause".to_string());
        }

        Ok(Expr::DefnMulti { name, arities })
    }

    fn parse_defmacro(&mut self) -> Result<Expr, String> {
        // (defmacro when [test & body] `(if ~test (do ~@body) nil))
        // Already saw 'defmacro', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "defmacro" {
                self.advance();
            }
        }

        // Get macro name
        let name = match self.current_token() {
            Token::Symbol(s, _) => s.clone(),
            _ => return Err("defmacro requires a macro name".to_string()),
        };
        self.advance();

        // Expect parameter vector: [x y z] or [x & rest]
        if !matches!(self.current_token(), Token::LBracket(_)) {
            return Err("defmacro requires a parameter vector [...]".to_string());
        }
        self.advance(); // consume [

        // Parse parameters (same as defn)
        let mut params = Vec::new();
        let mut rest_param = None;

        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed parameter vector in defmacro".to_string());
            }

            // Check for & symbol (rest parameter marker)
            if let Token::Symbol(s, _) = self.current_token() {
                if s == "&" {
                    self.advance(); // consume &

                    // Next symbol should be the rest parameter name
                    let rest_name = match self.current_token() {
                        Token::Symbol(s, _) => s.clone(),
                        _ => return Err("Rest parameter after & must be a symbol".to_string()),
                    };
                    self.advance();

                    rest_param = Some(rest_name);

                    // After rest parameter, there should be no more parameters
                    if !matches!(self.current_token(), Token::RBracket(_)) {
                        return Err("Rest parameter must be the last parameter".to_string());
                    }
                    break;
                }
            }

            // Get parameter name (must be a symbol for macros)
            let param = match self.current_token() {
                Token::Symbol(s, _) => s.clone(),
                _ => return Err("Macro parameter name must be a symbol".to_string()),
            };
            self.advance();

            params.push(param);
        }

        self.expect(Token::RBracket(Span::dummy()))?; // consume ]

        // Parse body (single expression, usually a syntax-quoted form)
        let body = self.parse_expr()?;

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Defmacro {
            name,
            params,
            rest_param,
            body: Box::new(body),
        })
    }

    fn parse_defrecord(&mut self) -> Result<Expr, String> {
        // (defrecord Person [name age email])
        // Can also have inline protocol implementations like deftype:
        // (defrecord Card [props bounds]
        //   IComponent
        //   (render [this ctx] ...))

        // Already saw 'defrecord', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "defrecord" {
                self.advance();
            }
        }

        // Get record type name
        let name = match self.current_token() {
            Token::Symbol(s, _) => s.clone(),
            _ => return Err("defrecord requires a record type name".to_string()),
        };
        self.advance();

        // Expect field vector: [field1 field2 field3]
        if !matches!(self.current_token(), Token::LBracket(_)) {
            return Err("defrecord requires a field vector [...]".to_string());
        }
        self.advance(); // consume [

        // Parse field names (all must be symbols)
        let mut fields = Vec::new();

        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed field vector in defrecord".to_string());
            }

            // Get field name (must be a symbol)
            let field = match self.current_token() {
                Token::Symbol(s, _) => s.clone(),
                _ => return Err("Record field name must be a symbol".to_string()),
            };
            self.advance();

            fields.push(field);
        }

        self.expect(Token::RBracket(Span::dummy()))?; // consume ]

        // Parse protocol implementations (optional, same as deftype)
        let mut protocols: Vec<(String, Vec<ProtocolMethodImpl>)> = Vec::new();

        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed defrecord form".to_string());
            }

            // Expect protocol name (symbol)
            let protocol_name = match self.current_token() {
                Token::Symbol(s, _) => s.clone(),
                _ => return Err("Expected protocol name in defrecord".to_string()),
            };
            self.advance();

            // Parse method implementations for this protocol
            let mut methods = Vec::new();
            while matches!(self.current_token(), Token::LParen(_)) {
                self.advance(); // consume (

                // Method name
                let method_name = match self.current_token() {
                    Token::Symbol(s, _) => s.clone(),
                    _ => return Err("Expected method name".to_string()),
                };
                self.advance();

                // Parameters
                if !matches!(self.current_token(), Token::LBracket(_)) {
                    return Err("Method requires parameter vector".to_string());
                }
                self.advance(); // consume [

                let mut params = Vec::new();
                while !matches!(self.current_token(), Token::RBracket(_)) {
                    params.push(self.parse_pattern()?);
                }
                self.expect(Token::RBracket(Span::dummy()))?; // consume ]

                // Method body (single expression)
                let body = Box::new(self.parse_expr()?);

                self.expect(Token::RParen(Span::dummy()))?; // consume )

                methods.push(ProtocolMethodImpl {
                    name: method_name,
                    params,
                    body,
                });
            }

            protocols.push((protocol_name, methods));
        }

        self.expect(Token::RParen(Span::dummy()))?; // consume closing )

        Ok(Expr::Defrecord {
            name,
            fields,
            protocols,
        })
    }

    fn parse_deftype(&mut self) -> Result<Expr, String> {
        // (deftype Button [props state]
        //   IComponent
        //   (render [this ctx] ...)
        //   (layout [this bounds] ...))

        // Already saw 'deftype', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "deftype" {
                self.advance();
            }
        }

        // Get type name
        let name = match self.current_token() {
            Token::Symbol(s, _) => s.clone(),
            _ => return Err("deftype requires a type name".to_string()),
        };
        self.advance();

        // Expect field vector: [field1 field2]
        if !matches!(self.current_token(), Token::LBracket(_)) {
            return Err(format!("deftype requires a field vector [...], found {:?}", self.current_token()));
        }
        self.advance(); // consume [

        // Parse field names (all must be symbols)
        let mut fields = Vec::new();
        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed field vector in deftype".to_string());
            }

            let field = match self.current_token() {
                Token::Symbol(s, _) => s.clone(),
                _ => return Err("deftype field name must be a symbol".to_string()),
            };
            self.advance();
            fields.push(field);
        }
        self.expect(Token::RBracket(Span::dummy()))?; // consume ]

        // Parse protocol implementations
        // Format: ProtocolName (method [params] body) (method [params] body) ...
        let mut protocols: Vec<(String, Vec<ProtocolMethodImpl>)> = Vec::new();

        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed deftype form".to_string());
            }

            // Expect protocol name (symbol)
            let protocol_name = match self.current_token() {
                Token::Symbol(s, _) => s.clone(),
                _ => return Err("Expected protocol name in deftype".to_string()),
            };
            self.advance();

            // Parse method implementations for this protocol
            let mut methods = Vec::new();
            while matches!(self.current_token(), Token::LParen(_)) {
                self.advance(); // consume (

                // Method name
                let method_name = match self.current_token() {
                    Token::Symbol(s, _) => s.clone(),
                    _ => return Err("Expected method name".to_string()),
                };
                self.advance();

                // Parameters
                if !matches!(self.current_token(), Token::LBracket(_)) {
                    return Err("Method requires parameter vector".to_string());
                }
                self.advance(); // consume [

                let mut params = Vec::new();
                while !matches!(self.current_token(), Token::RBracket(_)) {
                    params.push(self.parse_pattern()?);
                }
                self.expect(Token::RBracket(Span::dummy()))?; // consume ]

                // Method body (single expression)
                let body = Box::new(self.parse_expr()?);

                self.expect(Token::RParen(Span::dummy()))?; // consume )

                methods.push(ProtocolMethodImpl {
                    name: method_name,
                    params,
                    body,
                });
            }

            protocols.push((protocol_name, methods));
        }

        self.expect(Token::RParen(Span::dummy()))?; // consume closing )

        Ok(Expr::Deftype {
            name,
            fields,
            protocols,
        })
    }

    fn parse_defprotocol(&mut self) -> Result<Expr, String> {
        // (defprotocol Drawable (draw [this]) (bounds [this]))
        // Already saw 'defprotocol', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "defprotocol" {
                self.advance();
            }
        }

        // Get protocol name
        let name = match self.current_token() {
            Token::Symbol(s, _) => s.clone(),
            _ => return Err("defprotocol requires a protocol name".to_string()),
        };
        self.advance();

        // Parse method signatures: (method-name [params] "optional docstring")
        let mut methods = Vec::new();

        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed defprotocol".to_string());
            }

            // Each method is a list: (method-name [params] docstring?)
            if !matches!(self.current_token(), Token::LParen(_)) {
                return Err("Protocol method must be a list (method-name [params])".to_string());
            }
            self.advance(); // consume (

            // Get method name
            let method_name = match self.current_token() {
                Token::Symbol(s, _) => s.clone(),
                _ => return Err("Protocol method name must be a symbol".to_string()),
            };
            self.advance();

            // Parse parameter vector
            if !matches!(self.current_token(), Token::LBracket(_)) {
                return Err("Protocol method requires a parameter vector [...]".to_string());
            }
            self.advance(); // consume [

            let mut params = Vec::new();
            while !matches!(self.current_token(), Token::RBracket(_)) {
                if self.current_token() == &self.eof_token {
                    return Err("Unclosed parameter vector in protocol method".to_string());
                }

                let param = match self.current_token() {
                    Token::Symbol(s, _) => s.clone(),
                    _ => return Err("Protocol method parameter must be a symbol".to_string()),
                };
                self.advance();
                params.push(param);
            }

            self.expect(Token::RBracket(Span::dummy()))?; // consume ]

            // Optional docstring
            let docstring = if let Token::String(s, _) = self.current_token() {
                let doc = s.clone();
                self.advance();
                Some(doc)
            } else {
                None
            };

            self.expect(Token::RParen(Span::dummy()))?; // consume ) for method

            methods.push(crate::ast::ProtocolMethod {
                name: method_name,
                params,
                docstring,
            });
        }

        self.expect(Token::RParen(Span::dummy()))?; // consume ) for defprotocol

        Ok(Expr::Defprotocol { name, methods })
    }

    fn parse_extend_type(&mut self) -> Result<Expr, String> {
        // (extend-type Point Drawable (draw [this] ...) (bounds [this] ...))
        // Already saw 'extend-type', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "extend-type" {
                self.advance();
            }
        }

        // Get type name
        let type_name = match self.current_token() {
            Token::Symbol(s, _) => s.clone(),
            _ => return Err("extend-type requires a type name".to_string()),
        };
        self.advance();

        // Get protocol name
        let protocol_name = match self.current_token() {
            Token::Symbol(s, _) => s.clone(),
            _ => return Err("extend-type requires a protocol name".to_string()),
        };
        self.advance();

        // Parse method implementations: (method-name [params] body...)
        let mut methods = Vec::new();

        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed extend-type".to_string());
            }

            // Each method is a list: (method-name [params] body)
            if !matches!(self.current_token(), Token::LParen(_)) {
                return Err("extend-type method must be a list (method-name [params] body)".to_string());
            }
            self.advance(); // consume (

            // Get method name
            let method_name = match self.current_token() {
                Token::Symbol(s, _) => s.clone(),
                _ => return Err("Method name must be a symbol".to_string()),
            };
            self.advance();

            // Parse parameter vector with destructuring support
            if !matches!(self.current_token(), Token::LBracket(_)) {
                return Err("Method requires a parameter vector [...]".to_string());
            }
            self.advance(); // consume [

            let mut params = Vec::new();
            while !matches!(self.current_token(), Token::RBracket(_)) {
                if self.current_token() == &self.eof_token {
                    return Err("Unclosed parameter vector in method".to_string());
                }

                // Use parse_pattern to support destructuring
                params.push(self.parse_pattern()?);
            }

            self.expect(Token::RBracket(Span::dummy()))?; // consume ]

            // Parse body (single expression, or use do for multiple)
            let body = self.parse_expr()?;

            self.expect(Token::RParen(Span::dummy()))?; // consume ) for method

            methods.push(crate::ast::ProtocolMethodImpl {
                name: method_name,
                params,
                body: Box::new(body),
            });
        }

        self.expect(Token::RParen(Span::dummy()))?; // consume ) for extend-type

        Ok(Expr::ExtendType {
            type_name,
            protocol_name,
            methods,
        })
    }

    fn parse_defmulti(&mut self) -> Result<Expr, String> {
        // (defmulti area :type)
        // Already saw 'defmulti', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "defmulti" {
                self.advance();
            }
        }

        // Get multimethod name
        let name = match self.current_token() {
            Token::Symbol(s, _) => s.clone(),
            _ => return Err("defmulti requires a multimethod name".to_string()),
        };
        self.advance();

        // Parse dispatch function (can be any expression)
        let dispatch_fn = self.parse_expr()?;

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Defmulti {
            name,
            dispatch_fn: Box::new(dispatch_fn),
        })
    }

    fn parse_defmethod(&mut self) -> Result<Expr, String> {
        // (defmethod area :circle [shape] (* 3.14 ...))
        // Already saw 'defmethod', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "defmethod" {
                self.advance();
            }
        }

        // Get multimethod name
        let name = match self.current_token() {
            Token::Symbol(s, _) => s.clone(),
            _ => return Err("defmethod requires a multimethod name".to_string()),
        };
        self.advance();

        // Parse dispatch value (can be any expression - keyword, number, etc)
        let dispatch_value = self.parse_expr()?;

        // Parse parameter vector
        if !matches!(self.current_token(), Token::LBracket(_)) {
            return Err("defmethod requires a parameter vector [...]".to_string());
        }
        self.advance(); // consume [

        let mut params = Vec::new();
        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed parameter vector in defmethod".to_string());
            }

            // Use parse_pattern to support destructuring
            params.push(self.parse_pattern()?);
        }

        self.expect(Token::RBracket(Span::dummy()))?; // consume ]

        // Parse body
        let body = self.parse_expr()?;

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Defmethod {
            name,
            dispatch_value: Box::new(dispatch_value),
            params,
            body: Box::new(body),
        })
    }

    fn parse_fn(&mut self) -> Result<Expr, String> {
        // Already saw 'fn', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "fn" {
                self.advance();
            }
        }

        // Check if this is single-arity or multi-arity
        // Single: (fn [x y] body)
        // Multi:  (fn ([] body1) ([x] body2))
        if matches!(self.current_token(), Token::LBracket(_)) {
            // Single arity
            self.parse_single_arity_fn()
        } else if matches!(self.current_token(), Token::LParen(_)) {
            // Multi arity
            self.parse_multi_arity_fn()
        } else {
            Err("fn requires either parameter vector [...] or arity clauses (...)".to_string())
        }
    }

    fn parse_single_arity_fn(&mut self) -> Result<Expr, String> {
        // Expect parameter vector: [x y z]
        if !matches!(self.current_token(), Token::LBracket(_)) {
            return Err("fn requires a parameter vector [...]".to_string());
        }
        self.advance(); // consume [

        // Parse parameters
        let mut params = Vec::new();
        let mut rest_param = None;

        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed parameter vector in fn".to_string());
            }

            // Check for & symbol (rest parameter marker)
            if let Token::Symbol(s, _) = self.current_token() {
                if s == "&" {
                    self.advance(); // consume &

                    // Next symbol should be the rest parameter name
                    let rest_name = match self.current_token() {
                        Token::Symbol(s, _) => s.clone(),
                        _ => return Err("Rest parameter after & must be a symbol".to_string()),
                    };
                    self.advance();

                    rest_param = Some(rest_name);

                    // After rest parameter, there should be no more parameters
                    if !matches!(self.current_token(), Token::RBracket(_)) {
                        return Err("Rest parameter must be the last parameter".to_string());
                    }
                    break;
                }
            }

            // Parse parameter pattern (symbol, vector, map, etc.)
            let param = self.parse_pattern()?;
            params.push(param);
        }

        self.expect(Token::RBracket(Span::dummy()))?; // consume ]

        // Parse body (single expression for now)
        let body = self.parse_expr()?;

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Fn {
            params,
            rest_param,
            body: Box::new(body),
        })
    }

    fn parse_multi_arity_fn(&mut self) -> Result<Expr, String> {
        use crate::ast::FunctionArity;

        let mut arities = Vec::new();

        // Parse each arity clause: ([] body) ([x] body) ([x y] body)
        while matches!(self.current_token(), Token::LParen(_)) {
            self.advance(); // consume (

            // Parse parameter vector
            if !matches!(self.current_token(), Token::LBracket(_)) {
                return Err("Each arity clause must start with parameter vector [...]".to_string());
            }
            self.advance(); // consume [

            let mut params = Vec::new();
            let mut rest_param = None;

            while !matches!(self.current_token(), Token::RBracket(_)) {
                if self.current_token() == &self.eof_token {
                    return Err("Unclosed parameter vector in arity clause".to_string());
                }

                // Check for & symbol
                if let Token::Symbol(s, _) = self.current_token() {
                    if s == "&" {
                        self.advance(); // consume &

                        let rest_name = match self.current_token() {
                            Token::Symbol(s, _) => s.clone(),
                            _ => return Err("Rest parameter after & must be a symbol".to_string()),
                        };
                        self.advance();

                        rest_param = Some(rest_name);

                        if !matches!(self.current_token(), Token::RBracket(_)) {
                            return Err("Rest parameter must be the last parameter".to_string());
                        }
                        break;
                    }
                }

                // Parse parameter pattern (symbol, vector, map, etc.)
                let param = self.parse_pattern()?;
                params.push(param);
            }

            self.expect(Token::RBracket(Span::dummy()))?; // consume ]

            // Parse body
            let body = self.parse_expr()?;

            self.expect(Token::RParen(Span::dummy()))?; // consume ) closing arity clause

            arities.push(FunctionArity {
                params,
                rest_param,
                body: Box::new(body),
            });
        }

        self.expect(Token::RParen(Span::dummy()))?; // consume ) closing fn

        if arities.is_empty() {
            return Err("Multi-arity fn must have at least one arity clause".to_string());
        }

        Ok(Expr::FnMulti { arities })
    }

    fn parse_let(&mut self) -> Result<Expr, String> {
        // Already saw 'let', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "let" {
                self.advance();
            }
        }

        // Expect binding vector: [x 10 y 20]
        if !matches!(self.current_token(), Token::LBracket(_)) {
            return Err("let requires a binding vector [...]".to_string());
        }
        self.advance(); // consume [

        // Parse bindings
        let mut bindings = Vec::new();
        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed binding vector in let".to_string());
            }

            // Parse pattern (symbol, vector, or map destructuring)
            let pattern = self.parse_pattern()?;

            // Get binding value
            let value = self.parse_expr()?;

            bindings.push((pattern, Box::new(value)));
        }

        self.expect(Token::RBracket(Span::dummy()))?; // consume ]

        // Parse body (rest of expressions until closing paren)
        let mut body_exprs = Vec::new();
        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed let form".to_string());
            }
            body_exprs.push(self.parse_expr()?);
        }

        self.expect(Token::RParen(Span::dummy()))?;

        // Body is the last expression (or implicit nil if empty)
        let body = if body_exprs.is_empty() {
            Box::new(Expr::Nil)
        } else if body_exprs.len() == 1 {
            Box::new(body_exprs.into_iter().next().unwrap())
        } else {
            // Multiple expressions in body - wrap in an implicit do
            Box::new(Expr::Do {
                exprs: body_exprs
            })
        };

        Ok(Expr::Let { bindings, body })
    }

    /// Parse letfn: (letfn [(f [x] ...) (g [y] ...)] body)
    /// Creates local function bindings with mutual recursion support
    fn parse_letfn(&mut self) -> Result<Expr, String> {
        // Already saw 'letfn', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "letfn" {
                self.advance();
            }
        }

        // Expect binding vector: [(f [x] ...) (g [y] ...)]
        if !matches!(self.current_token(), Token::LBracket(_)) {
            return Err("letfn requires a binding vector [...]".to_string());
        }
        self.advance(); // consume [

        // Parse function bindings
        let mut bindings = Vec::new();
        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed binding vector in letfn".to_string());
            }

            // Each binding should be a list: (name [params] body)
            if !matches!(self.current_token(), Token::LParen(_)) {
                return Err("letfn binding must be a list (name [params] body)".to_string());
            }
            self.advance(); // consume (

            // Parse function name
            let name = match self.current_token() {
                Token::Symbol(s, _) => s.clone(),
                _ => return Err("letfn binding must start with function name".to_string()),
            };
            self.advance();

            // Parse parameter vector
            if !matches!(self.current_token(), Token::LBracket(_)) {
                return Err(format!("letfn function '{}' requires parameter vector", name));
            }
            self.advance(); // consume [

            let mut params = Vec::new();
            let mut rest_param = None;

            while !matches!(self.current_token(), Token::RBracket(_)) {
                if self.current_token() == &self.eof_token {
                    return Err(format!("Unclosed parameter vector for function '{}'", name));
                }

                // Check for & (rest parameter)
                if let Token::Symbol(s, _) = self.current_token() {
                    if s == "&" {
                        self.advance(); // consume &
                        // Next should be rest parameter name
                        match self.current_token() {
                            Token::Symbol(rest_name, _) => {
                                rest_param = Some(rest_name.clone());
                                self.advance();
                                break; // & must be last
                            }
                            _ => return Err(format!("Expected rest parameter name after & in function '{}'", name)),
                        }
                    }
                }

                // Parse parameter pattern
                params.push(self.parse_pattern()?);
            }

            self.expect(Token::RBracket(Span::dummy()))?; // consume ]

            // Parse function body (rest of expressions until closing paren)
            let mut body_exprs = Vec::new();
            while !matches!(self.current_token(), Token::RParen(_)) {
                if self.current_token() == &self.eof_token {
                    return Err(format!("Unclosed function body for '{}'", name));
                }
                body_exprs.push(self.parse_expr()?);
            }

            self.expect(Token::RParen(Span::dummy()))?; // consume )

            // Function body is the last expression (or implicit nil if empty)
            let body = if body_exprs.is_empty() {
                Box::new(Expr::Nil)
            } else if body_exprs.len() == 1 {
                Box::new(body_exprs.into_iter().next().unwrap())
            } else {
                // Multiple expressions in body - wrap in an implicit do
                Box::new(Expr::Do {
                    exprs: body_exprs
                })
            };

            bindings.push((name, params, rest_param, body));
        }

        self.expect(Token::RBracket(Span::dummy()))?; // consume ]

        // Parse letfn body (rest of expressions until closing paren)
        let mut body_exprs = Vec::new();
        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed letfn form".to_string());
            }
            body_exprs.push(self.parse_expr()?);
        }

        self.expect(Token::RParen(Span::dummy()))?;

        // Body is the last expression (or implicit nil if empty)
        let body = if body_exprs.is_empty() {
            Box::new(Expr::Nil)
        } else if body_exprs.len() == 1 {
            Box::new(body_exprs.into_iter().next().unwrap())
        } else {
            // Multiple expressions in body - wrap in an implicit do
            Box::new(Expr::Do {
                exprs: body_exprs
            })
        };

        Ok(Expr::Letfn { bindings, body })
    }

    /// Parse a destructuring pattern
    fn parse_pattern(&mut self) -> Result<crate::ast::Pattern, String> {
        use crate::ast::Pattern;

        match self.current_token() {
            Token::Symbol(s, _) if s == "_" => {
                self.advance();
                Ok(Pattern::Ignore)
            }

            Token::Symbol(s, _) => {
                let name = s.clone();
                self.advance();
                Ok(Pattern::Symbol(name))
            }

            Token::LBracket(_) => self.parse_vector_pattern(),
            Token::LBrace(_) => self.parse_map_pattern(),

            _ => Err(format!("Invalid pattern: {:?}", self.current_token()))
        }
    }

    /// Parse vector destructuring pattern: [a b c] or [a b & rest]
    fn parse_vector_pattern(&mut self) -> Result<crate::ast::Pattern, String> {
        use crate::ast::Pattern;

        self.expect(Token::LBracket(Span::dummy()))?;

        let mut elements = Vec::new();
        let mut rest_param = None;

        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed vector pattern".to_string());
            }

            // Check for & (rest parameter)
            if let Token::Symbol(s, _) = self.current_token() {
                if s == "&" {
                    self.advance(); // consume &

                    // Next must be a symbol
                    let rest_name = match self.current_token() {
                        Token::Symbol(s, _) => s.clone(),
                        _ => return Err("Rest parameter must be a symbol".to_string()),
                    };
                    self.advance();

                    rest_param = Some(rest_name);

                    // Rest must be last
                    if !matches!(self.current_token(), Token::RBracket(_)) {
                        return Err("Rest parameter must be last in pattern".to_string());
                    }
                    break;
                }
            }

            // Recursively parse nested pattern
            elements.push(self.parse_pattern()?);
        }

        self.expect(Token::RBracket(Span::dummy()))?;
        Ok(Pattern::Vector { elements, rest: rest_param, as_binding: None })
    }

    /// Parse map destructuring pattern: {:keys [x y]}
    fn parse_map_pattern(&mut self) -> Result<crate::ast::Pattern, String> {
        use crate::ast::{MapPatternKey, Pattern};

        self.expect(Token::LBrace(Span::dummy()))?;

        // For MVP: Only support :keys shorthand
        if let Token::Keyword(kw, _) = self.current_token() {
            if kw == "keys" {
                self.advance(); // consume :keys

                // Expect vector of symbols
                if !matches!(self.current_token(), Token::LBracket(_)) {
                    return Err(":keys requires a vector of symbols".to_string());
                }
                self.advance(); // consume [

                let mut bindings = Vec::new();
                while !matches!(self.current_token(), Token::RBracket(_)) {
                    match self.current_token() {
                        Token::Symbol(s, _) => {
                            let name = s.clone();
                            bindings.push((
                                MapPatternKey::Symbol(name.clone()),
                                Pattern::Symbol(name)
                            ));
                            self.advance();
                        }
                        _ => return Err(":keys vector must contain symbols".to_string()),
                    }
                }

                self.expect(Token::RBracket(Span::dummy()))?;
                self.expect(Token::RBrace(Span::dummy()))?;
                return Ok(Pattern::Map { bindings, defaults: None });
            }
        }

        // Long-form map destructuring: {bx :x by :y ...}
        let mut bindings = Vec::new();

        while !matches!(self.current_token(), Token::RBrace(_)) {
            // Expect: symbol (binding name)
            let binding_name = match self.current_token() {
                Token::Symbol(s, _) => s.clone(),
                _ => return Err("Map destructuring requires symbol before keyword".to_string()),
            };
            self.advance();

            // Expect: keyword (map key)
            let map_key = match self.current_token() {
                Token::Keyword(k, _) => k.clone(),
                _ => return Err(format!("Expected keyword after {} in map destructuring", binding_name)),
            };
            self.advance();

            bindings.push((
                MapPatternKey::Symbol(map_key),
                Pattern::Symbol(binding_name)
            ));
        }

        self.expect(Token::RBrace(Span::dummy()))?;
        Ok(Pattern::Map { bindings, defaults: None })
    }

    fn parse_def(&mut self) -> Result<Expr, String> {
        // Already saw 'def', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "def" {
                self.advance();
            }
        }

        // Get variable name
        let name = match self.current_token() {
            Token::Symbol(s, _) => s.clone(),
            _ => return Err("def requires a symbol name".to_string()),
        };
        self.advance();

        // Get value
        let value = self.parse_expr()?;

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Def {
            name,
            value: Box::new(value),
            metadata: None,
        })
    }

    fn parse_if(&mut self) -> Result<Expr, String> {
        // Already saw 'if', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "if" {
                self.advance();
            }
        }

        // Parse condition
        let condition = self.parse_expr()?;

        // Parse then branch
        let then_branch = self.parse_expr()?;

        // Parse else branch
        let else_branch = self.parse_expr()?;

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        })
    }

    fn parse_do(&mut self) -> Result<Expr, String> {
        // Already saw 'do', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "do" {
                self.advance();
            }
        }

        // Parse all expressions until )
        let mut exprs = Vec::new();
        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed do expression".to_string());
            }
            exprs.push(self.parse_expr()?);
        }

        self.expect(Token::RParen(Span::dummy()))?;

        // do must have at least one expression
        if exprs.is_empty() {
            return Err("do requires at least one expression".to_string());
        }

        Ok(Expr::Do { exprs })
    }

    fn parse_dosync(&mut self) -> Result<Expr, String> {
        // Already saw 'dosync', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "dosync" {
                self.advance();
            }
        }

        // Parse all expressions until )
        let mut exprs = Vec::new();
        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed dosync expression".to_string());
            }
            exprs.push(self.parse_expr()?);
        }

        self.expect(Token::RParen(Span::dummy()))?;

        // dosync must have at least one expression
        if exprs.is_empty() {
            return Err("dosync requires at least one expression".to_string());
        }

        Ok(Expr::Dosync { exprs })
    }

    fn parse_quote(&mut self) -> Result<Expr, String> {
        // (quote x)
        // Already saw 'quote', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "quote" {
                self.advance();
            }
        }

        // Parse the quoted expression
        let quoted_expr = self.parse_expr()?;

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Quote {
            expr: Box::new(quoted_expr),
        })
    }

    fn parse_use(&mut self) -> Result<Expr, String> {
        // (use rust.fs) or (use rust.fs [read write])
        self.advance(); // Skip 'use'

        // Parse module path (e.g., rust.fs)
        let module = match self.current_token() {
            Token::Symbol(name, _) => {
                let module_name = name.clone();
                self.advance();
                module_name
            }
            _ => return Err("use requires a module name (e.g., rust.fs)".to_string()),
        };

        // Check for optional import list [read write]
        let imports = if matches!(self.current_token(), Token::LBracket(_)) {
            self.advance(); // Skip [
            let mut import_list = Vec::new();

            while !matches!(self.current_token(), Token::RBracket(_)) {
                if matches!(self.current_token(), Token::Eof(_)) {
                    return Err("Unclosed import list in use".to_string());
                }

                match self.current_token() {
                    Token::Symbol(name, _) => {
                        import_list.push(name.clone());
                        self.advance();
                    }
                    _ => return Err("Import names must be symbols".to_string()),
                }
            }

            self.expect(Token::RBracket(Span::dummy()))?;
            import_list
        } else {
            Vec::new() // Empty = import all
        };

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Use { module, imports })
    }

    fn parse_loop(&mut self) -> Result<Expr, String> {
        // (loop [x 0 y 10] body)
        // Already saw 'loop', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "loop" {
                self.advance();
            }
        }

        // Expect binding vector: [x 0 y 10]
        if !matches!(self.current_token(), Token::LBracket(_)) {
            return Err("loop requires a binding vector [...]".to_string());
        }
        self.advance(); // consume [

        // Parse bindings (same as let)
        let mut bindings = Vec::new();
        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed binding vector in loop".to_string());
            }

            // Parse pattern (symbol, vector, or map destructuring)
            let pattern = self.parse_pattern()?;

            // Get binding value
            let value = self.parse_expr()?;

            bindings.push((pattern, Box::new(value)));
        }

        self.expect(Token::RBracket(Span::dummy()))?; // consume ]

        // Parse body (single expression)
        let body = self.parse_expr()?;

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Loop { bindings, body: Box::new(body) })
    }

    fn parse_recur(&mut self) -> Result<Expr, String> {
        // (recur new-x new-y)
        // Already saw 'recur', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "recur" {
                self.advance();
            }
        }

        // Parse arguments
        let mut args = Vec::new();
        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed recur expression".to_string());
            }
            args.push(self.parse_expr()?);
        }

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Recur { args })
    }

    fn parse_try(&mut self) -> Result<Expr, String> {
        // (try
        //   body-expr
        //   (catch ExceptionType e handler-expr)
        //   (catch AnotherType e2 handler-expr2)
        //   (finally cleanup-expr))

        // Already saw 'try', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "try" {
                self.advance();
            }
        }

        // Parse body expression (everything until we see catch or finally)
        let body = self.parse_expr()?;

        let mut catch_clauses = Vec::new();
        let mut finally_block = None;

        // Parse catch and finally clauses
        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed try expression".to_string());
            }

            // Must be either (catch ...) or (finally ...)
            if !matches!(self.current_token(), Token::LParen(_)) {
                return Err("Expected catch or finally clause in try".to_string());
            }
            self.advance(); // consume (

            match self.current_token() {
                Token::Symbol(s, _) if s == "catch" => {
                    self.advance(); // consume 'catch'

                    // Parse exception type (optional - if not present, catch all)
                    let exception_type = if matches!(self.current_token(), Token::Symbol(_, _)) {
                        let next = self.current_token().clone();
                        if let Token::Symbol(type_name, _) = next {
                            self.advance();
                            Some(type_name)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Parse binding variable
                    let binding = match self.current_token() {
                        Token::Symbol(var, _) => {
                            let v = var.clone();
                            self.advance();
                            v
                        }
                        _ => return Err("catch requires a binding variable".to_string()),
                    };

                    // Parse handler expression
                    let handler = self.parse_expr()?;

                    self.expect(Token::RParen(Span::dummy()))?; // close catch clause

                    catch_clauses.push(crate::ast::CatchClause {
                        exception_type,
                        binding,
                        handler: Box::new(handler),
                    });
                }

                Token::Symbol(s, _) if s == "finally" => {
                    self.advance(); // consume 'finally'

                    // Parse finally expression
                    let finally_expr = self.parse_expr()?;

                    self.expect(Token::RParen(Span::dummy()))?; // close finally clause

                    finally_block = Some(Box::new(finally_expr));

                    // finally must be last, so break
                    break;
                }

                _ => return Err("Expected 'catch' or 'finally' in try expression".to_string()),
            }
        }

        self.expect(Token::RParen(Span::dummy()))?; // close try

        Ok(Expr::Try {
            body: Box::new(body),
            catch_clauses,
            finally_block,
        })
    }

    fn parse_throw(&mut self) -> Result<Expr, String> {
        // (throw exception-expr)
        // Already saw 'throw', consume it
        if let Token::Symbol(s, _) = self.current_token() {
            if s == "throw" {
                self.advance();
            }
        }

        // Parse the expression to throw
        let expr = self.parse_expr()?;

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Throw {
            expr: Box::new(expr),
        })
    }

    fn parse_ns(&mut self) -> Result<Expr, String> {
        // (ns my.app.core
        //   (:require [my.lib :as lib] [other.lib :refer [func1 func2]])
        //   (:rust [egui-hello :as gui]))

        self.advance(); // Skip 'ns'

        // Parse namespace name
        let name = match self.current_token() {
            Token::Symbol(ns_name, _) => {
                let name = ns_name.clone();
                self.advance();
                name
            }
            _ => return Err("ns requires a namespace name (e.g., my.app.core)".to_string()),
        };

        // Skip optional docstring (Clojure compatibility)
        if matches!(self.current_token(), Token::String(_, _)) {
            self.advance();
        }

        let mut requires = Vec::new();
        let mut rust_imports = Vec::new();

        // Parse optional clauses: (:require ...) (:rust ...)
        while !matches!(self.current_token(), Token::RParen(_)) {
            if matches!(self.current_token(), Token::Eof(_)) {
                return Err("Unclosed ns declaration".to_string());
            }

            // Expect a clause starting with (
            if !matches!(self.current_token(), Token::LParen(_)) {
                return Err("ns clauses must be lists starting with :require or :rust".to_string());
            }
            self.advance(); // Skip (

            // Check the clause type
            match self.current_token() {
                Token::Keyword(kw, _) if kw == "require" => {
                    self.advance(); // Skip :require
                    // Parse all require specs until )
                    while !matches!(self.current_token(), Token::RParen(_)) {
                        requires.push(self.parse_require_spec()?);
                    }
                    self.expect(Token::RParen(Span::dummy()))?;
                }
                Token::Keyword(kw, _) if kw == "rust" => {
                    self.advance(); // Skip :rust
                    // Parse all rust import specs until )
                    while !matches!(self.current_token(), Token::RParen(_)) {
                        rust_imports.push(self.parse_rust_import_spec()?);
                    }
                    self.expect(Token::RParen(Span::dummy()))?;
                }
                _ => return Err("Unknown ns clause. Expected :require or :rust".to_string()),
            }
        }

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Ns {
            name,
            requires,
            rust_imports,
        })
    }

    fn parse_require(&mut self) -> Result<Expr, String> {
        // (require [my.lib :as lib :refer [func1 func2]]
        //          [other.lib :refer :all])

        self.advance(); // Skip 'require'

        let mut specs = Vec::new();

        while !matches!(self.current_token(), Token::RParen(_)) {
            if matches!(self.current_token(), Token::Eof(_)) {
                return Err("Unclosed require statement".to_string());
            }

            specs.push(self.parse_require_spec()?);
        }

        self.expect(Token::RParen(Span::dummy()))?;

        Ok(Expr::Require { specs })
    }

    fn parse_declare(&mut self) -> Result<Expr, String> {
        // (declare func1 func2 func3 ...)
        // Forward declare function names for mutual recursion

        self.advance(); // Skip 'declare'

        let mut names = Vec::new();

        while !matches!(self.current_token(), Token::RParen(_)) {
            if matches!(self.current_token(), Token::Eof(_)) {
                return Err("Unclosed declare statement".to_string());
            }

            // Each name should be a symbol
            match self.current_token() {
                Token::Symbol(name, _) => {
                    names.push(name.clone());
                    self.advance();
                }
                _ => return Err("declare only accepts function names (symbols)".to_string()),
            }
        }

        self.expect(Token::RParen(Span::dummy()))?;

        if names.is_empty() {
            return Err("declare requires at least one function name".to_string());
        }

        Ok(Expr::Declare { names })
    }

    fn parse_require_spec(&mut self) -> Result<crate::ast::RequireSpec, String> {
        // [my.lib :as lib :refer [func1 func2]]
        // or [my.lib :refer :all]

        self.expect(Token::LBracket(Span::dummy()))?;

        // Parse module name
        let module = match self.current_token() {
            Token::Symbol(name, _) => {
                let mod_name = name.clone();
                self.advance();
                mod_name
            }
            _ => return Err("require spec must start with a module name".to_string()),
        };

        let mut alias = None;
        let mut refer = Vec::new();
        let mut refer_all = false;

        // Parse options: :as alias, :refer [...]
        while !matches!(self.current_token(), Token::RBracket(_)) {
            match self.current_token() {
                Token::Keyword(kw, _) if kw == "as" => {
                    self.advance(); // Skip :as
                    match self.current_token() {
                        Token::Symbol(alias_name, _) => {
                            alias = Some(alias_name.clone());
                            self.advance();
                        }
                        _ => return Err(":as requires an alias name".to_string()),
                    }
                }
                Token::Keyword(kw, _) if kw == "refer" => {
                    self.advance(); // Skip :refer

                    // Check for :all
                    if matches!(self.current_token(), Token::Keyword(kw, _) if kw == "all") {
                        refer_all = true;
                        self.advance();
                    } else if matches!(self.current_token(), Token::LBracket(_)) {
                        // Parse [func1 func2 ...]
                        self.advance(); // Skip [
                        while !matches!(self.current_token(), Token::RBracket(_)) {
                            match self.current_token() {
                                Token::Symbol(name, _) => {
                                    refer.push(name.clone());
                                    self.advance();
                                }
                                _ => return Err(":refer list must contain symbols".to_string()),
                            }
                        }
                        self.expect(Token::RBracket(Span::dummy()))?;
                    } else {
                        return Err(":refer requires :all or a vector [func1 func2]".to_string());
                    }
                }
                _ => return Err(format!("Unknown option in require spec: {:?}", self.current_token())),
            }
        }

        self.expect(Token::RBracket(Span::dummy()))?;

        Ok(crate::ast::RequireSpec {
            module,
            alias,
            refer,
            refer_all,
        })
    }

    fn parse_rust_import_spec(&mut self) -> Result<crate::ast::RustImport, String> {
        // [egui-hello :as gui]

        self.expect(Token::LBracket(Span::dummy()))?;

        // Parse library name
        let library = match self.current_token() {
            Token::Symbol(name, _) => {
                let lib_name = name.clone();
                self.advance();
                lib_name
            }
            _ => return Err("rust import spec must start with a library name".to_string()),
        };

        let mut alias = None;

        // Parse :as option
        while !matches!(self.current_token(), Token::RBracket(_)) {
            match self.current_token() {
                Token::Keyword(kw, _) if kw == "as" => {
                    self.advance(); // Skip :as
                    match self.current_token() {
                        Token::Symbol(alias_name, _) => {
                            alias = Some(alias_name.clone());
                            self.advance();
                        }
                        _ => return Err(":as requires an alias name".to_string()),
                    }
                }
                _ => return Err(format!("Unknown option in rust import: {:?}", self.current_token())),
            }
        }

        self.expect(Token::RBracket(Span::dummy()))?;

        Ok(crate::ast::RustImport { library, alias })
    }

    fn parse_shorthand_fn(&mut self) -> Result<Expr, String> {
        // Already consumed #( token, now parse body until )
        // Shorthand syntax: #(* % 2) => (fn [%] (* % 2))
        //                   #(+ %1 %2) => (fn [%1 %2] (+ %1 %2))

        // Parse all elements until we hit )
        let mut elements = Vec::new();

        while !matches!(self.current_token(), Token::RParen(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed shorthand fn".to_string());
            }

            elements.push(self.parse_expr()?);
        }

        self.expect(Token::RParen(Span::dummy()))?;

        // The body is the list of elements
        let body = if elements.is_empty() {
            return Err("Shorthand fn body cannot be empty".to_string());
        } else if elements.len() == 1 {
            // Single expression: #(foo) => just foo
            elements.into_iter().next().unwrap()
        } else {
            // Multiple expressions: #(+ 1 2) => (+ 1 2)
            // Convert to Expr::Call if first element is a symbol (function call)
            if let Expr::Symbol(func_name) = &elements[0] {
                Expr::Call {
                    func: func_name.clone(),
                    args: elements[1..].to_vec(),
                }
            } else {
                Expr::List(elements)
            }
        };

        // Collect % parameters from the parsed body expression
        let param_set = self.collect_shorthand_params(&body)?;

        // Generate parameter list from collected params
        let params = self.generate_param_list(&param_set)?;

        // Transform to Fn expression
        Ok(Expr::Fn {
            params,
            rest_param: None, // Shorthand functions don't support rest params
            body: Box::new(body),
        })
    }

    /// Collect all shorthand parameter symbols from an expression
    /// Returns a set of parameter names: "%", "%1", "%2", etc.
    fn collect_shorthand_params(&self, expr: &Expr) -> Result<std::collections::HashSet<String>, String> {
        self.collect_shorthand_params_impl(expr, 0)
    }

    fn collect_shorthand_params_impl(&self, expr: &Expr, depth: usize) -> Result<std::collections::HashSet<String>, String> {
        use std::collections::HashSet;

        if depth > 100 {
            return Err("Maximum recursion depth exceeded in shorthand fn".to_string());
        }

        let mut params = HashSet::new();

        match expr {
            Expr::Symbol(s) => {
                if s.starts_with('%') {
                    params.insert(s.clone());
                }
                // Non-% symbols are ignored
            }

            Expr::List(elements) | Expr::Vector(elements) | Expr::Set(elements) => {
                for elem in elements {
                    params.extend(self.collect_shorthand_params_impl(elem, depth + 1)?);
                }
            }

            Expr::Map(pairs) => {
                for (k, v) in pairs {
                    params.extend(self.collect_shorthand_params_impl(k, depth + 1)?);
                    params.extend(self.collect_shorthand_params_impl(v, depth + 1)?);
                }
            }

            Expr::Let { bindings, body } => {
                for (_, value) in bindings {
                    params.extend(self.collect_shorthand_params_impl(value, depth + 1)?);
                }
                params.extend(self.collect_shorthand_params_impl(body, depth + 1)?);
            }

            Expr::Def { value, .. } => {
                params.extend(self.collect_shorthand_params_impl(value, depth + 1)?);
            }

            Expr::Defn { .. } | Expr::Fn { .. } => {
                // Don't recurse into nested fn bodies for shorthand param collection
                // Nested shorthand fns will handle their own % parameters
                // Only scan the current level
                // params.extend(self.collect_shorthand_params_impl(body, depth + 1)?);
            }

            Expr::Defmacro { .. } => {
                // Don't recurse into macro bodies for shorthand param collection
                // Macros operate on code, not shorthand functions
            }

            Expr::Defrecord { .. } => {
                // Don't recurse into record definitions for shorthand param collection
                // Records are just field declarations, no expressions
            }

            Expr::Deftype { .. } => {
                // Don't recurse into type definitions for shorthand param collection
                // Types are field + protocol declarations handled separately
            }

            Expr::Defprotocol { .. } => {
                // Don't recurse into protocol definitions for shorthand param collection
                // Protocols are just method signatures, no expressions
            }

            Expr::ExtendType { .. } => {
                // Don't recurse into extend-type for shorthand param collection
                // Method bodies are handled separately
            }

            Expr::Defmulti { .. } => {
                // Don't recurse into defmulti for shorthand param collection
                // Dispatch function is handled separately
            }

            Expr::Defmethod { .. } => {
                // Don't recurse into defmethod for shorthand param collection
                // Method bodies are handled separately
            }

            Expr::DefnMulti { .. } | Expr::FnMulti { .. } => {
                // Don't recurse into multi-arity function bodies either
                // Each arity clause handles its own parameters
            }

            Expr::Call { args, .. } => {
                for arg in args {
                    params.extend(self.collect_shorthand_params_impl(arg, depth + 1)?);
                }
            }

            Expr::If { condition, then_branch, else_branch } => {
                params.extend(self.collect_shorthand_params_impl(condition, depth + 1)?);
                params.extend(self.collect_shorthand_params_impl(then_branch, depth + 1)?);
                params.extend(self.collect_shorthand_params_impl(else_branch, depth + 1)?);
            }

            Expr::Do { exprs } => {
                for expr in exprs {
                    params.extend(self.collect_shorthand_params_impl(expr, depth + 1)?);
                }
            }

            Expr::Dosync { exprs } => {
                for expr in exprs {
                    params.extend(self.collect_shorthand_params_impl(expr, depth + 1)?);
                }
            }

            Expr::Quote { expr } => {
                // Recurse into quoted expressions to find % parameters
                params.extend(self.collect_shorthand_params_impl(expr, depth + 1)?);
            }

            Expr::SyntaxQuote { expr } | Expr::Unquote { expr } | Expr::UnquoteSplicing { expr } | Expr::Deref { expr } => {
                // Recurse into syntax-quoted/unquoted/deref expressions
                params.extend(self.collect_shorthand_params_impl(expr, depth + 1)?);
            }

            Expr::Loop { bindings, body } => {
                // Recurse into loop bindings and body
                for (_, value) in bindings {
                    params.extend(self.collect_shorthand_params_impl(value, depth + 1)?);
                }
                params.extend(self.collect_shorthand_params_impl(body, depth + 1)?);
            }

            Expr::Recur { args } => {
                // Recurse into recur arguments
                for arg in args {
                    params.extend(self.collect_shorthand_params_impl(arg, depth + 1)?);
                }
            }

            Expr::Try { body, catch_clauses, finally_block } => {
                // Recurse into try body
                params.extend(self.collect_shorthand_params_impl(body, depth + 1)?);

                // Recurse into catch handlers
                for clause in catch_clauses {
                    params.extend(self.collect_shorthand_params_impl(&clause.handler, depth + 1)?);
                }

                // Recurse into finally block
                if let Some(finally) = finally_block {
                    params.extend(self.collect_shorthand_params_impl(finally, depth + 1)?);
                }
            }

            Expr::Throw { expr } => {
                // Recurse into throw expression
                params.extend(self.collect_shorthand_params_impl(expr, depth + 1)?);
            }

            Expr::Declare { .. } => {
                // Declare doesn't contain expressions, just function names
            }

            Expr::Var { .. } => {
                // Var references don't contain % parameters
            }

            Expr::Binding { bindings, body } => {
                // Recurse into binding values and body
                for (_, value) in bindings {
                    params.extend(self.collect_shorthand_params_impl(value, depth + 1)?);
                }
                params.extend(self.collect_shorthand_params_impl(body, depth + 1)?);
            }

            // Atoms don't contain parameters
            Expr::Long(_) | Expr::Double(_) | Expr::String(_) | Expr::Keyword(_)
            | Expr::Bool(_) | Expr::Nil | Expr::Ns { .. }
            | Expr::Require { .. } | Expr::Use { .. } | Expr::Letfn { .. } => {}
        }

        Ok(params)
    }

    /// Generate ordered parameter list from shorthand symbols
    /// % => ["%"]
    /// %1, %3, %2 => ["%1", "%2", "%3"]
    /// % and %2 => ["%", "%2"]  (% is always first)
    fn generate_param_list(&self, symbols: &std::collections::HashSet<String>) -> Result<Vec<crate::ast::Pattern>, String> {
        use crate::ast::Pattern;

        let mut params = Vec::new();
        let mut numbered_params = Vec::new();

        for sym in symbols {
            if sym == "%" {
                // Plain % is the first (and often only) parameter
                params.push(Pattern::Symbol("%".to_string()));
            } else if sym == "%&" {
                // TODO: Handle rest parameters in future
                return Err("Shorthand rest parameters (%&) not yet supported".to_string());
            } else if sym.starts_with('%') {
                // Numbered parameter like %1, %2, %3
                let num_str = &sym[1..];
                if let Ok(num) = num_str.parse::<usize>() {
                    numbered_params.push((num, sym.clone()));
                } else {
                    return Err(format!("Invalid shorthand parameter: {}", sym));
                }
            }
        }

        // If we have both % and numbered params, that's an error
        if !params.is_empty() && !numbered_params.is_empty() {
            return Err("Cannot mix % and %N parameters in shorthand fn".to_string());
        }

        // Sort numbered parameters by their number
        if !numbered_params.is_empty() {
            numbered_params.sort_by_key(|(num, _)| *num);

            // Check for gaps in numbering (e.g., %1 and %3 without %2)
            let expected_nums: Vec<usize> = (1..=numbered_params.len()).collect();
            let actual_nums: Vec<usize> = numbered_params.iter().map(|(n, _)| *n).collect();
            if expected_nums != actual_nums {
                return Err(format!("Shorthand parameters must be sequential: got {:?}, expected {:?}",
                    actual_nums, expected_nums));
            }

            params.extend(numbered_params.into_iter().map(|(_, sym)| Pattern::Symbol(sym)));
        }

        // Allow zero-parameter shorthand functions (Clojure compatibility)
        // #(reset! count 0) => (fn [] (reset! count 0))
        // If no parameters found, return empty params list

        Ok(params)
    }

    fn parse_vector(&mut self) -> Result<Expr, String> {
        self.expect(Token::LBracket(Span::dummy()))?;
        let mut elements = Vec::new();

        while !matches!(self.current_token(), Token::RBracket(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed vector".to_string());
            }
            elements.push(self.parse_expr()?);
        }

        self.expect(Token::RBracket(Span::dummy()))?;
        Ok(Expr::Vector(elements))
    }

    fn parse_map(&mut self) -> Result<Expr, String> {
        self.expect(Token::LBrace(Span::dummy()))?;
        let mut pairs = Vec::new();

        while !matches!(self.current_token(), Token::RBrace(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed map".to_string());
            }

            let key = self.parse_expr()?;
            if matches!(self.current_token(), Token::RBrace(_)) {
                return Err("Map has odd number of elements".to_string());
            }
            let value = self.parse_expr()?;

            pairs.push((key, value));
        }

        self.expect(Token::RBrace(Span::dummy()))?;
        Ok(Expr::Map(pairs))
    }

    fn parse_set(&mut self) -> Result<Expr, String> {
        // Note: HashSetStart already consumed the #{ tokens
        let mut elements = Vec::new();

        while !matches!(self.current_token(), Token::RBrace(_)) {
            if self.current_token() == &self.eof_token {
                return Err("Unclosed set".to_string());
            }
            elements.push(self.parse_expr()?);
        }

        self.expect(Token::RBrace(Span::dummy()))?;
        Ok(Expr::Set(elements))
    }

    pub fn parse(&mut self) -> Result<Vec<Expr>, String> {
        let mut expressions = Vec::new();

        while !matches!(self.current_token(), Token::Eof(_)) {
            expressions.push(self.parse_expr()?);
        }

        Ok(expressions)
    }
}

/// Convenience function to parse a string directly
pub fn parse_str(input: &str) -> Result<Vec<Expr>, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_atoms() {
        let exprs = parse_str("42 \"hello\" true nil :keyword").unwrap();
        assert_eq!(exprs.len(), 5);
        assert_eq!(exprs[0], Expr::Long(42));
        assert_eq!(exprs[1], Expr::String("hello".to_string()));
        assert_eq!(exprs[2], Expr::Bool(true));
        assert_eq!(exprs[3], Expr::Nil);
        assert_eq!(exprs[4], Expr::Keyword("keyword".to_string()));
    }

    #[test]
    fn test_parse_list() {
        let exprs = parse_str("(+ 1 2)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::List(list) = &exprs[0] {
            assert_eq!(list.len(), 3);
            assert_eq!(list[0], Expr::Symbol("+".to_string()));
            assert_eq!(list[1], Expr::Long(1));
            assert_eq!(list[2], Expr::Long(2));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_vector() {
        let exprs = parse_str("[1 2 3]").unwrap();
        if let Expr::Vector(vec) = &exprs[0] {
            assert_eq!(vec.len(), 3);
        } else {
            panic!("Expected vector");
        }
    }

    #[test]
    fn test_parse_map() {
        let exprs = parse_str(r#"{:name "Alice" :age 30}"#).unwrap();
        if let Expr::Map(map) = &exprs[0] {
            assert_eq!(map.len(), 2);
        } else {
            panic!("Expected map");
        }
    }

    #[test]
    fn test_parse_nested() {
        let exprs = parse_str("(defn add [x y] (+ x y))").unwrap();
        // defn is now a special form, so it should be parsed as Expr::Defn
        if let Expr::Defn { name, params, .. } = &exprs[0] {
            assert_eq!(name, "add");
            assert_eq!(params, &vec![
                crate::ast::Pattern::Symbol("x".to_string()),
                crate::ast::Pattern::Symbol("y".to_string())
            ]);
        } else {
            panic!("Expected Defn expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_simple_ns() {
        let exprs = parse_str("(ns my.app.core)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Ns { name, requires, rust_imports } = &exprs[0] {
            assert_eq!(name, "my.app.core");
            assert!(requires.is_empty());
            assert!(rust_imports.is_empty());
        } else {
            panic!("Expected Ns expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_ns_with_require() {
        let exprs = parse_str("(ns my.app.core (:require [my.lib :as lib]))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Ns { name, requires, rust_imports } = &exprs[0] {
            assert_eq!(name, "my.app.core");
            assert_eq!(requires.len(), 1);
            assert_eq!(requires[0].module, "my.lib");
            assert_eq!(requires[0].alias, Some("lib".to_string()));
            assert!(requires[0].refer.is_empty());
            assert!(!requires[0].refer_all);
            assert!(rust_imports.is_empty());
        } else {
            panic!("Expected Ns expression");
        }
    }

    #[test]
    fn test_parse_ns_with_multiple_requires() {
        let exprs = parse_str(
            "(ns my.app.core (:require [my.lib :as lib] [other.lib :refer [func1 func2]]))"
        ).unwrap();

        if let Expr::Ns { name, requires, .. } = &exprs[0] {
            assert_eq!(name, "my.app.core");
            assert_eq!(requires.len(), 2);

            // First require
            assert_eq!(requires[0].module, "my.lib");
            assert_eq!(requires[0].alias, Some("lib".to_string()));

            // Second require
            assert_eq!(requires[1].module, "other.lib");
            assert_eq!(requires[1].refer, vec!["func1".to_string(), "func2".to_string()]);
        } else {
            panic!("Expected Ns expression");
        }
    }

    #[test]
    fn test_parse_ns_with_refer_all() {
        let exprs = parse_str("(ns my.app.core (:require [my.lib :refer :all]))").unwrap();

        if let Expr::Ns { requires, .. } = &exprs[0] {
            assert_eq!(requires.len(), 1);
            assert_eq!(requires[0].module, "my.lib");
            assert!(requires[0].refer_all);
            assert!(requires[0].refer.is_empty());
        } else {
            panic!("Expected Ns expression");
        }
    }

    #[test]
    fn test_parse_ns_with_rust_import() {
        let exprs = parse_str("(ns my.app.gui (:rust [egui-hello :as gui]))").unwrap();

        if let Expr::Ns { name, rust_imports, .. } = &exprs[0] {
            assert_eq!(name, "my.app.gui");
            assert_eq!(rust_imports.len(), 1);
            assert_eq!(rust_imports[0].library, "egui-hello");
            assert_eq!(rust_imports[0].alias, Some("gui".to_string()));
        } else {
            panic!("Expected Ns expression");
        }
    }

    #[test]
    fn test_parse_ns_complete() {
        let exprs = parse_str(
            "(ns my.app.core \
             (:require [my.lib :as lib] [other.lib :refer [f1 f2]]) \
             (:rust [egui-hello :as gui]))"
        ).unwrap();

        if let Expr::Ns { name, requires, rust_imports } = &exprs[0] {
            assert_eq!(name, "my.app.core");
            assert_eq!(requires.len(), 2);
            assert_eq!(rust_imports.len(), 1);
        } else {
            panic!("Expected Ns expression");
        }
    }

    #[test]
    fn test_parse_standalone_require() {
        let exprs = parse_str("(require [my.lib :as lib])").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Require { specs } = &exprs[0] {
            assert_eq!(specs.len(), 1);
            assert_eq!(specs[0].module, "my.lib");
            assert_eq!(specs[0].alias, Some("lib".to_string()));
        } else {
            panic!("Expected Require expression");
        }
    }

    #[test]
    fn test_parse_require_multiple_specs() {
        let exprs = parse_str("(require [my.lib :as lib] [other.lib :refer [func]])").unwrap();

        if let Expr::Require { specs } = &exprs[0] {
            assert_eq!(specs.len(), 2);
            assert_eq!(specs[0].module, "my.lib");
            assert_eq!(specs[1].module, "other.lib");
            assert_eq!(specs[1].refer, vec!["func".to_string()]);
        } else {
            panic!("Expected Require expression");
        }
    }

    #[test]
    fn test_parse_require_with_both_as_and_refer() {
        let exprs = parse_str("(require [my.lib :as lib :refer [func1 func2]])").unwrap();

        if let Expr::Require { specs } = &exprs[0] {
            assert_eq!(specs.len(), 1);
            assert_eq!(specs[0].module, "my.lib");
            assert_eq!(specs[0].alias, Some("lib".to_string()));
            assert_eq!(specs[0].refer, vec!["func1".to_string(), "func2".to_string()]);
        } else {
            panic!("Expected Require expression");
        }
    }

    #[test]
    fn test_parse_fn_basic() {
        let exprs = parse_str("(fn [x] x)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Fn { params, body, .. } = &exprs[0] {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], crate::ast::Pattern::Symbol("x".to_string()));
            assert!(matches!(**body, Expr::Symbol(_)));
        } else {
            panic!("Expected Fn expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_fn_multiple_params() {
        let exprs = parse_str("(fn [x y] (+ x y))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Fn { params, .. } = &exprs[0] {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], crate::ast::Pattern::Symbol("x".to_string()));
            assert_eq!(params[1], crate::ast::Pattern::Symbol("y".to_string()));
        } else {
            panic!("Expected Fn expression");
        }
    }

    #[test]
    fn test_parse_fn_no_params() {
        let exprs = parse_str("(fn [] 42)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Fn { params, body, .. } = &exprs[0] {
            assert_eq!(params.len(), 0);
            assert!(matches!(**body, Expr::Long(_)));
        } else {
            panic!("Expected Fn expression");
        }
    }

    // TODO: Shorthand fn tests disabled - causing stack overflow/infinite loop
    // Will be re-enabled once the parsing issue is fixed

    #[test]
    fn test_parse_do_single() {
        let exprs = parse_str("(do 42)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Do { exprs } = &exprs[0] {
            assert_eq!(exprs.len(), 1);
            assert!(matches!(exprs[0], Expr::Long(42)));
        } else {
            panic!("Expected Do expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_do_multiple() {
        let exprs = parse_str("(do (def x 10) (+ x 5) x)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Do { exprs } = &exprs[0] {
            assert_eq!(exprs.len(), 3);
            assert!(matches!(exprs[0], Expr::Def { .. }));
            assert!(matches!(exprs[1], Expr::List(_)));
            assert!(matches!(exprs[2], Expr::Symbol(_)));
        } else {
            panic!("Expected Do expression");
        }
    }

    #[test]
    fn test_parse_do_nested() {
        let exprs = parse_str("(do (do 1 2) (do 3 4))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Do { exprs } = &exprs[0] {
            assert_eq!(exprs.len(), 2);
            assert!(matches!(exprs[0], Expr::Do { .. }));
            assert!(matches!(exprs[1], Expr::Do { .. }));
        } else {
            panic!("Expected Do expression");
        }
    }

    #[test]
    fn test_parse_shorthand_fn_simple() {
        // #(* % 2) => (fn [%] (* % 2))
        eprintln!("\n=== TEST: test_parse_shorthand_fn_simple ===");
        let exprs = parse_str("#(* % 2)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Fn { params, body, .. } = &exprs[0] {
            assert_eq!(params, &vec![crate::ast::Pattern::Symbol("%".to_string())]);
            // Body should be a list (* % 2)
            match &**body {
                Expr::List(elements) => {
                    assert_eq!(elements.len(), 3);
                    assert_eq!(elements[0], Expr::Symbol("*".to_string()));
                    assert_eq!(elements[1], Expr::Symbol("%".to_string()));
                    assert!(matches!(elements[2], Expr::Long(2)) || matches!(elements[2], Expr::Double(2.0)));
                }
                Expr::Call { func, args } => {
                    assert_eq!(func, "*");
                    assert_eq!(args.len(), 2);
                    assert_eq!(args[0], Expr::Symbol("%".to_string()));
                    assert!(matches!(args[1], Expr::Long(2)) || matches!(args[1], Expr::Double(2.0)));
                }
                _ => {
                    panic!("Expected list or call body");
                }
            }
        } else {
            panic!("Expected Fn expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_shorthand_fn_numbered() {
        // #(+ %1 %2) => (fn [%1 %2] (+ %1 %2))
        let exprs = parse_str("#(+ %1 %2)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Fn { params, .. } = &exprs[0] {
            assert_eq!(params, &vec![
                crate::ast::Pattern::Symbol("%1".to_string()),
                crate::ast::Pattern::Symbol("%2".to_string())
            ]);
        } else {
            panic!("Expected Fn expression");
        }
    }

    #[test]
    fn test_parse_shorthand_fn_multiple_numbered() {
        // #(+ %1 %2 %3) => (fn [%1 %2 %3] (+ %1 %2 %3))
        let exprs = parse_str("#(+ %1 %2 %3)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Fn { params, .. } = &exprs[0] {
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], crate::ast::Pattern::Symbol("%1".to_string()));
            assert_eq!(params[1], crate::ast::Pattern::Symbol("%2".to_string()));
            assert_eq!(params[2], crate::ast::Pattern::Symbol("%3".to_string()));
        } else {
            panic!("Expected Fn expression");
        }
    }

    #[test]
    fn test_parse_shorthand_fn_nested() {
        // #(map #(* % 2) %1) => (fn [%1] (map (fn [%] (* % 2)) %1))
        let exprs = parse_str("#(map #(* % 2) %1)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Fn { params, .. } = &exprs[0] {
            // Outer function has %1 parameter
            assert_eq!(params, &vec![crate::ast::Pattern::Symbol("%1".to_string())]);
        } else {
            panic!("Expected Fn expression");
        }
    }

    #[test]
    fn test_parse_multi_arity_defn() {
        // Test multi-arity function definition
        let exprs = parse_str("(defn greet ([] 0) ([x] 1) ([x y] 2))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::DefnMulti { name, arities } = &exprs[0] {
            assert_eq!(name, "greet");
            assert_eq!(arities.len(), 3);

            // Check first arity: []
            assert_eq!(arities[0].params.len(), 0);
            assert!(arities[0].rest_param.is_none());

            // Check second arity: [x]
            assert_eq!(arities[1].params.len(), 1);
            assert_eq!(arities[1].params[0], crate::ast::Pattern::Symbol("x".to_string()));
            assert!(arities[1].rest_param.is_none());

            // Check third arity: [x y]
            assert_eq!(arities[2].params.len(), 2);
            assert_eq!(arities[2].params[0], crate::ast::Pattern::Symbol("x".to_string()));
            assert_eq!(arities[2].params[1], crate::ast::Pattern::Symbol("y".to_string()));
            assert!(arities[2].rest_param.is_none());
        } else {
            panic!("Expected DefnMulti expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_multi_arity_fn() {
        // Test multi-arity anonymous function
        let exprs = parse_str("(fn ([] 0) ([x] x) ([x y] (+ x y)))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::FnMulti { arities } = &exprs[0] {
            assert_eq!(arities.len(), 3);

            // Check first arity: []
            assert_eq!(arities[0].params.len(), 0);

            // Check second arity: [x]
            assert_eq!(arities[1].params.len(), 1);
            assert_eq!(arities[1].params[0], crate::ast::Pattern::Symbol("x".to_string()));

            // Check third arity: [x y]
            assert_eq!(arities[2].params.len(), 2);
        } else {
            panic!("Expected FnMulti expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_multi_arity_with_variadic() {
        // Test multi-arity with variadic parameter
        let exprs = parse_str("(defn sum ([] 0) ([x] x) ([x y & rest] (+ x y)))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::DefnMulti { name, arities } = &exprs[0] {
            assert_eq!(name, "sum");
            assert_eq!(arities.len(), 3);

            // Third arity should have rest_param
            assert_eq!(arities[2].params.len(), 2);
            assert_eq!(arities[2].rest_param, Some("rest".to_string()));
        } else {
            panic!("Expected DefnMulti expression");
        }
    }

    #[test]
    fn test_parse_loop() {
        // Test loop expression
        let exprs = parse_str("(loop [x 0 y 10] (if (< x y) (recur (+ x 1) y) x))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Loop { bindings, body } = &exprs[0] {
            assert_eq!(bindings.len(), 2);
            assert_eq!(bindings[0].0, crate::ast::Pattern::Symbol("x".to_string()));
            assert_eq!(bindings[1].0, crate::ast::Pattern::Symbol("y".to_string()));

            // Body should be an If expression
            assert!(matches!(**body, Expr::If { .. }));
        } else {
            panic!("Expected Loop expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_recur() {
        // Test recur expression
        let exprs = parse_str("(recur (+ x 1) y)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Recur { args } = &exprs[0] {
            assert_eq!(args.len(), 2);
            // First arg should be a list (+ x 1)
            assert!(matches!(args[0], Expr::List(_)));
            // Second arg should be symbol y
            assert_eq!(args[1], Expr::Symbol("y".to_string()));
        } else {
            panic!("Expected Recur expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_loop_simple() {
        // Test simple loop
        let exprs = parse_str("(loop [i 5] i)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Loop { bindings, .. } = &exprs[0] {
            assert_eq!(bindings.len(), 1);
            assert_eq!(bindings[0].0, crate::ast::Pattern::Symbol("i".to_string()));
        } else {
            panic!("Expected Loop expression");
        }
    }

    #[test]
    fn test_parse_defmacro() {
        // Test defmacro basic syntax
        let exprs = parse_str("(defmacro when [test & body] `(if ~test (do ~@body) nil))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Defmacro { name, params, rest_param, body } = &exprs[0] {
            assert_eq!(name, "when");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], "test");
            assert_eq!(rest_param, &Some("body".to_string()));
            // Body should be a syntax-quoted expression
            assert!(matches!(**body, Expr::SyntaxQuote { .. }));
        } else {
            panic!("Expected Defmacro expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_defmacro_no_rest() {
        // Test defmacro without rest param
        let exprs = parse_str("(defmacro unless [test then] `(if (not ~test) ~then nil))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Defmacro { name, params, rest_param, .. } = &exprs[0] {
            assert_eq!(name, "unless");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], "test");
            assert_eq!(params[1], "then");
            assert_eq!(rest_param, &None);
        } else {
            panic!("Expected Defmacro expression");
        }
    }

    #[test]
    fn test_parse_try_catch() {
        // Test try/catch parsing
        let exprs = parse_str("(try (div 10 0) (catch Exception e (println e)))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Try { body, catch_clauses, finally_block } = &exprs[0] {
            // Body should be (div 10 0)
            assert!(matches!(**body, Expr::Call { .. }));

            // Should have 1 catch clause
            assert_eq!(catch_clauses.len(), 1);
            assert_eq!(catch_clauses[0].exception_type, Some("Exception".to_string()));
            assert_eq!(catch_clauses[0].binding, "e");

            // No finally
            assert!(finally_block.is_none());
        } else {
            panic!("Expected Try expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_try_finally() {
        // Test try with finally
        let exprs = parse_str("(try (open-file) (finally (close-file)))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Try { catch_clauses, finally_block, .. } = &exprs[0] {
            // No catch clauses
            assert_eq!(catch_clauses.len(), 0);

            // Should have finally
            assert!(finally_block.is_some());
        } else {
            panic!("Expected Try expression");
        }
    }

    #[test]
    fn test_parse_try_catch_finally() {
        // Test try with both catch and finally
        let exprs = parse_str(r#"
            (try
              (risky-operation)
              (catch Error e (log e))
              (finally (cleanup)))
        "#).unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Try { catch_clauses, finally_block, .. } = &exprs[0] {
            assert_eq!(catch_clauses.len(), 1);
            assert!(finally_block.is_some());
        } else {
            panic!("Expected Try expression");
        }
    }

    #[test]
    fn test_parse_throw() {
        // Test throw parsing
        let exprs = parse_str("(throw (new Error \"Something went wrong\"))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Throw { expr } = &exprs[0] {
            // Expression should be a function call
            assert!(matches!(**expr, Expr::Call { .. }));
        } else {
            panic!("Expected Throw expression, got: {:?}", exprs[0]);
        }
    }

    #[test]
    fn test_parse_vector_pattern_simple() {
        // Test simple vector destructuring: (let [[a b c] [1 2 3]] (+ a b c))
        use crate::ast::Pattern;

        let exprs = parse_str("(let [[a b c] [1 2 3]] (+ a b c))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Let { bindings, .. } = &exprs[0] {
            assert_eq!(bindings.len(), 1);

            // Check pattern is a Vector with 3 Symbol elements
            if let Pattern::Vector { elements, rest, .. } = &bindings[0].0 {
                assert_eq!(elements.len(), 3);
                assert_eq!(elements[0], Pattern::Symbol("a".to_string()));
                assert_eq!(elements[1], Pattern::Symbol("b".to_string()));
                assert_eq!(elements[2], Pattern::Symbol("c".to_string()));
                assert_eq!(rest, &None);
            } else {
                panic!("Expected Vector pattern");
            }

            // Check value is a Vector with 3 numbers
            assert!(matches!(*bindings[0].1, Expr::Vector(_)));
        } else {
            panic!("Expected Let expression");
        }
    }

    #[test]
    fn test_parse_vector_pattern_rest() {
        // Test vector destructuring with rest: (let [[first & rest] numbers] first)
        use crate::ast::Pattern;

        let exprs = parse_str("(let [[first & rest] numbers] first)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Let { bindings, .. } = &exprs[0] {
            assert_eq!(bindings.len(), 1);

            // Check pattern has rest parameter
            if let Pattern::Vector { elements, rest, .. } = &bindings[0].0 {
                assert_eq!(elements.len(), 1);
                assert_eq!(elements[0], Pattern::Symbol("first".to_string()));
                assert_eq!(rest, &Some("rest".to_string()));
            } else {
                panic!("Expected Vector pattern with rest");
            }
        } else {
            panic!("Expected Let expression");
        }
    }

    #[test]
    fn test_parse_vector_pattern_nested() {
        // Test nested vector destructuring: (let [[a [b c]] nested] a)
        use crate::ast::Pattern;

        let exprs = parse_str("(let [[a [b c]] nested] a)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Let { bindings, .. } = &exprs[0] {
            assert_eq!(bindings.len(), 1);

            // Check outer pattern
            if let Pattern::Vector { elements, .. } = &bindings[0].0 {
                assert_eq!(elements.len(), 2);
                assert_eq!(elements[0], Pattern::Symbol("a".to_string()));

                // Check nested pattern
                if let Pattern::Vector { elements: inner_elements, .. } = &elements[1] {
                    assert_eq!(inner_elements.len(), 2);
                    assert_eq!(inner_elements[0], Pattern::Symbol("b".to_string()));
                    assert_eq!(inner_elements[1], Pattern::Symbol("c".to_string()));
                } else {
                    panic!("Expected nested Vector pattern");
                }
            } else {
                panic!("Expected Vector pattern");
            }
        } else {
            panic!("Expected Let expression");
        }
    }

    #[test]
    fn test_parse_map_pattern_keys() {
        // Test map destructuring with :keys: (let [{:keys [x y]} point] x)
        use crate::ast::{MapPatternKey, Pattern};

        let exprs = parse_str("(let [{:keys [x y]} point] x)").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Let { bindings, .. } = &exprs[0] {
            assert_eq!(bindings.len(), 1);

            // Check pattern is a Map with :keys bindings
            if let Pattern::Map { bindings: map_bindings, .. } = &bindings[0].0 {
                assert_eq!(map_bindings.len(), 2);

                // Check first binding
                assert_eq!(map_bindings[0].0, MapPatternKey::Symbol("x".to_string()));
                assert_eq!(map_bindings[0].1, Pattern::Symbol("x".to_string()));

                // Check second binding
                assert_eq!(map_bindings[1].0, MapPatternKey::Symbol("y".to_string()));
                assert_eq!(map_bindings[1].1, Pattern::Symbol("y".to_string()));
            } else {
                panic!("Expected Map pattern");
            }
        } else {
            panic!("Expected Let expression");
        }
    }

    #[test]
    fn test_parse_ignore_pattern() {
        // Test ignore pattern: (let [[a _ c] [1 2 3]] (+ a c))
        use crate::ast::Pattern;

        let exprs = parse_str("(let [[a _ c] [1 2 3]] (+ a c))").unwrap();
        assert_eq!(exprs.len(), 1);

        if let Expr::Let { bindings, .. } = &exprs[0] {
            assert_eq!(bindings.len(), 1);

            // Check pattern has ignore in the middle
            if let Pattern::Vector { elements, .. } = &bindings[0].0 {
                assert_eq!(elements.len(), 3);
                assert_eq!(elements[0], Pattern::Symbol("a".to_string()));
                assert_eq!(elements[1], Pattern::Ignore);
                assert_eq!(elements[2], Pattern::Symbol("c".to_string()));
            } else {
                panic!("Expected Vector pattern");
            }
        } else {
            panic!("Expected Let expression");
        }
    }
}
