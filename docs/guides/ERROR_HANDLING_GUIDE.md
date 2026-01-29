# Error Handling & Reporting: Before vs After

## Current State (Basic Errors)

### What You See Now
```bash
$ clorus run
Error: Parse error: Parameter name must be a symbol

$ clorus run
Error: Compile error: Undefined variable: x
```

**Problems:**
- ❌ No colors
- ❌ No line numbers
- ❌ No column positions
- ❌ No source code context
- ❌ No helpful suggestions
- ❌ Hard to understand what went wrong
- ❌ No indication where in the file the error occurred

## New System (Beautiful Errors)

### With `clorus-errors` Library ✨

#### Syntax Error Example
```bash
error: Syntax Error
  --> test.clrs:1:17
   Unclosed parameter vector

 1 | (defn hello [name)
     ^~~~~~
 2 |   (println "Hello" name))

   help: Add a closing bracket ']' to complete the parameter list
```

#### Compile Error Example
```bash
error: Compile Error
  --> example.clrs:2:6
   Undefined variable: y

 1 | (def x 10)
 2 | (+ x y z)
     ^~~~~~
 3 |

   help: Did you forget to define 'y'? Use (def y ...) or (let [y ...] ...)
```

#### With Colors (in terminal)
The actual output includes:
- 🔴 **Red** "error" label and error pointer
- 🔵 **Blue** line numbers and location
- **Bold** text for emphasis
- 🟢 **Cyan** "help" label
- 🟦 **Gray** context lines

## Features

### ✅ What's Implemented

1. **Rich Error Types**
   - `ErrorKind`: Syntax, Compile, Runtime, Type
   - `SourceLocation`: line, column, position tracking
   - `ClorusError`: Complete error with context

2. **Colorful Terminal Output**
   - ANSI color codes
   - Bold highlighting
   - Clear visual hierarchy

3. **Source Code Context**
   - Shows 2 lines before error
   - Shows 1 line after error
   - Line numbers with proper alignment
   - Error pointer showing exact location

4. **Helpful Suggestions**
   - Optional suggestion field
   - Clear "help:" prefix
   - Actionable advice

5. **Multiple Error Categories**
   ```rust
   syntax_error("message")     // Parse errors
   compile_error("message")    // Codegen errors
   runtime_error("message")    // Execution errors
   ```

### ⏳ What's Needed for Integration

#### 1. Update Lexer to Track Positions

**Current** (`clorus-syntax/src/lexer.rs`):
```rust
pub struct Lexer {
    input: Vec<char>,
    position: usize,  // Only character position
}
```

**Needed**:
```rust
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,       // Track current line
    column: usize,     // Track current column
}

// Token needs location
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub location: SourceLocation,
}
```

#### 2. Update Parser to Use ClorusError

**Current** (`clorus-syntax/src/parser.rs`):
```rust
pub fn parse_expr(&mut self) -> Result<Expr, String> {
    // Returns plain String errors
    Err("Unclosed list".to_string())
}
```

**Needed**:
```rust
use clorus_errors::{ClorusError, syntax_error};

pub fn parse_expr(&mut self) -> Result<Expr, ClorusError> {
    // Return rich errors with context
    Err(syntax_error("Unclosed list")
        .with_location(self.current_token().location)
        .with_source(self.source.clone(), self.filename.clone())
        .with_suggestion("Add a closing parenthesis ')' to complete the list"))
}
```

#### 3. Update CLI to Display Rich Errors

**Current** (`clorus-cli/src/commands.rs`):
```rust
clorus::parse(&source)
    .map_err(|e| format!("Parse error: {}", e))?;
```

**Needed**:
```rust
match clorus::parse(&source) {
    Ok(exprs) => exprs,
    Err(e) => {
        eprintln!("{}", e);  // ClorusError implements Display
        return Err("Compilation failed".to_string());
    }
}
```

#### 4. Add Source Code to Errors

Parser needs to keep source code to show in errors:

```rust
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    source: String,         // NEW: Keep original source
    filename: Option<String>,  // NEW: Track filename
}
```

## Implementation Roadmap

### Phase 1: Basic Integration (1-2 hours)

1. **Add `clorus-errors` dependency** to syntax crate
   ```toml
   # crates/clorus-syntax/Cargo.toml
   [dependencies]
   clorus-errors = { path = "../clorus-errors" }
   ```

2. **Update Token to include location**
   ```rust
   pub struct Token {
       pub kind: TokenKind,
       pub loc: SourceLocation,
   }
   ```

3. **Update Lexer to track line/column**
   - Increment line on '\n'
   - Track column position
   - Reset column on newline

