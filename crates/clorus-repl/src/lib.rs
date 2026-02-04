mod repl_engine;
mod project;

use repl_engine::ReplEngine;
use project::ProjectConfig;
use inkwell::context::Context;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{Editor, Helper};
use rustyline::completion::Pair;
use rustyline::hint::HistoryHinter;
use std::path::Path;
use std::io::{self, BufRead, IsTerminal};

// Import Value FFI functions from runtime
use clorus_runtime::value::{clorus_value_as_long, clorus_value_as_double, clorus_value_as_bool, clorus_value_as_cstring, clorus_free_cstring, Value, ValueTag};
use clorus_runtime::string::clorus_pr_str;
use clorus_runtime::value::clorus_release;

// Force inclusion of core FFI symbols
// This prevents the linker from stripping them, making them visible to LLVM JIT
#[used]
static FORCE_LINK_NIL: unsafe extern "C" fn() -> *mut Value = clorus_runtime::value::clorus_value_nil;
#[used]
static FORCE_LINK_BOOL: unsafe extern "C" fn(f64) -> *mut Value = clorus_runtime::value::clorus_value_bool;

// Force-link atom operations
#[used]
static FORCE_LINK_ATOM_CREATE: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::atom::clorus_atom;
#[used]
static FORCE_LINK_ATOM_DEREF: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::atom::clorus_deref;
#[used]
static FORCE_LINK_ATOM_RESET: unsafe extern "C" fn(*mut Value, *mut Value) -> *mut Value = clorus_runtime::atom::clorus_reset;
#[used]
static FORCE_LINK_ATOM_SWAP: unsafe extern "C" fn(*mut Value, *mut u8, *mut Value) -> *mut Value = clorus_runtime::atom::clorus_swap;

#[used]
static FORCE_LINK_AGENT: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::agent::clorus_agent;
#[used]
static FORCE_LINK_CHAN: unsafe extern "C" fn(i64) -> *mut Value = clorus_runtime::channel::clorus_chan;
#[used]
static FORCE_LINK_CHAN_PUT: unsafe extern "C" fn(*mut Value, *mut Value) -> *mut Value = clorus_runtime::channel::clorus_chan_put;
#[used]
static FORCE_LINK_CHAN_TAKE: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::channel::clorus_chan_take;
#[used]
static FORCE_LINK_CHAN_CLOSE: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::channel::clorus_chan_close;
#[used]
static FORCE_LINK_ALTS: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::channel::clorus_alts;
#[used]
static FORCE_LINK_GO: unsafe extern "C" fn(*mut Value, *mut Value) -> *mut Value = clorus_runtime::go_block::clorus_go;

// Force-link I/O operations
#[used]
static FORCE_LINK_PRINT: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::io::clorus_print;
#[used]
static FORCE_LINK_PRINTLN: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::io::clorus_println;
#[used]
static FORCE_LINK_PRN: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::io::clorus_prn;

/// Display a Value* for the REPL
/// Properly handles all value types: numbers, strings, booleans, nil, etc.
fn display_value(value_ptr: *mut u8) -> String {
    if value_ptr.is_null() {
        return "nil".to_string();
    }

    unsafe {
        // Cast u8* to Value*
        let value = value_ptr as *mut Value;

        // Check the value tag to determine the actual type
        match (*value).header().tag() {
            ValueTag::Long => {
                let num = clorus_value_as_long(value);
                format!("{}", num)
            }
            ValueTag::Double => {
                let num = clorus_value_as_double(value);
                format!("{}", num)
            }
            ValueTag::String => {
                let c_str = clorus_value_as_cstring(value);
                if !c_str.is_null() {
                    let rust_str = std::ffi::CStr::from_ptr(c_str);
                    let result = format!("\"{}\"", rust_str.to_string_lossy());
                    clorus_free_cstring(c_str);
                    result
                } else {
                    "<null string>".to_string()
                }
            }
            ValueTag::Bool => {
                let bool_val = clorus_value_as_bool(value);
                if bool_val { "true" } else { "false" }.to_string()
            }
            ValueTag::Nil => {
                "nil".to_string()
            }
            ValueTag::Vector | ValueTag::List | ValueTag::HashMap | ValueTag::HashSet => {
                // Use clorus_pr_str for proper formatting of collections
                let pr_str_result = clorus_pr_str(value);
                if !pr_str_result.is_null() {
                    let result_str = display_value(pr_str_result as *mut u8);
                    // Release the string value created by pr_str
                    clorus_release(pr_str_result);
                    // Remove surrounding quotes since pr_str returns a string
                    if result_str.starts_with('"') && result_str.ends_with('"') {
                        result_str[1..result_str.len()-1].to_string()
                    } else {
                        result_str
                    }
                } else {
                    format!("{:?}", *value)
                }
            }
            _ => {
                // For other types, show debug representation
                format!("{:?}", *value)
            }
        }
    }
}


/// Format the result of a REPL evaluation
/// Clojure-style output: #'namespace/name for def/defn, => value for expressions
fn format_result(result: &repl_engine::EvalResult, namespace: &str) -> String {
    use repl_engine::EvalKind;

    match &result.kind {
        EvalKind::Def(name) | EvalKind::Defn(name) => {
            format!("#'{}/{}", namespace, name)
        }
        EvalKind::Namespace | EvalKind::Import => {
            "nil".to_string()
        }
        EvalKind::Value => {
            format!("{}", display_value(result.value))
        }
    }
}


