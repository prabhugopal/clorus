use super::*;
use clorus_syntax::ast::FunctionArity;

impl<'ctx> CodeGen<'ctx> {
    /// Wrap a function body in an implicit self-loop so that `recur` at the
    /// tail of the body gets guaranteed O(1)-stack tail recursion -- a real
    /// LLVM loop via phi nodes, the same mechanism `loop`/`recur` already
    /// uses -- instead of relying on the optimizer to turn a self-call into
    /// a tail call (which it may not do, silently blowing the stack
    /// instead).
    ///
    /// Must be called right after `entry` is created and positioned, before
    /// any parameter destructuring. Returns one value per fixed parameter
    /// position (read from this iteration's phi node -- destructure
    /// parameter patterns against these instead of the raw
    /// `function.get_nth_param(i)` values), plus, when `has_rest` is true,
    /// the current iteration's rest-vector value.
    ///
    /// For a variadic function, `recur`'s extra args (beyond the fixed
    /// params) get packed into a fresh vector each iteration and fed to the
    /// rest phi -- matching how a normal call to this function packs
    /// trailing args, so recur's arg-count convention is unchanged from
    /// before this loop scheme existed (see Expr::Recur's rest_phi branch).
    pub(super) fn setup_fn_recur_loop(
        &mut self,
        function: FunctionValue<'ctx>,
        entry: BasicBlock<'ctx>,
        param_count: usize,
        has_rest: bool,
    ) -> (Vec<PointerValue<'ctx>>, Option<PointerValue<'ctx>>) {
        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        let mut param_values = Vec::with_capacity(param_count);
        for i in 0..param_count {
            param_values.push(function.get_nth_param(i as u32).unwrap().into_pointer_value());
        }
        let rest_value = if has_rest {
            Some(
                function
                    .get_nth_param(param_count as u32)
                    .unwrap()
                    .into_pointer_value(),
            )
        } else {
            None
        };

        let loop_start = self.context.append_basic_block(function, "fn_recur_start");
        self.builder.build_unconditional_branch(loop_start).unwrap();
        self.builder.position_at_end(loop_start);

        let mut phi_nodes = Vec::with_capacity(param_count);
        let mut phi_values = Vec::with_capacity(param_count);
        for (i, pv) in param_values.iter().enumerate() {
            let phi = self
                .builder
                .build_phi(value_ptr_type, &format!("fn_recur_param_{}", i))
                .unwrap();
            phi.add_incoming(&[(pv, entry)]);
            phi_values.push(phi.as_basic_value().into_pointer_value());
            phi_nodes.push(phi);
        }

        let (rest_phi, rest_phi_value) = if let Some(rv) = rest_value {
            let phi = self
                .builder
                .build_phi(value_ptr_type, "fn_recur_rest")
                .unwrap();
            phi.add_incoming(&[(&rv, entry)]);
            (Some(phi), Some(phi.as_basic_value().into_pointer_value()))
        } else {
            (None, None)
        };

        // LoopContext requires a loop_end block, but a function body returns
        // directly from wherever it ends up (see callers below) rather than
        // routing through a value-merging exit like Expr::Loop does -- so
        // this block is provably unreachable. Give it a terminator so LLVM's
        // verifier is satisfied, then never target it.
        let loop_end = self.context.append_basic_block(function, "fn_recur_unused_end");
        let resume = self.builder.get_insert_block();
        self.builder.position_at_end(loop_end);
        self.builder.build_unreachable().unwrap();
        if let Some(block) = resume {
            self.builder.position_at_end(block);
        }

        let binding_names = (0..param_count).map(|i| format!("__fn_recur_{}", i)).collect();
        self.loop_context = Some(LoopContext {
            loop_start,
            loop_end,
            binding_names,
            phi_nodes,
            rest_phi,
        });

        (phi_values, rest_phi_value)
    }

    pub(super) fn compile_defn_expr(
        &mut self,
        name: &str,
        params: &[Pattern],
        rest_param: &Option<String>,
        body: &Expr,
        metadata: &Option<Vec<(Expr, Expr)>>,
    ) -> Result<PointerValue<'ctx>, String> {
        // Generate mangled name based on current namespace
        // math/add -> clorus_math_add
        let mangled_name = if self.namespace.current == "user" {
            // In default namespace, use simple name
            name.to_string()
        } else {
            format!(
                "clorus_{}_{}",
                self.namespace.current.replace('.', "_").replace('-', "_"),
                name.replace('-', "_")
            )
        };

