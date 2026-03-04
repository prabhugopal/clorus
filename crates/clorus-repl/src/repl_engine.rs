/// REPL-specific execution engine that manages state across expressions
///
/// Strategy: Keep all source code and recompile everything for each expression.
/// This ensures variables and functions persist across REPL lines.
use clorus::{parse, CodeGen, ModuleLoader};
use clorus::codegen::RustLibrary;
use clorus_codegen::namespace_context::{ImportBinding, NamespaceContext};
use clorus_syntax::{Expr, RequireSpec};
use clorus_syntax::macros::{expand_macros_with_registry, MacroRegistry};
use inkwell::context::Context;
use inkwell::OptimizationLevel;
use std::collections::{HashSet, HashMap};
use std::env;

fn repl_debug_enabled() -> bool {
    env::var("CLORUS_DEBUG_REPL").is_ok()
}

/// Result of evaluating an expression in the REPL
pub struct EvalResult {
    pub value: *mut u8,
    pub kind: EvalKind,
}

/// Type of expression that was evaluated
pub enum EvalKind {
    /// Regular expression - display the value
    Value,
    /// (def name ...) - display #'namespace/name
    Def(String),
    /// (defn name ...) - display #'namespace/name
    Defn(String),
    /// (defmacro name ...) - display #'namespace/name
    Defmacro(String),
    /// (ns name) - display nil
    Namespace,
    /// (require ...) or (use ...) - display nil
    Import,
}

/// Namespaces that are auto-imported into every namespace (like clojure.core in Clojure)
const AUTO_IMPORT_NAMESPACES: &[&str] = &["clorus.core"];

pub struct ReplEngine<'ctx> {
    context: &'ctx Context,
    /// Interactive expressions entered by user
    history: Vec<String>,
    /// Project initialization forms (compiled always, def/defn executed always)
    init_forms: Vec<String>,
    expr_count: usize,
    /// Track which modules have been loaded
    loaded_modules: HashSet<String>,
    /// Current namespace context
    namespace: NamespaceContext,
    /// Module loader for (require ...) support
    module_loader: ModuleLoader,
    /// Rust FFI libraries registered with the REPL
    rust_libraries: Vec<RustLibrary>,
    /// .clip namespaces registered with the REPL (from loaded .clip packages)
    clip_namespaces: HashSet<String>,
    /// Symbol registry: namespace -> set of defined symbols
    /// Tracks what symbols (functions, vars) exist in each namespace
    symbol_registry: HashMap<String, HashSet<String>>,
    /// Stdlib loaded flag - stdlib is loaded once and never recompiled
    stdlib_loaded: bool,
    /// Parsed module expressions (from required modules) to include in every evaluation
    /// Stored as (namespace, aliases, expr) tuples to preserve namespace context and aliases
    module_exprs: Vec<(String, HashMap<String, String>, Expr)>,
    /// Persistent REPL macro registry so `(defmacro ...)` is available across inputs
    macro_registry: MacroRegistry,
    /// Track executed init forms for later reference
    /// Stores parsed expressions that have been successfully compiled and executed
    executed_init_exprs: Vec<(String, Expr)>,
}