/// Autocomplete helper for Clorus REPL
struct ClorusHelper {
    completions: Vec<String>,
    hinter: HistoryHinter,
}

impl ClorusHelper {
    fn new() -> Self {
        ClorusHelper {
            completions: vec![
                // Arithmetic operators
                "+".to_string(),
                "-".to_string(),
                "*".to_string(),
                "/".to_string(),
                // Comparison operators
                "<".to_string(),
                ">".to_string(),
                "=".to_string(),
                // Special forms
                "def".to_string(),
                "defn".to_string(),
                "let".to_string(),
                "if".to_string(),
                "use".to_string(),
                "ns".to_string(),
                "require".to_string(),
                // Namespace keywords
                ":as".to_string(),
                ":refer".to_string(),
                ":all".to_string(),
                ":rust".to_string(),
                ":require".to_string(),
                // rust.fs module functions
                "fs/read".to_string(),
                "fs/write".to_string(),
                "fs/append".to_string(),
                "fs/exists?".to_string(),
                "fs/is-file?".to_string(),
                "fs/is-dir?".to_string(),
                "fs/remove".to_string(),
                "fs/copy".to_string(),
                "fs/rename".to_string(),
                "fs/create-dir".to_string(),
                "fs/create-dir-all".to_string(),
                // clorus.core module functions
                "slurp".to_string(),
                "spit".to_string(),
                // Module names
                "rust.fs".to_string(),
                "clorus.core".to_string(),
            ],
            hinter: HistoryHinter::new(),
        }
    }
}

impl rustyline::completion::Completer for ClorusHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Find the word being completed
        let start = line[..pos]
            .rfind(|c: char| c.is_whitespace() || c == '(' || c == '[')
            .map(|i| i + 1)
            .unwrap_or(0);

        let word = &line[start..pos];

        // Filter completions that match
        let matches: Vec<Pair> = self.completions
            .iter()
            .filter(|s| s.starts_with(word))
            .map(|s| Pair {
                display: s.clone(),
                replacement: s.clone(),
            })
            .collect();

        Ok((start, matches))
    }
}

impl rustyline::hint::Hinter for ClorusHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl rustyline::highlight::Highlighter for ClorusHelper {}

impl rustyline::validate::Validator for ClorusHelper {}

impl Helper for ClorusHelper {}

/// Configuration options for the REPL
#[derive(Debug, Clone)]
pub struct ReplConfig {
    /// Path to history file (default: ~/.clorus_history)
    pub history_file: Option<String>,

    /// Don't print the banner
    pub no_banner: bool,

    /// Don't load stdlib automatically
    pub no_stdlib: bool,

    /// Custom stdlib path
    pub stdlib_path: Option<String>,

    /// Don't auto-load project entry file (more Clojure-like)
    pub no_auto_load: bool,

    /// Run on main thread (required for GUI on macOS)
    pub main_thread: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            history_file: None,
            no_banner: false,
            no_stdlib: false,
            stdlib_path: None,
            no_auto_load: false,  // Auto-load by default for convenience
            main_thread: false,    // Default: run on background threads (current behavior)
        }
    }
}

/// Main entry point for the REPL with default configuration
pub fn run() -> Result<(), String> {
    run_with_config(ReplConfig::default())
}

/// Main entry point for the REPL with custom configuration
pub fn run_with_config(config: ReplConfig) -> Result<(), String> {
    run_repl_impl(config)
}

