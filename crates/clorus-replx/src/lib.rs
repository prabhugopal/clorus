//! Clorus REPL Extended (replx)
//!
//! Enhanced REPL with smart, adaptive execution that prevents blocking
//! and auto-detects execution contexts (GUI, compute, I/O, etc.)

mod detector;
mod strategy;
mod wrapper;

pub use detector::{FunctionDetector, FunctionMetadata, ExecutionPattern};
pub use strategy::{ExecutionStrategy, AdaptiveConfig, AdaptationLevel};
pub use wrapper::AdaptiveWrapper;

use clorus_repl::{ReplConfig, ReplEngine, ProjectConfig};
use clorus_repl::{load_runtime_library, load_dynamic_library, parse_interface_file};
use inkwell::context::Context;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use std::io::{self, BufRead, IsTerminal};
use std::path::Path;

#[cfg(feature = "learning")]
use std::collections::HashMap;

/// Extended REPL engine with adaptive execution
pub struct ReplX {
    config: AdaptiveConfig,
    detector: FunctionDetector,

    #[cfg(feature = "learning")]
    profiles: HashMap<String, FunctionProfile>,
}

#[cfg(feature = "learning")]
#[derive(Debug, Clone)]
struct FunctionProfile {
    call_count: u64,
    blocked_count: u64,
    learned_pattern: Option<ExecutionPattern>,
}

