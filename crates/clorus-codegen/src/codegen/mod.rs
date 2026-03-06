mod arithmetic;
mod calls;
/// LLVM Code Generation for Clorus
mod closures;
mod core_calls;
mod ffi_calls;
mod functions;
mod quotes;

use clorus_syntax::{Expr, MapPatternKey, Pattern};
use clorus_types::{FfiFunction, FfiType};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{
    BasicMetadataValueEnum, FloatValue, FunctionValue, GlobalValue, IntValue, PhiValue,
    PointerValue,
};
use inkwell::AddressSpace;
use inkwell::{FloatPredicate, IntPredicate};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

// Import namespace context
use crate::namespace_context::NamespaceContext;

/// Rust FFI library metadata
#[derive(Debug, Clone)]
pub struct RustLibrary {
    pub name: String,
    pub functions: Vec<RustFunction>,
}

#[derive(Debug, Clone)]
pub struct RustFunction {
    pub name: String,
    pub params: Vec<RustParam>,
    pub return_type: String,
}

#[derive(Debug, Clone)]
pub struct RustParam {
    pub name: String,
    pub type_name: String,
}

/// Context for loop/recur compilation
#[derive(Clone)]
struct LoopContext<'ctx> {
    /// The basic block to jump to when recur is called
    loop_start: BasicBlock<'ctx>,
    /// The basic block after the loop (for normal exit)
    loop_end: BasicBlock<'ctx>,
    /// The binding variable names in order
    binding_names: Vec<String>,
    /// Phi nodes for loop parameters (for proper recur updates)
    phi_nodes: Vec<PhiValue<'ctx>>,
}

#[derive(Clone)]
struct RecurFnContext {
    name: String,
    fixed_param_count: usize,
    has_rest_param: bool,
}

#[derive(Debug, Clone)]
struct MultimethodMethod<'ctx> {
    dispatch_value: Expr,
    function: FunctionValue<'ctx>,
    arity: usize,
}

pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    /// Symbol table mapping variable names to their alloca pointers (local variables)
    variables: HashMap<String, PointerValue<'ctx>>,
    /// Global variables defined with (def ...)
    globals: HashMap<String, GlobalValue<'ctx>>,
    /// Function table mapping function names to their LLVM functions
    functions: HashMap<String, FunctionValue<'ctx>>,
    /// Function signature metadata for direct-call argument shaping
    /// Maps function name -> (fixed_param_count, has_rest_param)
    function_signatures: HashMap<String, (usize, bool)>,
    /// Rust FFI libraries available for use
    rust_libraries: HashMap<String, RustLibrary>,
    /// Namespace context for symbol resolution
    namespace: NamespaceContext,
    /// Counter for generating unique lambda names
    lambda_counter: usize,
    /// Current loop context (for loop/recur)
    loop_context: Option<LoopContext<'ctx>>,
    /// Current named function context for recur fallback outside loop.
    current_recur_fn: Option<RecurFnContext>,
    /// Forward declared functions (from declare form) - allows mutual recursion
    forward_declarations: HashSet<String>,
    /// Namespaces provided by .clip packages (Phase 4)
    clip_namespaces: HashSet<String>,
    /// Parameter context for nested closures to capture defn parameters
    parameter_context: HashMap<String, PointerValue<'ctx>>,
    /// Compilation start time for timeout detection
    compile_start: Instant,
    /// Compilation timeout duration
    compile_timeout: Duration,
    /// Number of expressions compiled (for progress tracking)
    expr_count: usize,
    /// Protocol methods for automatic dispatch
    /// Maps method_name -> (protocol_name, param_count)
    protocol_methods: HashMap<String, (String, usize)>,
    /// Multimethod dispatch expressions by multimethod name
    multimethod_dispatch: HashMap<String, Expr>,
    /// Registered multimethod methods by multimethod name
    multimethod_methods: HashMap<String, Vec<MultimethodMethod<'ctx>>>,
    /// Preference pairs (preferred, over) by multimethod name
    multimethod_preferences: HashMap<String, Vec<(Expr, Expr)>>,
    /// Lazy global init function for top-level defs in REPL/JIT
    global_init_fn: Option<FunctionValue<'ctx>>,
    global_init_block: Option<BasicBlock<'ctx>>,
}

impl<'ctx> CodeGen<'ctx> {
    fn runtime_arity(fixed_param_count: usize, has_rest_param: bool) -> i32 {
        if has_rest_param {
            // Negative arity encodes variadic with fixed prefix:
            // fixed = (-arity - 1)
            -((fixed_param_count as i32) + 1)
        } else {
            fixed_param_count as i32
        }
    }

    fn arity_variant_name(
        base_name: &str,
        fixed_param_count: usize,
        has_rest_param: bool,
    ) -> String {
        if has_rest_param {
            format!("{}_arity_{}_var", base_name, fixed_param_count)
        } else {
            format!("{}_arity_{}", base_name, fixed_param_count)
        }
    }

