use super::*;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_add(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.is_empty() {
            // (+ ) => 0 (as Long)
            let value_long_fn = self
                .module
                .get_function("clorus_value_long")
                .ok_or("clorus_value_long not declared")?;
            let zero = self.context.i64_type().const_zero();
            let result = self
                .builder
                .build_call(value_long_fn, &[zero.into()], "zero")
                .unwrap();
            return Ok(result
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value());
        }

        // Get clorus_add function
        let add_fn = self
            .module
            .get_function("clorus_add")
            .ok_or("clorus_add not declared")?;

        // Compile first argument
        let mut result = self.compile_expr(&args[0])?;

        // Add remaining arguments using clorus_add
        for arg in &args[1..] {
            let val_ptr = self.compile_expr(arg)?;
            let call_result = self
                .builder
                .build_call(add_fn, &[result.into(), val_ptr.into()], "add")
                .unwrap();
            result = call_result
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();
        }

        Ok(result)
    }

    pub(super) fn compile_sub(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.is_empty() {
            return Err("- requires at least one argument".to_string());
        }

        let sub_fn = self
            .module
            .get_function("clorus_sub")
            .ok_or("clorus_sub not declared")?;

        let mut result = self.compile_expr(&args[0])?;

        if args.len() == 1 {
            // Unary negation: (- 5) => (0 - 5)
            let value_long_fn = self
                .module
                .get_function("clorus_value_long")
                .ok_or("clorus_value_long not declared")?;
            let zero = self.context.i64_type().const_zero();
            let zero_val = self
                .builder
                .build_call(value_long_fn, &[zero.into()], "zero")
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();

            let call_result = self
                .builder
                .build_call(sub_fn, &[zero_val.into(), result.into()], "neg")
                .unwrap();
            return Ok(call_result
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value());
        }

        // Subtract remaining arguments
        for arg in &args[1..] {
            let val_ptr = self.compile_expr(arg)?;
            let call_result = self
                .builder
                .build_call(sub_fn, &[result.into(), val_ptr.into()], "sub")
                .unwrap();
            result = call_result
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();
        }

        Ok(result)
    }

    pub(super) fn compile_mul(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.is_empty() {
            let value_long_fn = self
                .module
                .get_function("clorus_value_long")
                .ok_or("clorus_value_long not declared")?;
            let one = self.context.i64_type().const_int(1, false);
            let result = self
                .builder
                .build_call(value_long_fn, &[one.into()], "one")
                .unwrap();
            return Ok(result
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value());
        }

        let mul_fn = self
            .module
            .get_function("clorus_mul")
            .ok_or("clorus_mul not declared")?;

        let mut result = self.compile_expr(&args[0])?;

        for arg in &args[1..] {
            let val_ptr = self.compile_expr(arg)?;
            let call_result = self
                .builder
                .build_call(mul_fn, &[result.into(), val_ptr.into()], "mul")
                .unwrap();
            result = call_result
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();
        }

        Ok(result)
    }

    pub(super) fn compile_div(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() < 2 {
            return Err("/ requires at least two arguments".to_string());
        }

        let div_fn = self
            .module
            .get_function("clorus_div")
            .ok_or("clorus_div not declared")?;

        let mut result = self.compile_expr(&args[0])?;

        for arg in &args[1..] {
            let val_ptr = self.compile_expr(arg)?;
            let call_result = self
                .builder
                .build_call(div_fn, &[result.into(), val_ptr.into()], "div")
                .unwrap();
            result = call_result
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();
        }

        Ok(result)
    }

    pub(super) fn compile_mod(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("mod requires exactly two arguments".to_string());
        }

        let mod_fn = self
            .module
            .get_function("clorus_mod")
            .ok_or("clorus_mod not declared")?;

        let left = self.compile_expr(&args[0])?;
        let right = self.compile_expr(&args[1])?;

        let call_result = self
            .builder
            .build_call(mod_fn, &[left.into(), right.into()], "mod")
            .unwrap();

        Ok(call_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }

    pub(super) fn compile_lt(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("< requires exactly two arguments".to_string());
        }

        // Unbox arguments
        let left_ptr = self.compile_expr(&args[0])?;
        let right_ptr = self.compile_expr(&args[1])?;
        let left = self.unbox_number(left_ptr);
        let right = self.unbox_number(right_ptr);

        let cmp = self
            .builder
            .build_float_compare(FloatPredicate::OLT, left, right, "lt")
            .unwrap();

        // Convert bool to float: true => 1.0, false => 0.0
        let bool_as_float = self
            .builder
            .build_unsigned_int_to_float(cmp, self.context.f64_type(), "bool_to_float")
            .unwrap();

        // Box as boolean value
        let bool_fn = self
            .module
            .get_function("clorus_value_bool")
            .ok_or("clorus_value_bool not declared")?;
        let call_result = self
            .builder
            .build_call(bool_fn, &[bool_as_float.into()], "bool_value")
            .unwrap();
        Ok(call_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }

    pub(super) fn compile_gt(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("> requires exactly two arguments".to_string());
        }

        // Unbox arguments
        let left_ptr = self.compile_expr(&args[0])?;
        let right_ptr = self.compile_expr(&args[1])?;
        let left = self.unbox_number(left_ptr);
        let right = self.unbox_number(right_ptr);

        let cmp = self
            .builder
            .build_float_compare(FloatPredicate::OGT, left, right, "gt")
            .unwrap();

        // Convert bool to float: true => 1.0, false => 0.0
        let bool_as_float = self
            .builder
            .build_unsigned_int_to_float(cmp, self.context.f64_type(), "bool_to_float")
            .unwrap();

        // Box as boolean value
        let bool_fn = self
            .module
            .get_function("clorus_value_bool")
            .ok_or("clorus_value_bool not declared")?;
        let call_result = self
            .builder
            .build_call(bool_fn, &[bool_as_float.into()], "bool_value")
            .unwrap();
        Ok(call_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }

    pub(super) fn compile_lte(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("<= requires exactly two arguments".to_string());
        }

        let left_ptr = self.compile_expr(&args[0])?;
        let right_ptr = self.compile_expr(&args[1])?;
        let left = self.unbox_number(left_ptr);
        let right = self.unbox_number(right_ptr);

        let cmp = self
            .builder
            .build_float_compare(FloatPredicate::OLE, left, right, "lte")
            .unwrap();

        let bool_as_float = self
            .builder
            .build_unsigned_int_to_float(cmp, self.context.f64_type(), "bool_to_float")
            .unwrap();

        // Box as boolean value
        let bool_fn = self
            .module
            .get_function("clorus_value_bool")
            .ok_or("clorus_value_bool not declared")?;
        let call_result = self
            .builder
            .build_call(bool_fn, &[bool_as_float.into()], "bool_value")
            .unwrap();
        Ok(call_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }

    pub(super) fn compile_gte(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err(">= requires exactly two arguments".to_string());
        }

        let left_ptr = self.compile_expr(&args[0])?;
        let right_ptr = self.compile_expr(&args[1])?;
        let left = self.unbox_number(left_ptr);
        let right = self.unbox_number(right_ptr);

        let cmp = self
            .builder
            .build_float_compare(FloatPredicate::OGE, left, right, "gte")
            .unwrap();

        let bool_as_float = self
            .builder
            .build_unsigned_int_to_float(cmp, self.context.f64_type(), "bool_to_float")
            .unwrap();

        // Box as boolean value
        let bool_fn = self
            .module
            .get_function("clorus_value_bool")
            .ok_or("clorus_value_bool not declared")?;
        let call_result = self
            .builder
            .build_call(bool_fn, &[bool_as_float.into()], "bool_value")
            .unwrap();
        Ok(call_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }

    pub(super) fn compile_eq(&mut self, args: &[Expr]) -> Result<PointerValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("= requires exactly two arguments".to_string());
        }

        // Compile both arguments to Value*
        let left_ptr = self.compile_expr(&args[0])?;
        let right_ptr = self.compile_expr(&args[1])?;

        // Call runtime equality function: clorus_equals(left, right) -> bool
        let equals_fn = self
            .module
            .get_function("clorus_equals")
            .ok_or("clorus_equals not declared - runtime functions not initialized")?;

        let eq_result = self
            .builder
            .build_call(equals_fn, &[left_ptr.into(), right_ptr.into()], "eq_call")
            .unwrap();

        // clorus_equals returns bool (i1)
        let bool_result = eq_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // Convert bool to float: true => 1.0, false => 0.0
        let bool_as_float = self
            .builder
            .build_unsigned_int_to_float(bool_result, self.context.f64_type(), "bool_to_float")
            .unwrap();

        // Box as boolean value
        let bool_fn = self
            .module
            .get_function("clorus_value_bool")
            .ok_or("clorus_value_bool not declared")?;
        let call_result = self
            .builder
            .build_call(bool_fn, &[bool_as_float.into()], "bool_value")
            .unwrap();
        Ok(call_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value())
    }

}