impl ReplX {
    /// Create a new extended REPL with smart execution
    pub fn new(config: AdaptiveConfig) -> Self {
        Self {
            config,
            detector: FunctionDetector::new(),

            #[cfg(feature = "learning")]
            profiles: HashMap::new(),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(AdaptiveConfig::default())
    }

    /// Convert to base REPL config
    pub fn to_repl_config(&self) -> ReplConfig {
        ReplConfig {
            main_thread: self.config.main_thread,
            ..Default::default()
        }
    }

    /// Analyze expression and determine if adaptation needed
    pub fn should_adapt(&self, expr: &str) -> Option<ExecutionStrategy> {
        if !self.config.enabled {
            return None;
        }

        let metadata = self.detector.analyze(expr);

        match self.config.level {
            AdaptationLevel::None => None,

            AdaptationLevel::Warn => {
                if metadata.blocks && metadata.needs_main_thread {
                    println!("⚠️  Warning: This function may block the REPL");
                    println!("💡 Consider using: clorus replx --main-thread");
                }
                None
            }

            AdaptationLevel::AutoAdapt | AdaptationLevel::Silent => {
                self.determine_strategy(&metadata)
            }
        }
    }

    fn determine_strategy(&self, metadata: &FunctionMetadata) -> Option<ExecutionStrategy> {
        // Auto-detach GUI functions that block
        if metadata.blocks && metadata.needs_main_thread && self.config.auto_detach_gui {
            if self.config.level == AdaptationLevel::AutoAdapt {
                println!("⚡ Detected: GUI function (blocks on main thread)");
                println!("✓ Auto-adapted: Spawned in detached context");
            }
            return Some(ExecutionStrategy::DetachedMainThread);
        }

        // Suggest async for long-running operations
        if metadata.estimated_duration_ms > self.config.warn_long_running_ms {
            if self.config.suggest_optimizations {
                println!("💡 Hint: Long computation detected.");
                println!("   Consider: (async {}) for background execution", metadata.name);
            }
        }

        None
    }

    #[cfg(feature = "learning")]
    pub fn learn_from_execution(&mut self, func_name: &str, blocked: bool) {
        let profile = self.profiles.entry(func_name.to_string())
            .or_insert(FunctionProfile {
                call_count: 0,
                blocked_count: 0,
                learned_pattern: None,
            });

        profile.call_count += 1;
        if blocked {
            profile.blocked_count += 1;
        }

        // Learn: if blocked >3 times, remember to detach
        if profile.blocked_count > 3 && profile.learned_pattern.is_none() {
            profile.learned_pattern = Some(ExecutionPattern::GUI);
            println!("📝 Learned: {} → auto-detach", func_name);
        }
    }
}

/// Run the extended REPL with adaptive features
pub fn run() -> Result<(), String> {
    run_with_config(AdaptiveConfig::default())
}

/// Run with custom configuration
pub fn run_with_config(config: AdaptiveConfig) -> Result<(), String> {
    // Load project configuration if available
    let project = ProjectConfig::load();

    // Create adaptive wrapper
    let wrapper = AdaptiveWrapper::new(config.clone());

    // Banner
    if config.level != AdaptationLevel::Silent {
        println!("╔════════════════════════════════════╗");
        println!("║  Clorus REPLX v0.1.0               ║");
        println!("║  Smart, Adaptive REPL              ║");
        println!("╚════════════════════════════════════╝");
        println!();

        // Display project info if in a project directory
        if let Some(ref proj_config) = project {
            println!("📦 Project: {} v{}", proj_config.package.name, proj_config.package.version);
            println!("📂 Namespace: {}", proj_config.namespace());
            println!();
        }

        if config.enabled {
            println!("🧠 Adaptive mode: Enabled");
            if config.auto_detach_gui {
                println!("   → GUI functions auto-detached");
            }
            println!();
        }
    }

    // Load clorus-runtime library FIRST (required for Value* operations)
    let _runtime_lib = match load_runtime_library() {
        Ok(_lib) => {
            // Keep library loaded for the duration of REPL
        }
        Err(e) => {
            if config.level != AdaptationLevel::Silent {
                eprintln!("⚠ Warning: Could not load runtime library: {}", e);
                eprintln!("  The REPL may not function correctly.");
            }
        }
    };

    if config.level != AdaptationLevel::Silent {
        println!();
    }

    // Create base REPL engine
    let context = Context::create();
    let mut repl_engine = ReplEngine::new(&context);

    // Auto-load core stdlib (clorus.core)
    let mut core_loaded = false;
    let mut core_stdlib_path = None;

    // Try to find core.clr in CLORUS_HOME/stdlib or ~/.clorus/stdlib
    if let Ok(clorus_home) = std::env::var("CLORUS_HOME") {
        let path = Path::new(&clorus_home).join("stdlib").join("core.clr");
        if path.exists() {
            core_stdlib_path = Some(path);
        }
    }

    if core_stdlib_path.is_none() {
        if let Ok(home) = std::env::var("HOME") {
            let path = Path::new(&home).join(".clorus").join("stdlib").join("core.clr");
            if path.exists() {
                core_stdlib_path = Some(path);
            }
        }
    }

    if let Some(core_path) = core_stdlib_path {
        match std::fs::read_to_string(&core_path) {
            Ok(source) => {
                match repl_engine.load_stdlib_batch(source) {
                    Ok(count) => {
                        if config.level != AdaptationLevel::Silent {
                            println!("✓ Loaded clorus.core ({} functions)", count);
                        }
                        core_loaded = true;
                    }
                    Err(e) => {
                        if config.level != AdaptationLevel::Silent {
                            eprintln!("⚠ Error loading stdlib: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                if config.level != AdaptationLevel::Silent {
                    eprintln!("⚠ Could not read stdlib: {}", e);
                }
            }
        }
    }

    if !core_loaded && config.level != AdaptationLevel::Silent {
        println!("⚠ clorus.core not loaded (stdlib not found)");
    }

    if config.level != AdaptationLevel::Silent {
        println!();
    }

    // Load project Rust FFI libraries if in a project directory
    let mut _rust_ffi_libs = Vec::new();
    if project.is_some() {
        if let Ok(manifest_str) = std::fs::read_to_string("Clorus.toml") {
            if let Ok(manifest) = toml::from_str::<toml::Value>(&manifest_str) {
                if let Some(rust_deps) = manifest.get("rust-dependencies").and_then(|v| v.as_table()) {
                    if config.level != AdaptationLevel::Silent {
                        println!("Loading Rust FFI libraries...");
                    }

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
                                        if config.level != AdaptationLevel::Silent {
                                            println!("✓ Loaded: {}", dep_name);
                                        }
                                        _rust_ffi_libs.push(lib);
                                        loaded = true;

                                        // Load and parse interface file to register functions
                                        let interface_path = format!("interfaces/{}.clorus-ffi", dep_name);
                                        if let Ok(interface_content) = std::fs::read_to_string(&interface_path) {
                                            match parse_interface_file(&interface_content, dep_name) {
                                                Ok(mut rust_lib) => {
                                                    // Register library with full rust.* module name for lookup
                                                    rust_lib.name = format!("rust.{}", dep_name.replace('-', "_"));
                                                    if config.level != AdaptationLevel::Silent {
                                                        println!("  → Registered {} functions from {}", rust_lib.functions.len(), dep_name);
                                                    }

                                                    // Register with both full module name and library name
                                                    repl_engine.register_rust_library(rust_lib.clone());
                                                    rust_lib.name = dep_name.to_string();
                                                    repl_engine.register_rust_library(rust_lib);
                                                }
                                                Err(e) => {
                                                    if config.level != AdaptationLevel::Silent {
                                                        eprintln!("  ⚠ Failed to parse interface file: {}", e);
                                                    }
                                                }
                                            }
                                        }

                                        break;
                                    }
                                    Err(e) => {
                                        if config.level != AdaptationLevel::Silent {
                                            eprintln!("  ⚠ Failed to load {}: {}", lib_path.display(), e);
                                        }
                                    }
                                }
                            }
                        }

                        if !loaded && config.level != AdaptationLevel::Silent {
                            eprintln!("  ⚠ Library not found: {}", dep_name);
                        }
                    }

                    if config.level != AdaptationLevel::Silent {
                        println!();
                    }
                }
            }
        }
    }

    // Load project entry file if in a project directory
    if project.is_some() {
        if let Some(ref proj_config) = project {
            if std::path::Path::new(&proj_config.build.entry).exists() {
                if config.level != AdaptationLevel::Silent {
                    println!("Loading {}...", proj_config.build.entry);
                }
                match std::fs::read_to_string(&proj_config.build.entry) {
                    Ok(source) => {
                        // Split source into individual top-level forms and evaluate each
                        let lines: Vec<&str> = source.lines().collect();
                        let mut current_form = String::new();
                        let mut paren_depth = 0;
                        let mut in_string = false;
                        let mut escape_next = false;
                        let mut forms_loaded = 0;
                        let mut had_error = false;

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
                                        if config.level != AdaptationLevel::Silent {
                                            eprintln!("⚠ Error loading form: {}", e);
                                        }
                                        had_error = true;
                                    }
                                }
                                current_form.clear();
                            }
                        }

                        if config.level != AdaptationLevel::Silent {
                            if !had_error {
                                println!("✓ Project loaded ({} forms)", forms_loaded);
                            } else {
                                println!("⚠ Project loaded with errors ({} forms)", forms_loaded);
                            }
                        }
                    }
                    Err(e) => {
                        if config.level != AdaptationLevel::Silent {
                            eprintln!("⚠ Could not read {}: {}", proj_config.build.entry, e);
                        }
                    }
                }
                if config.level != AdaptationLevel::Silent {
                    println!();
                }
            }
        }
    }

    // Detect if stdin is a TTY (interactive) or piped/redirected
    let is_interactive = io::stdin().is_terminal();

    if is_interactive {
        run_interactive_adaptive(repl_engine, wrapper, config)
    } else {
        run_piped_adaptive(repl_engine, wrapper)
    }
}

