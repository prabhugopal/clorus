mod repl_engine;
mod project;

// Re-export for clorus-replx and other consumers
pub use repl_engine::{ReplEngine, EvalResult};
pub use project::ProjectConfig;
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
static FORCE_LINK_ATOM_SWAP: unsafe extern "C" fn(*mut Value, *mut Value, *mut Value) -> *mut Value = clorus_runtime::atom::clorus_swap;

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
#[used]
static FORCE_LINK_DERIVE: extern "C" fn(*mut Value, *mut Value) -> *mut Value = clorus_runtime::hierarchy::clorus_derive;
#[used]
static FORCE_LINK_UNDERIVE: extern "C" fn(*mut Value, *mut Value) -> *mut Value = clorus_runtime::hierarchy::clorus_underive;
#[used]
static FORCE_LINK_ISA_I32: extern "C" fn(*mut Value, *mut Value) -> i32 = clorus_runtime::hierarchy::clorus_isa_i32;
#[used]
static FORCE_LINK_PARENTS: extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::hierarchy::clorus_parents;
#[used]
static FORCE_LINK_ANCESTORS: extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::hierarchy::clorus_ancestors;
#[used]
static FORCE_LINK_DESCENDANTS: extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::hierarchy::clorus_descendants;
#[used]
static FORCE_LINK_PROTOCOL_SATISFIES: extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char) -> i32 =
    clorus_runtime::protocols::clorus_protocol_satisfies_type_i32;