// Internal implementation - the actual REPL logic
fn run_repl_impl(config: ReplConfig) -> Result<(), String> {
    // Load project configuration if available
    let project = ProjectConfig::load();

    // Print banner unless disabled
    if !config.no_banner {
        println!("╔════════════════════════════════════╗");
        println!("║  Clorus REPL v0.2.0                ║");
        println!("║  Clojure-inspired systems language ║");
        println!("╚════════════════════════════════════╝");
        println!();

        // Display project info if in a project directory
        if let Some(ref proj_config) = project {
            println!("📦 Project: {} v{}", proj_config.package.name, proj_config.package.version);
            println!("📂 Namespace: {}", proj_config.namespace());
            println!();
        }

        println!("Type expressions to evaluate them.");
        println!("Commands: :examples :help :quit");
        println!("Tip: Use TAB for autocomplete, ↑↓ for history");

        // Show main-thread mode indicator
        if config.main_thread {
            println!();
            println!("🔧 Main-thread mode: GUI functions available (macOS-safe)");
        }

        println!();
    }

    // Load clorus-runtime library FIRST (required for Value* operations)
    let _runtime_lib = match load_runtime_library() {
        Ok(_lib) => {
            // Keep library loaded for the duration of REPL
        }
        Err(e) => {
            eprintln!("⚠ Warning: Could not load runtime library: {}", e);
            eprintln!("  The REPL may not function correctly.");
        }
    };

    // NOTE: clorus-std and clorus-core .dylib libraries cause hanging when loaded.
    // This is separate from the GUI threading issue. Need to investigate why
    // libloading blocks on macOS. For now, stdlib is loaded from source (.clr).
    println!();

    let context = Context::create();
    let mut repl_engine = ReplEngine::new(&context);

    // Auto-load core stdlib (clorus.core)
    // Skip if no_stdlib is set
    let mut core_loaded = false;
    let mut core_stdlib_path = None;

    if !config.no_stdlib {
        // Use custom stdlib path if provided
        if let Some(ref custom_path) = config.stdlib_path {
            let path = std::path::Path::new(custom_path);
            if path.exists() {
                core_stdlib_path = Some(path.to_path_buf());
            }
        }

        // Try to find core.clr in CLORUS_HOME/stdlib or ~/.clorus/stdlib
        if core_stdlib_path.is_none() {
            if let Ok(clorus_home) = std::env::var("CLORUS_HOME") {
                let path = std::path::Path::new(&clorus_home).join("stdlib").join("core.clr");
                if path.exists() {
                    core_stdlib_path = Some(path);
                }
            }
        }

        if core_stdlib_path.is_none() {
            if let Ok(home) = std::env::var("HOME") {
                let path = std::path::Path::new(&home).join(".clorus").join("stdlib").join("core.clr");
                if path.exists() {
                    core_stdlib_path = Some(path);
                }
            }
        }
    }

    if let Some(core_path) = core_stdlib_path {
        match std::fs::read_to_string(&core_path) {
            Ok(source) => {
                // Use new batch loader - compiles stdlib ONCE, never recompiled
                match repl_engine.load_stdlib_batch(source) {
                    Ok(count) => {
                        println!("✓ Loaded clorus.core ({} functions)", count);
                        core_loaded = true;
                    }
                    Err(e) => {
                        eprintln!("⚠ Error loading stdlib: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠ Could not read stdlib: {}", e);
            }
        }
    }

    if !core_loaded {
        println!("⚠ clorus.core not loaded (stdlib not found)");
    }
    println!();

    // Load project Rust FFI libraries if in a project directory
    let mut _rust_ffi_libs = Vec::new();
    if project.is_some() {
        if let Ok(manifest_str) = std::fs::read_to_string("Clorus.toml") {
            if let Ok(manifest) = toml::from_str::<toml::Value>(&manifest_str) {
                if let Some(rust_deps) = manifest.get("rust-dependencies").and_then(|v| v.as_table()) {
                    println!("Loading Rust FFI libraries...");

                    for (dep_name, _dep_info) in rust_deps {
                        // Convert dep name to library name (e.g., egui-hello -> libegui_hello_ffi.dylib)
                        let lib_name_base = dep_name.replace('-', "_");

                        #[cfg(target_os = "macos")]
                        let lib_name = format!("lib{}_ffi.dylib", lib_name_base);

                        #[cfg(target_os = "linux")]
                        let lib_name = format!("lib{}_ffi.so", lib_name_base);

                        #[cfg(target_os = "windows")]
                        let lib_name = format!("{}_ffi.dll", lib_name_base);

                        // Try target/debug, target/release, and target/rust-ffi/{name}_ffi/target/{debug,release}
                        let lib_paths = vec![
                            Path::new("target/debug").join(&lib_name),
                            Path::new("target/release").join(&lib_name),
                            Path::new(&format!("target/rust-ffi/{}_ffi/target/release", lib_name_base)).join(&lib_name),
                            Path::new(&format!("target/rust-ffi/{}_ffi/target/debug", lib_name_base)).join(&lib_name),
                        ];

                        let mut loaded = false;
                        for lib_path in &lib_paths {
                            if lib_path.exists() {
                                match load_dynamic_library(lib_path.as_path()) {
                                    Ok(lib) => {
                                        println!("✓ Loaded: {}", dep_name);
                                        _rust_ffi_libs.push(lib);
                                        loaded = true;

                                        // Load and parse interface file to register functions
                                        let interface_path = format!("interfaces/{}.clorus-ffi", dep_name);
                                        if let Ok(interface_content) = std::fs::read_to_string(&interface_path) {
                                            match parse_interface_file(&interface_content, dep_name) {
                                                Ok(mut rust_lib) => {
                                                    // Register library with full rust.* module name for lookup
                                                    rust_lib.name = format!("rust.{}", dep_name.replace('-', "_"));
                                                    println!("  → Registered {} functions from {}", rust_lib.functions.len(), dep_name);

                                                    // Register with both full module name and library name
                                                    // Full name (rust.egui_hello) for alias resolution
                                                    repl_engine.register_rust_library(rust_lib.clone());
                                                    // Library name (egui-hello) for ns declaration processing
                                                    rust_lib.name = dep_name.to_string();
                                                    repl_engine.register_rust_library(rust_lib);
                                                }
                                                Err(e) => {
                                                    eprintln!("  ⚠ Failed to parse interface file: {}", e);
                                                }
                                            }
                                        } else {
                                            eprintln!("  ⚠ Interface file not found: {}", interface_path);
                                        }
                                        break;
                                    }
                                    Err(e) => {
                                        eprintln!("⚠ Failed to load {}: {}", dep_name, e);
                                    }
                                }
                            }
                        }

                        if !loaded {
                            eprintln!("⚠ FFI library not found for: {}", dep_name);
                        }
                    }
                    println!();
                }
            }
        }
    }

    // Load project entry file if in a project directory (unless no_auto_load is set)
    if !config.no_auto_load {
        if let Some(ref proj_config) = project {
            if std::path::Path::new(&proj_config.build.entry).exists() {
                println!("Loading {}...", proj_config.build.entry);
                match std::fs::read_to_string(&proj_config.build.entry) {
                Ok(source) => {
                    // Split source into individual top-level forms and evaluate each
                    // This mimics Clojure's behavior of loading files
                    let lines: Vec<&str> = source.lines().collect();
                    let mut current_form = String::new();
                    let mut paren_depth = 0;
                    let mut in_string = false;
                    let mut escape_next = false;
                    let mut forms_loaded = 0;
                    let mut had_error = false;
                    let mut ffi_errors = Vec::new();  // Track FFI-related errors

                    for line in lines {
                        let trimmed = line.trim();

                        // Skip empty lines and comments when not building a form
                        if current_form.is_empty() && (trimmed.is_empty() || trimmed.starts_with(';')) {
                            continue;
                        }

                        current_form.push_str(line);
                        current_form.push('\n');

                        // Track parentheses depth and strings
                        for ch in line.chars() {
                            if escape_next {
                                escape_next = false;
                                continue;
                            }
                            if ch == '\\' {
                                escape_next = true;
                                continue;
                            }
                            if ch == '"' {
                                in_string = !in_string;
                            }
                            if !in_string {
                                if ch == '(' || ch == '[' || ch == '{' {
                                    paren_depth += 1;
                                } else if ch == ')' || ch == ']' || ch == '}' {
                                    paren_depth -= 1;
                                }
                            }
                        }

                        // When we have a complete form (paren_depth returns to 0)
                        if paren_depth == 0 && !current_form.trim().is_empty() {
                            match repl_engine.eval_init(&current_form) {
                                Ok(_) => {
                                    forms_loaded += 1;
                                }
                                Err(e) => {
                                    // Check if this is an FFI-related error
                                    if e.contains("Unknown function") || e.contains("rust.") {
                                        ffi_errors.push(e);
                                    } else {
                                        eprintln!("⚠ Error loading form: {}", e);
                                    }
                                    had_error = true;
                                }
                            }
                            current_form.clear();
                        }
                    }

                    if !had_error {
                        println!("✓ Project loaded ({} forms)", forms_loaded);
                    } else {
                        println!("⚠ Project loaded with errors ({} forms)", forms_loaded);
                    }
                }
                Err(e) => {
                    eprintln!("⚠ Could not read {}: {}", proj_config.build.entry, e);
                }
            }
            println!();
            }
        }
    }

    // Detect if stdin is a TTY (interactive) or piped/redirected
    let is_interactive = io::stdin().is_terminal();

    if is_interactive {
        // Interactive mode: use rustyline for autocomplete and history
        run_interactive_repl(repl_engine, config.clone());
    } else {
        // Piped/redirected mode: read from stdin directly
        run_piped_repl(repl_engine);
    }

    Ok(())
}

/// Run REPL in interactive mode with rustyline (autocomplete, history)
fn run_interactive_repl(mut repl_engine: ReplEngine, config: ReplConfig) {
    // Create rustyline editor with autocomplete
    let mut rl = Editor::<ClorusHelper, DefaultHistory>::new().unwrap();
    rl.set_helper(Some(ClorusHelper::new()));

    // Load history from file
    let history_file = config.history_file.clone().unwrap_or_else(|| {
        std::env::var("HOME")
            .map(|home| format!("{}/.clorus_history", home))
            .unwrap_or_else(|_| ".clorus_history".to_string())
    });

    let _ = rl.load_history(&history_file);

    loop {
        // Build prompt with current namespace
        let prompt = format!("{}λ> ", repl_engine.current_namespace());

        // Read line with autocomplete support
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let input = line.trim();

                // Handle empty input
                if input.is_empty() {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(input);

                // Process input
                if !process_input(&mut repl_engine, input) {
                    break;
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("Goodbye!");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    // Save history to file
    let _ = rl.save_history(&history_file);
}

/// Run REPL in piped mode (read from stdin directly)
fn run_piped_repl(mut repl_engine: ReplEngine) {
    let stdin = io::stdin();
    let reader = stdin.lock();

    for line in reader.lines() {
        match line {
            Ok(input) => {
                let input = input.trim();

                // Handle empty input
                if input.is_empty() {
                    continue;
                }

                // Process input (returns false if should quit)
                if !process_input(&mut repl_engine, input) {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
}

/// Process a single input line (returns false if should quit)
fn process_input(repl_engine: &mut ReplEngine, input: &str) -> bool {
    // Handle commands
    match input {
        ":quit" | ":q" => {
            println!("Goodbye!");
            return false;
        }
        ":help" | ":h" => {
            print_help();
            return true;
        }
        ":examples" | ":e" => {
            print_examples();
            return true;
        }
        _ if input.starts_with(':') => {
            println!("Unknown command: {}", input);
            println!("Type :help for available commands");
            return true;
        }
        _ => {}
    }

    // Evaluate the expression
    match repl_engine.eval(input) {
        Ok(result) => {
            let output = format_result(&result, repl_engine.current_namespace());
            println!("{}", output);
        }
        Err(e) => println!("Error: {}", e),
    }

    true
}

fn print_help() {
    println!("╔════════════════════════════════════╗");
    println!("║  Clorus REPL Help                  ║");
    println!("╚════════════════════════════════════╝");
    println!();
    println!("Commands:");
    println!("  :help, :h       Show this help");
    println!("  :examples, :e   Show example expressions");
    println!("  :quit, :q       Exit REPL");
    println!();
    println!("Editing:");
    println!("  TAB             Autocomplete");
    println!("  ↑/↓             Navigate history");
    println!("  Ctrl-A          Beginning of line");
    println!("  Ctrl-E          End of line");
    println!("  Ctrl-C          Cancel current line");
    println!("  Ctrl-D          Exit REPL");
    println!();
    println!("Operators:");
    println!("  +  Addition");
    println!("  -  Subtraction (unary negation with 1 arg)");
    println!("  *  Multiplication");
    println!("  /  Division");
    println!("  <  Less than (returns 0.0 or 1.0)");
    println!("  >  Greater than (returns 0.0 or 1.0)");
    println!("  =  Equality (returns 0.0 or 1.0)");
    println!();
    println!("Special Forms:");
    println!("  let   Local bindings: (let [x 10 y 20] (+ x y))");
    println!("  def   Define variable: (def x 10)");
    println!("  defn  Define function: (defn add [x y] (+ x y))");
    println!();
    println!("Namespaces:");
    println!("  ns       Declare namespace: (ns my.app.core)");
    println!("  require  Import module: (require [lib :as l])");
    println!("           With selective import: [lib :refer [func1 func2]]");
    println!("           Import all: [lib :refer :all]");
    println!();
}

fn print_examples() {
    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║  Clorus Examples - Type any to try!                  ║");
    println!("╚═══════════════════════════════════════════════════════╝");
    println!();

    println!("📐 BASIC ARITHMETIC");
    println!("  (+ 1 2)                    => 3");
    println!("  (- 10 3)                   => 7");
    println!("  (* 6 7)                    => 42");
    println!("  (/ 20 4)                   => 5");
    println!("  (- 5)                      => -5 (negation)");
    println!();

    println!("📦 VARIABLES (let)");
    println!("  (let [x 10] x)                      => 10");
    println!("  (let [x 5 y 10] (+ x y))            => 15");
    println!("  (let [radius 10] (* 3.14 radius))   => 31.4");
    println!();

    println!("🌍 GLOBAL DEFINITIONS (def)");
    println!("  (def pi 3.14159)           => pi defined");
    println!("  (* pi 2)                   => 6.28318");
    println!("  (def answer 42)            => answer defined");
    println!();

    println!("🔧 FUNCTIONS (defn)");
    println!("  (defn square [x] (* x x))         => function defined");
    println!("  (square 5)                        => 25");
    println!("  (defn circle-area [r] (* 3.14 (* r r)))");
    println!("  (circle-area 10)                  => 314");
    println!();

    println!("🔀 CONDITIONALS (if)");
    println!("  (if (< 5 10) 100 200)              => 100");
    println!("  (if (> 5 10) 100 200)              => 200");
    println!("  (if (= 42 42) (* 2 3) (+ 1 1))     => 6");
    println!();

    println!("🔁 RECURSION");
    println!("  (defn factorial [n]");
    println!("    (if (< n 2) 1 (* n (factorial (- n 1)))))");
    println!("  (factorial 5)                      => 120");
    println!();
    println!("  (defn fib [n]");
    println!("    (if (< n 2) n");
    println!("      (+ (fib (- n 1)) (fib (- n 2)))))");
    println!("  (fib 10)                           => 55");
    println!();

    println!("📦 NAMESPACES");
    println!("  (ns my.app)                        => switches to my.app namespace");
    println!("  (require [math :as m])             => require with alias");
    println!("  (require [lib :refer [func1 func2]]) => selective import");
    println!("  my.appλ>                           => prompt shows namespace");
    println!();

    println!("🔢 COMPARISONS");
    println!("  (< 5 10)                   => 1.0 (true)");
    println!("  (> 5 10)                   => 0.0 (false)");
    println!("  (= 42 42)                  => 1.0 (true)");
    println!();

    println!("📚 DATA STRUCTURES (Parsed, runtime support in progress)");
    println!("  Lists:");
    println!("    '(1 2 3)                        => (1 2 3)");
    println!("    (list 1 2 3)                    => (1 2 3)");
    println!();
    println!("  Vectors:");
    println!("    [1 2 3]                         => [1 2 3]");
    println!("    (vector 1 2 3)                  => [1 2 3]");
    println!("    (conj [1 2] 3)                  => [1 2 3]");
    println!("    (nth [10 20 30] 1)              => 20");
    println!();
    println!("  Maps:");
    println!("    {{:name \"Alice\" :age 30}}         => {{:name \"Alice\" :age 30}}");
    println!("    (get {{:x 10 :y 20}} :x)          => 10");
    println!();
    println!("  Keywords & Strings:");
    println!("    :keyword                        => :keyword");
    println!("    \"hello world\"                   => \"hello world\"");
    println!("    true, false, nil                => boolean/nil values");
    println!();

    println!("🗂️  FILE OPERATIONS (rust.fs)");
    println!("  (use rust.fs)                           => loads module");
    println!("  (fs/write \"test.txt\" \"Hello!\")         => 1 (success)");
    println!("  (fs/exists? \"test.txt\")                 => 1 (true)");
    println!("  (fs/is-file? \"test.txt\")                => 1 (true)");
    println!("  (fs/copy \"test.txt\" \"backup.txt\")      => 1 (success)");
    println!("  (fs/remove \"backup.txt\")                => 1 (success)");
    println!("  (fs/create-dir \"output\")                => 1 (success)");
    println!();

    println!("📖 CLOJURE-STYLE I/O (clorus.core)");
    println!("  (use clorus.core)                       => loads module");
    println!("  (spit \"file.txt\" \"content\")            => 1 (success)");
    println!("  (def ptr (slurp \"file.txt\"))            => <pointer>");
    println!("  ; Full string support coming with Value* types!");
    println!();

    println!("🔗 NESTED EXPRESSIONS");
    println!("  (+ (* 2 3) 4)                      => 10");
    println!("  (let [x 5] (* x x))                => 25");
    println!("  (/ (+ 10 20) (- 10 5))             => 6");
    println!();

    println!("💡 TIP: Use TAB to autocomplete function names!");
    println!();
}

/// Load clorus-std dynamic library to make fs functions available to JIT
fn load_std_library() -> Result<libloading::Library, String> {
    use std::env;
    use std::path::Path;

    // Determine library file name based on platform
    #[cfg(target_os = "macos")]
    let lib_name = "libclorus_std.dylib";

    #[cfg(target_os = "linux")]
    let lib_name = "libclorus_std.so";

    #[cfg(target_os = "windows")]
    let lib_name = "clorus_std.dll";

    // Try to find the library
    let mut lib_path = None;

    // Try CLORUS_HOME/lib first (for installed version)
    if let Ok(clorus_home) = env::var("CLORUS_HOME") {
        let installed_path = Path::new(&clorus_home).join("lib").join(lib_name);
        if installed_path.exists() {
            lib_path = Some(installed_path);
        }
    }

    // Try ~/.clorus/lib if CLORUS_HOME not set
    if lib_path.is_none() {
        if let Ok(home) = env::var("HOME") {
            let default_install = Path::new(&home).join(".clorus").join("lib").join(lib_name);
            if default_install.exists() {
                lib_path = Some(default_install);
            }
        }
    }

    // Try release build in current project
    if lib_path.is_none() {
        let release_path = Path::new("target/release").join(lib_name);
        if release_path.exists() {
            lib_path = Some(release_path);
        } else {
            // Try debug build
            let debug_path = Path::new("target/debug").join(lib_name);
            if debug_path.exists() {
                lib_path = Some(debug_path);
            }
        }
    }

    // If not found in current dir, try workspace root
    if lib_path.is_none() {
        let current_dir = env::current_dir().map_err(|e| format!("Failed to get current dir: {}", e))?;

        // Go up directories to find workspace root
        let mut search_dir = current_dir.as_path();
        for _ in 0..5 {
            let release_path = search_dir.join("target/release").join(lib_name);
            if release_path.exists() {
                lib_path = Some(release_path);
                break;
            }

            let debug_path = search_dir.join("target/debug").join(lib_name);
            if debug_path.exists() {
                lib_path = Some(debug_path);
                break;
            }

            if let Some(parent) = search_dir.parent() {
                search_dir = parent;
            } else {
                break;
            }
        }
    }

    let lib_path = lib_path.ok_or_else(|| {
        format!("clorus-std library not found")
    })?;

    // Load the library with RTLD_GLOBAL flag on Unix
    unsafe {
        #[cfg(unix)]
        {
            use libloading::os::unix::Library as UnixLibrary;
            use libloading::os::unix::RTLD_GLOBAL;
            use libloading::os::unix::RTLD_NOW;

            UnixLibrary::open(Some(&lib_path), RTLD_NOW | RTLD_GLOBAL)
                .map(|lib| lib.into())
                .map_err(|e| format!("Failed to load clorus-std library: {}", e))
        }

        #[cfg(not(unix))]
        {
            libloading::Library::new(&lib_path)
                .map_err(|e| format!("Failed to load clorus-std library: {}", e))
        }
    }
}

/// Load clorus-core dynamic library to make Clojure-style functions available to JIT
fn load_core_library() -> Result<libloading::Library, String> {
    use std::env;
    use std::path::Path;

    // Determine library file name based on platform
    #[cfg(target_os = "macos")]
    let lib_name = "libclorus_core.dylib";

    #[cfg(target_os = "linux")]
    let lib_name = "libclorus_core.so";

    #[cfg(target_os = "windows")]
    let lib_name = "clorus_core.dll";

    // Try to find the library
    let mut lib_path = None;

    // Try CLORUS_HOME/lib first (for installed version)
    if let Ok(clorus_home) = env::var("CLORUS_HOME") {
        let installed_path = Path::new(&clorus_home).join("lib").join(lib_name);
        if installed_path.exists() {
            lib_path = Some(installed_path);
        }
    }

    // Try ~/.clorus/lib if CLORUS_HOME not set
    if lib_path.is_none() {
        if let Ok(home) = env::var("HOME") {
            let default_install = Path::new(&home).join(".clorus").join("lib").join(lib_name);
            if default_install.exists() {
                lib_path = Some(default_install);
            }
        }
    }

    // Try release build in current project
    if lib_path.is_none() {
        let release_path = Path::new("target/release").join(lib_name);
        if release_path.exists() {
            lib_path = Some(release_path);
        } else {
            // Try debug build
            let debug_path = Path::new("target/debug").join(lib_name);
            if debug_path.exists() {
                lib_path = Some(debug_path);
            }
        }
    }

    // If not found in current dir, try workspace root
    if lib_path.is_none() {
        let current_dir = env::current_dir().map_err(|e| format!("Failed to get current dir: {}", e))?;

        // Go up directories to find workspace root
        let mut search_dir = current_dir.as_path();
        for _ in 0..5 {
            let release_path = search_dir.join("target/release").join(lib_name);
            if release_path.exists() {
                lib_path = Some(release_path);
                break;
            }

            let debug_path = search_dir.join("target/debug").join(lib_name);
            if debug_path.exists() {
                lib_path = Some(debug_path);
                break;
            }

            if let Some(parent) = search_dir.parent() {
                search_dir = parent;
            } else {
                break;
            }
        }
    }

    let lib_path = lib_path.ok_or_else(|| {
        format!("clorus-core library not found")
    })?;

    // Load the library with RTLD_GLOBAL flag on Unix
    unsafe {
        #[cfg(unix)]
        {
            use libloading::os::unix::Library as UnixLibrary;
            use libloading::os::unix::RTLD_GLOBAL;
            use libloading::os::unix::RTLD_NOW;

            UnixLibrary::open(Some(&lib_path), RTLD_NOW | RTLD_GLOBAL)
                .map(|lib| lib.into())
                .map_err(|e| format!("Failed to load clorus-core library: {}", e))
        }

        #[cfg(not(unix))]
        {
            libloading::Library::new(&lib_path)
                .map_err(|e| format!("Failed to load clorus-core library: {}", e))
        }
    }
}

/// Load clorus-runtime dynamic library to make Value* operations available to JIT
fn load_runtime_library() -> Result<libloading::Library, String> {
    use std::env;
    use std::path::Path;

    // Determine library file name based on platform
    #[cfg(target_os = "macos")]
    let lib_name = "libclorus_runtime.dylib";

    #[cfg(target_os = "linux")]
    let lib_name = "libclorus_runtime.so";

    #[cfg(target_os = "windows")]
    let lib_name = "clorus_runtime.dll";

    // Try to find the library in multiple locations
    let mut lib_path = None;

    // 1. Try current project's target/release/deps (where Cargo places cdylib)
    let release_deps_path = Path::new("target/release/deps").join(lib_name);
    if release_deps_path.exists() {
        lib_path = Some(release_deps_path);
    }

    // 2. Try current project's target/release
    if lib_path.is_none() {
        let release_path = Path::new("target/release").join(lib_name);
        if release_path.exists() {
            lib_path = Some(release_path);
        }
    }

    // 3. Try current project's target/debug/deps
    if lib_path.is_none() {
        let debug_deps_path = Path::new("target/debug/deps").join(lib_name);
        if debug_deps_path.exists() {
            lib_path = Some(debug_deps_path);
        }
    }

    // 4. Try current project's target/debug
    if lib_path.is_none() {
        let debug_path = Path::new("target/debug").join(lib_name);
        if debug_path.exists() {
            lib_path = Some(debug_path);
        }
    }

    // 3. Try to find it relative to the clorus executable (for installed version)
    if lib_path.is_none() {
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Check ../lib/ directory (typical installation layout)
                let installed_path = exe_dir.parent()
                    .map(|p| p.join("lib").join(lib_name));
                if let Some(p) = installed_path {
                    if p.exists() {
                        lib_path = Some(p);
                    }
                }

                // Check same directory as executable
                if lib_path.is_none() {
                    let same_dir = exe_dir.join(lib_name);
                    if same_dir.exists() {
                        lib_path = Some(same_dir);
                    }
                }
            }
        }
    }

    // 5. Try workspace target directory (for development)
    if lib_path.is_none() {
        // Walk up to find workspace root
        let mut current = env::current_dir().ok();
        while let Some(dir) = current {
            // Try deps directories first
            let workspace_release_deps = dir.join("target/release/deps").join(lib_name);
            if workspace_release_deps.exists() {
                lib_path = Some(workspace_release_deps);
                break;
            }
            let workspace_release = dir.join("target/release").join(lib_name);
            if workspace_release.exists() {
                lib_path = Some(workspace_release);
                break;
            }
            let workspace_debug_deps = dir.join("target/debug/deps").join(lib_name);
            if workspace_debug_deps.exists() {
                lib_path = Some(workspace_debug_deps);
                break;
            }
            let workspace_debug = dir.join("target/debug").join(lib_name);
            if workspace_debug.exists() {
                lib_path = Some(workspace_debug);
                break;
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }
    }

    let lib_path = lib_path.ok_or_else(|| {
        format!(
            "clorus-runtime library not found.\nSearched:\n  - target/release/{}\n  - target/debug/{}\n  - Installed library directory\n  - Workspace target directories",
            lib_name, lib_name
        )
    })?;

    // Load with RTLD_GLOBAL so JIT can find symbols
    unsafe {
        #[cfg(unix)]
        {
            use libloading::os::unix::Library as UnixLibrary;
            use libloading::os::unix::RTLD_GLOBAL;
            use libloading::os::unix::RTLD_NOW;

            UnixLibrary::open(Some(&lib_path), RTLD_NOW | RTLD_GLOBAL)
                .map(|lib| lib.into())
                .map_err(|e| format!("Failed to load clorus-runtime library: {}", e))
        }

        #[cfg(not(unix))]
        {
            libloading::Library::new(&lib_path)
                .map_err(|e| format!("Failed to load clorus-runtime library: {}", e))
        }
    }
}

/// Load a dynamic library with RTLD_GLOBAL flag on Unix
fn load_dynamic_library(lib_path: &std::path::Path) -> Result<libloading::Library, String> {
    unsafe {
        #[cfg(unix)]
        {
            use libloading::os::unix::Library as UnixLibrary;
            use libloading::os::unix::RTLD_GLOBAL;
            use libloading::os::unix::RTLD_NOW;

            UnixLibrary::open(Some(lib_path), RTLD_NOW | RTLD_GLOBAL)
                .map(|lib| lib.into())
                .map_err(|e| format!("Failed to load library {}: {}", lib_path.display(), e))
        }

        #[cfg(not(unix))]
        {
            libloading::Library::new(lib_path)
                .map_err(|e| format!("Failed to load library {}: {}", lib_path.display(), e))
        }
    }
}

/// Parse a .clorus-ffi interface file and create a RustLibrary
/// Format: (interface lib-name (fn func-name [param :type ...] :return-type "doc"))
fn parse_interface_file(content: &str, lib_name: &str) -> Result<clorus::codegen::RustLibrary, String> {
    use clorus::codegen::{RustLibrary, RustFunction, RustParam};

    let mut functions = Vec::new();

    // Simple S-expression parser for interface files
    // We can't use the full Clorus parser because interface syntax has type annotations
    let lines: Vec<&str> = content.lines()
        .filter(|line| !line.trim().starts_with(";;") && !line.trim().is_empty())
        .collect();

    let full_content = lines.join(" ");

    // Find all (fn ...) forms within the interface declaration
    let mut depth = 0;
    let mut in_interface = false;
    let mut current_fn = String::new();
    let mut in_fn = false;

    for ch in full_content.chars() {
        if ch == '(' {
            depth += 1;
            if depth == 2 && in_interface {
                // Start of potential fn form
                current_fn.clear();
                in_fn = true;
            }
        }

        if in_fn {
            current_fn.push(ch);
        }

        if ch == ')' {
            depth -= 1;
            if depth == 1 && in_interface && in_fn {
                // End of fn form
                if let Some(func) = parse_fn_form(&current_fn)? {
                    functions.push(func);
                }
                in_fn = false;
            }
            if depth == 0 {
                in_interface = false;
            }
        }

        // Detect (interface ...)
        if depth == 1 && !in_interface && full_content[0..].contains("interface") {
            in_interface = true;
        }
    }

    Ok(RustLibrary {
        name: lib_name.to_string(),
        functions,
    })
}

/// Parse a single (fn name [params] :return-type "doc") form
fn parse_fn_form(s: &str) -> Result<Option<clorus::codegen::RustFunction>, String> {
    use clorus::codegen::{RustFunction, RustParam};

    let s = s.trim();
    if !s.starts_with("(fn ") {
        return Ok(None);
    }

    // Remove outer parens and "fn"
    let s = s.strip_prefix("(fn ").unwrap().strip_suffix(")").unwrap_or(s);

    // Parse: name [param :type ...] :return-type "doc"
    let tokens: Vec<&str> = s.split_whitespace().collect();

    if tokens.is_empty() {
        return Ok(None);
    }

    let func_name = tokens[0].to_string();

    // Convert kebab-case to snake_case for Rust FFI
    // Clorus uses kebab-case (show-gui), Rust uses snake_case (show_gui)
    let rust_func_name = func_name.replace('-', "_");

    // Find parameter vector [...]
    let mut params = Vec::new();
    let mut return_type = "Value*".to_string();

    let vec_start = s.find('[');
    let vec_end = s.find(']');

    if let (Some(start), Some(end)) = (vec_start, vec_end) {
        let param_str = &s[start+1..end];
        let param_tokens: Vec<&str> = param_str.split_whitespace().collect();

        let mut i = 0;
        while i < param_tokens.len() {
            let param_name = param_tokens[i];

            // Check if next token is a type annotation
            let param_type = if i + 1 < param_tokens.len() && param_tokens[i + 1].starts_with(':') {
                i += 2;
                let raw_type = param_tokens[i - 1].strip_prefix(':').unwrap_or("Value*");
                // Normalize type names to match codegen expectations
                match raw_type {
                    "string" => "String",
                    "f64" => "f64",
                    "i32" => "i32",
                    "bool" => "bool",
                    _ => "Value*"
                }.to_string()
            } else {
                i += 1;
                "Value*".to_string()
            };

            params.push(RustParam {
                name: param_name.to_string(),
                type_name: param_type,
            });
        }

        // Look for return type after ]
        let after_vec = &s[end+1..];
        for token in after_vec.split_whitespace() {
            if token.starts_with(':') && !token.starts_with("::") {
                let raw_type = token.strip_prefix(':').unwrap_or("Value*");
                // Normalize type names to match codegen expectations
                return_type = match raw_type {
                    "string" => "String",
                    "f64" => "f64",
                    "i32" => "i32",
                    "bool" => "bool",
                    _ => "Value*"
                }.to_string();
                break;
            }
        }
    }

    Ok(Some(RustFunction {
        name: rust_func_name,
        params,
        return_type,
    }))
}
