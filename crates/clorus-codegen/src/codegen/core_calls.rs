use super::*;

impl<'ctx> CodeGen<'ctx> {
    /// Helper: compile string expression to C string pointer.
    /// Handles both string literals and values that evaluate to strings.
    pub(super) fn compile_string_to_ptr(
        &mut self,
        expr: &Expr,
    ) -> Result<PointerValue<'ctx>, String> {
        match expr {
            Expr::String(s) => {
                let c_str = self.builder.build_global_string_ptr(s, "str").unwrap();
                Ok(c_str.as_pointer_value())
            }
            Expr::Symbol(_) => {
                let value_ptr = self.compile_expr(expr)?;
                Ok(self.extract_cstring_from_value(value_ptr))
            }
            _ => Err("Expected string literal or string variable".to_string()),
        }
    }

    /// Compile clorus.core function calls (Clojure-style convenience functions)
    pub(super) fn compile_core_call(
        &mut self,
        func: &str,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
        // Numeric helpers present in clorus.core and frequently used qualified:
        // (inc x), (dec x), (zero? x)
        match func {
            "inc" => {
                if args.len() != 1 {
                    return Err("inc requires 1 argument".to_string());
                }
                let rewritten = vec![args[0].clone(), Expr::Long(1)];
                return self.compile_add(&rewritten);
            }
            "dec" => {
                if args.len() != 1 {
                    return Err("dec requires 1 argument".to_string());
                }
                let rewritten = vec![args[0].clone(), Expr::Long(1)];
                return self.compile_sub(&rewritten);
            }
            "zero?" => {
                if args.len() != 1 {
                    return Err("zero? requires 1 argument".to_string());
                }
                let rewritten = vec![args[0].clone(), Expr::Long(0)];
                return self.compile_eq(&rewritten);
            }
            _ => {}
        }

        if let Some(result) = self.compile_simple_mapped_call(func, args) {
            return result;
        }

        if let Some(runtime_predicate) = match func {
            "string?" | "__clorus_is_string" => Some("clorus_is_string_i32"),
            "number?" | "__clorus_is_number" => Some("clorus_is_number_i32"),
            "vector?" | "__clorus_is_vector" => Some("clorus_is_vector_i32"),
            "list?" | "__clorus_is_list" => Some("clorus_is_list_i32"),
            "map?" | "__clorus_is_map" => Some("clorus_is_map_i32"),
            "set?" | "__clorus_is_set" => Some("clorus_is_set_i32"),
            "keyword?" | "__clorus_is_keyword" => Some("clorus_is_keyword_i32"),
            "symbol?" | "__clorus_is_symbol" => Some("clorus_is_symbol_i32"),
            "nil?" | "__clorus_is_nil" => Some("clorus_is_nil_i32"),
            "boolean?" | "bool?" | "__clorus_is_bool" => Some("clorus_is_bool_i32"),
            "seq?" | "__clorus_is_seq" => Some("clorus_is_seq_i32"),
            "coll?" | "__clorus_is_coll" => Some("clorus_is_coll_i32"),
            "fn?" | "__clorus_is_fn" => Some("clorus_is_fn_i32"),
            _ => None,
        } {
            return self.compile_unary_predicate_call(func, runtime_predicate, args);
        }

        if func == "isa?" {
            return self.compile_binary_predicate_call("isa?", "clorus_isa_i32", args);
        }

        if func == "satisfies?" || func == "extends?" || func == "implements?" {
            if args.len() != 2 {
                return Err(format!(
                    "{} requires 2 arguments: protocol, type-or-value",
                    func
                ));
            }

            let protocol_name_raw = match &args[0] {
                Expr::Symbol(s) | Expr::String(s) | Expr::Keyword(s) => s.clone(),
                _ => {
                    return Err(format!(
                        "{} requires a protocol symbol/string/keyword as first argument",
                        func
                    ));
                }
            };
            let protocol_name = protocol_name_raw
                .split('/')
                .last()
                .unwrap_or(&protocol_name_raw)
                .to_string();

            let keyword_fn = self
                .module
                .get_function("clorus_keyword")
                .ok_or("clorus_keyword not declared")?;
            let map_get_fn = self
                .module
                .get_function("clorus_map_get")
                .ok_or("clorus_map_get not declared")?;
            let string_data_fn = self
                .module
                .get_function("clorus_string_data")
                .ok_or("clorus_string_data not declared")?;
            let satisfies_fn = self
                .module
                .get_function("clorus_protocol_satisfies_type_i32")
                .ok_or("clorus_protocol_satisfies_type_i32 not declared")?;
            let bool_fn = self
                .module
                .get_function("clorus_value_boolean")
                .ok_or("clorus_value_boolean not declared")?;
            let type_cstr = match &args[1] {
                Expr::Symbol(s) => {
                    let ty_name = s.split('/').last().unwrap_or(s);
                    let ty_name_str = self
                        .builder
                        .build_global_string_ptr(ty_name, "protocol_type_name")
                        .unwrap();
                    ty_name_str.as_pointer_value()
                }
                Expr::String(s) | Expr::Keyword(s) => {
                    let ty_name_str = self
                        .builder
                        .build_global_string_ptr(s, "protocol_type_name")
                        .unwrap();
                    ty_name_str.as_pointer_value()
                }
                _ => {
                    let val = self.compile_expr(&args[1])?;
                    let type_key_str = self
                        .builder
                        .build_global_string_ptr("__type__", "satisfies_type_key")
                        .unwrap();
                    let type_key = self
                        .builder
                        .build_call(
                            keyword_fn,
                            &[type_key_str.as_pointer_value().into()],
                            "satisfies_type_key_kw",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let type_val = self
                        .builder
                        .build_call(
                            map_get_fn,
                            &[val.into(), type_key.into()],
                            "satisfies_type_val",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    self.builder
                        .build_call(string_data_fn, &[type_val.into()], "satisfies_type_cstr")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value()
                }
            };

            let protocol_name_str = self
                .builder
                .build_global_string_ptr(&protocol_name, "satisfies_protocol_name")
                .unwrap();
            let satisfies_i32 = self
                .builder
                .build_call(
                    satisfies_fn,
                    &[
                        type_cstr.into(),
                        protocol_name_str.as_pointer_value().into(),
                    ],
                    "satisfies_i32",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_int_value();
            let satisfies_bool = self
                .builder
                .build_int_compare(
                    IntPredicate::NE,
                    satisfies_i32,
                    self.context.i32_type().const_zero(),
                    "satisfies_bool",
                )
                .unwrap();

            let boxed = self
                .builder
                .build_call(bool_fn, &[satisfies_bool.into()], "satisfies_boxed")
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();
            return Ok(boxed);
        }

        match func {
            "gensym" => {
                if args.len() > 1 {
                    return Err("gensym takes 0 or 1 argument".to_string());
                }

                let prefix_cstr = if args.is_empty() {
                    self.context
                        .i8_type()
                        .ptr_type(inkwell::AddressSpace::default())
                        .const_null()
                } else {
                    self.compile_string_to_ptr(&args[0])?
                };

                let gensym_fn = self
                    .module
                    .get_function("clorus_gensym")
                    .ok_or("clorus_gensym not declared")?;

                let result = self
                    .builder
                    .build_call(gensym_fn, &[prefix_cstr.into()], "gensym_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }
            "derive" => self.compile_simple_2arg_call("derive", "clorus_derive", args),
            "underive" => self.compile_simple_2arg_call("underive", "clorus_underive", args),
            "slurp" => {
                // slurp takes 1 arg: path (string)
                if args.len() != 1 {
                    return Err("slurp requires 1 argument: path".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;

                let slurp_fn = self
                    .module
                    .get_function("clorus_slurp")
                    .ok_or("slurp not declared. Did you forget (use clorus.core)?")?;

                let result = self
                    .builder
                    .build_call(slurp_fn, &[path_ptr.into()], "slurp_call")
                    .unwrap();

                // slurp returns *mut c_char (string pointer)
                // Box it into a Value* string
                let str_ptr = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                Ok(self.box_string(str_ptr))
            }

            "spit" => {
                // spit takes 2 args: path (string), content (string)
                if args.len() != 2 {
                    return Err("spit requires 2 arguments: path, content".to_string());
                }

                let path_ptr = self.compile_string_to_ptr(&args[0])?;
                let content_ptr = self.compile_string_to_ptr(&args[1])?;

                let spit_fn = self
                    .module
                    .get_function("clorus_spit")
                    .ok_or("spit not declared. Did you forget (use clorus.core)?")?;

                let result = self
                    .builder
                    .build_call(spit_fn, &[path_ptr.into(), content_ptr.into()], "spit_call")
                    .unwrap();

                // spit returns i32 (1 = success, 0 = failure)
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "i32_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "get" => {
                // get takes 2-3 args: collection, key, [default]
                if args.len() < 2 || args.len() > 3 {
                    return Err(
                        "get requires 2 or 3 arguments: collection, key, [default]".to_string()
                    );
                }

                let coll_ptr = self.compile_expr(&args[0])?;
                let key_ptr = self.compile_expr(&args[1])?;

                let get_fn = self
                    .module
                    .get_function("clorus_get")
                    .ok_or("get not declared")?;

                let result = self
                    .builder
                    .build_call(get_fn, &[coll_ptr.into(), key_ptr.into()], "get_call")
                    .unwrap();

                let result_ptr = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // If we have a default value and result is nil, return default
                if args.len() == 3 {
                    // Check if result is nil
                    let is_nil_fn = self
                        .module
                        .get_function("clorus_value_is_nil")
                        .ok_or("clorus_value_is_nil not declared")?;
                    let is_nil_result = self
                        .builder
                        .build_call(is_nil_fn, &[result_ptr.into()], "is_nil_check")
                        .unwrap();
                    let is_nil_i32 = is_nil_result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_int_value();

                    // Convert i32 to i1 for branch condition
                    let zero = self.context.i32_type().const_zero();
                    let is_nil = self
                        .builder
                        .build_int_compare(IntPredicate::NE, is_nil_i32, zero, "is_nil_bool")
                        .unwrap();

                    // If nil, return default
                    let current_fn = self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_parent()
                        .unwrap();
                    let then_block = self
                        .context
                        .append_basic_block(current_fn, "return_default");
                    let else_block = self.context.append_basic_block(current_fn, "return_value");
                    let merge_block = self.context.append_basic_block(current_fn, "merge");

                    self.builder
                        .build_conditional_branch(is_nil, then_block, else_block)
                        .unwrap();

                    // Then: return default
                    self.builder.position_at_end(then_block);
                    let default_ptr = self.compile_expr(&args[2])?;
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .unwrap();

                    // Else: return result
                    self.builder.position_at_end(else_block);
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .unwrap();

                    // Merge
                    self.builder.position_at_end(merge_block);
                    let phi = self
                        .builder
                        .build_phi(result_ptr.get_type(), "get_result")
                        .unwrap();
                    phi.add_incoming(&[(&default_ptr, then_block), (&result_ptr, else_block)]);

                    Ok(phi.as_basic_value().into_pointer_value())
                } else {
                    Ok(result_ptr)
                }
            }

            "get-in" => {
                // get-in takes 2-3 args: map, key-path, [default]
                if args.len() < 2 || args.len() > 3 {
                    return Err(
                        "get-in requires 2 or 3 arguments: map, key-path, [default]"
                            .to_string(),
                    );
                }

                let map_ptr = self.compile_expr(&args[0])?;
                let path_ptr = self.compile_expr(&args[1])?;
                if args.len() == 3 {
                    let default_ptr = self.compile_expr(&args[2])?;
                    self.call_runtime_fn(
                        "clorus_map_get_in_or",
                        &[map_ptr.into(), path_ptr.into(), default_ptr.into()],
                        "get_in_or_call",
                    )
                } else {
                    self.call_runtime_fn(
                        "clorus_map_get_in",
                        &[map_ptr.into(), path_ptr.into()],
                        "get_in_call",
                    )
                }
            }

            "assoc-in" => {
                if args.len() != 3 {
                    return Err("assoc-in requires 3 arguments: map, key-path, value".to_string());
                }
                self.compile_simple_3arg_call("assoc-in", "clorus_map_assoc_in", args)
            }

            "update-in" => {
                // update-in takes 3+ args:
                // (update-in m ks f) or (update-in m ks f a b ...)
                if args.len() < 3 {
                    return Err(
                        "update-in requires at least 3 arguments: map, key-path, function, [args...]"
                            .to_string(),
                    );
                }

                let map_ptr = self.compile_expr(&args[0])?;
                let path_ptr = self.compile_expr(&args[1])?;
                let func_ptr = self.compile_expr(&args[2])?;
                let current_value = self.call_runtime_fn(
                    "clorus_map_get_in",
                    &[map_ptr.into(), path_ptr.into()],
                    "update_in_current",
                )?;

                let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let mut call_args: Vec<PointerValue<'ctx>> = vec![current_value];
                for arg in args.iter().skip(3) {
                    call_args.push(self.compile_expr(arg)?);
                }

                let args_array_ptr = if call_args.is_empty() {
                    i8_ptr_type.ptr_type(AddressSpace::default()).const_null()
                } else {
                    let array_type = i8_ptr_type.array_type(call_args.len() as u32);
                    let array_alloca = self.builder.build_alloca(array_type, "update_in_args_array").unwrap();
                    for (i, arg_val) in call_args.iter().enumerate() {
                        let elem_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    array_type,
                                    array_alloca,
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_int(i as u64, false),
                                    ],
                                    &format!("update_in_arg_{}_ptr", i),
                                )
                                .unwrap()
                        };
                        self.builder.build_store(elem_ptr, *arg_val).unwrap();
                    }
                    self.builder
                        .build_pointer_cast(
                            array_alloca,
                            i8_ptr_type.ptr_type(AddressSpace::default()),
                            "update_in_args_array_cast",
                        )
                        .unwrap()
                };

                let function_call_fn = self
                    .module
                    .get_function("clorus_function_call")
                    .ok_or("clorus_function_call not declared")?;
                let arg_count_val = self
                    .context
                    .i32_type()
                    .const_int(call_args.len() as u64, false);
                let updated_value = self
                    .builder
                    .build_call(
                        function_call_fn,
                        &[func_ptr.into(), args_array_ptr.into(), arg_count_val.into()],
                        "update_in_apply_call",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                self.call_runtime_fn(
                    "clorus_map_assoc_in",
                    &[map_ptr.into(), path_ptr.into(), updated_value.into()],
                    "update_in_assoc_call",
                )
            }

            "nth" => {
                // nth takes 2-3 args: collection, index, [default]
                if args.len() < 2 || args.len() > 3 {
                    return Err("nth requires 2 or 3 arguments: collection, index, [default]".to_string());
                }

                let coll_ptr = self.compile_expr(&args[0])?;
                let index_ptr = self.compile_expr(&args[1])?;

                // Unbox index to i64
                let index_float = self.unbox_number(index_ptr);
                let index_i64 = self
                    .builder
                    .build_float_to_signed_int(index_float, self.context.i64_type(), "index_to_i64")
                    .unwrap();

                let nth_fn = self
                    .module
                    .get_function("clorus_nth")
                    .ok_or("nth not declared")?;

                let result = self
                    .builder
                    .build_call(nth_fn, &[coll_ptr.into(), index_i64.into()], "nth_call")
                    .unwrap();
                let result_ptr = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // If we have a default value and result is nil, return default.
                if args.len() == 3 {
                    let is_nil_fn = self
                        .module
                        .get_function("clorus_value_is_nil")
                        .ok_or("clorus_value_is_nil not declared")?;
                    let is_nil_result = self
                        .builder
                        .build_call(is_nil_fn, &[result_ptr.into()], "nth_is_nil_check")
                        .unwrap();
                    let is_nil_i32 = is_nil_result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_int_value();

                    let zero = self.context.i32_type().const_zero();
                    let is_nil = self
                        .builder
                        .build_int_compare(IntPredicate::NE, is_nil_i32, zero, "nth_is_nil_bool")
                        .unwrap();

                    let current_fn = self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_parent()
                        .unwrap();
                    let then_block = self
                        .context
                        .append_basic_block(current_fn, "nth_return_default");
                    let else_block = self
                        .context
                        .append_basic_block(current_fn, "nth_return_value");
                    let merge_block = self.context.append_basic_block(current_fn, "nth_merge");

                    self.builder
                        .build_conditional_branch(is_nil, then_block, else_block)
                        .unwrap();

                    self.builder.position_at_end(then_block);
                    let default_ptr = self.compile_expr(&args[2])?;
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .unwrap();

                    self.builder.position_at_end(else_block);
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .unwrap();

                    self.builder.position_at_end(merge_block);
                    let phi = self.builder.build_phi(
                        self.context.i8_type().ptr_type(AddressSpace::default()),
                        "nth_result",
                    ).unwrap();
                    phi.add_incoming(&[(&default_ptr, then_block), (&result_ptr, else_block)]);
                    Ok(phi.as_basic_value().into_pointer_value())
                } else {
                    Ok(result_ptr)
                }
            }

            "count" => {
                // count takes 1 arg: collection
                if args.len() != 1 {
                    return Err("count requires 1 argument: collection".to_string());
                }

                let coll_ptr = self.compile_expr(&args[0])?;

                let count_fn = self
                    .module
                    .get_function("clorus_count")
                    .ok_or("count not declared")?;

                let result = self
                    .builder
                    .build_call(count_fn, &[coll_ptr.into()], "count_call")
                    .unwrap();

                // count returns i64, box as number
                let count_i64 = result.try_as_basic_value().left().unwrap().into_int_value();
                let count_float = self
                    .builder
                    .build_signed_int_to_float(count_i64, self.context.f64_type(), "count_to_float")
                    .unwrap();

                Ok(self.box_number(count_float))
            }

            "meta" => {
                if args.len() != 1 {
                    return Err("meta requires 1 argument".to_string());
                }

                let target = self.compile_expr(&args[0])?;
                let meta_fn = self
                    .module
                    .get_function("clorus_meta")
                    .ok_or("clorus_meta not declared")?;
                let result = self
                    .builder
                    .build_call(meta_fn, &[target.into()], "meta_call")
                    .unwrap();
                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "with-meta" => {
                if args.len() != 2 {
                    return Err("with-meta requires 2 arguments".to_string());
                }

                let target = self.compile_expr(&args[0])?;
                let meta_map = self.compile_expr(&args[1])?;
                let with_meta_fn = self
                    .module
                    .get_function("clorus_with_meta")
                    .ok_or("clorus_with_meta not declared")?;
                let result = self
                    .builder
                    .build_call(
                        with_meta_fn,
                        &[target.into(), meta_map.into()],
                        "with_meta_call",
                    )
                    .unwrap();
                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "vary-meta" => {
                if args.len() < 2 {
                    return Err(
                        "vary-meta requires at least 2 arguments: value, function".to_string()
                    );
                }

                let target = self.compile_expr(&args[0])?;
                let fn_value = self.compile_expr(&args[1])?;

                let meta_fn = self
                    .module
                    .get_function("clorus_meta")
                    .ok_or("clorus_meta not declared")?;
                let current_meta = self
                    .builder
                    .build_call(meta_fn, &[target.into()], "vary_meta_current")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let mut dynamic_args: Vec<PointerValue> = vec![current_meta];
                for arg in &args[2..] {
                    dynamic_args.push(self.compile_expr(arg)?);
                }

                let args_array_ptr = if dynamic_args.is_empty() {
                    value_ptr_type
                        .ptr_type(AddressSpace::default())
                        .const_null()
                } else {
                    let array_type = value_ptr_type.array_type(dynamic_args.len() as u32);
                    let array_alloca = self
                        .builder
                        .build_alloca(array_type, "vary_meta_args")
                        .unwrap();

                    for (i, arg_val) in dynamic_args.iter().enumerate() {
                        let elem_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    array_type,
                                    array_alloca,
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_int(i as u64, false),
                                    ],
                                    &format!("vary_meta_arg_{}", i),
                                )
                                .unwrap()
                        };
                        self.builder.build_store(elem_ptr, *arg_val).unwrap();
                    }

                    self.builder
                        .build_pointer_cast(
                            array_alloca,
                            value_ptr_type.ptr_type(AddressSpace::default()),
                            "vary_meta_args_cast",
                        )
                        .unwrap()
                };

                let function_call_fn = self
                    .module
                    .get_function("clorus_function_call")
                    .ok_or("clorus_function_call not declared")?;
                let arg_count_val = self
                    .context
                    .i32_type()
                    .const_int(dynamic_args.len() as u64, false);
                let new_meta = self
                    .builder
                    .build_call(
                        function_call_fn,
                        &[fn_value.into(), args_array_ptr.into(), arg_count_val.into()],
                        "vary_meta_new_meta",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let with_meta_fn = self
                    .module
                    .get_function("clorus_with_meta")
                    .ok_or("clorus_with_meta not declared")?;
                let result = self
                    .builder
                    .build_call(
                        with_meta_fn,
                        &[target.into(), new_meta.into()],
                        "vary_meta_apply",
                    )
                    .unwrap();
                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "reset-meta!" => {
                if args.len() != 2 {
                    return Err("reset-meta! requires 2 arguments: value, meta-map".to_string());
                }

                let target = self.compile_expr(&args[0])?;
                let new_meta = self.compile_expr(&args[1])?;

                let with_meta_fn = self
                    .module
                    .get_function("clorus_with_meta")
                    .ok_or("clorus_with_meta not declared")?;
                let updated = self
                    .builder
                    .build_call(
                        with_meta_fn,
                        &[target.into(), new_meta.into()],
                        "reset_meta_apply",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let is_exception_fn = self
                    .module
                    .get_function("clorus_is_exception_i32")
                    .ok_or("clorus_is_exception_i32 not declared")?;
                let is_exception_i32 = self
                    .builder
                    .build_call(
                        is_exception_fn,
                        &[updated.into()],
                        "reset_meta_is_exception_i32",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let is_exception = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        is_exception_i32,
                        self.context.i32_type().const_zero(),
                        "reset_meta_is_exception",
                    )
                    .unwrap();

                let meta_fn = self
                    .module
                    .get_function("clorus_meta")
                    .ok_or("clorus_meta not declared")?;

                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(|block| block.get_parent())
                    .expect("No parent function");
                let exception_bb = self
                    .context
                    .append_basic_block(function, "reset_meta_exception");
                let ok_bb = self.context.append_basic_block(function, "reset_meta_ok");
                let merge_bb = self
                    .context
                    .append_basic_block(function, "reset_meta_merge");

                self.builder
                    .build_conditional_branch(is_exception, exception_bb, ok_bb)
                    .unwrap();

                self.builder.position_at_end(exception_bb);
                self.builder.build_unconditional_branch(merge_bb).unwrap();
                let exception_bb_end = self.builder.get_insert_block().unwrap();

                self.builder.position_at_end(ok_bb);
                let meta_result = self
                    .builder
                    .build_call(meta_fn, &[target.into()], "reset_meta_ok_result")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                self.builder.build_unconditional_branch(merge_bb).unwrap();
                let ok_bb_end = self.builder.get_insert_block().unwrap();

                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(
                        self.context.i8_type().ptr_type(AddressSpace::default()),
                        "reset_meta_result",
                    )
                    .unwrap();
                phi.add_incoming(&[(&updated, exception_bb_end), (&meta_result, ok_bb_end)]);
                Ok(phi.as_basic_value().into_pointer_value())
            }

            "alter-meta!" => {
                if args.len() < 2 {
                    return Err(
                        "alter-meta! requires at least 2 arguments: value, function".to_string()
                    );
                }

                let target = self.compile_expr(&args[0])?;
                let fn_value = self.compile_expr(&args[1])?;

                let meta_fn = self
                    .module
                    .get_function("clorus_meta")
                    .ok_or("clorus_meta not declared")?;
                let current_meta = self
                    .builder
                    .build_call(meta_fn, &[target.into()], "alter_meta_current")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let mut dynamic_args: Vec<PointerValue> = vec![current_meta];
                for arg in &args[2..] {
                    dynamic_args.push(self.compile_expr(arg)?);
                }

                let args_array_ptr = if dynamic_args.is_empty() {
                    value_ptr_type
                        .ptr_type(AddressSpace::default())
                        .const_null()
                } else {
                    let array_type = value_ptr_type.array_type(dynamic_args.len() as u32);
                    let array_alloca = self
                        .builder
                        .build_alloca(array_type, "alter_meta_args")
                        .unwrap();

                    for (i, arg_val) in dynamic_args.iter().enumerate() {
                        let elem_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    array_type,
                                    array_alloca,
                                    &[
                                        self.context.i32_type().const_zero(),
                                        self.context.i32_type().const_int(i as u64, false),
                                    ],
                                    &format!("alter_meta_arg_{}", i),
                                )
                                .unwrap()
                        };
                        self.builder.build_store(elem_ptr, *arg_val).unwrap();
                    }

                    self.builder
                        .build_pointer_cast(
                            array_alloca,
                            value_ptr_type.ptr_type(AddressSpace::default()),
                            "alter_meta_args_cast",
                        )
                        .unwrap()
                };

                let function_call_fn = self
                    .module
                    .get_function("clorus_function_call")
                    .ok_or("clorus_function_call not declared")?;
                let arg_count_val = self
                    .context
                    .i32_type()
                    .const_int(dynamic_args.len() as u64, false);
                let new_meta = self
                    .builder
                    .build_call(
                        function_call_fn,
                        &[fn_value.into(), args_array_ptr.into(), arg_count_val.into()],
                        "alter_meta_new_meta",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let with_meta_fn = self
                    .module
                    .get_function("clorus_with_meta")
                    .ok_or("clorus_with_meta not declared")?;
                let updated = self
                    .builder
                    .build_call(
                        with_meta_fn,
                        &[target.into(), new_meta.into()],
                        "alter_meta_apply",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let is_exception_fn = self
                    .module
                    .get_function("clorus_is_exception_i32")
                    .ok_or("clorus_is_exception_i32 not declared")?;
                let is_exception_i32 = self
                    .builder
                    .build_call(
                        is_exception_fn,
                        &[updated.into()],
                        "alter_meta_is_exception_i32",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let is_exception = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        is_exception_i32,
                        self.context.i32_type().const_zero(),
                        "alter_meta_is_exception",
                    )
                    .unwrap();

                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(|block| block.get_parent())
                    .expect("No parent function");
                let exception_bb = self
                    .context
                    .append_basic_block(function, "alter_meta_exception");
                let ok_bb = self.context.append_basic_block(function, "alter_meta_ok");
                let merge_bb = self
                    .context
                    .append_basic_block(function, "alter_meta_merge");

                self.builder
                    .build_conditional_branch(is_exception, exception_bb, ok_bb)
                    .unwrap();

                self.builder.position_at_end(exception_bb);
                self.builder.build_unconditional_branch(merge_bb).unwrap();
                let exception_bb_end = self.builder.get_insert_block().unwrap();

                self.builder.position_at_end(ok_bb);
                let meta_result = self
                    .builder
                    .build_call(meta_fn, &[target.into()], "alter_meta_ok_result")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                self.builder.build_unconditional_branch(merge_bb).unwrap();
                let ok_bb_end = self.builder.get_insert_block().unwrap();

                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(
                        self.context.i8_type().ptr_type(AddressSpace::default()),
                        "alter_meta_result",
                    )
                    .unwrap();
                phi.add_incoming(&[(&updated, exception_bb_end), (&meta_result, ok_bb_end)]);
                Ok(phi.as_basic_value().into_pointer_value())
            }

            "var" => {
                if args.len() != 1 {
                    return Err("var requires 1 symbol argument".to_string());
                }

                if let Expr::Symbol(name) = &args[0] {
                    self.compile_expr(&Expr::Var { name: name.clone() })
                } else {
                    Err("var requires a symbol argument".to_string())
                }
            }

            "empty?" => {
                // empty? takes 1 arg: collection
                if args.len() != 1 {
                    return Err("empty? requires 1 argument: collection".to_string());
                }

                let coll_ptr = self.compile_expr(&args[0])?;

                let count_fn = self
                    .module
                    .get_function("clorus_count")
                    .ok_or("clorus_count not declared")?;

                let count_result = self
                    .builder
                    .build_call(count_fn, &[coll_ptr.into()], "count_call")
                    .unwrap();

                // count returns i64, compare with 0
                let count_i64 = count_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let zero = self.context.i64_type().const_zero();
                let is_empty = self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::EQ, count_i64, zero, "is_empty")
                    .unwrap();

                // Convert bool to Value* (boolean)
                let value_bool_fn = self
                    .module
                    .get_function("clorus_value_boolean")
                    .ok_or("clorus_value_boolean not declared")?;
                let result = self
                    .builder
                    .build_call(value_bool_fn, &[is_empty.into()], "empty_bool")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "conj" => {
                // conj takes 2 args: collection, element
                if args.len() != 2 {
                    return Err("conj requires 2 arguments: collection, element".to_string());
                }

                let coll_ptr = self.compile_expr(&args[0])?;
                let elem_ptr = self.compile_expr(&args[1])?;

                // Use generic clorus_conj which dispatches based on collection type
                let conj_fn = self
                    .module
                    .get_function("clorus_conj")
                    .ok_or("clorus_conj not declared")?;

                let result = self
                    .builder
                    .build_call(conj_fn, &[coll_ptr.into(), elem_ptr.into()], "conj_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "list" => {
                // list takes 0+ args and returns a list with the same element order.
                let list_empty_fn = self
                    .module
                    .get_function("clorus_list_empty")
                    .ok_or("clorus_list_empty not declared")?;
                let list_cons_fn = self
                    .module
                    .get_function("clorus_list_cons")
                    .ok_or("clorus_list_cons not declared")?;

                // Build list from right to left because cons prepends.
                let mut list_ptr = self
                    .builder
                    .build_call(list_empty_fn, &[], "list_empty")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                for arg in args.iter().rev() {
                    let elem_ptr = self.compile_expr(arg)?;
                    list_ptr = self
                        .builder
                        .build_call(
                            list_cons_fn,
                            &[list_ptr.into(), elem_ptr.into()],
                            "list_cons",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(list_ptr)
            }

            "cons" => {
                // cons takes 2 args: element, collection
                // Note: cons is like conj but with reversed argument order (Clojure style)
                if args.len() != 2 {
                    return Err("cons requires 2 arguments: element, collection".to_string());
                }

                let elem_ptr = self.compile_expr(&args[0])?;
                let coll_ptr = self.compile_expr(&args[1])?;

                // Use clorus_list_cons
                let cons_fn = self
                    .module
                    .get_function("clorus_list_cons")
                    .ok_or("clorus_list_cons not declared")?;

                let result = self
                    .builder
                    .build_call(
                        cons_fn,
                        &[coll_ptr.into(), elem_ptr.into()], // Note: cons takes (list, elem)
                        "cons_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "disj" => {
                // disj takes 2 args: set, element
                if args.len() != 2 {
                    return Err("disj requires 2 arguments: set, element".to_string());
                }

                let set_ptr = self.compile_expr(&args[0])?;
                let elem_ptr = self.compile_expr(&args[1])?;

                let disj_fn = self
                    .module
                    .get_function("clorus_set_disj")
                    .ok_or("clorus_set_disj not declared")?;

                let result = self
                    .builder
                    .build_call(disj_fn, &[set_ptr.into(), elem_ptr.into()], "set_disj_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "assoc" => {
                // assoc takes 3 args: map, key, value
                if args.len() != 3 {
                    return Err("assoc requires 3 arguments: map, key, value".to_string());
                }

                let map_ptr = self.compile_expr(&args[0])?;
                let key_ptr = self.compile_expr(&args[1])?;
                let val_ptr = self.compile_expr(&args[2])?;

                let assoc_fn = self
                    .module
                    .get_function("clorus_map_assoc")
                    .ok_or("clorus_map_assoc not declared")?;

                let result = self
                    .builder
                    .build_call(
                        assoc_fn,
                        &[map_ptr.into(), key_ptr.into(), val_ptr.into()],
                        "map_assoc_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "dissoc" => {
                // dissoc takes 2 args: map, key
                if args.len() != 2 {
                    return Err("dissoc requires 2 arguments: map, key".to_string());
                }

                let map_ptr = self.compile_expr(&args[0])?;
                let key_ptr = self.compile_expr(&args[1])?;

                let dissoc_fn = self
                    .module
                    .get_function("clorus_map_dissoc")
                    .ok_or("clorus_map_dissoc not declared")?;

                let result = self
                    .builder
                    .build_call(
                        dissoc_fn,
                        &[map_ptr.into(), key_ptr.into()],
                        "map_dissoc_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "contains?" => {
                // contains? takes 2 args: collection, key/index/element
                if args.len() != 2 {
                    return Err("contains? requires 2 arguments: collection, key/index/element".to_string());
                }

                let coll_ptr = self.compile_expr(&args[0])?;
                let key_ptr = self.compile_expr(&args[1])?;

                let contains_fn = self
                    .module
                    .get_function("clorus_contains")
                    .ok_or("clorus_contains not declared")?;

                let result = self
                    .builder
                    .build_call(
                        contains_fn,
                        &[coll_ptr.into(), key_ptr.into()],
                        "contains_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "reduce" => {
                // reduce takes 2-3 args: function, init-val (optional), collection
                if args.len() < 2 || args.len() > 3 {
                    return Err(
                        "reduce requires 2 or 3 arguments: function, [init], collection"
                            .to_string(),
                    );
                }

                // Compile the reducing function - supports both named functions and inline lambdas
                let func_val = self.compile_expr(&args[0])?;

                let (init_val, coll_ptr, start_index) = if args.len() == 3 {
                    // (reduce f init coll) - explicit init value
                    let init_val = self.compile_expr(&args[1])?;
                    let coll_ptr = self.compile_expr(&args[2])?;
                    (init_val, coll_ptr, self.context.i64_type().const_zero())
                } else {
                    // (reduce f coll) - use first element as init
                    let coll_ptr = self.compile_expr(&args[1])?;

                    // Get first element using nth
                    let nth_fn = self
                        .module
                        .get_function("clorus_nth")
                        .ok_or("nth not declared")?;
                    let first_elem = self
                        .builder
                        .build_call(
                            nth_fn,
                            &[coll_ptr.into(), self.context.i64_type().const_zero().into()],
                            "first_elem",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Start from index 1 since we used index 0 as init
                    (
                        first_elem,
                        coll_ptr,
                        self.context.i64_type().const_int(1, false),
                    )
                };

                let count_fn = self
                    .module
                    .get_function("clorus_count")
                    .ok_or("count not declared")?;
                let count_i64 = self
                    .builder
                    .build_call(count_fn, &[coll_ptr.into()], "coll_count")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                // Loop setup
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let loop_block = self.context.append_basic_block(current_fn, "reduce_loop");
                let body_block = self.context.append_basic_block(current_fn, "reduce_body");
                let end_block = self.context.append_basic_block(current_fn, "reduce_end");

                let index_alloca = self
                    .builder
                    .build_alloca(self.context.i64_type(), "index")
                    .unwrap();
                self.builder.build_store(index_alloca, start_index).unwrap();

                let acc_alloca = self
                    .builder
                    .build_alloca(init_val.get_type(), "accumulator")
                    .unwrap();
                self.builder.build_store(acc_alloca, init_val).unwrap();

                self.builder.build_unconditional_branch(loop_block).unwrap();

                // Loop condition
                self.builder.position_at_end(loop_block);
                let current_index = self
                    .builder
                    .build_load(self.context.i64_type(), index_alloca, "current_index")
                    .unwrap()
                    .into_int_value();
                let condition = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::SLT,
                        current_index,
                        count_i64,
                        "loop_cond",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(condition, body_block, end_block)
                    .unwrap();

                // Loop body
                self.builder.position_at_end(body_block);
                let nth_fn = self
                    .module
                    .get_function("clorus_nth")
                    .ok_or("nth not declared")?;
                let elem = self
                    .builder
                    .build_call(nth_fn, &[coll_ptr.into(), current_index.into()], "elem")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let current_acc = self
                    .builder
                    .build_load(init_val.get_type(), acc_alloca, "current_acc")
                    .unwrap()
                    .into_pointer_value();

                // Call reducing function with (acc, elem) using dynamic dispatch
                let function_call_fn = self
                    .module
                    .get_function("clorus_function_call")
                    .ok_or("clorus_function_call not declared")?;

                let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                let args_array_type = i8_ptr_type.array_type(2);
                let args_array = self
                    .builder
                    .build_alloca(args_array_type, "reduce_args")
                    .unwrap();

                // Store acc as first arg
                let acc_ptr = unsafe {
                    self.builder
                        .build_gep(
                            args_array_type,
                            args_array,
                            &[
                                self.context.i32_type().const_zero(),
                                self.context.i32_type().const_zero(),
                            ],
                            "acc_ptr",
                        )
                        .unwrap()
                };
                self.builder.build_store(acc_ptr, current_acc).unwrap();

                // Store elem as second arg
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(
                            args_array_type,
                            args_array,
                            &[
                                self.context.i32_type().const_zero(),
                                self.context.i32_type().const_int(1, false),
                            ],
                            "elem_ptr",
                        )
                        .unwrap()
                };
                self.builder.build_store(elem_ptr, elem).unwrap();

                let args_array_ptr = self
                    .builder
                    .build_pointer_cast(
                        args_array,
                        i8_ptr_type.ptr_type(AddressSpace::default()),
                        "args_cast",
                    )
                    .unwrap();

                let new_acc = self
                    .builder
                    .build_call(
                        function_call_fn,
                        &[
                            func_val.into(),
                            args_array_ptr.into(),
                            self.context.i32_type().const_int(2, false).into(),
                        ],
                        "new_acc",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
                self.builder.build_store(acc_alloca, new_acc).unwrap();

                let next_index = self
                    .builder
                    .build_int_add(
                        current_index,
                        self.context.i64_type().const_int(1, false),
                        "next_index",
                    )
                    .unwrap();
                self.builder.build_store(index_alloca, next_index).unwrap();
                self.builder.build_unconditional_branch(loop_block).unwrap();

                // End
                self.builder.position_at_end(end_block);
                let final_acc = self
                    .builder
                    .build_load(init_val.get_type(), acc_alloca, "final_acc")
                    .unwrap()
                    .into_pointer_value();

                Ok(final_acc)
            }

            "atom" => {
                // atom takes 1 arg: initial value
                if args.len() != 1 {
                    return Err("atom requires 1 argument: initial value".to_string());
                }

                let initial_val = self.compile_expr(&args[0])?;

                let atom_fn = self
                    .module
                    .get_function("clorus_atom")
                    .ok_or("clorus_atom not declared")?;

                let result = self
                    .builder
                    .build_call(atom_fn, &[initial_val.into()], "atom_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "reset!" => {
                // reset! takes 2 args: atom, new-value
                if args.len() != 2 {
                    return Err("reset! requires 2 arguments: atom, new-value".to_string());
                }

                let atom_val = self.compile_expr(&args[0])?;
                let new_val = self.compile_expr(&args[1])?;

                let reset_fn = self
                    .module
                    .get_function("clorus_reset")
                    .ok_or("clorus_reset not declared")?;

                let result = self
                    .builder
                    .build_call(reset_fn, &[atom_val.into(), new_val.into()], "reset_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "swap!" => {
                // swap! takes 2+ args: atom, function, [args...]
                // For now, support: (swap! atom func arg)
                if args.len() < 2 {
                    return Err("swap! requires at least 2 arguments: atom, function".to_string());
                }

                let atom_val = self.compile_expr(&args[0])?;

                // Function value argument for swap! (supports fn literals and symbols)
                let func_val = match &args[1] {
                    Expr::Symbol(func_name) => {
                        match self.compile_expr(&args[1]) {
                            Ok(v) => v,
                            Err(_) => {
                                // Fallback: wrap core runtime function as a first-class function value.
                                let runtime_name = match func_name.as_str() {
                                    "+" => Some("clorus_add"),
                                    "-" => Some("clorus_sub"),
                                    "*" => Some("clorus_mul"),
                                    "/" => Some("clorus_div"),
                                    "mod" => Some("clorus_mod"),
                                    "conj" => Some("clorus_conj"),
                                    "disj" => Some("clorus_set_disj"),
                                    _ => None,
                                }
                                .ok_or_else(|| {
                                    format!("Function not found for swap!: {}", func_name)
                                })?;

                                let runtime_fn = self
                                    .module
                                    .get_function(runtime_name)
                                    .ok_or_else(|| format!("{} not declared", runtime_name))?;
                                let func_new_fn = self
                                    .module
                                    .get_function("clorus_function_new")
                                    .ok_or("clorus_function_new not declared")?;

                                let i8_ptr_type =
                                    self.context.i8_type().ptr_type(AddressSpace::default());
                                let fn_ptr = runtime_fn.as_global_value().as_pointer_value();
                                let fn_ptr_cast = self
                                    .builder
                                    .build_pointer_cast(fn_ptr, i8_ptr_type, "swap_runtime_fn_ptr")
                                    .unwrap();
                                let arity_val = self.context.i32_type().const_int(2, false);
                                let env_ptr = i8_ptr_type.const_null();
                                let env_size = self.context.i32_type().const_zero();

                                self.builder
                                    .build_call(
                                        func_new_fn,
                                        &[
                                            fn_ptr_cast.into(),
                                            arity_val.into(),
                                            env_ptr.into(),
                                            env_size.into(),
                                        ],
                                        "swap_runtime_func_val",
                                    )
                                    .unwrap()
                                    .try_as_basic_value()
                                    .left()
                                    .unwrap()
                                    .into_pointer_value()
                            }
                        }
                    }
                    _ => self.compile_expr(&args[1])?,
                };

                // For now, support single additional argument
                let arg_val = if args.len() >= 3 {
                    self.compile_expr(&args[2])?
                } else {
                    // No additional arg - pass null as sentinel for 1-arity swap! call.
                    self.context
                        .i8_type()
                        .ptr_type(AddressSpace::default())
                        .const_null()
                };

                let swap_fn = self
                    .module
                    .get_function("clorus_swap")
                    .ok_or("clorus_swap not declared")?;

                let result = self
                    .builder
                    .build_call(
                        swap_fn,
                        &[atom_val.into(), func_val.into(), arg_val.into()],
                        "swap_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "ref" => {
                // ref takes 1 arg: initial value
                if args.len() != 1 {
                    return Err("ref requires 1 argument: initial value".to_string());
                }

                let initial_val = self.compile_expr(&args[0])?;

                let ref_fn = self
                    .module
                    .get_function("clorus_ref")
                    .ok_or("clorus_ref not declared")?;

                let result = self
                    .builder
                    .build_call(ref_fn, &[initial_val.into()], "ref_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "ref-set" => {
                // ref-set takes 2 args: ref, new-value
                // Must be inside dosync
                if args.len() != 2 {
                    return Err("ref-set requires 2 arguments: ref, new-value".to_string());
                }

                let ref_val = self.compile_expr(&args[0])?;
                let new_val = self.compile_expr(&args[1])?;

                let ref_set_fn = self
                    .module
                    .get_function("clorus_ref_set")
                    .ok_or("clorus_ref_set not declared")?;

                let result = self
                    .builder
                    .build_call(
                        ref_set_fn,
                        &[ref_val.into(), new_val.into()],
                        "ref_set_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
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
                let ref_deref_fn = self
                    .module
                    .get_function("clorus_ref_deref")
                    .ok_or("clorus_ref_deref not declared")?;

                let current_val = self
                    .builder
                    .build_call(ref_deref_fn, &[ref_val.into()], "alter_deref")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Get the function to apply
                let func_name = match &args[1] {
                    Expr::Symbol(name) => name.clone(),
                    _ => return Err("alter requires a function as second argument".to_string()),
                };

                let function = self
                    .functions
                    .get(&func_name)
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

                let new_val = self
                    .builder
                    .build_call(function, &func_args, "alter_apply")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Now set the ref to the new value
                let alter_fn = self
                    .module
                    .get_function("clorus_alter")
                    .ok_or("clorus_alter not declared")?;

                let result = self
                    .builder
                    .build_call(alter_fn, &[ref_val.into(), new_val.into()], "alter_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "agent" => {
                // agent takes 1 arg: initial value
                if args.len() != 1 {
                    return Err("agent requires 1 argument: initial value".to_string());
                }

                let initial_val = self.compile_expr(&args[0])?;

                let agent_fn = self
                    .module
                    .get_function("clorus_agent")
                    .ok_or("clorus_agent not declared")?;

                let result = self
                    .builder
                    .build_call(agent_fn, &[initial_val.into()], "agent_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
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

                let function = self
                    .functions
                    .get(&func_name)
                    .ok_or_else(|| format!("Function not found: {}", func_name))?
                    .clone();

                // Convert function to pointer value
                let func_ptr = function.as_global_value().as_pointer_value();

                // Pack remaining args into vector
                let vector_empty_fn = self
                    .module
                    .get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;

                let mut args_vec = self
                    .builder
                    .build_call(vector_empty_fn, &[], "send_args_vec")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add each arg to vector
                let vector_conj_fn = self
                    .module
                    .get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                for i in 2..args.len() {
                    let arg_val = self.compile_expr(&args[i])?;
                    args_vec = self
                        .builder
                        .build_call(
                            vector_conj_fn,
                            &[args_vec.into(), arg_val.into()],
                            &format!("send_arg_{}", i),
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                // Call clorus_send
                let send_fn = self
                    .module
                    .get_function("clorus_send")
                    .ok_or("clorus_send not declared")?;

                // Cast func_ptr to *mut Value (i8*)
                let func_val = self
                    .builder
                    .build_pointer_cast(
                        func_ptr,
                        self.context
                            .i8_type()
                            .ptr_type(inkwell::AddressSpace::default()),
                        "func_as_value",
                    )
                    .unwrap();

                let result = self
                    .builder
                    .build_call(
                        send_fn,
                        &[agent_val.into(), func_val.into(), args_vec.into()],
                        "send_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "agent-error" => {
                // agent-error takes 1 arg: agent
                if args.len() != 1 {
                    return Err("agent-error requires 1 argument: agent".to_string());
                }

                let agent_val = self.compile_expr(&args[0])?;

                let agent_error_fn = self
                    .module
                    .get_function("clorus_agent_error")
                    .ok_or("clorus_agent_error not declared")?;

                let result = self
                    .builder
                    .build_call(agent_error_fn, &[agent_val.into()], "agent_error_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "await" => {
                // await takes 1 arg: agent or vector of agents
                if args.len() != 1 {
                    return Err("await requires 1 argument: agent or vector of agents".to_string());
                }

                let agents_val = self.compile_expr(&args[0])?;

                let await_fn = self
                    .module
                    .get_function("clorus_await")
                    .ok_or("clorus_await not declared")?;

                let result = self
                    .builder
                    .build_call(await_fn, &[agents_val.into()], "await_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "await-for" => {
                // await-for takes 2 args: agent, timeout-ms
                if args.len() != 2 {
                    return Err("await-for requires 2 arguments: agent, timeout-ms".to_string());
                }

                let agent_val = self.compile_expr(&args[0])?;
                let timeout_expr = self.compile_expr(&args[1])?;

                // Extract number value
                let value_as_double_fn = self
                    .module
                    .get_function("clorus_value_as_double")
                    .ok_or("clorus_value_as_double not declared")?;

                let timeout_f64 = self
                    .builder
                    .build_call(value_as_double_fn, &[timeout_expr.into()], "timeout_as_f64")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value();

                // Convert f64 to i64
                let timeout_i64 = self
                    .builder
                    .build_float_to_signed_int(timeout_f64, self.context.i64_type(), "timeout_i64")
                    .unwrap();

                let await_for_fn = self
                    .module
                    .get_function("clorus_await_for")
                    .ok_or("clorus_await_for not declared")?;

                let result = self
                    .builder
                    .build_call(
                        await_for_fn,
                        &[agent_val.into(), timeout_i64.into()],
                        "await_for_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
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
                    let value_as_double_fn = self
                        .module
                        .get_function("clorus_value_as_double")
                        .ok_or("clorus_value_as_double not declared")?;

                    let cap_f64 = self
                        .builder
                        .build_call(value_as_double_fn, &[cap_expr.into()], "cap_as_f64")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_float_value();

                    // Convert f64 to i64
                    self.builder
                        .build_float_to_signed_int(cap_f64, self.context.i64_type(), "cap_i64")
                        .unwrap()
                } else {
                    return Err("chan takes 0 or 1 argument: optional capacity".to_string());
                };

                let chan_fn = self
                    .module
                    .get_function("clorus_chan")
                    .ok_or("clorus_chan not declared")?;

                let result = self
                    .builder
                    .build_call(chan_fn, &[capacity.into()], "chan_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            ">!!" => {
                // >!! takes 2 args: channel, value (blocking put)
                if args.len() != 2 {
                    return Err(">!! requires 2 arguments: channel, value".to_string());
                }

                let chan_val = self.compile_expr(&args[0])?;
                let value_val = self.compile_expr(&args[1])?;

                let chan_put_fn = self
                    .module
                    .get_function("clorus_chan_put")
                    .ok_or("clorus_chan_put not declared")?;

                let result = self
                    .builder
                    .build_call(
                        chan_put_fn,
                        &[chan_val.into(), value_val.into()],
                        "chan_put_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "<!!" => {
                // <!! takes 1 arg: channel (blocking take)
                if args.len() != 1 {
                    return Err("<!! requires 1 argument: channel".to_string());
                }

                let chan_val = self.compile_expr(&args[0])?;

                let chan_take_fn = self
                    .module
                    .get_function("clorus_chan_take")
                    .ok_or("clorus_chan_take not declared")?;

                let result = self
                    .builder
                    .build_call(chan_take_fn, &[chan_val.into()], "chan_take_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "close!" => {
                // close! takes 1 arg: channel
                if args.len() != 1 {
                    return Err("close! requires 1 argument: channel".to_string());
                }

                let chan_val = self.compile_expr(&args[0])?;

                let chan_close_fn = self
                    .module
                    .get_function("clorus_chan_close")
                    .ok_or("clorus_chan_close not declared")?;

                let result = self
                    .builder
                    .build_call(chan_close_fn, &[chan_val.into()], "chan_close_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "alts!!" => {
                // alts!! takes 1 arg: vector of channels
                // Returns [value channel] from first available channel
                if args.len() != 1 {
                    return Err("alts!! requires 1 argument: vector of channels".to_string());
                }

                let channels_val = self.compile_expr(&args[0])?;

                let alts_fn = self
                    .module
                    .get_function("clorus_alts")
                    .ok_or("clorus_alts not declared")?;

                let result = self
                    .builder
                    .build_call(alts_fn, &[channels_val.into()], "alts_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
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
                let value_ptr_type = self
                    .context
                    .i8_type()
                    .ptr_type(inkwell::AddressSpace::default());
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
                    let nth_fn = self
                        .module
                        .get_function("clorus_vector_nth")
                        .ok_or("clorus_vector_nth not declared")?;

                    for (i, var_name) in free_vars.iter().enumerate() {
                        // Get value from captures vector
                        let index = self.context.i64_type().const_int(i as u64, false);
                        let var_value = self
                            .builder
                            .build_call(
                                nth_fn,
                                &[captures_param.into(), index.into()],
                                &format!("capture_{}", var_name),
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();

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
                let vector_empty_fn = self
                    .module
                    .get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;

                let mut captures_vec = self
                    .builder
                    .build_call(vector_empty_fn, &[], "captures_vec")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add each captured variable to vector
                if !free_vars.is_empty() {
                    let vector_conj_fn = self
                        .module
                        .get_function("clorus_vector_conj")
                        .ok_or("clorus_vector_conj not declared")?;

                    for var_name in &free_vars {
                        // Get variable value
                        let var_alloca = self
                            .variables
                            .get(var_name)
                            .ok_or_else(|| format!("Variable not found: {}", var_name))?;
                        let var_value = self
                            .builder
                            .build_load(value_ptr_type, *var_alloca, var_name)
                            .unwrap()
                            .into_pointer_value();

                        // Add to vector
                        captures_vec = self
                            .builder
                            .build_call(
                                vector_conj_fn,
                                &[captures_vec.into(), var_value.into()],
                                &format!("capture_add_{}", var_name),
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    }
                }

                // Get function pointer
                let func_ptr = function.as_global_value().as_pointer_value();

                // Cast to i8* for clorus_go
                let func_val = self
                    .builder
                    .build_pointer_cast(func_ptr, value_ptr_type, "go_func_ptr")
                    .unwrap();

                // Call clorus_go with function and captures
                let go_fn = self
                    .module
                    .get_function("clorus_go")
                    .ok_or("clorus_go not declared")?;

                let result = self
                    .builder
                    .build_call(go_fn, &[func_val.into(), captures_vec.into()], "go_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "apply" => {
                // apply takes 2+ args: function, [arg1 ...], collection
                if args.len() < 2 {
                    return Err(
                        "apply requires at least 2 arguments: function, [args...], collection"
                            .to_string(),
                    );
                }

                // Compile the function expression - supports both named functions and closures
                let func_val = self.compile_expr(&args[0])?;

                // Last argument must be a collection whose elements are appended.
                let coll_ptr = self.compile_expr(args.last().unwrap())?;
                let fixed_args: Vec<PointerValue<'ctx>> = args[1..args.len() - 1]
                    .iter()
                    .map(|arg| self.compile_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let fixed_count = fixed_args.len() as u64;

                // Get collection count
                let count_fn = self
                    .module
                    .get_function("clorus_count")
                    .ok_or("clorus_count not declared")?;
                let count_result = self
                    .builder
                    .build_call(count_fn, &[coll_ptr.into()], "apply_count")
                    .unwrap();
                let count_i64 = count_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();
                let fixed_count_i64 = self.context.i64_type().const_int(fixed_count, false);
                let total_count_i64 = self
                    .builder
                    .build_int_add(count_i64, fixed_count_i64, "apply_total_count")
                    .unwrap();

                // Get nth function
                let nth_fn = self
                    .module
                    .get_function("clorus_nth")
                    .ok_or("clorus_nth not declared")?;

                // Create loop blocks to append collection elements after fixed args.
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let loop_block = self.context.append_basic_block(current_fn, "apply_loop");
                let body_block = self.context.append_basic_block(current_fn, "apply_body");
                let end_block = self.context.append_basic_block(current_fn, "apply_end");

                let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                // Allocate exact-size argument array at runtime.
                let args_array = self
                    .builder
                    .build_array_alloca(value_ptr_type, total_count_i64, "apply_args_array")
                    .unwrap();

                // Pre-fill fixed args at indices [0..fixed_count).
                for (i, arg_val) in fixed_args.iter().enumerate() {
                    let idx = self.context.i64_type().const_int(i as u64, false);
                    let elem_ptr = unsafe {
                        self.builder
                            .build_gep(value_ptr_type, args_array, &[idx], "apply_fixed_arg_ptr")
                            .unwrap()
                    };
                    self.builder.build_store(elem_ptr, *arg_val).unwrap();
                }

                // Index counter over collection elements.
                let index_alloca = self
                    .builder
                    .build_alloca(self.context.i64_type(), "apply_coll_index")
                    .unwrap();
                self.builder
                    .build_store(index_alloca, self.context.i64_type().const_zero())
                    .unwrap();
                self.builder.build_unconditional_branch(loop_block).unwrap();

                // Loop condition: index < collection_count
                self.builder.position_at_end(loop_block);
                let current_index = self
                    .builder
                    .build_load(self.context.i64_type(), index_alloca, "current_index")
                    .unwrap()
                    .into_int_value();

                let condition = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::SLT,
                        current_index,
                        count_i64,
                        "apply_loop_cond",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(condition, body_block, end_block)
                    .unwrap();

                // Loop body: extract collection element and store after fixed args.
                self.builder.position_at_end(body_block);
                let elem = self
                    .builder
                    .build_call(
                        nth_fn,
                        &[coll_ptr.into(), current_index.into()],
                        "apply_elem",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let target_index = self
                    .builder
                    .build_int_add(current_index, fixed_count_i64, "apply_target_index")
                    .unwrap();
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(
                            value_ptr_type,
                            args_array,
                            &[target_index],
                            "apply_elem_ptr",
                        )
                        .unwrap()
                };
                self.builder.build_store(elem_ptr, elem).unwrap();

                // Increment index
                let next_index = self
                    .builder
                    .build_int_add(
                        current_index,
                        self.context.i64_type().const_int(1, false),
                        "next_index",
                    )
                    .unwrap();
                self.builder.build_store(index_alloca, next_index).unwrap();
                self.builder.build_unconditional_branch(loop_block).unwrap();

                // After loop: call function with extracted arguments using dynamic dispatch
                self.builder.position_at_end(end_block);
                let count_i32 = self
                    .builder
                    .build_int_cast(total_count_i64, self.context.i32_type(), "count_i32")
                    .unwrap();

                // Call clorus_function_call for dynamic dispatch
                let function_call_fn = self
                    .module
                    .get_function("clorus_function_call")
                    .ok_or("clorus_function_call not declared")?;

                let result = self
                    .builder
                    .build_call(
                        function_call_fn,
                        &[func_val.into(), args_array.into(), count_i32.into()],
                        "apply_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            // New Collection API functions
            "dissoc" => {
                if args.len() != 2 {
                    return Err("dissoc requires 2 arguments: map, key".to_string());
                }
                let map_val = self.compile_expr(&args[0])?;
                let key_val = self.compile_expr(&args[1])?;
                let dissoc_fn = self
                    .module
                    .get_function("clorus_map_dissoc")
                    .ok_or("clorus_map_dissoc not declared")?;
                let result = self
                    .builder
                    .build_call(dissoc_fn, &[map_val.into(), key_val.into()], "dissoc_call")
                    .unwrap();
                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "concat" => {
                if args.is_empty() {
                    // (concat) with no args returns empty vector
                    let vec_empty_fn = self
                        .module
                        .get_function("clorus_vector_empty")
                        .ok_or("clorus_vector_empty not declared")?;
                    let result = self
                        .builder
                        .build_call(vec_empty_fn, &[], "empty_vec")
                        .unwrap();
                    return Ok(result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value());
                }

                // (concat coll1 coll2 ...) - concatenate multiple collections
                // For now, support 2 collections
                if args.len() == 2 {
                    let coll1 = self.compile_expr(&args[0])?;
                    let coll2 = self.compile_expr(&args[1])?;

                    // Build a vector containing the two collections
                    let vec_empty_fn = self
                        .module
                        .get_function("clorus_vector_empty")
                        .ok_or("clorus_vector_empty not declared")?;
                    let vec_conj_fn = self
                        .module
                        .get_function("clorus_vector_conj")
                        .ok_or("clorus_vector_conj not declared")?;

                    let vec = self
                        .builder
                        .build_call(vec_empty_fn, &[], "concat_vec")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                    let vec = self
                        .builder
                        .build_call(vec_conj_fn, &[vec.into(), coll1.into()], "vec1")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                    let vec = self
                        .builder
                        .build_call(vec_conj_fn, &[vec.into(), coll2.into()], "vec2")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    let concat_fn = self
                        .module
                        .get_function("clorus_concat")
                        .ok_or("clorus_concat not declared")?;
                    let result = self
                        .builder
                        .build_call(concat_fn, &[vec.into()], "concat_call")
                        .unwrap();
                    Ok(result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value())
                } else {
                    // For more than 2 args, build a vector of all collections
                    let vec_empty_fn = self
                        .module
                        .get_function("clorus_vector_empty")
                        .ok_or("clorus_vector_empty not declared")?;
                    let vec_conj_fn = self
                        .module
                        .get_function("clorus_vector_conj")
                        .ok_or("clorus_vector_conj not declared")?;

                    let mut vec = self
                        .builder
                        .build_call(vec_empty_fn, &[], "concat_vec")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    for arg in args {
                        let coll = self.compile_expr(arg)?;
                        vec = self
                            .builder
                            .build_call(vec_conj_fn, &[vec.into(), coll.into()], "vec_conj")
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    }

                    let concat_fn = self
                        .module
                        .get_function("clorus_concat")
                        .ok_or("clorus_concat not declared")?;
                    let result = self
                        .builder
                        .build_call(concat_fn, &[vec.into()], "concat_call")
                        .unwrap();
                    Ok(result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value())
                }
            }

            // String operations
            "str" => {
                // str takes variable args and concatenates them
                // Create a vector of the arguments
                let vec_empty_fn = self
                    .module
                    .get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let mut vec_val = self
                    .builder
                    .build_call(vec_empty_fn, &[], "str_vec")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add each argument to the vector
                let vec_conj_fn = self
                    .module
                    .get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;
                for arg in args {
                    let arg_val = self.compile_expr(arg)?;
                    vec_val = self
                        .builder
                        .build_call(
                            vec_conj_fn,
                            &[vec_val.into(), arg_val.into()],
                            "str_vec_conj",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                // Call clorus_str with the vector
                let str_fn = self
                    .module
                    .get_function("clorus_str")
                    .ok_or("clorus_str not declared")?;
                let result = self
                    .builder
                    .build_call(str_fn, &[vec_val.into()], "str_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "subs" => {
                // subs takes 2 or 3 args: string, start, [end]
                if args.len() < 2 || args.len() > 3 {
                    return Err("subs requires 2 or 3 arguments: string, start, [end]".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let start_val = self.compile_expr(&args[1])?;
                let start_float = self.unbox_number(start_val);
                let start_i64 = self
                    .builder
                    .build_float_to_signed_int(start_float, self.context.i64_type(), "start_to_i64")
                    .unwrap();

                if args.len() == 2 {
                    // Call clorus_subs2
                    let subs_fn = self
                        .module
                        .get_function("clorus_subs2")
                        .ok_or("clorus_subs2 not declared")?;
                    let result = self
                        .builder
                        .build_call(subs_fn, &[str_val.into(), start_i64.into()], "subs2_call")
                        .unwrap();
                    Ok(result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value())
                } else {
                    // Call clorus_subs3
                    let end_val = self.compile_expr(&args[2])?;
                    let end_float = self.unbox_number(end_val);
                    let end_i64 = self
                        .builder
                        .build_float_to_signed_int(end_float, self.context.i64_type(), "end_to_i64")
                        .unwrap();

                    let subs_fn = self
                        .module
                        .get_function("clorus_subs3")
                        .ok_or("clorus_subs3 not declared")?;
                    let result = self
                        .builder
                        .build_call(
                            subs_fn,
                            &[str_val.into(), start_i64.into(), end_i64.into()],
                            "subs3_call",
                        )
                        .unwrap();
                    Ok(result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value())
                }
            }

            "split" => {
                // split takes 2 args: string, delimiter
                if args.len() != 2 {
                    return Err("split requires 2 arguments: string, delimiter".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let delim_val = self.compile_expr(&args[1])?;

                let split_fn = self
                    .module
                    .get_function("clorus_split")
                    .ok_or("clorus_split not declared")?;
                let result = self
                    .builder
                    .build_call(split_fn, &[str_val.into(), delim_val.into()], "split_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "join" => {
                // join takes 2 args: separator, collection
                if args.len() != 2 {
                    return Err("join requires 2 arguments: separator, collection".to_string());
                }

                let sep_val = self.compile_expr(&args[0])?;
                let coll_val = self.compile_expr(&args[1])?;

                let join_fn = self
                    .module
                    .get_function("clorus_join")
                    .ok_or("clorus_join not declared")?;
                let result = self
                    .builder
                    .build_call(join_fn, &[sep_val.into(), coll_val.into()], "join_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "upper-case" => {
                // upper-case takes 1 arg: string
                if args.len() != 1 {
                    return Err("upper-case requires 1 argument: string".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;

                let upper_fn = self
                    .module
                    .get_function("clorus_upper_case")
                    .ok_or("clorus_upper_case not declared")?;
                let result = self
                    .builder
                    .build_call(upper_fn, &[str_val.into()], "upper_case_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "lower-case" => {
                // lower-case takes 1 arg: string
                if args.len() != 1 {
                    return Err("lower-case requires 1 argument: string".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;

                let lower_fn = self
                    .module
                    .get_function("clorus_lower_case")
                    .ok_or("clorus_lower_case not declared")?;
                let result = self
                    .builder
                    .build_call(lower_fn, &[str_val.into()], "lower_case_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "trim" => {
                // trim takes 1 arg: string
                if args.len() != 1 {
                    return Err("trim requires 1 argument: string".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;

                let trim_fn = self
                    .module
                    .get_function("clorus_trim")
                    .ok_or("clorus_trim not declared")?;
                let result = self
                    .builder
                    .build_call(trim_fn, &[str_val.into()], "trim_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "trim-left" => {
                // trim-left takes 1 arg: string
                if args.len() != 1 {
                    return Err("trim-left requires 1 argument: string".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;

                let trim_fn = self
                    .module
                    .get_function("clorus_trim_left")
                    .ok_or("clorus_trim_left not declared")?;
                let result = self
                    .builder
                    .build_call(trim_fn, &[str_val.into()], "trim_left_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "trim-right" => {
                // trim-right takes 1 arg: string
                if args.len() != 1 {
                    return Err("trim-right requires 1 argument: string".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;

                let trim_fn = self
                    .module
                    .get_function("clorus_trim_right")
                    .ok_or("clorus_trim_right not declared")?;
                let result = self
                    .builder
                    .build_call(trim_fn, &[str_val.into()], "trim_right_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "replace" => {
                // replace takes 3 args: string, match, replacement
                if args.len() != 3 {
                    return Err(
                        "replace requires 3 arguments: string, match, replacement".to_string()
                    );
                }

                let str_val = self.compile_expr(&args[0])?;
                let match_val = self.compile_expr(&args[1])?;
                let repl_val = self.compile_expr(&args[2])?;

                let replace_fn = self
                    .module
                    .get_function("clorus_replace")
                    .ok_or("clorus_replace not declared")?;
                let result = self
                    .builder
                    .build_call(
                        replace_fn,
                        &[str_val.into(), match_val.into(), repl_val.into()],
                        "replace_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "replace-first" => {
                // replace-first takes 3 args: string, match, replacement
                if args.len() != 3 {
                    return Err(
                        "replace-first requires 3 arguments: string, match, replacement"
                            .to_string(),
                    );
                }

                let str_val = self.compile_expr(&args[0])?;
                let match_val = self.compile_expr(&args[1])?;
                let repl_val = self.compile_expr(&args[2])?;

                let replace_fn = self
                    .module
                    .get_function("clorus_replace_first")
                    .ok_or("clorus_replace_first not declared")?;
                let result = self
                    .builder
                    .build_call(
                        replace_fn,
                        &[str_val.into(), match_val.into(), repl_val.into()],
                        "replace_first_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }
            "re-find" => {
                if args.len() != 2 {
                    return Err("re-find requires 2 arguments: pattern, string".to_string());
                }

                let pattern = self.compile_expr(&args[0])?;
                let input = self.compile_expr(&args[1])?;

                let re_find_fn = self
                    .module
                    .get_function("clorus_re_find")
                    .ok_or("clorus_re_find not declared")?;
                let result = self
                    .builder
                    .build_call(re_find_fn, &[pattern.into(), input.into()], "re_find_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }
            "re-matches" => {
                if args.len() != 2 {
                    return Err("re-matches requires 2 arguments: pattern, string".to_string());
                }

                let pattern = self.compile_expr(&args[0])?;
                let input = self.compile_expr(&args[1])?;

                let re_matches_fn = self
                    .module
                    .get_function("clorus_re_matches")
                    .ok_or("clorus_re_matches not declared")?;
                let result = self
                    .builder
                    .build_call(
                        re_matches_fn,
                        &[pattern.into(), input.into()],
                        "re_matches_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }
            "re-seq" => {
                if args.len() != 2 {
                    return Err("re-seq requires 2 arguments: pattern, string".to_string());
                }

                let pattern = self.compile_expr(&args[0])?;
                let input = self.compile_expr(&args[1])?;

                let re_seq_fn = self
                    .module
                    .get_function("clorus_re_seq")
                    .ok_or("clorus_re_seq not declared")?;
                let result = self
                    .builder
                    .build_call(re_seq_fn, &[pattern.into(), input.into()], "re_seq_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }
            "re-replace" => {
                if args.len() != 3 {
                    return Err("re-replace requires 3 arguments: string, pattern, replacement".to_string());
                }

                let input = self.compile_expr(&args[0])?;
                let pattern = self.compile_expr(&args[1])?;
                let replacement = self.compile_expr(&args[2])?;

                let re_replace_fn = self
                    .module
                    .get_function("clorus_re_replace")
                    .ok_or("clorus_re_replace not declared")?;
                let result = self
                    .builder
                    .build_call(
                        re_replace_fn,
                        &[input.into(), pattern.into(), replacement.into()],
                        "re_replace_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }
            "re-replace-first" => {
                if args.len() != 3 {
                    return Err(
                        "re-replace-first requires 3 arguments: string, pattern, replacement"
                            .to_string(),
                    );
                }

                let input = self.compile_expr(&args[0])?;
                let pattern = self.compile_expr(&args[1])?;
                let replacement = self.compile_expr(&args[2])?;

                let re_replace_first_fn = self
                    .module
                    .get_function("clorus_re_replace_first")
                    .ok_or("clorus_re_replace_first not declared")?;
                let result = self
                    .builder
                    .build_call(
                        re_replace_first_fn,
                        &[input.into(), pattern.into(), replacement.into()],
                        "re_replace_first_call",
                    )
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            // I/O operations
            "print" => {
                // print takes 1 arg: value to print
                if args.len() != 1 {
                    return Err("print requires 1 argument: value".to_string());
                }

                let val = self.compile_expr(&args[0])?;

                let print_fn = self
                    .module
                    .get_function("clorus_print")
                    .ok_or("clorus_print not declared")?;
                let result = self
                    .builder
                    .build_call(print_fn, &[val.into()], "print_call")
                    .unwrap();

                Ok(result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            "println" => {
                // println is variadic - takes any number of arguments
                if args.is_empty() {
                    // No arguments - just print newline
                    let println_fn = self
                        .module
                        .get_function("clorus_println_variadic")
                        .ok_or("clorus_println_variadic not declared")?;

                    // Pass null for empty args
                    let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
                    let null_ptr = i8_ptr_type.const_null();

                    let result = self
                        .builder
                        .build_call(println_fn, &[null_ptr.into()], "println_call")
                        .unwrap();
                    return Ok(result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value());
                } else if args.len() == 1 {
                    // Single argument - use optimized single-arg version
                    let val = self.compile_expr(&args[0])?;
                    let println_fn = self
                        .module
                        .get_function("clorus_println")
                        .ok_or("clorus_println not declared")?;
                    let result = self
                        .builder
                        .build_call(println_fn, &[val.into()], "println_call")
                        .unwrap();
                    return Ok(result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value());
                } else {
                    // Multiple arguments - use variadic version
                    // Create a vector of the arguments (like str does)
                    let vec_empty_fn = self
                        .module
                        .get_function("clorus_vector_empty")
                        .ok_or("clorus_vector_empty not declared")?;
                    let mut vec_val = self
                        .builder
                        .build_call(vec_empty_fn, &[], "println_vec")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Add each argument to the vector
                    let vec_conj_fn = self
                        .module
                        .get_function("clorus_vector_conj")
                        .ok_or("clorus_vector_conj not declared")?;
                    for arg in args {
                        let arg_val = self.compile_expr(arg)?;
                        vec_val = self
                            .builder
                            .build_call(
                                vec_conj_fn,
                                &[vec_val.into(), arg_val.into()],
                                "println_vec_conj",
                            )
                            .unwrap()
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();
                    }

                    // Call variadic println with the vector
                    let println_fn = self
                        .module
                        .get_function("clorus_println_variadic")
                        .ok_or("clorus_println_variadic not declared")?;
                    let result = self
                        .builder
                        .build_call(println_fn, &[vec_val.into()], "println_call")
                        .unwrap();

                    return Ok(result
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value());
                }
            }

            "starts-with?" => {
                // starts-with? takes 2 args: string, prefix
                if args.len() != 2 {
                    return Err("starts-with? requires 2 arguments: string, prefix".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let prefix_val = self.compile_expr(&args[1])?;

                let starts_with_fn = self
                    .module
                    .get_function("clorus_starts_with")
                    .ok_or("clorus_starts_with not declared")?;
                let result = self
                    .builder
                    .build_call(
                        starts_with_fn,
                        &[str_val.into(), prefix_val.into()],
                        "starts_with_call",
                    )
                    .unwrap();

                // Convert i32 bool to Value* bool
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "bool_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "ends-with?" => {
                // ends-with? takes 2 args: string, suffix
                if args.len() != 2 {
                    return Err("ends-with? requires 2 arguments: string, suffix".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let suffix_val = self.compile_expr(&args[1])?;

                let ends_with_fn = self
                    .module
                    .get_function("clorus_ends_with")
                    .ok_or("clorus_ends_with not declared")?;
                let result = self
                    .builder
                    .build_call(
                        ends_with_fn,
                        &[str_val.into(), suffix_val.into()],
                        "ends_with_call",
                    )
                    .unwrap();

                // Convert i32 bool to Value* bool
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "bool_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            "includes?" => {
                // includes? takes 2 args: string, substring
                if args.len() != 2 {
                    return Err("includes? requires 2 arguments: string, substring".to_string());
                }

                let str_val = self.compile_expr(&args[0])?;
                let substr_val = self.compile_expr(&args[1])?;

                let includes_fn = self
                    .module
                    .get_function("clorus_includes")
                    .ok_or("clorus_includes not declared")?;
                let result = self
                    .builder
                    .build_call(
                        includes_fn,
                        &[str_val.into(), substr_val.into()],
                        "includes_call",
                    )
                    .unwrap();

                // Convert i32 bool to Value* bool
                let i32_result = result.try_as_basic_value().left().unwrap().into_int_value();
                let float_result = self
                    .builder
                    .build_unsigned_int_to_float(
                        i32_result,
                        self.context.f64_type(),
                        "bool_to_float",
                    )
                    .unwrap();

                Ok(self.box_number(float_result))
            }

            _ => Err(format!("Unknown clorus.core function: {}", func)),
        }
    }

    /// Wrap expression in a function so we can JIT execute it
    pub fn wrap_in_function(
        &mut self,
        expr: &Expr,
        fn_name: &str,
    ) -> Result<FunctionValue<'ctx>, String> {
        // Function returns Value* now
        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let fn_type = value_ptr_type.fn_type(&[], false);
        let function = self.module.add_function(fn_name, fn_type, None);

        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let result = self.compile_expr(expr)?; // Returns Value*
                                               // Ensure the entry block has a terminator. If compile_expr moved the builder,
                                               // link entry to the current block and return from there.
        let current_block = self.builder.get_insert_block();
        if let Some(block) = current_block {
            if entry.get_terminator().is_none() && entry != block {
                self.builder.position_at_end(entry);
                self.builder.build_unconditional_branch(block).unwrap();
            }
            if block.get_terminator().is_none() {
                self.builder.position_at_end(block);
                self.builder.build_return(Some(&result)).unwrap();
            }
        } else if entry.get_terminator().is_none() {
            // Fallback: if compile_expr cleared the insertion point, return from entry.
            self.builder.position_at_end(entry);
            self.builder.build_return(Some(&result)).unwrap();
        }

        Ok(function)
    }

    pub fn print_ir(&self) {
        println!("{}", self.module.print_to_string().to_string());
    }

}
