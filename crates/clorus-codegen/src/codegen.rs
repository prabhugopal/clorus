/// LLVM Code Generation for Clorus
use clorus_syntax::{Expr, Pattern, MapPatternKey};
use clorus_types::{FfiFunction, FfiType};
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{FloatValue, FunctionValue, PointerValue, GlobalValue, BasicMetadataValueEnum};
use inkwell::basic_block::BasicBlock;
use inkwell::{FloatPredicate, IntPredicate};
use inkwell::AddressSpace;
use std::collections::{HashMap, HashSet};

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
    /// Rust FFI libraries available for use
    rust_libraries: HashMap<String, RustLibrary>,
    /// Namespace context for symbol resolution
    namespace: NamespaceContext,
    /// Counter for generating unique lambda names
    lambda_counter: usize,
    /// Current loop context (for loop/recur)
    loop_context: Option<LoopContext<'ctx>>,
    /// Forward declared functions (from declare form) - allows mutual recursion
    forward_declarations: HashSet<String>,
}

impl<'ctx> CodeGen<'ctx> {
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
            rust_libraries: HashMap::new(),
            namespace: NamespaceContext::default_namespace(),
            lambda_counter: 0,
            loop_context: None,
            forward_declarations: HashSet::new(),
        };

        // Declare runtime functions
        codegen.declare_runtime_functions();

        // Declare core library functions (I/O, etc.)
        codegen.declare_core_functions();

        codegen
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

    /// Register a Rust FFI library so its functions can be used
    pub fn register_rust_library(&mut self, lib: RustLibrary) {
        self.rust_libraries.insert(lib.name.clone(), lib);
    }

    /// Declare all functions from a Rust FFI library based on metadata
    pub fn declare_rust_library_functions(&mut self, lib: &RustLibrary) -> Result<(), String> {
        let f64_type = self.context.f64_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        for func in &lib.functions {
            // Convert parameter types
            let mut param_types = Vec::new();
            for param in &func.params {
                let llvm_type = match param.type_name.as_str() {
                    "String" => i8_ptr_type.into(),
                    "f64" => f64_type.into(),
                    "i32" => self.context.i32_type().into(),
                    "bool" => self.context.bool_type().into(),
                    other => return Err(format!("Unsupported parameter type in FFI: {}", other)),
                };
                param_types.push(llvm_type);
            }

            // Convert return type
            let return_type = match func.return_type.as_str() {
                "String" => i8_ptr_type.fn_type(&param_types, false),
                "f64" => f64_type.fn_type(&param_types, false),
                "i32" => self.context.i32_type().fn_type(&param_types, false),
                "bool" => self.context.bool_type().fn_type(&param_types, false),
                "()" => self.context.void_type().fn_type(&param_types, false),
                other => return Err(format!("Unsupported return type in FFI: {}", other)),
            };

            // Declare function with clorus_ prefix
            let ffi_name = format!("clorus_{}", func.name);
            self.module.add_function(&ffi_name, return_type, None);
        }

        Ok(())
    }

    /// Compile a Rust FFI library function call generically based on metadata
    fn compile_rust_library_call(&mut self, lib: &RustLibrary, func_name: &str, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        // Convert kebab-case to snake_case for lookup (Clorus uses kebab, Rust uses snake)
        let rust_func_name = func_name.replace('-', "_");

        // Find the function metadata
        let func_meta = lib.functions.iter()
            .find(|f| f.name == rust_func_name)
            .ok_or_else(|| format!("Function {} not found in library {}", func_name, lib.name))?;

        // Check argument count
        if args.len() != func_meta.params.len() {
            return Err(format!(
                "{}/{} requires {} arguments, got {}",
                lib.name, func_name, func_meta.params.len(), args.len()
            ));
        }

        // Compile and convert arguments based on parameter types
        let mut ffi_args = Vec::new();
        for (arg_expr, param) in args.iter().zip(&func_meta.params) {
            let arg_val = self.compile_expr(arg_expr)?;

            let ffi_arg = match param.type_name.as_str() {
                "String" => {
                    // Extract C string from Value*
                    self.extract_cstring_from_value(arg_val).into()
                }
                "f64" => {
                    // Unbox number from Value*
                    self.unbox_number(arg_val).into()
                }
                "i32" => {
                    // Unbox number and convert to i32
                    let f64_val = self.unbox_number(arg_val);
                    self.builder.build_float_to_signed_int(
                        f64_val,
                        self.context.i32_type(),
                        "f64_to_i32"
                    ).unwrap().into()
                }
                "bool" => {
                    // Unbox number and convert to bool (non-zero = true)
                    let f64_val = self.unbox_number(arg_val);
                    let zero = self.context.f64_type().const_float(0.0);
                    self.builder.build_float_compare(
                        FloatPredicate::ONE,  // Ordered and Not Equal
                        f64_val,
                        zero,
                        "f64_to_bool"
                    ).unwrap().into()
                }
                other => return Err(format!("Unsupported parameter type: {}", other)),
            };

            ffi_args.push(ffi_arg);
        }

        // Call the FFI function
        let ffi_func_name = format!("clorus_{}", rust_func_name);
        let ffi_func = self.module.get_function(&ffi_func_name)
            .ok_or_else(|| format!("FFI function {} not found - did you (use rust.{})?", ffi_func_name, lib.name))?;

        let call_result = self.builder.build_call(
            ffi_func,
            &ffi_args,
            &format!("call_{}", rust_func_name)
        ).unwrap();

        // Convert return value based on return type
        match func_meta.return_type.as_str() {
            "String" => {
                let str_ptr = call_result.try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(self.box_string(str_ptr))
            }
            "f64" => {
                let f64_val = call_result.try_as_basic_value().left().unwrap().into_float_value();
                Ok(self.box_number(f64_val))
            }
            "i32" => {
                let i32_val = call_result.try_as_basic_value().left().unwrap().into_int_value();
                let f64_val = self.builder.build_signed_int_to_float(
                    i32_val,
                    self.context.f64_type(),
                    "i32_to_f64"
                ).unwrap();
                Ok(self.box_number(f64_val))
            }
            "bool" => {
                let bool_val = call_result.try_as_basic_value().left().unwrap().into_int_value();
                let f64_val = self.builder.build_unsigned_int_to_float(
                    bool_val,
                    self.context.f64_type(),
                    "bool_to_f64"
                ).unwrap();
                Ok(self.box_number(f64_val))
            }
            "()" => {
                // Void return - return nil (0.0)
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }
            other => Err(format!("Unsupported return type: {}", other)),
        }
    }

    /// Compile a Rust FFI function call using canonical FfiFunction type
    /// This is the modern, type-safe version that handles pointers correctly
    fn compile_ffi_function_call(&mut self, func: &FfiFunction, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        // Check argument count
        if args.len() != func.params.len() {
            return Err(format!(
                "{} requires {} arguments, got {}",
                func.name, func.params.len(), args.len()
            ));
        }

        // Compile and convert arguments based on canonical FFI types
        let mut ffi_args = Vec::new();
        for (arg_expr, param) in args.iter().zip(&func.params) {
            let arg_val = self.compile_expr(arg_expr)?;

            let ffi_arg = match &param.ty {
                FfiType::F64 => {
                    // Unbox number from Value*
                    self.unbox_number(arg_val).into()
                }
                FfiType::I64 => {
                    // Unbox number and convert to i64
                    let f64_val = self.unbox_number(arg_val);
                    self.builder.build_float_to_signed_int(
                        f64_val,
                        self.context.i64_type(),
                        "f64_to_i64"
                    ).unwrap().into()
                }
                FfiType::Bool => {
                    // Unbox number and convert to bool (non-zero = true)
                    let f64_val = self.unbox_number(arg_val);
                    let zero = self.context.f64_type().const_float(0.0);
                    self.builder.build_float_compare(
                        FloatPredicate::ONE,  // Ordered and Not Equal
                        f64_val,
                        zero,
                        "f64_to_bool"
                    ).unwrap().into()
                }
                FfiType::String => {
                    // Extract C string from Value*
                    self.extract_cstring_from_value(arg_val).into()
                }
                FfiType::OpaquePointer { .. } => {
                    // Extract raw pointer from Value*
                    // Pointers are stored as i8* in the Value's data field
                    self.extract_pointer_from_value(arg_val).into()
                }
                FfiType::Void => {
                    return Err("Void cannot be a parameter type".to_string());
                }
                _ => {
                    return Err(format!("Unsupported parameter type: {}", param.ty.display_name()));
                }
            };

            ffi_args.push(ffi_arg);
        }

        // Call the FFI function
        let ffi_func_name = format!("clorus_{}", func.name);
        let ffi_func = self.module.get_function(&ffi_func_name)
            .ok_or_else(|| format!("FFI function {} not found", ffi_func_name))?;

        let call_result = self.builder.build_call(
            ffi_func,
            &ffi_args,
            &format!("call_{}", func.name)
        ).unwrap();

        // Convert return value based on canonical FFI type
        match &func.return_type {
            FfiType::Void => {
                // Void return - return nil (0.0)
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }
            FfiType::F64 => {
                let f64_val = call_result.try_as_basic_value().left().unwrap().into_float_value();
                Ok(self.box_number(f64_val))
            }
            FfiType::I64 => {
                let i64_val = call_result.try_as_basic_value().left().unwrap().into_int_value();
                let f64_val = self.builder.build_signed_int_to_float(
                    i64_val,
                    self.context.f64_type(),
                    "i64_to_f64"
                ).unwrap();
                Ok(self.box_number(f64_val))
            }
            FfiType::Bool => {
                let bool_val = call_result.try_as_basic_value().left().unwrap().into_int_value();
                let f64_val = self.builder.build_unsigned_int_to_float(
                    bool_val,
                    self.context.f64_type(),
                    "bool_to_f64"
                ).unwrap();
                Ok(self.box_number(f64_val))
            }
            FfiType::String => {
                let str_ptr = call_result.try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(self.box_string(str_ptr))
            }
            FfiType::OpaquePointer { .. } => {
                // Box raw pointer into Value*
                let ptr = call_result.try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(self.box_pointer(ptr))
            }
            _ => {
                Err(format!("Unsupported return type: {}", func.return_type.display_name()))
            }
        }
    }

    /// Extract a raw pointer from a Value* (for opaque pointers)
    fn extract_pointer_from_value(&self, value_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        // Value* contains a pointer in its data field
        // For pointers, we stored them as i8* directly
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // Cast Value* to i8**
        let ptr_ptr = self.builder.build_pointer_cast(
            value_ptr,
            i8_ptr_type.ptr_type(AddressSpace::default()),
            "value_to_ptr_ptr"
        ).unwrap();

        // Load the i8*
        self.builder.build_load(
            i8_ptr_type,
            ptr_ptr,
            "load_ptr"
        ).unwrap().into_pointer_value()
    }

    /// Box a raw pointer into a Value* (for opaque pointers)
    fn box_pointer(&self, ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        // Allocate a Value* to hold the pointer
        let value_ptr = self.builder.build_malloc(
            self.context.i8_type().ptr_type(AddressSpace::default()),
            "alloc_value_ptr"
        ).unwrap();

        // Store the pointer
        self.builder.build_store(value_ptr, ptr).unwrap();

        value_ptr
    }

    /// Declare runtime library FFI functions
    fn declare_runtime_functions(&mut self) {
        // ===== Memory Management =====
        self.declare_void_fn("clorus_retain", 1);
        self.declare_void_fn("clorus_release", 1);

        // ===== Vector Functions =====
        self.declare_no_arg_value_fn("clorus_vector_empty");
        self.declare_value_fn("clorus_vector_conj", 2);
        self.declare_value_fn("clorus_conj", 2);  // Generic, dispatches by type
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
        self.declare_value_to_f64_fn("clorus_value_as_number");  // Handles both Long and Double
        self.declare_value_fn("clorus_value_string", 1);
        self.declare_value_fn("clorus_value_as_cstring", 1);
        self.declare_void_fn("clorus_free_cstring", 1);
        self.declare_value_fn("clorus_keyword", 1);
        self.declare_no_arg_value_fn("clorus_value_nil");
        self.declare_f64_to_value_fn("clorus_value_bool");
        self.declare_value_to_i32_fn("clorus_is_truthy");
        self.declare_bool_to_value_fn("clorus_value_boolean");

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
                self.context.i32_type().into()
            ],
            false
        );
        self.module.add_function("clorus_function_new", function_new_type, None);

        // clorus_function_call(func: *mut Value, args: *const *mut Value, arg_count: i32) -> *mut Value
        let function_call_type = i8_ptr_type.fn_type(
            &[
                i8_ptr_type.into(),
                i8_ptr_type.ptr_type(AddressSpace::default()).into(),
                self.context.i32_type().into()
            ],
            false
        );
        self.module.add_function("clorus_function_call", function_call_type, None);

        // ===== Exception Handling (C++ ABI) =====
        // __cxa_allocate_exception(size: size_t) -> *mut i8
        let allocate_exception_type = i8_ptr_type.fn_type(&[self.context.i64_type().into()], false);
        self.module.add_function("__cxa_allocate_exception", allocate_exception_type, None);

        // __cxa_throw(exception: *mut i8, tinfo: *mut i8, destructor: *mut i8) -> void
        let throw_type = self.context.void_type().fn_type(
            &[i8_ptr_type.into(), i8_ptr_type.into(), i8_ptr_type.into()],
            false
        );
        self.module.add_function("__cxa_throw", throw_type, None);

        self.declare_value_fn("__cxa_begin_catch", 1);
        self.declare_no_arg_void_fn("__cxa_end_catch");

        // __gxx_personality_v0 - C++ personality function (variadic)
        let personality_type = self.context.i32_type().fn_type(&[], true);
        self.module.add_function("__gxx_personality_v0", personality_type, None);

        // ===== Map Operations =====
        self.declare_value_fn("clorus_map_dissoc", 2);
        self.declare_value_fn("clorus_map_keys", 1);
        self.declare_value_fn("clorus_map_vals", 1);
        self.declare_value_fn("clorus_map_merge", 1);
        self.declare_value_fn("clorus_map_get_in", 2);
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
        self.declare_value_to_i32_fn("clorus_is_string");
        self.declare_value2_to_i32_fn("clorus_starts_with");
        self.declare_value2_to_i32_fn("clorus_ends_with");
        self.declare_value2_to_i32_fn("clorus_includes");
        self.declare_value2_to_i64_fn("clorus_compare_strings");

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
    }

    /// Declare rust.fs module functions
    fn declare_fs_functions(&mut self) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i32_type = self.context.i32_type();

        // clorus_fs_read(path: *const c_char) -> *mut c_char
        let fs_read_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_read", fs_read_type, None);

        // clorus_fs_write(path: *const c_char, content: *const c_char) -> i32
        let fs_write_type = i32_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_write", fs_write_type, None);

        // clorus_fs_append(path: *const c_char, content: *const c_char) -> i32
        let fs_append_type = i32_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_append", fs_append_type, None);

        // clorus_fs_exists(path: *const c_char) -> i32
        let fs_exists_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_exists", fs_exists_type, None);

        // clorus_fs_is_file(path: *const c_char) -> i32
        let fs_is_file_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_is_file", fs_is_file_type, None);

        // clorus_fs_is_dir(path: *const c_char) -> i32
        let fs_is_dir_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_is_dir", fs_is_dir_type, None);

        // clorus_fs_remove(path: *const c_char) -> i32
        let fs_remove_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_remove", fs_remove_type, None);

        // clorus_fs_copy(src: *const c_char, dst: *const c_char) -> i32
        let fs_copy_type = i32_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_copy", fs_copy_type, None);

        // clorus_fs_rename(old: *const c_char, new: *const c_char) -> i32
        let fs_rename_type = i32_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_rename", fs_rename_type, None);

        // clorus_fs_create_dir(path: *const c_char) -> i32
        let fs_create_dir_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_create_dir", fs_create_dir_type, None);

        // clorus_fs_create_dir_all(path: *const c_char) -> i32
        let fs_create_dir_all_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_create_dir_all", fs_create_dir_all_type, None);

        // clorus_fs_free_string(s: *mut c_char)
        let fs_free_string_type = self.context.void_type().fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_fs_free_string", fs_free_string_type, None);
    }

    /// Declare rust.path module functions (placeholder)
    fn declare_path_functions(&mut self) {
        // TODO: Add path functions
    }

    /// Declare example-rust-lib functions for testing FFI
    fn declare_rust_example_functions(&mut self) {
        let f64_type = self.context.f64_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // clorus_add(x: f64, y: f64) -> f64
        let add_type = f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
        self.module.add_function("clorus_add", add_type, None);

        // clorus_multiply(a: f64, b: f64) -> f64
        let multiply_type = f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
        self.module.add_function("clorus_multiply", multiply_type, None);

        // clorus_factorial(n: f64) -> f64
        let factorial_type = f64_type.fn_type(&[f64_type.into()], false);
        self.module.add_function("clorus_factorial", factorial_type, None);

        // clorus_greet(name: *const c_char) -> *mut c_char
        let greet_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_greet", greet_type, None);

        // clorus_to_upper(s: *const c_char) -> *mut c_char
        let to_upper_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_to_upper", to_upper_type, None);

        // clorus_string_length(s: *const c_char) -> f64
        let string_length_type = f64_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_string_length", string_length_type, None);
    }

    /// Declare async-demo functions
    fn declare_async_demo_functions(&mut self) {
        let f64_type = self.context.f64_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // clorus_hello_blocking() -> *mut c_char
        let hello_type = i8_ptr_type.fn_type(&[], false);
        self.module.add_function("clorus_hello_blocking", hello_type, None);

        // clorus_countdown_blocking(n: f64) -> f64
        let countdown_type = f64_type.fn_type(&[f64_type.into()], false);
        self.module.add_function("clorus_countdown_blocking", countdown_type, None);
    }

    /// Declare async-hello functions (futures-based)
    fn declare_async_hello_functions(&mut self) {
        let f64_type = self.context.f64_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // clorus_greet_blocking(name: *const c_char) -> *mut c_char
        let greet_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_greet_blocking", greet_type, None);

        // clorus_add_blocking(x: f64, y: f64) -> f64
        let add_type = f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
        self.module.add_function("clorus_add_blocking", add_type, None);
    }

    /// Declare egui-hello module functions
    fn declare_egui_hello_functions(&mut self) {
        let f64_type = self.context.f64_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // clorus_show_gui(message: *const c_char) -> f64
        let show_gui_type = f64_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_show_gui", show_gui_type, None);

        // clorus_get_gui_version() -> *mut c_char
        let get_version_type = i8_ptr_type.fn_type(&[], false);
        self.module.add_function("clorus_get_gui_version", get_version_type, None);
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
        let free_string_type = self.context.void_type().fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_free_string", free_string_type, None);

        // clorus_print(val: *mut Value) -> *mut Value
        let print_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_print", print_type, None);

        // clorus_println(val: *mut Value) -> *mut Value
        let println_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function("clorus_println", println_type, None);
    }

    /// Helper: Create a Value from an f64
    fn create_value_from_float(&self, float_val: FloatValue<'ctx>) -> inkwell::values::PointerValue<'ctx> {
        let value_double_fn = self.module.get_function("clorus_value_double")
            .expect("clorus_value_double not declared - runtime functions not initialized");
        let call_result = self.builder.build_call(
            value_double_fn,
            &[float_val.into()],
            "value_from_float"
        ).expect("Failed to build call to clorus_value_double");
        call_result.try_as_basic_value().left()
            .expect("clorus_value_double should return a value, got void")
            .into_pointer_value()
    }

    /// Helper: Extract f64 from a Value
    fn extract_float_from_value(&self, value_ptr: inkwell::values::PointerValue<'ctx>) -> FloatValue<'ctx> {
        let value_as_number_fn = self.module.get_function("clorus_value_as_number")
            .expect("clorus_value_as_number not declared - runtime functions not initialized");
        let call_result = self.builder.build_call(
            value_as_number_fn,
            &[value_ptr.into()],
            "number_from_value"
        ).expect("Failed to build call to clorus_value_as_number");
        call_result.try_as_basic_value().left()
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
        let value_string_fn = self.module.get_function("clorus_value_string")
            .expect("clorus_value_string not declared - runtime functions not initialized");
        let call_result = self.builder.build_call(
            value_string_fn,
            &[str_ptr.into()],
            "box_string"
        ).expect("Failed to build call to clorus_value_string");
        call_result.try_as_basic_value().left()
            .expect("clorus_value_string should return pointer, got void")
            .into_pointer_value()
    }

    /// Helper: Extract C string from Value*
    fn extract_cstring_from_value(&self, value_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        let value_as_cstring_fn = self.module.get_function("clorus_value_as_cstring")
            .expect("clorus_value_as_cstring not declared - runtime functions not initialized");
        let call_result = self.builder.build_call(
            value_as_cstring_fn,
            &[value_ptr.into()],
            "extract_cstring"
        ).expect("Failed to build call to clorus_value_as_cstring");
        call_result.try_as_basic_value().left()
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
        call_name: &str
    ) -> Result<PointerValue<'ctx>, String> {
        let func = self.module.get_function(fn_name)
            .ok_or_else(|| format!("{} not declared - did you forget to call declare_runtime_functions()?", fn_name))?;

        let call_result = self.builder.build_call(func, args, call_name)
            .map_err(|e| format!("Failed to build call to {}: {:?}", fn_name, e))?;

        call_result.try_as_basic_value().left()
            .ok_or_else(|| format!("{} returned void, expected value", fn_name))
            .map(|v| v.into_pointer_value())
    }

    /// Helper: Call a runtime function (non-Result version with expect for infallible calls)
    /// Use this when the function call is guaranteed to succeed (helper functions)
    fn call_runtime_fn_unchecked(
        &self,
        fn_name: &str,
        args: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
        call_name: &str
    ) -> PointerValue<'ctx> {
        self.call_runtime_fn(fn_name, args, call_name)
            .expect(&format!("Infallible call to {} failed - this is a compiler bug", fn_name))
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
        let fn_type = self.context.i64_type().fn_type(&[i8_ptr_type.into()], false);
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
        let fn_type = self.context.f64_type().fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns i32 (typically bool)
    fn declare_value_to_i32_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes 2 Value* args and returns i32 (typically bool)
    fn declare_value2_to_i32_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self.context.i32_type().fn_type(
            &[i8_ptr_type.into(), i8_ptr_type.into()],
            false
        );
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes 2 Value* args and returns i64
    fn declare_value2_to_i64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self.context.i64_type().fn_type(
            &[i8_ptr_type.into(), i8_ptr_type.into()],
            false
        );
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and 2 i64s, returns Value*
    fn declare_value_i64_i64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i8_ptr_type.fn_type(
            &[i8_ptr_type.into(), i64_type.into(), i64_type.into()],
            false
        );
        self.module.add_function(name, fn_type, None);
    }

    // ===== Code Generation Helpers for compile_core_call =====
    // These reduce boilerplate in the large match statement

    /// Helper: Compile a simple 1-argument core function call
    /// Pattern: (func-name arg) => clorus_func_name(compile(arg))
    fn compile_simple_1arg_call(
        &mut self,
        func_display_name: &str,
        runtime_fn_name: &str,
        args: &[Expr]
    ) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!("{} requires 1 argument", func_display_name));
        }
        let arg = self.compile_expr(&args[0])?;
        self.call_runtime_fn(runtime_fn_name, &[arg.into()], &format!("{}_call", func_display_name))
    }

    /// Helper: Compile a simple 2-argument core function call
    /// Pattern: (func-name arg1 arg2) => clorus_func_name(compile(arg1), compile(arg2))
    fn compile_simple_2arg_call(
        &mut self,
        func_display_name: &str,
        runtime_fn_name: &str,
        args: &[Expr]
    ) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!("{} requires 2 arguments", func_display_name));
        }
        let arg1 = self.compile_expr(&args[0])?;
        let arg2 = self.compile_expr(&args[1])?;
        self.call_runtime_fn(
            runtime_fn_name,
            &[arg1.into(), arg2.into()],
            &format!("{}_call", func_display_name)
        )
    }

    /// Helper: Compile a simple 3-argument core function call
    /// Pattern: (func-name arg1 arg2 arg3) => clorus_func_name(compile(arg1), compile(arg2), compile(arg3))
    fn compile_simple_3arg_call(
        &mut self,
        func_display_name: &str,
        runtime_fn_name: &str,
        args: &[Expr]
    ) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 3 {
            return Err(format!("{} requires 3 arguments", func_display_name));
        }
        let arg1 = self.compile_expr(&args[0])?;
        let arg2 = self.compile_expr(&args[1])?;
        let arg3 = self.compile_expr(&args[2])?;
        self.call_runtime_fn(
            runtime_fn_name,
            &[arg1.into(), arg2.into(), arg3.into()],
            &format!("{}_call", func_display_name)
        )
    }

    /// Helper: Compile a quoted expression (returns data, not evaluated)
    fn compile_quoted(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
        match expr {
            // Literals are returned as-is
            Expr::Long(n) => {
                let value_long_fn = self.module.get_function("clorus_value_long")
                    .ok_or("clorus_value_long not declared")?;
                let long_val = self.context.i64_type().const_int(*n as u64, false);
                let result = self.builder.build_call(value_long_fn, &[long_val.into()], "value_long")
                    .expect("Failed to build call to clorus_value_long");
                Ok(result.try_as_basic_value().left()
                    .expect("clorus_value_long should return pointer")
                    .into_pointer_value())
            }
            Expr::Double(n) => {
                let float_val = self.context.f64_type().const_float(*n);
                Ok(self.box_number(float_val))
            }
            Expr::String(s) => {
                let c_str = self.builder.build_global_string_ptr(s, "str")
                    .expect("Failed to build global string for quoted string");
                Ok(self.box_string(c_str.as_pointer_value()))
            }
            Expr::Keyword(k) => {
                let c_str = self.builder.build_global_string_ptr(k, "keyword")
                    .expect("Failed to build global string for keyword");
                let keyword_fn = self.module.get_function("clorus_keyword")
                    .ok_or("clorus_keyword not declared")?;
                let call_result = self.builder.build_call(
                    keyword_fn,
                    &[c_str.as_pointer_value().into()],
                    "keyword"
                ).expect("Failed to build call to clorus_keyword");
                Ok(call_result.try_as_basic_value().left()
                    .expect("clorus_keyword should return pointer")
                    .into_pointer_value())
            }
            Expr::Bool(b) => {
                let float_val = self.context.f64_type().const_float(if *b { 1.0 } else { 0.0 });
                Ok(self.box_number(float_val))
            }
            Expr::Nil => {
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            // Symbols become strings for now (TODO: add Symbol value type)
            Expr::Symbol(s) => {
                let c_str = self.builder.build_global_string_ptr(s, "quoted_symbol")
                    .expect("Failed to build global string for quoted symbol");
                Ok(self.box_string(c_str.as_pointer_value()))
            }

            // Vectors: recursively quote each element
            Expr::Vector(elements) => {
                let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let empty_vec_call = self.builder.build_call(
                    vec_empty_fn,
                    &[],
                    "quoted_vec_empty"
                ).expect("Failed to build call to clorus_vector_empty");
                let mut vec_val = empty_vec_call.try_as_basic_value()
                    .left()
                    .expect("clorus_vector_empty should return pointer")
                    .into_pointer_value();

                let vec_conj_fn = self.module.get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                for (i, elem) in elements.iter().enumerate() {
                    // Recursively quote each element
                    let elem_val = self.compile_quoted(elem)?;

                    let conj_call = self.builder.build_call(
                        vec_conj_fn,
                        &[vec_val.into(), elem_val.into()],
                        &format!("quoted_vec_conj_{}", i)
                    ).expect("Failed to build call to clorus_vector_conj");

                    vec_val = conj_call.try_as_basic_value()
                        .left()
                        .expect("clorus_vector_conj should return pointer")
                        .into_pointer_value();
                }

                Ok(vec_val)
            }

            // Lists are converted to vectors (since we don't have List literals at runtime yet)
            Expr::List(elements) => {
                let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let empty_vec_call = self.builder.build_call(
                    vec_empty_fn,
                    &[],
                    "quoted_list_empty"
                ).unwrap();
                let mut vec_val = empty_vec_call.try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let vec_conj_fn = self.module.get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                for (i, elem) in elements.iter().enumerate() {
                    let elem_val = self.compile_quoted(elem)?;

                    let conj_call = self.builder.build_call(
                        vec_conj_fn,
                        &[vec_val.into(), elem_val.into()],
                        &format!("quoted_list_conj_{}", i)
                    ).unwrap();

                    vec_val = conj_call.try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(vec_val)
            }

            // Maps: recursively quote keys and values
            Expr::Map(entries) => {
                let map_empty_fn = self.module.get_function("clorus_map_empty")
                    .ok_or("clorus_map_empty not declared")?;
                let empty_map_call = self.builder.build_call(
                    map_empty_fn,
                    &[],
                    "quoted_map_empty"
                ).unwrap();
                let mut map_val = empty_map_call.try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let map_assoc_fn = self.module.get_function("clorus_map_assoc")
                    .ok_or("clorus_map_assoc not declared")?;

                for (i, (key_expr, val_expr)) in entries.iter().enumerate() {
                    let key_val = self.compile_quoted(key_expr)?;
                    let val_val = self.compile_quoted(val_expr)?;

                    let assoc_call = self.builder.build_call(
                        map_assoc_fn,
                        &[map_val.into(), key_val.into(), val_val.into()],
                        &format!("quoted_map_assoc_{}", i)
                    ).unwrap();

                    map_val = assoc_call.try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(map_val)
            }

            // Sets are treated like vectors when quoted
            Expr::Set(elements) => {
                let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let empty_vec_call = self.builder.build_call(
                    vec_empty_fn,
                    &[],
                    "quoted_set_empty"
                ).unwrap();
                let mut vec_val = empty_vec_call.try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let vec_conj_fn = self.module.get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                for (i, elem) in elements.iter().enumerate() {
                    // Recursively quote each element
                    let elem_val = self.compile_quoted(elem)?;

                    let conj_call = self.builder.build_call(
                        vec_conj_fn,
                        &[vec_val.into(), elem_val.into()],
                        &format!("quoted_set_conj_{}", i)
                    ).unwrap();

                    vec_val = conj_call.try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(vec_val)
            }

            // Nested quotes: '(quote x) => just return (quote x) as data
            Expr::Quote { expr: inner } => {
                // Create a vector with symbol "quote" and the inner expression
                let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let vec_val = self.builder.build_call(vec_empty_fn, &[], "nested_quote_vec")
                    .unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                let vec_conj_fn = self.module.get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                // Add "quote" symbol as string
                let quote_str = self.builder.build_global_string_ptr("quote", "quote_symbol").unwrap();
                let quote_val = self.box_string(quote_str.as_pointer_value());
                let vec_with_quote = self.builder.build_call(
                    vec_conj_fn,
                    &[vec_val.into(), quote_val.into()],
                    "nested_quote_with_sym"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                // Add the inner expression
                let inner_val = self.compile_quoted(inner)?;
                let final_vec = self.builder.build_call(
                    vec_conj_fn,
                    &[vec_with_quote.into(), inner_val.into()],
                    "nested_quote_final"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                Ok(final_vec)
            }

            // Other expression types that shouldn't appear in quotes
            _ => Err(format!("Cannot quote expression: {:?}", expr)),
        }
    }

    /// Compile syntax-quoted expression (backtick `)
    /// Like quote but evaluates unquoted (~) sections
    fn compile_syntax_quoted(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
        match expr {
            // Unquote: evaluate the expression
            Expr::Unquote { expr: inner } => {
                self.compile_expr(inner)
            }

            // Unquote-splicing: evaluate and expect a sequence
            // For now, treat same as unquote (splicing happens in parent context)
            Expr::UnquoteSplicing { expr: inner } => {
                self.compile_expr(inner)
            }

            // Literals: same as quote
            Expr::Long(n) => {
                let float_val = self.context.f64_type().const_float(*n as f64);
                Ok(self.box_number(float_val))
            }

            Expr::Double(n) => {
                let float_val = self.context.f64_type().const_float(*n);
                Ok(self.box_number(float_val))
            }

            Expr::String(s) => {
                let c_str = self.builder.build_global_string_ptr(s, "str").unwrap();
                Ok(self.box_string(c_str.as_pointer_value()))
            }

            Expr::Symbol(s) => {
                // TODO: Namespace resolution - for now, just return as string
                let c_str = self.builder.build_global_string_ptr(s, "syntax_quoted_symbol").unwrap();
                Ok(self.box_string(c_str.as_pointer_value()))
            }

            Expr::Keyword(k) => {
                let keyword_fn = self.module.get_function("clorus_keyword")
                    .ok_or("clorus_keyword not declared")?;
                let c_str = self.builder.build_global_string_ptr(k, "keyword").unwrap();
                let call_result = self.builder.build_call(
                    keyword_fn,
                    &[c_str.as_pointer_value().into()],
                    "keyword_value"
                ).unwrap();
                Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            Expr::Bool(b) => {
                let bool_fn = self.module.get_function("clorus_value_bool")
                    .ok_or("clorus_value_bool not declared")?;
                let bool_val = if *b { 1.0 } else { 0.0 };
                let float_val = self.context.f64_type().const_float(bool_val);
                let call_result = self.builder.build_call(
                    bool_fn,
                    &[float_val.into()],
                    "bool_value"
                ).unwrap();
                Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            Expr::Nil => {
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            // Collections: recursively process, checking for unquote-splicing
            Expr::Vector(elements) => {
                self.compile_syntax_quoted_sequence(elements, true)
            }

            Expr::List(elements) => {
                // Lists become vectors (like regular quote)
                self.compile_syntax_quoted_sequence(elements, true)
            }

            Expr::Set(elements) => {
                // Sets become vectors (like regular quote)
                self.compile_syntax_quoted_sequence(elements, true)
            }

            Expr::Map(entries) => {
                let map_empty_fn = self.module.get_function("clorus_map_empty")
                    .ok_or("clorus_map_empty not declared")?;
                let mut map_val = self.builder.build_call(map_empty_fn, &[], "sq_map_empty")
                    .unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                let map_assoc_fn = self.module.get_function("clorus_map_assoc")
                    .ok_or("clorus_map_assoc not declared")?;

                for (i, (key_expr, val_expr)) in entries.iter().enumerate() {
                    let key_val = self.compile_syntax_quoted(key_expr)?;
                    let val_val = self.compile_syntax_quoted(val_expr)?;

                    let assoc_call = self.builder.build_call(
                        map_assoc_fn,
                        &[map_val.into(), key_val.into(), val_val.into()],
                        &format!("sq_map_assoc_{}", i)
                    ).unwrap();

                    map_val = assoc_call.try_as_basic_value().left().unwrap().into_pointer_value();
                }

                Ok(map_val)
            }

            // Nested syntax-quote: treat as data
            Expr::SyntaxQuote { expr: inner } => {
                let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let vec_val = self.builder.build_call(vec_empty_fn, &[], "nested_sq_vec")
                    .unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                let vec_conj_fn = self.module.get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                // Add "syntax-quote" symbol
                let sq_str = self.builder.build_global_string_ptr("syntax-quote", "sq_symbol").unwrap();
                let sq_val = self.box_string(sq_str.as_pointer_value());
                let vec_with_sq = self.builder.build_call(
                    vec_conj_fn,
                    &[vec_val.into(), sq_val.into()],
                    "nested_sq_with_sym"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                // Add the inner expression
                let inner_val = self.compile_syntax_quoted(inner)?;
                let final_vec = self.builder.build_call(
                    vec_conj_fn,
                    &[vec_with_sq.into(), inner_val.into()],
                    "nested_sq_final"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                Ok(final_vec)
            }

            // Calls: treat as a list (function call as data)
            Expr::Call { func, args } => {
                // Create a list with the function name and arguments
                let mut all_elems = vec![Expr::Symbol(func.clone())];
                all_elems.extend(args.clone());
                self.compile_syntax_quoted_sequence(&all_elems, true)
            }

            _ => Err(format!("Cannot syntax-quote expression: {:?}", expr)),
        }
    }

    /// Helper: compile a sequence for syntax-quote, handling unquote-splicing
    fn compile_syntax_quoted_sequence(&mut self, elements: &[Expr], _as_vector: bool) -> Result<PointerValue<'ctx>, String> {
        let vec_empty_fn = self.module.get_function("clorus_vector_empty")
            .ok_or("clorus_vector_empty not declared")?;
        let mut vec_val = self.builder.build_call(vec_empty_fn, &[], "sq_seq_empty")
            .unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

        let vec_conj_fn = self.module.get_function("clorus_vector_conj")
            .ok_or("clorus_vector_conj not declared")?;

        for (i, elem) in elements.iter().enumerate() {
            match elem {
                Expr::UnquoteSplicing { expr: inner } => {
                    // Evaluate the inner expression and splice its elements
                    // For now, just evaluate and add as-is (proper splicing needs iteration)
                    let spliced_val = self.compile_expr(inner)?;

                    // TODO: Iterate over spliced_val and add each element
                    // For MVP, just add the whole collection
                    let conj_call = self.builder.build_call(
                        vec_conj_fn,
                        &[vec_val.into(), spliced_val.into()],
                        &format!("sq_seq_splice_{}", i)
                    ).unwrap();

                    vec_val = conj_call.try_as_basic_value().left().unwrap().into_pointer_value();
                }
                _ => {
                    // Regular element: recursively syntax-quote
                    let elem_val = self.compile_syntax_quoted(elem)?;

                    let conj_call = self.builder.build_call(
                        vec_conj_fn,
                        &[vec_val.into(), elem_val.into()],
                        &format!("sq_seq_conj_{}", i)
                    ).unwrap();

                    vec_val = conj_call.try_as_basic_value().left().unwrap().into_pointer_value();
                }
            }
        }

        Ok(vec_val)
    }

    /// Create a stack allocation for a variable (stores Value*)
    fn create_entry_block_alloca(&self, name: &str) -> PointerValue<'ctx> {
        let builder = self.context.create_builder();

        // Find the entry block of the current function
        let function = self.builder.get_insert_block()
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

    /// Find free variables in an expression
    /// Returns a list of variable names that are referenced but not locally bound
    fn find_free_variables(&self, expr: &Expr) -> Vec<String> {
        use std::collections::HashSet;
        let mut free_vars = Vec::new();
        let mut seen = HashSet::new();
        self.collect_free_vars(expr, &mut free_vars, &mut seen, &HashSet::new());
        free_vars
    }

    /// Helper to recursively collect free variables
    fn collect_free_vars(
        &self,
        expr: &Expr,
        free_vars: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
        bound: &std::collections::HashSet<String>,
    ) {
        use std::collections::HashSet;

        match expr {
            Expr::Symbol(name) => {
                // If it's not bound locally and not already collected, it's free
                if !bound.contains(name) && !seen.contains(name) && self.variables.contains_key(name) {
                    free_vars.push(name.clone());
                    seen.insert(name.clone());
                }
            }
            Expr::Let { bindings, body } => {
                // Variables bound in let are not free within the body
                let mut new_bound = bound.clone();
                for (pattern, value_expr) in bindings {
                    // Value expression can reference outer variables
                    self.collect_free_vars(value_expr, free_vars, seen, bound);
                    // Extract variable names from pattern (simplified - only handles Symbol patterns)
                    if let clorus_syntax::ast::Pattern::Symbol(name) = pattern {
                        new_bound.insert(name.clone());
                    }
                }
                self.collect_free_vars(body, free_vars, seen, &new_bound);
            }
            Expr::Call { func, args } => {
                // func is a String (function name) - check if it's a free variable
                if !bound.contains(func) && !seen.contains(func) && self.variables.contains_key(func) {
                    free_vars.push(func.clone());
                    seen.insert(func.clone());
                }
                // Check arguments for free variables
                for arg in args {
                    self.collect_free_vars(arg, free_vars, seen, bound);
                }
            }
            Expr::If { condition, then_branch, else_branch } => {
                self.collect_free_vars(condition, free_vars, seen, bound);
                self.collect_free_vars(then_branch, free_vars, seen, bound);
                self.collect_free_vars(else_branch, free_vars, seen, bound);
            }
            Expr::Vector(elements) => {
                for elem in elements {
                    self.collect_free_vars(elem, free_vars, seen, bound);
                }
            }
            Expr::List(elements) => {
                for elem in elements {
                    self.collect_free_vars(elem, free_vars, seen, bound);
                }
            }
            Expr::Map(pairs) => {
                for (key, val) in pairs {
                    self.collect_free_vars(key, free_vars, seen, bound);
                    self.collect_free_vars(val, free_vars, seen, bound);
                }
            }
            // Literals and other non-recursive cases
            _ => {}
        }
    }

    /// Set up exception handling personality function for a function
    fn set_personality_function(&self, function: FunctionValue<'ctx>) {
        let personality_fn = self.module.get_function("__gxx_personality_v0")
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

            Pattern::Vector { elements, rest, as_binding: _ } => {
                // Get clorus_vector_nth function
                let nth_fn = self.module.get_function("clorus_vector_nth")
                    .ok_or("clorus_vector_nth not declared")?;

                // Extract each element
                for (i, elem_pattern) in elements.iter().enumerate() {
                    let index = self.context.i64_type().const_int(i as u64, false);
                    let elem_val = self.builder.build_call(
                        nth_fn,
                        &[value.into(), index.into()],
                        &format!("vec_elem_{}", i)
                    ).unwrap()
                    .try_as_basic_value().left().unwrap().into_pointer_value();

                    // Recursively destructure
                    let nested = self.destructure_pattern(elem_pattern, elem_val)?;
                    bindings.extend(nested);
                }

                // Handle rest parameter
                if let Some(rest_name) = rest {
                    // Get clorus_vector_rest function (takes vector and start index)
                    let rest_fn = self.module.get_function("clorus_vector_rest")
                        .ok_or("clorus_vector_rest not declared")?;

                    let start_index = self.context.i64_type().const_int(elements.len() as u64, false);
                    let rest_val = self.builder.build_call(
                        rest_fn,
                        &[value.into(), start_index.into()],
                        "vec_rest"
                    ).unwrap()
                    .try_as_basic_value().left().unwrap().into_pointer_value();

                    // Bind rest parameter
                    let alloca = self.create_entry_block_alloca(rest_name);
                    self.builder.build_store(alloca, rest_val).unwrap();
                    self.variables.insert(rest_name.clone(), alloca);
                    bindings.push((rest_name.clone(), alloca));
                }
            }

            Pattern::Map { bindings: pattern_bindings, defaults: _ } => {
                // Get map_get and keyword functions
                let get_fn = self.module.get_function("clorus_map_get")
                    .ok_or("clorus_map_get not declared")?;
                let keyword_fn = self.module.get_function("clorus_keyword")
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
                    let keyword_c_str = self.builder.build_global_string_ptr(key_str, "key").unwrap();
                    let keyword_val = self.builder.build_call(
                        keyword_fn,
                        &[keyword_c_str.as_pointer_value().into()],
                        "keyword"
                    ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                    // Get value from map
                    let map_val = self.builder.build_call(
                        get_fn,
                        &[value.into(), keyword_val.into()],
                        &format!("map_get_{}", key_str)
                    ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

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
            Pattern::Vector { elements, rest, as_binding: _ } => {
                let mut names = Vec::new();
                for elem in elements {
                    names.extend(Self::collect_pattern_names(elem));
                }
                if let Some(rest_name) = rest {
                    names.push(rest_name.clone());
                }
                names
            }
            Pattern::Map { bindings, defaults: _ } => {
                let mut names = Vec::new();
                for (_, value_pattern) in bindings {
                    names.extend(Self::collect_pattern_names(value_pattern));
                }
                names
            }
        }
    }

    /// Compile an expression to a Value*
    /// All expressions now return boxed values
    pub fn compile_expr(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
        match expr {
            Expr::Long(n) => {
                // Box long integer into Value* using clorus_value_long
                let value_long_fn = self.module.get_function("clorus_value_long")
                    .ok_or("clorus_value_long not declared")?;
                let long_val = self.context.i64_type().const_int(*n as u64, false);
                let result = self.builder.build_call(
                    value_long_fn,
                    &[long_val.into()],
                    "value_long"
                ).unwrap();
                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            Expr::Double(n) => {
                // Box double into Value* using clorus_value_double
                let float_val = self.context.f64_type().const_float(*n);
                Ok(self.box_number(float_val))
            }

            Expr::Symbol(name) => {
                // Variable reference - check globals first, then locals, then functions
                // Variables now store Value* instead of f64
                if let Some(global) = self.globals.get(name) {
                    // Load Value* from global variable
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let val = self.builder.build_load(
                        value_ptr_type,
                        global.as_pointer_value(),
                        name
                    ).unwrap();
                    Ok(val.into_pointer_value())
                } else if let Some(ptr) = self.variables.get(name) {
                    // Load Value* from local variable
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let val = self.builder.build_load(
                        value_ptr_type,
                        *ptr,
                        name
                    ).unwrap();
                    Ok(val.into_pointer_value())
                } else {
                    // Try to find function - check both unmangled and mangled names
                    let function = self.functions.get(name).or_else(|| {
                        // Try mangled name for current namespace
                        let mangled_name = if self.namespace.current == "user" {
                            name.clone()
                        } else {
                            format!("clorus_{}_{}",
                                self.namespace.current.replace('.', "_"),
                                name.replace('-', "_"))
                        };
                        self.functions.get(&mangled_name)
                    });

                    if let Some(function) = function {
                        // Function reference - wrap in function value
                        // Call clorus_function_new with the function pointer and arity
                        let func_new_fn = self.module.get_function("clorus_function_new")
                            .ok_or("clorus_function_new not declared")?;

                        // Get function pointer (cast to *const u8)
                        let func_ptr = function.as_global_value().as_pointer_value();
                        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                        let func_ptr_cast = self.builder.build_pointer_cast(
                            func_ptr,
                            i8_ptr_type,
                            "func_ptr_cast"
                        ).unwrap();

                        // Get arity from function type
                        // For top-level defn functions, arity = param_count
                        // (they don't have an environment parameter)
                        let fn_type = function.get_type();
                        let arity = fn_type.count_param_types();
                        let arity_val = self.context.i32_type().const_int(arity as u64, false);

                        // Empty environment (nullptr)
                        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                        let env_ptr = value_ptr_type.const_null();

                        // env_size = 0
                        let env_size = self.context.i32_type().const_int(0, false);

                        // Call clorus_function_new
                        let func_val = self.builder.build_call(
                            func_new_fn,
                            &[func_ptr_cast.into(), arity_val.into(), env_ptr.into(), env_size.into()],
                            "func_value"
                        ).unwrap();

                        Ok(func_val.try_as_basic_value().left().unwrap().into_pointer_value())
                    } else {
                        Err(format!("Undefined variable: {}", name))
                    }
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
                let keyword_fn = self.module.get_function("clorus_keyword")
                    .ok_or("clorus_keyword not declared")?;

                let call_result = self.builder.build_call(
                    keyword_fn,
                    &[c_str.as_pointer_value().into()],
                    "keyword"
                ).unwrap();

                Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            Expr::Nil => {
                // Nil represented as boxed nil value
                let nil_fn = self.module.get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let call_result = self.builder.build_call(
                    nil_fn,
                    &[],
                    "nil_value"
                ).unwrap();
                Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            Expr::Bool(b) => {
                // Bool represented as boxed bool value
                let bool_fn = self.module.get_function("clorus_value_bool")
                    .ok_or("clorus_value_bool not declared")?;
                let bool_val = if *b { 1.0 } else { 0.0 };
                let float_val = self.context.f64_type().const_float(bool_val);
                let call_result = self.builder.build_call(
                    bool_fn,
                    &[float_val.into()],
                    "bool_value"
                ).unwrap();
                Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            Expr::Vector(elements) => {
                // Vector literals: [1 2 3]
                // Create empty vector
                let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let empty_vec_call = self.builder.build_call(
                    vec_empty_fn,
                    &[],
                    "vec_empty"
                ).unwrap();
                let mut vec_val = empty_vec_call.try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add each element using clorus_vector_conj
                let vec_conj_fn = self.module.get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                for (i, elem) in elements.iter().enumerate() {
                    // Compile the element
                    let elem_val = self.compile_expr(elem)?;

                    // Call clorus_vector_conj(vec, elem) -> new_vec
                    let conj_call = self.builder.build_call(
                        vec_conj_fn,
                        &[vec_val.into(), elem_val.into()],
                        &format!("vec_conj_{}", i)
                    ).unwrap();

                    // Update vec_val to the new vector
                    vec_val = conj_call.try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(vec_val)
            }

            Expr::Map(entries) => {
                // Map literals: {:key1 val1 :key2 val2}
                // Create empty map
                let map_empty_fn = self.module.get_function("clorus_map_empty")
                    .ok_or("clorus_map_empty not declared")?;
                let empty_map_call = self.builder.build_call(
                    map_empty_fn,
                    &[],
                    "map_empty"
                ).unwrap();
                let mut map_val = empty_map_call.try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add each key-value pair using clorus_map_assoc
                let map_assoc_fn = self.module.get_function("clorus_map_assoc")
                    .ok_or("clorus_map_assoc not declared")?;

                for (i, (key_expr, val_expr)) in entries.iter().enumerate() {
                    // Compile the key
                    let key_val = self.compile_expr(key_expr)?;

                    // Compile the value
                    let val_val = self.compile_expr(val_expr)?;

                    // Call clorus_map_assoc(map, key, val) -> new_map
                    let assoc_call = self.builder.build_call(
                        map_assoc_fn,
                        &[map_val.into(), key_val.into(), val_val.into()],
                        &format!("map_assoc_{}", i)
                    ).unwrap();

                    // Update map_val to the new map
                    map_val = assoc_call.try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(map_val)
            }

            Expr::Set(elements) => {
                // Set literals: #{1 2 3}
                // Create empty set
                let set_empty_fn = self.module.get_function("clorus_set_empty")
                    .ok_or("clorus_set_empty not declared")?;
                let empty_set_call = self.builder.build_call(
                    set_empty_fn,
                    &[],
                    "set_empty"
                ).unwrap();
                let mut set_val = empty_set_call.try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add each element using clorus_set_conj
                let set_conj_fn = self.module.get_function("clorus_set_conj")
                    .ok_or("clorus_set_conj not declared")?;

                for (i, elem) in elements.iter().enumerate() {
                    // Compile the element
                    let elem_val = self.compile_expr(elem)?;

                    // Call clorus_set_conj(set, elem) -> new_set
                    let conj_call = self.builder.build_call(
                        set_conj_fn,
                        &[set_val.into(), elem_val.into()],
                        &format!("set_conj_{}", i)
                    ).unwrap();

                    // Update set_val to the new set
                    set_val = conj_call.try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(set_val)
            }

            Expr::Let { bindings, body} => {
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
                let retain_fn = self.module.get_function("clorus_retain")
                    .ok_or("clorus_retain not declared")?;
                self.builder.build_call(
                    retain_fn,
                    &[result.into()],
                    "retain_result"
                ).unwrap();

                // Release local variables before exiting scope
                let release_fn = self.module.get_function("clorus_release")
                    .ok_or("clorus_release not declared")?;

                for (var_name, var_ptr) in &local_vars {
                    // Load the Value* from the variable
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let val = self.builder.build_load(
                        value_ptr_type,
                        *var_ptr,
                        &format!("{}_cleanup", var_name)
                    ).unwrap().into_pointer_value();

                    // Call clorus_release(val)
                    self.builder.build_call(
                        release_fn,
                        &[val.into()],
                        &format!("release_{}", var_name)
                    ).unwrap();
                }

                // Restore previous scope (let creates local scope)
                self.variables = saved_vars;

                Ok(result)
            }

            Expr::Def { name, value, metadata: _ } => {
                // Compile the value (returns Value*)
                let val = self.compile_expr(value)?;

                // Create or update global variable (now stores Value*)
                let global = if let Some(existing_global) = self.globals.get(name) {
                    // If global already exists, just update it
                    *existing_global
                } else {
                    // Create new global variable of type Value* (i8*)
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let global = self.module.add_global(value_ptr_type, Some(AddressSpace::default()), name);
                    // Initialize with null pointer
                    global.set_initializer(&value_ptr_type.const_null());
                    self.globals.insert(name.clone(), global);
                    global
                };

                // Store the Value* to the global
                self.builder.build_store(global.as_pointer_value(), val).unwrap();

                // Return the value
                Ok(val)
            }

            Expr::Defn { name, params, rest_param, body } => {
                // Generate mangled name based on current namespace
                // math/add -> clorus_math_add
                let mangled_name = if self.namespace.current == "user" {
                    // In default namespace, use simple name
                    name.clone()
                } else {
                    format!("clorus_{}_{}",
                        self.namespace.current.replace('.', "_"),
                        name.replace('-', "_"))
                };

                // If this function was forward-declared, remove the placeholder first
                // LLVM doesn't allow replacing a function with a different signature
                if self.forward_declarations.contains(name) {
                    if let Some(old_func) = self.module.get_function(&mangled_name) {
                        // Remove from function table
                        self.functions.remove(&mangled_name);
                        // Delete the LLVM function declaration
                        unsafe { old_func.delete(); }
                    }
                }

                // Create function type: all parameters are Value*, return is Value*
                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                // For variadic functions, we need all fixed params
                let param_types: Vec<_> = params.iter()
                    .map(|_| value_ptr_type.into())
                    .collect();

                // If there's a rest parameter, the function is variadic
                let is_variadic = rest_param.is_some();
                let fn_type = value_ptr_type.fn_type(&param_types, is_variadic);
                let function = self.module.add_function(&mangled_name, fn_type, None);

                // Add function to table BEFORE compiling body (for recursion)
                self.functions.insert(mangled_name.clone(), function);

                // Save current state
                let saved_vars = self.variables.clone();
                let saved_block = self.builder.get_insert_block();

                // Create entry block for the function
                let entry = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry);

                // Clear variables for function scope
                self.variables.clear();

                // Bind fixed parameters to allocas (parameters are now Value*)
                // Support destructuring in function parameters
                for (i, param_pattern) in params.iter().enumerate() {
                    let param_val = function.get_nth_param(i as u32)
                        .unwrap()
                        .into_pointer_value();

                    // Destructure parameter pattern
                    self.destructure_pattern(param_pattern, param_val)?;
                }

                // Handle rest parameter if present
                if let Some(rest_name) = rest_param {
                    // Create an empty vector to hold rest arguments
                    let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                        .ok_or("clorus_vector_empty not declared")?;
                    let rest_vec = self.builder.build_call(
                        vec_empty_fn,
                        &[],
                        "rest_vec_empty"
                    ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                    // For now, create the rest vector binding (empty for MVP)
                    // TODO: Implement proper vararg collection using va_list
                    // LLVM varargs are complex and require platform-specific handling

                    let rest_alloca = self.create_entry_block_alloca(rest_name);
                    self.builder.build_store(rest_alloca, rest_vec).unwrap();
                    self.variables.insert(rest_name.clone(), rest_alloca);
                }

                // Compile function body (returns Value*)
                let result = self.compile_expr(body)?;
                self.builder.build_return(Some(&result)).unwrap();

                // Restore previous state
                self.variables = saved_vars;
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }

                // Return 0.0 boxed as Value* (defn returns nil-like value)
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            Expr::DefnMulti { name, arities } => {
                // Multi-arity functions: generate one function per arity
                // Each arity gets a mangled name: foo_arity_0, foo_arity_1, foo_arity_2

                let base_name = if self.namespace.current == "user" {
                    name.clone()
                } else {
                    format!("clorus_{}_{}",
                        self.namespace.current.replace('.', "_"),
                        name.replace('-', "_"))
                };

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                // Generate a function for each arity
                for (arity_index, arity) in arities.iter().enumerate() {
                    let arity_name = format!("{}_arity_{}", base_name, arity.params.len());

                    // Create parameter types for this arity
                    let param_types: Vec<_> = arity.params.iter()
                        .map(|_| value_ptr_type.into())
                        .collect();

                    let is_variadic = arity.rest_param.is_some();
                    let fn_type = value_ptr_type.fn_type(&param_types, is_variadic);
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
                        let param_val = function.get_nth_param(i as u32)
                            .unwrap()
                            .into_pointer_value();

                        self.destructure_pattern(param_pattern, param_val)?;
                    }

                    // Handle rest parameter if present
                    if let Some(rest_name) = &arity.rest_param {
                        let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                            .ok_or("clorus_vector_empty not declared")?;
                        let rest_vec = self.builder.build_call(
                            vec_empty_fn,
                            &[],
                            "rest_vec_empty"
                        ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                        let rest_alloca = self.create_entry_block_alloca(rest_name);
                        self.builder.build_store(rest_alloca, rest_vec).unwrap();
                        self.variables.insert(rest_name.clone(), rest_alloca);
                    }

                    // Compile body
                    let result = self.compile_expr(&arity.body)?;
                    self.builder.build_return(Some(&result)).unwrap();

                    // Restore state
                    self.variables = saved_vars.clone();
                    if let Some(block) = saved_block {
                        self.builder.position_at_end(block);
                    }

                    // Register this arity function
                    // For now, we'll register the last one under the base name
                    // TODO: Implement proper multi-arity dispatch
                    if arity_index == arities.len() - 1 {
                        self.functions.insert(base_name.clone(), function);
                    }
                }

                // Return nil
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            Expr::Fn { params, rest_param, body } => {
                // Generate unique lambda name
                let lambda_name = format!("_lambda_{}", self.lambda_counter);
                self.lambda_counter += 1;

                // Find free variables (captured from outer scope)
                // Build bound set: function parameters + rest param
                let mut bound = HashSet::new();
                for param in params {
                    match param {
                        Pattern::Symbol(name) => { bound.insert(name.clone()); }
                        _ => {} // TODO: Handle destructuring patterns
                    }
                }
                if let Some(rest_name) = rest_param {
                    bound.insert(rest_name.clone());
                }

                // Find free variables in body
                let mut free_vars = Vec::new();
                let mut seen = HashSet::new();
                self.collect_free_vars(body, &mut free_vars, &mut seen, &bound);

                // Create function type: all parameters are Value*, plus environment parameter as LAST arg
                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let mut param_types: Vec<_> = params.iter()
                    .map(|_| value_ptr_type.into())
                    .collect();

                // Add environment parameter as the LAST parameter
                param_types.push(value_ptr_type.into());

                // If there's a rest parameter, the function is variadic
                let is_variadic = rest_param.is_some();
                let fn_type = value_ptr_type.fn_type(&param_types, is_variadic);
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
                    let param_val = function.get_nth_param(i as u32)
                        .unwrap()
                        .into_pointer_value();

                    // Destructure parameter pattern
                    self.destructure_pattern(param_pattern, param_val)?;
                }

                // Handle rest parameter if present
                if let Some(rest_name) = rest_param {
                    // Create an empty vector to hold rest arguments
                    let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                        .ok_or("clorus_vector_empty not declared")?;
                    let rest_vec = self.builder.build_call(
                        vec_empty_fn,
                        &[],
                        "rest_vec_empty"
                    ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                    // For now, create the rest vector binding (empty for MVP)
                    // TODO: Implement proper vararg collection using va_list

                    let rest_alloca = self.create_entry_block_alloca(rest_name);
                    self.builder.build_store(rest_alloca, rest_vec).unwrap();
                    self.variables.insert(rest_name.clone(), rest_alloca);
                }

                // Unpack captured variables from environment (LAST parameter)
                if !free_vars.is_empty() {
                    let env_param = function.get_nth_param(params.len() as u32)
                        .unwrap()
                        .into_pointer_value();

                    for (i, var_name) in free_vars.iter().enumerate() {
                        // Each captured variable is a *mut Value in the environment
                        // Environment is passed as a *mut Value pointing to the first element
                        // We need to offset by i to get to the i-th captured variable

                        let offset = self.context.i64_type().const_int(i as u64, false);
                        let var_ptr = unsafe {
                            self.builder.build_gep(
                                value_ptr_type,
                                env_param,
                                &[offset],
                                &format!("env_{}", var_name)
                            ).unwrap()
                        };

                        // Load the value from environment
                        let var_value = self.builder.build_load(
                            value_ptr_type,
                            var_ptr,
                            &format!("load_{}", var_name)
                        ).unwrap().into_pointer_value();

                        // Store in local variable
                        let alloca = self.create_entry_block_alloca(var_name);
                        self.builder.build_store(alloca, var_value).unwrap();
                        self.variables.insert(var_name.clone(), alloca);
                    }
                }

                // Compile function body (returns Value*)
                let result = self.compile_expr(body)?;
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
                    let env_array = self.builder.build_alloca(env_array_type, "env_array").unwrap();

                    for (i, var_name) in free_vars.iter().enumerate() {
                        // Get the value from current scope
                        let var_value = if let Some(var_ptr) = saved_vars.get(var_name) {
                            self.builder.build_load(value_ptr_type, *var_ptr, var_name)
                                .unwrap()
                                .into_pointer_value()
                        } else if let Some(global) = self.globals.get(var_name) {
                            self.builder.build_load(value_ptr_type, global.as_pointer_value(), var_name)
                                .unwrap()
                                .into_pointer_value()
                        } else {
                            return Err(format!("Captured variable not found: {}", var_name));
                        };

                        // Store in environment array
                        let elem_ptr = unsafe {
                            self.builder.build_gep(
                                env_array_type,
                                env_array,
                                &[
                                    self.context.i32_type().const_zero(),
                                    self.context.i32_type().const_int(i as u64, false)
                                ],
                                &format!("env_elem_{}", i)
                            ).unwrap()
                        };
                        self.builder.build_store(elem_ptr, var_value).unwrap();
                    }

                    // Cast to *const *mut Value
                    self.builder.build_pointer_cast(
                        env_array,
                        value_ptr_type.ptr_type(AddressSpace::default()),
                        "env_ptr"
                    ).unwrap()
                } else {
                    // No captures - pass null
                    value_ptr_type.ptr_type(AddressSpace::default()).const_null()
                };

                // Create a proper Function value using clorus_function_new
                // clorus_function_new(func_ptr: *const u8, arity: i32, env: *const *mut Value, env_size: u32) -> *mut Value
                let function_new_fn = self.module.get_function("clorus_function_new")
                    .ok_or("clorus_function_new not declared")?;

                let fn_ptr = function.as_global_value().as_pointer_value();
                let arity = self.context.i32_type().const_int(params.len() as u64, false);
                let env_size = self.context.i32_type().const_int(free_vars.len() as u64, false);

                let func_val = self.builder.build_call(
                    function_new_fn,
                    &[fn_ptr.into(), arity.into(), env_ptr.into(), env_size.into()],
                    "new_function"
                ).unwrap();

                Ok(func_val.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            Expr::FnMulti { arities } => {
                // Multi-arity anonymous function
                // Generate one function per arity with unique lambda names

                let base_lambda_name = format!("_lambda_{}", self.lambda_counter);
                self.lambda_counter += 1;

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let mut last_function = None;

                // Generate a function for each arity
                for arity in arities.iter() {
                    let arity_name = format!("{}_arity_{}", base_lambda_name, arity.params.len());

                    // Create parameter types for this arity
                    let param_types: Vec<_> = arity.params.iter()
                        .map(|_| value_ptr_type.into())
                        .collect();

                    let is_variadic = arity.rest_param.is_some();
                    let fn_type = value_ptr_type.fn_type(&param_types, is_variadic);
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
                        let param_val = function.get_nth_param(i as u32)
                            .unwrap()
                            .into_pointer_value();

                        self.destructure_pattern(param_pattern, param_val)?;
                    }

                    // Handle rest parameter if present
                    if let Some(rest_name) = &arity.rest_param {
                        let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                            .ok_or("clorus_vector_empty not declared")?;
                        let rest_vec = self.builder.build_call(
                            vec_empty_fn,
                            &[],
                            "rest_vec_empty"
                        ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                        let rest_alloca = self.create_entry_block_alloca(rest_name);
                        self.builder.build_store(rest_alloca, rest_vec).unwrap();
                        self.variables.insert(rest_name.clone(), rest_alloca);
                    }

                    // Compile body
                    let result = self.compile_expr(&arity.body)?;
                    self.builder.build_return(Some(&result)).unwrap();

                    // Restore state
                    self.variables = saved_vars.clone();
                    if let Some(block) = saved_block {
                        self.builder.position_at_end(block);
                    }

                    // Keep track of the last function for returning
                    last_function = Some(function);
                }

                // Return the last arity function as a boxed pointer (temporary solution)
                // TODO: Implement proper multi-arity dispatch for anonymous functions
                if let Some(function) = last_function {
                    let fn_ptr = function.as_global_value().as_pointer_value();
                    let fn_ptr_as_int = self.builder.build_ptr_to_int(
                        fn_ptr,
                        self.context.i64_type(),
                        "fn_ptr_to_int"
                    ).unwrap();
                    let fn_ptr_as_float = self.builder.build_unsigned_int_to_float(
                        fn_ptr_as_int,
                        self.context.f64_type(),
                        "fn_ptr_to_float"
                    ).unwrap();

                    Ok(self.box_number(fn_ptr_as_float))
                } else {
                    Err("Multi-arity fn must have at least one arity".to_string())
                }
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
                let tx_begin_fn = self.module.get_function("clorus_tx_begin")
                    .ok_or("clorus_tx_begin not declared")?;
                let tx_commit_fn = self.module.get_function("clorus_tx_commit")
                    .ok_or("clorus_tx_commit not declared")?;
                let tx_abort_fn = self.module.get_function("clorus_tx_abort")
                    .ok_or("clorus_tx_abort not declared")?;

                // Begin transaction
                self.builder.build_call(
                    tx_begin_fn,
                    &[],
                    "dosync_begin"
                ).unwrap();

                // Compile all expressions in the transaction
                let mut result = None;
                for expr in exprs {
                    result = Some(self.compile_expr(expr)?);
                }
                let body_result = result.unwrap();

                // Attempt to commit
                // TODO: Add retry logic in future version
                // For now, just commit once
                let commit_success = self.builder.build_call(
                    tx_commit_fn,
                    &[],
                    "dosync_commit"
                ).unwrap().try_as_basic_value().left().unwrap().into_int_value();

                // Check if commit succeeded
                // If failed, abort transaction
                // TODO: Add retry loop
                let zero = self.context.bool_type().const_zero();
                let commit_failed = self.builder.build_int_compare(
                    IntPredicate::EQ,
                    commit_success,
                    zero,
                    "commit_failed"
                ).unwrap();

                let current_fn = self.builder.get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("Dosync must be inside a function")?;

                let abort_block = self.context.append_basic_block(current_fn, "dosync_abort");
                let continue_block = self.context.append_basic_block(current_fn, "dosync_continue");

                self.builder.build_conditional_branch(
                    commit_failed,
                    abort_block,
                    continue_block
                ).unwrap();

                // Abort block
                self.builder.position_at_end(abort_block);
                self.builder.build_call(
                    tx_abort_fn,
                    &[],
                    "dosync_abort_call"
                ).unwrap();
                // TODO: In future, add retry logic here
                // For now, just continue after abort
                self.builder.build_unconditional_branch(continue_block).unwrap();

                // Continue block
                self.builder.position_at_end(continue_block);

                Ok(body_result)
            }

            Expr::Loop { bindings, body } => {
                // Compile loop with tail-call optimization using LLVM basic blocks
                // Strategy:
                // 1. Create loop_start, loop_body, loop_end basic blocks
                // 2. Initialize binding variables
                // 3. Branch to loop_start
                // 4. In loop_body, compile the body
                // 5. If body contains recur, it will update bindings and branch back to loop_start
                // 6. Otherwise, branch to loop_end with result

                let current_fn = self.builder.get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("Loop must be inside a function")?;

                // Create basic blocks
                let loop_start = self.context.append_basic_block(current_fn, "loop_start");
                let loop_end = self.context.append_basic_block(current_fn, "loop_end");

                // Save current loop context (for nested loops)
                let saved_loop_context = self.loop_context.clone();

                // Collect binding names from patterns
                let binding_names: Vec<String> = bindings.iter()
                    .flat_map(|(pattern, _)| Self::collect_pattern_names(pattern))
                    .collect();

                // Set new loop context
                self.loop_context = Some(LoopContext {
                    loop_start,
                    loop_end,
                    binding_names: binding_names.clone(),
                });

                // Initialize binding variables with destructuring
                for (pattern, init_val) in bindings {
                    let val = self.compile_expr(init_val)?;
                    self.destructure_pattern(pattern, val)?;
                }

                // Branch to loop start
                self.builder.build_unconditional_branch(loop_start).unwrap();

                // Position builder at loop_start
                self.builder.position_at_end(loop_start);

                // Compile body
                let result = self.compile_expr(body)?;

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
                    let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let phi = self.builder.build_phi(value_ptr_type, "loop_result").unwrap();
                    phi.add_incoming(&[(&result, result_block)]);
                    Ok(phi.as_basic_value().into_pointer_value())
                } else {
                    // Loop never exits (infinite loop or always returns/throws)
                    // Return a dummy value - this code is unreachable
                    let nil_fn = self.module.get_function("clorus_value_nil")
                        .ok_or("clorus_value_nil not declared")?;
                    let dummy_call = self.builder.build_call(nil_fn, &[], "unreachable_loop_result").unwrap();
                    Ok(dummy_call.try_as_basic_value().left().unwrap().into_pointer_value())
                }
            }

            Expr::Recur { args } => {
                // Jump back to the nearest loop with new values
                let loop_ctx = self.loop_context.clone()
                    .ok_or("recur can only be used inside a loop")?;

                // Check that arg count matches binding count
                if args.len() != loop_ctx.binding_names.len() {
                    return Err(format!(
                        "recur argument count mismatch: expected {}, got {}",
                        loop_ctx.binding_names.len(),
                        args.len()
                    ));
                }

                // Evaluate all new values first
                let mut new_values = Vec::new();
                for arg in args {
                    new_values.push(self.compile_expr(arg)?);
                }

                // Update all binding variables
                for (i, name) in loop_ctx.binding_names.iter().enumerate() {
                    let alloca = self.variables.get(name)
                        .ok_or(format!("Loop binding '{}' not found", name))?;
                    self.builder.build_store(*alloca, new_values[i]).unwrap();
                }

                // Branch back to loop start
                self.builder.build_unconditional_branch(loop_ctx.loop_start).unwrap();

                // Position builder after the branch (unreachable code, but required)
                let current_fn = self.builder.get_insert_block()
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

            Expr::Defrecord { name, fields } => {
                // Generate constructor function: ->RecordName
                // Example: (defrecord Person [name age]) creates ->Person function
                let constructor_name = format!("->{}", name);

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                // Create parameter types (one Value* for each field)
                let param_types: Vec<_> = fields.iter()
                    .map(|_| value_ptr_type.into())
                    .collect();

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

                // Create empty map to hold record fields
                let map_empty_fn = self.module.get_function("clorus_map_empty")
                    .ok_or("clorus_map_empty not declared")?;
                let mut map_val = self.builder.build_call(
                    map_empty_fn,
                    &[],
                    "record_map"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                // Get map_assoc and keyword functions
                let map_assoc_fn = self.module.get_function("clorus_map_assoc")
                    .ok_or("clorus_map_assoc not declared")?;
                let keyword_fn = self.module.get_function("clorus_keyword")
                    .ok_or("clorus_keyword not declared")?;

                // For each field, add keyword->value pair to map
                for (i, field_name) in fields.iter().enumerate() {
                    // Get parameter value for this field
                    let param_val = function.get_nth_param(i as u32)
                        .unwrap()
                        .into_pointer_value();

                    // Create keyword for field name
                    let field_c_str = self.builder.build_global_string_ptr(field_name, "field_name").unwrap();
                    let keyword_val = self.builder.build_call(
                        keyword_fn,
                        &[field_c_str.as_pointer_value().into()],
                        &format!("field_keyword_{}", i)
                    ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                    // Add keyword->value pair to map
                    map_val = self.builder.build_call(
                        map_assoc_fn,
                        &[map_val.into(), keyword_val.into(), param_val.into()],
                        &format!("record_map_{}", i)
                    ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();
                }

                // Return the constructed record (map)
                self.builder.build_return(Some(&map_val)).unwrap();

                // Restore previous state
                self.variables = saved_vars;
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }

                // defrecord returns nil (like defn)
                let nil_fn = self.module.get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self.builder.build_call(nil_fn, &[], "defrecord_nil").unwrap()
                    .try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(nil_val)
            }

            Expr::Defprotocol { .. } => {
                // Protocols are compile-time only metadata
                // They define method signatures but don't generate runtime code
                // The actual implementation is in extend-type
                let nil_fn = self.module.get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self.builder.build_call(nil_fn, &[], "defprotocol_nil").unwrap()
                    .try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(nil_val)
            }

            Expr::ExtendType { type_name, protocol_name, methods } => {
                // Generate functions for each protocol method implementation
                // Function names are mangled: TypeName_ProtocolName_methodName
                // Example: Point_Drawable_draw

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                for method in methods {
                    // Generate mangled function name
                    let func_name = format!("{}_{}_{}", type_name, protocol_name, method.name);

                    // Create parameter types (all Value*)
                    let param_types: Vec<_> = method.params.iter()
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
                        let param_val = function.get_nth_param(i as u32)
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
                }

                // extend-type returns nil
                let nil_fn = self.module.get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self.builder.build_call(nil_fn, &[], "extend_type_nil").unwrap()
                    .try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(nil_val)
            }

            Expr::Defmulti { name, dispatch_fn } => {
                // For MVP: compile and store dispatch function
                // Generate a function for the dispatch logic
                let dispatch_fn_name = format!("{}_dispatch", name);

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                // Dispatch function takes one parameter (the arguments)
                // For simplicity, takes a single Value* parameter
                let param_types = vec![value_ptr_type.into()];
                let fn_type = value_ptr_type.fn_type(&param_types, false);
                let function = self.module.add_function(&dispatch_fn_name, fn_type, None);

                // Add to function table
                self.functions.insert(dispatch_fn_name.clone(), function);

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

                // defmulti returns nil
                let nil_fn = self.module.get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self.builder.build_call(nil_fn, &[], "defmulti_nil").unwrap()
                    .try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(nil_val)
            }

            Expr::Defmethod { name, dispatch_value, params, body } => {
                // Generate function name based on multimethod name and dispatch value
                // Convert dispatch value to string for function name
                let dispatch_str = match dispatch_value.as_ref() {
                    Expr::Keyword(k) => k.clone(),
                    Expr::Symbol(s) => s.clone(),
                    Expr::Long(n) => format!("{}", *n),
                    Expr::Double(n) => format!("{}", *n as i64),
                    Expr::String(s) => s.replace("-", "_").replace(" ", "_"),
                    _ => return Err("Dispatch value must be a keyword, symbol, number, or string".to_string()),
                };

                let func_name = format!("{}_{}", name, dispatch_str);

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

                // Create parameter types (all Value*)
                let param_types: Vec<_> = params.iter()
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
                for (i, param_pattern) in params.iter().enumerate() {
                    let param_val = function.get_nth_param(i as u32)
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
                let nil_fn = self.module.get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                let nil_val = self.builder.build_call(nil_fn, &[], "defmethod_nil").unwrap()
                    .try_as_basic_value().left().unwrap().into_pointer_value();
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

            Expr::Try { body, catch_clauses, finally_block } => {
                // Simplified exception handling for MVP
                // For now, execute body normally. If it throws, the exception will propagate
                // Full invoke/landingpad support requires more complex integration

                // Save current variable scope
                let saved_vars = self.variables.clone();

                // Execute the body
                let body_result = self.compile_expr(body)?;

                // Execute finally block if present (always runs in this simplified version)
                if let Some(ref finally_expr) = finally_block {
                    self.compile_expr(finally_expr)?;
                }

                // Restore variables
                self.variables = saved_vars;

                // Return result of body
                // TODO: Implement proper invoke/landingpad when ready
                // For now, exceptions will propagate normally via __cxa_throw
                Ok(body_result)
            }

            Expr::Throw { expr } => {
                // Proper exception throwing using C++ exception ABI
                let exception_value = self.compile_expr(expr)?;

                // Get __cxa_allocate_exception function
                let allocate_fn = self.module.get_function("__cxa_allocate_exception")
                    .ok_or("__cxa_allocate_exception not declared")?;

                // Allocate exception storage (size of pointer = 8 bytes on 64-bit)
                let exception_size = self.context.i64_type().const_int(8, false);
                let exception_storage = self.builder.build_call(
                    allocate_fn,
                    &[exception_size.into()],
                    "exception_storage"
                ).unwrap()
                .try_as_basic_value().left().unwrap().into_pointer_value();

                // Store the exception Value* into the allocated storage
                self.builder.build_store(exception_storage, exception_value).unwrap();

                // Get __cxa_throw function
                let throw_fn = self.module.get_function("__cxa_throw")
                    .ok_or("__cxa_throw not declared")?;

                // Type info (null for now - catch-all)
                let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let null_typeinfo = i8_ptr_type.const_null();

                // Destructor (null for now - no cleanup)
                let null_destructor = i8_ptr_type.const_null();

                // Call __cxa_throw (this function never returns - it unwinds the stack)
                self.builder.build_call(
                    throw_fn,
                    &[exception_storage.into(), null_typeinfo.into(), null_destructor.into()],
                    "throw"
                ).unwrap();

                // After throw, we need an unreachable instruction
                self.builder.build_unreachable().unwrap();

                // Return a dummy value (never actually reached)
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            Expr::Deref { expr } => {
                // Deref: @my-atom => (deref my-atom)
                let atom_val = self.compile_expr(expr)?;

                let deref_fn = self.module.get_function("clorus_deref")
                    .ok_or("clorus_deref not declared")?;

                let result = self.builder.build_call(
                    deref_fn,
                    &[atom_val.into()],
                    "deref_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            Expr::Call { func, args } => {
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
                        let resolved_namespace = self.namespace.aliases
                            .get(namespace_or_alias)
                            .map(|s| s.as_str())
                            .unwrap_or(namespace_or_alias);

                        // Try to find it as a Clorus function first
                        // Generate mangled name: math/add -> clorus_math_add
                        let mangled_name = format!("clorus_{}_{}",
                            resolved_namespace.replace('.', "_"),
                            func_name.replace('-', "_"));

                        if let Some(function) = self.functions.get(&mangled_name) {
                            // Found a Clorus function - compile arguments and call it
                            let function = function.clone();
                            let mut arg_values = Vec::new();
                            for arg in args {
                                arg_values.push(self.compile_expr(arg)?.into());
                            }

                            let call_result = self.builder
                                .build_call(function, &arg_values, "call")
                                .unwrap();

                            return Ok(call_result.try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_pointer_value());
                        }

                        // Not a Clorus function - try Rust FFI libraries
                        let rust_lib = self.rust_libraries.get(namespace_or_alias).cloned()
                            .or_else(|| {
                                // Check if this is an alias for a rust.* module
                                self.namespace.aliases.get(namespace_or_alias)
                                    .and_then(|resolved| {
                                        if resolved.starts_with("rust.") {
                                            // Try full resolved name with underscores: rust.egui_hello
                                            self.rust_libraries.get(resolved).cloned()
                                                .or_else(|| {
                                                    // Try with hyphens for Clojure-style: rust.egui-hello
                                                    let hyphenated = resolved.replace('_', "-");
                                                    self.rust_libraries.get(&hyphenated).cloned()
                                                })
                                                .or_else(|| {
                                                    // Try without prefix for backward compatibility
                                                    let lib_name = resolved.strip_prefix("rust.").unwrap();
                                                    self.rust_libraries.get(lib_name).cloned()
                                                })
                                        } else {
                                            None
                                        }
                                    })
                            });

                        if let Some(lib) = rust_lib {
                            return self.compile_rust_library_call(&lib, func_name, args);
                        }
                    }

                    // Fall back to hardcoded libraries for backwards compatibility
                    if func.starts_with("fs/") {
                        return self.compile_fs_call(func, args);
                    }
                    if func.starts_with("example/") {
                        return self.compile_rust_example_call(func, args);
                    }
                    if func.starts_with("async-demo/") {
                        return self.compile_async_demo_call(func, args);
                    }

                    return Err(format!("Unknown function: {}. Did you (use rust.{})?", func, func.split('/').next().unwrap()));
                }

                // Check if this is a clorus.core function call
                let core_functions = [
                    "slurp", "spit", "get", "nth", "first", "rest", "last", "count", "empty?",
                    "reduce", "apply", "conj", "disj", "contains?", "concat", "assoc", "dissoc",
                    "atom", "reset!", "swap!",
                    // Agent operations
                    "agent", "send", "await", "await-for", "agent-error",
                    // Channel operations (CSP)
                    "chan", ">!!", "<!!", "close!", "alts!!",
                    // Go blocks
                    "go",
                    // String operations
                    "str", "subs", "split", "join",
                    "upper-case", "lower-case",
                    "trim", "trim-left", "trim-right",
                    "replace", "replace-first",
                    "string?", "starts-with?", "ends-with?", "includes?",
                    // I/O operations
                    "print", "println"
                ];
                if core_functions.contains(&func.as_str()) {
                    return self.compile_core_call(func, args);
                }

                // Check if function name is a local variable (parameter or let-binding)
                // This allows first-class functions: (fn [f] (f 42))
                if let Some(var_ptr) = self.variables.get(func).cloned() {
                    // It's a variable - use dynamic dispatch via clorus_function_call
                    let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let func_val = self.builder.build_load(i8_ptr_type, var_ptr, "load_func_var")
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
                        let array_alloca = self.builder.build_alloca(array_type, "args_array").unwrap();

                        for (i, arg_val) in arg_values.iter().enumerate() {
                            let elem_ptr = unsafe {
                                self.builder.build_gep(
                                    array_type,
                                    array_alloca,
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_int(i as u64, false)
                                    ],
                                    &format!("arg_{}_ptr", i)
                                ).unwrap()
                            };
                            self.builder.build_store(elem_ptr, *arg_val).unwrap();
                        }

                        self.builder.build_pointer_cast(
                            array_alloca,
                            i8_ptr_type.ptr_type(AddressSpace::default()),
                            "args_array_cast"
                        ).unwrap()
                    } else {
                        i8_ptr_type.ptr_type(AddressSpace::default()).const_null()
                    };

                    // Call clorus_function_call
                    let function_call_fn = self.module.get_function("clorus_function_call")
                        .ok_or("clorus_function_call not declared")?;

                    let arg_count_val = self.context.i32_type().const_int(arg_count as u64, false);

                    let call_result = self.builder.build_call(
                        function_call_fn,
                        &[func_val.into(), args_array_ptr.into(), arg_count_val.into()],
                        "dynamic_call"
                    ).unwrap();

                    return Ok(call_result.try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value());
                }

                // Check if this is a referred symbol (imported via :refer)
                // If so, resolve to the full qualified name
                let function_to_lookup = if let Some(source_namespace) = self.namespace.imports.get(func) {
                    // This is a referred symbol - create the mangled name
                    format!("clorus_{}_{}",
                        source_namespace.replace('.', "_"),
                        func.replace('-', "_"))
                } else if self.namespace.current != "user" {
                    // Try current namespace's function (for local calls)
                    format!("clorus_{}_{}",
                        self.namespace.current.replace('.', "_"),
                        func.replace('-', "_"))
                } else {
                    // Default namespace - use simple name
                    func.to_string()
                };

                // Look up the function (clone to avoid borrow issues)
                // Try the resolved name first, then fall back to simple name
                // For multi-arity functions, try to find the specific arity variant
                let function = {
                    // Compile arguments first to know the count
                    let mut arg_values = Vec::new();
                    for arg in args {
                        arg_values.push(self.compile_expr(arg)?.into());
                    }

                    // Try to find multi-arity variant: function_arity_N
                    let arity_variant = format!("{}_arity_{}", function_to_lookup, arg_values.len());

                    if let Some(func) = self.functions.get(&arity_variant) {
                        // Found multi-arity variant
                        (func.clone(), arg_values)
                    } else {
                        // Try regular function lookup
                        let func = self.functions.get(&function_to_lookup)
                            .or_else(|| self.functions.get(func))
                            .ok_or_else(|| format!("Undefined function: {} (tried {} and multi-arity variants)", func, function_to_lookup))?
                            .clone();
                        (func, arg_values)
                    }
                };

                let (function, arg_values) = function;

                // Build call (function now returns Value*)
                let call_result = self.builder
                    .build_call(function, &arg_values, "call")
                    .unwrap();

                Ok(call_result.try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::If { condition, then_branch, else_branch } => {
                // Compile condition (returns Value*)
                let cond_val_ptr = self.compile_expr(condition)?;

                // Use clorus_is_truthy to check if condition is truthy
                let is_truthy_fn = self.module.get_function("clorus_is_truthy")
                    .ok_or("clorus_is_truthy not declared")?;
                let truthy_result = self.builder.build_call(
                    is_truthy_fn,
                    &[cond_val_ptr.into()],
                    "is_truthy"
                ).unwrap();
                let truthy_i32 = truthy_result.try_as_basic_value().left().unwrap().into_int_value();

                // Convert i32 to i1 (bool) for conditional branch
                let cond_bool = self.builder.build_int_compare(
                    inkwell::IntPredicate::NE,
                    truthy_i32,
                    self.context.i32_type().const_int(0, false),
                    "ifcond"
                ).unwrap();

                // Get current function
                let function = self.builder.get_insert_block()
                    .and_then(|block| block.get_parent())
                    .expect("No parent function");

                // Create basic blocks
                let then_bb = self.context.append_basic_block(function, "then");
                let else_bb = self.context.append_basic_block(function, "else");
                let merge_bb = self.context.append_basic_block(function, "ifcont");

                // Build conditional branch
                self.builder.build_conditional_branch(cond_bool, then_bb, else_bb).unwrap();

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
                    let nil_fn = self.module.get_function("clorus_value_nil")
                        .ok_or("clorus_value_nil not declared")?;
                    let dummy_call = self.builder.build_call(nil_fn, &[], "unreachable_value").unwrap();
                    Ok(dummy_call.try_as_basic_value().left().unwrap().into_pointer_value())
                }
            }

            Expr::Ns { name, requires, rust_imports } => {
                // Update namespace context
                self.namespace.current = name.clone();

                // Process requires - update namespace context
                for req_spec in requires {
                    if let Some(ref alias) = req_spec.alias {
                        self.namespace.aliases.insert(alias.clone(), req_spec.module.clone());
                    }
                    // Import specific symbols
                    for symbol in &req_spec.refer {
                        self.namespace.imports.insert(symbol.clone(), req_spec.module.clone());
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
                    if let Some(lib) = self.rust_libraries.get(&rust_import.library).cloned() {
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
                        self.namespace.aliases.insert(alias.clone(), req_spec.module.clone());
                    }
                    // Import specific symbols
                    for symbol in &req_spec.refer {
                        self.namespace.imports.insert(symbol.clone(), req_spec.module.clone());
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
                        format!("clorus_{}_{}",
                            self.namespace.current.replace('.', "_"),
                            name.replace('-', "_"))
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

                    if let Some(lib) = self.rust_libraries.get(lib_name).cloned() {
                        // Auto-declare all functions from this library
                        self.declare_rust_library_functions(&lib)?;
                    } else {
                        // Fall back to hardcoded libraries (for backwards compatibility)
                        match module.as_str() {
                            "rust.fs" => self.declare_fs_functions(),
                            "rust.path" => self.declare_path_functions(),
                            "rust.example" => self.declare_rust_example_functions(),
                            "rust.async-demo" => self.declare_async_demo_functions(),
                            _ => return Err(format!("Unknown rust module: {}. Did you add it to rust-dependencies in Clorus.toml?", module)),
                        }
                    }
                } else if module == "clorus.core" {
                    self.declare_core_functions();
                } else {
                    return Err(format!("Unknown module: {}", module));
                }

                // use statements don't return a meaningful value, return boxed 0
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }

            Expr::List(list) if !list.is_empty() => {
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
                    Err("First element of list must be a symbol".to_string())
                }
            }

            Expr::List(_) => Err("Empty list cannot be compiled".to_string()),

            _ => Err(format!("Cannot compile expression: {:?}", expr)),
        }
    }

    fn compile_add(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.is_empty() {
            // (+ ) => 0 (as Long)
            let value_long_fn = self.module.get_function("clorus_value_long")
                .ok_or("clorus_value_long not declared")?;
            let zero = self.context.i64_type().const_zero();
            let result = self.builder.build_call(value_long_fn, &[zero.into()], "zero").unwrap();
            return Ok(result.try_as_basic_value().left().unwrap().into_pointer_value());
        }

        // Get clorus_add function
        let add_fn = self.module.get_function("clorus_add")
            .ok_or("clorus_add not declared")?;

        // Compile first argument
        let mut result = self.compile_expr(&args[0])?;

        // Add remaining arguments using clorus_add
        for arg in &args[1..] {
            let val_ptr = self.compile_expr(arg)?;
            let call_result = self.builder.build_call(
                add_fn,
                &[result.into(), val_ptr.into()],
                "add"
            ).unwrap();
            result = call_result.try_as_basic_value().left().unwrap().into_pointer_value();
        }

        Ok(result)
    }

    fn compile_sub(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.is_empty() {
            return Err("- requires at least one argument".to_string());
        }

        let sub_fn = self.module.get_function("clorus_sub")
            .ok_or("clorus_sub not declared")?;

        let mut result = self.compile_expr(&args[0])?;

        if args.len() == 1 {
            // Unary negation: (- 5) => (0 - 5)
            let value_long_fn = self.module.get_function("clorus_value_long")
                .ok_or("clorus_value_long not declared")?;
            let zero = self.context.i64_type().const_zero();
            let zero_val = self.builder.build_call(value_long_fn, &[zero.into()], "zero").unwrap()
                .try_as_basic_value().left().unwrap().into_pointer_value();

            let call_result = self.builder.build_call(
                sub_fn,
                &[zero_val.into(), result.into()],
                "neg"
            ).unwrap();
            return Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value());
        }

        // Subtract remaining arguments
        for arg in &args[1..] {
            let val_ptr = self.compile_expr(arg)?;
            let call_result = self.builder.build_call(
                sub_fn,
                &[result.into(), val_ptr.into()],
                "sub"
            ).unwrap();
            result = call_result.try_as_basic_value().left().unwrap().into_pointer_value();
        }

        Ok(result)
    }

    fn compile_mul(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.is_empty() {
            let value_long_fn = self.module.get_function("clorus_value_long")
                .ok_or("clorus_value_long not declared")?;
            let one = self.context.i64_type().const_int(1, false);
            let result = self.builder.build_call(value_long_fn, &[one.into()], "one").unwrap();
            return Ok(result.try_as_basic_value().left().unwrap().into_pointer_value());
        }

        let mul_fn = self.module.get_function("clorus_mul")
            .ok_or("clorus_mul not declared")?;

        let mut result = self.compile_expr(&args[0])?;

        for arg in &args[1..] {
            let val_ptr = self.compile_expr(arg)?;
            let call_result = self.builder.build_call(
                mul_fn,
                &[result.into(), val_ptr.into()],
                "mul"
            ).unwrap();
            result = call_result.try_as_basic_value().left().unwrap().into_pointer_value();
        }

        Ok(result)
    }

    fn compile_div(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() < 2 {
            return Err("/ requires at least two arguments".to_string());
        }

        let div_fn = self.module.get_function("clorus_div")
            .ok_or("clorus_div not declared")?;

        let mut result = self.compile_expr(&args[0])?;

        for arg in &args[1..] {
            let val_ptr = self.compile_expr(arg)?;
            let call_result = self.builder.build_call(
                div_fn,
                &[result.into(), val_ptr.into()],
                "div"
            ).unwrap();
            result = call_result.try_as_basic_value().left().unwrap().into_pointer_value();
        }

        Ok(result)
    }

    fn compile_mod(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("mod requires exactly two arguments".to_string());
        }

        let mod_fn = self.module.get_function("clorus_mod")
            .ok_or("clorus_mod not declared")?;

        let left = self.compile_expr(&args[0])?;
        let right = self.compile_expr(&args[1])?;

        let call_result = self.builder.build_call(
            mod_fn,
            &[left.into(), right.into()],
            "mod"
        ).unwrap();

        Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
    }

    fn compile_lt(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("< requires exactly two arguments".to_string());
        }

        // Unbox arguments
        let left_ptr = self.compile_expr(&args[0])?;
        let right_ptr = self.compile_expr(&args[1])?;
        let left = self.unbox_number(left_ptr);
        let right = self.unbox_number(right_ptr);

        let cmp = self.builder.build_float_compare(
            FloatPredicate::OLT,
            left,
            right,
            "lt"
        ).unwrap();

        // Convert bool to float: true => 1.0, false => 0.0
        let bool_as_float = self.builder.build_unsigned_int_to_float(
            cmp,
            self.context.f64_type(),
            "bool_to_float"
        ).unwrap();

        // Box as boolean value
        let bool_fn = self.module.get_function("clorus_value_bool")
            .ok_or("clorus_value_bool not declared")?;
        let call_result = self.builder.build_call(
            bool_fn,
            &[bool_as_float.into()],
            "bool_value"
        ).unwrap();
        Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
    }

    fn compile_gt(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("> requires exactly two arguments".to_string());
        }

        // Unbox arguments
        let left_ptr = self.compile_expr(&args[0])?;
        let right_ptr = self.compile_expr(&args[1])?;
        let left = self.unbox_number(left_ptr);
        let right = self.unbox_number(right_ptr);

        let cmp = self.builder.build_float_compare(
            FloatPredicate::OGT,
            left,
            right,
            "gt"
        ).unwrap();

        // Convert bool to float: true => 1.0, false => 0.0
        let bool_as_float = self.builder.build_unsigned_int_to_float(
            cmp,
            self.context.f64_type(),
            "bool_to_float"
        ).unwrap();

        // Box as boolean value
        let bool_fn = self.module.get_function("clorus_value_bool")
            .ok_or("clorus_value_bool not declared")?;
        let call_result = self.builder.build_call(
            bool_fn,
            &[bool_as_float.into()],
            "bool_value"
        ).unwrap();
        Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
    }

    fn compile_lte(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("<= requires exactly two arguments".to_string());
        }

        let left_ptr = self.compile_expr(&args[0])?;
        let right_ptr = self.compile_expr(&args[1])?;
        let left = self.unbox_number(left_ptr);
        let right = self.unbox_number(right_ptr);

        let cmp = self.builder.build_float_compare(
            FloatPredicate::OLE,
            left,
            right,
            "lte"
        ).unwrap();

        let bool_as_float = self.builder.build_unsigned_int_to_float(
            cmp,
            self.context.f64_type(),
            "bool_to_float"
        ).unwrap();

        // Box as boolean value
        let bool_fn = self.module.get_function("clorus_value_bool")
            .ok_or("clorus_value_bool not declared")?;
        let call_result = self.builder.build_call(
            bool_fn,
            &[bool_as_float.into()],
            "bool_value"
        ).unwrap();
        Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
    }

    fn compile_gte(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err(">= requires exactly two arguments".to_string());
        }

        let left_ptr = self.compile_expr(&args[0])?;
        let right_ptr = self.compile_expr(&args[1])?;
        let left = self.unbox_number(left_ptr);
        let right = self.unbox_number(right_ptr);

        let cmp = self.builder.build_float_compare(
            FloatPredicate::OGE,
            left,
            right,
            "gte"
        ).unwrap();

        let bool_as_float = self.builder.build_unsigned_int_to_float(
            cmp,
            self.context.f64_type(),
            "bool_to_float"
        ).unwrap();

        // Box as boolean value
        let bool_fn = self.module.get_function("clorus_value_bool")
            .ok_or("clorus_value_bool not declared")?;
        let call_result = self.builder.build_call(
            bool_fn,
            &[bool_as_float.into()],
            "bool_value"
        ).unwrap();
        Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
    }

    fn compile_eq(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("= requires exactly two arguments".to_string());
        }

        // Unbox arguments
        let left_ptr = self.compile_expr(&args[0])?;
        let right_ptr = self.compile_expr(&args[1])?;
        let left = self.unbox_number(left_ptr);
        let right = self.unbox_number(right_ptr);

        let cmp = self.builder.build_float_compare(
            FloatPredicate::OEQ,
            left,
            right,
            "eq"
        ).unwrap();

        // Convert bool to float: true => 1.0, false => 0.0
        let bool_as_float = self.builder.build_unsigned_int_to_float(
            cmp,
            self.context.f64_type(),
            "bool_to_float"
        ).unwrap();

        // Box as boolean value
        let bool_fn = self.module.get_function("clorus_value_bool")
            .ok_or("clorus_value_bool not declared")?;
        let call_result = self.builder.build_call(
            bool_fn,
            &[bool_as_float.into()],
            "bool_value"
        ).unwrap();
        Ok(call_result.try_as_basic_value().left().unwrap().into_pointer_value())
    }

    /// Helper: Compile string expression to C string pointer
    /// Handles both string literals and string variables (Value*)
    fn compile_string_to_ptr(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
        match expr {
            Expr::String(s) => {
                // Create global string constant (null-terminated C string)
                let c_str = self.builder.build_global_string_ptr(s, "str").unwrap();
                Ok(c_str.as_pointer_value())
            }
            Expr::Symbol(_) => {
                // Variable containing a Value* (which may be a string)
                // Compile to get the Value*, then extract C string from it
                let value_ptr = self.compile_expr(expr)?;
                Ok(self.extract_cstring_from_value(value_ptr))
            }
            _ => Err("Expected string literal or string variable".to_string()),
        }
    }

    /// Compile fs/* function calls (rust.fs module functions)
    fn compile_fs_call(&mut self, func: &str, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        match func {
            "fs/read" => {
                // fs/read takes 1 arg: path (string)
                if args.len() != 1 {
                    return Err("fs/read requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_read_fn = self.module.get_function("clorus_fs_read")
                    .ok_or("fs/read not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_read_fn,
                    &[path_ptr.into()],
                    "fs_read_call"
                ).unwrap();

                // fs/read returns *mut c_char (string pointer)
                // Box it into a Value* string
                let str_ptr = result.try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(self.box_string(str_ptr))
            }

            "fs/write" => {
                // fs/write takes 2 args: path (string), content (string)
                if args.len() != 2 {
                    return Err("fs/write requires 2 arguments: path, content".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;
                let content_ptr = self.compile_string_to_ptr(&args[1])?;

                let fs_write_fn = self.module.get_function("clorus_fs_write")
                    .ok_or("fs/write not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_write_fn,
                    &[path_ptr.into(), content_ptr.into()],
                    "fs_write_call"
                ).unwrap();

                // fs/write returns i32 (1 = success, 0 = failure)
                // Convert to f64 and box as number
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/append" => {
                if args.len() != 2 {
                    return Err("fs/append requires 2 arguments: path, content".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;
                let content_ptr = self.compile_string_to_ptr(&args[1])?;

                let fs_append_fn = self.module.get_function("clorus_fs_append")
                    .ok_or("fs/append not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_append_fn,
                    &[path_ptr.into(), content_ptr.into()],
                    "fs_append_call"
                ).unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/exists?" => {
                if args.len() != 1 {
                    return Err("fs/exists? requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_exists_fn = self.module.get_function("clorus_fs_exists")
                    .ok_or("fs/exists? not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_exists_fn,
                    &[path_ptr.into()],
                    "fs_exists_call"
                ).unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/is-file?" => {
                if args.len() != 1 {
                    return Err("fs/is-file? requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_is_file_fn = self.module.get_function("clorus_fs_is_file")
                    .ok_or("fs/is-file? not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_is_file_fn,
                    &[path_ptr.into()],
                    "fs_is_file_call"
                ).unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/is-dir?" => {
                if args.len() != 1 {
                    return Err("fs/is-dir? requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_is_dir_fn = self.module.get_function("clorus_fs_is_dir")
                    .ok_or("fs/is-dir? not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_is_dir_fn,
                    &[path_ptr.into()],
                    "fs_is_dir_call"
                ).unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/remove" => {
                if args.len() != 1 {
                    return Err("fs/remove requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_remove_fn = self.module.get_function("clorus_fs_remove")
                    .ok_or("fs/remove not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_remove_fn,
                    &[path_ptr.into()],
                    "fs_remove_call"
                ).unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/copy" => {
                if args.len() != 2 {
                    return Err("fs/copy requires 2 arguments: src, dst".to_string());
                }

                let src_ptr = self.compile_string_to_ptr(&args[0])?;
                let dst_ptr = self.compile_string_to_ptr(&args[1])?;

                let fs_copy_fn = self.module.get_function("clorus_fs_copy")
                    .ok_or("fs/copy not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_copy_fn,
                    &[src_ptr.into(), dst_ptr.into()],
                    "fs_copy_call"
                ).unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/rename" => {
                if args.len() != 2 {
                    return Err("fs/rename requires 2 arguments: old, new".to_string());
                }

                let old_ptr = self.compile_string_to_ptr(&args[0])?;
                let new_ptr = self.compile_string_to_ptr(&args[1])?;

                let fs_rename_fn = self.module.get_function("clorus_fs_rename")
                    .ok_or("fs/rename not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_rename_fn,
                    &[old_ptr.into(), new_ptr.into()],
                    "fs_rename_call"
                ).unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/create-dir" => {
                if args.len() != 1 {
                    return Err("fs/create-dir requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_create_dir_fn = self.module.get_function("clorus_fs_create_dir")
                    .ok_or("fs/create-dir not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_create_dir_fn,
                    &[path_ptr.into()],
                    "fs_create_dir_call"
                ).unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "fs/create-dir-all" => {
                if args.len() != 1 {
                    return Err("fs/create-dir-all requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let fs_create_dir_all_fn = self.module.get_function("clorus_fs_create_dir_all")
                    .ok_or("fs/create-dir-all not declared. Did you forget (use rust.fs)?")?;

                let result = self.builder.build_call(
                    fs_create_dir_all_fn,
                    &[path_ptr.into()],
                    "fs_create_dir_all_call"
                ).unwrap();

                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            _ => Err(format!("Unknown fs function: {}", func)),
        }
    }

    /// Compile rust.example function calls (example Rust library for FFI testing)
    fn compile_rust_example_call(&mut self, func: &str, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        match func {
            "example/add" => {
                // add takes 2 args: x, y (both f64)
                if args.len() != 2 {
                    return Err("example/add requires 2 arguments: x y".to_string());
                }

                // Compile arguments and unbox to f64
                let x_ptr = self.compile_expr(&args[0])?;
                let y_ptr = self.compile_expr(&args[1])?;
                let x = self.unbox_number(x_ptr);
                let y = self.unbox_number(y_ptr);

                // Call clorus_add
                let add_fn = self.module.get_function("clorus_add")
                    .ok_or_else(|| "clorus_add not found - did you (use rust.example)?".to_string())?;

                let result = self.builder.build_call(
                    add_fn,
                    &[x.into(), y.into()],
                    "example_add"
                ).unwrap();

                let f64_result = result.try_as_basic_value().left().unwrap().into_float_value();

                // Box the result
                Ok(self.box_number(f64_result))
            }

            "example/multiply" => {
                if args.len() != 2 {
                    return Err("example/multiply requires 2 arguments: a b".to_string());
                }

                let a_ptr = self.compile_expr(&args[0])?;
                let b_ptr = self.compile_expr(&args[1])?;
                let a = self.unbox_number(a_ptr);
                let b = self.unbox_number(b_ptr);

                let multiply_fn = self.module.get_function("clorus_multiply")
                    .ok_or_else(|| "clorus_multiply not found - did you (use rust.example)?".to_string())?;

                let result = self.builder.build_call(
                    multiply_fn,
                    &[a.into(), b.into()],
                    "example_multiply"
                ).unwrap();

                let f64_result = result.try_as_basic_value().left().unwrap().into_float_value();
                Ok(self.box_number(f64_result))
            }

            "example/factorial" => {
                if args.len() != 1 {
                    return Err("example/factorial requires 1 argument: n".to_string());
                }

                let n_ptr = self.compile_expr(&args[0])?;
                let n = self.unbox_number(n_ptr);

                let factorial_fn = self.module.get_function("clorus_factorial")
                    .ok_or_else(|| "clorus_factorial not found - did you (use rust.example)?".to_string())?;

                let result = self.builder.build_call(
                    factorial_fn,
                    &[n.into()],
                    "example_factorial"
                ).unwrap();

                let f64_result = result.try_as_basic_value().left().unwrap().into_float_value();
                Ok(self.box_number(f64_result))
            }

            _ => Err(format!("Unknown rust.example function: {}", func))
        }
    }

    /// Compile rust.async-demo function calls
    fn compile_async_demo_call(&mut self, func: &str, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        match func {
            "async-demo/hello-blocking" => {
                // hello_blocking takes no args, returns String
                if !args.is_empty() {
                    return Err("async-demo/hello-blocking takes no arguments".to_string());
                }

                let hello_fn = self.module.get_function("clorus_hello_blocking")
                    .ok_or_else(|| "clorus_hello_blocking not found - did you (use rust.async-demo)?".to_string())?;

                let result = self.builder.build_call(
                    hello_fn,
                    &[],
                    "async_hello"
                ).unwrap();

                let str_ptr = result.try_as_basic_value().left().unwrap().into_pointer_value();

                // Box the returned C string into Value*
                Ok(self.box_string(str_ptr))
            }

            "async-demo/countdown-blocking" => {
                // countdown_blocking takes 1 arg (n: f64), returns f64
                if args.len() != 1 {
                    return Err("async-demo/countdown-blocking requires 1 argument: n".to_string());
                }

                let n_ptr = self.compile_expr(&args[0])?;
                let n = self.unbox_number(n_ptr);

                let countdown_fn = self.module.get_function("clorus_countdown_blocking")
                    .ok_or_else(|| "clorus_countdown_blocking not found - did you (use rust.async-demo)?".to_string())?;

                let result = self.builder.build_call(
                    countdown_fn,
                    &[n.into()],
                    "async_countdown"
                ).unwrap();

                let f64_result = result.try_as_basic_value().left().unwrap().into_float_value();
                Ok(self.box_number(f64_result))
            }

            _ => Err(format!("Unknown rust.async-demo function: {}", func))
        }
    }

    /// Compile rust.async-hello function calls
    fn compile_async_hello_call(&mut self, func: &str, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        match func {
            "async-hello/greet-blocking" => {
                // greet_blocking takes 1 arg (name: String), returns String
                if args.len() != 1 {
                    return Err("async-hello/greet-blocking requires 1 argument: name".to_string());
                }

                let name_val = self.compile_expr(&args[0])?;
                let name_cstr = self.extract_cstring_from_value(name_val);

                let greet_fn = self.module.get_function("clorus_greet_blocking")
                    .ok_or_else(|| "clorus_greet_blocking not found - did you (use rust.async-hello)?".to_string())?;

                let result = self.builder.build_call(
                    greet_fn,
                    &[name_cstr.into()],
                    "async_greet"
                ).unwrap();

                let str_ptr = result.try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(self.box_string(str_ptr))
            }

            "async-hello/add-blocking" => {
                // add_blocking takes 2 args (x: f64, y: f64), returns f64
                if args.len() != 2 {
                    return Err("async-hello/add-blocking requires 2 arguments: x y".to_string());
                }

                let x_ptr = self.compile_expr(&args[0])?;
                let y_ptr = self.compile_expr(&args[1])?;
                let x = self.unbox_number(x_ptr);
                let y = self.unbox_number(y_ptr);

                let add_fn = self.module.get_function("clorus_add_blocking")
                    .ok_or_else(|| "clorus_add_blocking not found - did you (use rust.async-hello)?".to_string())?;

                let result = self.builder.build_call(
                    add_fn,
                    &[x.into(), y.into()],
                    "async_add"
                ).unwrap();

                let f64_result = result.try_as_basic_value().left().unwrap().into_float_value();
                Ok(self.box_number(f64_result))
            }

            _ => Err(format!("Unknown rust.async-hello function: {}", func))
        }
    }

    /// Compile rust.egui-hello function calls
    fn compile_egui_hello_call(&mut self, func: &str, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        match func {
            "egui-hello/show-gui" => {
                // show_gui takes 1 arg (message: String), returns f64
                if args.len() != 1 {
                    return Err("egui-hello/show-gui requires 1 argument: message".to_string());
                }

                let message_val = self.compile_expr(&args[0])?;
                let message_cstr = self.extract_cstring_from_value(message_val);

                let show_gui_fn = self.module.get_function("clorus_show_gui")
                    .ok_or_else(|| "clorus_show_gui not found - did you (use rust.egui-hello)?".to_string())?;

                let result = self.builder.build_call(
                    show_gui_fn,
                    &[message_cstr.into()],
                    "show_gui"
                ).unwrap();

                let f64_result = result.try_as_basic_value().left().unwrap().into_float_value();
                Ok(self.box_number(f64_result))
            }

            "egui-hello/get-gui-version" => {
                // get_gui_version takes no args, returns String
                if !args.is_empty() {
                    return Err("egui-hello/get-gui-version takes no arguments".to_string());
                }

                let get_version_fn = self.module.get_function("clorus_get_gui_version")
                    .ok_or_else(|| "clorus_get_gui_version not found - did you (use rust.egui-hello)?".to_string())?;

                let result = self.builder.build_call(
                    get_version_fn,
                    &[],
                    "get_gui_version"
                ).unwrap();

                let str_ptr = result.try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(self.box_string(str_ptr))
            }

            _ => Err(format!("Unknown rust.egui-hello function: {}", func))
        }
    }

    /// Compile clorus.core function calls (Clojure-style convenience functions)
    fn compile_core_call(&mut self, func: &str, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        match func {
            "slurp" => {
                // slurp takes 1 arg: path (string)
                if args.len() != 1 {
                    return Err("slurp requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let slurp_fn = self.module.get_function("clorus_slurp")
                    .ok_or("slurp not declared. Did you forget (use clorus.core)?")?;

                let result = self.builder.build_call(
                    slurp_fn,
                    &[path_ptr.into()],
                    "slurp_call"
                ).unwrap();

                // slurp returns *mut c_char (string pointer)
                // Box it into a Value* string
                let str_ptr = result.try_as_basic_value().left().unwrap().into_pointer_value();
                Ok(self.box_string(str_ptr))
            }

            "spit" => {
                // spit takes 2 args: path (string), content (string)
                if args.len() != 2 {
                    return Err("spit requires 2 arguments: path, content".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;
                let content_ptr = self.compile_string_to_ptr(&args[1])?;

                let spit_fn = self.module.get_function("clorus_spit")
                    .ok_or("spit not declared. Did you forget (use clorus.core)?")?;

                let result = self.builder.build_call(
                    spit_fn,
                    &[path_ptr.into(), content_ptr.into()],
                    "spit_call"
                ).unwrap();

                // spit returns i32 (1 = success, 0 = failure)
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "i32_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "get" => {
                // get takes 2-3 args: collection, key, [default]
                if args.len() < 2 || args.len() > 3 {
                    return Err("get requires 2 or 3 arguments: collection, key, [default]".to_string());
                }

                let coll_ptr = self.compile_expr(&args[0])?;
                let key_ptr = self.compile_expr(&args[1])?;

                let get_fn = self.module.get_function("clorus_get")
                    .ok_or("get not declared")?;

                let result = self.builder.build_call(
                    get_fn,
                    &[coll_ptr.into(), key_ptr.into()],
                    "get_call"
                ).unwrap();

                let result_ptr = result.try_as_basic_value().left().unwrap().into_pointer_value();

                // If we have a default value and result is nil, return default
                if args.len() == 3 {
                    // Check if result is nil
                    let is_nil_fn = self.module.get_function("clorus_value_is_nil")
                        .ok_or("clorus_value_is_nil not declared")?;
                    let is_nil_result = self.builder.build_call(
                        is_nil_fn,
                        &[result_ptr.into()],
                        "is_nil_check"
                    ).unwrap();
                    let is_nil = is_nil_result.try_as_basic_value().left().unwrap().into_int_value();

                    // If nil, return default
                    let current_fn = self.builder.get_insert_block().unwrap().get_parent().unwrap();
                    let then_block = self.context.append_basic_block(current_fn, "return_default");
                    let else_block = self.context.append_basic_block(current_fn, "return_value");
                    let merge_block = self.context.append_basic_block(current_fn, "merge");

                    self.builder.build_conditional_branch(is_nil, then_block, else_block).unwrap();

                    // Then: return default
                    self.builder.position_at_end(then_block);
                    let default_ptr = self.compile_expr(&args[2])?;
                    self.builder.build_unconditional_branch(merge_block).unwrap();

                    // Else: return result
                    self.builder.position_at_end(else_block);
                    self.builder.build_unconditional_branch(merge_block).unwrap();

                    // Merge
                    self.builder.position_at_end(merge_block);
                    let phi = self.builder.build_phi(result_ptr.get_type(), "get_result").unwrap();
                    phi.add_incoming(&[(&default_ptr, then_block), (&result_ptr, else_block)]);

                    Ok(phi.as_basic_value().into_pointer_value())
                } else {
                    Ok(result_ptr)
                }
            }

            "nth" => {
                // nth takes 2 args: collection, index
                if args.len() != 2 {
                    return Err("nth requires 2 arguments: collection, index".to_string());
                }

                let coll_ptr = self.compile_expr(&args[0])?;
                let index_ptr = self.compile_expr(&args[1])?;

                // Unbox index to i64
                let index_float = self.unbox_number(index_ptr);
                let index_i64 = self.builder.build_float_to_signed_int(
                    index_float,
                    self.context.i64_type(),
                    "index_to_i64"
                ).unwrap();

                let nth_fn = self.module.get_function("clorus_nth")
                    .ok_or("nth not declared")?;

                let result = self.builder.build_call(
                    nth_fn,
                    &[coll_ptr.into(), index_i64.into()],
                    "nth_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "first" => self.compile_simple_1arg_call("first", "clorus_first", args),

            "rest" => self.compile_simple_1arg_call("rest", "clorus_rest", args),

            "last" => self.compile_simple_1arg_call("last", "clorus_last", args),

            "count" => {
                // count takes 1 arg: collection
                if args.len() != 1 {
                    return Err("count requires 1 argument: collection".to_string());
                }

                let coll_ptr = self.compile_expr(&args[0])?;

                let count_fn = self.module.get_function("clorus_count")
                    .ok_or("count not declared")?;

                let result = self.builder.build_call(
                    count_fn,
                    &[coll_ptr.into()],
                    "count_call"
                ).unwrap();

                // count returns i64, box as number
                let count_i64 = result.try_as_basic_value().left().unwrap().into_int_value();
                let count_float = self.builder.build_signed_int_to_float(
                    count_i64,
                    self.context.f64_type(),
                    "count_to_float"
                ).unwrap();

                Ok(self.box_number(count_float))
            }

            "empty?" => {
                // empty? takes 1 arg: collection
                if args.len() != 1 {
                    return Err("empty? requires 1 argument: collection".to_string());
                }

                let coll_ptr = self.compile_expr(&args[0])?;

                let count_fn = self.module.get_function("clorus_count")
                    .ok_or("clorus_count not declared")?;

                let count_result = self.builder.build_call(
                    count_fn,
                    &[coll_ptr.into()],
                    "count_call"
                ).unwrap();

                // count returns i64, compare with 0
                let count_i64 = count_result.try_as_basic_value().left().unwrap().into_int_value();
                let zero = self.context.i64_type().const_zero();
                let is_empty = self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    count_i64,
                    zero,
                    "is_empty"
                ).unwrap();

                // Convert bool to Value* (boolean)
                let value_bool_fn = self.module.get_function("clorus_value_boolean")
                    .ok_or("clorus_value_boolean not declared")?;
                let result = self.builder.build_call(
                    value_bool_fn,
                    &[is_empty.into()],
                    "empty_bool"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "conj" => {
                // conj takes 2 args: collection, element
                if args.len() != 2 {
                    return Err("conj requires 2 arguments: collection, element".to_string());
                }

                let coll_ptr = self.compile_expr(&args[0])?;
                let elem_ptr = self.compile_expr(&args[1])?;

                // Use generic clorus_conj which dispatches based on collection type
                let conj_fn = self.module.get_function("clorus_conj")
                    .ok_or("clorus_conj not declared")?;

                let result = self.builder.build_call(
                    conj_fn,
                    &[coll_ptr.into(), elem_ptr.into()],
                    "conj_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "disj" => {
                // disj takes 2 args: set, element
                if args.len() != 2 {
                    return Err("disj requires 2 arguments: set, element".to_string());
                }

                let set_ptr = self.compile_expr(&args[0])?;
                let elem_ptr = self.compile_expr(&args[1])?;

                let disj_fn = self.module.get_function("clorus_set_disj")
                    .ok_or("clorus_set_disj not declared")?;

                let result = self.builder.build_call(
                    disj_fn,
                    &[set_ptr.into(), elem_ptr.into()],
                    "set_disj_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "assoc" => {
                // assoc takes 3 args: map, key, value
                if args.len() != 3 {
                    return Err("assoc requires 3 arguments: map, key, value".to_string());
                }

                let map_ptr = self.compile_expr(&args[0])?;
                let key_ptr = self.compile_expr(&args[1])?;
                let val_ptr = self.compile_expr(&args[2])?;

                let assoc_fn = self.module.get_function("clorus_map_assoc")
                    .ok_or("clorus_map_assoc not declared")?;

                let result = self.builder.build_call(
                    assoc_fn,
                    &[map_ptr.into(), key_ptr.into(), val_ptr.into()],
                    "map_assoc_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "dissoc" => {
                // dissoc takes 2 args: map, key
                if args.len() != 2 {
                    return Err("dissoc requires 2 arguments: map, key".to_string());
                }

                let map_ptr = self.compile_expr(&args[0])?;
                let key_ptr = self.compile_expr(&args[1])?;

                let dissoc_fn = self.module.get_function("clorus_map_dissoc")
                    .ok_or("clorus_map_dissoc not declared")?;

                let result = self.builder.build_call(
                    dissoc_fn,
                    &[map_ptr.into(), key_ptr.into()],
                    "map_dissoc_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "contains?" => {
                // contains? takes 2 args: set, element
                if args.len() != 2 {
                    return Err("contains? requires 2 arguments: set, element".to_string());
                }

                let set_ptr = self.compile_expr(&args[0])?;
                let elem_ptr = self.compile_expr(&args[1])?;

                let contains_fn = self.module.get_function("clorus_set_contains")
                    .ok_or("clorus_set_contains not declared")?;

                let result = self.builder.build_call(
                    contains_fn,
                    &[set_ptr.into(), elem_ptr.into()],
                    "set_contains_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "reduce" => {
                // reduce takes 2-3 args: function, init-val (optional), collection
                if args.len() < 2 || args.len() > 3 {
                    return Err("reduce requires 2 or 3 arguments: function, [init], collection".to_string());
                }

                // Compile the reducing function - supports both named functions and inline lambdas
                let func_val = self.compile_expr(&args[0])?;

                let (init_expr, coll_expr) = if args.len() == 3 {
                    (&args[1], &args[2])
                } else {
                    // No init value - use first element
                    return Err("reduce without init value not yet supported".to_string());
                };

                let init_val = self.compile_expr(init_expr)?;
                let coll_ptr = self.compile_expr(coll_expr)?;

                let count_fn = self.module.get_function("clorus_count")
                    .ok_or("count not declared")?;
                let count_i64 = self.builder.build_call(count_fn, &[coll_ptr.into()], "coll_count")
                    .unwrap().try_as_basic_value().left().unwrap().into_int_value();

                // Loop setup
                let current_fn = self.builder.get_insert_block().unwrap().get_parent().unwrap();
                let loop_block = self.context.append_basic_block(current_fn, "reduce_loop");
                let body_block = self.context.append_basic_block(current_fn, "reduce_body");
                let end_block = self.context.append_basic_block(current_fn, "reduce_end");

                let index_alloca = self.builder.build_alloca(self.context.i64_type(), "index").unwrap();
                self.builder.build_store(index_alloca, self.context.i64_type().const_zero()).unwrap();

                let acc_alloca = self.builder.build_alloca(init_val.get_type(), "accumulator").unwrap();
                self.builder.build_store(acc_alloca, init_val).unwrap();

                self.builder.build_unconditional_branch(loop_block).unwrap();

                // Loop condition
                self.builder.position_at_end(loop_block);
                let current_index = self.builder.build_load(self.context.i64_type(), index_alloca, "current_index")
                    .unwrap().into_int_value();
                let condition = self.builder.build_int_compare(
                    inkwell::IntPredicate::SLT, current_index, count_i64, "loop_cond"
                ).unwrap();
                self.builder.build_conditional_branch(condition, body_block, end_block).unwrap();

                // Loop body
                self.builder.position_at_end(body_block);
                let nth_fn = self.module.get_function("clorus_nth")
                    .ok_or("nth not declared")?;
                let elem = self.builder.build_call(nth_fn, &[coll_ptr.into(), current_index.into()], "elem")
                    .unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                let current_acc = self.builder.build_load(init_val.get_type(), acc_alloca, "current_acc")
                    .unwrap().into_pointer_value();

                // Call reducing function with (acc, elem) using dynamic dispatch
                let function_call_fn = self.module.get_function("clorus_function_call")
                    .ok_or("clorus_function_call not declared")?;

                let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let args_array_type = i8_ptr_type.array_type(2);
                let args_array = self.builder.build_alloca(args_array_type, "reduce_args").unwrap();

                // Store acc as first arg
                let acc_ptr = unsafe {
                    self.builder.build_gep(
                        args_array_type,
                        args_array,
                        &[
                            self.context.i32_type().const_zero(),
                            self.context.i32_type().const_zero()
                        ],
                        "acc_ptr"
                    ).unwrap()
                };
                self.builder.build_store(acc_ptr, current_acc).unwrap();

                // Store elem as second arg
                let elem_ptr = unsafe {
                    self.builder.build_gep(
                        args_array_type,
                        args_array,
                        &[
                            self.context.i32_type().const_zero(),
                            self.context.i32_type().const_int(1, false)
                        ],
                        "elem_ptr"
                    ).unwrap()
                };
                self.builder.build_store(elem_ptr, elem).unwrap();

                let args_array_ptr = self.builder.build_pointer_cast(
                    args_array,
                    i8_ptr_type.ptr_type(AddressSpace::default()),
                    "args_cast"
                ).unwrap();

                let new_acc = self.builder.build_call(
                    function_call_fn,
                    &[func_val.into(), args_array_ptr.into(), self.context.i32_type().const_int(2, false).into()],
                    "new_acc"
                ).unwrap()
                .try_as_basic_value().left().unwrap().into_pointer_value();
                self.builder.build_store(acc_alloca, new_acc).unwrap();

                let next_index = self.builder.build_int_add(
                    current_index, self.context.i64_type().const_int(1, false), "next_index"
                ).unwrap();
                self.builder.build_store(index_alloca, next_index).unwrap();
                self.builder.build_unconditional_branch(loop_block).unwrap();

                // End
                self.builder.position_at_end(end_block);
                let final_acc = self.builder.build_load(init_val.get_type(), acc_alloca, "final_acc")
                    .unwrap().into_pointer_value();

                Ok(final_acc)
            }

            "atom" => {
                // atom takes 1 arg: initial value
                if args.len() != 1 {
                    return Err("atom requires 1 argument: initial value".to_string());
                }

                let initial_val = self.compile_expr(&args[0])?;

                let atom_fn = self.module.get_function("clorus_atom")
                    .ok_or("clorus_atom not declared")?;

                let result = self.builder.build_call(
                    atom_fn,
                    &[initial_val.into()],
                    "atom_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "reset!" => {
                // reset! takes 2 args: atom, new-value
                if args.len() != 2 {
                    return Err("reset! requires 2 arguments: atom, new-value".to_string());
                }

                let atom_val = self.compile_expr(&args[0])?;
                let new_val = self.compile_expr(&args[1])?;

                let reset_fn = self.module.get_function("clorus_reset")
                    .ok_or("clorus_reset not declared")?;

                let result = self.builder.build_call(
                    reset_fn,
                    &[atom_val.into(), new_val.into()],
                    "reset_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "swap!" => {
                // swap! takes 2+ args: atom, function, [args...]
                // For now, support: (swap! atom func arg)
                if args.len() < 2 {
                    return Err("swap! requires at least 2 arguments: atom, function".to_string());
                }

                let atom_val = self.compile_expr(&args[0])?;

                // Get the function to apply
                let func_name = match &args[1] {
                    Expr::Symbol(name) => name.clone(),
                    _ => return Err("swap! requires a function as second argument".to_string()),
                };

                let function = self.functions.get(&func_name)
                    .ok_or_else(|| format!("Function not found: {}", func_name))?
                    .clone();

                // Get function pointer
                let func_ptr = function.as_global_value().as_pointer_value();

                // For now, support single additional argument
                let arg_val = if args.len() >= 3 {
                    self.compile_expr(&args[2])?
                } else {
                    // No additional arg - pass nil
                    self.box_number(self.context.f64_type().const_float(0.0))
                };

                let swap_fn = self.module.get_function("clorus_swap")
                    .ok_or("clorus_swap not declared")?;

                let result = self.builder.build_call(
                    swap_fn,
                    &[atom_val.into(), func_ptr.into(), arg_val.into()],
                    "swap_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "ref" => {
                // ref takes 1 arg: initial value
                if args.len() != 1 {
                    return Err("ref requires 1 argument: initial value".to_string());
                }

                let initial_val = self.compile_expr(&args[0])?;

                let ref_fn = self.module.get_function("clorus_ref")
                    .ok_or("clorus_ref not declared")?;

                let result = self.builder.build_call(
                    ref_fn,
                    &[initial_val.into()],
                    "ref_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "ref-set" => {
                // ref-set takes 2 args: ref, new-value
                // Must be inside dosync
                if args.len() != 2 {
                    return Err("ref-set requires 2 arguments: ref, new-value".to_string());
                }

                let ref_val = self.compile_expr(&args[0])?;
                let new_val = self.compile_expr(&args[1])?;

                let ref_set_fn = self.module.get_function("clorus_ref_set")
                    .ok_or("clorus_ref_set not declared")?;

                let result = self.builder.build_call(
                    ref_set_fn,
                    &[ref_val.into(), new_val.into()],
                    "ref_set_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "alter" => {
                // alter takes 2+ args: ref, function, [args...]
                // Simplified version: (alter ref func arg)
                // The function application happens here, then we call clorus_alter
                if args.len() < 2 {
                    return Err("alter requires at least 2 arguments: ref, function".to_string());
                }

                let ref_val = self.compile_expr(&args[0])?;

                // Get current value from ref
                let ref_deref_fn = self.module.get_function("clorus_ref_deref")
                    .ok_or("clorus_ref_deref not declared")?;

                let current_val = self.builder.build_call(
                    ref_deref_fn,
                    &[ref_val.into()],
                    "alter_deref"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                // Get the function to apply
                let func_name = match &args[1] {
                    Expr::Symbol(name) => name.clone(),
                    _ => return Err("alter requires a function as second argument".to_string()),
                };

                let function = self.functions.get(&func_name)
                    .ok_or_else(|| format!("Function not found: {}", func_name))?
                    .clone();

                // Apply function to current value
                // For now, support single additional argument
                let func_args: Vec<BasicMetadataValueEnum> = if args.len() >= 3 {
                    let arg_val = self.compile_expr(&args[2])?;
                    vec![current_val.into(), arg_val.into()]
                } else {
                    vec![current_val.into()]
                };

                let new_val = self.builder.build_call(
                    function,
                    &func_args,
                    "alter_apply"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                // Now set the ref to the new value
                let alter_fn = self.module.get_function("clorus_alter")
                    .ok_or("clorus_alter not declared")?;

                let result = self.builder.build_call(
                    alter_fn,
                    &[ref_val.into(), new_val.into()],
                    "alter_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "agent" => {
                // agent takes 1 arg: initial value
                if args.len() != 1 {
                    return Err("agent requires 1 argument: initial value".to_string());
                }

                let initial_val = self.compile_expr(&args[0])?;

                let agent_fn = self.module.get_function("clorus_agent")
                    .ok_or("clorus_agent not declared")?;

                let result = self.builder.build_call(
                    agent_fn,
                    &[initial_val.into()],
                    "agent_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "send" => {
                // send takes 2+ args: agent, function, [args...]
                if args.len() < 2 {
                    return Err("send requires at least 2 arguments: agent, function".to_string());
                }

                let agent_val = self.compile_expr(&args[0])?;

                // Get the function to apply
                let func_name = match &args[1] {
                    Expr::Symbol(name) => name.clone(),
                    _ => return Err("send requires a function as second argument".to_string()),
                };

                let function = self.functions.get(&func_name)
                    .ok_or_else(|| format!("Function not found: {}", func_name))?
                    .clone();

                // Convert function to pointer value
                let func_ptr = function.as_global_value().as_pointer_value();

                // Pack remaining args into vector
                let vector_empty_fn = self.module.get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;

                let mut args_vec = self.builder.build_call(
                    vector_empty_fn,
                    &[],
                    "send_args_vec"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                // Add each arg to vector
                let vector_conj_fn = self.module.get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                for i in 2..args.len() {
                    let arg_val = self.compile_expr(&args[i])?;
                    args_vec = self.builder.build_call(
                        vector_conj_fn,
                        &[args_vec.into(), arg_val.into()],
                        &format!("send_arg_{}", i)
                    ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();
                }

                // Call clorus_send
                let send_fn = self.module.get_function("clorus_send")
                    .ok_or("clorus_send not declared")?;

                // Cast func_ptr to *mut Value (i8*)
                let func_val = self.builder.build_pointer_cast(
                    func_ptr,
                    self.context.i8_type().ptr_type(inkwell::AddressSpace::default()),
                    "func_as_value"
                ).unwrap();

                let result = self.builder.build_call(
                    send_fn,
                    &[agent_val.into(), func_val.into(), args_vec.into()],
                    "send_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "agent-error" => {
                // agent-error takes 1 arg: agent
                if args.len() != 1 {
                    return Err("agent-error requires 1 argument: agent".to_string());
                }

                let agent_val = self.compile_expr(&args[0])?;

                let agent_error_fn = self.module.get_function("clorus_agent_error")
                    .ok_or("clorus_agent_error not declared")?;

                let result = self.builder.build_call(
                    agent_error_fn,
                    &[agent_val.into()],
                    "agent_error_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "await" => {
                // await takes 1 arg: agent or vector of agents
                if args.len() != 1 {
                    return Err("await requires 1 argument: agent or vector of agents".to_string());
                }

                let agents_val = self.compile_expr(&args[0])?;

                let await_fn = self.module.get_function("clorus_await")
                    .ok_or("clorus_await not declared")?;

                let result = self.builder.build_call(
                    await_fn,
                    &[agents_val.into()],
                    "await_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "await-for" => {
                // await-for takes 2 args: agent, timeout-ms
                if args.len() != 2 {
                    return Err("await-for requires 2 arguments: agent, timeout-ms".to_string());
                }

                let agent_val = self.compile_expr(&args[0])?;
                let timeout_expr = self.compile_expr(&args[1])?;

                // Extract number value
                let value_as_double_fn = self.module.get_function("clorus_value_as_double")
                    .ok_or("clorus_value_as_double not declared")?;

                let timeout_f64 = self.builder.build_call(
                    value_as_double_fn,
                    &[timeout_expr.into()],
                    "timeout_as_f64"
                ).unwrap().try_as_basic_value().left().unwrap().into_float_value();

                // Convert f64 to i64
                let timeout_i64 = self.builder.build_float_to_signed_int(
                    timeout_f64,
                    self.context.i64_type(),
                    "timeout_i64"
                ).unwrap();

                let await_for_fn = self.module.get_function("clorus_await_for")
                    .ok_or("clorus_await_for not declared")?;

                let result = self.builder.build_call(
                    await_for_fn,
                    &[agent_val.into(), timeout_i64.into()],
                    "await_for_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "chan" => {
                // chan takes 0 or 1 arg: optional capacity
                // (chan) => unbounded, (chan n) => buffered with capacity n
                let capacity = if args.is_empty() {
                    // Unbounded channel (-1)
                    self.context.i64_type().const_int(-1i64 as u64, false)
                } else if args.len() == 1 {
                    // Get capacity from argument
                    let cap_expr = self.compile_expr(&args[0])?;

                    // Extract number value
                    let value_as_double_fn = self.module.get_function("clorus_value_as_double")
                        .ok_or("clorus_value_as_double not declared")?;

                    let cap_f64 = self.builder.build_call(
                        value_as_double_fn,
                        &[cap_expr.into()],
                        "cap_as_f64"
                    ).unwrap().try_as_basic_value().left().unwrap().into_float_value();

                    // Convert f64 to i64
                    self.builder.build_float_to_signed_int(
                        cap_f64,
                        self.context.i64_type(),
                        "cap_i64"
                    ).unwrap()
                } else {
                    return Err("chan takes 0 or 1 argument: optional capacity".to_string());
                };

                let chan_fn = self.module.get_function("clorus_chan")
                    .ok_or("clorus_chan not declared")?;

                let result = self.builder.build_call(
                    chan_fn,
                    &[capacity.into()],
                    "chan_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            ">!!" => {
                // >!! takes 2 args: channel, value (blocking put)
                if args.len() != 2 {
                    return Err(">!! requires 2 arguments: channel, value".to_string());
                }

                let chan_val = self.compile_expr(&args[0])?;
                let value_val = self.compile_expr(&args[1])?;

                let chan_put_fn = self.module.get_function("clorus_chan_put")
                    .ok_or("clorus_chan_put not declared")?;

                let result = self.builder.build_call(
                    chan_put_fn,
                    &[chan_val.into(), value_val.into()],
                    "chan_put_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "<!!" => {
                // <!! takes 1 arg: channel (blocking take)
                if args.len() != 1 {
                    return Err("<!! requires 1 argument: channel".to_string());
                }

                let chan_val = self.compile_expr(&args[0])?;

                let chan_take_fn = self.module.get_function("clorus_chan_take")
                    .ok_or("clorus_chan_take not declared")?;

                let result = self.builder.build_call(
                    chan_take_fn,
                    &[chan_val.into()],
                    "chan_take_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "close!" => {
                // close! takes 1 arg: channel
                if args.len() != 1 {
                    return Err("close! requires 1 argument: channel".to_string());
                }

                let chan_val = self.compile_expr(&args[0])?;

                let chan_close_fn = self.module.get_function("clorus_chan_close")
                    .ok_or("clorus_chan_close not declared")?;

                let result = self.builder.build_call(
                    chan_close_fn,
                    &[chan_val.into()],
                    "chan_close_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "alts!!" => {
                // alts!! takes 1 arg: vector of channels
                // Returns [value channel] from first available channel
                if args.len() != 1 {
                    return Err("alts!! requires 1 argument: vector of channels".to_string());
                }

                let channels_val = self.compile_expr(&args[0])?;

                let alts_fn = self.module.get_function("clorus_alts")
                    .ok_or("clorus_alts not declared")?;

                let result = self.builder.build_call(
                    alts_fn,
                    &[channels_val.into()],
                    "alts_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "go" => {
                // go takes 1 arg: body expression to execute asynchronously
                if args.len() != 1 {
                    return Err("go requires 1 argument: body expression".to_string());
                }

                // Find free variables in the body (variables accessed but not defined locally)
                let free_vars = self.find_free_variables(&args[0]);

                // Generate unique function name for go block
                let go_fn_name = format!("_go_block_{}", self.lambda_counter);
                self.lambda_counter += 1;

                // Create function type: takes captures vector, returns Value*
                let value_ptr_type = self.context.i8_type().ptr_type(inkwell::AddressSpace::default());
                let fn_type = value_ptr_type.fn_type(&[value_ptr_type.into()], false);

                // Create the function
                let function = self.module.add_function(&go_fn_name, fn_type, None);

                // Save current state
                let saved_block = self.builder.get_insert_block();
                let saved_vars = self.variables.clone();

                // Create entry block for go function
                let entry = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry);

                // Get captures parameter
                let captures_param = function.get_nth_param(0).unwrap().into_pointer_value();

                // Unpack captured variables
                // captures is a vector: [var1_value, var2_value, ...]
                if !free_vars.is_empty() {
                    let nth_fn = self.module.get_function("clorus_vector_nth")
                        .ok_or("clorus_vector_nth not declared")?;

                    for (i, var_name) in free_vars.iter().enumerate() {
                        // Get value from captures vector
                        let index = self.context.i64_type().const_int(i as u64, false);
                        let var_value = self.builder.build_call(
                            nth_fn,
                            &[captures_param.into(), index.into()],
                            &format!("capture_{}", var_name)
                        ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                        // Create alloca for this variable
                        let alloca = self.create_entry_block_alloca(var_name);
                        self.builder.build_store(alloca, var_value).unwrap();
                        self.variables.insert(var_name.clone(), alloca);
                    }
                }

                // Compile the body expression
                let body_result = self.compile_expr(&args[0])?;

                // Return the result
                self.builder.build_return(Some(&body_result)).unwrap();

                // Restore position and state
                if let Some(block) = saved_block {
                    self.builder.position_at_end(block);
                }
                self.variables = saved_vars;

                // Register function
                self.functions.insert(go_fn_name.clone(), function);

                // Build captures vector
                let vector_empty_fn = self.module.get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;

                let mut captures_vec = self.builder.build_call(
                    vector_empty_fn,
                    &[],
                    "captures_vec"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                // Add each captured variable to vector
                if !free_vars.is_empty() {
                    let vector_conj_fn = self.module.get_function("clorus_vector_conj")
                        .ok_or("clorus_vector_conj not declared")?;

                    for var_name in &free_vars {
                        // Get variable value
                        let var_alloca = self.variables.get(var_name)
                            .ok_or_else(|| format!("Variable not found: {}", var_name))?;
                        let var_value = self.builder.build_load(
                            value_ptr_type,
                            *var_alloca,
                            var_name
                        ).unwrap().into_pointer_value();

                        // Add to vector
                        captures_vec = self.builder.build_call(
                            vector_conj_fn,
                            &[captures_vec.into(), var_value.into()],
                            &format!("capture_add_{}", var_name)
                        ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();
                    }
                }

                // Get function pointer
                let func_ptr = function.as_global_value().as_pointer_value();

                // Cast to i8* for clorus_go
                let func_val = self.builder.build_pointer_cast(
                    func_ptr,
                    value_ptr_type,
                    "go_func_ptr"
                ).unwrap();

                // Call clorus_go with function and captures
                let go_fn = self.module.get_function("clorus_go")
                    .ok_or("clorus_go not declared")?;

                let result = self.builder.build_call(
                    go_fn,
                    &[func_val.into(), captures_vec.into()],
                    "go_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "apply" => {
                // apply takes 2 args: function, collection
                if args.len() != 2 {
                    return Err("apply requires 2 arguments: function, collection".to_string());
                }

                // Compile the function expression - supports both named functions and closures
                let func_val = self.compile_expr(&args[0])?;

                // Compile the collection
                let coll_ptr = self.compile_expr(&args[1])?;

                // Get collection count
                let count_fn = self.module.get_function("clorus_count")
                    .ok_or("clorus_count not declared")?;
                let count_result = self.builder.build_call(
                    count_fn,
                    &[coll_ptr.into()],
                    "apply_count"
                ).unwrap();
                let count_i64 = count_result.try_as_basic_value().left().unwrap().into_int_value();

                // Get nth function
                let nth_fn = self.module.get_function("clorus_nth")
                    .ok_or("clorus_nth not declared")?;

                // Build argument vector by extracting each element
                // For now, support up to 10 arguments
                let max_args = 10usize;

                // Create a loop to extract arguments
                let current_fn = self.builder.get_insert_block().unwrap().get_parent().unwrap();
                let loop_block = self.context.append_basic_block(current_fn, "apply_loop");
                let body_block = self.context.append_basic_block(current_fn, "apply_body");
                let end_block = self.context.append_basic_block(current_fn, "apply_end");

                // Allocate space for argument array (fixed size for now)
                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let arg_array_type = value_ptr_type.array_type(max_args as u32);
                let arg_array = self.builder.build_alloca(arg_array_type, "arg_array").unwrap();

                // Index counter
                let index_alloca = self.builder.build_alloca(self.context.i64_type(), "apply_index").unwrap();
                self.builder.build_store(index_alloca, self.context.i64_type().const_zero()).unwrap();

                // Actual count storage
                let actual_count_alloca = self.builder.build_alloca(self.context.i64_type(), "actual_count").unwrap();
                self.builder.build_store(actual_count_alloca, count_i64).unwrap();

                self.builder.build_unconditional_branch(loop_block).unwrap();

                // Loop condition: index < count && index < max_args
                self.builder.position_at_end(loop_block);
                let current_index = self.builder.build_load(
                    self.context.i64_type(),
                    index_alloca,
                    "current_index"
                ).unwrap().into_int_value();

                let cond1 = self.builder.build_int_compare(
                    inkwell::IntPredicate::SLT,
                    current_index,
                    count_i64,
                    "cond1"
                ).unwrap();

                let max_args_const = self.context.i64_type().const_int(max_args as u64, false);
                let cond2 = self.builder.build_int_compare(
                    inkwell::IntPredicate::SLT,
                    current_index,
                    max_args_const,
                    "cond2"
                ).unwrap();

                let condition = self.builder.build_and(cond1, cond2, "loop_cond").unwrap();
                self.builder.build_conditional_branch(condition, body_block, end_block).unwrap();

                // Loop body: extract element and store in array
                self.builder.position_at_end(body_block);
                let elem = self.builder.build_call(
                    nth_fn,
                    &[coll_ptr.into(), current_index.into()],
                    "apply_elem"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                // Store in arg_array[index]
                let elem_ptr = unsafe {
                    self.builder.build_gep(
                        arg_array_type,
                        arg_array,
                        &[
                            self.context.i64_type().const_zero(),
                            current_index
                        ],
                        "elem_ptr"
                    ).unwrap()
                };
                self.builder.build_store(elem_ptr, elem).unwrap();

                // Increment index
                let next_index = self.builder.build_int_add(
                    current_index,
                    self.context.i64_type().const_int(1, false),
                    "next_index"
                ).unwrap();
                self.builder.build_store(index_alloca, next_index).unwrap();
                self.builder.build_unconditional_branch(loop_block).unwrap();

                // After loop: call function with extracted arguments using dynamic dispatch
                self.builder.position_at_end(end_block);

                // Load actual count
                let actual_count = self.builder.build_load(
                    self.context.i64_type(),
                    actual_count_alloca,
                    "actual_count"
                ).unwrap().into_int_value();

                // Cast arg_array to *const *mut Value for clorus_function_call
                let args_array_ptr = self.builder.build_pointer_cast(
                    arg_array,
                    value_ptr_type.ptr_type(AddressSpace::default()),
                    "args_array_cast"
                ).unwrap();

                // Cast count to i32 for clorus_function_call
                let count_i32 = self.builder.build_int_cast(
                    actual_count,
                    self.context.i32_type(),
                    "count_i32"
                ).unwrap();

                // Call clorus_function_call for dynamic dispatch
                let function_call_fn = self.module.get_function("clorus_function_call")
                    .ok_or("clorus_function_call not declared")?;

                let result = self.builder.build_call(
                    function_call_fn,
                    &[func_val.into(), args_array_ptr.into(), count_i32.into()],
                    "apply_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            // New Collection API functions

            "dissoc" => {
                if args.len() != 2 {
                    return Err("dissoc requires 2 arguments: map, key".to_string());
                }
                let map_val = self.compile_expr(&args[0])?;
                let key_val = self.compile_expr(&args[1])?;
                let dissoc_fn = self.module.get_function("clorus_map_dissoc")
                    .ok_or("clorus_map_dissoc not declared")?;
                let result = self.builder.build_call(dissoc_fn, &[map_val.into(), key_val.into()], "dissoc_call").unwrap();
                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "keys" => self.compile_simple_1arg_call("keys", "clorus_map_keys", args),

            "vals" => self.compile_simple_1arg_call("vals", "clorus_map_vals", args),

            "merge" => self.compile_simple_1arg_call("merge", "clorus_map_merge", args),

            "get-in" => self.compile_simple_2arg_call("get-in", "clorus_map_get_in", args),

            "assoc-in" => self.compile_simple_3arg_call("assoc-in", "clorus_map_assoc_in", args),

            "concat" => {
                if args.is_empty() {
                    // (concat) with no args returns empty vector
                    let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                        .ok_or("clorus_vector_empty not declared")?;
                    let result = self.builder.build_call(vec_empty_fn, &[], "empty_vec").unwrap();
                    return Ok(result.try_as_basic_value().left().unwrap().into_pointer_value());
                }

                // (concat coll1 coll2 ...) - concatenate multiple collections
                // For now, support 2 collections
                if args.len() == 2 {
                    let coll1 = self.compile_expr(&args[0])?;
                    let coll2 = self.compile_expr(&args[1])?;

                    // Build a vector containing the two collections
                    let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                        .ok_or("clorus_vector_empty not declared")?;
                    let vec_conj_fn = self.module.get_function("clorus_vector_conj")
                        .ok_or("clorus_vector_conj not declared")?;

                    let vec = self.builder.build_call(vec_empty_fn, &[], "concat_vec").unwrap()
                        .try_as_basic_value().left().unwrap().into_pointer_value();
                    let vec = self.builder.build_call(vec_conj_fn, &[vec.into(), coll1.into()], "vec1").unwrap()
                        .try_as_basic_value().left().unwrap().into_pointer_value();
                    let vec = self.builder.build_call(vec_conj_fn, &[vec.into(), coll2.into()], "vec2").unwrap()
                        .try_as_basic_value().left().unwrap().into_pointer_value();

                    let concat_fn = self.module.get_function("clorus_concat")
                        .ok_or("clorus_concat not declared")?;
                    let result = self.builder.build_call(concat_fn, &[vec.into()], "concat_call").unwrap();
                    Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
                } else {
                    // For more than 2 args, build a vector of all collections
                    let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                        .ok_or("clorus_vector_empty not declared")?;
                    let vec_conj_fn = self.module.get_function("clorus_vector_conj")
                        .ok_or("clorus_vector_conj not declared")?;

                    let mut vec = self.builder.build_call(vec_empty_fn, &[], "concat_vec").unwrap()
                        .try_as_basic_value().left().unwrap().into_pointer_value();

                    for arg in args {
                        let coll = self.compile_expr(arg)?;
                        vec = self.builder.build_call(vec_conj_fn, &[vec.into(), coll.into()], "vec_conj").unwrap()
                            .try_as_basic_value().left().unwrap().into_pointer_value();
                    }

                    let concat_fn = self.module.get_function("clorus_concat")
                        .ok_or("clorus_concat not declared")?;
                    let result = self.builder.build_call(concat_fn, &[vec.into()], "concat_call").unwrap();
                    Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
                }
            }

            "interleave" => self.compile_simple_1arg_call("interleave", "clorus_interleave", args),

            "interpose" => self.compile_simple_2arg_call("interpose", "clorus_interpose", args),

            "distinct" => self.compile_simple_1arg_call("distinct", "clorus_distinct", args),

            "dedupe" => self.compile_simple_1arg_call("dedupe", "clorus_dedupe", args),

            "flatten" => self.compile_simple_1arg_call("flatten", "clorus_flatten", args),

            // String operations
            "str" => {
                // str takes variable args and concatenates them
                // Create a vector of the arguments
                let vec_empty_fn = self.module.get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let mut vec_val = self.builder.build_call(
                    vec_empty_fn,
                    &[],
                    "str_vec"
                ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();

                // Add each argument to the vector
                let vec_conj_fn = self.module.get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;
                for arg in args {
                    let arg_val = self.compile_expr(arg)?;
                    vec_val = self.builder.build_call(
                        vec_conj_fn,
                        &[vec_val.into(), arg_val.into()],
                        "str_vec_conj"
                    ).unwrap().try_as_basic_value().left().unwrap().into_pointer_value();
                }

                // Call clorus_str with the vector
                let str_fn = self.module.get_function("clorus_str")
                    .ok_or("clorus_str not declared")?;
                let result = self.builder.build_call(
                    str_fn,
                    &[vec_val.into()],
                    "str_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "subs" => {
                // subs takes 2 or 3 args: string, start, [end]
                if args.len() < 2 || args.len() > 3 {
                    return Err("subs requires 2 or 3 arguments: string, start, [end]".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let start_val = self.compile_expr(&args[1])?;
                let start_float = self.unbox_number(start_val);
                let start_i64 = self.builder.build_float_to_signed_int(
                    start_float,
                    self.context.i64_type(),
                    "start_to_i64"
                ).unwrap();

                if args.len() == 2 {
                    // Call clorus_subs2
                    let subs_fn = self.module.get_function("clorus_subs2")
                        .ok_or("clorus_subs2 not declared")?;
                    let result = self.builder.build_call(
                        subs_fn,
                        &[str_val.into(), start_i64.into()],
                        "subs2_call"
                    ).unwrap();
                    Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
                } else {
                    // Call clorus_subs3
                    let end_val = self.compile_expr(&args[2])?;
                    let end_float = self.unbox_number(end_val);
                    let end_i64 = self.builder.build_float_to_signed_int(
                        end_float,
                        self.context.i64_type(),
                        "end_to_i64"
                    ).unwrap();

                    let subs_fn = self.module.get_function("clorus_subs3")
                        .ok_or("clorus_subs3 not declared")?;
                    let result = self.builder.build_call(
                        subs_fn,
                        &[str_val.into(), start_i64.into(), end_i64.into()],
                        "subs3_call"
                    ).unwrap();
                    Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
                }
            }

            "split" => {
                // split takes 2 args: string, delimiter
                if args.len() != 2 {
                    return Err("split requires 2 arguments: string, delimiter".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let delim_val = self.compile_expr(&args[1])?;

                let split_fn = self.module.get_function("clorus_split")
                    .ok_or("clorus_split not declared")?;
                let result = self.builder.build_call(
                    split_fn,
                    &[str_val.into(), delim_val.into()],
                    "split_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "join" => {
                // join takes 2 args: separator, collection
                if args.len() != 2 {
                    return Err("join requires 2 arguments: separator, collection".to_string());
                }

                let sep_val = self.compile_expr(&args[0])?;
                let coll_val = self.compile_expr(&args[1])?;

                let join_fn = self.module.get_function("clorus_join")
                    .ok_or("clorus_join not declared")?;
                let result = self.builder.build_call(
                    join_fn,
                    &[sep_val.into(), coll_val.into()],
                    "join_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "upper-case" => {
                // upper-case takes 1 arg: string
                if args.len() != 1 {
                    return Err("upper-case requires 1 argument: string".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;

                let upper_fn = self.module.get_function("clorus_upper_case")
                    .ok_or("clorus_upper_case not declared")?;
                let result = self.builder.build_call(
                    upper_fn,
                    &[str_val.into()],
                    "upper_case_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "lower-case" => {
                // lower-case takes 1 arg: string
                if args.len() != 1 {
                    return Err("lower-case requires 1 argument: string".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;

                let lower_fn = self.module.get_function("clorus_lower_case")
                    .ok_or("clorus_lower_case not declared")?;
                let result = self.builder.build_call(
                    lower_fn,
                    &[str_val.into()],
                    "lower_case_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "trim" => {
                // trim takes 1 arg: string
                if args.len() != 1 {
                    return Err("trim requires 1 argument: string".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;

                let trim_fn = self.module.get_function("clorus_trim")
                    .ok_or("clorus_trim not declared")?;
                let result = self.builder.build_call(
                    trim_fn,
                    &[str_val.into()],
                    "trim_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "trim-left" => {
                // trim-left takes 1 arg: string
                if args.len() != 1 {
                    return Err("trim-left requires 1 argument: string".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;

                let trim_fn = self.module.get_function("clorus_trim_left")
                    .ok_or("clorus_trim_left not declared")?;
                let result = self.builder.build_call(
                    trim_fn,
                    &[str_val.into()],
                    "trim_left_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "trim-right" => {
                // trim-right takes 1 arg: string
                if args.len() != 1 {
                    return Err("trim-right requires 1 argument: string".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;

                let trim_fn = self.module.get_function("clorus_trim_right")
                    .ok_or("clorus_trim_right not declared")?;
                let result = self.builder.build_call(
                    trim_fn,
                    &[str_val.into()],
                    "trim_right_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "replace" => {
                // replace takes 3 args: string, match, replacement
                if args.len() != 3 {
                    return Err("replace requires 3 arguments: string, match, replacement".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let match_val = self.compile_expr(&args[1])?;
                let repl_val = self.compile_expr(&args[2])?;

                let replace_fn = self.module.get_function("clorus_replace")
                    .ok_or("clorus_replace not declared")?;
                let result = self.builder.build_call(
                    replace_fn,
                    &[str_val.into(), match_val.into(), repl_val.into()],
                    "replace_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "replace-first" => {
                // replace-first takes 3 args: string, match, replacement
                if args.len() != 3 {
                    return Err("replace-first requires 3 arguments: string, match, replacement".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let match_val = self.compile_expr(&args[1])?;
                let repl_val = self.compile_expr(&args[2])?;

                let replace_fn = self.module.get_function("clorus_replace_first")
                    .ok_or("clorus_replace_first not declared")?;
                let result = self.builder.build_call(
                    replace_fn,
                    &[str_val.into(), match_val.into(), repl_val.into()],
                    "replace_first_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            // I/O operations
            "print" => {
                // print takes 1 arg: value to print
                if args.len() != 1 {
                    return Err("print requires 1 argument: value".to_string());
                }

                let val = self.compile_expr(&args[0])?;

                let print_fn = self.module.get_function("clorus_print")
                    .ok_or("clorus_print not declared")?;
                let result = self.builder.build_call(
                    print_fn,
                    &[val.into()],
                    "print_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "println" => {
                // println takes 1 arg: value to print
                if args.len() != 1 {
                    return Err("println requires 1 argument: value".to_string());
                }

                let val = self.compile_expr(&args[0])?;

                let println_fn = self.module.get_function("clorus_println")
                    .ok_or("clorus_println not declared")?;
                let result = self.builder.build_call(
                    println_fn,
                    &[val.into()],
                    "println_call"
                ).unwrap();

                Ok(result.try_as_basic_value().left().unwrap().into_pointer_value())
            }

            "string?" => {
                // string? takes 1 arg: value
                if args.len() != 1 {
                    return Err("string? requires 1 argument: value".to_string());
                }

                let val = self.compile_expr(&args[0])?;

                let is_string_fn = self.module.get_function("clorus_is_string")
                    .ok_or("clorus_is_string not declared")?;
                let result = self.builder.build_call(
                    is_string_fn,
                    &[val.into()],
                    "is_string_call"
                ).unwrap();

                // Convert i32 bool to Value* bool
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "bool_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "starts-with?" => {
                // starts-with? takes 2 args: string, prefix
                if args.len() != 2 {
                    return Err("starts-with? requires 2 arguments: string, prefix".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let prefix_val = self.compile_expr(&args[1])?;

                let starts_with_fn = self.module.get_function("clorus_starts_with")
                    .ok_or("clorus_starts_with not declared")?;
                let result = self.builder.build_call(
                    starts_with_fn,
                    &[str_val.into(), prefix_val.into()],
                    "starts_with_call"
                ).unwrap();

                // Convert i32 bool to Value* bool
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "bool_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "ends-with?" => {
                // ends-with? takes 2 args: string, suffix
                if args.len() != 2 {
                    return Err("ends-with? requires 2 arguments: string, suffix".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let suffix_val = self.compile_expr(&args[1])?;

                let ends_with_fn = self.module.get_function("clorus_ends_with")
                    .ok_or("clorus_ends_with not declared")?;
                let result = self.builder.build_call(
                    ends_with_fn,
                    &[str_val.into(), suffix_val.into()],
                    "ends_with_call"
                ).unwrap();

                // Convert i32 bool to Value* bool
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "bool_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            "includes?" => {
                // includes? takes 2 args: string, substring
                if args.len() != 2 {
                    return Err("includes? requires 2 arguments: string, substring".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let substr_val = self.compile_expr(&args[1])?;

                let includes_fn = self.module.get_function("clorus_includes")
                    .ok_or("clorus_includes not declared")?;
                let result = self.builder.build_call(
                    includes_fn,
                    &[str_val.into(), substr_val.into()],
                    "includes_call"
                ).unwrap();

                // Convert i32 bool to Value* bool
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self.builder.build_unsigned_int_to_float(
                    i32_result,
                    self.context.f64_type(),
                    "bool_to_float"
                ).unwrap();

                Ok(self.box_number(float_result))
            }

            _ => Err(format!("Unknown clorus.core function: {}", func)),
        }
    }

    /// Wrap expression in a function so we can JIT execute it
    pub fn wrap_in_function(&mut self, expr: &Expr, fn_name: &str) -> Result<FunctionValue<'ctx>, String> {
        // Function returns Value* now
        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = value_ptr_type.fn_type(&[], false);
        let function = self.module.add_function(fn_name, fn_type, None);

        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let result = self.compile_expr(expr)?;  // Returns Value*
        self.builder.build_return(Some(&result)).unwrap();

        Ok(function)
    }

    pub fn print_ir(&self) {
        println!("{}", self.module.print_to_string().to_string());
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
            format!("clorus_{}_{}",
                self.namespace.current.replace('.', "_"),
                name.replace('-', "_"))
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
}