        // If this function was forward-declared, remove the placeholder first
        // LLVM doesn't allow replacing a function with a different signature
        if self.forward_declarations.contains(name) {
            if let Some(old_func) = self.module.get_function(&mangled_name) {
                // Remove from function table
                self.functions.remove(&mangled_name);
                // Delete the LLVM function declaration
                unsafe {
                    old_func.delete();
                }
            }
        }

        // Create function type: all parameters are Value*, return is Value*
        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // Fixed parameters
        let mut param_types: Vec<_> =
            params.iter().map(|_| value_ptr_type.into()).collect();

        // Variadic functions receive collected rest args as a vector
        if rest_param.is_some() {
            param_types.push(value_ptr_type.into());
        }

        // Add environment parameter as the LAST parameter (for consistency with runtime)
        // Even top-level defn functions need this to match the calling convention
        param_types.push(value_ptr_type.into());

        let fn_type = value_ptr_type.fn_type(&param_types, false);
        let function = self.module.add_function(&mangled_name, fn_type, None);

        // Add function to table BEFORE compiling body (for recursion)
        self.functions.insert(mangled_name.clone(), function);
        self.function_signatures
            .insert(mangled_name.clone(), (params.len(), rest_param.is_some()));

        // Save current state
        let saved_vars = self.variables.clone();
        let saved_block = self.builder.get_insert_block();

        // Create entry block for the function
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Clear variables for function scope
        let saved_loop_context = self.loop_context.clone();
        self.loop_context = None;
        self.variables.clear();

        // Bind fixed parameters to allocas (parameters are now Value*)
        // Support destructuring in function parameters
        // Track parameter bindings for nested closures to capture.
        //
        // Guaranteed-TCO path: wrap the body in an implicit self-loop so
        // recur at the tail is a real loop back-edge, not a self-call.
        let (phi_values, rest_phi_value) =
            self.setup_fn_recur_loop(function, entry, params.len(), rest_param.is_some());
        let mut param_bindings = Vec::new();
        for (param_pattern, phi_val) in params.iter().zip(phi_values.iter()) {
            let bindings = self.destructure_pattern(param_pattern, *phi_val)?;
            param_bindings.extend(bindings);
        }

        // Populate parameter_context so nested closures can capture these parameters
        // This is critical for transducers which use nested multi-arity closures
        self.parameter_context.clear();
        for (name, alloca) in &param_bindings {
            self.parameter_context.insert(name.clone(), *alloca);
        }

        // Handle rest parameter if present
        if let Some(rest_name) = rest_param {
            let rest_vec = rest_phi_value.expect("rest_phi_value set when rest_param is Some");
            let rest_alloca = self.create_entry_block_alloca(rest_name);
            self.builder.build_store(rest_alloca, rest_vec).unwrap();
            self.variables.insert(rest_name.clone(), rest_alloca);
        }

        // Compile function body (returns Value*)
        let saved_recur_ctx = self.current_recur_fn.clone();
        self.current_recur_fn = Some(RecurFnContext {
            name: name.to_string(),
            fixed_param_count: params.len(),
            has_rest_param: rest_param.is_some(),
        });
        let result = self.compile_expr(body)?;
        self.current_recur_fn = saved_recur_ctx;
        self.loop_context = saved_loop_context;
        self.builder.build_return(Some(&result)).unwrap();

        // Clear parameter context before restoring variables
        self.parameter_context.clear();