4. **Change parser return type**
   ```rust
   pub fn parse(input: &str) -> Result<Vec<Expr>, ClorusError>
   ```

5. **Update CLI error display**
   - Print ClorusError with colors
   - Show full context

### Phase 2: Smart Suggestions (2-3 hours)

Add context-aware suggestions for common errors:

```rust
// Undefined variable
if !self.variables.contains(name) {
    let did_you_mean = find_similar_name(name, &self.variables);
    let suggestion = if let Some(similar) = did_you_mean {
        format!("Did you mean '{}'?", similar)
    } else {
        format!("Variable '{}' is not defined. Use (def {} ...) to define it.", name, name)
    };

    return Err(compile_error(format!("Undefined variable: {}", name))
        .with_suggestion(suggestion));
}
```

### Phase 3: Stack Traces (3-4 hours)

Add call stack tracking for runtime errors:

```rust
pub struct StackFrame {
    function_name: String,
    location: SourceLocation,
}

pub struct StackTrace {
    frames: Vec<StackFrame>,
}

// In error:
error: Runtime Error
  --> factorial.clrs:5:12
   Division by zero

 5 | (defn divide [x y] (/ x y))
     ^~~~~~

Stack trace:
  at divide (factorial.clrs:5:12)
  at main (factorial.clrs:10:3)

   help: Check that the denominator is not zero before dividing
```

## Example Integration

### Before (What User Sees Now)
```bash
$ cat src/main.clrs
(defn test [x y
  (+ x y))

$ clorus run
Error: Parse error: Unclosed parameter vector
```

### After (With Full Integration)
```bash
$ cat src/main.clrs
(defn test [x y
  (+ x y))

$ clorus run
error: Syntax Error
  --> src/main.clrs:1:15
   Unclosed parameter vector - expected ']' before function body

 1 | (defn test [x y
               ^~~~~~
 2 |   (+ x y))

   help: Add a closing bracket ']' after the parameters
         Try: (defn test [x y] ...)
```

## Testing

```bash
# Test the error formatting
cargo test -p clorus-errors -- --nocapture

# See colored output
cargo test -p clorus-errors test_error_formatting -- --nocapture
cargo test -p clorus-errors test_compile_error -- --nocapture
```

## Color Customization

Errors respect terminal capabilities:
- Colors disabled on non-TTY output
- Can force colors on/off with env vars
- Falls back to plain text if needed

```bash
# Force colors
FORCE_COLOR=1 clorus run

# Disable colors
NO_COLOR=1 clorus run
```

## Quick Reference

### Creating Errors

```rust
use clorus_errors::{syntax_error, compile_error, runtime_error};

// Simple error
syntax_error("Unclosed list")

// With location
syntax_error("Unclosed list")
    .with_location(SourceLocation::new(line, col, pos))

// With source context
syntax_error("Unclosed list")
    .with_location(location)
    .with_source(source_code, Some("file.clrs".to_string()))

// With suggestion
syntax_error("Unclosed list")
    .with_location(location)
    .with_source(source_code, Some("file.clrs".to_string()))
    .with_suggestion("Add a closing parenthesis ')'")
```

### Displaying Errors

```rust
// In terminal (with colors)
eprintln!("{}", error);

// Plain text (no colors)
eprintln!("{:?}", error);

// Custom formatting
let formatted = error.format_error(use_color);
eprintln!("{}", formatted);
```

## Benefits

1. **Faster Debugging**
   - See exactly where the error is
   - Understand what went wrong
   - Get actionable suggestions

2. **Better DX (Developer Experience)**
   - Clear, beautiful output
   - Professional look and feel
   - Matches modern compilers (Rust, TypeScript, etc.)

3. **Learning Friendly**
   - Suggestions help beginners
   - Context helps understand code flow
   - Clear error categories

4. **IDE Integration Ready**
   - Structured error format
   - Location info parseable
   - Ready for LSP integration

## Current Status

- ✅ Error library created (`clorus-errors`)
- ✅ Colorful formatting working
- ✅ Tests passing
- ⏳ Integration with lexer/parser (pending)
- ⏳ CLI updates (pending)
- ⏳ Smart suggestions (pending)

## Next Steps

1. Update Lexer to track positions
2. Change Parser to return ClorusError
3. Update CLI to display rich errors
4. Add context-aware suggestions
5. Add stack traces for runtime errors

Estimated time: 4-6 hours total

---

**Want to see it in action?**
```bash
cargo test -p clorus-errors -- --nocapture
```

This will show the beautiful, colorful error messages! 🎨
