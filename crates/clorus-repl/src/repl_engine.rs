/// REPL-specific execution engine that manages state across expressions
///
/// Strategy: Keep all source code and recompile everything for each expression.
/// This ensures variables and functions persist across REPL lines.
use clorus::{parse, expand_macros, CodeGen, ModuleLoader, NamespaceContext};
use clorus::codegen::RustLibrary;
use clorus_syntax::{Expr, RequireSpec};
use inkwell::context::Context;
use inkwell::OptimizationLevel;
use std::collections::{HashSet, HashMap};

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
    /// (ns name) - display nil
    Namespace,
    /// (require ...) or (use ...) - display nil
    Import,
}

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
}

impl<'ctx> ReplEngine<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        ReplEngine {
            context,
            history: Vec::new(),
            init_forms: Vec::new(),
            expr_count: 0,
            loaded_modules: HashSet::new(),
            namespace: NamespaceContext::new("user"),
            module_loader: ModuleLoader::new(),
            rust_libraries: Vec::new(),
        }
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

    /// Switch to a new namespace
    pub fn set_namespace(&mut self, namespace: &str) {
        self.namespace = NamespaceContext::new(namespace);
    }

    /// Process a require spec and update namespace context
    fn process_require(&mut self, spec: &RequireSpec) {
        self.namespace.process_require(spec);
    }

    /// Evaluate a project initialization form (stores separately, executes once)
    pub fn eval_init(&mut self, input: &str) -> Result<EvalResult, String> {
        // Store in init_forms for future compilations
        self.init_forms.push(input.to_string());
        // Evaluate it (will be marked as executed after first successful run)
        self.eval_internal(input, false)
    }

    /// Evaluate an expression and add it to history (for interactive REPL)
    pub fn eval(&mut self, input: &str) -> Result<EvalResult, String> {
        self.eval_internal(input, true)
    }

    fn eval_internal(&mut self, input: &str, add_to_history: bool) -> Result<EvalResult, String> {
        // Parse the new input to validate it FIRST (before adding to history)
        let exprs = parse(input)?;
        if exprs.is_empty() {
            return Err("No expression to evaluate".to_string());
        }

        // Determine the kind of expression for output formatting (use first expression)
        let eval_kind = match &exprs[0] {
            Expr::Def { name, .. } => EvalKind::Def(name.clone()),
            Expr::Defn { name, .. } => EvalKind::Defn(name.clone()),
            Expr::Ns { .. } => EvalKind::Namespace,
            Expr::Require { .. } | Expr::Use { .. } => EvalKind::Import,
            _ => EvalKind::Value,
        };

        // Handle special namespace commands
        match &exprs[0] {
            Expr::Ns { name, requires, rust_imports } => {
                // Switch to new namespace
                self.set_namespace(name);

                // Process all requires
                for req_spec in requires {
                    self.process_require(req_spec);
                }

                // Process Rust imports
                for rust_import in rust_imports {
                    if let Some(alias) = &rust_import.alias {
                        // Register the rust library alias in the namespace
                        // Example: gui -> egui-hello
                        self.namespace.aliases.insert(alias.clone(), rust_import.library.clone());
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
                // Process all require specs
                for spec in specs {
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

        // Register all Rust FFI libraries
        for lib in &self.rust_libraries {
            codegen.register_rust_library(lib.clone());
        }

        // Set the current namespace context
        // Convert clorus::NamespaceContext to codegen::NamespaceContext
        let codegen_ns = clorus_codegen::NamespaceContext {
            current: self.namespace.current.clone(),
            aliases: self.namespace.aliases.clone(),
            imports: HashMap::new(), // Simplified for now
        };
        codegen.set_namespace(codegen_ns);

        // Re-declare all loaded modules in this fresh CodeGen
        // This ensures functions like slurp/spit are available
        for module in &self.loaded_modules {
            let use_expr = Expr::Use {
                module: module.clone(),
                imports: vec![],
            };
            let _ = codegen.compile_expr(&use_expr)?;
        }

        // Compile all forms in order:
        // 1. Init forms (from project files) - compile always, execute only if !init_executed
        // 2. History forms (interactive) - compile always, execute def/defn always
        // 3. Current expression - compile and execute
        let mut latest_fn_name = String::new();
        let mut def_fn_names = Vec::new();

        // Compile init forms (project files)
        // Note: def/defn MUST execute every time to initialize globals in the new JIT
        for (i, init_input) in self.init_forms.iter().enumerate() {
            let init_exprs = parse(init_input)?;
            if init_exprs.is_empty() {
                continue;
            }

            let expr = &init_exprs[0];
            let expanded_expr = expand_macros(expr);
            let is_def_or_defn = matches!(expanded_expr, clorus_syntax::Expr::Def { .. } | clorus_syntax::Expr::Defn { .. });

            let fn_name = format!("init_{}", i);
            codegen.wrap_in_function(&expanded_expr, &fn_name)?;

            // Execute def/defn to initialize globals (required for JIT architecture)
            if is_def_or_defn {
                def_fn_names.push(fn_name);
            }
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
            let expanded_expr = expand_macros(expr);

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
        let current_expr = &exprs[0];
        let expanded_current = expand_macros(current_expr);
        let fn_name = format!("eval_{}", self.expr_count);
        self.expr_count += 1;
        codegen.wrap_in_function(&expanded_current, &fn_name)?;
        latest_fn_name = fn_name;

        // Create JIT engine
        let engine = codegen.get_module()
            .create_jit_execution_engine(OptimizationLevel::None)
            .map_err(|e| format!("JIT error: {}", e))?;

        // Execute historical def/defn statements to initialize globals and functions
        for fn_name in &def_fn_names {
            unsafe {
                type EvalFunc = unsafe extern "C" fn() -> *mut u8;
                if let Ok(jit_fn) = engine.get_function::<EvalFunc>(fn_name) {
                    jit_fn.call(); // We don't care about the return value
                }
            }
        }

        // Execute the latest expression and return its result
        unsafe {
            type EvalFunc = unsafe extern "C" fn() -> *mut u8;
            let jit_fn = engine.get_function::<EvalFunc>(&latest_fn_name)
                .map_err(|e| format!("Function not found: {}", e))?;
            let value = jit_fn.call();

            // Only add to history after successful evaluation
            if add_to_history {
                self.history.push(input.to_string());
            }

            Ok(EvalResult { value, kind: eval_kind })
        }
    }
}
