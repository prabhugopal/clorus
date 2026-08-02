use super::*;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn declare_runtime_functions(&mut self) {
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
        self.declare_value_fn("clorus_commute", 3);
        self.declare_value_fn("clorus_ensure", 1);

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
    pub(super) fn declare_core_functions(&mut self) {
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
    pub(super) fn create_value_from_float(
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
    pub(super) fn extract_float_from_value(
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
    pub(super) fn box_number(&self, float_val: FloatValue<'ctx>) -> PointerValue<'ctx> {
        self.create_value_from_float(float_val)
    }

    /// Helper: Unbox a Value* to f64 (alias for consistency)
    pub(super) fn unbox_number(&self, value_ptr: PointerValue<'ctx>) -> FloatValue<'ctx> {
        self.extract_float_from_value(value_ptr)
    }

    /// Helper: Box a C string pointer into Value*
    pub(super) fn box_string(&self, str_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
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
    pub(super) fn extract_cstring_from_value(&self, value_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
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
    pub(super) fn call_runtime_fn(
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
    pub(super) fn call_runtime_fn_unchecked(
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
    pub(super) fn declare_void_fn(&mut self, name: &str, num_args: usize) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let args: Vec<_> = (0..num_args).map(|_| i8_ptr_type.into()).collect();
        let fn_type = self.context.void_type().fn_type(&args, false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* args and returns Value*
    pub(super) fn declare_value_fn(&mut self, name: &str, num_args: usize) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let args: Vec<_> = (0..num_args).map(|_| i8_ptr_type.into()).collect();
        let fn_type = i8_ptr_type.fn_type(&args, false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns i64
    pub(super) fn declare_value_to_i64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i64_type()
            .fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and i64, returns Value*
    pub(super) fn declare_value_i64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes no args and returns Value*
    pub(super) fn declare_no_arg_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes i64 and returns Value*
    pub(super) fn declare_i64_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[self.context.i64_type().into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes f64 and returns Value*
    pub(super) fn declare_f64_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[self.context.f64_type().into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes bool and returns Value*
    pub(super) fn declare_bool_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[self.context.bool_type().into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes i8* pointer and returns Value*
    pub(super) fn declare_ptr_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns i8* pointer
    pub(super) fn declare_value_to_ptr_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes no args and returns bool
    pub(super) fn declare_no_arg_bool_fn(&mut self, name: &str) {
        let fn_type = self.context.bool_type().fn_type(&[], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes no args and returns void
    pub(super) fn declare_no_arg_void_fn(&mut self, name: &str) {
        let fn_type = self.context.void_type().fn_type(&[], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns f64
    pub(super) fn declare_value_to_f64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .f64_type()
            .fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns i32 (typically bool)
    pub(super) fn declare_value_to_i32_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and returns C bool (i8)
    /// Rust `extern "C" fn(...) -> bool` uses C `_Bool` ABI, which is byte-sized.
    pub(super) fn declare_value_to_bool_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self.context.i8_type().fn_type(&[i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes 2 Value* args and returns i32 (typically bool)
    pub(super) fn declare_value2_to_i32_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i32_type()
            .fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes 2 Value* args and returns i64
    pub(super) fn declare_value2_to_i64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i64_type()
            .fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes 2 Value* args and returns C bool (i8)
    pub(super) fn declare_value2_to_bool_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = self
            .context
            .i8_type()
            .fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes 2 Value* args and returns Value*
    pub(super) fn declare_value2_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and i64, returns Value*
    pub(super) fn declare_value_and_i64_to_value_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
        self.module.add_function(name, fn_type, None);
    }

    /// Declare a function that takes Value* and 2 i64s, returns Value*
    pub(super) fn declare_value_i64_i64_fn(&mut self, name: &str) {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i8_ptr_type.fn_type(
            &[i8_ptr_type.into(), i64_type.into(), i64_type.into()],
            false,
        );
        self.module.add_function(name, fn_type, None);
    }

}