#[used]
static FORCE_LINK_GENSYM: extern "C" fn(*const std::ffi::c_char) -> *mut Value =
    clorus_runtime::value::clorus_gensym;

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
pub fn format_result(result: &repl_engine::EvalResult, namespace: &str) -> String {
    use repl_engine::EvalKind;

    match &result.kind {
        EvalKind::Def(name) | EvalKind::Defn(name) | EvalKind::Defmacro(name) => {
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

        // Try to find clorus/core.clr in CLORUS_HOME/stdlib or ~/.clorus/stdlib
        if core_stdlib_path.is_none() {
            if let Ok(clorus_home) = std::env::var("CLORUS_HOME") {
                let path = std::path::Path::new(&clorus_home)
                    .join("stdlib")
                    .join("clorus")
                    .join("core.clr");
                if path.exists() {
                    core_stdlib_path = Some(path);
                }
            }
        }

        if core_stdlib_path.is_none() {
            // Binary-relative fallback:
            // - installed layout: <CLORUS_HOME>/bin/clorus + <CLORUS_HOME>/stdlib/clorus/core.clr
            // - repo layout: <repo>/target/{debug,release}/clorus + <repo>/stdlib/clorus/core.clr
            if let Ok(exe) = std::env::current_exe() {
                let mut candidates = Vec::new();
                if let Some(exe_dir) = exe.parent() {
                    candidates.push(exe_dir.join("../stdlib/clorus/core.clr"));     // installed
                    candidates.push(exe_dir.join("../../stdlib/clorus/core.clr"));  // repo target/{debug,release}
                    candidates.push(exe_dir.join("../../../stdlib/clorus/core.clr")); // extra fallback
                }
                for path in candidates {
                    if path.exists() {
                        core_stdlib_path = Some(path);
                        break;
                    }
                }
            }
        }

        if core_stdlib_path.is_none() {
            if let Ok(home) = std::env::var("HOME") {
                let path = std::path::Path::new(&home)
                    .join(".clorus")
                    .join("stdlib")
                    .join("clorus")
                    .join("core.clr");
                if path.exists() {
                    core_stdlib_path = Some(path);
                }
            }
        }

        // Local development fallback (project/workspace-relative)
        if core_stdlib_path.is_none() {
            let candidates = [
                std::path::PathBuf::from("stdlib/clorus/core.clr"),
                std::path::PathBuf::from("../stdlib/clorus/core.clr"),
                std::path::PathBuf::from("../../stdlib/clorus/core.clr"),
            ];
            for path in candidates {
                if path.exists() {
                    core_stdlib_path = Some(path);
                    break;
                }
            }
        }
    }

    let mut _core_lib: Option<libloading::Library> = None;
    let disable_core_lib = std::env::var("CLORUS_REPL_NO_CORE_LIB")
        .ok()
        .map(|v| v != "0")
        .unwrap_or(false);

    // Always load clorus-core dylib unless explicitly disabled.
    // REPL JIT stdlib loads function bodies, but runtime externs like clorus_slurp/clorus_spit
    // still need process-global symbols to resolve at execution time.
    if !disable_core_lib {
        match load_core_library() {
            Ok(lib) => {
                _core_lib = Some(lib);
            }
            Err(e) => {
                eprintln!("⚠ Failed to load clorus-core dylib: {}", e);
            }
        }
    }

    if let Some(core_path) = core_stdlib_path.as_ref() {
        // Default to JIT stdlib in REPL so core functions like `map` are callable.
        let enable_jit_stdlib = std::env::var("CLORUS_REPL_LOAD_STDLIB")
            .ok()
            .map(|v| v != "0")
            .unwrap_or(true);

        // Primary path: JIT-compile stdlib.
        if enable_jit_stdlib {
            match std::fs::read_to_string(core_path) {
                Ok(source) => match repl_engine.load_stdlib_batch(source) {
                    Ok(count) => {
                        println!("✓ Loaded clorus.core ({} functions) via JIT stdlib", count);
                        core_loaded = true;
                    }
                    Err(e) => {
                        eprintln!("⚠ Error loading stdlib via JIT: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("⚠ Could not read stdlib: {}", e);
                }
            }
        }

        // Optional fallback path: symbols-only preload from stdlib source.
        // This mode exists for environments where JIT stdlib loading is disabled.
        if !core_loaded && !disable_core_lib {
            if _core_lib.is_some() {
                match std::fs::read_to_string(core_path) {
                    Ok(source) => match repl_engine.load_stdlib_symbols_only(source) {
                        Ok(count) => {
                            println!(
                                "✓ Loaded clorus.core symbols ({} functions) via clorus-core dylib",
                                count
                            );
                            core_loaded = true;
                        }
                        Err(e) => {
                            eprintln!("⚠ Error loading stdlib symbols: {}", e);
                        }
                    },
                    Err(e) => {
                        eprintln!("⚠ Could not read stdlib: {}", e);
                    }
                }
            }
        }

        if !core_loaded {
            println!("⚠ clorus.core not loaded (set CLORUS_REPL_NO_CORE_LIB=1 to skip dylib, or CLORUS_REPL_LOAD_STDLIB=1 to JIT)");
        }
    }

    if !core_loaded && core_stdlib_path.is_none() {
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
                                        let rust_lib_result = if let Ok(interface_content) = std::fs::read_to_string(&interface_path) {
                                            // Use interface file if available
                                            parse_interface_file(&interface_content, dep_name)
                                        } else {
                                            // Auto-parse from generated wrapper source (interface = false case)
                                            let lib_name_base = dep_name.replace('-', "_");
                                            let wrapper_path = format!("target/rust-ffi/{}_ffi/src/lib.rs", lib_name_base);
                                            if let Ok(wrapper_content) = std::fs::read_to_string(&wrapper_path) {
                                                parse_wrapper_source(&wrapper_content, dep_name)
                                            } else {
                                                Err(format!("Neither interface file nor generated wrapper found for {}", dep_name))
                                            }
                                        };

                                        match rust_lib_result {
                                            Ok(mut rust_lib) => {
                                                // Register library with full rust.* module name for lookup
                                                rust_lib.name = format!("rust.{}", dep_name);
                                                println!("  → Registered {} functions from {}", rust_lib.functions.len(), dep_name);

                                                // DEBUG: Print all functions
                                                if std::env::var("CLORUS_DEBUG_REPL").is_ok() {
                                                    eprintln!("DEBUG: Functions in rust.{}:", dep_name);
                                                }
                                                for (i, func) in rust_lib.functions.iter().enumerate() {
                                                    if i < 10 {
                                                        eprintln!("  - {}", func.name);
                                                    }
                                                }
                                                if rust_lib.functions.len() > 10 {
                                                    eprintln!("  ... and {} more", rust_lib.functions.len() - 10);
                                                }

                                                // Register with both full module name and library name
                                                // Full name (rust.egui_hello) for alias resolution
                                                repl_engine.register_rust_library(rust_lib.clone());
                                                // Library name (egui-hello) for ns declaration processing
                                                rust_lib.name = dep_name.to_string();
                                                repl_engine.register_rust_library(rust_lib);
                                            }
                                            Err(e) => {
                                                eprintln!("  ⚠ Failed to register FFI functions: {}", e);
                                            }
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

    // Load .clip dependencies if in a project directory
    let mut _clip_libs = Vec::new();  // Keep loaded .dylibs alive
    if let Some(ref proj_config) = project {
        // Try to load Clorus.toml manifest for dependencies
        if std::path::Path::new("Clorus.toml").exists() {
            // Load manifest using clorus-cli's manifest loader
            match load_clip_dependencies_for_repl() {
                Ok((libs, packages)) => {
                    if !packages.is_empty() {
                        println!("📦 Loaded {} .clip package(s) for REPL", packages.len());
                        for (i, pkg) in packages.iter().enumerate() {
                            println!("   ✓ {} v{}", pkg.name, pkg.version);
                            // Register the namespace with the REPL engine
                            repl_engine.register_clip_namespace(&pkg.name);

                            // Register discovered modules with full namespace paths
                            for module in &pkg.modules {
                                let full_module_path = format!("{}.{}", pkg.name, module);
                                repl_engine.register_clip_namespace(&full_module_path);
                            }

                            // Call the library initialization function to register protocols
                            let init_fn_name = format!("clorus_{}_init", pkg.name.replace('-', "_"));
                            if let Some(lib) = libs.get(i) {
                                unsafe {
                                    type InitFunc = unsafe extern "C" fn() -> *mut std::ffi::c_void;
                                    match lib.get::<InitFunc>(init_fn_name.as_bytes()) {
                                        Ok(init_fn) => {
                                            init_fn();
                                        }
                                        Err(_) => {
                                            // Init function not found - that's OK for older packages
                                        }
                                    }
                                }
                            }
                        }
                        println!();
                        _clip_libs = libs;  // Keep libraries loaded
                    }
                }
                Err(e) => {
                    eprintln!("⚠ Warning: Could not load .clip dependencies: {}", e);
                    eprintln!("  REPL will continue but .clip functions may not be available.");
                    println!();
                }
            }
        }
    }

    // Load project entry file if in a project directory, unless disabled.
    // Env override helps scripted/piped REPL use-cases where project auto-load is undesirable.
    let no_auto_load = config.no_auto_load
        || std::env::var("CLORUS_REPL_NO_AUTO_LOAD")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);

    if !no_auto_load {
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
                            if std::env::var("CLORUS_DEBUG_REPL").is_ok() {
                                eprintln!("DEBUG: Loading form {}...", forms_loaded + 1);
                            }
                            match repl_engine.eval_init(&current_form) {
                                Ok(_) => {
                                    forms_loaded += 1;
                                    if std::env::var("CLORUS_DEBUG_REPL").is_ok() {
                                        eprintln!("DEBUG: Form {} loaded successfully", forms_loaded);
                                    }
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

                        // Print first few FFI errors to help debug
                        if !ffi_errors.is_empty() {
                            eprintln!();
                            eprintln!("FFI-related errors during project load:");
                            for (i, err) in ffi_errors.iter().enumerate().take(3) {
                                eprintln!("  {}. {}", i + 1, err);
                            }
                            if ffi_errors.len() > 3 {
                                eprintln!("  ... and {} more", ffi_errors.len() - 3);
                            }
                        }
                    }

                    // Finalize project loading - clear init_forms to avoid recompilation
                    // This prevents O(n²) behavior for interactive REPL inputs
                    repl_engine.finalize_project_load();
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

    // Try local build in current project
    if lib_path.is_none() {
        #[cfg(debug_assertions)]
        let local_candidates = [Path::new("target/debug").join(lib_name), Path::new("target/release").join(lib_name)];
        #[cfg(not(debug_assertions))]
        let local_candidates = [Path::new("target/release").join(lib_name), Path::new("target/debug").join(lib_name)];

        for candidate in local_candidates {
            if candidate.exists() {
                lib_path = Some(candidate);
                break;
            }
        }
    }

    // If not found in current dir, try workspace root
    if lib_path.is_none() {
        let current_dir = env::current_dir().map_err(|e| format!("Failed to get current dir: {}", e))?;

        // Go up directories to find workspace root
        let mut search_dir = current_dir.as_path();
        for _ in 0..5 {
            #[cfg(debug_assertions)]
            let search_candidates = [search_dir.join("target/debug").join(lib_name), search_dir.join("target/release").join(lib_name)];
            #[cfg(not(debug_assertions))]
            let search_candidates = [search_dir.join("target/release").join(lib_name), search_dir.join("target/debug").join(lib_name)];

            for candidate in search_candidates {
                if candidate.exists() {
                    lib_path = Some(candidate);
                    break;
                }
            }
            if lib_path.is_some() {
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
pub fn load_runtime_library() -> Result<libloading::Library, String> {
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
pub fn load_dynamic_library(lib_path: &std::path::Path) -> Result<libloading::Library, String> {
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

/// Parse generated wrapper source to extract FFI function signatures
/// This handles the case when interface = false and no interface file exists
/// Parses the auto-generated target/rust-ffi/{name}_ffi/src/lib.rs file
pub fn parse_wrapper_source(content: &str, lib_name: &str) -> Result<clorus::codegen::RustLibrary, String> {
    use clorus::codegen::{RustLibrary, RustFunction, RustParam};

    let mut functions = Vec::new();

    // Parse Rust source for #[no_mangle] pub extern "C" fn declarations
    // Example: pub extern "C" fn clorus_draw_rect(handle: *mut u8, x: f64, y: f64, w: f64, h: f64, color: f64) -> ()

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Look for #[no_mangle]
        if line == "#[no_mangle]" && i + 1 < lines.len() {
            let next_line = lines[i + 1].trim();

            // Check if next line is a function declaration
            if next_line.starts_with("pub extern \"C\" fn ") {
                // Extract function signature
                // Format: pub extern "C" fn clorus_func_name(params...) -> return_type

                if let Some(fn_start) = next_line.find(" fn ") {
                    if let Some(paren_start) = next_line.find('(') {
                        // Extract function name (clorus_func_name)
                        let full_fn_name = next_line[fn_start + 4..paren_start].trim();

                        // Convert clorus_func_name to func-name for Clorus
                        let fn_name = if full_fn_name.starts_with("clorus_") {
                            full_fn_name[7..].replace('_', "-")
                        } else {
                            full_fn_name.replace('_', "-")
                        };

                        // Extract parameters (simplified - just count them as Value*)
                        // For now, assume all params are Value* (we can improve this later)
                        let params_end = if let Some(paren_end) = next_line.find(')') {
                            paren_end
                        } else {
                            next_line.len()
                        };

                        let params_str = &next_line[paren_start + 1..params_end];
                        let mut params = Vec::new();

                        // Split by comma to get individual parameters
                        if !params_str.trim().is_empty() {
                            for param in params_str.split(',') {
                                let param = param.trim();
                                if !param.is_empty() {
                                    // Extract parameter name and type (name: type)
                                    if let Some(colon_pos) = param.find(':') {
                                        let param_name = param[..colon_pos].trim();
                                        let param_type = param[colon_pos + 1..].trim();

                                        // Map Rust types to Clorus FFI types
                                        let clorus_type = match param_type {
                                            "*mut u8" => "*mut u8",  // Keep as-is for opaque pointers
                                            "*mut c_char" => "String",
                                            "f64" => "f64",
                                            "f32" => "f32",
                                            "i64" => "i64",
                                            "i32" => "i32",
                                            "bool" => "bool",
                                            "u32" => "u32",
                                            "u64" => "u64",
                                            _ => param_type, // Keep unknown types as-is
                                        };

                                        params.push(RustParam {
                                            name: param_name.to_string(),
                                            type_name: clorus_type.to_string(),
                                        });
                                    }
                                }
                            }
                        }

                        // Extract return type
                        let return_type = if let Some(arrow_pos) = next_line.find("-> ") {
                            let return_str = next_line[arrow_pos + 3..].trim();
                            // Remove trailing { if present
                            let clean_return = if let Some(brace_pos) = return_str.find('{') {
                                return_str[..brace_pos].trim()
                            } else {
                                return_str
                            };

                            // Map Rust return types to Clorus FFI types
                            match clean_return {
                                "()" => "()",  // Keep as-is for void
                                "*mut u8" => "*mut u8",  // Keep as-is for opaque pointers
                                "*mut c_char" => "String",
                                "bool" => "bool",
                                "f64" => "f64",
                                "f32" => "f32",
                                "i64" => "i64",
                                "i32" => "i32",
                                "u32" => "u32",
                                "u64" => "u64",
                                _ => clean_return,
                            }.to_string()
                        } else {
                            "()".to_string()  // Default to void, not "void"
                        };

                        functions.push(RustFunction {
                            name: fn_name.clone(),
                            params,
                            return_type,
                        });

                        // DEBUG: Print first 5 functions
                        if functions.len() <= 5 {
                            if std::env::var("CLORUS_DEBUG_REPL").is_ok() {
                                eprintln!("DEBUG: Registered FFI function: {} (from {})", fn_name, full_fn_name);
                            }
                        }
                    }
                }
            }

            i += 2; // Skip the #[no_mangle] and function declaration lines
        } else {
            i += 1;
        }
    }

    Ok(RustLibrary {
        name: lib_name.to_string(),
        functions,
    })
}

/// Parse a .clorus-ffi interface file and create a RustLibrary
/// Format: (interface lib-name (fn func-name [param :type ...] :return-type "doc"))
pub fn parse_interface_file(content: &str, lib_name: &str) -> Result<clorus::codegen::RustLibrary, String> {
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

// Simplified ClipPackage struct for REPL
struct ClipPackage {
    name: String,
    version: String,
    object_path: Option<std::path::PathBuf>,
    rust_ffi_libs: Vec<std::path::PathBuf>,  // Rust FFI static libraries to link
    frameworks: Vec<String>,  // macOS frameworks to link (from [link] section)
    modules: Vec<String>,  // Discovered modules (e.g., ["components.button", "utils.text"])
    clip_path: String,  // Original .clip file path (for module discovery)
}

/// Load .clip dependencies for REPL
/// Returns (Vec<libloading::Library>, Vec<ClipPackage>) to keep libraries loaded
fn load_clip_dependencies_for_repl() -> Result<(Vec<libloading::Library>, Vec<ClipPackage>), String> {
    use std::fs;
    use std::path::PathBuf;

    // Read and parse Clorus.toml
    let manifest_content = fs::read_to_string("Clorus.toml")
        .map_err(|e| format!("Failed to read Clorus.toml: {}", e))?;

    let manifest: toml::Value = toml::from_str(&manifest_content)
        .map_err(|e| format!("Failed to parse Clorus.toml: {}", e))?;

    let mut packages = Vec::new();
    let mut loaded_libs = Vec::new();

    // Extract rust-dependencies for FFI linking
    let mut rust_ffi_libs: Vec<PathBuf> = Vec::new();
    if let Some(rust_deps) = manifest.get("rust-dependencies").and_then(|v| v.as_table()) {
        for (dep_name, dep_info) in rust_deps {
            // Get the path to the Rust crate
            if let Some(path_table) = dep_info.as_table() {
                if let Some(path) = path_table.get("path").and_then(|p| p.as_str()) {
                    // Convert dep name to library name (e.g., coral-gfx -> coral_gfx_ffi)
                    let lib_name_base = dep_name.replace('-', "_");
                    let lib_name = format!("lib{}_ffi.a", lib_name_base);

                    // The rust-dependencies path points to the Rust source directory
                    // But the built library is in the parent directory's target/rust-ffi/
                    let rust_path = PathBuf::from(path);
                    let parent_dir = rust_path.parent()
                        .ok_or_else(|| format!("Cannot get parent directory of: {}", path))?;

                    let rust_target_dir = parent_dir
                        .join("target/rust-ffi")
                        .join(format!("{}_ffi", lib_name_base))
                        .join("target/release");
                    let lib_path = rust_target_dir.join(&lib_name);

                    if lib_path.exists() {
                        rust_ffi_libs.push(lib_path);
                    } else {
                        eprintln!("   ⚠ Rust FFI library not found: {}", lib_path.display());
                    }
                }
            }
        }
    }

    // Extract frameworks from [link] section (macOS)
    let mut frameworks = Vec::new();
    if let Some(link_section) = manifest.get("link").and_then(|l| l.as_table()) {
        if let Some(fws) = link_section.get("frameworks").and_then(|f| f.as_array()) {
            for fw in fws {
                if let Some(fw_str) = fw.as_str() {
                    frameworks.push(fw_str.to_string());
                }
            }
        }
    }

    // Extract dependencies
    if let Some(deps) = manifest.get("dependencies").and_then(|d| d.as_table()) {
        for (_name, dep_value) in deps.iter() {
            // Check if it's a path dependency pointing to .clip file
            let clip_path = if let Some(path_table) = dep_value.as_table() {
                if let Some(path) = path_table.get("path").and_then(|p| p.as_str()) {
                    if path.ends_with(".clip") {
                        Some(path.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(clip_path) = clip_path {
                // Extract .clip package
                let mut package = extract_clip_package(&clip_path)?;

                // Attach Rust FFI libraries to this package
                package.rust_ffi_libs = rust_ffi_libs.clone();

                // Attach frameworks from [link] section
                package.frameworks = frameworks.clone();

                // Discover modules from source directory (if available)
                match discover_clip_modules(&clip_path, &package.name) {
                    Ok(modules) => {
                        if !modules.is_empty() {
                            println!("   → Found {} modules in {}", modules.len(), package.name);
                            package.modules = modules;
                        }
                    }
                    Err(e) => {
                        // Non-fatal: source directory might not be available
                        eprintln!("   ⚠ Could not discover modules for {}: {}", package.name, e);
                    }
                }

                // Build as dynamic library for REPL
                match build_dylib_for_package(&package) {
                    Ok(dylib_path) => {
                        // Load the dynamic library
                        match load_dynamic_library(&dylib_path) {
                            Ok(lib) => {
                                loaded_libs.push(lib);
                                packages.push(package);
                            }
                            Err(e) => {
                                eprintln!("   ⚠ Failed to load {}: {}", package.name, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("   ⚠ Failed to build {} for REPL: {}", package.name, e);
                    }
                }
            }
        }
    }

    Ok((loaded_libs, packages))
}

fn extract_clip_package(clip_path: &str) -> Result<ClipPackage, String> {
    use std::fs;
    use std::path::PathBuf;

    let clip_path_buf = PathBuf::from(clip_path);
    if !clip_path_buf.exists() {
        return Err(format!(".clip file not found: {}", clip_path));
    }

    // Create temp directory for extraction
    let temp_dir = std::env::temp_dir().join(format!(
        "clorus-clip-{}",
        clip_path_buf.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
    ));

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to clean temp dir: {}", e))?;
    }
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    // Extract ZIP archive
    let file = fs::File::open(&clip_path_buf)
        .map_err(|e| format!("Failed to open .clip file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read .clip archive: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;
        let outpath = temp_dir.join(file.name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir: {}", e))?;
            }
            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
        }
    }

    // Read clip.toml
    let clip_toml_path = temp_dir.join("clip.toml");
    let clip_toml_content = fs::read_to_string(&clip_toml_path)
        .map_err(|e| format!("Failed to read clip.toml: {}", e))?;
    let clip_manifest: toml::Value = toml::from_str(&clip_toml_content)
        .map_err(|e| format!("Failed to parse clip.toml: {}", e))?;

    let name = clip_manifest.get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .ok_or("Missing package name in clip.toml")?
        .to_string();

    let version = clip_manifest.get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .ok_or("Missing package version in clip.toml")?
        .to_string();

    // Find object file
    let lib_dir = temp_dir.join("lib");
    let mut object_path = None;

    if lib_dir.exists() {
        for entry_result in fs::read_dir(&lib_dir)
            .map_err(|e| format!("Failed to read lib directory: {}", e))? {
            let entry = entry_result.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if let Some(ext) = path.extension() {
                if ext == "o" {
                    object_path = Some(path);
                    break;
                }
            }
        }
    }

    Ok(ClipPackage {
        name,
        version,
        object_path,
        rust_ffi_libs: Vec::new(),  // Will be populated by caller
        frameworks: Vec::new(),  // Will be populated by caller
        modules: Vec::new(),  // Will be populated by caller after module discovery
        clip_path: clip_path.to_string(),  // Store original path for module discovery
    })
}

/// Discover modules from source directory for a .clip package
/// Returns list of module paths like ["components.button", "components.textfield", "utils.text"]
fn discover_clip_modules(clip_path: &str, package_name: &str) -> Result<Vec<String>, String> {
    use std::fs;
    use std::path::{Path, PathBuf};

    let clip_path_buf = PathBuf::from(clip_path);

    // Try to find source directory
    // .clip is usually in dist/, source is in ../package-name/src/
    let clip_dir = clip_path_buf.parent()
        .ok_or_else(|| format!("Cannot get parent directory of: {}", clip_path))?;

    // Go up one directory from dist/
    let workspace_dir = clip_dir.parent()
        .ok_or_else(|| format!("Cannot get workspace directory from: {}", clip_dir.display()))?;

    // Look for package-name/src/ directory
    let src_dir = workspace_dir.join(package_name).join("src");

    if !src_dir.exists() {
        // Source directory not found - this is OK, just return empty list
        return Ok(Vec::new());
    }

    println!("   → Scanning {} for modules...", src_dir.display());

    // Recursively scan for .clrs files
    let mut modules = Vec::new();
    scan_directory_for_modules(&src_dir, &src_dir, &mut modules)?;

    Ok(modules)
}

/// Recursively scan directory for .clrs files and build module paths
fn scan_directory_for_modules(
    base_dir: &Path,
    current_dir: &Path,
    modules: &mut Vec<String>
) -> Result<(), String> {
    use std::fs;

    let entries = fs::read_dir(current_dir)
        .map_err(|e| format!("Failed to read directory {}: {}", current_dir.display(), e))?;

    for entry_result in entries {
        let entry = entry_result
            .map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively scan subdirectory
            scan_directory_for_modules(base_dir, &path, modules)?;
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "clrs" {
                    // Convert file path to module path
                    // e.g., src/components/button.clrs -> components.button
                    if let Ok(rel_path) = path.strip_prefix(base_dir) {
                        let module_path = rel_path
                            .with_extension("") // Remove .clrs
                            .to_string_lossy()
                            .replace('/', ".")
                            .replace('\\', ".");

                        // Skip lib.clrs (entry point)
                        if module_path != "lib" {
                            modules.push(module_path);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn build_dylib_for_package(package: &ClipPackage) -> Result<std::path::PathBuf, String> {
    use std::fs;
    use std::path::PathBuf;
    const REPL_DYLIB_CACHE_VERSION: &str = "repl-link-v2-dynamic-lookup";

    // Determine cache directory (.repl/ in current directory)
    let cache_dir = PathBuf::from(".repl").join(&package.name);
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create .repl cache directory: {}", e))?;

    // Determine dynamic library name based on platform
    #[cfg(target_os = "macos")]
    let dylib_name = format!("lib{}.dylib", package.name.replace('-', "_"));

    #[cfg(target_os = "linux")]
    let dylib_name = format!("lib{}.so", package.name.replace('-', "_"));

    #[cfg(target_os = "windows")]
    let dylib_name = format!("{}.dll", package.name.replace('-', "_"));

    let dylib_path = cache_dir.join(&dylib_name);
    let cache_version_path = cache_dir.join(".cache-version");

    // Check if already cached with current linker strategy.
    // If version changed, force rebuild to avoid stale dylibs linked with old runtime settings.
    if dylib_path.exists() {
        let cache_version = fs::read_to_string(&cache_version_path).ok();
        if cache_version.as_deref() == Some(REPL_DYLIB_CACHE_VERSION) {
            return Ok(dylib_path);
        }
    }

    // Get object file from package
    let object_path = package.object_path.as_ref()
        .ok_or_else(|| format!("No object file in .clip package: {}", package.name))?;

    println!("   Building {} for REPL...", package.name);

    // Link as dynamic library
    let mut link_cmd = std::process::Command::new("cc");
    link_cmd
        .arg("-shared")
        .arg(object_path)
        .arg("-o").arg(&dylib_path)
        .arg("-lc++");

    #[cfg(not(target_os = "macos"))]
    {
        // Non-macOS targets still require an explicit runtime library.
        let runtime_lib = find_runtime_lib()?;
        link_cmd.arg(runtime_lib);
    }

    // Add Rust FFI libraries
    for rust_lib in &package.rust_ffi_libs {
        link_cmd.arg(rust_lib);
    }

    #[cfg(target_os = "macos")]
    {
        link_cmd.arg("-dynamiclib");
        // Important: do NOT statically link clorus runtime into REPL clip dylibs.
        // We want unresolved clorus_* symbols to bind to the host clorus process
        // so retain/release/value lifetimes use a single runtime instance.
        link_cmd.arg("-Wl,-undefined,dynamic_lookup");

        // Ensure init function is not stripped by the linker
        // We export all symbols (default for dylib) but explicitly keep the init function
        let init_symbol = format!("_clorus_{}_init", package.name.replace('-', "_"));
        link_cmd.arg(format!("-Wl,-u,{}", init_symbol));

        // Always link CoreFoundation and Security (base macOS requirements)
        link_cmd.arg("-framework").arg("CoreFoundation");
        link_cmd.arg("-framework").arg("Security");
        // Add frameworks from [link] section in Clorus.toml
        for framework in &package.frameworks {
            link_cmd.arg("-framework").arg(framework);
        }
    }

    let status = link_cmd
        .status()
        .map_err(|e| format!("Failed to run linker: {}", e))?;

    if !status.success() {
        return Err(format!("Failed to link dynamic library for {}", package.name));
    }

    // Record cache strategy/version for future reuse.
    let _ = fs::write(&cache_version_path, REPL_DYLIB_CACHE_VERSION);

    Ok(dylib_path)
}

fn find_runtime_lib() -> Result<String, String> {
    let debug = std::env::var("CLORUS_DEBUG_RUNTIME").is_ok();
    // First, try CLORUS_HOME/lib (installed layout)
    if let Ok(clorus_home) = std::env::var("CLORUS_HOME") {
        let lib_dir = std::path::Path::new(&clorus_home).join("lib");
        let candidates = [
            lib_dir.join("libclorus_runtime.a"),
            lib_dir.join("libclorus_runtime.dylib"),
            lib_dir.join("libclorus_runtime.rlib"),
        ];
        if debug {
            eprintln!("   [DEBUG] CLORUS_HOME/lib = {}", lib_dir.display());
        }
        for path in candidates {
            if debug {
                eprintln!("   [DEBUG] Checking {}", path.display());
            }
            if path.exists() {
                if debug {
                    eprintln!("   [DEBUG] Found runtime: {}", path.display());
                }
                return Ok(path.to_string_lossy().to_string());
            }
        }
    }

    // Next, try relative to clorus executable (same directory)
    if let Ok(exe_path) = std::env::current_exe() {
        // Resolve symlinks
        let exe_path = if let Ok(canonical) = exe_path.canonicalize() {
            canonical
        } else {
            exe_path
        };

        if let Some(exe_dir) = exe_path.parent() {
            // Check same directory as executable (.a static library)
            let runtime_path = exe_dir.join("libclorus_runtime.a");
            if runtime_path.exists() {
                return Ok(runtime_path.to_string_lossy().to_string());
            }

            // Check dylib in same directory
            let runtime_dylib = exe_dir.join("libclorus_runtime.dylib");
            if runtime_dylib.exists() {
                return Ok(runtime_dylib.to_string_lossy().to_string());
            }

            // Check .rlib (Rust library)
            let runtime_rlib = exe_dir.join("libclorus_runtime.rlib");
            if runtime_rlib.exists() {
                return Ok(runtime_rlib.to_string_lossy().to_string());
            }
        }
    }

    // Try CLORUS_HOME/lib (installed layout)
    if let Ok(clorus_home) = std::env::var("CLORUS_HOME") {
        let lib_dir = std::path::Path::new(&clorus_home).join("lib");
        let candidates = [
            lib_dir.join("libclorus_runtime.a"),
            lib_dir.join("libclorus_runtime.dylib"),
            lib_dir.join("libclorus_runtime.rlib"),
        ];
        for path in candidates {
            if path.exists() {
                return Ok(path.to_string_lossy().to_string());
            }
        }
    }

    // Search for libclorus_runtime.* in common build locations
    let search_paths = vec![
        "target/release/deps/libclorus_runtime.a",
        "target/release/libclorus_runtime.a",
        "target/debug/deps/libclorus_runtime.a",
        "target/debug/libclorus_runtime.a",
        "../target/release/deps/libclorus_runtime.a",
        "../target/release/libclorus_runtime.a",
        "../target/debug/deps/libclorus_runtime.a",
        "../target/debug/libclorus_runtime.a",
        "../../target/release/deps/libclorus_runtime.a",
        "../../target/release/libclorus_runtime.a",
        "../../target/debug/deps/libclorus_runtime.a",
        "../../target/debug/libclorus_runtime.a",
        "../../../target/release/deps/libclorus_runtime.a",
        "../../../target/release/libclorus_runtime.a",
        "../../../target/debug/deps/libclorus_runtime.a",
        "../../../target/debug/libclorus_runtime.a",
        "target/release/deps/libclorus_runtime.dylib",
        "target/release/libclorus_runtime.dylib",
        "target/debug/deps/libclorus_runtime.dylib",
        "target/debug/libclorus_runtime.dylib",
        "../target/release/deps/libclorus_runtime.dylib",
        "../target/release/libclorus_runtime.dylib",
        "../target/debug/deps/libclorus_runtime.dylib",
        "../target/debug/libclorus_runtime.dylib",
        "../../target/release/deps/libclorus_runtime.dylib",
        "../../target/release/libclorus_runtime.dylib",
        "../../target/debug/deps/libclorus_runtime.dylib",
        "../../target/debug/libclorus_runtime.dylib",
    ];

    for path in search_paths {
        if debug {
            eprintln!("   [DEBUG] Checking {}", path);
        }
        if std::path::Path::new(path).exists() {
            if debug {
                eprintln!("   [DEBUG] Found runtime: {}", path);
            }
            return Ok(path.to_string());
        }
    }

    Err("Runtime library not found".to_string())
}
