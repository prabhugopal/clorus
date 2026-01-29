/// Interface file (.clorus-ffi) parser
use clorus_syntax::{Expr, parse};
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
}

/// Represents a parameter in a function declaration
#[derive(Debug, Clone)]
pub struct InterfaceParam {
    pub name: String,
    pub type_name: String,
}

/// Parse a .clorus-ffi interface file
pub fn parse_interface_file(path: &Path) -> Result<InterfaceFile, String> {

    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read interface file {}: {}", path.display(), e))?;


    let exprs = match parse(&source) {
        Ok(exprs) => exprs,
        Err(e) => {
            return Err(format!("Failed to parse interface file {}: {}", path.display(), e));
        }
    };


    // Expect (interface name ...)
    if exprs.is_empty() {
        return Err("Interface file is empty".to_string());
    }

    match &exprs[0] {
        Expr::Call { func, args } => {

            if func == "interface" {
                return parse_interface_from_call(args);
            }

            Err("Interface file must start with (interface ...)".to_string())
        }
        Expr::List(items) => {

            if items.is_empty() {
                return Err("Empty interface declaration".to_string());
            }

            // Check for (interface ...)
            if let Expr::Symbol(s) = &items[0] {
                if s == "interface" {
                    return Ok(parse_interface_declaration(&items[1..])?);
                }
            }

            Err("Interface file must start with (interface ...)".to_string())
        }
        other => {
            Err("Interface file must start with (interface ...)".to_string())
        }
    }
}

/// Parse interface from a Call expression
fn parse_interface_from_call(args: &[Expr]) -> Result<InterfaceFile, String> {
    if args.is_empty() {
        return Err("Interface declaration missing name".to_string());
    }

    // Get interface name
    let name = match &args[0] {
        Expr::Symbol(s) => s.clone(),
        _ => return Err("Interface name must be a symbol".to_string()),
    };


    // Parse function declarations from remaining args
    let mut functions = Vec::new();
    for arg in &args[1..] {
        if let Some(func) = parse_function_from_call(arg)? {
            functions.push(func);
        }
    }

    Ok(InterfaceFile { name, functions })
}

/// Parse function from a Call expression
fn parse_function_from_call(expr: &Expr) -> Result<Option<InterfaceFunction>, String> {
    match expr {
        Expr::Call { func, args } => {
            if func == "fn" || func == "defn" {
                return parse_fn(args).map(Some);
            }
            Ok(None)
        }
        _ => Ok(None)
    }
}

/// Parse the contents of an (interface name ...) declaration
fn parse_interface_declaration(items: &[Expr]) -> Result<InterfaceFile, String> {
    if items.is_empty() {
        return Err("Interface declaration missing name".to_string());
    }

    // Get interface name
    let name = match &items[0] {
        Expr::Symbol(s) => s.clone(),
        _ => return Err("Interface name must be a symbol".to_string()),
    };

    // Parse function declarations
    let mut functions = Vec::new();
    for item in &items[1..] {
        if let Some(func) = parse_function_declaration(item)? {
            functions.push(func);
        }
    }

    Ok(InterfaceFile { name, functions })
}

/// Parse a function declaration: (fn name [params...] return-type "doc")
fn parse_function_declaration(expr: &Expr) -> Result<Option<InterfaceFunction>, String> {
    match expr {
        Expr::List(items) => {
            if items.is_empty() {
                return Ok(None);
            }

            // Check for (fn ...)
            if let Expr::Symbol(s) = &items[0] {
                if s == "fn" || s == "defn" {
                    return parse_fn(&items[1..]).map(Some);
                }
            }

            // Ignore other forms (comments, etc.)
            Ok(None)
        }
        _ => Ok(None), // Ignore non-list forms
    }
}

/// Parse (fn name [params...] return-type "doc")
fn parse_fn(items: &[Expr]) -> Result<InterfaceFunction, String> {
    for (idx, item) in items.iter().enumerate() {
    }

    if items.len() < 3 {
        return Err("defn requires at least name, params, and return type".to_string());
    }

    // Get function name
    let name = match &items[0] {
        Expr::Symbol(s) => {
            s.clone()
        }
        other => {
            return Err("Function name must be a symbol".to_string());
        }
    };

    // Get parameters
    let params = match &items[1] {
        Expr::Vector(param_list) => {
            parse_params(param_list)?
        }
        other => {
            return Err("Function parameters must be a vector [...]".to_string());
        }
    };

    // Get return type
    let return_type = match &items[2] {
        Expr::Keyword(k) => {
            map_type_keyword(k)
        }
        other => {
            return Err("Return type must be a keyword like :f64 or :string".to_string());
        }
    };

    // Get optional docstring
    let doc = if items.len() > 3 {
        match &items[3] {
            Expr::String(s) => Some(s.clone()),
            _ => None,
        }
    } else {
        None
    };

    Ok(InterfaceFunction {
        name,
        params,
        return_type,
        doc,
    })
}

/// Parse function parameters: [name :type name :type ...]
fn parse_params(param_list: &[Expr]) -> Result<Vec<InterfaceParam>, String> {
    let mut params = Vec::new();
    let mut i = 0;

    for (idx, expr) in param_list.iter().enumerate() {
    }

    while i < param_list.len() {
        // Expect: name :type
        if i + 1 >= param_list.len() {
            return Err("Parameter must have both name and type".to_string());
        }

        let name = match &param_list[i] {
            Expr::Symbol(s) => s.clone(),
            other => {
                return Err("Parameter name must be a symbol".to_string());
            }
        };

        let type_name = match &param_list[i + 1] {
            Expr::Keyword(k) => map_type_keyword(k),
            other => {
                return Err("Parameter type must be a keyword like :f64 or :string".to_string());
            }
        };

        params.push(InterfaceParam { name, type_name });
        i += 2;
    }

    Ok(params)
}

/// Map Clorus type keywords to Rust type strings
fn map_type_keyword(keyword: &str) -> String {
    match keyword {
        "f64" => "f64".to_string(),
        "i32" => "i32".to_string(),
        "i64" => "i64".to_string(),
        "bool" => "bool".to_string(),
        "string" => "String".to_string(),
        "unit" => "()".to_string(),
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
    }
}
