/// REPL-specific execution engine that manages state across expressions
///
/// Strategy: Keep all source code and recompile everything for each expression.
/// This ensures variables and functions persist across REPL lines.
use clorus::{parse, expand_macros, CodeGen, ModuleLoader, NamespaceContext};
use clorus_codegen;
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
    /// All expressions entered so far
    history: Vec<String>,
    expr_count: usize,
    /// Track which modules have been loaded
    loaded_modules: HashSet<String>,
    /// Current namespace context
    namespace: NamespaceContext,
    /// Module loader for (require ...) support
    module_loader: ModuleLoader,
}

impl<'ctx> ReplEngine<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        ReplEngine {
            context,
            history: Vec::new(),
            expr_count: 0,
            loaded_modules: HashSet::new(),
            namespace: NamespaceContext::new("user"),
            module_loader: ModuleLoader::new(),
        }
    }

    /// Get the current namespace name
    pub fn current_namespace(&self) -> &str {
        &self.namespace.current
    }

    /// Switch to a new namespace
    pub fn set_namespace(&mut self, namespace: &str) {
        self.namespace = NamespaceContext::new(namespace);
    }

    /// Process a require spec and update namespace context
    fn process_require(&mut self, spec: &RequireSpec) {
        self.namespace.process_require(spec);
    }

    pub fn eval(&mut self, input: &str) -> Result<EvalResult, String> {
        // Add to history
        self.history.push(input.to_string());

        // Parse the new input to validate it
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

                // TODO: Handle rust_imports when we integrate FFI with namespaces
                if !rust_imports.is_empty() {
                    println!("Note: Rust imports in ns will be supported soon");
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

        // Compile all history to build up globals and function definitions
        // Execute def/defn statements from history, but only execute the latest expression for output
        let mut latest_fn_name = String::new();
        let mut def_fn_names = Vec::new();

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

            // For the latest expression, save its function name for execution
            if i == self.history.len() - 1 {
                let fn_name = format!("eval_{}", self.expr_count);
                self.expr_count += 1;
                codegen.wrap_in_function(&expanded_expr, &fn_name)?;
                latest_fn_name = fn_name;
            } else {
                // For older expressions that define globals or functions,
                // we need to compile and sometimes execute them
                let fn_name = format!("history_{}", i);
                codegen.wrap_in_function(&expanded_expr, &fn_name)?;

                // If it's a def, we need to execute it to set the global value
                if is_def_or_defn {
                    def_fn_names.push(fn_name);
                }
            }
        }

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
            Ok(EvalResult { value, kind: eval_kind })
        }
    }
}
