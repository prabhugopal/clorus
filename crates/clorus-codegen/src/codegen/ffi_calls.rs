use super::*;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_multimethod_dispatch_key(
        &mut self,
        dispatch_expr: &Expr,
        arg_values: &[PointerValue<'ctx>],
    ) -> Result<PointerValue<'ctx>, String> {
        // Common Clojure fast-path: keyword dispatch like :type.
        if let Expr::Keyword(k) = dispatch_expr {
            if arg_values.is_empty() {
                let nil_fn = self
                    .module
                    .get_function("clorus_value_nil")
                    .ok_or("clorus_value_nil not declared")?;
                return Ok(self
                    .builder
                    .build_call(nil_fn, &[], "mm_noarg_dispatch_nil")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value());
            }
            let keyword_fn = self
                .module
                .get_function("clorus_keyword")
                .ok_or("clorus_keyword not declared")?;
            let map_get_fn = self
                .module
                .get_function("clorus_map_get")
                .ok_or("clorus_map_get not declared")?;
            let kw_str = self
                .builder
                .build_global_string_ptr(k, "mm_dispatch_kw")
                .unwrap();
            let kw_val = self
                .builder
                .build_call(
                    keyword_fn,
                    &[kw_str.as_pointer_value().into()],
                    "mm_dispatch_kw_val",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();
            let out = self
                .builder
                .build_call(
                    map_get_fn,
                    &[arg_values[0].into(), kw_val.into()],
                    "mm_dispatch_map_get",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();
            return Ok(out);
        }

        let dispatch_fn_val = self.compile_expr(dispatch_expr)?;
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let arg_count = arg_values.len();
        let args_array_ptr = if arg_count > 0 {
            let array_type = i8_ptr_type.array_type(arg_count as u32);
            let array_alloca = self
                .builder
                .build_alloca(array_type, "mm_dispatch_args")
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
                            &format!("mm_dispatch_arg_{}_ptr", i),
                        )
                        .unwrap()
                };
                self.builder.build_store(elem_ptr, *arg_val).unwrap();
            }
            self.builder
                .build_pointer_cast(
                    array_alloca,
                    i8_ptr_type.ptr_type(AddressSpace::default()),
                    "mm_dispatch_args_cast",
                )
                .unwrap()
        } else {
            i8_ptr_type.ptr_type(AddressSpace::default()).const_null()
        };

        let function_call_fn = self
            .module
            .get_function("clorus_function_call")
            .ok_or("clorus_function_call not declared")?;
        let arg_count_val = self.context.i32_type().const_int(arg_count as u64, false);
        let out = self
            .builder
            .build_call(
                function_call_fn,
                &[
                    dispatch_fn_val.into(),
                    args_array_ptr.into(),
                    arg_count_val.into(),
                ],
                "mm_dispatch_call",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        Ok(out)
    }

    /// Compile a Rust FFI library function call generically based on metadata
    pub(super) fn compile_rust_library_call(
        &mut self,
        lib: &RustLibrary,
        func_name: &str,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
        // Convert kebab-case to snake_case for lookup (Clorus uses kebab, Rust uses snake)
        let rust_func_name = func_name.replace('-', "_");

        // Find the function metadata
        let func_meta = lib
            .functions
            .iter()
            .find(|f| f.name == rust_func_name)
            .ok_or_else(|| format!("Function {} not found in library {}", func_name, lib.name))?;

        // Check argument count
        if args.len() != func_meta.params.len() {
            return Err(format!(
                "{}/{} requires {} arguments, got {}",
                lib.name,
                func_name,
                func_meta.params.len(),
                args.len()
            ));
        }

        // Compile and convert arguments based on parameter types
        let mut ffi_args = Vec::new();
        let mut owned_cstrings_to_free = Vec::new();
        for (arg_expr, param) in args.iter().zip(&func_meta.params) {
            let arg_val = self.compile_expr(arg_expr)?;

            let ffi_arg = match param.type_name.as_str() {
                "String" => {
                    // Extract C string from Value*. clorus_value_as_cstring allocates a
                    // fresh CString for this call; it must be freed after the FFI call.
                    let cstr_ptr = self.extract_cstring_from_value(arg_val);
                    owned_cstrings_to_free.push(cstr_ptr);
                    cstr_ptr.into()
                }
                "f32" => {
                    let f64_val = self.unbox_number(arg_val);
                    self.builder
                        .build_float_trunc(f64_val, self.context.f32_type(), "f64_to_f32")
                        .unwrap()
                        .into()
                }
                "f64" => {
                    // Unbox number from Value*
                    self.unbox_number(arg_val).into()
                }
                "i32" => {
                    // Unbox number and convert to i32
                    let f64_val = self.unbox_number(arg_val);
                    self.builder
                        .build_float_to_signed_int(f64_val, self.context.i32_type(), "f64_to_i32")
                        .unwrap()
                        .into()
                }
                "u32" => {
                    let f64_val = self.unbox_number(arg_val);
                    self.builder
                        .build_float_to_unsigned_int(f64_val, self.context.i32_type(), "f64_to_u32")
                        .unwrap()
                        .into()
                }
                "i64" | "isize" => {
                    let f64_val = self.unbox_number(arg_val);
                    self.builder
                        .build_float_to_signed_int(f64_val, self.context.i64_type(), "f64_to_i64")
                        .unwrap()
                        .into()
                }
                "u64" | "usize" => {
                    let f64_val = self.unbox_number(arg_val);
                    self.builder
                        .build_float_to_unsigned_int(f64_val, self.context.i64_type(), "f64_to_u64")
                        .unwrap()
                        .into()
                }
                "bool" => {
                    // Extract from the Bool-tagged Value directly. Do NOT route
                    // through unbox_number: clorus_value_as_number returns 0.0
                    // for any non-numeric tag, including Bool, which would
                    // silently collapse both true and false to false.
                    self.extract_bool_from_value(arg_val).into()
                }
                "*mut u8" | "*const u8" => {
                    // Extract pointer from Value*
                    self.extract_pointer_from_value(arg_val).into()
                }
                other if Self::is_opaque_ffi_pointer_type_name(other) => {
                    // Typed pointer/reference to a named Rust type (*mut T, *const T,
                    // &T, &mut T). Carried as a plain pointer; the generated wrapper
                    // casts/derefs it back to the exact Rust type.
                    self.extract_pointer_from_value(arg_val).into()
                }
                other if Self::option_ffi_inner_type(other).is_some() => {
                    let inner = Self::option_ffi_inner_type(other).unwrap();
                    self.compile_option_ffi_param(arg_val, &inner, &mut owned_cstrings_to_free)?
                        .into()
                }
                other => return Err(format!("Unsupported parameter type: {}", other)),
            };

            ffi_args.push(ffi_arg);
        }

        // Call the FFI function
        let ffi_func_name = Self::rust_ffi_symbol_name(&lib.name, &rust_func_name);
        let ffi_func = self.module.get_function(&ffi_func_name).ok_or_else(|| {
            format!(
                "FFI function {} not found - did you (use rust.{})?",
                ffi_func_name, lib.name
            )
        })?;

        let call_result = self
            .builder
            .build_call(ffi_func, &ffi_args, &format!("call_{}", rust_func_name))
            .unwrap();

        // Free the C strings allocated for string arguments now that the call is done.
        for cstr_ptr in owned_cstrings_to_free {
            self.free_c_string(cstr_ptr);
        }

        // Convert return value based on return type
        match func_meta.return_type.as_str() {
            "String" => {
                // Wrapper returns an owned *mut c_char (CString::into_raw()); box it
                // and free the source buffer.
                let str_ptr = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(self.box_owned_c_string(str_ptr))
            }
            "f32" => {
                let f32_val = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value();
                let f64_val = self
                    .builder
                    .build_float_ext(f32_val, self.context.f64_type(), "f32_to_f64")
                    .unwrap();
                Ok(self.box_number(f64_val))
            }
            "f64" => {
                let f64_val = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value();
                Ok(self.box_number(f64_val))
            }
            "i32" => {
                let i32_val = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let f64_val = self
                    .builder
                    .build_signed_int_to_float(i32_val, self.context.f64_type(), "i32_to_f64")
                    .unwrap();
                Ok(self.box_number(f64_val))
            }
            "u32" => {
                let u32_val = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let f64_val = self
                    .builder
                    .build_unsigned_int_to_float(u32_val, self.context.f64_type(), "u32_to_f64")
                    .unwrap();
                Ok(self.box_number(f64_val))
            }
            "i64" | "isize" => {
                let i64_val = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let f64_val = self
                    .builder
                    .build_signed_int_to_float(i64_val, self.context.f64_type(), "i64_to_f64")
                    .unwrap();
                Ok(self.box_number(f64_val))
            }
            "u64" | "usize" => {
                let u64_val = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let f64_val = self
                    .builder
                    .build_unsigned_int_to_float(u64_val, self.context.f64_type(), "u64_to_f64")
                    .unwrap();
                Ok(self.box_number(f64_val))
            }
            "bool" => {
                // Box as a genuine Bool-tagged Value, not a Number: boxing as
                // a Number would make a `false` return truthy (0.0 is truthy
                // in Clorus, matching Clojure -- only nil/false are falsy).
                let bool_val = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let f64_val = self
                    .builder
                    .build_unsigned_int_to_float(bool_val, self.context.f64_type(), "bool_to_f64")
                    .unwrap();
                let value_bool_fn = self
                    .module
                    .get_function("clorus_value_bool")
                    .ok_or("clorus_value_bool not declared")?;
                Ok(self
                    .builder
                    .build_call(value_bool_fn, &[f64_val.into()], "box_bool")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }
            "()" => {
                // Void return - return nil (0.0)
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }
            "*mut u8" | "*const u8" => {
                // Box pointer return value
                let ptr = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(self.box_pointer(ptr))
            }
            other if Self::is_opaque_ffi_pointer_type_name(other) => {
                // Typed pointer to a named Rust type (*mut T / *const T). Bare
                // reference return types are rejected upstream, before codegen.
                let ptr = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(self.box_pointer(ptr))
            }
            other if Self::option_ffi_inner_type(other).is_some() => {
                let inner = Self::option_ffi_inner_type(other).unwrap();
                let raw_ptr = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                self.compile_option_ffi_return(raw_ptr, &inner)
            }
            other if Self::result_ffi_inner_types(other).is_some() => {
                let (ok_ty, _err_ty) = Self::result_ffi_inner_types(other).unwrap();
                let raw_ptr = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                self.compile_result_ffi_return(raw_ptr, &ok_ty, &lib.name)
            }
            other => Err(format!("Unsupported return type: {}", other)),
        }
    }

    /// Rust primitive type names -- a pointer/reference to one of these is
    /// deliberately not treated as an opaque handle (kept in sync with
    /// clorus-cli's rust_ffi::RustFfiProcessor::FFI_PRIMITIVE_TYPE_NAMES,
    /// the source of truth for which type-name strings reach codegen).
    const FFI_PRIMITIVE_TYPE_NAMES: &'static [&'static str] = &[
        "f32", "f64", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "isize", "usize",
        "bool", "str", "()",
    ];

    /// Recognizes `*mut Name`, `*const Name`, `&Name`, `&mut Name` for an
    /// arbitrary named (non-primitive) Rust type -- all carried as a plain
    /// opaque pointer at the C ABI boundary. Mirrors clorus-cli's rust_ffi
    /// analyzer, which is the source of these type-name strings.
    pub(super) fn is_opaque_ffi_pointer_type_name(type_name: &str) -> bool {
        let pointee = type_name
            .strip_prefix("*mut ")
            .or_else(|| type_name.strip_prefix("*const "))
            .or_else(|| type_name.strip_prefix("&mut "))
            .or_else(|| type_name.strip_prefix('&'));
        match pointee {
            Some(name) => {
                !name.is_empty()
                    && !Self::FFI_PRIMITIVE_TYPE_NAMES.contains(&name)
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    && name.chars().next().is_some_and(|c| !c.is_ascii_digit())
            }
            None => false,
        }
    }

    /// Strips `Wrapper<...>` down to the inner `...` (mirrors clorus-cli's
    /// rust_ffi::RustFfiProcessor::strip_generic_wrapper).
    fn strip_generic_wrapper<'a>(type_name: &'a str, wrapper: &str) -> Option<&'a str> {
        type_name
            .strip_prefix(wrapper)?
            .strip_prefix('<')?
            .strip_suffix('>')
    }

    fn split_top_level_comma(s: &str) -> Option<(String, String)> {
        let mut depth = 0i32;
        for (i, c) in s.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => depth -= 1,
                ',' if depth == 0 => {
                    return Some((s[..i].trim().to_string(), s[i + 1..].trim().to_string()));
                }
                _ => {}
            }
        }
        None
    }

    pub(super) fn option_ffi_inner_type(type_name: &str) -> Option<String> {
        Self::strip_generic_wrapper(type_name, "Option").map(|s| s.trim().to_string())
    }

    pub(super) fn result_ffi_inner_types(type_name: &str) -> Option<(String, String)> {
        Self::strip_generic_wrapper(type_name, "Result").and_then(Self::split_top_level_comma)
    }

    /// Compiles an `Option<T>` (T pointer-shaped) FFI argument: nil becomes
    /// a null pointer, any other value is converted the same way a plain T
    /// argument would be. Both arms produce an i8* so a phi can merge them.
    fn compile_option_ffi_param(
        &mut self,
        arg_val: PointerValue<'ctx>,
        inner_type: &str,
        owned_cstrings_to_free: &mut Vec<PointerValue<'ctx>>,
    ) -> Result<PointerValue<'ctx>, String> {
        let is_nil_fn = self
            .module
            .get_function("clorus_value_is_nil")
            .ok_or("clorus_value_is_nil not declared")?;
        let is_nil_i32 = self
            .builder
            .build_call(is_nil_fn, &[arg_val.into()], "opt_param_is_nil_i32")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();
        let is_nil = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::NE,
                is_nil_i32,
                self.context.i32_type().const_zero(),
                "opt_param_is_nil",
            )
            .unwrap();

        let function = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .expect("no parent function");
        let none_bb = self.context.append_basic_block(function, "opt_param_none");
        let some_bb = self.context.append_basic_block(function, "opt_param_some");
        let merge_bb = self.context.append_basic_block(function, "opt_param_merge");

        self.builder
            .build_conditional_branch(is_nil, none_bb, some_bb)
            .unwrap();

        self.builder.position_at_end(none_bb);
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let none_val = i8_ptr_type.const_null();
        self.builder.build_unconditional_branch(merge_bb).unwrap();
        let none_bb_end = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(some_bb);
        let some_val = if matches!(inner_type, "String" | "&str") {
            self.extract_cstring_from_value(arg_val)
        } else {
            self.extract_pointer_from_value(arg_val)
        };
        self.builder.build_unconditional_branch(merge_bb).unwrap();
        let some_bb_end = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(i8_ptr_type, "opt_param_result")
            .unwrap();
        phi.add_incoming(&[(&none_val, none_bb_end), (&some_val, some_bb_end)]);
        let result = phi.as_basic_value().into_pointer_value();

        if matches!(inner_type, "String" | "&str") {
            // Freeing a null pointer is a documented no-op in clorus_free_cstring,
            // so pushing the merged (possibly-null) result is safe either way.
            owned_cstrings_to_free.push(result);
        }

        Ok(result)
    }

    /// Boxes a raw FFI return pointer as a Value*, treating it as if
    /// `inner_type` were the function's own (non-Option/Result) return type.
    /// Used for the Some/Ok arm of Option<T>/Result<T,E> returns.
    fn box_ffi_return_value_for_type(
        &mut self,
        ptr: PointerValue<'ctx>,
        inner_type: &str,
    ) -> PointerValue<'ctx> {
        if matches!(inner_type, "String" | "&str") {
            self.box_owned_c_string(ptr)
        } else {
            self.box_pointer(ptr)
        }
    }

    /// Compiles an `Option<T>` (T pointer-shaped) FFI return value: a null
    /// pointer becomes Clorus nil, anything else is unboxed as if the
    /// function had simply returned T.
    fn compile_option_ffi_return(
        &mut self,
        raw_ptr: PointerValue<'ctx>,
        inner_type: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let is_null = self
            .builder
            .build_is_null(raw_ptr, "opt_ret_is_null")
            .unwrap();

        let function = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .expect("no parent function");
        let none_bb = self.context.append_basic_block(function, "opt_ret_none");
        let some_bb = self.context.append_basic_block(function, "opt_ret_some");
        let merge_bb = self.context.append_basic_block(function, "opt_ret_merge");

        self.builder
            .build_conditional_branch(is_null, none_bb, some_bb)
            .unwrap();

        self.builder.position_at_end(none_bb);
        let nil_fn = self
            .module
            .get_function("clorus_value_nil")
            .ok_or("clorus_value_nil not declared")?;
        let nil_val = self
            .builder
            .build_call(nil_fn, &[], "opt_ret_nil")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        self.builder.build_unconditional_branch(merge_bb).unwrap();
        let none_bb_end = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(some_bb);
        let some_val = self.box_ffi_return_value_for_type(raw_ptr, inner_type);
        self.builder.build_unconditional_branch(merge_bb).unwrap();
        let some_bb_end = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(i8_ptr_type, "opt_ret_result")
            .unwrap();
        phi.add_incoming(&[(&nil_val, none_bb_end), (&some_val, some_bb_end)]);
        Ok(phi.as_basic_value().into_pointer_value())
    }

    /// Compiles a `Result<T, E>` (T pointer-shaped) FFI return value: a null
    /// pointer means the wrapper hit `Err` and stashed a message in its
    /// thread-local error slot, retrieved here and raised as a genuine
    /// Clorus exception (the same mechanism `(throw ...)` uses). A non-null
    /// pointer is unboxed as if the function had simply returned T.
    fn compile_result_ffi_return(
        &mut self,
        raw_ptr: PointerValue<'ctx>,
        ok_type: &str,
        lib_name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let is_null = self
            .builder
            .build_is_null(raw_ptr, "result_ret_is_null")
            .unwrap();

        let function = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .expect("no parent function");
        let err_bb = self.context.append_basic_block(function, "result_ret_err");
        let ok_bb = self.context.append_basic_block(function, "result_ret_ok");
        let merge_bb = self.context.append_basic_block(function, "result_ret_merge");

        self.builder
            .build_conditional_branch(is_null, err_bb, ok_bb)
            .unwrap();

        self.builder.position_at_end(err_bb);
        let take_err_name = Self::rust_ffi_symbol_name(lib_name, "take_last_error");
        let take_err_fn = self
            .module
            .get_function(&take_err_name)
            .ok_or_else(|| format!("{} not declared", take_err_name))?;
        let err_cstr = self
            .builder
            .build_call(take_err_fn, &[], "result_ret_take_err")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        let err_msg_val = self.box_owned_c_string(err_cstr);
        let wrap_fn = self
            .module
            .get_function("clorus_value_exception")
            .ok_or("clorus_value_exception not declared")?;
        let exception_val = self
            .builder
            .build_call(wrap_fn, &[err_msg_val.into()], "result_ret_exception")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();
        self.builder.build_unconditional_branch(merge_bb).unwrap();
        let err_bb_end = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(ok_bb);
        let ok_val = self.box_ffi_return_value_for_type(raw_ptr, ok_type);
        self.builder.build_unconditional_branch(merge_bb).unwrap();
        let ok_bb_end = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(i8_ptr_type, "result_ret_result")
            .unwrap();
        phi.add_incoming(&[(&exception_val, err_bb_end), (&ok_val, ok_bb_end)]);
        Ok(phi.as_basic_value().into_pointer_value())
    }

    /// Compile a Rust FFI function call using canonical FfiFunction type
    /// This is the modern, type-safe version that handles pointers correctly
    pub(super) fn compile_ffi_function_call(
        &mut self,
        func: &FfiFunction,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
        // Check argument count
        if args.len() != func.params.len() {
            return Err(format!(
                "{} requires {} arguments, got {}",
                func.name,
                func.params.len(),
                args.len()
            ));
        }

        // Compile and convert arguments based on canonical FFI types
        let mut ffi_args = Vec::new();
        let mut owned_cstrings_to_free = Vec::new();
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
                    self.builder
                        .build_float_to_signed_int(f64_val, self.context.i64_type(), "f64_to_i64")
                        .unwrap()
                        .into()
                }
                FfiType::Bool => {
                    // Unbox number and convert to bool (non-zero = true)
                    let f64_val = self.unbox_number(arg_val);
                    let zero = self.context.f64_type().const_float(0.0);
                    self.builder
                        .build_float_compare(
                            FloatPredicate::ONE, // Ordered and Not Equal
                            f64_val,
                            zero,
                            "f64_to_bool",
                        )
                        .unwrap()
                        .into()
                }
                FfiType::String => {
                    // Extract C string from Value*. clorus_value_as_cstring allocates a
                    // fresh CString for this call; it must be freed after the FFI call.
                    let cstr_ptr = self.extract_cstring_from_value(arg_val);
                    owned_cstrings_to_free.push(cstr_ptr);
                    cstr_ptr.into()
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
                    return Err(format!(
                        "Unsupported parameter type: {}",
                        param.ty.display_name()
                    ));
                }
            };

            ffi_args.push(ffi_arg);
        }

        // Call the FFI function
        let ffi_func_name = format!("clorus_{}", func.name);
        let ffi_func = self
            .module
            .get_function(&ffi_func_name)
            .ok_or_else(|| format!("FFI function {} not found", ffi_func_name))?;

        let call_result = self
            .builder
            .build_call(ffi_func, &ffi_args, &format!("call_{}", func.name))
            .unwrap();

        // Free the C strings allocated for string arguments now that the call is done.
        for cstr_ptr in owned_cstrings_to_free {
            self.free_c_string(cstr_ptr);
        }

        // Convert return value based on canonical FFI type
        match &func.return_type {
            FfiType::Void => {
                // Void return - return nil (0.0)
                Ok(self.box_number(self.context.f64_type().const_float(0.0)))
            }
            FfiType::F64 => {
                let f64_val = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value();
                Ok(self.box_number(f64_val))
            }
            FfiType::I64 => {
                let i64_val = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let f64_val = self
                    .builder
                    .build_signed_int_to_float(i64_val, self.context.f64_type(), "i64_to_f64")
                    .unwrap();
                Ok(self.box_number(f64_val))
            }
            FfiType::Bool => {
                let bool_val = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let f64_val = self
                    .builder
                    .build_unsigned_int_to_float(bool_val, self.context.f64_type(), "bool_to_f64")
                    .unwrap();
                Ok(self.box_number(f64_val))
            }
            FfiType::String => {
                // Wrapper returns an owned *mut c_char (CString::into_raw()); box it
                // and free the source buffer.
                let str_ptr = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(self.box_owned_c_string(str_ptr))
            }
            FfiType::OpaquePointer { .. } => {
                // Box raw pointer into Value*
                let ptr = call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(self.box_pointer(ptr))
            }
            _ => Err(format!(
                "Unsupported return type: {}",
                func.return_type.display_name()
            )),
        }
    }
}
