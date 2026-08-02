use super::*;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn runtime_arity(fixed_param_count: usize, has_rest_param: bool) -> i32 {
        if has_rest_param {
            // Negative arity encodes variadic with fixed prefix:
            // fixed = (-arity - 1)
            -((fixed_param_count as i32) + 1)
        } else {
            fixed_param_count as i32
        }
    }

    pub(super) fn arity_variant_name(
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

    pub(super) fn build_rest_vector_from_values(
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

    pub(super) fn build_rest_vector_from_arg_array(
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

}
