/// Beautiful error reporting for Clorus
///
/// Provides:
/// - Colorful terminal output
/// - Source code context with line/column positions
/// - Helpful error messages with suggestions
/// - Different error categories (Syntax, Compile, Runtime)

use std::fmt;

/// ANSI color codes for terminal output
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const CYAN: &str = "\x1b[36m";
    pub const GRAY: &str = "\x1b[90m";
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
}

/// Source location (line and column)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub position: usize,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize, position: usize) -> Self {
        Self { line, column, position }
    }

    pub fn unknown() -> Self {
        Self { line: 0, column: 0, position: 0 }
    }
}

/// Error category
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax,      // Parse errors
    Compile,     // Codegen errors
    Runtime,     // Execution errors
    Type,        // Type errors (future)
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Syntax => write!(f, "Syntax Error"),
            ErrorKind::Compile => write!(f, "Compile Error"),
            ErrorKind::Runtime => write!(f, "Runtime Error"),
            ErrorKind::Type => write!(f, "Type Error"),
        }
    }
}

/// A Clorus error with rich context
pub struct ClorusError {
    pub kind: ErrorKind,
    pub message: String,
    pub location: SourceLocation,
    pub source_file: Option<String>,
    pub source_code: Option<String>,
    pub suggestion: Option<String>,
}

impl ClorusError {
    pub fn new(kind: ErrorKind, message: String) -> Self {
        Self {
            kind,
            message,
            location: SourceLocation::unknown(),
            source_file: None,
            source_code: None,
            suggestion: None,
        }
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = location;
        self
    }

    pub fn with_source(mut self, source_code: String, source_file: Option<String>) -> Self {
        self.source_code = Some(source_code);
        self.source_file = source_file;
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Format the error beautifully for terminal output
    pub fn format_error(&self, use_color: bool) -> String {
        let mut output = String::new();

        // Header: error kind
        if use_color {
            output.push_str(&format!(
                "{}{}{}error{}{}: {}{}\n",
                colors::BOLD,
                colors::BRIGHT_RED,
                "",
                colors::RESET,
                colors::BOLD,
                self.kind,
                colors::RESET
            ));
        } else {
            output.push_str(&format!("error: {}\n", self.kind));
        }

        // Location info
        if self.location.line > 0 {
            let location_str = if let Some(ref file) = self.source_file {
                format!("{}:{}:{}", file, self.location.line, self.location.column)
            } else {
                format!("line {}:{}", self.location.line, self.location.column)
            };

            if use_color {
                output.push_str(&format!(
                    "  {}-->{}  {}\n",
                    colors::BLUE,
                    colors::RESET,
                    location_str
                ));
            } else {
                output.push_str(&format!("  --> {}\n", location_str));
            }
        }

        // Message
        if use_color {
            output.push_str(&format!(
                "   {}{}{}\n",
                colors::BOLD,
                self.message,
                colors::RESET
            ));
        } else {
            output.push_str(&format!("   {}\n", self.message));
        }

        // Source code context
        if let Some(ref source) = self.source_code {
            output.push('\n');
            output.push_str(&self.format_source_context(source, use_color));
        }

        // Suggestion
        if let Some(ref suggestion) = self.suggestion {
            output.push('\n');
            if use_color {
                output.push_str(&format!(
                    "   {}{}help:{}{} {}\n",
                    colors::BOLD,
                    colors::CYAN,
                    colors::RESET,
                    colors::BOLD,
                    suggestion
                ));
                output.push_str(colors::RESET);
            } else {
                output.push_str(&format!("   help: {}\n", suggestion));
            }
        }

        output
    }

    /// Format source code context with line numbers and highlighting
    fn format_source_context(&self, source: &str, use_color: bool) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let mut output = String::new();

        if self.location.line == 0 || self.location.line > lines.len() {
            return output;
        }

        let line_idx = self.location.line - 1;
        let context_before = 2;
        let context_after = 1;

        let start = line_idx.saturating_sub(context_before);
        let end = (line_idx + context_after + 1).min(lines.len());

        // Calculate width for line numbers
        let max_line_num = end;
        let line_num_width = format!("{}", max_line_num).len();

        for (idx, line) in lines[start..end].iter().enumerate() {
            let line_num = start + idx + 1;
            let is_error_line = line_num == self.location.line;

            if use_color {
                if is_error_line {
                    // Error line
                    output.push_str(&format!(
                        " {}{:width$}{} {} {}\n",
                        colors::BLUE,
                        line_num,
                        colors::RESET,
                        colors::BLUE,
                        line,
                        width = line_num_width
                    ));

                    // Underline the error position
                    output.push_str(&format!(
                        " {:width$}   {}{}^{} {}\n",
                        "",
                        colors::BRIGHT_RED,
                        colors::BOLD,
                        "~".repeat(5),
                        colors::RESET,
                        width = line_num_width
                    ));
                } else {
                    // Context line
                    output.push_str(&format!(
                        " {}{:width$}{} {} {}\n",
                        colors::GRAY,
                        line_num,
                        colors::RESET,
                        colors::GRAY,
                        line,
                        width = line_num_width
                    ));
                }
            } else {
                output.push_str(&format!(
                    " {:width$} | {}\n",
                    line_num,
                    line,
                    width = line_num_width
                ));

                if is_error_line {
                    output.push_str(&format!(
                        " {:width$} | {}^\n",
                        "",
                        " ".repeat(self.location.column.saturating_sub(1)),
                        width = line_num_width
                    ));
                }
            }
        }

        output
    }
}

impl fmt::Display for ClorusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_error(true))
    }
}

impl fmt::Debug for ClorusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_error(false))
    }
}

/// Helper functions for creating common errors
pub fn syntax_error(message: impl Into<String>) -> ClorusError {
    ClorusError::new(ErrorKind::Syntax, message.into())
}

pub fn compile_error(message: impl Into<String>) -> ClorusError {
    ClorusError::new(ErrorKind::Compile, message.into())
}

pub fn runtime_error(message: impl Into<String>) -> ClorusError {
    ClorusError::new(ErrorKind::Runtime, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_formatting() {
        let source = r#"(defn hello [name)
  (println "Hello" name))

(hello "World")"#;

        let error = syntax_error("Unclosed parameter vector")
            .with_location(SourceLocation::new(1, 17, 16))
            .with_source(source.to_string(), Some("test.clrs".to_string()))
            .with_suggestion("Add a closing bracket ']' to complete the parameter list");

        println!("{}", error);
    }

    #[test]
    fn test_compile_error() {
        let source = r#"(def x 10)
(+ x y z)"#;

        let error = compile_error("Undefined variable: y")
            .with_location(SourceLocation::new(2, 6, 15))
            .with_source(source.to_string(), Some("example.clrs".to_string()))
            .with_suggestion("Did you forget to define 'y'? Use (def y ...) or (let [y ...] ...)");

        println!("{}", error);
    }
}