/// Run adaptive REPL in interactive mode
fn run_interactive_adaptive(
    mut repl_engine: ReplEngine,
    wrapper: AdaptiveWrapper,
    config: AdaptiveConfig,
) -> Result<(), String> {
    use clorus_repl::format_result;

    // Create rustyline editor
    let mut rl = Editor::<(), DefaultHistory>::new().unwrap();

    // Load history
    let history_file = std::env::var("HOME")
        .map(|home| format!("{}/.clorus_history", home))
        .unwrap_or_else(|_| ".clorus_history".to_string());
    let _ = rl.load_history(&history_file);

    loop {
        let prompt = format!("{}λ> ", repl_engine.current_namespace());
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(input);

                // Handle commands
                if input.starts_with(':') {
                    match input {
                        ":quit" | ":q" => {
                            println!("Goodbye!");
                            break;
                        }
                        _ => {
                            println!("Unknown command: {}", input);
                            continue;
                        }
                    }
                }

                // Transform expression if needed
                let (transformed, was_adapted) = wrapper.transform_expr(input);

                // Evaluate
                match repl_engine.eval(&transformed) {
                    Ok(result) => {
                        let output = format_result(&result, repl_engine.current_namespace());
                        if was_adapted && config.level == AdaptationLevel::AutoAdapt {
                            println!("→ {}", output);
                        } else {
                            println!("{}", output);
                        }
                    }
                    Err(e) => println!("Error: {}", e),
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

    // Save history
    let _ = rl.save_history(&history_file);

    Ok(())
}

/// Run adaptive REPL in piped mode
fn run_piped_adaptive(
    mut repl_engine: ReplEngine,
    wrapper: AdaptiveWrapper,
) -> Result<(), String> {
    use clorus_repl::format_result;

    let stdin = io::stdin();
    let reader = stdin.lock();

    for line in reader.lines() {
        match line {
            Ok(input) => {
                let input = input.trim();

                if input.is_empty() || input.starts_with(':') {
                    continue;
                }

                // Transform if needed
                let (transformed, _) = wrapper.transform_expr(input);

                // Evaluate
                match repl_engine.eval(&transformed) {
                    Ok(result) => {
                        let output = format_result(&result, repl_engine.current_namespace());
                        println!("{}", output);
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }

    Ok(())
}
