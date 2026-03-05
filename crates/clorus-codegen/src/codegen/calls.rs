use super::*;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    // ===== Code Generation Helpers for compile_core_call =====
    // These reduce boilerplate in the large match statement

    /// Helper: Compile a simple 1-argument core function call
    /// Pattern: (func-name arg) => clorus_func_name(compile(arg))
    pub(super) fn compile_simple_1arg_call(
        &mut self,
        func_display_name: &str,
        runtime_fn_name: &str,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!("{} requires 1 argument", func_display_name));
        }
        let arg = self.compile_expr(&args[0])?;
        self.call_runtime_fn(
            runtime_fn_name,
            &[arg.into()],
            &format!("{}_call", func_display_name),
        )
    }

    /// Helper: Compile a simple 2-argument core function call
    /// Pattern: (func-name arg1 arg2) => clorus_func_name(compile(arg1), compile(arg2))
    pub(super) fn compile_simple_2arg_call(
        &mut self,
        func_display_name: &str,
        runtime_fn_name: &str,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!("{} requires 2 arguments", func_display_name));
        }
        let arg1 = self.compile_expr(&args[0])?;
        let arg2 = self.compile_expr(&args[1])?;
        self.call_runtime_fn(
            runtime_fn_name,
            &[arg1.into(), arg2.into()],
            &format!("{}_call", func_display_name),
        )
    }

    /// Helper: Compile a simple 3-argument core function call
    /// Pattern: (func-name arg1 arg2 arg3) => clorus_func_name(compile(arg1), compile(arg2), compile(arg3))
    pub(super) fn compile_simple_3arg_call(
        &mut self,
        func_display_name: &str,
        runtime_fn_name: &str,
        args: &[Expr],
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
            &format!("{}_call", func_display_name),
        )
    }

    /// Helper: compile unary predicate that returns integer-like truthy and box as Value bool.
    pub(super) fn compile_unary_predicate_call(
        &mut self,
        func_display_name: &str,
        runtime_fn_name: &str,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!("{} requires 1 argument: value", func_display_name));
        }

        let val = self.compile_expr(&args[0])?;
        let pred_fn = self
            .module
            .get_function(runtime_fn_name)
            .ok_or(format!("{} not declared", runtime_fn_name))?;
        let result = self
            .builder
            .build_call(
                pred_fn,
                &[val.into()],
                &format!("{}_call", func_display_name.replace('?', "_qmark")),
            )
            .unwrap();

        let int_result = result
            .try_as_basic_value()
            .left()
            .ok_or(format!("{} returned no value", runtime_fn_name))?
            .into_int_value();
        // C `_Bool` only guarantees the least-significant bit.
        let normalized = self
            .builder
            .build_and(
                int_result,
                int_result.get_type().const_int(1, false),
                "pred_bool_lsb",
            )
            .unwrap();
        let zero = int_result.get_type().const_zero();
        let bool_val = self
            .builder
            .build_int_compare(IntPredicate::NE, normalized, zero, "pred_bool_val")
            .unwrap();

        let bool_fn = self
            .module
            .get_function("clorus_value_boolean")
            .ok_or("clorus_value_boolean not declared")?;
        let boxed = self
            .builder
            .build_call(bool_fn, &[bool_val.into()], "pred_boxed_bool")
            .unwrap();

        Ok(boxed
            .try_as_basic_value()
            .left()
            .ok_or("pred_boxed_bool returned no value")?
            .into_pointer_value())
    }

    /// Helper: compile binary predicate that returns integer-like truthy and box as Value bool.
    pub(super) fn compile_binary_predicate_call(
        &mut self,
        func_display_name: &str,
        runtime_fn_name: &str,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!("{} requires 2 arguments", func_display_name));
        }

        let left = self.compile_expr(&args[0])?;
        let right = self.compile_expr(&args[1])?;

        let pred_fn = self
            .module
            .get_function(runtime_fn_name)
            .ok_or(format!("{} not declared", runtime_fn_name))?;
        let result = self
            .builder
            .build_call(
                pred_fn,
                &[left.into(), right.into()],
                &format!("{}_call", func_display_name.replace('?', "_qmark")),
            )
            .unwrap();

        let int_result = result
            .try_as_basic_value()
            .left()
            .ok_or(format!("{} returned no value", runtime_fn_name))?
            .into_int_value();
        let normalized = self
            .builder
            .build_and(
                int_result,
                int_result.get_type().const_int(1, false),
                "pred2_bool_lsb",
            )
            .unwrap();
        let zero = int_result.get_type().const_zero();
        let bool_val = self
            .builder
            .build_int_compare(IntPredicate::NE, normalized, zero, "pred2_bool_val")
            .unwrap();

        let bool_fn = self
            .module
            .get_function("clorus_value_boolean")
            .ok_or("clorus_value_boolean not declared")?;
        let boxed = self
            .builder
            .build_call(bool_fn, &[bool_val.into()], "pred2_boxed_bool")
            .unwrap();

        Ok(boxed
            .try_as_basic_value()
            .left()
            .ok_or("pred2_boxed_bool returned no value")?
            .into_pointer_value())
    }

    /// Helper: Compile simple core calls from a mapping table.
    pub(super) fn compile_simple_mapped_call(
        &mut self,
        func: &str,
        args: &[Expr],
    ) -> Option<Result<PointerValue<'ctx>, String>> {
        const SIMPLE_BUILTINS: &[(&str, &str, usize)] = &[
            ("first", "clorus_first", 1),
            ("rest", "clorus_rest", 1),
            ("last", "clorus_last", 1),
            ("keys", "clorus_map_keys", 1),
            ("vals", "clorus_map_vals", 1),
            ("merge", "clorus_map_merge", 1),
            ("distinct", "clorus_distinct", 1),
            ("dedupe", "clorus_dedupe", 1),
            ("flatten", "clorus_flatten", 1),
            ("interleave", "clorus_interleave", 1),
            ("interpose", "clorus_interpose", 2),
            ("parents", "clorus_parents", 1),
            ("ancestors", "clorus_ancestors", 1),
            ("descendants", "clorus_descendants", 1),
        ];

        for (name, runtime_fn, arity) in SIMPLE_BUILTINS {
            if func == *name {
                return Some(match arity {
                    1 => self.compile_simple_1arg_call(name, runtime_fn, args),
                    2 => self.compile_simple_2arg_call(name, runtime_fn, args),
                    3 => self.compile_simple_3arg_call(name, runtime_fn, args),
                    _ => Err(format!("{} has unsupported arity {}", name, arity)),
                });
            }
        }

        None
    }

    pub(super) fn find_variadic_arity_variant(
        &self,
        function_base_name: &str,
        arg_count: usize,
    ) -> Option<String> {
        let prefix = format!("{}_arity_", function_base_name);
        let mut best: Option<(usize, String)> = None;

        for (name, (fixed_params, has_rest)) in self.function_signatures.iter() {
            if !*has_rest || !name.starts_with(&prefix) || *fixed_params > arg_count {
                continue;
            }

            match &best {
                Some((best_fixed, _)) if *best_fixed >= *fixed_params => {}
                _ => best = Some((*fixed_params, name.clone())),
            }
        }

        best.map(|(_, name)| name)
    }

    pub(super) fn has_any_arity_variants(&self, function_base_name: &str) -> bool {
        let prefix = format!("{}_arity_", function_base_name);
        self.function_signatures
            .keys()
            .any(|name| name.starts_with(&prefix))
    }
}