impl<'ctx> ReplEngine<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        let mut engine = ReplEngine {
            context,
            history: Vec::new(),
            init_forms: Vec::new(),
            expr_count: 0,
            loaded_modules: HashSet::new(),
            namespace: NamespaceContext::new("user"),
            module_loader: ModuleLoader::new(),
            rust_libraries: Vec::new(),
            clip_namespaces: HashSet::new(),
            symbol_registry: HashMap::new(),
            stdlib_loaded: false,
            module_exprs: Vec::new(),
            macro_registry: MacroRegistry::new(),
            executed_init_exprs: Vec::new(),
        };

        // Note: Stdlib is now loaded separately in lib.rs to avoid O(n²) recompilation

        engine
    }

    /// Load and compile stdlib ONCE (does not add to init_forms)
    /// This is called from lib.rs during REPL startup
    pub fn load_stdlib_batch(&mut self, stdlib_source: String) -> Result<usize, String> {
        if self.stdlib_loaded {
            return Ok(0); // Already loaded
        }

        // Parse all forms in the stdlib file
        let exprs = parse(&stdlib_source)?;
        if exprs.is_empty() {
            return Ok(0);
        }

        // We replay stdlib forms through executed_init_exprs so they use the same
        // codepath as project init/history defs (stable in REPL).
        let mut current_ns = "clorus.core".to_string();
        let mut symbols_registered = 0;
        let mut macro_registry = MacroRegistry::new();

        for expr in exprs {
            let expanded_expr = expand_macros_with_registry(&expr, &mut macro_registry);

            match &expanded_expr {
                Expr::Ns { name, .. } => {
                    current_ns = name.clone();
                }
                Expr::Require { .. } | Expr::Use { .. } | Expr::Declare { .. } => {
                    // Skip directives; no runtime evaluation needed here.
                }
                Expr::Def { name, .. } | Expr::Defn { name, .. } => {
                    self.register_symbol(&current_ns, name);
                    symbols_registered += 1;
                    self.executed_init_exprs.push((current_ns.clone(), expr.clone()));
                }
                _ => {
                    self.executed_init_exprs.push((current_ns.clone(), expr.clone()));
                }
            }
        }

        self.stdlib_loaded = true;

        // Mark clorus.core as loaded to prevent re-loading when required
        self.loaded_modules.insert("clorus.core".to_string());

        // After stdlib is loaded, refresh user namespace to get core symbols
        self.set_namespace("user");

        Ok(symbols_registered)
    }

    /// Load stdlib symbols only (no JIT compilation).
    /// Useful when clorus-core dylib provides implementations.
    pub fn load_stdlib_symbols_only(&mut self, stdlib_source: String) -> Result<usize, String> {
        if self.stdlib_loaded {
            return Ok(0);
        }

        let exprs = parse(&stdlib_source)?;
        if exprs.is_empty() {
            return Ok(0);
        }

        let mut current_ns = "clorus.core".to_string();
        let mut symbols_registered = 0;
        let mut macro_registry = MacroRegistry::new();

        for expr in &exprs {
            let expanded_expr = expand_macros_with_registry(expr, &mut macro_registry);

            if let Expr::Ns { name, .. } = &expanded_expr {
                current_ns = name.clone();
                continue;
            }

            let symbol_name = match &expanded_expr {
                Expr::Def { name, .. } => Some(name.clone()),
                Expr::Defn { name, .. } => Some(name.clone()),
                _ => None,
            };

            if let Some(name) = symbol_name {
                self.register_symbol(&current_ns, &name);
                symbols_registered += 1;
            }
        }

        // Do not mark stdlib as fully loaded here. This path only registers names.
        // Full callable stdlib defs are provided by load_stdlib_batch().
        self.loaded_modules.insert("clorus.core".to_string());
        self.set_namespace("user");

        Ok(symbols_registered)
    }

    /// Get the current namespace name
    pub fn current_namespace(&self) -> &str {
        &self.namespace.current
    }

    /// Register a Rust FFI library with the REPL
    /// This makes the library's functions available for all future evaluations
    pub fn register_rust_library(&mut self, lib: RustLibrary) {
        self.rust_libraries.push(lib);
    }

    /// Register a .clip namespace with the REPL
    /// This tells the compiler that functions from this namespace will be available at runtime
    pub fn register_clip_namespace(&mut self, namespace: &str) {
        self.clip_namespaces.insert(namespace.to_string());
    }

    /// Switch to a new namespace
    pub fn set_namespace(&mut self, namespace: &str) {
        self.namespace = NamespaceContext::new(namespace);

        // Auto-import configured namespaces (like clojure.core in Clojure)
        // This makes stdlib functions available everywhere without explicit require
        for auto_ns in AUTO_IMPORT_NAMESPACES {
            if namespace != *auto_ns {
                if let Some(symbols) = self.get_namespace_symbols(auto_ns) {
                    for symbol in symbols.clone() {
                        self.namespace.imports.insert(
                            symbol.clone(),
                            ImportBinding {
                                namespace: auto_ns.to_string(),
                                symbol,
                            },
                        );
                    }
                }
            }
        }
    }

    /// Register a symbol in the symbol registry for the given namespace
    fn register_symbol(&mut self, namespace: &str, symbol: &str) {
        self.symbol_registry
            .entry(namespace.to_string())
            .or_insert_with(HashSet::new)
            .insert(symbol.to_string());
    }

    /// Get all symbols defined in a namespace
    fn get_namespace_symbols(&self, namespace: &str) -> Option<&HashSet<String>> {
        self.symbol_registry.get(namespace)
    }

    /// Convert namespace to file path
    /// demos.shapes-demo → src/demos/shapes-demo.clrs
    /// clorus.core → $CLORUS_HOME/stdlib/clorus/core.clr or ~/.clorus/stdlib/clorus/core.clr (global stdlib)
    fn namespace_to_path(&self, namespace: &str) -> Result<std::path::PathBuf, String> {
        use std::path::PathBuf;
        use std::env;

        let parts: Vec<&str> = namespace.split('.').collect();

        if parts.is_empty() {
            return Err(format!("Invalid namespace: {}", namespace));
        }

        // Check if this is a clorus.* namespace (stdlib)
        if parts[0] == "clorus" {
            // Clojure-style mapping: clorus.set -> stdlib/clorus/set.clr
            // Search CLORUS_HOME/stdlib first, then ~/.clorus/stdlib.
            let stdlib_paths = vec![
                env::var("CLORUS_HOME").ok().map(|home| PathBuf::from(home).join("stdlib")),
                env::var("HOME").ok().map(|home| PathBuf::from(home).join(".clorus/stdlib")),
            ];

            // Convert clorus.string.utils -> clorus/string/utils.clr
            let mut ns_path = PathBuf::new();
            for part in &parts[..parts.len() - 1] {
                ns_path = ns_path.join(part);
            }
            ns_path = ns_path.join(format!("{}.clr", parts[parts.len() - 1]));

            for stdlib_dir in stdlib_paths.into_iter().flatten() {
                let stdlib_path = stdlib_dir.join(&ns_path);
                if stdlib_path.exists() {
                    return Ok(stdlib_path);
                }
            }

            return Err(format!(
                "Stdlib module not found: {} (expected {} in CLORUS_HOME/stdlib or ~/.clorus/stdlib)",
                namespace,
                ns_path.to_string_lossy()
            ));
        }

        // Get current working directory as base path
        let base_path = env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;

        // Build path: src/demos/shapes-demo.clrs
        let mut path = base_path.join("src");

        // Add directory components
        for part in &parts[..parts.len()-1] {
            path = path.join(part);
        }

        // Add file name
        let file_name = format!("{}.clrs", parts[parts.len()-1]);
        path = path.join(file_name);

        if !path.exists() {
            return Err(format!("Module file not found: {} (looking for {})", namespace, path.display()));
        }

        Ok(path)
    }

    /// Recursively load a module and its dependencies
    /// Returns parsed expressions with their namespace (in dependency order)
    /// Excludes require/ns statements which are already processed
    fn load_module_recursive(&mut self, namespace: &str) -> Result<Vec<(String, HashMap<String, String>, Expr)>, String> {
        use std::fs;

        if repl_debug_enabled() {
            eprintln!("DEBUG load_module_recursive: Loading {}", namespace);
        }

        // Skip if already loaded
        if self.loaded_modules.contains(namespace) {
            if repl_debug_enabled() {
                eprintln!("DEBUG load_module_recursive: {} already loaded, skipping", namespace);
            }
            return Ok(Vec::new());
        }

        // Skip if from .clip package
        let is_clip = self.clip_namespaces.iter().any(|prefix| {
            namespace.starts_with(prefix)
        });
        if is_clip {
            if repl_debug_enabled() {
                eprintln!("DEBUG load_module_recursive: {} is from .clip package, skipping source load", namespace);
            }
            return Ok(Vec::new());
        }

        // Mark as loaded (before recursion to handle circular deps)
        self.loaded_modules.insert(namespace.to_string());

        // Convert namespace to file path
        let file_path = self.namespace_to_path(namespace)?;

        // Read the file
        let source = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read module {}: {}", namespace, e))?;

        // Parse the source
        let exprs = parse(&source)?;

        // Recursively load dependencies first
        let mut all_exprs = Vec::new();

        // Track this module's own aliases (from its ns declaration)
        let mut module_aliases = HashMap::new();

        for expr in &exprs {
            // Check top-level Expr::Require
            if let Expr::Require { specs } = expr {
                for spec in specs {
                    let dep_exprs = self.load_module_recursive(&spec.module)?;
                    all_exprs.extend(dep_exprs);
                }
            }

            // Check Expr::Ns for embedded :require and :rust clauses
            if let Expr::Ns { requires, rust_imports, .. } = expr {
                for spec in requires {
                    let dep_exprs = self.load_module_recursive(&spec.module)?;
                    all_exprs.extend(dep_exprs);

                    // Track this module's alias for this require
                    if let Some(alias) = &spec.alias {
                        module_aliases.insert(alias.clone(), spec.module.clone());
                    }
                }

                // Process rust imports - add rust.* aliases for this module
                for rust_import in rust_imports {
                    if let Some(alias) = &rust_import.alias {
                        let rust_module = format!("rust.{}", rust_import.library.replace('-', "_"));
                        module_aliases.insert(alias.clone(), rust_module);
                    }
                }
            }
        }

        // Add this module's expressions AFTER its dependencies
        // BUT exclude Require and Ns expressions (already processed)
        // Tag each expression with its namespace AND aliases
        for expr in exprs {
            match expr {
                Expr::Require { .. } | Expr::Ns { .. } => {
                    // Skip - already processed
                }
                _ => {
                    // Add expression with its namespace and aliases
                    all_exprs.push((namespace.to_string(), module_aliases.clone(), expr));
                }
            }
        }

        Ok(all_exprs)
    }

    /// Process a require spec and update namespace context
    fn process_require(&mut self, spec: &RequireSpec) {
        // Add namespace alias if specified
        if let Some(alias) = &spec.alias {
            self.namespace.aliases.insert(alias.clone(), spec.module.clone());
        }

        // Handle :refer :all - import all symbols from the module
        if spec.refer_all {
            // Look up all symbols in the target namespace
            if let Some(symbols) = self.get_namespace_symbols(&spec.module).cloned() {
                for symbol in symbols {
                    self.namespace.imports.insert(
                        symbol.clone(),
                        ImportBinding {
                            namespace: spec.module.clone(),
                            symbol,
                        },
                    );
                }
            }
        } else {
            // Add specific referred symbols to imports
            for symbol in &spec.refer {
                self.namespace.imports.insert(
                    symbol.clone(),
                    ImportBinding {
                        namespace: spec.module.clone(),
                        symbol: symbol.clone(),
                    },
                );
            }
            for (source_symbol, local_symbol) in &spec.rename {
                self.namespace.imports.insert(
                    local_symbol.clone(),
                    ImportBinding {
                        namespace: spec.module.clone(),
                        symbol: source_symbol.clone(),
                    },
                );
            }
        }
    }

    /// Evaluate a project initialization form (stores separately, executes once)
    pub fn eval_init(&mut self, input: &str) -> Result<EvalResult, String> {
        // Parse and register project forms without executing runtime code.
        // We replay definitions in eval_internal when building the fresh JIT module.
        let exprs = parse(input)?;
        if exprs.is_empty() {
            return Err("No expression to evaluate".to_string());
        }

        let mut current_ns = self.namespace.current.clone();
        let mut macro_registry = MacroRegistry::new();
        for expr in exprs {
            let expanded = expand_macros_with_registry(&expr, &mut macro_registry);

            match &expanded {
                Expr::Ns { name, requires, rust_imports } => {
                    self.set_namespace(name);
                    current_ns = name.clone();

                    for req_spec in requires {
                        let module_exprs = self.load_module_recursive(&req_spec.module)?;
                        self.module_exprs.extend(module_exprs);
                        self.process_require(req_spec);
                    }

                    for rust_import in rust_imports {
                        if let Some(alias) = &rust_import.alias {
                            let rust_module = format!("rust.{}", rust_import.library.replace('-', "_"));
                            self.namespace.aliases.insert(alias.clone(), rust_module);
                        }
                    }
                }
                Expr::Require { specs } => {
                    for spec in specs {
                        let module_exprs = self.load_module_recursive(&spec.module)?;
                        self.module_exprs.extend(module_exprs);
                        self.process_require(spec);
                    }
                }
                _ => {
                    self.executed_init_exprs.push((current_ns.clone(), expr.clone()));

                    if let Expr::Def { name, .. } | Expr::Defn { name, .. } = &expanded {
                        self.register_symbol(&current_ns, name);
                    }
                }
            }
        }

        Ok(EvalResult {
            value: std::ptr::null_mut(),
            kind: EvalKind::Value,
        })
    }

    /// Finalize project loading (no-op for now)
    pub fn finalize_project_load(&mut self) {
        // No-op - init_forms is already empty
        if repl_debug_enabled() {
            eprintln!("DEBUG: Project loading finalized");
        }
    }

    /// Evaluate an expression and add it to history (for interactive REPL)
    pub fn eval(&mut self, input: &str) -> Result<EvalResult, String> {
        self.eval_internal(input, true, true)
    }

    fn eval_internal(&mut self, input: &str, add_to_history: bool, execute_init_defs: bool) -> Result<EvalResult, String> {
        // Parse the new input to validate it FIRST (before adding to history)
        let exprs = parse(input)?;
        if exprs.is_empty() {
            return Err("No expression to evaluate".to_string());
        }

        // During project loading (add_to_history=false), skip re-executing init forms
        // They've already been executed once, we only need to recompile for symbol resolution
        let skip_init_execution = !execute_init_defs;

        // Determine the kind of expression for output formatting (use first expression)
        let eval_kind = match &exprs[0] {
            Expr::Def { name, .. } => EvalKind::Def(name.clone()),
            Expr::Defn { name, .. } => EvalKind::Defn(name.clone()),
            Expr::Defmacro { name, .. } => EvalKind::Defmacro(name.clone()),
            Expr::Ns { .. } => EvalKind::Namespace,
            Expr::Require { .. } | Expr::Use { .. } => EvalKind::Import,
            _ => EvalKind::Value,
        };

        // Handle special namespace commands
        match &exprs[0] {
            Expr::Ns { name, requires, rust_imports } => {
                // Switch to new namespace
                self.set_namespace(name);

                // Load and process all requires (synchronously)
                for req_spec in requires {
                    // Load the module and all its dependencies
                    let module_exprs = self.load_module_recursive(&req_spec.module)?;

                    // Add loaded module expressions to module_exprs for compilation
                    self.module_exprs.extend(module_exprs);

                    // Update namespace aliases after loading
                    self.process_require(req_spec);
                }

                // Process Rust imports
                for rust_import in rust_imports {
                    if let Some(alias) = &rust_import.alias {
                        // Convert library name to module format: "coral-gfx" -> "rust.coral_gfx"
                        let rust_module = format!("rust.{}", rust_import.library.replace('-', "_"));
                        self.namespace.aliases.insert(alias.clone(), rust_module);
                    }
                }

                // Add to history after successful processing (for interactive mode)
                if add_to_history {
                    self.history.push(input.to_string());
                }

                // Return nil (represented as null pointer for now)
                return Ok(EvalResult {
                    value: std::ptr::null_mut(),
                    kind: eval_kind,
                });
            }

            Expr::Require { specs } => {
                // Load each required module synchronously (blocking)
                // This matches Clojure REPL behavior
                for spec in specs {
                    // Load the module and all its dependencies
                    let module_exprs = self.load_module_recursive(&spec.module)?;

                    // Add loaded module expressions to module_exprs for compilation
                    self.module_exprs.extend(module_exprs);

                    // Update namespace aliases after loading
                    self.process_require(spec);
                }

                // Add to history after successful processing (for interactive mode)
                if add_to_history {
                    self.history.push(input.to_string());
                }

                // Return nil
                return Ok(EvalResult {
                    value: std::ptr::null_mut(),
                    kind: eval_kind,
                });
            }

            _ => {} // Continue with normal evaluation
        }

        // Track module loading (old Use syntax)
        if let Some(Expr::Use { module, .. }) = exprs.first() {
            self.loaded_modules.insert(module.clone());
        }

        // Create a fresh CodeGen for this evaluation
        let mut codegen = CodeGen::new(self.context, "repl_session");
        let mut macro_registry = self.macro_registry.clone();

        // Register all Rust FFI libraries
        for lib in &self.rust_libraries {
            codegen.register_rust_library(lib.clone());
            // Declare FFI functions in LLVM module
            codegen.declare_rust_library_functions(lib)?;
        }

        // Register all .clip namespaces (from loaded .clip packages)
        for namespace in &self.clip_namespaces {
            codegen.register_clip_namespace(namespace);
        }

        // Register all loaded local modules (demos.*, utils.*, etc.)
        // This allows the codegen to resolve qualified function calls like textfield/render
        for namespace in &self.loaded_modules {
            codegen.register_clip_namespace(namespace);
        }

        // Set the current namespace context
        // Convert clorus::NamespaceContext to codegen::NamespaceContext
        let codegen_ns = clorus_codegen::NamespaceContext {
            current: self.namespace.current.clone(),
            aliases: self.namespace.aliases.clone(),
            imports: self.namespace.imports.clone(),
        };
        codegen.set_namespace(codegen_ns);

        // Compile loaded module expressions (before user/project history replay)
        // These are from required modules (e.g., demos.shapes-demo)
        // Each expression is compiled with its module's namespace AND aliases to properly resolve references
        for (module_idx, (module_namespace, module_aliases, module_expr)) in self.module_exprs.iter().enumerate() {
            // Set namespace context for this module's expressions, INCLUDING ITS ALIASES
            let module_codegen_ns = clorus_codegen::NamespaceContext {
                current: module_namespace.clone(),
                aliases: module_aliases.clone(),  // Use the module's own aliases!
                imports: HashMap::new(),
            };
            codegen.set_namespace(module_codegen_ns);

            let expanded = expand_macros_with_registry(module_expr, &mut macro_registry);

            // Skip namespace declarations (already processed)
            if matches!(expanded, Expr::Ns { .. }) {
                continue;
            }

            // Skip declare statements
            if let Expr::Declare { names } = &expanded {
                for name in names {
                    codegen.add_forward_declaration(name);
                }
                continue;
            }

            let fn_name = format!("module_{}_{}", module_namespace.replace('.', "_"), module_idx);
            codegen.wrap_in_function(&expanded, &fn_name)?;
        }

        // Restore user's namespace for subsequent compilation
        let user_codegen_ns = clorus_codegen::NamespaceContext {
            current: self.namespace.current.clone(),
            aliases: self.namespace.aliases.clone(),
            imports: self.namespace.imports.clone(),
        };
        codegen.set_namespace(user_codegen_ns.clone());

        // Collect init wrappers to execute in the fresh JIT module
        let mut init_fn_names: Vec<String> = Vec::new();
        // Collect def/defn wrappers to execute (from init forms + history)
        let mut def_fn_names: Vec<String> = Vec::new();

        // Compile executed init forms (from project loading)
        // These are forms that have already been executed once
        // We recompile them (without re-executing) so their symbols are available
        if repl_debug_enabled() {
            eprintln!("DEBUG eval_internal: Compiling {} executed init forms", self.executed_init_exprs.len());
        }
        for (init_idx, (init_ns, init_expr)) in self.executed_init_exprs.iter().enumerate() {
            // Compile each executed init expression in its original namespace
            let exec_codegen_ns = clorus_codegen::NamespaceContext {
                current: init_ns.clone(),
                aliases: self.namespace.aliases.clone(),
                imports: self.namespace.imports.clone(),
            };
            codegen.set_namespace(exec_codegen_ns);

            let expanded = expand_macros_with_registry(init_expr, &mut macro_registry);

            // Skip namespace declarations and compile-time directives
            if matches!(expanded, Expr::Ns { .. } | Expr::Require { .. } | Expr::Use { .. }) {
                continue;
            }

            // Skip declare statements
            if let Expr::Declare { names } = &expanded {
                for name in names {
                    codegen.add_forward_declaration(name);
                }
                continue;
            }

            // Wrap all expressions in functions so symbol resolution works in the fresh module.
            let fn_name = format!("executed_init_{}", init_idx);
            codegen.wrap_in_function(&expanded, &fn_name)?;

            // Re-execute ONLY def/defn from executed init forms.
            // Non-def top-level init expressions often perform global side effects
            // (e.g. protocol registration) that live in Rust globals and should not
            // be replayed on every interactive eval.
            if !skip_init_execution {
                let is_def_or_defn =
                    matches!(expanded, Expr::Def { .. } | Expr::Defn { .. });
                if is_def_or_defn {
                    def_fn_names.push(fn_name);
                }
            }
        }

        // Loaded modules are already registered in the CodeGen above.
        // Avoid compiling (use ...) here to prevent emitting extra IR into prior functions.

        // Compile all forms in order:
        // 1. Init forms (from project files) - compile always, execute only if !init_executed
        // 2. History forms (interactive) - compile always, execute def/defn always
        // 3. Current expression - compile and execute
        let mut latest_fn_name = String::new();

        // Compile init forms (project files)
        // Note: def/defn MUST execute every time to initialize globals in the new JIT
        // Track the current namespace during init form loading for symbol registration
        let mut current_init_namespace = "user".to_string();
        // Collect symbols to register after init form processing (avoids borrow checker issues)
        let mut symbols_to_register: Vec<(String, String)> = Vec::new();

        // Clone init_forms to avoid borrow checker issues when updating self.namespace
        let init_forms_clone = self.init_forms.clone();

        if repl_debug_enabled() {
            eprintln!("DEBUG eval_internal: Processing {} init forms", init_forms_clone.len());
        }
        for (form_idx, init_input) in init_forms_clone.iter().enumerate() {
            if repl_debug_enabled() {
                eprintln!("DEBUG eval_internal: Processing init form {}: {} chars", form_idx, init_input.len());
            }
            let init_exprs = parse(init_input)?;
            if init_exprs.is_empty() {
                continue;
            }

            // Process ALL expressions in this init form (not just first one!)
            for (expr_idx, expr) in init_exprs.iter().enumerate() {
                let expanded_expr = expand_macros_with_registry(expr, &mut macro_registry);

                // Handle namespace declarations
                if let clorus_syntax::Expr::Ns { name, requires, rust_imports } = &expanded_expr {
                    // Update tracked namespace for symbol registration
                    current_init_namespace = name.clone();

                    // Update ReplEngine's namespace (important so subsequent forms compile in correct namespace)
                    self.set_namespace(name);

                    // Process requires to update namespace imports/aliases
                    for req_spec in requires {
                        self.process_require(req_spec);
                    }

                    // Process rust imports
                    for rust_import in rust_imports {
                        if let Some(alias) = &rust_import.alias {
                            // Convert library name to module format: "egui-hello" -> "rust.egui_hello"
                            let rust_module = format!("rust.{}", rust_import.library.replace('-', "_"));
                            self.namespace.aliases.insert(alias.clone(), rust_module);
                        }
                    }

                    // Create namespace context for codegen (with imports/aliases)
                    let codegen_ns = clorus_codegen::NamespaceContext {
                        current: self.namespace.current.clone(),
                        aliases: self.namespace.aliases.clone(),
                        imports: self.namespace.imports.clone(),
                    };
                    codegen.set_namespace(codegen_ns);

                    // Skip compilation for ns form (it's metadata, not executable)
                    continue;
                }

                // Handle standalone require expressions (load modules)
                if let clorus_syntax::Expr::Require { specs } = &expanded_expr {
                    // Load each required module synchronously
                    for spec in specs {
                        // Load the module and all its dependencies
                        let module_exprs = self.load_module_recursive(&spec.module)?;

                        // Add loaded module expressions to module_exprs for compilation
                        self.module_exprs.extend(module_exprs);

                        // Update namespace aliases after loading
                        self.process_require(spec);
                    }

                    // Skip compilation for require (it's a compile-time directive)
                    continue;
                }

                // Handle forward declarations (compile-time directive)
                if let clorus_syntax::Expr::Declare { names } = &expanded_expr {
                    // Process declare directly - adds forward declarations to codegen
                    for name in names {
                        codegen.add_forward_declaration(name);
                    }
                    // Skip wrapping - declare is compile-time only
                    continue;
                }

                let is_def_or_defn = matches!(expanded_expr, clorus_syntax::Expr::Def { .. } | clorus_syntax::Expr::Defn { .. });

                // Extract symbol name for registration
                let symbol_name = match &expanded_expr {
                    clorus_syntax::Expr::Def { name, .. } => Some(name.clone()),
                    clorus_syntax::Expr::Defn { name, .. } => Some(name.clone()),
                    _ => None,
                };

                let fn_name = format!("init_{}_{}", form_idx, expr_idx);
                codegen.wrap_in_function(&expanded_expr, &fn_name)?;

                // Execute def/defn to initialize globals (required for JIT architecture)
                // Skip during project loading to avoid O(n²) re-execution
                if is_def_or_defn && !skip_init_execution {
                    def_fn_names.push(fn_name);

                    // Collect symbol for registration (will register after loop to avoid borrow issues)
                    if let Some(name) = symbol_name {
                        symbols_to_register.push((current_init_namespace.clone(), name));
                    }
                }
            }
        }

        // Register all symbols collected from init forms
        for (namespace, symbol) in symbols_to_register {
            self.register_symbol(&namespace, &symbol);
        }

        // IMPORTANT: Re-register all loaded modules after processing init forms
        // This is necessary because init forms may contain (require ...) statements
        // that load new modules, and those need to be registered with the codegen
        for namespace in &self.loaded_modules {
            codegen.register_clip_namespace(namespace);
        }

        // Compile interactive history forms
        for (i, historical_input) in self.history.iter().enumerate() {
            let historical_exprs = parse(historical_input)?;
            if historical_exprs.is_empty() {
                continue;
            }

            // Process only the first expression from each history entry
            let expr = &historical_exprs[0];

            // Expand macros before compiling
            let expanded_expr = expand_macros_with_registry(expr, &mut macro_registry);

            // Check if this is a def or defn expression
            let is_def_or_defn = matches!(expanded_expr, clorus_syntax::Expr::Def { .. } | clorus_syntax::Expr::Defn { .. });

            // Compile historical expressions
            let fn_name = format!("history_{}", i);
            codegen.wrap_in_function(&expanded_expr, &fn_name)?;

            // If it's a def, we need to execute it to set the global value
            if is_def_or_defn {
                def_fn_names.push(fn_name);
            }
        }

        // Now compile the CURRENT expression (the new one we're evaluating)
        if repl_debug_enabled() {
            eprintln!("DEBUG eval_internal: Compiling current expression...");
        }
        let current_expr = &exprs[0];
        let expanded_current = expand_macros_with_registry(current_expr, &mut macro_registry);
        let fn_name = format!("eval_{}", self.expr_count);
        self.expr_count += 1;
        if repl_debug_enabled() {
            eprintln!("DEBUG eval_internal: Wrapping in function {}...", fn_name);
        }
        codegen.wrap_in_function(&expanded_current, &fn_name)?;
        latest_fn_name = fn_name.clone();

        if repl_debug_enabled() {
            eprintln!("DEBUG eval_internal: Verifying LLVM module...");
        }
        codegen.finalize_global_init();
        // Verify LLVM module before JIT
        if let Err(e) = codegen.get_module().verify() {
            if std::env::var("CLORUS_DEBUG_IR").is_ok() {
                let ir = codegen.get_module().print_to_string().to_string();
                let _ = std::fs::write("/tmp/clorus_repl_ir.ll", ir);
            }
            return Err(format!("LLVM module verification failed: {:?}", e));
        }

        if repl_debug_enabled() {
            eprintln!("DEBUG eval_internal: Creating JIT engine...");
        }
        // Create JIT engine
        let engine = codegen.get_module()
            .create_jit_execution_engine(OptimizationLevel::None)
            .map_err(|e| format!("JIT error: {}", e))?;

        if repl_debug_enabled() {
            eprintln!("DEBUG eval_internal: Executing {} init statements...", init_fn_names.len());
        }

        // Execute init statements to reconstruct program state in the fresh JIT module
        for (idx, fn_name) in init_fn_names.iter().enumerate() {
            if repl_debug_enabled() {
                eprintln!("DEBUG eval_internal: Executing init {} of {}: {}...", idx + 1, init_fn_names.len(), fn_name);
            }
            unsafe {
                type EvalFunc = unsafe extern "C" fn() -> *mut u8;
                match engine.get_function::<EvalFunc>(fn_name) {
                    Ok(jit_fn) => {
                        jit_fn.call();
                    }
                    Err(e) => {
                        return Err(format!("Failed to execute {}: {:?}", fn_name, e));
                    }
                }
            }
        }

        if repl_debug_enabled() {
            eprintln!("DEBUG eval_internal: Executing {} def/defn statements...", def_fn_names.len());
        }

        // Execute historical def/defn statements to initialize globals and functions
        for (idx, fn_name) in def_fn_names.iter().enumerate() {
            if repl_debug_enabled() {
                eprintln!("DEBUG eval_internal: Executing def/defn {} of {}: {}...", idx + 1, def_fn_names.len(), fn_name);
            }
            unsafe {
                type EvalFunc = unsafe extern "C" fn() -> *mut u8;
                match engine.get_function::<EvalFunc>(fn_name) {
                    Ok(jit_fn) => {
                        jit_fn.call();
                    }
                    Err(e) => {
                        return Err(format!("Failed to execute {}: {:?}", fn_name, e));
                    }
                }
            }
        }

        // Execute the latest expression and return its result
        if repl_debug_enabled() {
            eprintln!("DEBUG eval_internal: Executing current expression {}...", latest_fn_name);
        }
        unsafe {
            type EvalFunc = unsafe extern "C" fn() -> *mut u8;
            let jit_fn = engine.get_function::<EvalFunc>(&latest_fn_name)
                .map_err(|e| format!("Function not found: {}", e))?;
            if repl_debug_enabled() {
                eprintln!("DEBUG eval_internal: Calling JIT function...");
            }
            let value = jit_fn.call();
            if repl_debug_enabled() {
                eprintln!("DEBUG eval_internal: JIT function returned successfully");
            }

            // Only add to history after successful evaluation
            if add_to_history {
                self.history.push(input.to_string());
            }

            // Register symbol in the registry (for :refer :all support)
            // This tracks symbols defined interactively in the REPL
            match &eval_kind {
                EvalKind::Def(name) | EvalKind::Defn(name) | EvalKind::Defmacro(name) => {
                    let current_ns = self.namespace.current.clone();
                    self.register_symbol(&current_ns, name);
                }
                _ => {}
            }

            self.macro_registry = macro_registry;
            Ok(EvalResult { value, kind: eval_kind })
        }
    }
}