        // Restore previous state
        self.variables = saved_vars;
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }

        // Create a function Value and store it in a Var-backed global so
        // #'name and (meta #'name) work for defn as well.
        let fn_value = {
            let function_ptr = function.as_global_value().as_pointer_value();
            let function_ptr_as_i8 = self
                .builder
                .build_pointer_cast(function_ptr, value_ptr_type, "defn_func_ptr_cast")
                .unwrap();
            let function_new_fn = self
                .module
                .get_function("clorus_function_new")
                .ok_or("clorus_function_new not declared")?;
            let runtime_arity = Self::runtime_arity(params.len(), rest_param.is_some());
            let arity_val = self
                .context
                .i32_type()
                .const_int(runtime_arity as u64, true);
            let null_env = value_ptr_type
                .ptr_type(AddressSpace::default())
                .const_null();
            let env_size = self.context.i32_type().const_zero();
            self.builder
                .build_call(
                    function_new_fn,
                    &[
                        function_ptr_as_i8.into(),
                        arity_val.into(),
                        null_env.into(),
                        env_size.into(),
                    ],
                    "defn_value",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value()
        };

        let stored_val = {
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
                .build_global_string_ptr(name, "defn_var_name")
                .unwrap();
            let name_val = self
                .builder
                .build_call(
                    str_fn,
                    &[name_str.as_pointer_value().into()],
                    "defn_var_name_val",
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
                    "defn_var_ptr",
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
                        .build_global_string_ptr(&key_name, "defn_meta_key")
                        .unwrap();
                    let key_val = self
                        .builder
                        .build_call(
                            str_fn,
                            &[key_str.as_pointer_value().into()],
                            "defn_meta_key_val",
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
                            "defn_set_meta",
                        )
                        .unwrap();
                }
            }

            self.builder
                .build_call(value_from_var_fn, &[var_ptr.into()], "defn_var_value")
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value()
        };

        // Create or update global Value* for this defn.
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
            .build_call(retain_fn, &[stored_val.into()], "retain_defn_val")
            .unwrap();

        let old_val = self
            .builder
            .build_load(value_ptr_type, global.as_pointer_value(), "old_defn_val")
            .unwrap()
            .into_pointer_value();
        let null_ptr = value_ptr_type.const_null();
        let is_null = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                old_val,
                null_ptr,
                "old_defn_is_null",
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
            .ok_or("No current function for defn")?;
        let release_block = self
            .context
            .append_basic_block(current_fn, "defn_release_old");
        let cont_block = self
            .context
            .append_basic_block(current_fn, "defn_store_new");
        self.builder
            .build_conditional_branch(is_null, cont_block, release_block)
            .unwrap();
        self.builder.position_at_end(release_block);
        self.builder
            .build_call(release_fn, &[old_val.into()], "release_old_defn")
            .unwrap();
        self.builder.build_unconditional_branch(cont_block).unwrap();
        self.builder.position_at_end(cont_block);
        self.builder
            .build_store(global.as_pointer_value(), stored_val)
            .unwrap();

        // Return a symbol representing the var: #'namespace/function-name
        let var_name = if self.namespace.current == "user" {
            format!("#'{}", name)
        } else {
            format!("#'{}/{}", self.namespace.current, name)
        };

        // Create symbol (symbols are implemented as keywords in runtime)
        let keyword_fn = self
            .module
            .get_function("clorus_keyword")
            .ok_or("clorus_keyword not declared")?;
        let var_str = self
            .builder
            .build_global_string_ptr(&var_name, "var_name")
            .expect("Failed to build var name string");
        let symbol_result = self
            .builder
            .build_call(
                keyword_fn,
                &[var_str.as_pointer_value().into()],
                "var_symbol",
            )
            .unwrap();

        Ok(symbol_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }

    pub(super) fn compile_defn_multi_expr(
        &mut self,
        name: &str,
        arities: &[FunctionArity],
        metadata: &Option<Vec<(Expr, Expr)>>,
    ) -> Result<PointerValue<'ctx>, String> {
        // Multi-arity functions: generate one function per arity
        // Each arity gets a mangled name: foo_arity_0, foo_arity_1, foo_arity_2

        let base_name = if self.namespace.current == "user" {
            name.to_string()
        } else {
            format!(
                "clorus_{}_{}",
                self.namespace.current.replace('.', "_").replace('-', "_"),
                name.replace('-', "_")
            )
        };

        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // STEP 1: Create all LLVM function declarations first (for mutual/self recursion)
        let mut arity_functions = Vec::new();
        for arity in arities.iter() {
            let arity_name =
                Self::arity_variant_name(&base_name, arity.params.len(), arity.rest_param.is_some());

            // Create parameter types for this arity
            let mut param_types: Vec<_> =
                arity.params.iter().map(|_| value_ptr_type.into()).collect();

            if arity.rest_param.is_some() {
                param_types.push(value_ptr_type.into());
            }

            // Add environment parameter as the LAST parameter (for consistency with runtime)
            param_types.push(value_ptr_type.into());

            let fn_type = value_ptr_type.fn_type(&param_types, false);
            let function = self.module.add_function(&arity_name, fn_type, None);

            arity_functions.push((arity_name.clone(), function));

            // Register this arity function immediately so bodies can reference it
            self.functions.insert(arity_name.clone(), function);
            self.function_signatures.insert(
                arity_name,
                (arity.params.len(), arity.rest_param.is_some()),
            );
        }

        // Register the last arity under the base name for backward compatibility
        if let Some((_, last_fn)) = arity_functions.last() {
            self.functions.insert(base_name.clone(), *last_fn);
        }

        // STEP 2: Now compile all function bodies (they can call any arity including themselves)
        for (arity_index, arity) in arities.iter().enumerate() {
            let function = arity_functions[arity_index].1;

            // Save current state
            let saved_vars = self.variables.clone();
            let saved_block = self.builder.get_insert_block();

            // Create entry block
            let entry = self.context.append_basic_block(function, "entry");
            self.builder.position_at_end(entry);

            // Clear variables
            let saved_loop_context = self.loop_context.clone();
            self.loop_context = None;
            self.variables.clear();

            // Bind parameters with destructuring support.
            // Guaranteed-TCO path: see setup_fn_recur_loop's doc comment.
            let (phi_values, rest_phi_value) = self.setup_fn_recur_loop(
                function,
                entry,
                arity.params.len(),
                arity.rest_param.is_some(),
            );
            for (param_pattern, phi_val) in arity.params.iter().zip(phi_values.iter()) {
                self.destructure_pattern(param_pattern, *phi_val)?;
            }

            // Handle rest parameter if present
            if let Some(rest_name) = &arity.rest_param {
                let rest_vec = rest_phi_value.expect("rest_phi_value set when rest_param is Some");
                let rest_alloca = self.create_entry_block_alloca(rest_name);
                self.builder.build_store(rest_alloca, rest_vec).unwrap();
                self.variables.insert(rest_name.clone(), rest_alloca);
            }

            // Compile body
            let saved_recur_ctx = self.current_recur_fn.clone();
            self.current_recur_fn = Some(RecurFnContext {
                name: name.to_string(),
                fixed_param_count: arity.params.len(),
                has_rest_param: arity.rest_param.is_some(),
            });
            let result = self.compile_expr(&arity.body)?;
            self.current_recur_fn = saved_recur_ctx;
            self.loop_context = saved_loop_context;
            self.builder.build_return(Some(&result)).unwrap();

            // Restore state
            self.variables = saved_vars.clone();
            if let Some(block) = saved_block {
                self.builder.position_at_end(block);
            }
        }

        // Build a multi-arity function value and store it in Var-backed global.
        let multi_fn_value = {
            let i32_type = self.context.i32_type();
            let arity_variant_type = self
                .context
                .struct_type(&[i32_type.into(), value_ptr_type.into()], false);
            let arity_variants_array_type =
                arity_variant_type.array_type(arity_functions.len() as u32);
            let arity_variants_array = self
                .builder
                .build_alloca(arity_variants_array_type, "defn_multi_arity_variants")
                .unwrap();

            for (i, (_, function)) in arity_functions.iter().enumerate() {
                let variant_ptr = unsafe {
                    self.builder
                        .build_gep(
                            arity_variants_array_type,
                            arity_variants_array,
                            &[i32_type.const_zero(), i32_type.const_int(i as u64, false)],
                            &format!("defn_multi_variant_{}", i),
                        )
                        .unwrap()
                };

                let arity_field_ptr = unsafe {
                    self.builder
                        .build_gep(
                            arity_variant_type,
                            variant_ptr,
                            &[i32_type.const_zero(), i32_type.const_zero()],
                            "defn_multi_arity_field",
                        )
                        .unwrap()
                };
                let runtime_arity = Self::runtime_arity(
                    arities[i].params.len(),
                    arities[i].rest_param.is_some(),
                );
                let arity_val = i32_type.const_int(runtime_arity as u64, true);
                self.builder
                    .build_store(arity_field_ptr, arity_val)
                    .unwrap();

                let func_ptr_field_ptr = unsafe {
                    self.builder
                        .build_gep(
                            arity_variant_type,
                            variant_ptr,
                            &[i32_type.const_zero(), i32_type.const_int(1, false)],
                            "defn_multi_func_ptr_field",
                        )
                        .unwrap()
                };
                let fn_ptr = function.as_global_value().as_pointer_value();
                self.builder
                    .build_store(func_ptr_field_ptr, fn_ptr)
                    .unwrap();
            }

            let arities_ptr = self
                .builder
                .build_pointer_cast(
                    arity_variants_array,
                    value_ptr_type,
                    "defn_multi_arities_ptr",
                )
                .unwrap();
            let multi_arity_function_new_fn = self
                .module
                .get_function("clorus_multi_arity_function_new")
                .ok_or("clorus_multi_arity_function_new not declared")?;
            let arity_count = i32_type.const_int(arity_functions.len() as u64, false);
            let env_ptr = value_ptr_type
                .ptr_type(AddressSpace::default())
                .const_null();
            let env_size = i32_type.const_zero();
            self.builder
                .build_call(
                    multi_arity_function_new_fn,
                    &[
                        arities_ptr.into(),
                        arity_count.into(),
                        env_ptr.into(),
                        env_size.into(),
                    ],
                    "defn_multi_value",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value()
        };

        let stored_val = {
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
                .build_global_string_ptr(name, "defn_multi_var_name")
                .unwrap();
            let name_val = self
                .builder
                .build_call(
                    str_fn,
                    &[name_str.as_pointer_value().into()],
                    "defn_multi_var_name_val",
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
                    &[name_val.into(), multi_fn_value.into()],
                    "defn_multi_var_ptr",
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
                        .build_global_string_ptr(&key_name, "defn_multi_meta_key")
                        .unwrap();
                    let key_val = self
                        .builder
                        .build_call(
                            str_fn,
                            &[key_str.as_pointer_value().into()],
                            "defn_multi_meta_key_val",
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
                            "defn_multi_set_meta",
                        )
                        .unwrap();
                }
            }

            self.builder
                .build_call(value_from_var_fn, &[var_ptr.into()], "defn_multi_var_value")
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value()
        };

        let global = if let Some(existing_global) = self.globals.get(&base_name) {
            *existing_global
        } else {
            let global = self.module.add_global(
                value_ptr_type,
                Some(AddressSpace::default()),
                &base_name,
            );
            global.set_initializer(&value_ptr_type.const_null());
            self.globals.insert(base_name.clone(), global);
            global
        };

        let retain_fn = self
            .module
            .get_function("clorus_retain")
            .ok_or("clorus_retain not declared")?;
        self.builder
            .build_call(retain_fn, &[stored_val.into()], "retain_defn_multi_val")
            .unwrap();

        let old_val = self
            .builder
            .build_load(
                value_ptr_type,
                global.as_pointer_value(),
                "old_defn_multi_val",
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
                "old_defn_multi_is_null",
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
            .ok_or("No current function for defn multi")?;
        let release_block = self
            .context
            .append_basic_block(current_fn, "defn_multi_release_old");
        let cont_block = self
            .context
            .append_basic_block(current_fn, "defn_multi_store_new");
        self.builder
            .build_conditional_branch(is_null, cont_block, release_block)
            .unwrap();
        self.builder.position_at_end(release_block);
        self.builder
            .build_call(release_fn, &[old_val.into()], "release_old_defn_multi")
            .unwrap();
        self.builder.build_unconditional_branch(cont_block).unwrap();
        self.builder.position_at_end(cont_block);
        self.builder
            .build_store(global.as_pointer_value(), stored_val)
            .unwrap();

        // Return a symbol representing the var: #'namespace/function-name
        let var_name = if self.namespace.current == "user" {
            format!("#'{}", name)
        } else {
            format!("#'{}/{}", self.namespace.current, name)
        };

        // Create symbol (symbols are implemented as keywords in runtime)
        let keyword_fn = self
            .module
            .get_function("clorus_keyword")
            .ok_or("clorus_keyword not declared")?;
        let var_str = self
            .builder
            .build_global_string_ptr(&var_name, "var_name")
            .expect("Failed to build var name string");
        let symbol_result = self
            .builder
            .build_call(
                keyword_fn,
                &[var_str.as_pointer_value().into()],
                "var_symbol",
            )
            .unwrap();

        Ok(symbol_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }
}