    fn build_rest_vector_from_values(
        &self,
        values: &[PointerValue<'ctx>],
    ) -> Result<PointerValue<'ctx>, String> {
        let vector_empty_fn = self
            .module
            .get_function("clorus_vector_empty")
            .ok_or("clorus_vector_empty not declared")?;
        let vector_conj_fn = self
            .module
            .get_function("clorus_vector_conj")
            .ok_or("clorus_vector_conj not declared")?;

        let mut rest_vec = self
            .builder
            .build_call(vector_empty_fn, &[], "rest_vec_empty")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        for (i, val) in values.iter().enumerate() {
            rest_vec = self
                .builder
                .build_call(
                    vector_conj_fn,
                    &[rest_vec.into(), (*val).into()],
                    &format!("rest_vec_conj_{}", i),
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();
        }

        Ok(rest_vec)
    }

    fn build_rest_vector_from_arg_array(
        &mut self,
        args_param: PointerValue<'ctx>,
        start_index: u64,
        arg_count: IntValue<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        let vector_empty_fn = self
            .module
            .get_function("clorus_vector_empty")
            .ok_or("clorus_vector_empty not declared")?;
        let vector_conj_fn = self
            .module
            .get_function("clorus_vector_conj")
            .ok_or("clorus_vector_conj not declared")?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("No current function for rest arg extraction")?;

        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let one = i64_type.const_int(1, false);

        let rest_vec_alloca = self
            .builder
            .build_alloca(value_ptr_type, "rest_vec_alloca")
            .unwrap();
        let index_alloca = self.builder.build_alloca(i64_type, "rest_idx").unwrap();

        let empty_vec = self
            .builder
            .build_call(vector_empty_fn, &[], "rest_vec_empty")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        self.builder
            .build_store(rest_vec_alloca, empty_vec)
            .unwrap();
        self.builder
            .build_store(index_alloca, i64_type.const_int(start_index, false))
            .unwrap();

        let cond_bb = self
            .context
            .append_basic_block(current_fn, "rest_loop_cond");
        let body_bb = self
            .context
            .append_basic_block(current_fn, "rest_loop_body");
        let end_bb = self.context.append_basic_block(current_fn, "rest_loop_end");

        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(cond_bb);
        let idx = self
            .builder
            .build_load(i64_type, index_alloca, "rest_idx_cur")
            .unwrap()
            .into_int_value();
        let in_range = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::ULT,
                idx,
                arg_count,
                "rest_idx_in_range",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(in_range, body_bb, end_bb)
            .unwrap();

        self.builder.position_at_end(body_bb);
        let arg_ptr = unsafe {
            self.builder
                .build_gep(value_ptr_type, args_param, &[idx], "rest_arg_ptr")
                .unwrap()
        };
        let arg_val = self
            .builder
            .build_load(value_ptr_type, arg_ptr, "rest_arg_val")
            .unwrap()
            .into_pointer_value();
        let cur_vec = self
            .builder
            .build_load(value_ptr_type, rest_vec_alloca, "rest_vec_cur")
            .unwrap()
            .into_pointer_value();
        let next_vec = self
            .builder
            .build_call(
                vector_conj_fn,
                &[cur_vec.into(), arg_val.into()],
                "rest_vec_next",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        self.builder.build_store(rest_vec_alloca, next_vec).unwrap();

        let next_idx = self
            .builder
            .build_int_add(idx, one, "rest_idx_next")
            .unwrap();
        self.builder.build_store(index_alloca, next_idx).unwrap();
        self.builder.build_unconditional_branch(cond_bb).unwrap();

        self.builder.position_at_end(end_bb);
        let out = self
            .builder
            .build_load(value_ptr_type, rest_vec_alloca, "rest_vec_out")
            .unwrap()
            .into_pointer_value();
        Ok(out)
    }

    fn multimethod_prefers(&self, name: &str, preferred: &Expr, over: &Expr) -> bool {
        self.multimethod_preferences
            .get(name)
            .map(|pairs| pairs.iter().any(|(p, o)| p == preferred && o == over))
            .unwrap_or(false)
    }

    fn multimethod_name_from_symbol_arg(arg: &Expr) -> Option<String> {
        match arg {
            Expr::Symbol(s) => Some(s.clone()),
            _ => None,
        }
    }

    fn build_function_value_from_method_set(
        &mut self,
        methods: &[MultimethodMethod<'ctx>],
        label: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        if methods.is_empty() {
            let nil_fn = self
                .module
                .get_function("clorus_value_nil")
                .ok_or("clorus_value_nil not declared")?;
            return Ok(self
                .builder
                .build_call(nil_fn, &[], &format!("{}_nil", label))
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value());
        }

        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i32_type = self.context.i32_type();

        if methods.len() == 1 {
            let method = &methods[0];
            let function_new_fn = self
                .module
                .get_function("clorus_function_new")
                .ok_or("clorus_function_new not declared")?;
            let fn_ptr = method.function.as_global_value().as_pointer_value();
            let fn_ptr_cast = self
                .builder
                .build_pointer_cast(fn_ptr, value_ptr_type, &format!("{}_fn_ptr_cast", label))
                .unwrap();
            let arity_val = i32_type.const_int(method.arity as u64, false);
            let null_env = value_ptr_type
                .ptr_type(AddressSpace::default())
                .const_null();
            let env_size = i32_type.const_zero();
            return Ok(self
                .builder
                .build_call(
                    function_new_fn,
                    &[
                        fn_ptr_cast.into(),
                        arity_val.into(),
                        null_env.into(),
                        env_size.into(),
                    ],
                    &format!("{}_fn_value", label),
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value());
        }

        let mut unique_methods: Vec<MultimethodMethod<'ctx>> = Vec::new();
        for method in methods {
            if !unique_methods.iter().any(|m| m.arity == method.arity) {
                unique_methods.push(method.clone());
            }
        }
        unique_methods.sort_by_key(|m| m.arity);

        let arity_variant_type = self
            .context
            .struct_type(&[i32_type.into(), value_ptr_type.into()], false);
        let variants_array_type = arity_variant_type.array_type(unique_methods.len() as u32);
        let variants_array = self
            .builder
            .build_alloca(variants_array_type, &format!("{}_variants", label))
            .unwrap();

        for (i, method) in unique_methods.iter().enumerate() {
            let variant_ptr = unsafe {
                self.builder
                    .build_gep(
                        variants_array_type,
                        variants_array,
                        &[i32_type.const_zero(), i32_type.const_int(i as u64, false)],
                        &format!("{}_variant_{}", label, i),
                    )
                    .unwrap()
            };

            let arity_ptr = unsafe {
                self.builder
                    .build_gep(
                        arity_variant_type,
                        variant_ptr,
                        &[i32_type.const_zero(), i32_type.const_zero()],
                        &format!("{}_arity_ptr_{}", label, i),
                    )
                    .unwrap()
            };
            self.builder
                .build_store(arity_ptr, i32_type.const_int(method.arity as u64, true))
                .unwrap();

            let fn_ptr_field = unsafe {
                self.builder
                    .build_gep(
                        arity_variant_type,
                        variant_ptr,
                        &[i32_type.const_zero(), i32_type.const_int(1, false)],
                        &format!("{}_fn_ptr_field_{}", label, i),
                    )
                    .unwrap()
            };
            let fn_ptr = method.function.as_global_value().as_pointer_value();
            self.builder.build_store(fn_ptr_field, fn_ptr).unwrap();
        }

        let arities_ptr = self
            .builder
            .build_pointer_cast(
                variants_array,
                value_ptr_type,
                &format!("{}_arities_ptr", label),
            )
            .unwrap();

        let multi_arity_fn = self
            .module
            .get_function("clorus_multi_arity_function_new")
            .ok_or("clorus_multi_arity_function_new not declared")?;
        let null_env = value_ptr_type
            .ptr_type(AddressSpace::default())
            .const_null();
        let env_size = i32_type.const_zero();
        Ok(self
            .builder
            .build_call(
                multi_arity_fn,
                &[
                    arities_ptr.into(),
                    i32_type
                        .const_int(unique_methods.len() as u64, false)
                        .into(),
                    null_env.into(),
                    env_size.into(),
                ],
                &format!("{}_multi_fn_value", label),
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }

    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        let mut codegen = CodeGen {
            context,
            module,
            builder,
            variables: HashMap::new(),
            globals: HashMap::new(),
            functions: HashMap::new(),
            function_signatures: HashMap::new(),
            rust_libraries: HashMap::new(),
            namespace: NamespaceContext::default_namespace(),
            lambda_counter: 0,
            loop_context: None,
            current_recur_fn: None,
            forward_declarations: HashSet::new(),
            clip_namespaces: HashSet::new(),
            parameter_context: HashMap::new(),
            compile_start: Instant::now(),
            compile_timeout: Duration::from_secs(60), // 60 second default timeout
            expr_count: 0,
            protocol_methods: HashMap::new(),
            multimethod_dispatch: HashMap::new(),
            multimethod_methods: HashMap::new(),
            multimethod_preferences: HashMap::new(),
            global_init_fn: None,
            global_init_block: None,
        };

        // Declare runtime functions
        codegen.declare_runtime_functions();

        // Declare core library functions (I/O, etc.)
        codegen.declare_core_functions();

        codegen
    }

    /// Ensure a valid insertion block for top-level expressions (REPL/JIT).
    fn ensure_global_init_block(&mut self) {
        if self.builder.get_insert_block().is_some() {
            return;
        }

        if self.global_init_fn.is_none() {
            let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
            let fn_type = value_ptr_type.fn_type(&[], false);
            let func = self
                .module
                .add_function("__clorus_global_init", fn_type, None);
            let entry = self.context.append_basic_block(func, "entry");
            self.global_init_fn = Some(func);
            self.global_init_block = Some(entry);
        }

        if let Some(block) = self.global_init_block {
            self.builder.position_at_end(block);
        }
    }

    /// Finalize the lazy global init block by adding a return if needed.
    pub fn finalize_global_init(&mut self) {
        if let Some(block) = self.global_init_block {
            if block.get_terminator().is_none() {
                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                self.builder.position_at_end(block);
                let _ = self
                    .builder
                    .build_return(Some(&value_ptr_type.const_null()));
            }
        }
    }

    fn is_core_function(&self, func: &str) -> bool {
        const CORE_FUNCTIONS: &[&str] = &[
            "slurp",
            "spit",
            "get",
            "nth",
            "first",
            "rest",
            "last",
            "count",
            "empty?",
            "reduce",
            "apply",
            "inc",
            "dec",
            "zero?",
            "list",
            "conj",
            "cons",
            "disj",
            "contains?",
            "concat",
            "assoc",
            "dissoc",
            "meta",
            "with-meta",
            "vary-meta",
            "reset-meta!",
            "alter-meta!",
            "var",
            // Hierarchy
            "derive",
            "underive",
            "isa?",
            "parents",
            "ancestors",
            "descendants",
            // Protocol introspection
            "satisfies?",
            "extends?",
            "implements?",
            // Agent operations
            "agent",
            "send",
            "await",
            "await-for",
            "agent-error",
            // Atom operations
            "atom",
            "reset!",
            "swap!",
            "deref",
            "compare-and-set!",
            // Channel operations (CSP)
            "chan",
            ">!!",
            "<!!",
            "close!",
            "alts!!",
            // Go blocks
            "go",
            // String operations
            "str",
            "subs",
            "split",
            "join",
            "upper-case",
            "lower-case",
            "trim",
            "trim-left",
            "trim-right",
            "replace",
            "replace-first",
            "compare",
            "__clorus_compare_values",
            "re-find",
            "re-matches",
            "re-seq",
            "re-replace",
            "re-replace-first",
            "gensym",
            "string?",
            "starts-with?",
            "ends-with?",
            "includes?",
            // Type predicates (public)
            "string?",
            "number?",
            "vector?",
            "list?",
            "map?",
            "set?",
            "keyword?",
            "symbol?",
            "nil?",
            "boolean?",
            "bool?",
            "seq?",
            "coll?",
            "fn?",
            // Type predicates (internal runtime-backed helpers for stdlib wrappers)
            "__clorus_is_string",
            "__clorus_is_number",
            "__clorus_is_vector",
            "__clorus_is_list",
            "__clorus_is_map",
            "__clorus_is_set",
            "__clorus_is_keyword",
            "__clorus_is_symbol",
            "__clorus_is_nil",
            "__clorus_is_bool",
            "__clorus_is_seq",
            "__clorus_is_coll",
            "__clorus_is_fn",
            "__clorus_regex_valid",
            // Collection helpers
            "keys",
            "vals",
            "merge",
            "get-in",
            "assoc-in",
            "update-in",
            "interleave",
            "interpose",
            "distinct",
            "dedupe",
            "flatten",
            // I/O operations
            "print",
            "println",
        ];

        CORE_FUNCTIONS.contains(&func)
    }

    /// Set the namespace context
    pub fn set_namespace(&mut self, namespace: NamespaceContext) {
        self.namespace = namespace;
    }

    /// Get the current namespace context
    pub fn get_namespace(&self) -> &NamespaceContext {
        &self.namespace
    }

    /// Get mutable namespace context
    pub fn get_namespace_mut(&mut self) -> &mut NamespaceContext {
        &mut self.namespace
    }

    /// Set compilation timeout duration
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.compile_timeout = timeout;
    }

    /// Reset compilation timer (call when starting a new compilation unit)
    pub fn reset_timer(&mut self) {
        self.compile_start = Instant::now();
        self.expr_count = 0;
    }

    /// Check if compilation has exceeded timeout
    /// Returns error if timeout exceeded
    fn check_timeout(&self) -> Result<(), String> {
        let elapsed = self.compile_start.elapsed();
        if elapsed > self.compile_timeout {
            return Err(format!(
                "Compilation timeout after {} seconds.\n\
                 Compiled {} expressions before timeout.\n\
                 Possible infinite loop during compilation or overly complex code.\n\
                 Hint: Check for infinite recursion or simplify complex expressions.",
                elapsed.as_secs(),
                self.expr_count
            ));
        }
        Ok(())
    }

    fn rust_library_lookup_keys(name: &str) -> Vec<String> {
        let mut keys = Vec::new();
        let raw = name.to_string();
        let no_prefix = raw.strip_prefix("rust.").unwrap_or(&raw).to_string();
        let hyphen = no_prefix.replace('_', "-");
        let underscore = no_prefix.replace('-', "_");

        keys.push(raw.clone());
        keys.push(no_prefix.clone());
        keys.push(format!("rust.{}", no_prefix));

        if hyphen != no_prefix {
            keys.push(hyphen.clone());
            keys.push(format!("rust.{}", hyphen));
        }
        if underscore != no_prefix {
            keys.push(underscore.clone());
            keys.push(format!("rust.{}", underscore));
        }

        let mut unique = Vec::new();
        for key in keys {
            if !unique.contains(&key) {
                unique.push(key);
            }
        }
        unique
    }

    fn resolve_rust_library(&self, name: &str) -> Option<RustLibrary> {
        for key in Self::rust_library_lookup_keys(name) {
            if let Some(lib) = self.rust_libraries.get(&key) {
                return Some(lib.clone());
            }
        }
        None
    }

    fn rust_ffi_symbol_name(lib_name: &str, func_name: &str) -> String {
        fn normalize(raw: &str) -> String {
            raw.chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect()
        }

        format!("clorus_{}__{}", normalize(lib_name), normalize(func_name))
    }

    /// Register a Rust FFI library so its functions can be used
    pub fn register_rust_library(&mut self, lib: RustLibrary) {
        for key in Self::rust_library_lookup_keys(&lib.name) {
            self.rust_libraries.insert(key, lib.clone());
        }
    }

    /// Register a namespace as coming from a .clip package (Phase 4)
    /// This prevents the compiler from looking for source files for this namespace
    pub fn register_clip_namespace(&mut self, namespace: &str) {
        self.clip_namespaces.insert(namespace.to_string());
    }

    /// Check if a namespace is provided by a .clip package
    pub fn is_clip_namespace(&self, namespace: &str) -> bool {
        // Check if namespace starts with any registered .clip package name
        self.clip_namespaces
            .iter()
            .any(|clip_ns| namespace.starts_with(clip_ns))
    }

    /// Declare all functions from a Rust FFI library based on metadata
    pub fn declare_rust_library_functions(&mut self, lib: &RustLibrary) -> Result<(), String> {
        let f32_type = self.context.f32_type();
        let f64_type = self.context.f64_type();
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        for func in &lib.functions {
            // Convert parameter types
            let mut param_types = Vec::new();
            for param in &func.params {
                let llvm_type = match param.type_name.as_str() {
                    "String" => i8_ptr_type.into(),
                    "f32" => f32_type.into(),
                    "f64" => f64_type.into(),
                    "i32" | "u32" => i32_type.into(),
                    "i64" | "u64" | "isize" | "usize" => i64_type.into(),
                    "bool" => self.context.bool_type().into(),
                    "*mut u8" => i8_ptr_type.into(), // Opaque pointers
                    other => return Err(format!("Unsupported parameter type in FFI: {}", other)),
                };
                param_types.push(llvm_type);
            }

            // Convert return type
            let return_type = match func.return_type.as_str() {
                "String" => i8_ptr_type.fn_type(&param_types, false),
                "f32" => f32_type.fn_type(&param_types, false),
                "f64" => f64_type.fn_type(&param_types, false),
                "i32" | "u32" => i32_type.fn_type(&param_types, false),
                "i64" | "u64" | "isize" | "usize" => i64_type.fn_type(&param_types, false),
                "bool" => self.context.bool_type().fn_type(&param_types, false),
                "()" => self.context.void_type().fn_type(&param_types, false),
                "*mut u8" => i8_ptr_type.fn_type(&param_types, false), // Opaque pointers
                other => return Err(format!("Unsupported return type in FFI: {}", other)),
            };

            // Declare function with dependency-scoped prefix to avoid collisions with core runtime FFI symbols.
            let ffi_name = Self::rust_ffi_symbol_name(&lib.name, &func.name);
            self.module.add_function(&ffi_name, return_type, None);
        }

        Ok(())
    }

    /// Extract a raw pointer from a Value* (for opaque pointers)
    fn extract_pointer_from_value(&self, value_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        // Use runtime function to properly extract opaque pointer from Value*
        let extract_fn = self
            .module
            .get_function("clorus_extract_opaque_pointer")
            .expect(
                "clorus_extract_opaque_pointer not declared - runtime functions not initialized",
            );
        let call_result = self
            .builder
            .build_call(extract_fn, &[value_ptr.into()], "extract_ptr")
            .unwrap();
        call_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }

    /// Box a raw pointer into a Value* (for opaque pointers)
    fn box_pointer(&self, ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        // Use runtime function to properly create Value* with OpaquePointer tag
        let box_fn = self
            .module
            .get_function("clorus_value_opaque_pointer")
            .expect("clorus_value_opaque_pointer not declared - runtime functions not initialized");
        let call_result = self
            .builder
            .build_call(box_fn, &[ptr.into()], "box_ptr")
            .unwrap();
        call_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }

    /// Declare runtime library FFI functions
    fn declare_runtime_functions(&mut self) {
        // ===== Memory Management =====
        self.declare_void_fn("clorus_retain", 1);
        self.declare_void_fn("clorus_release", 1);

        // malloc for closure environments
        let i64_type = self.context.i64_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let malloc_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
        self.module.add_function("malloc", malloc_type, None);

        // ===== Vector Functions =====
        self.declare_no_arg_value_fn("clorus_vector_empty");
        self.declare_value_fn("clorus_vector_conj", 2);
        self.declare_value_fn("clorus_conj", 2); // Generic, dispatches by type
        self.declare_value_i64_fn("clorus_vector_nth");
        self.declare_value_to_i64_fn("clorus_vector_count");
        self.declare_value_i64_fn("clorus_vector_rest");

        // ===== List Functions =====
        self.declare_no_arg_value_fn("clorus_list_empty");
        self.declare_value_fn("clorus_list_cons", 2);

        // ===== Map Functions =====
        self.declare_no_arg_value_fn("clorus_map_empty");
        self.declare_value_fn("clorus_map_assoc", 3);
        self.declare_value_fn("clorus_map_get", 2);
        self.declare_value_to_i64_fn("clorus_map_count");

        // ===== Set Functions =====
        self.declare_no_arg_value_fn("clorus_set_empty");
        self.declare_value_fn("clorus_set_conj", 2);
        self.declare_value_fn("clorus_set_disj", 2);
        self.declare_value_fn("clorus_set_contains", 2);
        self.declare_value_fn("clorus_contains", 2);
        self.declare_value_to_i64_fn("clorus_set_count");

        // ===== Atom Functions =====
        self.declare_value_fn("clorus_atom", 1);
        self.declare_value_fn("clorus_deref", 1);
        self.declare_value_fn("clorus_reset", 2);
        self.declare_value_fn("clorus_swap", 3);

        // ===== Ref Functions (STM) =====
        self.declare_value_fn("clorus_ref", 1);
        self.declare_value_fn("clorus_ref_deref", 1);
        self.declare_value_fn("clorus_ref_set", 2);
        self.declare_value_fn("clorus_alter", 2);

        // ===== Transaction Functions =====
        self.declare_no_arg_bool_fn("clorus_tx_begin");
        self.declare_no_arg_bool_fn("clorus_tx_commit");
        self.declare_no_arg_void_fn("clorus_tx_abort");
        self.declare_no_arg_bool_fn("clorus_tx_active");

        // ===== Agent Functions =====
        self.declare_value_fn("clorus_agent", 1);
        self.declare_value_fn("clorus_agent_deref", 1);
        self.declare_value_fn("clorus_send", 3);
        self.declare_value_fn("clorus_agent_error", 1);
        self.declare_value_fn("clorus_await", 1);
        self.declare_value_i64_fn("clorus_await_for");

        // ===== Channel Functions =====
        self.declare_i64_to_value_fn("clorus_chan");
        self.declare_value_fn("clorus_chan_put", 2);
        self.declare_value_fn("clorus_chan_take", 1);
        self.declare_value_fn("clorus_chan_close", 1);
        self.declare_value_fn("clorus_alts", 1);

        // ===== Go Block Functions =====
        self.declare_value_fn("clorus_go", 2);

        // ===== Value Creation Functions =====
        self.declare_i64_to_value_fn("clorus_value_long");
        self.declare_f64_to_value_fn("clorus_value_double");
        self.declare_value_to_i64_fn("clorus_value_as_long");
        self.declare_value_to_f64_fn("clorus_value_as_double");
        self.declare_value_to_f64_fn("clorus_value_as_number"); // Handles both Long and Double
        self.declare_value_fn("clorus_value_string", 1);
        self.declare_value_fn("clorus_value_as_cstring", 1);
        self.declare_void_fn("clorus_free_cstring", 1);
        self.declare_value_fn("clorus_keyword", 1);
        self.declare_value_fn("clorus_symbol", 1);
        self.declare_value_fn("clorus_gensym", 1);
        self.declare_no_arg_value_fn("clorus_value_nil");
        self.declare_f64_to_value_fn("clorus_value_bool");
        self.declare_value_to_i32_fn("clorus_is_truthy");
        self.declare_bool_to_value_fn("clorus_value_boolean");

        // Opaque pointer boxing/unboxing (for FFI pointers like window handles)
        self.declare_ptr_to_value_fn("clorus_value_opaque_pointer");
        self.declare_value_to_ptr_fn("clorus_extract_opaque_pointer");
        self.declare_value_fn("clorus_value_exception", 1);
        self.declare_value_fn("clorus_exception_payload", 1);
        self.declare_value_to_i32_fn("clorus_is_exception_i32");
        self.declare_value_fn("clorus_var_new", 2);
        self.declare_value_fn("clorus_var_get", 1);
        self.declare_value_fn("clorus_var_set", 2);
        self.declare_void_fn("clorus_var_push_binding", 2);
        self.declare_void_fn("clorus_var_pop_binding", 1);
        self.declare_void_fn("clorus_var_set_meta", 3);
        self.declare_value_fn("clorus_var_meta", 1);
        self.declare_value_fn("clorus_value_from_var", 1);
        self.declare_value_to_ptr_fn("clorus_value_as_var");
        self.declare_value_fn("clorus_deref_var_value", 1);
        self.declare_value_fn("clorus_meta", 1);
        self.declare_value_fn("clorus_with_meta", 2);
        self.declare_value_fn("clorus_derive", 2);
        self.declare_value_fn("clorus_underive", 2);
        self.declare_value2_to_i32_fn("clorus_isa_i32");
        self.declare_value_fn("clorus_parents", 1);
        self.declare_value_fn("clorus_ancestors", 1);
        self.declare_value_fn("clorus_descendants", 1);

        // ===== Value Comparison =====
        self.declare_value2_to_bool_fn("clorus_equals"); // General equality function

        // ===== Collection Access Functions =====
        self.declare_value_fn("clorus_get", 2);
        self.declare_value_i64_fn("clorus_nth");
        self.declare_value_fn("clorus_first", 1);
        self.declare_value_fn("clorus_rest", 1);
        self.declare_value_fn("clorus_last", 1);
        self.declare_value_to_i64_fn("clorus_count");
        self.declare_value_to_i32_fn("clorus_value_is_nil");

        // ===== Function-Related (Complex signatures - declared manually) =====
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // clorus_function_new(func_ptr: *const u8, arity: i32, env: *const *mut Value, env_size: u32) -> *mut Value
        let function_new_type = i8_ptr_type.fn_type(
            &[
                i8_ptr_type.into(),
                self.context.i32_type().into(),
                i8_ptr_type.ptr_type(AddressSpace::default()).into(),
                self.context.i32_type().into(),
            ],
            false,
        );
        self.module
            .add_function("clorus_function_new", function_new_type, None);

        // clorus_function_call(func: *mut Value, args: *const *mut Value, arg_count: i32) -> *mut Value
        let function_call_type = i8_ptr_type.fn_type(
            &[
                i8_ptr_type.into(),
                i8_ptr_type.ptr_type(AddressSpace::default()).into(),
                self.context.i32_type().into(),
            ],
            false,
        );
        self.module
            .add_function("clorus_function_call", function_call_type, None);

        // clorus_multi_arity_function_new(arities: *const ArityVariant, arity_count: u32, env: *const *mut Value, env_size: u32) -> *mut Value
        let multi_arity_function_new_type = i8_ptr_type.fn_type(
            &[
                i8_ptr_type.into(),             // arities pointer (treated as opaque)
                self.context.i32_type().into(), // arity_count
                i8_ptr_type.ptr_type(AddressSpace::default()).into(), // env pointer
                self.context.i32_type().into(), // env_size
            ],
            false,
        );
        self.module.add_function(
            "clorus_multi_arity_function_new",
            multi_arity_function_new_type,
            None,
        );

        // clorus_multi_arity_function_call(func: *mut Value, args: *const *mut Value, arg_count: i32) -> *mut Value
        let multi_arity_function_call_type = i8_ptr_type.fn_type(
            &[
                i8_ptr_type.into(),
                i8_ptr_type.ptr_type(AddressSpace::default()).into(),
                self.context.i32_type().into(),
            ],
            false,
        );
        self.module.add_function(
            "clorus_multi_arity_function_call",
            multi_arity_function_call_type,
            None,
        );

        // ===== Exception Handling (C++ ABI) =====
        // __cxa_allocate_exception(size: size_t) -> *mut i8
        let allocate_exception_type = i8_ptr_type.fn_type(&[self.context.i64_type().into()], false);
        self.module
            .add_function("__cxa_allocate_exception", allocate_exception_type, None);

        // __cxa_throw(exception: *mut i8, tinfo: *mut i8, destructor: *mut i8) -> void
        let throw_type = self.context.void_type().fn_type(
            &[i8_ptr_type.into(), i8_ptr_type.into(), i8_ptr_type.into()],
            false,
        );
        self.module.add_function("__cxa_throw", throw_type, None);

        self.declare_value_fn("__cxa_begin_catch", 1);
        self.declare_no_arg_void_fn("__cxa_end_catch");

        // __gxx_personality_v0 - C++ personality function (variadic)
        let personality_type = self.context.i32_type().fn_type(&[], true);
        self.module
            .add_function("__gxx_personality_v0", personality_type, None);

        // ===== Map Operations =====
        self.declare_value_fn("clorus_map_dissoc", 2);
        self.declare_value_fn("clorus_map_keys", 1);
        self.declare_value_fn("clorus_map_vals", 1);
        self.declare_value_fn("clorus_map_merge", 1);
        self.declare_value_fn("clorus_map_get_in", 2);
        self.declare_value_fn("clorus_map_get_in_or", 3);
        self.declare_value_fn("clorus_map_assoc_in", 3);
        self.declare_value_fn("clorus_map_update", 3);

        // ===== Sequential Operations =====
        self.declare_value_i64_fn("clorus_take");
        self.declare_value_i64_fn("clorus_drop");
        self.declare_value_fn("clorus_concat", 1);
        self.declare_value_fn("clorus_interleave", 1);
        self.declare_value_fn("clorus_interpose", 2);

        // ===== Deduplication =====
        self.declare_value_fn("clorus_distinct", 1);
        self.declare_value_fn("clorus_dedupe", 1);

        // ===== Flattening =====
        self.declare_value_fn("clorus_flatten", 1);

        // ===== String Operations =====
        self.declare_value_fn("clorus_str", 1);
        self.declare_value_i64_fn("clorus_subs2");
        self.declare_value_i64_i64_fn("clorus_subs3");
        self.declare_value_fn("clorus_split", 2);
        self.declare_value_fn("clorus_join", 2);
        self.declare_value_fn("clorus_upper_case", 1);
        self.declare_value_fn("clorus_lower_case", 1);
        self.declare_value_fn("clorus_trim", 1);
        self.declare_value_fn("clorus_trim_left", 1);
        self.declare_value_fn("clorus_trim_right", 1);
        self.declare_value_fn("clorus_replace", 3);
        self.declare_value_fn("clorus_replace_first", 3);
        self.declare_value_fn("clorus_re_find", 2);
        self.declare_value_fn("clorus_re_matches", 2);
        self.declare_value_fn("clorus_re_seq", 2);
        self.declare_value_fn("clorus_re_replace", 3);
        self.declare_value_fn("clorus_re_replace_first", 3);
        self.declare_value_to_bool_fn("clorus_is_string");
        self.declare_value_to_bool_fn("clorus_is_number");
        self.declare_value_to_bool_fn("clorus_is_vector");
        self.declare_value_to_bool_fn("clorus_is_list");
        self.declare_value_to_bool_fn("clorus_is_map");
        self.declare_value_to_bool_fn("clorus_is_set");
        self.declare_value_to_bool_fn("clorus_is_keyword");
        self.declare_value_to_bool_fn("clorus_is_symbol");
        self.declare_value_to_bool_fn("clorus_is_nil");
        self.declare_value_to_bool_fn("clorus_is_bool");
        self.declare_value_to_bool_fn("clorus_is_seq"); // seq? predicate
        self.declare_value_to_bool_fn("clorus_is_coll"); // coll? predicate
        self.declare_value_to_bool_fn("clorus_is_fn"); // fn? predicate
        self.declare_value_to_i32_fn("clorus_is_string_i32");
        self.declare_value_to_i32_fn("clorus_is_number_i32");
        self.declare_value_to_i32_fn("clorus_is_vector_i32");
        self.declare_value_to_i32_fn("clorus_is_list_i32");
        self.declare_value_to_i32_fn("clorus_is_map_i32");
        self.declare_value_to_i32_fn("clorus_is_set_i32");
        self.declare_value_to_i32_fn("clorus_is_keyword_i32");
        self.declare_value_to_i32_fn("clorus_is_symbol_i32");
        self.declare_value_to_i32_fn("clorus_is_nil_i32");
        self.declare_value_to_i32_fn("clorus_is_bool_i32");
        self.declare_value_to_i32_fn("clorus_is_seq_i32");
        self.declare_value_to_i32_fn("clorus_is_coll_i32");
        self.declare_value_to_i32_fn("clorus_is_fn_i32");
        self.declare_value_to_i32_fn("clorus_is_var_i32");
        self.declare_value_to_i32_fn("clorus_regex_valid_i32");
        self.declare_value2_to_i32_fn("clorus_starts_with");
        self.declare_value2_to_i32_fn("clorus_ends_with");

        // clorus_string_data(Value*) -> *const c_char
        // Extract C string from String Value* for protocol dispatch
        let string_data_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module
            .add_function("clorus_string_data", string_data_type, None);

        // ===== Protocol Dispatch =====
        // clorus_register_protocol_method(type_name: *const c_char, protocol_name: *const c_char,
        //                                  method_name: *const c_char, fn_ptr: usize) -> void
        let register_proto_type = self.context.void_type().fn_type(
            &[
                i8_ptr_type.into(),             // type_name
                i8_ptr_type.into(),             // protocol_name
                i8_ptr_type.into(),             // method_name
                self.context.i64_type().into(), // fn_ptr
            ],
            false,
        );
        self.module
            .add_function("clorus_register_protocol_method", register_proto_type, None);

        // clorus_lookup_protocol_method(type_name: *const c_char, protocol_name: *const c_char,
        //                                method_name: *const c_char) -> usize
        let lookup_proto_type = self.context.i64_type().fn_type(
            &[
                i8_ptr_type.into(), // type_name
                i8_ptr_type.into(), // protocol_name
                i8_ptr_type.into(), // method_name
            ],
            false,
        );
        self.module
            .add_function("clorus_lookup_protocol_method", lookup_proto_type, None);
        let protocol_satisfies_type = self.context.i32_type().fn_type(
            &[
                i8_ptr_type.into(), // type_name
                i8_ptr_type.into(), // protocol_name
            ],
            false,
        );
        self.module.add_function(
            "clorus_protocol_satisfies_type_i32",
            protocol_satisfies_type,
            None,
        );
        self.declare_value2_to_i32_fn("clorus_includes");
        self.declare_value2_to_i64_fn("clorus_compare_strings");
        self.declare_value2_to_i64_fn("clorus_compare_values");

        // String utility functions
        self.declare_value_and_i64_to_value_fn("clorus_char_at");
        self.declare_value2_to_value_fn("clorus_index_of");

        // ===== Arithmetic Operations =====
        self.declare_value_fn("clorus_add", 2);
        self.declare_value_fn("clorus_sub", 2);
        self.declare_value_fn("clorus_mul", 2);
        self.declare_value_fn("clorus_div", 2);
        self.declare_value_fn("clorus_mod", 2);

        // ===== Comparison Operations =====
        self.declare_value_fn("clorus_lt", 2);
        self.declare_value_fn("clorus_lte", 2);
        self.declare_value_fn("clorus_gt", 2);
        self.declare_value_fn("clorus_gte", 2);

        // ===== Bitwise Operations =====
        self.declare_value_fn("clorus_bit_and", 2);
        self.declare_value_fn("clorus_bit_or", 2);
        self.declare_value_fn("clorus_bit_xor", 2);
        self.declare_value_fn("clorus_bit_shift_left", 2);
        self.declare_value_fn("clorus_bit_shift_right", 2);
        self.declare_value_fn("clorus_bit_not", 1);

        // ===== I/O Operations =====
        self.declare_value_fn("clorus_print_value", 1);

        // ===== Transducer Support =====
        self.declare_value_fn("clorus_reduced", 1); // Wrap value as reduced
        self.declare_value_to_bool_fn("clorus_is_reduced"); // Check if value is reduced
        self.declare_value_fn("clorus_deref_reduced", 1); // Extract value from reduced
        self.declare_value_fn("clorus_ensure_reduced", 1); // Ensure value is reduced
    }

    /// Declare clorus.core module functions (Clojure-style)
    fn declare_core_functions(&mut self) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i32_type = self.context.i32_type();

        // clorus_slurp(path: *const c_char) -> *mut c_char
        let slurp_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_slurp", slurp_type, None);

        // clorus_spit(path: *const c_char, content: *const c_char) -> i32
        let spit_type = i32_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function("clorus_spit", spit_type, None);

        // clorus_free_string(s: *mut c_char)
        let free_string_type = self
            .context
            .void_type()
            .fn_type(&[i8_ptr_type.into()], false);
        self.module
            .add_function("clorus_free_string", free_string_type, None);

        // clorus_print(val: *mut Value) -> *mut Value
        let print_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_print", print_type, None);

        // clorus_println(val: *mut Value) -> *mut Value
        let println_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module
            .add_function("clorus_println", println_type, None);

        // clorus_println_variadic(args: *mut Value) -> *mut Value (takes vector of args)
        let println_variadic_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module
            .add_function("clorus_println_variadic", println_variadic_type, None);
    }

    /// Helper: Create a Value from an f64
    fn create_value_from_float(
        &self,
        float_val: FloatValue<'ctx>,
    ) -> inkwell::values::PointerValue<'ctx> {
        let value_double_fn = self
            .module
            .get_function("clorus_value_double")
            .expect("clorus_value_double not declared - runtime functions not initialized");
        let call_result = self
            .builder
            .build_call(value_double_fn, &[float_val.into()], "value_from_float")
            .expect("Failed to build call to clorus_value_double");
        call_result
            .try_as_basic_value()
            .left()
            .expect("clorus_value_double should return a value, got void")
            .into_pointer_value()
    }

    /// Helper: Extract f64 from a Value
    fn extract_float_from_value(
        &self,
        value_ptr: inkwell::values::PointerValue<'ctx>,
    ) -> FloatValue<'ctx> {
        let value_as_number_fn = self
            .module
            .get_function("clorus_value_as_number")
            .expect("clorus_value_as_number not declared - runtime functions not initialized");
        let call_result = self
            .builder
            .build_call(value_as_number_fn, &[value_ptr.into()], "number_from_value")
            .expect("Failed to build call to clorus_value_as_number");
        call_result
            .try_as_basic_value()
            .left()
            .expect("clorus_value_as_number should return f64, got void")
            .into_float_value()
    }

    /// Helper: Box an f64 into a Value* (alias for consistency)
    fn box_number(&self, float_val: FloatValue<'ctx>) -> PointerValue<'ctx> {
        self.create_value_from_float(float_val)
    }

    /// Helper: Unbox a Value* to f64 (alias for consistency)
    fn unbox_number(&self, value_ptr: PointerValue<'ctx>) -> FloatValue<'ctx> {
        self.extract_float_from_value(value_ptr)
    }

    /// Helper: Box a C string pointer into Value*
    fn box_string(&self, str_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        let value_string_fn = self
            .module
            .get_function("clorus_value_string")
            .expect("clorus_value_string not declared - runtime functions not initialized");
        let call_result = self
            .builder
            .build_call(value_string_fn, &[str_ptr.into()], "box_string")
            .expect("Failed to build call to clorus_value_string");
        call_result
            .try_as_basic_value()
            .left()
            .expect("clorus_value_string should return pointer, got void")
            .into_pointer_value()
    }

    /// Helper: Extract C string from Value*
    fn extract_cstring_from_value(&self, value_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        let value_as_cstring_fn = self
            .module
            .get_function("clorus_value_as_cstring")
            .expect("clorus_value_as_cstring not declared - runtime functions not initialized");
        let call_result = self
            .builder
            .build_call(value_as_cstring_fn, &[value_ptr.into()], "extract_cstring")
            .expect("Failed to build call to clorus_value_as_cstring");
        call_result
            .try_as_basic_value()
            .left()
            .expect("clorus_value_as_cstring should return pointer, got void")
            .into_pointer_value()
    }

    /// Helper: Call a runtime function and return pointer result
    /// This encapsulates the common pattern: get_function + build_call + type conversion
    /// Eliminates unwrap/expect boilerplate
    fn call_runtime_fn(
        &self,
        fn_name: &str,
        args: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
        call_name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let func = self.module.get_function(fn_name).ok_or_else(|| {
            format!(
                "{} not declared - did you forget to call declare_runtime_functions()?",
                fn_name
            )
        })?;

        let call_result = self
            .builder
            .build_call(func, args, call_name)
            .map_err(|e| format!("Failed to build call to {}: {:?}", fn_name, e))?;

        call_result
            .try_as_basic_value()
            .left()
            .ok_or_else(|| format!("{} returned void, expected value", fn_name))
            .map(|v| v.into_pointer_value())
    }

    /// Helper: Call a runtime function (non-Result version with expect for infallible calls)
    /// Use this when the function call is guaranteed to succeed (helper functions)
    fn call_runtime_fn_unchecked(
        &self,
        fn_name: &str,
        args: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
        call_name: &str,
    ) -> PointerValue<'ctx> {
        self.call_runtime_fn(fn_name, args, call_name)
            .expect(&format!(
                "Infallible call to {} failed - this is a compiler bug",
                fn_name
            ))
    }

    // ===== Function Declaration Helpers =====
    // These reduce boilerplate in declare_runtime_functions()

    /// Declare a function that takes Value* args and returns void
    fn declare_void_fn(&mut self, name: &str, num_args: usize) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let args: Vec<_> = (0..num_args).map(|_| i8_ptr_type.into()).collect();
        let fn_type = self.context.void_type().fn_type(&args, false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* args and returns Value*
    fn declare_value_fn(&mut self, name: &str, num_args: usize) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let args: Vec<_> = (0..num_args).map(|_| i8_ptr_type.into()).collect();
        let fn_type = i8_ptr_type.fn_type(&args, false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns i64
    fn declare_value_to_i64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i64_type()
            .fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and i64, returns Value*
    fn declare_value_i64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes no args and returns Value*
    fn declare_no_arg_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes i64 and returns Value*
    fn declare_i64_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[self.context.i64_type().into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes f64 and returns Value*
    fn declare_f64_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[self.context.f64_type().into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes bool and returns Value*
    fn declare_bool_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[self.context.bool_type().into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes i8* pointer and returns Value*
    fn declare_ptr_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns i8* pointer
    fn declare_value_to_ptr_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes no args and returns bool
    fn declare_no_arg_bool_fn(&mut self, name: &str) {
        let fn_type = self.context.bool_type().fn_type(&[], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes no args and returns void
    fn declare_no_arg_void_fn(&mut self, name: &str) {
        let fn_type = self.context.void_type().fn_type(&[], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns f64
    fn declare_value_to_f64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .f64_type()
            .fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns i32 (typically bool)
    fn declare_value_to_i32_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns C bool (i8)
    /// Rust `extern "C" fn(...) -> bool` uses C `_Bool` ABI, which is byte-sized.
    fn declare_value_to_bool_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self.context.i8_type().fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes 2 Value* args and returns i32 (typically bool)
    fn declare_value2_to_i32_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes 2 Value* args and returns i64
    fn declare_value2_to_i64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i64_type()
            .fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes 2 Value* args and returns C bool (i8)
    fn declare_value2_to_bool_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i8_type()
            .fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes 2 Value* args and returns Value*
    fn declare_value2_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and i64, returns Value*
    fn declare_value_and_i64_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and 2 i64s, returns Value*
    fn declare_value_i64_i64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i8_ptr_type.fn_type(
            &[i8_ptr_type.into(), i64_type.into(), i64_type.into()],
            false,
        );
        self.module.add_function(name, fn_type, None);
    }

    /// Create a stack allocation for a variable (stores Value*)
    fn create_entry_block_alloca(&self, name: &str) -> PointerValue<'ctx> {
        let builder = self.context.create_builder();

        // Find the entry block of the current function
        let function = self
            .builder
            .get_insert_block()
            .and_then(|block| block.get_parent())
            .expect("No parent function");

        let entry = function.get_first_basic_block().unwrap();

        match entry.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry),
        }

        // Allocate Value* pointer (i8* in LLVM)
        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        builder.build_alloca(value_ptr_type, name).unwrap()
    }

    /// If the given value is a Var, auto-deref to its root value.
    /// Symbol reads should behave as var dereference by default.
    fn maybe_deref_var_value(
        &mut self,
        val_ptr: PointerValue<'ctx>,
        label: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let deref_fn = self
            .module
            .get_function("clorus_deref_var_value")
            .ok_or("clorus_deref_var_value not declared")?;
        let result = self
            .builder
            .build_call(deref_fn, &[val_ptr.into()], &format!("{}_deref_var", label))
            .unwrap();
        Ok(result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }

    /// Set up exception handling personality function for a function
    fn set_personality_function(&self, function: FunctionValue<'ctx>) {
        let personality_fn = self
            .module
            .get_function("__gxx_personality_v0")
            .expect("__gxx_personality_v0 not declared");
        function.set_personality_function(personality_fn);
    }

    /// Destructure a pattern and bind all variables
    /// Returns list of (var_name, alloca) for cleanup tracking
    fn destructure_pattern(
        &mut self,
        pattern: &Pattern,
        value: PointerValue<'ctx>,
    ) -> Result<Vec<(String, PointerValue<'ctx>)>, String> {
        let mut bindings = Vec::new();

        match pattern {
            Pattern::Symbol(name) => {
                // Simple binding - store value to variable
                let alloca = self.create_entry_block_alloca(name);
                self.builder.build_store(alloca, value).unwrap();
                self.variables.insert(name.clone(), alloca);
                bindings.push((name.clone(), alloca));
            }

            Pattern::Ignore => {
                // No binding - just evaluate for side effects
            }

            Pattern::Vector {
                elements,
                rest,
                as_binding: _,
            } => {
                // Get clorus_vector_nth function
                let nth_fn = self
                    .module
                    .get_function("clorus_vector_nth")
                    .ok_or("clorus_vector_nth not declared")?;

                // Extract each element
                for (i, elem_pattern) in elements.iter().enumerate() {
                    let index = self.context.i64_type().const_int(i as u64, false);
                    let elem_val = self
                        .builder
                        .build_call(
                            nth_fn,
                            &[value.into(), index.into()],
                            &format!("vec_elem_{}", i),
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Recursively destructure
                    let nested = self.destructure_pattern(elem_pattern, elem_val)?;
                    bindings.extend(nested);
                }

                // Handle rest parameter
                if let Some(rest_name) = rest {
                    // Get clorus_vector_rest function (takes vector and start index)
                    let rest_fn = self
                        .module
                        .get_function("clorus_vector_rest")
                        .ok_or("clorus_vector_rest not declared")?;

                    let start_index = self
                        .context
                        .i64_type()
                        .const_int(elements.len() as u64, false);
                    let rest_val = self
                        .builder
                        .build_call(rest_fn, &[value.into(), start_index.into()], "vec_rest")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Bind rest parameter
                    let alloca = self.create_entry_block_alloca(rest_name);
                    self.builder.build_store(alloca, rest_val).unwrap();
                    self.variables.insert(rest_name.clone(), alloca);
                    bindings.push((rest_name.clone(), alloca));
                }
            }

            Pattern::Map {
                bindings: pattern_bindings,
                defaults: _,
            } => {
                // Get map_get and keyword functions
                let get_fn = self
                    .module
                    .get_function("clorus_map_get")
                    .ok_or("clorus_map_get not declared")?;
                let keyword_fn = self
                    .module
                    .get_function("clorus_keyword")
                    .ok_or("clorus_keyword not declared")?;

                for (key, value_pattern) in pattern_bindings {
                    // Get key string
                    let key_str = match key {
                        MapPatternKey::Keyword(k) => k,
                        MapPatternKey::Symbol(s) => s,
                        MapPatternKey::Str(s) => s,
                        MapPatternKey::Sym(s) => s,
                    };

                    // Create keyword for lookup
                    let keyword_c_str = self
                        .builder
                        .build_global_string_ptr(key_str, "key")
                        .unwrap();
                    let keyword_val = self
                        .builder
                        .build_call(
                            keyword_fn,
                            &[keyword_c_str.as_pointer_value().into()],
                            "keyword",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Get value from map
                    let map_val = self
                        .builder
                        .build_call(
                            get_fn,
                            &[value.into(), keyword_val.into()],
                            &format!("map_get_{}", key_str),
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Recursively destructure
                    let nested = self.destructure_pattern(value_pattern, map_val)?;
                    bindings.extend(nested);
                }
            }
        }

        Ok(bindings)
    }

    /// Collect all variable names from a pattern (for loop recur)
    fn collect_pattern_names(pattern: &Pattern) -> Vec<String> {
        match pattern {
            Pattern::Symbol(name) => vec![name.clone()],
            Pattern::Ignore => vec![],
            Pattern::Vector {
                elements,
                rest,
                as_binding: _,
            } => {
                let mut names = Vec::new();
                for elem in elements {
                    names.extend(Self::collect_pattern_names(elem));
                }
                if let Some(rest_name) = rest {
                    names.push(rest_name.clone());
                }
                names
            }
            Pattern::Map {
                bindings,
                defaults: _,
            } => {
                let mut names = Vec::new();
                for (_, value_pattern) in bindings {
                    names.extend(Self::collect_pattern_names(value_pattern));
                }
                names
            }
        }
    }

    /// Generate a record/type constructor function: ->TypeName
    /// Creates a function that takes field values and returns a map
    /// Shared by defrecord and deftype implementations
    fn generate_record_constructor(
        &mut self,
        type_name: &str,
        fields: &[String],
    ) -> Result<(), String> {
        let constructor_name = format!("->{}", type_name);
        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // Create parameter types (one Value* for each field)
        let param_types: Vec<_> = fields.iter().map(|_| value_ptr_type.into()).collect();

        let fn_type = value_ptr_type.fn_type(&param_types, false);
        let function = self.module.add_function(&constructor_name, fn_type, None);

        // Add function to table for calls
        self.functions.insert(constructor_name.clone(), function);

        // Save current state
        let saved_vars = self.variables.clone();
        let saved_block = self.builder.get_insert_block();

        // Create entry block for constructor
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        self.variables.clear();

        // Create empty map to hold record fields
        let map_empty_fn = self
            .module
            .get_function("clorus_map_empty")
            .ok_or("clorus_map_empty not declared")?;
        let mut map_val = self
            .builder
            .build_call(map_empty_fn, &[], "record_map")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Get map_assoc and keyword functions
        let map_assoc_fn = self
            .module
            .get_function("clorus_map_assoc")
            .ok_or("clorus_map_assoc not declared")?;
        let keyword_fn = self
            .module
            .get_function("clorus_keyword")
            .ok_or("clorus_keyword not declared")?;

        // For each field, add keyword->value pair to map
        for (i, field_name) in fields.iter().enumerate() {
            // Get parameter value for this field
            let param_val = function
                .get_nth_param(i as u32)
                .unwrap()
                .into_pointer_value();

            // Create keyword for field name
            let field_c_str = self
                .builder
                .build_global_string_ptr(field_name, "field_name")
                .unwrap();
            let keyword_val = self
                .builder
                .build_call(
                    keyword_fn,
                    &[field_c_str.as_pointer_value().into()],
                    &format!("field_keyword_{}", i),
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();

            // Add keyword->value pair to map
            map_val = self
                .builder
                .build_call(
                    map_assoc_fn,
                    &[map_val.into(), keyword_val.into(), param_val.into()],
                    &format!("record_map_{}", i),
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();
        }

        // Add __type__ field for runtime protocol dispatch
        // This lets the runtime identify what type an instance is
        let type_keyword_str = self
            .builder
            .build_global_string_ptr("__type__", "type_keyword_name")
            .unwrap();
        let type_keyword = self
            .builder
            .build_call(
                keyword_fn,
                &[type_keyword_str.as_pointer_value().into()],
                "type_keyword",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Create string value for type name
        let type_name_str = self
            .builder
            .build_global_string_ptr(type_name, "type_name_str")
            .unwrap();
        let string_fn = self
            .module
            .get_function("clorus_value_string")
            .ok_or("clorus_value_string not declared")?;
        let type_name_val = self
            .builder
            .build_call(
                string_fn,
                &[type_name_str.as_pointer_value().into()],
                "type_name_val",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Add __type__ -> "TypeName" to map
        map_val = self
            .builder
            .build_call(
                map_assoc_fn,
                &[map_val.into(), type_keyword.into(), type_name_val.into()],
                "record_with_type",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Return the constructed record (map)
        self.builder.build_return(Some(&map_val)).unwrap();

        // Restore previous state
        self.variables = saved_vars;
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }

        Ok(())
    }

    /// Compile an expression to a Value*
    /// All expressions now return boxed values
    pub fn compile_expr(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
        // Check for compilation timeout
        self.check_timeout()?;

        // Increment expression counter for progress tracking
        self.expr_count += 1;

        // Ensure we have an insertion point for top-level expressions (REPL/JIT).
        self.ensure_global_init_block();

        // Show progress every 100 expressions (disabled for cleaner output)
        // Uncomment for debugging large compilations
        // if self.expr_count % 100 == 0 {
        //     let elapsed = self.compile_start.elapsed();
        //     eprintln!("   [PROGRESS] Compiled {} expressions ({:.1}s elapsed)",
        //              self.expr_count, elapsed.as_secs_f32());
        // }

        match expr {
            Expr::Long(n) => {
                // Box long integer into Value* using clorus_value_long
                let value_long_fn = self
                    .module
                    .get_function("clorus_value_long")
                    .ok_or("clorus_value_long not declared")?;
                let long_val = self.context.i64_type().const_int(*n as u64, false);
                let result = self
                    .builder
                    .build_call(value_long_fn, &[long_val.into()], "value_long")
                    .unwrap();
                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::Double(n) => {
                // Box double into Value* using clorus_value_double
                let float_val = self.context.f64_type().const_float(*n);
                Ok(self.box_number(float_val))
            }

            Expr::Symbol(name) => {
                // Variable reference - check LOCALS first (to allow shadowing), then globals, then functions
                // Variables now store Value* instead of f64

                // First, try to resolve qualified names (namespace/var or alias/var)
                // Globals are now namespace-mangled like functions
                let resolved_name = if name.contains('/') {
                    let parts: Vec<&str> = name.split('/').collect();
                    if parts.len() == 2 {
                        let namespace_or_alias = parts[0];
                        let var_name = parts[1];

                        // Resolve alias to actual namespace
                        let resolved_namespace = self
                            .namespace
                            .aliases
                            .get(namespace_or_alias)
                            .map(|s| s.as_str())
                            .unwrap_or(namespace_or_alias);

                        // Generate mangled name for global lookup
                        // const/PADDING-SMALL with const=utils.constants
                        // -> clorus_utils_constants_PADDING_SMALL
                        if resolved_namespace == "user" {
                            var_name.to_string()
                        } else {
                            format!(
                                "clorus_{}_{}",
                                resolved_namespace.replace('.', "_").replace('-', "_"),
                                var_name.replace('-', "_")
                            )
                        }
                    } else {
                        name.clone()
                    }
                } else {
                    // Unqualified name - first honor :refer/:rename imports, then current namespace
                    if let Some(binding) = self.namespace.imports.get(name) {
                        format!(
                            "clorus_{}_{}",
                            binding.namespace.replace('.', "_").replace('-', "_"),
                            binding.symbol.replace('-', "_")
                        )
                    } else if self.namespace.current == "user" {
                        name.clone()
                    } else {
                        format!(
                            "clorus_{}_{}",
                            self.namespace.current.replace('.', "_").replace('-', "_"),
                            name.replace('-', "_")
                        )
                    }
                };

                // BUGFIX: Check LOCALS first to allow shadowing of globals
                // Locals are never namespace-mangled (always use simple name)
                if let Some(ptr) = self.variables.get(name) {
                    // Load Value* from local variable
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let val = self.builder.build_load(value_ptr_type, *ptr, name).unwrap();
                    let val_ptr = val.into_pointer_value();
                    self.maybe_deref_var_value(val_ptr, "symbol_local")
                } else if let Some(global) = self.globals.get(&resolved_name) {
                    // Load Value* from global variable (namespace-mangled)
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let val = self
                        .builder
                        .build_load(value_ptr_type, global.as_pointer_value(), &resolved_name)
                        .unwrap();
                    let val_ptr = val.into_pointer_value();
                    self.maybe_deref_var_value(val_ptr, "symbol_global")
                } else {
                    // Built-in arithmetic/comparison operators as first-class function values.
                    // This enables forms like `(reduce + [1 2 3])` with Clojure-like semantics.
                    if !name.contains('/') {
                        let builtin_fn = match name.as_str() {
                            "+" => Some(("clorus_add", 2)),
                            "-" => Some(("clorus_sub", 2)),
                            "*" => Some(("clorus_mul", 2)),
                            "/" => Some(("clorus_div", 2)),
                            "=" => Some(("clorus_eq", 2)),
                            "<" => Some(("clorus_lt", 2)),
                            ">" => Some(("clorus_gt", 2)),
                            "<=" => Some(("clorus_lte", 2)),
                            ">=" => Some(("clorus_gte", 2)),
                            _ => None,
                        };

                        if let Some((runtime_name, arity)) = builtin_fn {
                            if let Some(function) = self.module.get_function(runtime_name) {
                                let func_new_fn = self
                                    .module
                                    .get_function("clorus_function_new")
                                    .ok_or("clorus_function_new not declared")?;

                                let func_ptr = function.as_global_value().as_pointer_value();
                                let i8_ptr_type =
                                    self.context.i8_type().ptr_type(AddressSpace::default());
                                let func_ptr_cast = self
                                    .builder
                                    .build_pointer_cast(
                                        func_ptr,
                                        i8_ptr_type,
                                        "builtin_func_ptr_cast",
                                    )
                                    .unwrap();

                                let arity_val =
                                    self.context.i32_type().const_int(arity as u64, true);
                                let value_ptr_type =
                                    self.context.i8_type().ptr_type(AddressSpace::default());
                                let env_ptr = value_ptr_type.const_null();
                                let env_size = self.context.i32_type().const_int(0, false);

                                let func_val = self
                                    .builder
                                    .build_call(
                                        func_new_fn,
                                        &[
                                            func_ptr_cast.into(),
                                            arity_val.into(),
                                            env_ptr.into(),
                                            env_size.into(),
                                        ],
                                        "builtin_func_value",
                                    )
                                    .unwrap();

                                return Ok(func_val
                                    .try_as_basic_value()
                                    .left()
                                    .unwrap()
                                    .into_pointer_value());
                            }
                        }
                    }

                    // Try to find function - check both unmangled and mangled names
                    // Also handle qualified names (namespace/function or alias/function)
                    let function = if name.contains('/') {
                        // Qualified name - resolve alias and look up function
                        let parts: Vec<&str> = name.split('/').collect();
                        if parts.len() == 2 {
                            let namespace_or_alias = parts[0];
                            let func_name = parts[1];

                            // Resolve alias to actual namespace
                            let resolved_namespace = self
                                .namespace
                                .aliases
                                .get(namespace_or_alias)
                                .map(|s| s.as_str())
                                .unwrap_or(namespace_or_alias);

                            // Generate mangled name: demos.textfield-demo/render -> clorus_demos_textfield_demo_render
                            let mangled_name = format!(
                                "clorus_{}_{}",
                                resolved_namespace.replace('.', "_").replace('-', "_"),
                                func_name.replace('-', "_")
                            );

                            self.functions.get(&mangled_name).copied()
                        } else {
                            None
                        }
                    } else {
                        // Unqualified name - try direct lookup first, then mangled for current namespace
                        self.functions
                            .get(name)
                            .or_else(|| {
                                if let Some(binding) = self.namespace.imports.get(name) {
                                    let mangled_name = format!(
                                        "clorus_{}_{}",
                                        binding.namespace.replace('.', "_").replace('-', "_"),
                                        binding.symbol.replace('-', "_")
                                    );
                                    self.functions.get(&mangled_name)
                                } else {
                                    None
                                }
                            })
                            .or_else(|| {
                                // Try mangled name for current namespace
                                let mangled_name = if self.namespace.current == "user" {
                                    name.clone()
                                } else {
                                    format!(
                                        "clorus_{}_{}",
                                        self.namespace.current.replace('.', "_").replace('-', "_"),
                                        name.replace('-', "_")
                                    )
                                };
                                self.functions.get(&mangled_name)
                            })
                            .copied()
                    };

                    if let Some(function) = function {
                        // Function reference - wrap in function value
                        // Call clorus_function_new with the function pointer and arity
                        let func_new_fn = self
                            .module
                            .get_function("clorus_function_new")
                            .ok_or("clorus_function_new not declared")?;

                        // Get function pointer (cast to *const u8)
                        let func_ptr = function.as_global_value().as_pointer_value();
                        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                        let func_ptr_cast = self
                            .builder
                            .build_pointer_cast(func_ptr, i8_ptr_type, "func_ptr_cast")
                            .unwrap();

                        let fn_name = function.get_name().to_string_lossy().to_string();
                        let (fixed_params, has_rest) = self
                            .function_signatures
                            .get(&fn_name)
                            .copied()
                            .unwrap_or_else(|| {
                                // Fallback for externally declared functions
                                let fn_type = function.get_type();
                                let param_count = fn_type.count_param_types();
                                let fixed = if param_count > 0 {
                                    (param_count - 1) as usize
                                } else {
                                    0
                                };
                                (fixed, false)
                            });
                        let runtime_arity = Self::runtime_arity(fixed_params, has_rest);
                        let arity_val = self
                            .context
                            .i32_type()
                            .const_int(runtime_arity as u64, true);

                        // Empty environment (nullptr)
                        let value_ptr_type =
                            self.context.i8_type().ptr_type(AddressSpace::default());
                        let env_ptr = value_ptr_type.const_null();

                        // env_size = 0
                        let env_size = self.context.i32_type().const_int(0, false);

                        // Call clorus_function_new
                        let func_val = self
                            .builder
                            .build_call(
                                func_new_fn,
                                &[
                                    func_ptr_cast.into(),
                                    arity_val.into(),
                                    env_ptr.into(),
                                    env_size.into(),
                                ],
                                "func_value",
                            )
                            .unwrap();

                        Ok(func_val
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value())
                    } else {
                        Err(format!("Undefined variable: {}", name))
                    }
                }
            }

            Expr::SetBang { target, value } => {
                let new_value = self.compile_expr(value)?;

                // Local assignment path (including binding-expanded locals).
                if let Some(ptr) = self.variables.get(target) {
                    self.builder.build_store(*ptr, new_value).unwrap();
                    return Ok(new_value);
                }

                // Resolve global target (qualified, imported, or current namespace).
                let resolved_name = if target.contains('/') {
                    let parts: Vec<&str> = target.split('/').collect();
                    if parts.len() == 2 {
                        let namespace_or_alias = parts[0];
                        let var_name = parts[1];
                        let resolved_namespace = self
                            .namespace
                            .aliases
                            .get(namespace_or_alias)
                            .map(|s| s.as_str())
                            .unwrap_or(namespace_or_alias);
                        if resolved_namespace == "user" {
                            var_name.to_string()
                        } else {
                            format!(
                                "clorus_{}_{}",
                                resolved_namespace.replace('.', "_").replace('-', "_"),
                                var_name.replace('-', "_")
                            )
                        }
                    } else {
                        target.clone()
                    }
                } else if let Some(binding) = self.namespace.imports.get(target) {
                    format!(
                        "clorus_{}_{}",
                        binding.namespace.replace('.', "_").replace('-', "_"),
                        binding.symbol.replace('-', "_")
                    )
                } else if self.namespace.current == "user" {
                    target.clone()
                } else {
                    format!(
                        "clorus_{}_{}",
                        self.namespace.current.replace('.', "_").replace('-', "_"),
                        target.replace('-', "_")
                    )
                };

                let global = self
                    .globals
                    .get(&resolved_name)
                    .ok_or_else(|| format!("set! target is not assignable: {}", target))?;
                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let current_raw = self
                    .builder
                    .build_load(
                        value_ptr_type,
                        global.as_pointer_value(),
                        &format!("set_current_{}", resolved_name),
                    )
                    .unwrap()
                    .into_pointer_value();

                // If global contains a Var wrapper, mutate root via clorus_var_set.
                let is_var_fn = self
                    .module
                    .get_function("clorus_is_var_i32")
                    .ok_or("clorus_is_var_i32 not declared")?;
                let as_var_fn = self
                    .module
                    .get_function("clorus_value_as_var")
                    .ok_or("clorus_value_as_var not declared")?;
                let var_set_fn = self
                    .module
                    .get_function("clorus_var_set")
                    .ok_or("clorus_var_set not declared")?;

                let is_var_i32 = self
                    .builder
                    .build_call(is_var_fn, &[current_raw.into()], "set_is_var_i32")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let is_var = self
                    .builder
                    .build_int_compare(
                        IntPredicate::NE,
                        is_var_i32,
                        self.context.i32_type().const_zero(),
                        "set_is_var",
                    )
                    .unwrap();

                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|bb| bb.get_parent())
                    .ok_or("No parent function for set!")?;
                let then_bb = self.context.append_basic_block(current_fn, "set_var_then");
                let else_bb = self.context.append_basic_block(current_fn, "set_var_else");
                let merge_bb = self.context.append_basic_block(current_fn, "set_var_merge");
                self.builder
                    .build_conditional_branch(is_var, then_bb, else_bb)
                    .unwrap();

                self.builder.position_at_end(then_bb);
                let var_ptr = self
                    .builder
                    .build_call(as_var_fn, &[current_raw.into()], "set_var_ptr")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                let set_res = self
                    .builder
                    .build_call(
                        var_set_fn,
                        &[var_ptr.into(), new_value.into()],
                        "set_var_root",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                let then_end = self.builder.get_insert_block().unwrap();
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(else_bb);
                self.builder
                    .build_store(global.as_pointer_value(), new_value)
                    .unwrap();
                let else_end = self.builder.get_insert_block().unwrap();
                self.builder.build_unconditional_branch(merge_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                let assigned = self
                    .builder
                    .build_phi(value_ptr_type, "set_assigned")
                    .unwrap();
                assigned.add_incoming(&[(&set_res, then_end), (&new_value, else_end)]);
                Ok(assigned.as_basic_value().into_pointer_value())
            }

            Expr::Var { name } => {
                // Return Var object itself (no auto-deref).
                // Supports #'x / #'ns/x for metadata and var operations.
                let resolved_name = if name.contains('/') {
                    let parts: Vec<&str> = name.split('/').collect();
                    if parts.len() == 2 {
                        let namespace_or_alias = parts[0];
                        let var_name = parts[1];
                        let resolved_namespace = self
                            .namespace
                            .aliases
                            .get(namespace_or_alias)
                            .map(|s| s.as_str())
                            .unwrap_or(namespace_or_alias);

                        if resolved_namespace == "user" {
                            var_name.to_string()
                        } else {
                            format!(
                                "clorus_{}_{}",
                                resolved_namespace.replace('.', "_").replace('-', "_"),
                                var_name.replace('-', "_")
                            )
                        }
                    } else {
                        name.clone()
                    }
                } else if self.namespace.current == "user" {
                    name.clone()
                } else {
                    format!(
                        "clorus_{}_{}",
                        self.namespace.current.replace('.', "_").replace('-', "_"),
                        name.replace('-', "_")
                    )
                };

                if let Some(global) = self.globals.get(&resolved_name) {
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let val = self
                        .builder
                        .build_load(
                            value_ptr_type,
                            global.as_pointer_value(),
                            &format!("{}_var", resolved_name),
                        )
                        .unwrap()
                        .into_pointer_value();

                    let is_var_fn = self
                        .module
                        .get_function("clorus_is_var_i32")
                        .ok_or("clorus_is_var_i32 not declared")?;
                    let retain_fn = self
                        .module
                        .get_function("clorus_retain")
                        .ok_or("clorus_retain not declared")?;
                    let var_new_fn = self
                        .module
                        .get_function("clorus_var_new")
                        .ok_or("clorus_var_new not declared")?;
                    let value_from_var_fn = self
                        .module
                        .get_function("clorus_value_from_var")
                        .ok_or("clorus_value_from_var not declared")?;

                    let is_var = self
                        .builder
                        .build_call(is_var_fn, &[val.into()], "is_var_quote")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_int_value();
                    let zero = self.context.i32_type().const_zero();
                    let is_var_bool = self
                        .builder
                        .build_int_compare(IntPredicate::NE, is_var, zero, "is_var_quote_bool")
                        .unwrap();

                    let current_fn = self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_parent()
                        .unwrap();
                    let then_block = self.context.append_basic_block(current_fn, "varq_is_var");
                    let else_block = self.context.append_basic_block(current_fn, "varq_make_var");
                    let merge_block = self.context.append_basic_block(current_fn, "varq_merge");

                    self.builder
                        .build_conditional_branch(is_var_bool, then_block, else_block)
                        .unwrap();

                    self.builder.position_at_end(then_block);
                    self.builder
                        .build_call(retain_fn, &[val.into()], "retain_existing_var")
                        .unwrap();
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .unwrap();
                    let then_end = self.builder.get_insert_block().unwrap();

                    self.builder.position_at_end(else_block);
                    let string_fn = self
                        .module
                        .get_function("clorus_value_string")
                        .ok_or("clorus_value_string not declared")?;
                    let name_cstr = self
                        .builder
                        .build_global_string_ptr(name, "var_name_cstr")
                        .expect("Failed to build var name");
                    let name_val_ptr = self
                        .builder
                        .build_call(
                            string_fn,
                            &[name_cstr.as_pointer_value().into()],
                            "var_name_val",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                    let var_ptr = self
                        .builder
                        .build_call(
                            var_new_fn,
                            &[name_val_ptr.into(), val.into()],
                            "varq_new_var",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                    let wrapped = self
                        .builder
                        .build_call(value_from_var_fn, &[var_ptr.into()], "varq_wrapped")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .unwrap();
                    let else_end = self.builder.get_insert_block().unwrap();

                    self.builder.position_at_end(merge_block);
                    let phi = self
                        .builder
                        .build_phi(value_ptr_type, "varq_result")
                        .unwrap();
                    phi.add_incoming(&[(&val, then_end), (&wrapped, else_end)]);
                    Ok(phi.as_basic_value().into_pointer_value())
                } else {
                    Err(format!("Undefined var: {}", name))
                }
            }

            Expr::String(s) => {
                // String literals now work! Box them into Value*
                let c_str = self.builder.build_global_string_ptr(s, "str").unwrap();
                Ok(self.box_string(c_str.as_pointer_value()))
            }

            Expr::Keyword(k) => {
                // Keywords use interning for fast equality
                let c_str = self.builder.build_global_string_ptr(k, "keyword").unwrap();

                // Call clorus_keyword(name) to get the interned keyword
                let keyword_fn = self
                    .module
                    .get_function("clorus_keyword")
                    .ok_or("clorus_keyword not declared")?;

                let call_result = self
                    .builder
                    .build_call(keyword_fn, &[c_str.as_pointer_value().into()], "keyword")
                    .unwrap();

                Ok(call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::Nil => {
                // Nil represented as boxed nil value
                let nil_fn = self
                    .module
                    .get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let call_result = self.builder.build_call(nil_fn, &[], "nil_value").unwrap();
                Ok(call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::Bool(b) => {
                // Bool represented as boxed bool value
                let bool_fn = self
                    .module
                    .get_function("clorus_value_bool")
                    .ok_or("clorus_value_bool not declared")?;
                let bool_val = if *b { 1.0 } else { 0.0 };
                let float_val = self.context.f64_type().const_float(bool_val);
                let call_result = self
                    .builder
                    .build_call(bool_fn, &[float_val.into()], "bool_value")
                    .unwrap();
                Ok(call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::Vector(elements) => {
                // Vector literals: [1 2 3]
                // Create empty vector
                let vec_empty_fn = self
                    .module
                    .get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let empty_vec_call = self
                    .builder
                    .build_call(vec_empty_fn, &[], "vec_empty")
                    .unwrap();
                let mut vec_val = empty_vec_call
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add each element using clorus_vector_conj
                let vec_conj_fn = self
                    .module
                    .get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                for (i, elem) in elements.iter().enumerate() {
                    // Compile the element
                    let elem_val = self.compile_expr(elem)?;

                    // Call clorus_vector_conj(vec, elem) -> new_vec
                    let conj_call = self
                        .builder
                        .build_call(
                            vec_conj_fn,
                            &[vec_val.into(), elem_val.into()],
                            &format!("vec_conj_{}", i),
                        )
                        .unwrap();

                    // Update vec_val to the new vector
                    vec_val = conj_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(vec_val)
            }

            Expr::Map(entries) => {
                // Map literals: {:key1 val1 :key2 val2}
                // Create empty map
                let map_empty_fn = self
                    .module
                    .get_function("clorus_map_empty")
                    .ok_or("clorus_map_empty not declared")?;
                let empty_map_call = self
                    .builder
                    .build_call(map_empty_fn, &[], "map_empty")
                    .unwrap();
                let mut map_val = empty_map_call
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add each key-value pair using clorus_map_assoc
                let map_assoc_fn = self
                    .module
                    .get_function("clorus_map_assoc")
                    .ok_or("clorus_map_assoc not declared")?;

                for (i, (key_expr, val_expr)) in entries.iter().enumerate() {
                    // Compile the key
                    let key_val = self.compile_expr(key_expr)?;

                    // Compile the value
                    let val_val = self.compile_expr(val_expr)?;

                    // Call clorus_map_assoc(map, key, val) -> new_map
                    let assoc_call = self
                        .builder
                        .build_call(
                            map_assoc_fn,
                            &[map_val.into(), key_val.into(), val_val.into()],
                            &format!("map_assoc_{}", i),
                        )
                        .unwrap();

                    // Update map_val to the new map
                    map_val = assoc_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(map_val)
            }

            Expr::Set(elements) => {
                // Set literals: #{1 2 3}
                // Create empty set
                let set_empty_fn = self
                    .module
                    .get_function("clorus_set_empty")
                    .ok_or("clorus_set_empty not declared")?;
                let empty_set_call = self
                    .builder
                    .build_call(set_empty_fn, &[], "set_empty")
                    .unwrap();
                let mut set_val = empty_set_call
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add each element using clorus_set_conj
                let set_conj_fn = self
                    .module
                    .get_function("clorus_set_conj")
                    .ok_or("clorus_set_conj not declared")?;

                for (i, elem) in elements.iter().enumerate() {
                    // Compile the element
                    let elem_val = self.compile_expr(elem)?;

                    // Call clorus_set_conj(set, elem) -> new_set
                    let conj_call = self
                        .builder
                        .build_call(
                            set_conj_fn,
                            &[set_val.into(), elem_val.into()],
                            &format!("set_conj_{}", i),
                        )
                        .unwrap();

                    // Update set_val to the new set
                    set_val = conj_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(set_val)
            }

            Expr::Let { bindings, body } => {
                // Save current variable scope
                let saved_vars = self.variables.clone();

                // Track local variables for cleanup
                let mut local_vars = Vec::new();

                // Process each binding with destructuring
                for (pattern, value_expr) in bindings {
                    // Compile the value
                    let value = self.compile_expr(value_expr)?;

                    // Destructure pattern and bind variables
                    let pattern_bindings = self.destructure_pattern(pattern, value)?;
                    local_vars.extend(pattern_bindings);
                }

                // Compile the body with bindings in scope
                let result = self.compile_expr(body)?;

                // Phase C-1: Scope-based memory management
                // Retain the return value so it survives scope cleanup
                let retain_fn = self
                    .module
                    .get_function("clorus_retain")
                    .ok_or("clorus_retain not declared")?;
                self.builder
                    .build_call(retain_fn, &[result.into()], "retain_result")
                    .unwrap();

                // Release local variables before exiting scope
                let release_fn = self
                    .module
                    .get_function("clorus_release")
                    .ok_or("clorus_release not declared")?;

                for (var_name, var_ptr) in &local_vars {
                    // Load the Value* from the variable
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let val = self
                        .builder
                        .build_load(value_ptr_type, *var_ptr, &format!("{}_cleanup", var_name))
                        .unwrap()
                        .into_pointer_value();

                    // Call clorus_release(val)
                    self.builder
                        .build_call(release_fn, &[val.into()], &format!("release_{}", var_name))
                        .unwrap();
                }

                // Restore previous scope (let creates local scope)
                self.variables = saved_vars;

                Ok(result)
            }

            Expr::Binding { bindings, body } => {
                let as_var_fn = self
                    .module
                    .get_function("clorus_value_as_var")
                    .ok_or("clorus_value_as_var not declared")?;
                let push_binding_fn = self
                    .module
                    .get_function("clorus_var_push_binding")
                    .ok_or("clorus_var_push_binding not declared")?;
                let pop_binding_fn = self
                    .module
                    .get_function("clorus_var_pop_binding")
                    .ok_or("clorus_var_pop_binding not declared")?;
                let retain_fn = self
                    .module
                    .get_function("clorus_retain")
                    .ok_or("clorus_retain not declared")?;

                // Track pushed var pointers so we can restore in reverse order.
                let mut bound_vars = Vec::with_capacity(bindings.len());

                for (name, value_expr) in bindings {
                    let var_value = self.compile_expr(&Expr::Var { name: name.clone() })?;
                    let var_ptr = self
                        .builder
                        .build_call(as_var_fn, &[var_value.into()], "binding_var_ptr")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                    let new_value = self.compile_expr(value_expr)?;

                    self.builder
                        .build_call(
                            push_binding_fn,
                            &[var_ptr.into(), new_value.into()],
                            "binding_push",
                        )
                        .unwrap();
                    bound_vars.push(var_ptr);
                }

                let result = self.compile_expr(body)?;
                self.builder
                    .build_call(retain_fn, &[result.into()], "retain_binding_result")
                    .unwrap();

                // Restore bindings in reverse order.
                for var_ptr in bound_vars.iter().rev() {
                    self.builder
                        .build_call(pop_binding_fn, &[(*var_ptr).into()], "binding_pop")
                        .unwrap();
                }

                Ok(result)
            }

            Expr::Letfn { bindings, body } => {
                // letfn creates local function bindings with mutual recursion support
                // Strategy:
                // 1. Save current variable scope
                // 2. Create function prototypes for all letfn functions (so they can reference each other)
                // 3. Compile all function bodies
                // 4. Create function Value* objects and bind to variables
                // 5. Compile letfn body
                // 6. Cleanup and restore scope

                // Save current variable scope
                let saved_vars = self.variables.clone();
                let saved_recur_ctx = self.current_recur_fn.clone();
                let mut local_vars = Vec::new();

                // Step 1: Create function prototypes (declarations)
                let mut fn_values = Vec::new();
                for (name, params, rest_param, _body) in bindings {
                    // Use the same calling convention as defn/fn:
                    // Value* fn(Value* arg1, ..., Value* rest_vec?, Value* env)
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let mut param_types: Vec<_> =
                        params.iter().map(|_| value_ptr_type.into()).collect();

                    if rest_param.is_some() {
                        param_types.push(value_ptr_type.into());
                    }

                    // Environment parameter (unused for letfn; kept for ABI consistency)
                    param_types.push(value_ptr_type.into());

                    let fn_type = value_ptr_type.fn_type(&param_types, false);

                    // Create function with unique name
                    let fn_name = format!("letfn_{}_{}", name, self.module.get_functions().count());
                    let function = self.module.add_function(&fn_name, fn_type, None);

                    fn_values.push((name.clone(), function, params.clone(), rest_param.clone()));
                }

                // Step 2: Compile function bodies
                for (i, (_name, function, params, rest_param)) in fn_values.iter().enumerate() {
                    let (_fn_name, _fn_params, _fn_rest, fn_body) = &bindings[i];

                    // Create entry block for function
                    let entry_block = self.context.append_basic_block(*function, "entry");
                    let saved_block = self.builder.get_insert_block();
                    self.builder.position_at_end(entry_block);

                    // Save current variables and restore with letfn functions visible
                    let saved_fn_vars = self.variables.clone();
                    self.variables = saved_vars.clone();

                    // Make all letfn functions visible to each other during compilation
                    // Create function Value* objects for each letfn function
                    let clorus_make_fn = self
                        .module
                        .get_function("clorus_function_new")
                        .ok_or("clorus_function_new not declared")?;

                    for (letfn_name, letfn_fn, letfn_params, letfn_rest) in &fn_values {
                        let runtime_arity =
                            Self::runtime_arity(letfn_params.len(), letfn_rest.is_some());
                        let arity_val = self
                            .context
                            .i32_type()
                            .const_int(runtime_arity as u64, true);

                        // Cast function pointer to *const u8
                        let fn_ptr = letfn_fn.as_global_value().as_pointer_value();

                        // No captures for letfn functions (they reference each other but not outer scope)
                        let null_env = self
                            .context
                            .i8_type()
                            .ptr_type(AddressSpace::default())
                            .ptr_type(AddressSpace::default())
                            .const_null();
                        let zero_env_size = self.context.i32_type().const_zero();

                        // Create function Value*
                        // clorus_function_new(func_ptr, arity, env, env_size)
                        let fn_value = self
                            .builder
                            .build_call(
                                clorus_make_fn,
                                &[
                                    fn_ptr.into(),
                                    arity_val.into(),
                                    null_env.into(),
                                    zero_env_size.into(),
                                ],
                                &format!("make_{}", letfn_name),
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();

                        // Bind to variable so it's accessible during compilation
                        let alloca = self.create_entry_block_alloca(letfn_name);
                        self.builder.build_store(alloca, fn_value).unwrap();
                        self.variables.insert(letfn_name.clone(), alloca);
                    }

                    // Bind fixed parameters to allocas with destructuring.
                    for (param_idx, pattern) in params.iter().enumerate() {
                        let param_val = function
                            .get_nth_param(param_idx as u32)
                            .unwrap()
                            .into_pointer_value();
                        self.destructure_pattern(pattern, param_val)?;
                    }

                    // Handle rest parameter if present
                    if let Some(rest_name) = rest_param {
                        let rest_vec = function
                            .get_nth_param(params.len() as u32)
                            .unwrap()
                            .into_pointer_value();
                        let alloca = self.create_entry_block_alloca(rest_name);
                        self.builder.build_store(alloca, rest_vec).unwrap();
                        self.variables.insert(rest_name.clone(), alloca);
                    }

                    // Compile function body
                    self.current_recur_fn = Some(RecurFnContext {
                        name: _fn_name.clone(),
                        fixed_param_count: params.len(),
                        has_rest_param: rest_param.is_some(),
                    });
                    let result = self.compile_expr(fn_body)?;
                    self.current_recur_fn = saved_recur_ctx.clone();

                    // Return the result
                    self.builder.build_return(Some(&result)).unwrap();

                    // Restore variables and block
                    self.variables = saved_fn_vars;
                    if let Some(block) = saved_block {
                        self.builder.position_at_end(block);
                    }
                }

                // Step 3: In the outer scope, create function Value* objects and bind to variables
                let clorus_make_fn = self
                    .module
                    .get_function("clorus_function_new")
                    .ok_or("clorus_function_new not declared")?;

                for (name, function, params, rest_param) in &fn_values {
                    let runtime_arity = Self::runtime_arity(params.len(), rest_param.is_some());
                    let arity_val = self
                        .context
                        .i32_type()
                        .const_int(runtime_arity as u64, true);

                    // Function pointer
                    let fn_ptr = function.as_global_value().as_pointer_value();

                    // No captures for letfn functions
                    let null_env = self
                        .context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .ptr_type(AddressSpace::default())
                        .const_null();
                    let zero_env_size = self.context.i32_type().const_zero();

                    // Create function Value*
                    // clorus_function_new(func_ptr, arity, env, env_size)
                    let fn_value = self
                        .builder
                        .build_call(
                            clorus_make_fn,
                            &[
                                fn_ptr.into(),
                                arity_val.into(),
                                null_env.into(),
                                zero_env_size.into(),
                            ],
                            &format!("make_{}", name),
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Bind to variable
                    let alloca = self.create_entry_block_alloca(name);
                    self.builder.build_store(alloca, fn_value).unwrap();
                    self.variables.insert(name.clone(), alloca);
                    local_vars.push((name.clone(), alloca));
                }

                // Step 4: Compile letfn body
                let result = self.compile_expr(body)?;

                // Step 5: Cleanup
                let retain_fn = self
                    .module
                    .get_function("clorus_retain")
                    .ok_or("clorus_retain not declared")?;
                self.builder
                    .build_call(retain_fn, &[result.into()], "retain_result")
                    .unwrap();

                // Release local variables
                let release_fn = self
                    .module
                    .get_function("clorus_release")
                    .ok_or("clorus_release not declared")?;

                for (var_name, var_ptr) in &local_vars {
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let val = self
                        .builder
                        .build_load(value_ptr_type, *var_ptr, &format!("{}_cleanup", var_name))
                        .unwrap()
                        .into_pointer_value();

                    self.builder
                        .build_call(release_fn, &[val.into()], &format!("release_{}", var_name))
                        .unwrap();
                }

                // Restore scope
                self.variables = saved_vars;
                self.current_recur_fn = saved_recur_ctx;

                Ok(result)
            }

            Expr::Def {
                name,
                value,
                metadata,
            } => {
                // Compile the value (returns Value*)
                let val = self.compile_expr(value)?;

                // When metadata is present, back the def with a Var and attach metadata.
                // Symbol resolution auto-derefs Vars, preserving existing value semantics.
                let stored_val = if let Some(meta_entries) = metadata {
                    let str_fn = self
                        .module
                        .get_function("clorus_value_string")
                        .ok_or("clorus_value_string not declared")?;
                    let var_new_fn = self
                        .module
                        .get_function("clorus_var_new")
                        .ok_or("clorus_var_new not declared")?;
                    let value_from_var_fn = self
                        .module
                        .get_function("clorus_value_from_var")
                        .ok_or("clorus_value_from_var not declared")?;
                    let set_meta_fn = self
                        .module
                        .get_function("clorus_var_set_meta")
                        .ok_or("clorus_var_set_meta not declared")?;

                    let name_str = self
                        .builder
                        .build_global_string_ptr(name, "def_var_name")
                        .unwrap();
                    let name_val = self
                        .builder
                        .build_call(
                            str_fn,
                            &[name_str.as_pointer_value().into()],
                            "def_var_name_val",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let var_ptr = self
                        .builder
                        .build_call(var_new_fn, &[name_val.into(), val.into()], "def_var_ptr")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    for (k, v) in meta_entries {
                        let key_name = match k {
                            Expr::Keyword(s) => Some(s.clone()),
                            Expr::Symbol(s) => Some(s.clone()),
                            Expr::String(s) => Some(s.clone()),
                            _ => None,
                        };
                        let Some(key_name) = key_name else { continue };

                        let key_str = self
                            .builder
                            .build_global_string_ptr(&key_name, "def_meta_key")
                            .unwrap();
                        let key_val = self
                            .builder
                            .build_call(
                                str_fn,
                                &[key_str.as_pointer_value().into()],
                                "def_meta_key_val",
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();

                        let meta_val = self.compile_expr(v)?;
                        self.builder
                            .build_call(
                                set_meta_fn,
                                &[var_ptr.into(), key_val.into(), meta_val.into()],
                                "def_set_meta",
                            )
                            .unwrap();
                    }

                    self.builder
                        .build_call(value_from_var_fn, &[var_ptr.into()], "def_var_value")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value()
                } else {
                    val
                };

                // Generate mangled name based on current namespace (like defn)
                // utils.constants/PADDING-SMALL -> clorus_utils_constants_PADDING_SMALL
                let mangled_name = if self.namespace.current == "user" {
                    // In default namespace, use simple name
                    name.clone()
                } else {
                    format!(
                        "clorus_{}_{}",
                        self.namespace.current.replace('.', "_").replace('-', "_"),
                        name.replace('-', "_")
                    )
                };

                // Create or update global variable (now stores Value*)
                let global = if let Some(existing_global) = self.globals.get(&mangled_name) {
                    // If global already exists, just update it
                    *existing_global
                } else {
                    // Create new global variable of type Value* (i8*)
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let global = self.module.add_global(
                        value_ptr_type,
                        Some(AddressSpace::default()),
                        &mangled_name,
                    );
                    // Initialize with null pointer
                    global.set_initializer(&value_ptr_type.const_null());
                    self.globals.insert(mangled_name.clone(), global);
                    global
                };

                // Retain new value for global ownership BEFORE releasing old.
                // This avoids a use-after-free when def re-evaluates to the same pointer
                // (common in REPL where init defs are re-executed each eval).
                let retain_fn = self
                    .module
                    .get_function("clorus_retain")
                    .ok_or("clorus_retain not declared")?;
                self.builder
                    .build_call(retain_fn, &[stored_val.into()], "retain_def_val")
                    .unwrap();

                // Release old value if present
                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let old_val = self
                    .builder
                    .build_load(value_ptr_type, global.as_pointer_value(), "old_def_val")
                    .unwrap()
                    .into_pointer_value();
                let null_ptr = value_ptr_type.const_null();
                let is_null = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        old_val,
                        null_ptr,
                        "old_def_is_null",
                    )
                    .unwrap();
                let release_fn = self
                    .module
                    .get_function("clorus_release")
                    .ok_or("clorus_release not declared")?;
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("No current function for def")?;
                let release_block = self
                    .context
                    .append_basic_block(current_fn, "def_release_old");
                let cont_block = self.context.append_basic_block(current_fn, "def_store_new");
                self.builder
                    .build_conditional_branch(is_null, cont_block, release_block)
                    .unwrap();
                self.builder.position_at_end(release_block);
                self.builder
                    .build_call(release_fn, &[old_val.into()], "release_old_def")
                    .unwrap();
                self.builder.build_unconditional_branch(cont_block).unwrap();
                self.builder.position_at_end(cont_block);

                // Store the Value* to the global
                self.builder
                    .build_store(global.as_pointer_value(), stored_val)
                    .unwrap();

                // Return the raw def value for compatibility with existing behavior.
                Ok(val)
            }

            Expr::Defn {
                name,
                params,
                rest_param,
                body,
                metadata,
            } => self.compile_defn_expr(name, params, rest_param, body, metadata),

            Expr::DefnMulti {
                name,
                arities,
                metadata,
            } => self.compile_defn_multi_expr(name, arities, metadata),

            Expr::Fn {
                params,
                rest_param,
                body,
            } => {
                // Generate unique lambda name
                let lambda_name = format!("_lambda_{}", self.lambda_counter);
                self.lambda_counter += 1;

                // Find free variables (captured from outer scope)
                // Build bound set: function parameters + rest param
                let mut bound = HashSet::new();
                for param in params {
                    for name in Self::collect_pattern_names(param) {
                        bound.insert(name);
                    }
                }
                if let Some(rest_name) = rest_param {
                    bound.insert(rest_name.clone());
                }

                // Find free variables in body
                let mut free_vars: Vec<String> = Vec::new();
                let mut seen = HashSet::new();
                self.collect_free_vars(body, &mut free_vars, &mut seen, &bound);

                // Create function type: all parameters are Value*, plus environment parameter as LAST arg
                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let mut param_types: Vec<_> =
                    params.iter().map(|_| value_ptr_type.into()).collect();

                if rest_param.is_some() {
                    param_types.push(value_ptr_type.into());
                }

                // Add environment parameter as the LAST parameter
                param_types.push(value_ptr_type.into());

                let fn_type = value_ptr_type.fn_type(&param_types, false);
                let function = self.module.add_function(&lambda_name, fn_type, None);

                // Add function to table (for potential recursive calls)
                self.functions.insert(lambda_name.clone(), function);

                // Save current state
                let saved_vars = self.variables.clone();
                let saved_block = self.builder.get_insert_block();

                // Create entry block for the lambda
                let entry = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry);

                // Clear variables for function scope
                self.variables.clear();

                // Bind fixed parameters to allocas (parameters are Value*)
                // Support destructuring in function parameters
                for (i, param_pattern) in params.iter().enumerate() {
                    let param_val = function
                        .get_nth_param(i as u32)
                        .unwrap()
                        .into_pointer_value();

                    // Destructure parameter pattern
                    self.destructure_pattern(param_pattern, param_val)?;
                }

                // Handle rest parameter if present
                if let Some(rest_name) = rest_param {
                    let rest_vec = function
                        .get_nth_param(params.len() as u32)
                        .unwrap()
                        .into_pointer_value();
                    let rest_alloca = self.create_entry_block_alloca(rest_name);
                    self.builder.build_store(rest_alloca, rest_vec).unwrap();
                    self.variables.insert(rest_name.clone(), rest_alloca);
                }

                // Unpack captured variables from environment (LAST parameter)
                if !free_vars.is_empty() {
                    let env_param_index = params.len() + if rest_param.is_some() { 1 } else { 0 };
                    let env_param = function
                        .get_nth_param(env_param_index as u32)
                        .unwrap()
                        .into_pointer_value();

                    for (i, var_name) in free_vars.iter().enumerate() {
                        // Each captured variable is a *mut Value in the environment
                        // Environment is passed as a *mut Value pointing to the first element
                        // We need to offset by i to get to the i-th captured variable

                        let offset = self.context.i64_type().const_int(i as u64, false);
                        let var_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    value_ptr_type,
                                    env_param,
                                    &[offset],
                                    &format!("env_{}", var_name),
                                )
                                .unwrap()
                        };

                        // Load the value from environment
                        let var_value = self
                            .builder
                            .build_load(value_ptr_type, var_ptr, &format!("load_{}", var_name))
                            .unwrap()
                            .into_pointer_value();

                        // Store in local variable
                        let alloca = self.create_entry_block_alloca(var_name);
                        self.builder.build_store(alloca, var_value).unwrap();
                        self.variables.insert(var_name.clone(), alloca);
                    }
                }

                // Compile function body (returns Value*)
                let saved_recur_ctx = self.current_recur_fn.clone();
                self.current_recur_fn = None;
                let result = self.compile_expr(body)?;
                self.current_recur_fn = saved_recur_ctx;
                self.builder.build_return(Some(&result)).unwrap();

                // Restore previous state
                self.variables = saved_vars.clone();
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }

                // Build environment array with captured values
                let env_ptr = if !free_vars.is_empty() {
                    // Allocate array for environment: [*mut Value; free_vars.len()]
                    let env_array_type = value_ptr_type.array_type(free_vars.len() as u32);
                    let env_array = self
                        .builder
                        .build_alloca(env_array_type, "env_array")
                        .unwrap();

                    for (i, var_name) in free_vars.iter().enumerate() {
                        // Get the value from current scope (check saved_vars, parameter_context, then globals)
                        let var_value = if let Some(var_ptr) = saved_vars.get(var_name) {
                            self.builder
                                .build_load(value_ptr_type, *var_ptr, var_name)
                                .unwrap()
                                .into_pointer_value()
                        } else if let Some(param_ptr) = self.parameter_context.get(var_name) {
                            // Check parameter context for captured defn parameters
                            self.builder
                                .build_load(value_ptr_type, *param_ptr, var_name)
                                .unwrap()
                                .into_pointer_value()
                        } else if let Some(global) = self.globals.get(var_name) {
                            self.builder
                                .build_load(value_ptr_type, global.as_pointer_value(), var_name)
                                .unwrap()
                                .into_pointer_value()
                        } else {
                            return Err(format!("Captured variable not found: {}", var_name));
                        };

                        // Store in environment array
                        let elem_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    env_array_type,
                                    env_array,
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_int(i as u64, false),
                                    ],
                                    &format!("env_elem_{}", i),
                                )
                                .unwrap()
                        };
                        self.builder.build_store(elem_ptr, var_value).unwrap();
                    }

                    // Cast to *const *mut Value
                    self.builder
                        .build_pointer_cast(
                            env_array,
                            value_ptr_type.ptr_type(AddressSpace::default()),
                            "env_ptr",
                        )
                        .unwrap()
                } else {
                    // No captures - pass null
                    value_ptr_type
                        .ptr_type(AddressSpace::default())
                        .const_null()
                };

                // Create a proper Function value using clorus_function_new
                // clorus_function_new(func_ptr: *const u8, arity: i32, env: *const *mut Value, env_size: u32) -> *mut Value
                let function_new_fn = self
                    .module
                    .get_function("clorus_function_new")
                    .ok_or("clorus_function_new not declared")?;

                let fn_ptr = function.as_global_value().as_pointer_value();
                let runtime_arity = Self::runtime_arity(params.len(), rest_param.is_some());
                let arity = self
                    .context
                    .i32_type()
                    .const_int(runtime_arity as u64, true);
                let env_size = self
                    .context
                    .i32_type()
                    .const_int(free_vars.len() as u64, false);

                let func_val = self
                    .builder
                    .build_call(
                        function_new_fn,
                        &[fn_ptr.into(), arity.into(), env_ptr.into(), env_size.into()],
                        "new_function",
                    )
                    .unwrap();

                Ok(func_val
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::FnMulti { arities } => {
                // Multi-arity anonymous function
                // Generate one function per arity with unique lambda names

                let base_lambda_name = format!("_lambda_{}", self.lambda_counter);
                self.lambda_counter += 1;

                // Find free variables captured from outer scope (shared across all arities)
                // Build bound set from ALL arities' parameters
                let mut all_bound = HashSet::new();
                for arity in arities {
                    for param in &arity.params {
                        match param {
                            Pattern::Symbol(name) => {
                                all_bound.insert(name.clone());
                            }
                            _ => {}
                        }
                    }
                    if let Some(rest_name) = &arity.rest_param {
                        all_bound.insert(rest_name.clone());
                    }
                }

                // Collect free variables from ALL arities
                let mut free_vars: Vec<String> = Vec::new();
                let mut seen = HashSet::new();
                for arity in arities {
                    self.collect_free_vars(&arity.body, &mut free_vars, &mut seen, &all_bound);
                }

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let mut arity_functions = Vec::new();

                // Generate a function for each arity
                for arity in arities.iter() {
                    let arity_name = Self::arity_variant_name(
                        &base_lambda_name,
                        arity.params.len(),
                        arity.rest_param.is_some(),
                    );

                    // Create parameter types for this arity
                    let mut param_types: Vec<_> =
                        arity.params.iter().map(|_| value_ptr_type.into()).collect();

                    if arity.rest_param.is_some() {
                        param_types.push(value_ptr_type.into());
                    }

                    // Add environment parameter as the LAST parameter (for captured variables)
                    param_types.push(value_ptr_type.into());

                    let fn_type = value_ptr_type.fn_type(&param_types, false);
                    let function = self.module.add_function(&arity_name, fn_type, None);

                    // Save current state
                    let saved_vars = self.variables.clone();
                    let saved_block = self.builder.get_insert_block();

                    // Create entry block
                    let entry = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry);

                    // Clear variables
                    self.variables.clear();

                    // Bind parameters with destructuring support
                    for (i, param_pattern) in arity.params.iter().enumerate() {
                        let param_val = function
                            .get_nth_param(i as u32)
                            .unwrap()
                            .into_pointer_value();

                        self.destructure_pattern(param_pattern, param_val)?;
                    }

                    // Handle rest parameter if present
                    if let Some(rest_name) = &arity.rest_param {
                        let rest_vec = function
                            .get_nth_param(arity.params.len() as u32)
                            .unwrap()
                            .into_pointer_value();
                        let rest_alloca = self.create_entry_block_alloca(rest_name);
                        self.builder.build_store(rest_alloca, rest_vec).unwrap();
                        self.variables.insert(rest_name.clone(), rest_alloca);
                    }

                    // Unpack captured variables from environment (LAST parameter)
                    if !free_vars.is_empty() {
                        let env_param_index =
                            arity.params.len() + if arity.rest_param.is_some() { 1 } else { 0 };
                        let env_param = function
                            .get_nth_param(env_param_index as u32)
                            .unwrap()
                            .into_pointer_value();

                        for (i, var_name) in free_vars.iter().enumerate() {
                            // Each captured variable is a *mut Value in the environment
                            // Environment is passed as a *mut Value pointing to the first element
                            let offset = self.context.i64_type().const_int(i as u64, false);
                            let var_ptr = unsafe {
                                self.builder
                                    .build_gep(
                                        value_ptr_type,
                                        env_param,
                                        &[offset],
                                        &format!("env_{}", var_name),
                                    )
                                    .unwrap()
                            };

                            // Load the captured value
                            let captured_val = self
                                .builder
                                .build_load(value_ptr_type, var_ptr, &format!("load_{}", var_name))
                                .unwrap()
                                .into_pointer_value();

                            // Create alloca and store the captured value
                            let alloca = self.create_entry_block_alloca(var_name);
                            self.builder.build_store(alloca, captured_val).unwrap();
                            self.variables.insert(var_name.clone(), alloca);
                        }
                    }

                    // Compile body
                    let result = self.compile_expr(&arity.body)?;
                    self.builder.build_return(Some(&result)).unwrap();

                    // Restore state
                    self.variables = saved_vars.clone();
                    if let Some(block) = saved_block {
                        self.builder.position_at_end(block);
                    }

                    // Store function with its arity
                    let arity_count =
                        Self::runtime_arity(arity.params.len(), arity.rest_param.is_some());
                    arity_functions.push((arity_count, function));
                }

                // Now create the multi-arity closure value with captured environment
                // Build environment array with captured values (similar to single-arity Fn)
                let env_ptr = if !free_vars.is_empty() {
                    // Allocate array for environment: [*mut Value; free_vars.len()]
                    let env_array_type = value_ptr_type.array_type(free_vars.len() as u32);
                    let env_array = self
                        .builder
                        .build_alloca(env_array_type, "env_array")
                        .unwrap();

                    for (i, var_name) in free_vars.iter().enumerate() {
                        // Get the value from current scope (check variables, parameter_context, then globals)
                        let var_value = if let Some(var_ptr) = self.variables.get(var_name) {
                            self.builder
                                .build_load(value_ptr_type, *var_ptr, var_name)
                                .unwrap()
                                .into_pointer_value()
                        } else if let Some(param_ptr) = self.parameter_context.get(var_name) {
                            // Check parameter context for captured defn parameters
                            self.builder
                                .build_load(value_ptr_type, *param_ptr, var_name)
                                .unwrap()
                                .into_pointer_value()
                        } else if let Some(global) = self.globals.get(var_name) {
                            self.builder
                                .build_load(value_ptr_type, global.as_pointer_value(), var_name)
                                .unwrap()
                                .into_pointer_value()
                        } else {
                            return Err(format!("Captured variable not found: {}", var_name));
                        };

                        // Store in environment array
                        let elem_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    env_array_type,
                                    env_array,
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_int(i as u64, false),
                                    ],
                                    &format!("env_elem_{}", i),
                                )
                                .unwrap()
                        };
                        self.builder.build_store(elem_ptr, var_value).unwrap();
                    }

                    // Cast to *const *mut Value
                    self.builder
                        .build_pointer_cast(
                            env_array,
                            value_ptr_type.ptr_type(AddressSpace::default()),
                            "env_ptr",
                        )
                        .unwrap()
                } else {
                    // No captures - pass null
                    value_ptr_type
                        .ptr_type(AddressSpace::default())
                        .const_null()
                };

                // Create ArityVariant array
                // struct ArityVariant { arity: i32, func_ptr: *const u8 }
                let i32_type = self.context.i32_type();
                let arity_variant_type = self
                    .context
                    .struct_type(&[i32_type.into(), value_ptr_type.into()], false);
                let arity_variants_array_type =
                    arity_variant_type.array_type(arity_functions.len() as u32);
                let arity_variants_array = self
                    .builder
                    .build_alloca(arity_variants_array_type, "arity_variants")
                    .unwrap();

                for (i, (arity_count, function)) in arity_functions.iter().enumerate() {
                    // Create ArityVariant struct: { arity: i32, func_ptr: *const u8 }
                    let variant_ptr = unsafe {
                        self.builder
                            .build_gep(
                                arity_variants_array_type,
                                arity_variants_array,
                                &[i32_type.const_zero(), i32_type.const_int(i as u64, false)],
                                &format!("variant_{}", i),
                            )
                            .unwrap()
                    };

                    // Store arity (field 0)
                    let arity_field_ptr = unsafe {
                        self.builder
                            .build_gep(
                                arity_variant_type,
                                variant_ptr,
                                &[i32_type.const_zero(), i32_type.const_zero()],
                                "arity_field",
                            )
                            .unwrap()
                    };
                    let arity_val = i32_type.const_int(*arity_count as u64, true); // signed
                    self.builder
                        .build_store(arity_field_ptr, arity_val)
                        .unwrap();

                    // Store func_ptr (field 1)
                    let func_ptr_field_ptr = unsafe {
                        self.builder
                            .build_gep(
                                arity_variant_type,
                                variant_ptr,
                                &[i32_type.const_zero(), i32_type.const_int(1, false)],
                                "func_ptr_field",
                            )
                            .unwrap()
                    };
                    let fn_ptr = function.as_global_value().as_pointer_value();
                    self.builder
                        .build_store(func_ptr_field_ptr, fn_ptr)
                        .unwrap();
                }

                // Cast arity variants array to *const ArityVariant (*const u8 for FFI)
                let arities_ptr = self
                    .builder
                    .build_pointer_cast(arity_variants_array, value_ptr_type, "arities_ptr")
                    .unwrap();

                // Call clorus_multi_arity_function_new
                let multi_arity_function_new_fn = self
                    .module
                    .get_function("clorus_multi_arity_function_new")
                    .ok_or("clorus_multi_arity_function_new not declared")?;

                let arity_count = i32_type.const_int(arity_functions.len() as u64, false);
                let env_size = i32_type.const_int(free_vars.len() as u64, false);

                let func_val = self
                    .builder
                    .build_call(
                        multi_arity_function_new_fn,
                        &[
                            arities_ptr.into(),
                            arity_count.into(),
                            env_ptr.into(),
                            env_size.into(),
                        ],
                        "new_multi_arity_function",
                    )
                    .unwrap();

                Ok(func_val
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::Do { exprs } => {
                // Compile all expressions in sequence
                // Return the value of the last expression
                if exprs.is_empty() {
                    return Err("do expression cannot be empty".to_string());
                }

                let mut result = None;
                for expr in exprs {
                    result = Some(self.compile_expr(expr)?);
                }

                Ok(result.unwrap())
            }

            Expr::Dosync { exprs } => {
                // Transaction block: begin, execute, commit with retry
                if exprs.is_empty() {
                    return Err("dosync expression cannot be empty".to_string());
                }

                // Get transaction functions
                let tx_begin_fn = self
                    .module
                    .get_function("clorus_tx_begin")
                    .ok_or("clorus_tx_begin not declared")?;
                let tx_commit_fn = self
                    .module
                    .get_function("clorus_tx_commit")
                    .ok_or("clorus_tx_commit not declared")?;
                let tx_abort_fn = self
                    .module
                    .get_function("clorus_tx_abort")
                    .ok_or("clorus_tx_abort not declared")?;

                // Begin transaction
                self.builder
                    .build_call(tx_begin_fn, &[], "dosync_begin")
                    .unwrap();

                // Compile all expressions in the transaction
                let mut result = None;
                for expr in exprs {
                    result = Some(self.compile_expr(expr)?);
                }
                let body_result = result.unwrap();

                // Attempt to commit
                // TODO: Add retry logic in future version
                // For now, just commit once
                let commit_success = self
                    .builder
                    .build_call(tx_commit_fn, &[], "dosync_commit")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                // Check if commit succeeded
                // If failed, abort transaction
                // TODO: Add retry loop
                let zero = self.context.bool_type().const_zero();
                let commit_failed = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, commit_success, zero, "commit_failed")
                    .unwrap();

                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("Dosync must be inside a function")?;

                let abort_block = self.context.append_basic_block(current_fn, "dosync_abort");
                let continue_block = self
                    .context
                    .append_basic_block(current_fn, "dosync_continue");

                self.builder
                    .build_conditional_branch(commit_failed, abort_block, continue_block)
                    .unwrap();

                // Abort block
                self.builder.position_at_end(abort_block);
                self.builder
                    .build_call(tx_abort_fn, &[], "dosync_abort_call")
                    .unwrap();
                // TODO: In future, add retry logic here
                // For now, just continue after abort
                self.builder
                    .build_unconditional_branch(continue_block)
                    .unwrap();

                // Continue block
                self.builder.position_at_end(continue_block);

                Ok(body_result)
            }

            Expr::Loop { bindings, body } => {
                // Compile loop with tail-call optimization using LLVM basic blocks and phi nodes
                // CRITICAL FIX: Use phi nodes to properly update loop parameters on recur
                //
                // Strategy:
                // 1. Compile initial values in pre-loop block
                // 2. Branch to loop_start
                // 3. At loop_start: Create phi nodes for each binding
                // 4. Store phi values into allocas (so body can read them normally)
                // 5. Compile body reading from allocas
                // 6. On recur: Load from allocas, add to phi nodes, branch back
                // This ensures loop parameters update via phi nodes between iterations

                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("Loop must be inside a function")?;

                // Save current block (pre-loop)
                let pre_loop_block = self.builder.get_insert_block().unwrap();

                // Create basic blocks
                let loop_start = self.context.append_basic_block(current_fn, "loop_start");
                let loop_end = self.context.append_basic_block(current_fn, "loop_end");

                // Save current loop context (for nested loops)
                let saved_loop_context = self.loop_context.clone();

                // Collect binding names from patterns
                let binding_names: Vec<String> = bindings
                    .iter()
                    .flat_map(|(pattern, _)| Self::collect_pattern_names(pattern))
                    .collect();

                // Compile initial values IN PRE-LOOP BLOCK
                let mut init_values = Vec::new();
                for (pattern, init_expr) in bindings {
                    let val = self.compile_expr(init_expr)?;
                    init_values.push(val);
                }

                // Branch from pre-loop to loop_start
                self.builder.build_unconditional_branch(loop_start).unwrap();

                // Position at loop_start
                self.builder.position_at_end(loop_start);

                // CREATE PHI NODES for each binding variable
                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let mut phi_nodes = Vec::new();
                for (i, name) in binding_names.iter().enumerate() {
                    let phi = self
                        .builder
                        .build_phi(value_ptr_type, &format!("loop_phi_{}", name))
                        .unwrap();
                    phi.add_incoming(&[(&init_values[i], pre_loop_block)]);
                    phi_nodes.push(phi);
                }

                // CREATE ALLOCAS and store phi values into them
                // This allows the body to read variables normally via load instructions
                let mut loop_allocas = Vec::new();
                for (i, name) in binding_names.iter().enumerate() {
                    let alloca = self.create_entry_block_alloca(name);
                    let phi_val = phi_nodes[i].as_basic_value().into_pointer_value();
                    self.builder.build_store(alloca, phi_val).unwrap();
                    loop_allocas.push(alloca);
                }

                // Update variables map to point to these allocas
                let saved_variables = self.variables.clone();
                for (i, name) in binding_names.iter().enumerate() {
                    self.variables.insert(name.clone(), loop_allocas[i]);
                }

                // Set new loop context WITH PHI NODES
                self.loop_context = Some(LoopContext {
                    loop_start,
                    loop_end,
                    binding_names: binding_names.clone(),
                    phi_nodes: phi_nodes.clone(),
                });

                // Compile body (will read from allocas)
                let result = self.compile_expr(body)?;

                // Restore variables map
                self.variables = saved_variables;

                // If we reach here (no recur), branch to loop_end with result value
                // Check if the current block already has a terminator (from recur/return/throw)
                let result_block = self.builder.get_insert_block().unwrap();
                let has_terminator = result_block.get_terminator().is_some();
                if !has_terminator {
                    self.builder.build_unconditional_branch(loop_end).unwrap();
                }

                // Position builder at loop_end
                self.builder.position_at_end(loop_end);

                // Restore previous loop context
                self.loop_context = saved_loop_context;

                // Create phi node if the loop_end is reachable
                // If the body always recurses (has terminator), loop_end is unreachable
                if !has_terminator {
                    let phi = self
                        .builder
                        .build_phi(value_ptr_type, "loop_result")
                        .unwrap();
                    phi.add_incoming(&[(&result, result_block)]);
                    Ok(phi.as_basic_value().into_pointer_value())
                } else {
                    // Loop never exits (infinite loop or always returns/throws)
                    // Return a dummy value - this code is unreachable
                    let nil_fn = self
                        .module
                        .get_function("clorus_value_nil")
                        .ok_or("clorus_value_nil not declared")?;
                    let dummy_call = self
                        .builder
                        .build_call(nil_fn, &[], "unreachable_loop_result")
                        .unwrap();
                    Ok(dummy_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value())
                }
            }

            Expr::Recur { args } => {
                // Jump back to the nearest loop with new values
                // CRITICAL FIX: Add incoming values to phi nodes instead of updating allocas
                // The phi nodes will carry updated values to the next iteration
                let Some(loop_ctx) = self.loop_context.clone() else {
                    // Function recur fallback: when inside a named function body and no loop
                    // context is active, compile recur as a self tail call.
                    let recur_ctx = self
                        .current_recur_fn
                        .clone()
                        .ok_or("recur can only be used inside a loop")?;

                    if !recur_ctx.has_rest_param && args.len() != recur_ctx.fixed_param_count {
                        return Err(format!(
                            "recur argument count mismatch: expected {}, got {}",
                            recur_ctx.fixed_param_count,
                            args.len()
                        ));
                    }
                    if recur_ctx.has_rest_param && args.len() < recur_ctx.fixed_param_count {
                        return Err(format!(
                            "recur argument count mismatch: expected at least {}, got {}",
                            recur_ctx.fixed_param_count,
                            args.len()
                        ));
                    }

                    let recur_call = Expr::Call {
                        func: recur_ctx.name,
                        args: args.clone(),
                    };
                    return self.compile_expr(&recur_call);
                };

                // Check that arg count matches binding count
                if args.len() != loop_ctx.binding_names.len() {
                    return Err(format!(
                        "recur argument count mismatch: expected {}, got {}",
                        loop_ctx.binding_names.len(),
                        args.len()
                    ));
                }

                // Evaluate all new values first (before any phi updates)
                let mut new_values = Vec::new();
                for arg in args {
                    new_values.push(self.compile_expr(arg)?);
                }

                // Get current block (recur source block)
                let current_block = self.builder.get_insert_block().unwrap();

                // ADD INCOMING VALUES TO PHI NODES
                // This is the key fix - phi nodes properly merge control flow
                for (i, phi) in loop_ctx.phi_nodes.iter().enumerate() {
                    phi.add_incoming(&[(&new_values[i], current_block)]);
                }

                // Branch back to loop start
                // When execution returns to loop_start, phi nodes will have the new values
                self.builder
                    .build_unconditional_branch(loop_ctx.loop_start)
                    .unwrap();

                // Position builder after the branch (unreachable code, but required)
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("recur must be inside a function")?;
                let unreachable_block = self.context.append_basic_block(current_fn, "after_recur");
                self.builder.position_at_end(unreachable_block);

                // Return nil (never actually reached)
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            Expr::Defmacro { .. } => {
                // Macros are compile-time only and don't generate runtime code
                // They should have been expanded during macro expansion phase
                // If we reach here, just return nil
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            Expr::Defrecord {
                name,
                fields,
                protocols,
            } => {
                // Defrecord combines constructor + optional inline protocol implementations
                // 1. Generate constructor function: ->RecordName
                self.generate_record_constructor(name, fields)?;

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                // 2. Generate protocol method implementations (if any)
                for (protocol_name, methods) in protocols {
                    // Extract protocol simple name (after / if qualified)
                    // E.g., "p/IComponent" -> "IComponent"
                    let protocol_simple_name: &str = if protocol_name.contains('/') {
                        protocol_name.split('/').last().unwrap()
                    } else {
                        protocol_name.as_str()
                    };

                    for method in methods {
                        let func_name = format!("{}_{}_{}", name, protocol_name, method.name);

                        let param_types: Vec<_> = method
                            .params
                            .iter()
                            .map(|_| value_ptr_type.into())
                            .collect();

                        let fn_type = value_ptr_type.fn_type(&param_types, false);
                        let function = self.module.add_function(&func_name, fn_type, None);
                        self.functions.insert(func_name.clone(), function);

                        // Save state
                        let saved_vars = self.variables.clone();
                        let saved_block = self.builder.get_insert_block();

                        // Create entry block
                        let entry = self.context.append_basic_block(function, "entry");
                        self.builder.position_at_end(entry);
                        self.variables.clear();

                        // Bind parameters
                        for (i, param_pattern) in method.params.iter().enumerate() {
                            let param_val = function
                                .get_nth_param(i as u32)
                                .unwrap()
                                .into_pointer_value();
                            self.destructure_pattern(param_pattern, param_val)?;
                        }

                        // Compile method body
                        let result = self.compile_expr(&method.body)?;
                        self.builder.build_return(Some(&result)).unwrap();

                        // Restore state
                        self.variables = saved_vars;
                        if let Some(block) = saved_block {
                            self.builder.position_at_end(block);
                            eprintln!(
                                "[CODEGEN] Restored block for defrecord '{}' method '{}'",
                                name, method.name
                            );
                        } else {
                            eprintln!("[CODEGEN] WARNING: No saved block for defrecord '{}' method '{}' - will use current block", name, method.name);
                        }

                        // Register protocol method in runtime registry
                        let register_fn = self
                            .module
                            .get_function("clorus_register_protocol_method")
                            .ok_or("clorus_register_protocol_method not declared")?;

                        // Create string constants for registration
                        let type_name_str = self
                            .builder
                            .build_global_string_ptr(name, "type_name_str")
                            .unwrap();
                        let proto_name_str = self
                            .builder
                            .build_global_string_ptr(protocol_simple_name, "proto_name_str")
                            .unwrap();
                        let method_name_str = self
                            .builder
                            .build_global_string_ptr(&method.name, "method_name_str")
                            .unwrap();

                        // Get function pointer as i64
                        let fn_ptr = function.as_global_value().as_pointer_value();
                        let fn_ptr_int = self
                            .builder
                            .build_ptr_to_int(fn_ptr, self.context.i64_type(), "fn_ptr_int")
                            .unwrap();

                        // Call registration function
                        self.builder
                            .build_call(
                                register_fn,
                                &[
                                    type_name_str.as_pointer_value().into(),
                                    proto_name_str.as_pointer_value().into(),
                                    method_name_str.as_pointer_value().into(),
                                    fn_ptr_int.into(),
                                ],
                                "register_protocol",
                            )
                            .unwrap();
                    }
                }

                // defrecord returns nil (like defn)
                let nil_fn = self
                    .module
                    .get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self
                    .builder
                    .build_call(nil_fn, &[], "defrecord_nil")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(nil_val)
            }

            Expr::Defprotocol { name, methods } => {
                // PHASE 5.1: Track protocol methods for automatic dispatch
                // Store each method with its protocol name and parameter count
                for method in methods {
                    let param_count = method.params.len();
                    self.protocol_methods
                        .insert(method.name.clone(), (name.clone(), param_count));
                }

                // Generate dispatch functions for each protocol method
                // These allow cross-module calls like coral-ui.core.protocols/measure
                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                for method in methods {
                    // Generate function name using current namespace (same logic as defn)
                    let func_name = if self.namespace.current == "user" {
                        format!("clorus_{}", method.name.replace('-', "_"))
                    } else {
                        format!(
                            "clorus_{}_{}",
                            self.namespace.current.replace('.', "_").replace('-', "_"),
                            method.name.replace('-', "_")
                        )
                    };

                    // Skip if already defined (from forward declaration or previous protocol)
                    if self.functions.contains_key(&func_name) {
                        continue;
                    }

                    // Create parameter types (all Value*, plus env)
                    let param_count = method.params.len();
                    let mut param_types: Vec<_> =
                        (0..param_count).map(|_| value_ptr_type.into()).collect();
                    param_types.push(value_ptr_type.into()); // env parameter

                    let fn_type = value_ptr_type.fn_type(&param_types, false);
                    let function = self.module.add_function(&func_name, fn_type, None);
                    self.functions.insert(func_name.clone(), function);

                    // Save state
                    let saved_vars = self.variables.clone();
                    let saved_block = self.builder.get_insert_block();

                    // Create entry block and generate dispatch logic
                    let entry = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry);
                    self.variables.clear();

                    // Generate the dispatch logic (same as in compile_expr for protocol calls)
                    // Get the first parameter (the instance)
                    let instance = function.get_nth_param(0).unwrap().into_pointer_value();

                    // Extract __type__ from first argument
                    let map_get_fn = self
                        .module
                        .get_function("clorus_map_get")
                        .ok_or("clorus_map_get not declared")?;

                    let keyword_fn = self
                        .module
                        .get_function("clorus_keyword")
                        .ok_or("clorus_keyword not declared")?;
                    let type_keyword_str = self
                        .builder
                        .build_global_string_ptr("__type__", "proto_dispatch_type_keyword")
                        .unwrap();
                    let type_keyword = self
                        .builder
                        .build_call(
                            keyword_fn,
                            &[type_keyword_str.as_pointer_value().into()],
                            "proto_dispatch_type_kw",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let type_name_val = self
                        .builder
                        .build_call(
                            map_get_fn,
                            &[instance.into(), type_keyword.into()],
                            "proto_dispatch_type_name",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Get string data from Value*
                    let str_data_fn = self
                        .module
                        .get_function("clorus_string_data")
                        .ok_or("clorus_string_data not declared")?;
                    let type_name_cstr = self
                        .builder
                        .build_call(
                            str_data_fn,
                            &[type_name_val.into()],
                            "proto_dispatch_type_cstr",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Lookup protocol method
                    let lookup_fn = self
                        .module
                        .get_function("clorus_lookup_protocol_method")
                        .ok_or("clorus_lookup_protocol_method not declared")?;

                    let protocol_name_str = self
                        .builder
                        .build_global_string_ptr(name, "proto_dispatch_proto")
                        .unwrap();
                    let method_name_str = self
                        .builder
                        .build_global_string_ptr(&method.name, "proto_dispatch_method")
                        .unwrap();

                    let fn_ptr_int = self
                        .builder
                        .build_call(
                            lookup_fn,
                            &[
                                type_name_cstr.into(),
                                protocol_name_str.as_pointer_value().into(),
                                method_name_str.as_pointer_value().into(),
                            ],
                            "proto_dispatch_fn_ptr",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_int_value();

                    // Check if method was found
                    let zero = self.context.i64_type().const_zero();
                    let found = self
                        .builder
                        .build_int_compare(
                            IntPredicate::NE,
                            fn_ptr_int,
                            zero,
                            "proto_dispatch_found",
                        )
                        .unwrap();

                    let found_block = self
                        .context
                        .append_basic_block(function, "proto_dispatch_found");
                    let not_found_block = self
                        .context
                        .append_basic_block(function, "proto_dispatch_not_found");
                    let continue_block = self
                        .context
                        .append_basic_block(function, "proto_dispatch_continue");

                    self.builder
                        .build_conditional_branch(found, found_block, not_found_block)
                        .unwrap();

                    // Found block: cast and call
                    self.builder.position_at_end(found_block);

                    let fn_ptr = self
                        .builder
                        .build_int_to_ptr(fn_ptr_int, value_ptr_type, "proto_dispatch_fn")
                        .unwrap();

                    let method_param_types: Vec<_> =
                        (0..param_count).map(|_| value_ptr_type.into()).collect();
                    let method_fn_type = value_ptr_type.fn_type(&method_param_types, false);

                    let fn_ptr_typed = self
                        .builder
                        .build_pointer_cast(
                            fn_ptr,
                            method_fn_type.ptr_type(AddressSpace::default()),
                            "proto_dispatch_fn_typed",
                        )
                        .unwrap();

                    // Collect arguments (exclude env parameter)
                    let arg_metadata: Vec<_> = (0..param_count)
                        .map(|i| function.get_nth_param(i as u32).unwrap().into())
                        .collect();

                    let result = self
                        .builder
                        .build_indirect_call(
                            method_fn_type,
                            fn_ptr_typed,
                            &arg_metadata,
                            "proto_dispatch_result",
                        )
                        .unwrap();

                    let result_val = result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                    self.builder
                        .build_unconditional_branch(continue_block)
                        .unwrap();

                    // Not found block: error
                    self.builder.position_at_end(not_found_block);
                    let error_msg = format!(
                        "No implementation of protocol method {}.{} found for type",
                        name, method.name
                    );
                    let error_str = self
                        .builder
                        .build_global_string_ptr(&error_msg, "proto_dispatch_error")
                        .unwrap();

                    let str_fn = self
                        .module
                        .get_function("clorus_value_string")
                        .ok_or("clorus_value_string not declared")?;
                    let error_val = self
                        .builder
                        .build_call(str_fn, &[error_str.as_pointer_value().into()], "error_val")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let println_fn = self
                        .module
                        .get_function("clorus_println")
                        .ok_or("clorus_println not declared")?;
                    self.builder
                        .build_call(println_fn, &[error_val.into()], "print_error")
                        .unwrap();

                    let nil_fn = self
                        .module
                        .get_function("clorus_value_nil")
                        .ok_or("clorus_value_nil not declared")?;
                    let error_result = self
                        .builder
                        .build_call(nil_fn, &[], "error_nil")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    self.builder
                        .build_unconditional_branch(continue_block)
                        .unwrap();

                    // Continue block: merge results
                    self.builder.position_at_end(continue_block);
                    let phi = self
                        .builder
                        .build_phi(value_ptr_type, "proto_dispatch_phi")
                        .unwrap();
                    phi.add_incoming(&[
                        (&result_val, found_block),
                        (&error_result, not_found_block),
                    ]);

                    self.builder
                        .build_return(Some(&phi.as_basic_value()))
                        .unwrap();

                    // Restore state
                    self.variables = saved_vars;
                    if let Some(block) = saved_block {
                        self.builder.position_at_end(block);
                    }
                }

                // Return nil for defprotocol form
                let nil_fn = self
                    .module
                    .get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self
                    .builder
                    .build_call(nil_fn, &[], "defprotocol_nil")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(nil_val)
            }

            Expr::Deftype {
                name,
                fields,
                protocols,
            } => {
                // Deftype combines defrecord + inline protocol implementations
                // 1. Generate constructor using shared helper
                self.generate_record_constructor(name, fields)?;

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                // 2. Generate protocol method implementations
                for (protocol_name, methods) in protocols {
                    // Extract protocol simple name (after / if qualified)
                    let protocol_simple_name: &str = if protocol_name.contains('/') {
                        protocol_name.split('/').last().unwrap()
                    } else {
                        protocol_name.as_str()
                    };

                    for method in methods {
                        let func_name = format!("{}_{}_{}", name, protocol_name, method.name);

                        let param_types: Vec<_> = method
                            .params
                            .iter()
                            .map(|_| value_ptr_type.into())
                            .collect();

                        let fn_type = value_ptr_type.fn_type(&param_types, false);
                        let function = self.module.add_function(&func_name, fn_type, None);
                        self.functions.insert(func_name.clone(), function);

                        // Save state
                        let saved_vars = self.variables.clone();
                        let saved_block = self.builder.get_insert_block();

                        // Create entry block
                        let entry = self.context.append_basic_block(function, "entry");
                        self.builder.position_at_end(entry);
                        self.variables.clear();

                        // Bind parameters
                        for (i, param_pattern) in method.params.iter().enumerate() {
                            let param_val = function
                                .get_nth_param(i as u32)
                                .unwrap()
                                .into_pointer_value();
                            self.destructure_pattern(param_pattern, param_val)?;
                        }

                        // Compile method body
                        let result = self.compile_expr(&method.body)?;
                        self.builder.build_return(Some(&result)).unwrap();

                        // Restore state
                        self.variables = saved_vars;
                        if let Some(block) = saved_block {
                            self.builder.position_at_end(block);
                        }

                        // PHASE 4.3: Register protocol method in runtime registry
                        // This enables automatic protocol dispatch at runtime
                        let register_fn = self
                            .module
                            .get_function("clorus_register_protocol_method")
                            .ok_or("clorus_register_protocol_method not declared")?;

                        // Create string constants for registration
                        let type_name_str = self
                            .builder
                            .build_global_string_ptr(name, "type_name_str")
                            .unwrap();
                        let proto_name_str = self
                            .builder
                            .build_global_string_ptr(protocol_simple_name, "proto_name_str")
                            .unwrap();
                        let method_name_str = self
                            .builder
                            .build_global_string_ptr(&method.name, "method_name_str")
                            .unwrap();

                        // Get function pointer as i64
                        let fn_ptr = function.as_global_value().as_pointer_value();
                        let fn_ptr_int = self
                            .builder
                            .build_ptr_to_int(fn_ptr, self.context.i64_type(), "fn_ptr_int")
                            .unwrap();

                        // Call registration function
                        self.builder
                            .build_call(
                                register_fn,
                                &[
                                    type_name_str.as_pointer_value().into(),
                                    proto_name_str.as_pointer_value().into(),
                                    method_name_str.as_pointer_value().into(),
                                    fn_ptr_int.into(),
                                ],
                                "register_protocol",
                            )
                            .unwrap();
                    }
                }

                // Return nil for the deftype form itself
                let nil_fn = self
                    .module
                    .get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self
                    .builder
                    .build_call(nil_fn, &[], "deftype_nil")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(nil_val)
            }

            Expr::ExtendType {
                type_name,
                protocol_name,
                methods,
            } => {
                // Generate functions for each protocol method implementation
                // Function names are mangled: TypeName_ProtocolName_methodName
                // Example: Point_Drawable_draw

                // Extract protocol simple name (after / if qualified)
                let protocol_simple_name: &str = if protocol_name.contains('/') {
                    protocol_name.split('/').last().unwrap()
                } else {
                    protocol_name.as_str()
                };

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                for method in methods {
                    // Generate mangled function name
                    let func_name = format!("{}_{}_{}", type_name, protocol_name, method.name);

                    // Create parameter types (all Value*)
                    let param_types: Vec<_> = method
                        .params
                        .iter()
                        .map(|_| value_ptr_type.into())
                        .collect();

                    let fn_type = value_ptr_type.fn_type(&param_types, false);
                    let function = self.module.add_function(&func_name, fn_type, None);

                    // Add to function table
                    self.functions.insert(func_name.clone(), function);

                    // Save current state
                    let saved_vars = self.variables.clone();
                    let saved_block = self.builder.get_insert_block();

                    // Create entry block
                    let entry = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry);

                    // Clear variables for function scope
                    self.variables.clear();

                    // Bind parameters with destructuring support
                    for (i, param_pattern) in method.params.iter().enumerate() {
                        let param_val = function
                            .get_nth_param(i as u32)
                            .unwrap()
                            .into_pointer_value();

                        self.destructure_pattern(param_pattern, param_val)?;
                    }

                    // Compile method body
                    let result = self.compile_expr(&method.body)?;
                    self.builder.build_return(Some(&result)).unwrap();

                    // Restore previous state
                    self.variables = saved_vars.clone();
                    if let Some(block) = saved_block {
                        self.builder.position_at_end(block);
                    }

                    // PHASE 4.3: Register protocol method in runtime registry
                    let register_fn = self
                        .module
                        .get_function("clorus_register_protocol_method")
                        .ok_or("clorus_register_protocol_method not declared")?;

                    // Create string constants
                    let type_name_str = self
                        .builder
                        .build_global_string_ptr(type_name, "type_name_str")
                        .unwrap();
                    let proto_name_str = self
                        .builder
                        .build_global_string_ptr(protocol_simple_name, "proto_name_str")
                        .unwrap();
                    let method_name_str = self
                        .builder
                        .build_global_string_ptr(&method.name, "method_name_str")
                        .unwrap();

                    // Get function pointer as i64
                    let fn_ptr = function.as_global_value().as_pointer_value();
                    let fn_ptr_int = self
                        .builder
                        .build_ptr_to_int(fn_ptr, self.context.i64_type(), "fn_ptr_int")
                        .unwrap();

                    // Call registration function
                    self.builder
                        .build_call(
                            register_fn,
                            &[
                                type_name_str.as_pointer_value().into(),
                                proto_name_str.as_pointer_value().into(),
                                method_name_str.as_pointer_value().into(),
                                fn_ptr_int.into(),
                            ],
                            "register_protocol",
                        )
                        .unwrap();
                }

                // extend-type returns nil
                let nil_fn = self
                    .module
                    .get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self
                    .builder
                    .build_call(nil_fn, &[], "extend_type_nil")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(nil_val)
            }

            Expr::Defmulti {
                name,
                dispatch_fn,
                metadata,
            } => {
                // Compile dispatch function and expose it as a var-backed global with metadata.
                let dispatch_fn_name = format!("{}_dispatch", name);
                let mangled_name = if self.namespace.current == "user" {
                    name.clone()
                } else {
                    format!(
                        "clorus_{}_{}",
                        self.namespace.current.replace('.', "_").replace('-', "_"),
                        name.replace('-', "_")
                    )
                };

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                // Dispatch function takes argument value + environment.
                let param_types = vec![value_ptr_type.into(), value_ptr_type.into()];
                let fn_type = value_ptr_type.fn_type(&param_types, false);
                let function = self.module.add_function(&dispatch_fn_name, fn_type, None);

                // Add to function table under both internal dispatch name and callable symbol name.
                self.functions.insert(dispatch_fn_name.clone(), function);
                self.functions.insert(mangled_name.clone(), function);
                self.function_signatures
                    .insert(mangled_name.clone(), (1, false));
                self.multimethod_dispatch
                    .insert(name.clone(), *dispatch_fn.clone());
                // Redefining defmulti resets its method/preference table in this compile unit.
                self.multimethod_methods.remove(name);
                self.multimethod_preferences.remove(name);

                // Save current state
                let saved_vars = self.variables.clone();
                let saved_block = self.builder.get_insert_block();

                // Create entry block
                let entry = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry);

                // Clear variables for function scope
                self.variables.clear();

                // Bind parameter as 'arg'
                let param_val = function.get_nth_param(0).unwrap().into_pointer_value();
                let arg_alloca = self.create_entry_block_alloca("arg");
                self.builder.build_store(arg_alloca, param_val).unwrap();
                self.variables.insert("arg".to_string(), arg_alloca);

                // Compile dispatch function body
                let result = self.compile_expr(dispatch_fn)?;
                self.builder.build_return(Some(&result)).unwrap();

                // Restore previous state
                self.variables = saved_vars;
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }

                // Create and store var-backed function value so #'name carries metadata.
                let function_ptr = function.as_global_value().as_pointer_value();
                let function_ptr_as_i8 = self
                    .builder
                    .build_pointer_cast(function_ptr, value_ptr_type, "defmulti_func_ptr_cast")
                    .unwrap();
                let function_new_fn = self
                    .module
                    .get_function("clorus_function_new")
                    .ok_or("clorus_function_new not declared")?;
                let arity_val = self.context.i32_type().const_int(1, false);
                let null_env = value_ptr_type
                    .ptr_type(AddressSpace::default())
                    .const_null();
                let env_size = self.context.i32_type().const_zero();
                let fn_value = self
                    .builder
                    .build_call(
                        function_new_fn,
                        &[
                            function_ptr_as_i8.into(),
                            arity_val.into(),
                            null_env.into(),
                            env_size.into(),
                        ],
                        "defmulti_value",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let str_fn = self
                    .module
                    .get_function("clorus_value_string")
                    .ok_or("clorus_value_string not declared")?;
                let var_new_fn = self
                    .module
                    .get_function("clorus_var_new")
                    .ok_or("clorus_var_new not declared")?;
                let value_from_var_fn = self
                    .module
                    .get_function("clorus_value_from_var")
                    .ok_or("clorus_value_from_var not declared")?;
                let set_meta_fn = self
                    .module
                    .get_function("clorus_var_set_meta")
                    .ok_or("clorus_var_set_meta not declared")?;
                let name_str = self
                    .builder
                    .build_global_string_ptr(name, "defmulti_var_name")
                    .unwrap();
                let name_val = self
                    .builder
                    .build_call(
                        str_fn,
                        &[name_str.as_pointer_value().into()],
                        "defmulti_var_name_val",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                let var_ptr = self
                    .builder
                    .build_call(
                        var_new_fn,
                        &[name_val.into(), fn_value.into()],
                        "defmulti_var_ptr",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                if let Some(meta_entries) = metadata {
                    for (k, v) in meta_entries {
                        let key_name = match k {
                            Expr::Keyword(s) => Some(s.clone()),
                            Expr::Symbol(s) => Some(s.clone()),
                            Expr::String(s) => Some(s.clone()),
                            _ => None,
                        };
                        let Some(key_name) = key_name else { continue };

                        let key_str = self
                            .builder
                            .build_global_string_ptr(&key_name, "defmulti_meta_key")
                            .unwrap();
                        let key_val = self
                            .builder
                            .build_call(
                                str_fn,
                                &[key_str.as_pointer_value().into()],
                                "defmulti_meta_key_val",
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        let meta_val = self.compile_expr(v)?;
                        self.builder
                            .build_call(
                                set_meta_fn,
                                &[var_ptr.into(), key_val.into(), meta_val.into()],
                                "defmulti_set_meta",
                            )
                            .unwrap();
                    }
                }

                let stored_val = self
                    .builder
                    .build_call(value_from_var_fn, &[var_ptr.into()], "defmulti_var_value")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let global = if let Some(existing_global) = self.globals.get(&mangled_name) {
                    *existing_global
                } else {
                    let global = self.module.add_global(
                        value_ptr_type,
                        Some(AddressSpace::default()),
                        &mangled_name,
                    );
                    global.set_initializer(&value_ptr_type.const_null());
                    self.globals.insert(mangled_name.clone(), global);
                    global
                };
                let retain_fn = self
                    .module
                    .get_function("clorus_retain")
                    .ok_or("clorus_retain not declared")?;
                self.builder
                    .build_call(retain_fn, &[stored_val.into()], "retain_defmulti_val")
                    .unwrap();
                let old_val = self
                    .builder
                    .build_load(
                        value_ptr_type,
                        global.as_pointer_value(),
                        "old_defmulti_val",
                    )
                    .unwrap()
                    .into_pointer_value();
                let null_ptr = value_ptr_type.const_null();
                let is_null = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        old_val,
                        null_ptr,
                        "old_defmulti_is_null",
                    )
                    .unwrap();
                let release_fn = self
                    .module
                    .get_function("clorus_release")
                    .ok_or("clorus_release not declared")?;
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("No current function for defmulti")?;
                let release_block = self
                    .context
                    .append_basic_block(current_fn, "defmulti_release_old");
                let cont_block = self
                    .context
                    .append_basic_block(current_fn, "defmulti_store_new");
                self.builder
                    .build_conditional_branch(is_null, cont_block, release_block)
                    .unwrap();
                self.builder.position_at_end(release_block);
                self.builder
                    .build_call(release_fn, &[old_val.into()], "release_old_defmulti")
                    .unwrap();
                self.builder.build_unconditional_branch(cont_block).unwrap();
                self.builder.position_at_end(cont_block);
                self.builder
                    .build_store(global.as_pointer_value(), stored_val)
                    .unwrap();

                // defmulti returns nil
                let nil_fn = self
                    .module
                    .get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self
                    .builder
                    .build_call(nil_fn, &[], "defmulti_nil")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(nil_val)
            }

            Expr::Defmethod {
                name,
                dispatch_value,
                params,
                body,
            } => {
                // Generate function name based on multimethod name and dispatch value
                // Convert dispatch value to string for function name
                let dispatch_str = match dispatch_value.as_ref() {
                    Expr::Keyword(k) => k.clone(),
                    Expr::Symbol(s) => s.clone(),
                    Expr::Long(n) => format!("{}", *n),
                    Expr::Double(n) => format!("{}", *n as i64),
                    Expr::String(s) => s.replace("-", "_").replace(" ", "_"),
                    _ => {
                        return Err(
                            "Dispatch value must be a keyword, symbol, number, or string"
                                .to_string(),
                        )
                    }
                };

                let func_name = if self.namespace.current == "user" {
                    format!("{}_{}", name, dispatch_str)
                } else {
                    format!(
                        "clorus_{}_{}_{}",
                        self.namespace.current.replace('.', "_").replace('-', "_"),
                        name.replace('-', "_"),
                        dispatch_str
                    )
                };

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                // Create parameter types (all Value*)
                let param_types: Vec<_> = params.iter().map(|_| value_ptr_type.into()).collect();

                let fn_type = value_ptr_type.fn_type(&param_types, false);
                let function = self.module.add_function(&func_name, fn_type, None);

                // Add to function table
                self.functions.insert(func_name.clone(), function);
                self.multimethod_methods.entry(name.clone()).or_default();
                let methods = self
                    .multimethod_methods
                    .get_mut(name)
                    .expect("multimethod method table exists");
                let target_dispatch = *dispatch_value.clone();
                // Clojure semantics: redefining the same dispatch value replaces prior implementation.
                methods
                    .retain(|m| !(m.arity == params.len() && m.dispatch_value == target_dispatch));
                methods.push(MultimethodMethod {
                    dispatch_value: target_dispatch,
                    function,
                    arity: params.len(),
                });

                // Save current state
                let saved_vars = self.variables.clone();
                let saved_block = self.builder.get_insert_block();

                // Create entry block
                let entry = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry);

                // Clear variables for function scope
                self.variables.clear();

                // Bind parameters with destructuring support
                for (i, param_pattern) in params.iter().enumerate() {
                    let param_val = function
                        .get_nth_param(i as u32)
                        .unwrap()
                        .into_pointer_value();

                    self.destructure_pattern(param_pattern, param_val)?;
                }

                // Compile method body
                let result = self.compile_expr(body)?;
                self.builder.build_return(Some(&result)).unwrap();

                // Restore previous state
                self.variables = saved_vars;
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }

                // defmethod returns nil
                let nil_fn = self
                    .module
                    .get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self
                    .builder
                    .build_call(nil_fn, &[], "defmethod_nil")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(nil_val)
            }

            Expr::PreferMethod {
                name,
                preferred_dispatch,
                over_dispatch,
            } => {
                if !self.multimethod_dispatch.contains_key(name) {
                    return Err(format!(
                        "prefer-method references undefined multimethod '{}'",
                        name
                    ));
                }
                let entry = self
                    .multimethod_preferences
                    .entry(name.clone())
                    .or_default();
                let preferred = *preferred_dispatch.clone();
                let over = *over_dispatch.clone();
                if !entry.iter().any(|(p, o)| *p == preferred && *o == over) {
                    entry.push((preferred, over));
                }

                let nil_fn = self
                    .module
                    .get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self
                    .builder
                    .build_call(nil_fn, &[], "prefer_method_nil")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(nil_val)
            }

            Expr::RemoveMethod {
                name,
                dispatch_value,
            } => {
                if !self.multimethod_dispatch.contains_key(name) {
                    return Err(format!(
                        "remove-method references undefined multimethod '{}'",
                        name
                    ));
                }
                let target = *dispatch_value.clone();
                if let Some(methods) = self.multimethod_methods.get_mut(name) {
                    methods.retain(|m| m.dispatch_value != target);
                }
                if let Some(prefs) = self.multimethod_preferences.get_mut(name) {
                    prefs.retain(|(preferred, over)| preferred != &target && over != &target);
                }

                let nil_fn = self
                    .module
                    .get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self
                    .builder
                    .build_call(nil_fn, &[], "remove_method_nil")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(nil_val)
            }

            Expr::Quote { expr } => {
                // Quote prevents evaluation - return the expression as data
                self.compile_quoted(expr)
            }

            Expr::SyntaxQuote { expr } => {
                // Syntax-quote: like quote but processes unquotes
                self.compile_syntax_quoted(expr)
            }

            Expr::Unquote { .. } => {
                // Unquote outside syntax-quote is an error
                Err("Unquote (~) can only be used inside syntax-quote (`)".to_string())
            }

            Expr::UnquoteSplicing { .. } => {
                // Unquote-splicing outside syntax-quote is an error
                Err("Unquote-splicing (~@) can only be used inside syntax-quote (`)".to_string())
            }

            Expr::Try {
                body,
                catch_clauses,
                finally_block,
            } => {
                // Language-level exceptions are regular Value* wrappers (ValueTag::Exception).
                // try/catch inspects the body result and dispatches to catch handlers.

                let saved_vars = self.variables.clone();
                let body_result = self.compile_expr(body)?;

                let mut result_value = body_result;

                if !catch_clauses.is_empty() {
                    let is_exception_fn = self
                        .module
                        .get_function("clorus_is_exception_i32")
                        .ok_or("clorus_is_exception_i32 not declared")?;
                    let is_exception = self
                        .builder
                        .build_call(is_exception_fn, &[body_result.into()], "try_is_exception")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_int_value();

                    let is_exception_bool = self
                        .builder
                        .build_int_compare(
                            inkwell::IntPredicate::NE,
                            is_exception,
                            self.context.i32_type().const_zero(),
                            "try_is_exception_bool",
                        )
                        .unwrap();

                    let function = self
                        .builder
                        .get_insert_block()
                        .and_then(|block| block.get_parent())
                        .expect("No parent function");

                    let catch_dispatch_bb = self
                        .context
                        .append_basic_block(function, "try_catch_dispatch");
                    let no_catch_bb = self.context.append_basic_block(function, "try_no_catch");
                    let merge_bb = self.context.append_basic_block(function, "try_merge");

                    self.builder
                        .build_conditional_branch(is_exception_bool, catch_dispatch_bb, no_catch_bb)
                        .unwrap();

                    self.builder.position_at_end(no_catch_bb);
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                    let no_catch_bb_end = self.builder.get_insert_block().unwrap();

                    self.builder.position_at_end(catch_dispatch_bb);
                    let payload_fn = self
                        .module
                        .get_function("clorus_exception_payload")
                        .ok_or("clorus_exception_payload not declared")?;
                    let payload = self
                        .builder
                        .build_call(payload_fn, &[body_result.into()], "catch_payload")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let mut incoming: Vec<(
                        PointerValue<'ctx>,
                        inkwell::basic_block::BasicBlock<'ctx>,
                    )> = vec![(body_result, no_catch_bb_end)];
                    let mut current_check_bb = catch_dispatch_bb;

                    for (i, catch_clause) in catch_clauses.iter().enumerate() {
                        self.builder.position_at_end(current_check_bb);

                        let type_match = if let Some(exception_type) = &catch_clause.exception_type
                        {
                            let simple = exception_type
                                .rsplit(['.', '/'])
                                .next()
                                .unwrap_or(exception_type)
                                .to_ascii_lowercase();

                            let predicate_name = match simple.as_str() {
                                "string" => Some("clorus_is_string_i32"),
                                "number" | "long" | "double" | "int" | "float" => {
                                    Some("clorus_is_number_i32")
                                }
                                "vector" => Some("clorus_is_vector_i32"),
                                "list" => Some("clorus_is_list_i32"),
                                "map" | "hashmap" => Some("clorus_is_map_i32"),
                                "set" | "hashset" => Some("clorus_is_set_i32"),
                                "keyword" => Some("clorus_is_keyword_i32"),
                                "symbol" => Some("clorus_is_symbol_i32"),
                                "nil" => Some("clorus_is_nil_i32"),
                                "bool" | "boolean" => Some("clorus_is_bool_i32"),
                                "seq" => Some("clorus_is_seq_i32"),
                                "coll" | "collection" => Some("clorus_is_coll_i32"),
                                "fn" | "function" => Some("clorus_is_fn_i32"),
                                // In current language-level model, Exception/Error/Throwable are catch-all.
                                "exception" | "error" | "throwable" | "object" | "any" => None,
                                _ => Some("__clorus_unknown_catch_type__"),
                            };

                            match predicate_name {
                                None => self.context.bool_type().const_int(1, false),
                                Some("__clorus_unknown_catch_type__") => {
                                    self.context.bool_type().const_zero()
                                }
                                Some(pred) => {
                                    let pred_fn = self
                                        .module
                                        .get_function(pred)
                                        .ok_or(format!("{} not declared", pred))?;
                                    let pred_i32 = self
                                        .builder
                                        .build_call(
                                            pred_fn,
                                            &[payload.into()],
                                            &format!("catch_match_{}_i32", i),
                                        )
                                        .unwrap()
                                        .try_as_basic_value()
                                        .left()
                                        .unwrap()
                                        .into_int_value();
                                    self.builder
                                        .build_int_compare(
                                            inkwell::IntPredicate::NE,
                                            pred_i32,
                                            self.context.i32_type().const_zero(),
                                            &format!("catch_match_{}", i),
                                        )
                                        .unwrap()
                                }
                            }
                        } else {
                            self.context.bool_type().const_int(1, false)
                        };

                        let handler_bb = self
                            .context
                            .append_basic_block(function, &format!("try_handler_{}", i));
                        let next_check_bb = if i == catch_clauses.len() - 1 {
                            self.context.append_basic_block(function, "try_no_match")
                        } else {
                            self.context
                                .append_basic_block(function, &format!("try_check_next_{}", i + 1))
                        };

                        self.builder
                            .build_conditional_branch(type_match, handler_bb, next_check_bb)
                            .unwrap();

                        self.builder.position_at_end(handler_bb);
                        let catch_saved_vars = self.variables.clone();
                        let catch_alloca = self.create_entry_block_alloca(&catch_clause.binding);
                        self.builder.build_store(catch_alloca, payload).unwrap();
                        self.variables
                            .insert(catch_clause.binding.clone(), catch_alloca);

                        let handler_value = self.compile_expr(&catch_clause.handler)?;
                        self.variables = catch_saved_vars;

                        let handler_end = self.builder.get_insert_block().unwrap();
                        if handler_end.get_terminator().is_none() {
                            self.builder.build_unconditional_branch(merge_bb).unwrap();
                            let handler_after_branch = self.builder.get_insert_block().unwrap();
                            incoming.push((handler_value, handler_after_branch));
                        }

                        current_check_bb = next_check_bb;
                    }

                    // No catch clause matched: propagate original exception wrapper.
                    self.builder.position_at_end(current_check_bb);
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                    let no_match_end = self.builder.get_insert_block().unwrap();
                    incoming.push((body_result, no_match_end));

                    self.builder.position_at_end(merge_bb);
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let phi = self
                        .builder
                        .build_phi(value_ptr_type, "try_result")
                        .unwrap();
                    for (val, bb) in incoming {
                        phi.add_incoming(&[(&val, bb)]);
                    }
                    result_value = phi.as_basic_value().into_pointer_value();
                }

                if let Some(finally_expr) = finally_block {
                    self.compile_expr(finally_expr)?;
                }

                self.variables = saved_vars;
                Ok(result_value)
            }

            Expr::Throw { expr } => {
                // throw returns a language-level exception wrapper that propagates through expressions.
                let payload = self.compile_expr(expr)?;
                let wrap_fn = self
                    .module
                    .get_function("clorus_value_exception")
                    .ok_or("clorus_value_exception not declared")?;
                let wrapped = self
                    .builder
                    .build_call(wrap_fn, &[payload.into()], "throw_exception_value")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(wrapped)
            }

            Expr::Deref { expr } => {
                // Deref: @my-atom => (deref my-atom)
                let atom_val = self.compile_expr(expr)?;

                let deref_fn = self
                    .module
                    .get_function("clorus_deref")
                    .ok_or("clorus_deref not declared")?;

                let result = self
                    .builder
                    .build_call(deref_fn, &[atom_val.into()], "deref_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::Call { func, args } => {
                if func == "methods" {
                    if args.len() != 1 {
                        return Err(
                            "methods expects exactly one argument: (methods multimethod-symbol)"
                                .to_string(),
                        );
                    }
                    let Some(mm_name) = Self::multimethod_name_from_symbol_arg(&args[0]) else {
                        return Err("methods requires a multimethod symbol argument".to_string());
                    };
                    if !self.multimethod_dispatch.contains_key(&mm_name) {
                        return Err(format!(
                            "methods references undefined multimethod '{}'",
                            mm_name
                        ));
                    }

                    let map_empty_fn = self
                        .module
                        .get_function("clorus_map_empty")
                        .ok_or("clorus_map_empty not declared")?;
                    let map_assoc_fn = self
                        .module
                        .get_function("clorus_map_assoc")
                        .ok_or("clorus_map_assoc not declared")?;

                    let mut out_map = self
                        .builder
                        .build_call(map_empty_fn, &[], "methods_empty")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let mut grouped: Vec<(Expr, Vec<MultimethodMethod<'ctx>>)> = Vec::new();
                    for method in self
                        .multimethod_methods
                        .get(&mm_name)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                    {
                        if let Some((_, entries)) = grouped
                            .iter_mut()
                            .find(|(dispatch, _)| *dispatch == method.dispatch_value)
                        {
                            entries.push(method);
                        } else {
                            grouped.push((method.dispatch_value.clone(), vec![method]));
                        }
                    }

                    for (idx, (dispatch_expr, methods)) in grouped.iter().enumerate() {
                        let key = self.compile_expr(dispatch_expr)?;
                        let fn_val = self.build_function_value_from_method_set(
                            methods,
                            &format!("methods_value_{}", idx),
                        )?;
                        out_map = self
                            .builder
                            .build_call(
                                map_assoc_fn,
                                &[out_map.into(), key.into(), fn_val.into()],
                                &format!("methods_assoc_{}", idx),
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    }
                    return Ok(out_map);
                }

                if func == "prefers" {
                    if args.len() != 1 {
                        return Err(
                            "prefers expects exactly one argument: (prefers multimethod-symbol)"
                                .to_string(),
                        );
                    }
                    let Some(mm_name) = Self::multimethod_name_from_symbol_arg(&args[0]) else {
                        return Err("prefers requires a multimethod symbol argument".to_string());
                    };
                    if !self.multimethod_dispatch.contains_key(&mm_name) {
                        return Err(format!(
                            "prefers references undefined multimethod '{}'",
                            mm_name
                        ));
                    }

                    let map_empty_fn = self
                        .module
                        .get_function("clorus_map_empty")
                        .ok_or("clorus_map_empty not declared")?;
                    let map_assoc_fn = self
                        .module
                        .get_function("clorus_map_assoc")
                        .ok_or("clorus_map_assoc not declared")?;
                    let set_empty_fn = self
                        .module
                        .get_function("clorus_set_empty")
                        .ok_or("clorus_set_empty not declared")?;
                    let set_conj_fn = self
                        .module
                        .get_function("clorus_set_conj")
                        .ok_or("clorus_set_conj not declared")?;

                    let mut grouped: Vec<(Expr, Vec<Expr>)> = Vec::new();
                    for (preferred, over) in self
                        .multimethod_preferences
                        .get(&mm_name)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                    {
                        if let Some((_, overs)) = grouped
                            .iter_mut()
                            .find(|(dispatch, _)| *dispatch == preferred)
                        {
                            if !overs.iter().any(|x| x == &over) {
                                overs.push(over);
                            }
                        } else {
                            grouped.push((preferred, vec![over]));
                        }
                    }

                    let mut out_map = self
                        .builder
                        .build_call(map_empty_fn, &[], "prefers_empty")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    for (idx, (preferred, overs)) in grouped.iter().enumerate() {
                        let key = self.compile_expr(preferred)?;
                        let mut over_set = self
                            .builder
                            .build_call(set_empty_fn, &[], &format!("prefers_set_empty_{}", idx))
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        for (over_idx, over_expr) in overs.iter().enumerate() {
                            let over_val = self.compile_expr(over_expr)?;
                            over_set = self
                                .builder
                                .build_call(
                                    set_conj_fn,
                                    &[over_set.into(), over_val.into()],
                                    &format!("prefers_set_conj_{}_{}", idx, over_idx),
                                )
                                .unwrap()
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value();
                        }
                        out_map = self
                            .builder
                            .build_call(
                                map_assoc_fn,
                                &[out_map.into(), key.into(), over_set.into()],
                                &format!("prefers_assoc_{}", idx),
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    }
                    return Ok(out_map);
                }

                if func == "get-method" {
                    if args.len() != 2 {
                        return Err("get-method expects exactly two arguments: (get-method multimethod-symbol dispatch-val)".to_string());
                    }
                    let Some(mm_name) = Self::multimethod_name_from_symbol_arg(&args[0]) else {
                        return Err("get-method requires a multimethod symbol as first argument"
                            .to_string());
                    };
                    if !self.multimethod_dispatch.contains_key(&mm_name) {
                        return Err(format!(
                            "get-method references undefined multimethod '{}'",
                            mm_name
                        ));
                    }

                    let dispatch_key = self.compile_expr(&args[1])?;
                    let equals_fn = self
                        .module
                        .get_function("clorus_equals")
                        .ok_or("clorus_equals not declared")?;
                    let isa_fn = self
                        .module
                        .get_function("clorus_isa_i32")
                        .ok_or("clorus_isa_i32 not declared")?;
                    let nil_fn = self
                        .module
                        .get_function("clorus_value_nil")
                        .ok_or("clorus_value_nil not declared")?;

                    let mut grouped: Vec<(Expr, Vec<MultimethodMethod<'ctx>>)> = Vec::new();
                    let mut default_methods: Vec<MultimethodMethod<'ctx>> = Vec::new();
                    for method in self
                        .multimethod_methods
                        .get(&mm_name)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                    {
                        if method.dispatch_value == Expr::Keyword("default".to_string()) {
                            default_methods.push(method);
                            continue;
                        }
                        if let Some((_, entries)) = grouped
                            .iter_mut()
                            .find(|(dispatch, _)| *dispatch == method.dispatch_value)
                        {
                            entries.push(method);
                        } else {
                            grouped.push((method.dispatch_value.clone(), vec![method]));
                        }
                    }

                    if grouped.len() > 1 {
                        grouped.sort_by(|(a, _), (b, _)| {
                            if self.multimethod_prefers(&mm_name, a, b) {
                                std::cmp::Ordering::Less
                            } else if self.multimethod_prefers(&mm_name, b, a) {
                                std::cmp::Ordering::Greater
                            } else {
                                std::cmp::Ordering::Equal
                            }
                        });
                    }

                    let function = self
                        .builder
                        .get_insert_block()
                        .and_then(|b| b.get_parent())
                        .ok_or("Call must be inside a function")?;
                    let merge_bb = self
                        .context
                        .append_basic_block(function, "get_method_merge");
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let mut incoming: Vec<(PointerValue<'ctx>, BasicBlock<'ctx>)> = Vec::new();

                    for (idx, (dispatch_expr, methods)) in grouped.iter().enumerate() {
                        let match_bb = self
                            .context
                            .append_basic_block(function, &format!("get_method_match_{}", idx));
                        let next_bb = self
                            .context
                            .append_basic_block(function, &format!("get_method_next_{}", idx));

                        let expected = self.compile_expr(dispatch_expr)?;
                        let exact_match_raw = self
                            .builder
                            .build_call(
                                equals_fn,
                                &[dispatch_key.into(), expected.into()],
                                &format!("get_method_exact_match_raw_{}", idx),
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_int_value();
                        let exact_match_norm = self
                            .builder
                            .build_and(
                                exact_match_raw,
                                exact_match_raw.get_type().const_int(1, false),
                                &format!("get_method_exact_match_norm_{}", idx),
                            )
                            .unwrap();
                        let exact_match = self
                            .builder
                            .build_int_compare(
                                IntPredicate::NE,
                                exact_match_norm,
                                exact_match_raw.get_type().const_zero(),
                                &format!("get_method_exact_match_{}", idx),
                            )
                            .unwrap();
                        let isa_match_i32 = self
                            .builder
                            .build_call(
                                isa_fn,
                                &[dispatch_key.into(), expected.into()],
                                &format!("get_method_isa_match_{}", idx),
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_int_value();
                        let isa_match = self
                            .builder
                            .build_int_compare(
                                IntPredicate::NE,
                                isa_match_i32,
                                self.context.i32_type().const_zero(),
                                &format!("get_method_isa_match_bool_{}", idx),
                            )
                            .unwrap();
                        let is_match = self
                            .builder
                            .build_or(
                                exact_match,
                                isa_match,
                                &format!("get_method_is_match_{}", idx),
                            )
                            .unwrap();
                        self.builder
                            .build_conditional_branch(is_match, match_bb, next_bb)
                            .unwrap();

                        self.builder.position_at_end(match_bb);
                        let fn_val = self.build_function_value_from_method_set(
                            methods,
                            &format!("get_method_value_{}", idx),
                        )?;
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                        let bb = self.builder.get_insert_block().unwrap();
                        incoming.push((fn_val, bb));

                        self.builder.position_at_end(next_bb);
                    }

                    let fallback = if default_methods.is_empty() {
                        self.builder
                            .build_call(nil_fn, &[], "get_method_nil")
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value()
                    } else {
                        self.build_function_value_from_method_set(
                            &default_methods,
                            "get_method_default",
                        )?
                    };
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                    let fallback_bb = self.builder.get_insert_block().unwrap();
                    incoming.push((fallback, fallback_bb));

                    self.builder.position_at_end(merge_bb);
                    let phi = self
                        .builder
                        .build_phi(value_ptr_type, "get_method_result")
                        .unwrap();
                    for (v, b) in incoming {
                        phi.add_incoming(&[(&v, b)]);
                    }
                    return Ok(phi.as_basic_value().into_pointer_value());
                }

                // Multimethod dispatch: resolve dispatch value and route to matching defmethod.
                if let Some(dispatch_expr) = self.multimethod_dispatch.get(func).cloned() {
                    let methods_all = self
                        .multimethod_methods
                        .get(func)
                        .cloned()
                        .unwrap_or_default();

                    let mut arg_values = Vec::new();
                    for arg in args {
                        arg_values.push(self.compile_expr(arg)?);
                    }
                    let dispatch_key =
                        self.compile_multimethod_dispatch_key(&dispatch_expr, &arg_values)?;

                    let mut methods = Vec::new();
                    let mut default_method: Option<MultimethodMethod<'ctx>> = None;
                    for m in methods_all.into_iter() {
                        if m.arity != arg_values.len() {
                            continue;
                        }
                        if m.dispatch_value == Expr::Keyword("default".to_string()) {
                            default_method = Some(m);
                        } else {
                            methods.push(m);
                        }
                    }

                    if methods.len() > 1 {
                        methods.sort_by(|a, b| {
                            if self.multimethod_prefers(func, &a.dispatch_value, &b.dispatch_value)
                            {
                                std::cmp::Ordering::Less
                            } else if self.multimethod_prefers(
                                func,
                                &b.dispatch_value,
                                &a.dispatch_value,
                            ) {
                                std::cmp::Ordering::Greater
                            } else {
                                std::cmp::Ordering::Equal
                            }
                        });
                    }

                    let function = self
                        .builder
                        .get_insert_block()
                        .and_then(|b| b.get_parent())
                        .ok_or("Call must be inside a function")?;
                    let merge_bb = self.context.append_basic_block(function, "mm_merge");
                    let mut incoming: Vec<(PointerValue<'ctx>, BasicBlock<'ctx>)> = Vec::new();
                    let equals_fn = self
                        .module
                        .get_function("clorus_equals")
                        .ok_or("clorus_equals not declared")?;
                    let isa_fn = self
                        .module
                        .get_function("clorus_isa_i32")
                        .ok_or("clorus_isa_i32 not declared")?;

                    for (idx, method) in methods.iter().enumerate() {
                        let match_bb = self
                            .context
                            .append_basic_block(function, &format!("mm_match_{}", idx));
                        let next_bb = self
                            .context
                            .append_basic_block(function, &format!("mm_next_{}", idx));

                        let expected = self.compile_expr(&method.dispatch_value)?;
                        let exact_match_raw = self
                            .builder
                            .build_call(
                                equals_fn,
                                &[dispatch_key.into(), expected.into()],
                                &format!("mm_exact_match_{}", idx),
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_int_value();
                        let exact_match_norm = self
                            .builder
                            .build_and(
                                exact_match_raw,
                                exact_match_raw.get_type().const_int(1, false),
                                &format!("mm_exact_match_norm_{}", idx),
                            )
                            .unwrap();
                        let exact_match = self
                            .builder
                            .build_int_compare(
                                IntPredicate::NE,
                                exact_match_norm,
                                exact_match_raw.get_type().const_zero(),
                                &format!("mm_exact_match_bool_{}", idx),
                            )
                            .unwrap();
                        let isa_match_i32 = self
                            .builder
                            .build_call(
                                isa_fn,
                                &[dispatch_key.into(), expected.into()],
                                &format!("mm_isa_match_{}", idx),
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_int_value();
                        let isa_match = self
                            .builder
                            .build_int_compare(
                                IntPredicate::NE,
                                isa_match_i32,
                                self.context.i32_type().const_zero(),
                                &format!("mm_isa_match_bool_{}", idx),
                            )
                            .unwrap();
                        let is_match = self
                            .builder
                            .build_or(exact_match, isa_match, &format!("mm_is_match_{}", idx))
                            .unwrap();
                        self.builder
                            .build_conditional_branch(is_match, match_bb, next_bb)
                            .unwrap();

                        self.builder.position_at_end(match_bb);
                        let metadata_args: Vec<BasicMetadataValueEnum> =
                            arg_values.iter().map(|v| (*v).into()).collect();
                        let result = self
                            .builder
                            .build_call(
                                method.function,
                                &metadata_args,
                                &format!("mm_call_{}", idx),
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                        let match_end = self.builder.get_insert_block().unwrap();
                        incoming.push((result, match_end));

                        self.builder.position_at_end(next_bb);
                    }

                    if let Some(method) = default_method {
                        let metadata_args: Vec<BasicMetadataValueEnum> =
                            arg_values.iter().map(|v| (*v).into()).collect();
                        let result = self
                            .builder
                            .build_call(method.function, &metadata_args, "mm_call_default")
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                        let bb = self.builder.get_insert_block().unwrap();
                        incoming.push((result, bb));
                    } else {
                        let nil_fn = self
                            .module
                            .get_function("clorus_value_nil")
                            .ok_or("clorus_value_nil not declared")?;
                        let nil_val = self
                            .builder
                            .build_call(nil_fn, &[], "mm_no_method_nil")
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                        let bb = self.builder.get_insert_block().unwrap();
                        incoming.push((nil_val, bb));
                    }

                    self.builder.position_at_end(merge_bb);
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let phi = self.builder.build_phi(value_ptr_type, "mm_result").unwrap();
                    for (v, b) in incoming {
                        phi.add_incoming(&[(&v, b)]);
                    }
                    return Ok(phi.as_basic_value().into_pointer_value());
                }

                // PHASE 5.2: Automatic Protocol Dispatch
                // Check if this is a protocol method call
                if let Some((protocol_name, param_count)) = self.protocol_methods.get(func).cloned()
                {
                    // Verify argument count matches
                    if args.len() != param_count {
                        return Err(format!(
                            "Protocol method {} expects {} arguments, got {}",
                            func,
                            param_count,
                            args.len()
                        ));
                    }

                    // Compile all arguments first
                    let mut arg_values = Vec::new();
                    for arg in args {
                        arg_values.push(self.compile_expr(arg)?);
                    }

                    // Extract __type__ from first argument (the instance)
                    let instance = arg_values[0];

                    // Get map-get function to extract __type__ field
                    let map_get_fn = self
                        .module
                        .get_function("clorus_map_get")
                        .ok_or("clorus_map_get not declared")?;

                    // Create __type__ keyword
                    let keyword_fn = self
                        .module
                        .get_function("clorus_keyword")
                        .ok_or("clorus_keyword not declared")?;
                    let type_keyword_str = self
                        .builder
                        .build_global_string_ptr("__type__", "dispatch_type_keyword")
                        .unwrap();
                    let type_keyword = self
                        .builder
                        .build_call(
                            keyword_fn,
                            &[type_keyword_str.as_pointer_value().into()],
                            "dispatch_type_kw",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Extract type name from instance
                    let type_name_val = self
                        .builder
                        .build_call(
                            map_get_fn,
                            &[instance.into(), type_keyword.into()],
                            "dispatch_type_name",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Get string data from Value*
                    let str_data_fn = self
                        .module
                        .get_function("clorus_string_data")
                        .ok_or("clorus_string_data not declared")?;
                    let type_name_cstr = self
                        .builder
                        .build_call(str_data_fn, &[type_name_val.into()], "dispatch_type_cstr")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Lookup protocol method in registry
                    let lookup_fn = self
                        .module
                        .get_function("clorus_lookup_protocol_method")
                        .ok_or("clorus_lookup_protocol_method not declared")?;

                    // Create string constants for protocol and method names
                    let protocol_name_str = self
                        .builder
                        .build_global_string_ptr(&protocol_name, "dispatch_proto")
                        .unwrap();
                    let method_name_str = self
                        .builder
                        .build_global_string_ptr(func, "dispatch_method")
                        .unwrap();

                    // Call lookup function
                    let fn_ptr_int = self
                        .builder
                        .build_call(
                            lookup_fn,
                            &[
                                type_name_cstr.into(),
                                protocol_name_str.as_pointer_value().into(),
                                method_name_str.as_pointer_value().into(),
                            ],
                            "dispatch_fn_ptr",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_int_value();

                    // Check if method was found (non-zero)
                    let zero = self.context.i64_type().const_zero();
                    let found = self
                        .builder
                        .build_int_compare(IntPredicate::NE, fn_ptr_int, zero, "dispatch_found")
                        .unwrap();

                    let current_fn = self
                        .builder
                        .get_insert_block()
                        .and_then(|b| b.get_parent())
                        .ok_or("Call must be inside a function")?;

                    let found_block = self
                        .context
                        .append_basic_block(current_fn, "dispatch_found");
                    let not_found_block = self
                        .context
                        .append_basic_block(current_fn, "dispatch_not_found");
                    let continue_block = self
                        .context
                        .append_basic_block(current_fn, "dispatch_continue");

                    self.builder
                        .build_conditional_branch(found, found_block, not_found_block)
                        .unwrap();

                    // Found block: cast and call the function
                    self.builder.position_at_end(found_block);

                    let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let fn_ptr = self
                        .builder
                        .build_int_to_ptr(fn_ptr_int, i8_ptr_type, "dispatch_fn")
                        .unwrap();

                    // Create function type: (Value*, ...) -> Value*
                    let param_types: Vec<_> =
                        (0..param_count).map(|_| i8_ptr_type.into()).collect();
                    let fn_type = i8_ptr_type.fn_type(&param_types, false);

                    // Cast to function pointer type
                    let fn_ptr_typed = self
                        .builder
                        .build_pointer_cast(
                            fn_ptr,
                            fn_type.ptr_type(AddressSpace::default()),
                            "dispatch_fn_typed",
                        )
                        .unwrap();

                    // Call the function
                    let arg_metadata: Vec<_> = arg_values.iter().map(|&v| v.into()).collect();

                    let result = self
                        .builder
                        .build_indirect_call(
                            fn_type,
                            fn_ptr_typed,
                            &arg_metadata,
                            "dispatch_result",
                        )
                        .unwrap();

                    let result_val = result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                    self.builder
                        .build_unconditional_branch(continue_block)
                        .unwrap();

                    // Not found block: error
                    self.builder.position_at_end(not_found_block);

                    // Build error message that includes the actual type name
                    let println_fn = self
                        .module
                        .get_function("clorus_println_variadic")
                        .ok_or("clorus_println_variadic not declared")?;
                    let str_fn = self
                        .module
                        .get_function("clorus_value_string")
                        .ok_or("clorus_value_string not declared")?;
                    let vec_empty_fn = self
                        .module
                        .get_function("clorus_vector_empty")
                        .ok_or("clorus_vector_empty not declared")?;
                    let vec_conj_fn = self
                        .module
                        .get_function("clorus_vector_conj")
                        .ok_or("clorus_vector_conj not declared")?;

                    // Create error message parts
                    let error_msg1 = format!(
                        "No implementation of protocol method {}.{} found for type: ",
                        protocol_name, func
                    );
                    let error_str1 = self
                        .builder
                        .build_global_string_ptr(&error_msg1, "dispatch_error1")
                        .unwrap();
                    let error_val1 = self
                        .builder
                        .build_call(
                            str_fn,
                            &[error_str1.as_pointer_value().into()],
                            "error_val1",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Create vector with error message and type name
                    let vec_empty = self
                        .builder
                        .build_call(vec_empty_fn, &[], "error_vec_empty")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let vec_one = self
                        .builder
                        .build_call(
                            vec_conj_fn,
                            &[vec_empty.into(), error_val1.into()],
                            "error_vec_one",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let vec_val = self
                        .builder
                        .build_call(
                            vec_conj_fn,
                            &[vec_one.into(), type_name_val.into()],
                            "error_vec",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    self.builder
                        .build_call(println_fn, &[vec_val.into()], "print_error")
                        .unwrap();

                    // Return nil for error case
                    let nil_fn = self
                        .module
                        .get_function("clorus_value_nil")
                        .ok_or("clorus_value_nil not declared")?;
                    let error_result = self
                        .builder
                        .build_call(nil_fn, &[], "error_nil")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    self.builder
                        .build_unconditional_branch(continue_block)
                        .unwrap();

                    // Continue block: merge results
                    self.builder.position_at_end(continue_block);
                    let phi = self.builder.build_phi(i8_ptr_type, "dispatch_phi").unwrap();
                    phi.add_incoming(&[
                        (&result_val, found_block),
                        (&error_result, not_found_block),
                    ]);

                    return Ok(phi.as_basic_value().into_pointer_value());
                }

                // Handle arithmetic operators (can come from macro expansion)
                match func.as_str() {
                    "+" => return self.compile_add(args),
                    "-" => return self.compile_sub(args),
                    "*" => return self.compile_mul(args),
                    "/" => return self.compile_div(args),
                    "mod" => return self.compile_mod(args),
                    "<" => return self.compile_lt(args),
                    ">" => return self.compile_gt(args),
                    "<=" => return self.compile_lte(args),
                    ">=" => return self.compile_gte(args),
                    "=" => return self.compile_eq(args),
                    _ => {}
                }

                // Check if this is a qualified call (namespace/function or alias/function)
                if func.contains('/') {
                    let parts: Vec<&str> = func.split('/').collect();
                    if parts.len() == 2 {
                        let namespace_or_alias = parts[0];
                        let func_name = parts[1];

                        // First check if this is a Clorus namespace (aliased or direct)
                        let resolved_namespace = self
                            .namespace
                            .aliases
                            .get(namespace_or_alias)
                            .map(|s| s.as_str())
                            .unwrap_or(namespace_or_alias);

                        // Support explicit clorus.core qualification/aliasing for core intrinsics.
                        // Example: (:require [clorus.core :as core]) then (core/zero? x), (core/dec x)
                        if (namespace_or_alias == "core"
                            || namespace_or_alias == "clorus.core"
                            || resolved_namespace == "clorus.core")
                            && self.is_core_function(func_name)
                        {
                            return self.compile_core_call(func_name, args);
                        }

                        // Try to find it as a Clorus function first
                        // Generate mangled name: math/add -> clorus_math_add
                        let mangled_name = format!(
                            "clorus_{}_{}",
                            resolved_namespace.replace('.', "_").replace('-', "_"),
                            func_name.replace('-', "_")
                        );

                        if let Some(function) = self.functions.get(&mangled_name) {
                            // Found a Clorus function - compile arguments and call it
                            let function = function.clone();
                            let mut arg_values = Vec::new();
                            for arg in args {
                                arg_values.push(self.compile_expr(arg)?.into());
                            }

                            // All Clorus functions take an environment parameter as the last argument
                            // For direct calls, pass NULL (no captured environment)
                            let i8_ptr_type =
                                self.context.i8_type().ptr_type(AddressSpace::default());
                            let null_env = i8_ptr_type.const_null();
                            arg_values.push(null_env.into());

                            let call_result = self
                                .builder
                                .build_call(function, &arg_values, "call")
                                .unwrap();

                            return Ok(call_result
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value());
                        }

                        // Not a Clorus function - try Rust FFI libraries
                        let rust_lib =
                            self.resolve_rust_library(namespace_or_alias).or_else(|| {
                                // Check if this is an alias for a rust.* module.
                                self.namespace
                                    .aliases
                                    .get(namespace_or_alias)
                                    .and_then(|resolved| self.resolve_rust_library(resolved))
                            });

                        if let Some(lib) = rust_lib {
                            return self.compile_rust_library_call(&lib, func_name, args);
                        }
                    }

                    // Check if this function is from a .clip package (Phase 4)
                    if let Some(namespace_or_alias) = func.split('/').next() {
                        let func_name = func.split('/').nth(1);

                        // Resolve alias to actual namespace
                        let resolved_namespace = self
                            .namespace
                            .aliases
                            .get(namespace_or_alias)
                            .map(|s| s.as_str())
                            .unwrap_or(namespace_or_alias);

                        // Check if this namespace is from a .clip package
                        if self.is_clip_namespace(resolved_namespace) {
                            let i8_ptr_type = self
                                .context
                                .i8_type()
                                .ptr_type(inkwell::AddressSpace::default());

                            // Auto-declare function from .clip package
                            // Use FULL resolved namespace for mangling
                            let full_func_name = if let Some(fname) = func_name {
                                format!("{}/{}", resolved_namespace, fname)
                            } else {
                                func.to_string()
                            };

                            let mangled_name = format!(
                                "clorus_{}",
                                full_func_name
                                    .replace('/', "_")
                                    .replace('.', "_")
                                    .replace('-', "_")
                            );

                            // Declare if not already declared
                            if self.module.get_function(&mangled_name).is_none() {
                                // All Clorus functions: (Value*, ..., Value*) -> Value*
                                // Plus environment parameter as last arg
                                let param_types: Vec<inkwell::types::BasicMetadataTypeEnum> =
                                    std::iter::repeat(i8_ptr_type.into())
                                        .take(args.len() + 1) // +1 for environment
                                        .collect();

                                let fn_type = i8_ptr_type.fn_type(&param_types, false);
                                self.module.add_function(&mangled_name, fn_type, None);

                                // Debug: Auto-declared .clip function
                                // eprintln!("   [DEBUG] Auto-declared .clip function: {} -> {}",
                                //     func, mangled_name);
                            }

                            // Now call it like a normal Clorus function
                            if let Some(function) = self.module.get_function(&mangled_name) {
                                let mut arg_values: Vec<inkwell::values::BasicMetadataValueEnum> =
                                    Vec::new();
                                for arg in args {
                                    let val = self.compile_expr(arg)?;
                                    arg_values.push(val.into());
                                }

                                // Add NULL environment (no captured variables for .clip functions)
                                let null_env = i8_ptr_type.const_null();
                                arg_values.push(null_env.into());

                                let call_result = self
                                    .builder
                                    .build_call(function, &arg_values, "call")
                                    .unwrap();

                                return Ok(call_result
                                    .try_as_basic_value()
                                    .left()
                                    .unwrap()
                                    .into_pointer_value());
                            }
                        }
                    }

                    return Err(format!(
                        "Unknown function: {}. Did you (use rust.{})?",
                        func,
                        func.split('/').next().unwrap()
                    ));
                }

                // Check if this is a clorus.core function call
                if self.is_core_function(func) {
                    return self.compile_core_call(func, args);
                }

                // Check if function name is a local variable (first-class functions)
                // This allows: (fn [f] (f 42))
                let func_var_ptr = self.variables.get(func).cloned();

                if let Some(var_ptr) = func_var_ptr {
                    // It's a variable - use dynamic dispatch via clorus_function_call
                    let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let raw_func_val = self
                        .builder
                        .build_load(i8_ptr_type, var_ptr, "load_func_var")
                        .unwrap()
                        .into_pointer_value();
                    let deref_fn = self
                        .module
                        .get_function("clorus_deref_var_value")
                        .ok_or("clorus_deref_var_value not declared")?;
                    let func_val = self
                        .builder
                        .build_call(deref_fn, &[raw_func_val.into()], "deref_func_var")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Compile arguments
                    let mut arg_values = Vec::new();
                    for arg in args {
                        arg_values.push(self.compile_expr(arg)?);
                    }

                    // Create array of argument pointers
                    let arg_count = arg_values.len();
                    let args_array_ptr = if arg_count > 0 {
                        let array_type = i8_ptr_type.array_type(arg_count as u32);
                        let array_alloca =
                            self.builder.build_alloca(array_type, "args_array").unwrap();

                        for (i, arg_val) in arg_values.iter().enumerate() {
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_gep(
                                        array_type,
                                        array_alloca,
                                        &[
                                            self.context.i32_type().const_zero(),
                                            self.context.i32_type().const_int(i as u64, false),
                                        ],
                                        &format!("arg_{}_ptr", i),
                                    )
                                    .unwrap()
                            };
                            self.builder.build_store(elem_ptr, *arg_val).unwrap();
                        }

                        self.builder
                            .build_pointer_cast(
                                array_alloca,
                                i8_ptr_type.ptr_type(AddressSpace::default()),
                                "args_array_cast",
                            )
                            .unwrap()
                    } else {
                        i8_ptr_type.ptr_type(AddressSpace::default()).const_null()
                    };

                    // Call clorus_function_call
                    let function_call_fn = self
                        .module
                        .get_function("clorus_function_call")
                        .ok_or("clorus_function_call not declared")?;

                    let arg_count_val = self.context.i32_type().const_int(arg_count as u64, false);

                    let call_result = self
                        .builder
                        .build_call(
                            function_call_fn,
                            &[func_val.into(), args_array_ptr.into(), arg_count_val.into()],
                            "dynamic_call",
                        )
                        .unwrap();

                    return Ok(call_result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value());
                }

                // Check if this is a referred symbol (imported via :refer)
                // If so, resolve to the full qualified name
                let function_to_lookup = if let Some(binding) = self.namespace.imports.get(func) {
                    // This is a referred symbol - create the mangled name
                    format!(
                        "clorus_{}_{}",
                        binding.namespace.replace('.', "_").replace('-', "_"),
                        binding.symbol.replace('-', "_")
                    )
                } else if self.namespace.current != "user" {
                    // Try current namespace's function (for local calls)
                    format!(
                        "clorus_{}_{}",
                        self.namespace.current.replace('.', "_").replace('-', "_"),
                        func.replace('-', "_")
                    )
                } else {
                    // Default namespace - use simple name
                    func.to_string()
                };

                // Prefer direct/static function calls when possible (important for defn->defn calls),
                // and only fall back to global dynamic dispatch when no static target exists.
                let arity_variant = format!("{}_arity_{}", function_to_lookup, args.len());
                let fallback_arity_variant = format!("{}_arity_{}", func, args.len());
                let variadic_variant =
                    self.find_variadic_arity_variant(&function_to_lookup, args.len());
                let fallback_variadic_variant = if function_to_lookup != *func {
                    self.find_variadic_arity_variant(func, args.len())
                } else {
                    None
                };
                let has_static_target = self.functions.contains_key(&arity_variant)
                    || self.functions.contains_key(&fallback_arity_variant)
                    || variadic_variant
                        .as_ref()
                        .map(|name| self.functions.contains_key(name))
                        .unwrap_or(false)
                    || fallback_variadic_variant
                        .as_ref()
                        .map(|name| self.functions.contains_key(name))
                        .unwrap_or(false)
                    || self.functions.contains_key(&function_to_lookup)
                    || self.functions.contains_key(func);
                if !has_static_target {
                    let resolved_name = if func.contains('/') {
                        let parts: Vec<&str> = func.split('/').collect();
                        if parts.len() == 2 {
                            let namespace_or_alias = parts[0];
                            let var_name = parts[1];
                            let resolved_namespace = self
                                .namespace
                                .aliases
                                .get(namespace_or_alias)
                                .map(|s| s.as_str())
                                .unwrap_or(namespace_or_alias);
                            if resolved_namespace == "user" {
                                var_name.to_string()
                            } else {
                                format!(
                                    "clorus_{}_{}",
                                    resolved_namespace.replace('.', "_").replace('-', "_"),
                                    var_name.replace('-', "_")
                                )
                            }
                        } else {
                            func.clone()
                        }
                    } else if self.namespace.current == "user" {
                        func.clone()
                    } else {
                        format!(
                            "clorus_{}_{}",
                            self.namespace.current.replace('.', "_").replace('-', "_"),
                            func.replace('-', "_")
                        )
                    };

                    if let Some(global_ptr) = self
                        .globals
                        .get(&resolved_name)
                        .map(|g| g.as_pointer_value())
                    {
                        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                        let raw_func_val = self
                            .builder
                            .build_load(i8_ptr_type, global_ptr, "load_global_func_var")
                            .unwrap()
                            .into_pointer_value();
                        let deref_fn = self
                            .module
                            .get_function("clorus_deref_var_value")
                            .ok_or("clorus_deref_var_value not declared")?;
                        let func_val = self
                            .builder
                            .build_call(deref_fn, &[raw_func_val.into()], "deref_global_func_var")
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();

                        let mut arg_values = Vec::new();
                        for arg in args {
                            arg_values.push(self.compile_expr(arg)?);
                        }

                        let arg_count = arg_values.len();
                        let args_array_ptr = if arg_count > 0 {
                            let array_type = i8_ptr_type.array_type(arg_count as u32);
                            let array_alloca = self
                                .builder
                                .build_alloca(array_type, "global_args_array")
                                .unwrap();
                            for (i, arg_val) in arg_values.iter().enumerate() {
                                let elem_ptr = unsafe {
                                    self.builder
                                        .build_gep(
                                            array_type,
                                            array_alloca,
                                            &[
                                                self.context.i32_type().const_zero(),
                                                self.context.i32_type().const_int(i as u64, false),
                                            ],
                                            &format!("global_arg_{}_ptr", i),
                                        )
                                        .unwrap()
                                };
                                self.builder.build_store(elem_ptr, *arg_val).unwrap();
                            }
                            self.builder
                                .build_pointer_cast(
                                    array_alloca,
                                    i8_ptr_type.ptr_type(AddressSpace::default()),
                                    "global_args_array_cast",
                                )
                                .unwrap()
                        } else {
                            i8_ptr_type.ptr_type(AddressSpace::default()).const_null()
                        };

                        let function_call_fn = self
                            .module
                            .get_function("clorus_function_call")
                            .ok_or("clorus_function_call not declared")?;
                        let arg_count_val =
                            self.context.i32_type().const_int(arg_count as u64, false);
                        let call_result = self
                            .builder
                            .build_call(
                                function_call_fn,
                                &[func_val.into(), args_array_ptr.into(), arg_count_val.into()],
                                "global_dynamic_call",
                            )
                            .unwrap();
                        return Ok(call_result
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value());
                    }
                }

                // Look up the function (clone to avoid borrow issues)
                // Try the resolved name first, then fall back to simple name
                // For multi-arity functions, try to find the specific arity variant
                let function = {
                    // Compile arguments first to know the count
                    let mut arg_values = Vec::new();
                    for arg in args {
                        arg_values.push(self.compile_expr(arg)?);
                    }

                    // Try to find multi-arity variant: function_arity_N
                    let arity_variant =
                        format!("{}_arity_{}", function_to_lookup, arg_values.len());
                    let fallback_arity_variant = format!("{}_arity_{}", func, arg_values.len());

                    if let Some(func) = self.functions.get(&arity_variant) {
                        // Found multi-arity variant
                        (func.clone(), arg_values)
                    } else if let Some(func) = self.functions.get(&fallback_arity_variant) {
                        // Found multi-arity variant via bare fallback name
                        (func.clone(), arg_values)
                    } else if let Some(variadic_name) =
                        self.find_variadic_arity_variant(&function_to_lookup, arg_values.len())
                    {
                        if let Some(func) = self.functions.get(&variadic_name) {
                            (func.clone(), arg_values)
                        } else {
                            return Err(format!(
                                "Undefined variadic function variant: {} for call {}",
                                variadic_name, func
                            ));
                        }
                    } else if let Some(variadic_name) =
                        self.find_variadic_arity_variant(func, arg_values.len())
                    {
                        if let Some(func) = self.functions.get(&variadic_name) {
                            (func.clone(), arg_values)
                        } else {
                            return Err(format!(
                                "Undefined variadic function variant: {} for call {}",
                                variadic_name, func
                            ));
                        }
                    } else {
                        // If there are generated arity variants for this symbol, do not fall
                        // back to a base-name function body when no matching arity exists.
                        // That fallback can select an incompatible function signature.
                        if self.has_any_arity_variants(&function_to_lookup)
                            || (function_to_lookup != *func && self.has_any_arity_variants(func))
                        {
                            return Err(format!(
                                "Arity mismatch: function '{}' has no {}-arity variant",
                                func,
                                arg_values.len()
                            ));
                        }

                        // Try regular function lookup
                        let func = self
                            .functions
                            .get(&function_to_lookup)
                            .or_else(|| self.functions.get(func))
                            .ok_or_else(|| {
                                format!(
                                    "Undefined function: {} (tried {} and multi-arity variants)",
                                    func, function_to_lookup
                                )
                            })?
                            .clone();
                        (func, arg_values)
                    }
                };

                let (function, mut arg_values) = function;

                // Shape direct-call args for variadic functions:
                // fixed args + collected rest vector + env
                let function_name = function.get_name().to_string_lossy().to_string();
                if let Some((fixed_params, has_rest)) =
                    self.function_signatures.get(&function_name).copied()
                {
                    if has_rest {
                        if arg_values.len() < fixed_params {
                            return Err(format!(
                                "Arity mismatch: function '{}' expects at least {} argument(s), got {}",
                                function_name,
                                fixed_params,
                                arg_values.len()
                            ));
                        }

                        let fixed_args: Vec<_> =
                            arg_values.iter().take(fixed_params).cloned().collect();
                        let rest_args: Vec<_> =
                            arg_values.iter().skip(fixed_params).copied().collect();
                        let rest_vec = self.build_rest_vector_from_values(&rest_args)?;

                        arg_values.clear();
                        for arg in fixed_args {
                            arg_values.push(arg);
                        }
                        arg_values.push(rest_vec);
                    } else if arg_values.len() != fixed_params {
                        return Err(format!(
                            "Arity mismatch: function '{}' expects {}, got {}",
                            function_name,
                            fixed_params,
                            arg_values.len()
                        ));
                    }
                }

                // All Clorus functions take an environment parameter as the last argument
                // For direct calls, pass NULL (no captured environment)
                let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let null_env = i8_ptr_type.const_null();
                arg_values.push(null_env);

                let metadata_args: Vec<BasicMetadataValueEnum> =
                    arg_values.iter().map(|v| (*v).into()).collect();

                // Build call (function now returns Value*)
                let call_result = self
                    .builder
                    .build_call(function, &metadata_args, "call")
                    .unwrap();

                Ok(call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                // Compile condition (returns Value*)
                let cond_val_ptr = self.compile_expr(condition)?;

                // Use clorus_is_truthy to check if condition is truthy
                let is_truthy_fn = self
                    .module
                    .get_function("clorus_is_truthy")
                    .ok_or("clorus_is_truthy not declared")?;
                let truthy_result = self
                    .builder
                    .build_call(is_truthy_fn, &[cond_val_ptr.into()], "is_truthy")
                    .unwrap();
                let truthy_i32 = truthy_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                // Convert i32 to i1 (bool) for conditional branch
                let cond_bool = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        truthy_i32,
                        self.context.i32_type().const_int(0, false),
                        "ifcond",
                    )
                    .unwrap();

                // Get current function
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(|block| block.get_parent())
                    .expect("No parent function");

                // Create basic blocks
                let then_bb = self.context.append_basic_block(function, "then");
                let else_bb = self.context.append_basic_block(function, "else");
                let merge_bb = self.context.append_basic_block(function, "ifcont");

                // Build conditional branch
                self.builder
                    .build_conditional_branch(cond_bool, then_bb, else_bb)
                    .unwrap();

                // Build then block
                self.builder.position_at_end(then_bb);
                let then_val = self.compile_expr(then_branch)?;

                // Only branch to merge if block doesn't already have a terminator (from recur/return/throw)
                let then_bb_after = self.builder.get_insert_block().unwrap();
                let then_has_terminator = then_bb_after.get_terminator().is_some();
                if !then_has_terminator {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }
                let then_bb = self.builder.get_insert_block().unwrap();

                // Build else block
                self.builder.position_at_end(else_bb);
                let else_val = self.compile_expr(else_branch)?;

                // Only branch to merge if block doesn't already have a terminator
                let else_bb_after = self.builder.get_insert_block().unwrap();
                let else_has_terminator = else_bb_after.get_terminator().is_some();
                if !else_has_terminator {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }
                let else_bb = self.builder.get_insert_block().unwrap();

                // Build merge block with phi node (phi node type is now Value*)
                self.builder.position_at_end(merge_bb);

                // Check if merge block is reachable (at least one branch reaches here)
                if !then_has_terminator || !else_has_terminator {
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let phi = self.builder.build_phi(value_ptr_type, "iftmp").unwrap();

                    // Add incoming values only from branches that reach here
                    if !then_has_terminator {
                        phi.add_incoming(&[(&then_val, then_bb)]);
                    }
                    if !else_has_terminator {
                        phi.add_incoming(&[(&else_val, else_bb)]);
                    }

                    Ok(phi.as_basic_value().into_pointer_value())
                } else {
                    // Both branches have terminators (recur/return/throw), merge block is unreachable
                    // Return a dummy value - this code path is unreachable but needed for compilation
                    // The merge block will be removed by LLVM's dead code elimination
                    let nil_fn = self
                        .module
                        .get_function("clorus_value_nil")
                        .ok_or("clorus_value_nil not declared")?;
                    let dummy_call = self
                        .builder
                        .build_call(nil_fn, &[], "unreachable_value")
                        .unwrap();
                    Ok(dummy_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value())
                }
            }

            Expr::Ns {
                name,
                requires,
                rust_imports,
            } => {
                // Update namespace context
                self.namespace.current = name.clone();

                // Process requires - update namespace context
                for req_spec in requires {
                    if let Some(ref alias) = req_spec.alias {
                        self.namespace
                            .register_alias(alias.clone(), req_spec.module.clone())?;
                    }
                    // Import specific symbols
                    for symbol in &req_spec.refer {
                        self.namespace.register_import(
                            symbol.clone(),
                            req_spec.module.clone(),
                            symbol.clone(),
                        )?;
                    }
                    for (source_symbol, local_symbol) in &req_spec.rename {
                        self.namespace.register_import(
                            local_symbol.clone(),
                            req_spec.module.clone(),
                            source_symbol.clone(),
                        )?;
                    }
                }

                // Process rust imports - register aliases AND declare FFI functions
                for rust_import in rust_imports {
                    if let Some(ref alias) = rust_import.alias {
                        // Register as rust.library format (keep hyphens as-is for lookup)
                        let rust_module = format!("rust.{}", rust_import.library);
                        self.namespace.aliases.insert(alias.clone(), rust_module);
                    }

                    // Declare FFI functions for this library
                    if let Some(lib) = self.resolve_rust_library(&rust_import.library) {
                        self.declare_rust_library_functions(&lib)?;
                    }
                }

                // Return nil
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            Expr::Require { specs } => {
                // Process all require specs
                for req_spec in specs {
                    if let Some(ref alias) = req_spec.alias {
                        self.namespace
                            .register_alias(alias.clone(), req_spec.module.clone())?;
                    }
                    // Import specific symbols
                    for symbol in &req_spec.refer {
                        self.namespace.register_import(
                            symbol.clone(),
                            req_spec.module.clone(),
                            symbol.clone(),
                        )?;
                    }
                    for (source_symbol, local_symbol) in &req_spec.rename {
                        self.namespace.register_import(
                            local_symbol.clone(),
                            req_spec.module.clone(),
                            source_symbol.clone(),
                        )?;
                    }
                }

                // Return nil
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            Expr::Declare { names } => {
                // Forward declare functions for mutual recursion
                // This allows functions to reference each other before they're defined
                for name in names {
                    // Add to forward declarations set
                    self.forward_declarations.insert(name.clone());

                    // Compute namespace-mangled name (same logic as in Defn)
                    let mangled_name = if self.namespace.current == "user" {
                        name.clone()
                    } else {
                        format!(
                            "clorus_{}_{}",
                            self.namespace.current.replace('.', "_").replace('-', "_"),
                            name.replace('-', "_")
                        )
                    };

                    // Create an LLVM function declaration (prototype) with variadic args
                    // We use variadic because we don't know the arity yet
                    // The actual implementation will replace this when defn is compiled
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let fn_type = value_ptr_type.fn_type(&[], true); // variadic

                    // Add or get the function (don't error if it exists)
                    if self.module.get_function(&mangled_name).is_none() {
                        let func = self.module.add_function(&mangled_name, fn_type, None);
                        // IMPORTANT: Use mangled name as key (same as Defn does)
                        self.functions.insert(mangled_name.clone(), func);
                    }
                }

                // Return nil (declare is compile-time only, produces no runtime value)
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            Expr::Use { module, imports: _ } => {
                // Check if this is a registered Rust FFI library
                if module.starts_with("rust.") {
                    let lib_name = module.strip_prefix("rust.").unwrap();

                    if let Some(lib) = self.resolve_rust_library(lib_name) {
                        // Auto-declare all functions from this library
                        self.declare_rust_library_functions(&lib)?;
                    } else {
                        return Err(format!(
                            "Unknown rust module: {}. Add it to rust-dependencies in Clorus.toml.",
                            module
                        ));
                    }
                } else if module == "clorus.core" {
                    self.declare_core_functions();
                } else if self.is_clip_namespace(module) {
                    // Module is from a .clip package or registered local module
                    // Functions from these modules are available at runtime via FFI
                    // No need to declare anything here
                } else {
                    return Err(format!("Unknown module: {}", module));
                }

                // use statements don't return a meaningful value, return boxed 0
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            Expr::List(list) if !list.is_empty() => {
                // Handle keyword calls: (:key map) => (get map :key)
                if let Expr::Keyword(key) = &list[0] {
                    if list.len() == 2 {
                        // Keyword call with one argument: (:key map) => (get map :key)
                        return self.compile_core_call(
                            "get",
                            &[list[1].clone(), Expr::Keyword(key.clone())],
                        );
                    } else if list.len() == 3 {
                        // Keyword call with default: (:key map default) => (get map :key default)
                        return self.compile_core_call(
                            "get",
                            &[list[1].clone(), Expr::Keyword(key.clone()), list[2].clone()],
                        );
                    } else {
                        return Err(format!("Keyword call {:?} requires 1 or 2 arguments", key));
                    }
                }

                // Handle (op arg1 arg2 ...)
                if let Expr::Symbol(op) = &list[0] {
                    match op.as_str() {
                        "+" => self.compile_add(&list[1..]),
                        "-" => self.compile_sub(&list[1..]),
                        "*" => self.compile_mul(&list[1..]),
                        "/" => self.compile_div(&list[1..]),
                        "<" => self.compile_lt(&list[1..]),
                        ">" => self.compile_gt(&list[1..]),
                        "=" => self.compile_eq(&list[1..]),
                        _ => Err(format!("Unknown operator: {}", op)),
                    }
                } else {
                    Err("First element of list must be a symbol or keyword".to_string())
                }
            }

            Expr::List(_) => Err("Empty list cannot be compiled".to_string()),

            _ => Err(format!("Cannot compile expression: {:?}", expr)),
        }
    }

    pub fn get_module(&self) -> &Module<'ctx> {
        &self.module
    }

    pub fn get_builder(&self) -> &Builder<'ctx> {
        &self.builder
    }

    pub fn get_functions(&self) -> &HashMap<String, FunctionValue<'ctx>> {
        &self.functions
    }

    pub fn add_function(&mut self, name: String, function: FunctionValue<'ctx>) {
        self.functions.insert(name, function);
    }

    /// Add a forward declaration for a function (for mutual recursion)
    /// This is a compile-time directive that doesn't produce any runtime code
    pub fn add_forward_declaration(&mut self, name: &str) {
        // Add to forward declarations set
        self.forward_declarations.insert(name.to_string());

        // Compute namespace-mangled name (same logic as in Defn)
        let mangled_name = if self.namespace.current == "user" {
            name.to_string()
        } else {
            format!(
                "clorus_{}_{}",
                self.namespace.current.replace('.', "_"),
                name.replace('-', "_")
            )
        };

        // Create an LLVM function declaration (prototype) with variadic args
        // We use variadic because we don't know the arity yet
        // The actual implementation will replace this when defn is compiled
        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = value_ptr_type.fn_type(&[], true); // variadic

        // Add or get the function (don't error if it exists)
        if self.module.get_function(&mangled_name).is_none() {
            let func = self.module.add_function(&mangled_name, fn_type, None);
            self.functions.insert(mangled_name, func);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clorus_syntax::parse;

    #[test]
    fn test_compile_number() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let expr = Expr::Double(42.0);

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&expr, "test_number");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_add() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let exprs = parse("(+ 1 2)").unwrap();

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&exprs[0], "test_add");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_nested() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let exprs = parse("(+ (* 2 3) 4)").unwrap();

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&exprs[0], "test_nested");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_fn_basic() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        // Parse: (fn [x] x)
        let exprs = parse("(fn [x] x)").unwrap();

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&exprs[0], "test_fn_basic");
        assert!(result.is_ok());

        // Verify a lambda function was created
        assert!(codegen.functions.contains_key("_lambda_0"));
    }

    #[test]
    fn test_compile_fn_with_body() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        // Parse: (fn [x y] (+ x y))
        let exprs = parse("(fn [x y] (+ x y))").unwrap();

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&exprs[0], "test_fn_body");
        assert!(result.is_ok());

        // Verify a lambda function was created
        assert!(codegen.functions.contains_key("_lambda_0"));
    }

    #[test]
    fn test_compile_fn_no_params() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        // Parse: (fn [] 42)
        let exprs = parse("(fn [] 42)").unwrap();

        // Wrap in function to provide basic block context
        let result = codegen.wrap_in_function(&exprs[0], "test_fn_no_params");
        assert!(result.is_ok());

        // Verify a lambda function was created
        assert!(codegen.functions.contains_key("_lambda_0"));
    }

    #[test]
    fn test_rust_library_resolution_hyphen_underscore_prefix_variants() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        codegen.register_rust_library(RustLibrary {
            name: "egui-hello".to_string(),
            functions: vec![],
        });

        assert!(codegen.resolve_rust_library("egui-hello").is_some());
        assert!(codegen.resolve_rust_library("egui_hello").is_some());
        assert!(codegen.resolve_rust_library("rust.egui-hello").is_some());
        assert!(codegen.resolve_rust_library("rust.egui_hello").is_some());
    }

    #[test]
    fn test_rust_library_resolution_when_registered_with_prefixed_name() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        codegen.register_rust_library(RustLibrary {
            name: "rust.coral-gfx".to_string(),
            functions: vec![],
        });

        assert!(codegen.resolve_rust_library("coral-gfx").is_some());
        assert!(codegen.resolve_rust_library("coral_gfx").is_some());
        assert!(codegen.resolve_rust_library("rust.coral-gfx").is_some());
        assert!(codegen.resolve_rust_library("rust.coral_gfx").is_some());
    }

    #[test]
    fn test_use_rust_module_accepts_hyphen_and_underscore_variants() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        codegen.register_rust_library(RustLibrary {
            name: "coral-gfx".to_string(),
            functions: vec![],
        });

        let exprs = parse("(use rust.coral_gfx)").unwrap();
        let result = codegen.wrap_in_function(&exprs[0], "test_use_rust_coral_gfx_underscore");
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());

        let exprs = parse("(use rust.coral-gfx)").unwrap();
        let result = codegen.wrap_in_function(&exprs[0], "test_use_rust_coral_gfx_hyphen");
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[test]
    fn test_use_rust_module_accepts_prefixed_library_registration() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        codegen.register_rust_library(RustLibrary {
            name: "rust.coral-gfx".to_string(),
            functions: vec![],
        });

        let exprs = parse("(use rust.coral_gfx)").unwrap();
        let result =
            codegen.wrap_in_function(&exprs[0], "test_use_rust_coral_gfx_prefixed_library");
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[test]
    fn test_rust_ffi_symbol_name_is_dependency_scoped() {
        let symbol = CodeGen::rust_ffi_symbol_name("example-rust-lib", "add");
        assert_eq!(symbol, "clorus_example_rust_lib__add");
    }

    #[test]
    fn test_declare_rust_library_functions_supports_extended_numeric_types() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let lib = RustLibrary {
            name: "mathx".to_string(),
            functions: vec![
                RustFunction {
                    name: "f32_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "f32".to_string(),
                    }],
                    return_type: "f32".to_string(),
                },
                RustFunction {
                    name: "u32_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "u32".to_string(),
                    }],
                    return_type: "u32".to_string(),
                },
                RustFunction {
                    name: "u64_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "u64".to_string(),
                    }],
                    return_type: "u64".to_string(),
                },
                RustFunction {
                    name: "isize_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "isize".to_string(),
                    }],
                    return_type: "isize".to_string(),
                },
                RustFunction {
                    name: "usize_id".to_string(),
                    params: vec![RustParam {
                        name: "x".to_string(),
                        type_name: "usize".to_string(),
                    }],
                    return_type: "usize".to_string(),
                },
            ],
        };

        let result = codegen.declare_rust_library_functions(&lib);
        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
        for fn_name in ["f32_id", "u32_id", "u64_id", "isize_id", "usize_id"] {
            assert!(
                codegen
                    .module
                    .get_function(&format!("clorus_mathx__{}", fn_name))
                    .is_some()
            );
        }
    }

    #[test]
    fn test_compile_multiple_fn() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        // Parse multiple lambda functions
        let exprs1 = parse("(fn [x] (* x 2))").unwrap();
        let exprs2 = parse("(fn [y] (+ y 1))").unwrap();

        // Compile both
        let result1 = codegen.wrap_in_function(&exprs1[0], "test_fn1");
        assert!(result1.is_ok());

        let result2 = codegen.wrap_in_function(&exprs2[0], "test_fn2");
        assert!(result2.is_ok());

        // Verify two different lambda functions were created
        assert!(codegen.functions.contains_key("_lambda_0"));
        assert!(codegen.functions.contains_key("_lambda_1"));
    }

    #[test]
    fn test_namespace_alias_conflict_errors() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let exprs = parse(
            "(ns tests.alias-conflict
               (:require [clorus.set :as s]
                         [clorus.string :as s]))",
        )
        .unwrap();

        let result = codegen.wrap_in_function(&exprs[0], "test_ns_alias_conflict");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Namespace alias conflict"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_namespace_import_conflict_errors() {
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");

        let exprs = parse(
            "(ns tests.import-conflict
               (:require [clorus.set :rename {union shared}]
                         [clorus.string :rename {join shared}]))",
        )
        .unwrap();

        let result = codegen.wrap_in_function(&exprs[0], "test_ns_import_conflict");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Namespace import conflict"),
            "unexpected error: {}",
            err
        );
    }
}
