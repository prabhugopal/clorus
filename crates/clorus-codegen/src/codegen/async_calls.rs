use super::*;

impl<'ctx> CodeGen<'ctx> {
    /// Compile rust.example function calls (example Rust library for FFI testing)
    pub(super) fn compile_rust_example_call(
        &mut self,
        func: &str,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
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
                let add_fn = self.module.get_function("clorus_add").ok_or_else(|| {
                    "clorus_add not found - did you (use rust.example)?".to_string()
                })?;

                let result = self
                    .builder
                    .build_call(add_fn, &[x.into(), y.into()], "example_add")
                    .unwrap();

                let f64_result = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value();

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

                let multiply_fn = self.module.get_function("clorus_multiply").ok_or_else(|| {
                    "clorus_multiply not found - did you (use rust.example)?".to_string()
                })?;

                let result = self
                    .builder
                    .build_call(multiply_fn, &[a.into(), b.into()], "example_multiply")
                    .unwrap();

                let f64_result = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value();
                Ok(self.box_number(f64_result))
            }

            "example/factorial" => {
                if args.len() != 1 {
                    return Err("example/factorial requires 1 argument: n".to_string());
                }

                let n_ptr = self.compile_expr(&args[0])?;
                let n = self.unbox_number(n_ptr);

                let factorial_fn =
                    self.module
                        .get_function("clorus_factorial")
                        .ok_or_else(|| {
                            "clorus_factorial not found - did you (use rust.example)?".to_string()
                        })?;

                let result = self
                    .builder
                    .build_call(factorial_fn, &[n.into()], "example_factorial")
                    .unwrap();

                let f64_result = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value();
                Ok(self.box_number(f64_result))
            }

            _ => Err(format!("Unknown rust.example function: {}", func)),
        }
    }

    /// Compile rust.async-demo function calls

    /// Compile rust.async-demo function calls
    pub(super) fn compile_async_demo_call(
        &mut self,
        func: &str,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
        match func {
            "async-demo/hello-blocking" => {
                // hello_blocking takes no args, returns String
                if !args.is_empty() {
                    return Err("async-demo/hello-blocking takes no arguments".to_string());
                }

                let hello_fn = self
                    .module
                    .get_function("clorus_hello_blocking")
                    .ok_or_else(|| {
                        "clorus_hello_blocking not found - did you (use rust.async-demo)?"
                            .to_string()
                    })?;

                let result = self
                    .builder
                    .build_call(hello_fn, &[], "async_hello")
                    .unwrap();

                let str_ptr = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

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

                let countdown_fn = self
                    .module
                    .get_function("clorus_countdown_blocking")
                    .ok_or_else(|| {
                        "clorus_countdown_blocking not found - did you (use rust.async-demo)?"
                            .to_string()
                    })?;

                let result = self
                    .builder
                    .build_call(countdown_fn, &[n.into()], "async_countdown")
                    .unwrap();

                let f64_result = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value();
                Ok(self.box_number(f64_result))
            }

            _ => Err(format!("Unknown rust.async-demo function: {}", func)),
        }
    }

    /// Compile rust.async-hello function calls

    /// Compile rust.async-hello function calls
    pub(super) fn compile_async_hello_call(
        &mut self,
        func: &str,
        args: &[Expr],
    ) -> Result<PointerValue<'ctx>, String> {
        match func {
            "async-hello/greet-blocking" => {
                // greet_blocking takes 1 arg (name: String), returns String
                if args.len() != 1 {
                    return Err("async-hello/greet-blocking requires 1 argument: name".to_string());
                }

                let name_val = self.compile_expr(&args[0])?;
                let name_cstr = self.extract_cstring_from_value(name_val);

                let greet_fn = self
                    .module
                    .get_function("clorus_greet_blocking")
                    .ok_or_else(|| {
                        "clorus_greet_blocking not found - did you (use rust.async-hello)?"
                            .to_string()
                    })?;

                let result = self
                    .builder
                    .build_call(greet_fn, &[name_cstr.into()], "async_greet")
                    .unwrap();

                let str_ptr = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();
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

                let add_fn = self
                    .module
                    .get_function("clorus_add_blocking")
                    .ok_or_else(|| {
                        "clorus_add_blocking not found - did you (use rust.async-hello)?"
                            .to_string()
                    })?;

                let result = self
                    .builder
                    .build_call(add_fn, &[x.into(), y.into()], "async_add")
                    .unwrap();

                let f64_result = result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value();
                Ok(self.box_number(f64_result))
            }

            _ => Err(format!("Unknown rust.async-hello function: {}", func)),
        }
    }

}
