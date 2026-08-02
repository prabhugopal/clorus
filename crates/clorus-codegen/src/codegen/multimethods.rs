use super::*;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn multimethod_prefers(&self, name: &str, preferred: &Expr, over: &Expr) -> bool {
        self.multimethod_preferences
            .get(name)
            .map(|pairs| pairs.iter().any(|(p, o)| p == preferred && o == over))
            .unwrap_or(false)
    }

    pub(super) fn multimethod_name_from_symbol_arg(arg: &Expr) -> Option<String> {
        match arg {
            Expr::Symbol(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub(super) fn build_function_value_from_method_set(
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

}
