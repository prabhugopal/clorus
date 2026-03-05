use super::*;

impl<'ctx> CodeGen<'ctx> {
    /// Helper: Compile a quoted expression (returns data, not evaluated)
    pub(super) fn compile_quoted(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
        match expr {
            // Literals are returned as-is
            Expr::Long(n) => {
                let value_long_fn = self
                    .module
                    .get_function("clorus_value_long")
                    .ok_or("clorus_value_long not declared")?;
                let long_val = self.context.i64_type().const_int(*n as u64, false);
                let result = self
                    .builder
                    .build_call(value_long_fn, &[long_val.into()], "value_long")
                    .expect("Failed to build call to clorus_value_long");
                Ok(result
                    .try_as_basic_value()
                    .left()
                    .expect("clorus_value_long should return pointer")
                    .into_pointer_value())
            }
            Expr::Double(n) => {
                let float_val = self.context.f64_type().const_float(*n);
                Ok(self.box_number(float_val))
            }
            Expr::String(s) => {
                let c_str = self
                    .builder
                    .build_global_string_ptr(s, "str")
                    .expect("Failed to build global string for quoted string");
                Ok(self.box_string(c_str.as_pointer_value()))
            }
            Expr::Keyword(k) => {
                let c_str = self
                    .builder
                    .build_global_string_ptr(k, "keyword")
                    .expect("Failed to build global string for keyword");
                let keyword_fn = self
                    .module
                    .get_function("clorus_keyword")
                    .ok_or("clorus_keyword not declared")?;
                let call_result = self
                    .builder
                    .build_call(keyword_fn, &[c_str.as_pointer_value().into()], "keyword")
                    .expect("Failed to build call to clorus_keyword");
                Ok(call_result
                    .try_as_basic_value()
                    .left()
                    .expect("clorus_keyword should return pointer")
                    .into_pointer_value())
            }
            Expr::Bool(b) => {
                let float_val = self
                    .context
                    .f64_type()
                    .const_float(if *b { 1.0 } else { 0.0 });
                Ok(self.box_number(float_val))
            }
            Expr::Nil => Ok(self.box_number(self.context.f64_type().const_float(0.0))),

            // Symbols become strings for now (TODO: add Symbol value type)
            Expr::Symbol(s) => {
                let c_str = self
                    .builder
                    .build_global_string_ptr(s, "quoted_symbol")
                    .expect("Failed to build global string for quoted symbol");
                Ok(self.box_string(c_str.as_pointer_value()))
            }

            // Vectors: recursively quote each element
            Expr::Vector(elements) => {
                let vec_empty_fn = self
                    .module
                    .get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let empty_vec_call = self
                    .builder
                    .build_call(vec_empty_fn, &[], "quoted_vec_empty")
                    .expect("Failed to build call to clorus_vector_empty");
                let mut vec_val = empty_vec_call
                    .try_as_basic_value()
                    .left()
                    .expect("clorus_vector_empty should return pointer")
                    .into_pointer_value();

                let vec_conj_fn = self
                    .module
                    .get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                for (i, elem) in elements.iter().enumerate() {
                    // Recursively quote each element
                    let elem_val = self.compile_quoted(elem)?;

                    let conj_call = self
                        .builder
                        .build_call(
                            vec_conj_fn,
                            &[vec_val.into(), elem_val.into()],
                            &format!("quoted_vec_conj_{}", i),
                        )
                        .expect("Failed to build call to clorus_vector_conj");

                    vec_val = conj_call
                        .try_as_basic_value()
                        .left()
                        .expect("clorus_vector_conj should return pointer")
                        .into_pointer_value();
                }

                Ok(vec_val)
            }

            // Lists are converted to vectors (since we don't have List literals at runtime yet)
            Expr::List(elements) => {
                let vec_empty_fn = self
                    .module
                    .get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let empty_vec_call = self
                    .builder
                    .build_call(vec_empty_fn, &[], "quoted_list_empty")
                    .unwrap();
                let mut vec_val = empty_vec_call
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let vec_conj_fn = self
                    .module
                    .get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                for (i, elem) in elements.iter().enumerate() {
                    let elem_val = self.compile_quoted(elem)?;

                    let conj_call = self
                        .builder
                        .build_call(
                            vec_conj_fn,
                            &[vec_val.into(), elem_val.into()],
                            &format!("quoted_list_conj_{}", i),
                        )
                        .unwrap();

                    vec_val = conj_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(vec_val)
            }

            // Maps: recursively quote keys and values
            Expr::Map(entries) => {
                let map_empty_fn = self
                    .module
                    .get_function("clorus_map_empty")
                    .ok_or("clorus_map_empty not declared")?;
                let empty_map_call = self
                    .builder
                    .build_call(map_empty_fn, &[], "quoted_map_empty")
                    .unwrap();
                let mut map_val = empty_map_call
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let map_assoc_fn = self
                    .module
                    .get_function("clorus_map_assoc")
                    .ok_or("clorus_map_assoc not declared")?;

                for (i, (key_expr, val_expr)) in entries.iter().enumerate() {
                    let key_val = self.compile_quoted(key_expr)?;
                    let val_val = self.compile_quoted(val_expr)?;

                    let assoc_call = self
                        .builder
                        .build_call(
                            map_assoc_fn,
                            &[map_val.into(), key_val.into(), val_val.into()],
                            &format!("quoted_map_assoc_{}", i),
                        )
                        .unwrap();

                    map_val = assoc_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(map_val)
            }

            // Sets are treated like vectors when quoted
            Expr::Set(elements) => {
                let vec_empty_fn = self
                    .module
                    .get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let empty_vec_call = self
                    .builder
                    .build_call(vec_empty_fn, &[], "quoted_set_empty")
                    .unwrap();
                let mut vec_val = empty_vec_call
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let vec_conj_fn = self
                    .module
                    .get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                for (i, elem) in elements.iter().enumerate() {
                    // Recursively quote each element
                    let elem_val = self.compile_quoted(elem)?;

                    let conj_call = self
                        .builder
                        .build_call(
                            vec_conj_fn,
                            &[vec_val.into(), elem_val.into()],
                            &format!("quoted_set_conj_{}", i),
                        )
                        .unwrap();

                    vec_val = conj_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(vec_val)
            }

            // Nested quotes: '(quote x) => just return (quote x) as data
            Expr::Quote { expr: inner } => {
                // Create a vector with symbol "quote" and the inner expression
                let vec_empty_fn = self
                    .module
                    .get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let vec_val = self
                    .builder
                    .build_call(vec_empty_fn, &[], "nested_quote_vec")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let vec_conj_fn = self
                    .module
                    .get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                // Add "quote" symbol as string
                let quote_str = self
                    .builder
                    .build_global_string_ptr("quote", "quote_symbol")
                    .unwrap();
                let quote_val = self.box_string(quote_str.as_pointer_value());
                let vec_with_quote = self
                    .builder
                    .build_call(
                        vec_conj_fn,
                        &[vec_val.into(), quote_val.into()],
                        "nested_quote_with_sym",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add the inner expression
                let inner_val = self.compile_quoted(inner)?;
                let final_vec = self
                    .builder
                    .build_call(
                        vec_conj_fn,
                        &[vec_with_quote.into(), inner_val.into()],
                        "nested_quote_final",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                Ok(final_vec)
            }

            // Calls in quoted form are data, not executed.
            // Represent them like quoted lists: ["func" arg1 arg2 ...]
            Expr::Call { func, args } => {
                let vec_empty_fn = self
                    .module
                    .get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let empty_vec_call = self
                    .builder
                    .build_call(vec_empty_fn, &[], "quoted_call_empty")
                    .expect("Failed to build call to clorus_vector_empty");
                let mut vec_val = empty_vec_call
                    .try_as_basic_value()
                    .left()
                    .expect("clorus_vector_empty should return pointer")
                    .into_pointer_value();

                let vec_conj_fn = self
                    .module
                    .get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                let func_sym = Expr::Symbol(func.clone());
                let func_val = self.compile_quoted(&func_sym)?;
                let func_conj = self
                    .builder
                    .build_call(
                        vec_conj_fn,
                        &[vec_val.into(), func_val.into()],
                        "quoted_call_conj_func",
                    )
                    .expect("Failed to build call to clorus_vector_conj");
                vec_val = func_conj
                    .try_as_basic_value()
                    .left()
                    .expect("clorus_vector_conj should return pointer")
                    .into_pointer_value();

                for (i, arg) in args.iter().enumerate() {
                    let arg_val = self.compile_quoted(arg)?;
                    let conj_call = self
                        .builder
                        .build_call(
                            vec_conj_fn,
                            &[vec_val.into(), arg_val.into()],
                            &format!("quoted_call_conj_arg_{}", i),
                        )
                        .expect("Failed to build call to clorus_vector_conj");
                    vec_val = conj_call
                        .try_as_basic_value()
                        .left()
                        .expect("clorus_vector_conj should return pointer")
                        .into_pointer_value();
                }

                Ok(vec_val)
            }

            // Other expression types that shouldn't appear in quotes
            _ => Err(format!("Cannot quote expression: {:?}", expr)),
        }
    }

    /// Compile syntax-quoted expression (backtick `)
    /// Like quote but evaluates unquoted (~) sections
    pub(super) fn compile_syntax_quoted(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
        match expr {
            // Unquote: evaluate the expression
            Expr::Unquote { expr: inner } => self.compile_expr(inner),

            // Unquote-splicing: evaluate and expect a sequence
            // For now, treat same as unquote (splicing happens in parent context)
            Expr::UnquoteSplicing { expr: inner } => self.compile_expr(inner),

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
                let c_str = self
                    .builder
                    .build_global_string_ptr(s, "syntax_quoted_symbol")
                    .unwrap();
                Ok(self.box_string(c_str.as_pointer_value()))
            }

            Expr::Keyword(k) => {
                let keyword_fn = self
                    .module
                    .get_function("clorus_keyword")
                    .ok_or("clorus_keyword not declared")?;
                let c_str = self.builder.build_global_string_ptr(k, "keyword").unwrap();
                let call_result = self
                    .builder
                    .build_call(
                        keyword_fn,
                        &[c_str.as_pointer_value().into()],
                        "keyword_value",
                    )
                    .unwrap();
                Ok(call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::Bool(b) => {
                let bool_fn = self
                    .module
                    .get_function("clorus_value_bool")
                    .ok_or("clorus_value_bool not declared")?;
                let bool_val = if *b { 1.0 } else { 0.0 };
                let float_val = self.context.f64_type().const_float(bool_val);
                let call_result = self
                    .builder
                    .build_call(bool_fn, &[float_val.into()], "bool_value")
                    .unwrap();
                Ok(call_result
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value())
            }

            Expr::Nil => Ok(self.box_number(self.context.f64_type().const_float(0.0))),

            // Collections: recursively process, checking for unquote-splicing
            Expr::Vector(elements) => self.compile_syntax_quoted_sequence(elements, true),

            Expr::List(elements) => {
                // Lists become vectors (like regular quote)
                self.compile_syntax_quoted_sequence(elements, true)
            }

            Expr::Set(elements) => {
                // Sets become vectors (like regular quote)
                self.compile_syntax_quoted_sequence(elements, true)
            }

            Expr::Map(entries) => {
                let map_empty_fn = self
                    .module
                    .get_function("clorus_map_empty")
                    .ok_or("clorus_map_empty not declared")?;
                let mut map_val = self
                    .builder
                    .build_call(map_empty_fn, &[], "sq_map_empty")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let map_assoc_fn = self
                    .module
                    .get_function("clorus_map_assoc")
                    .ok_or("clorus_map_assoc not declared")?;

                for (i, (key_expr, val_expr)) in entries.iter().enumerate() {
                    let key_val = self.compile_syntax_quoted(key_expr)?;
                    let val_val = self.compile_syntax_quoted(val_expr)?;

                    let assoc_call = self
                        .builder
                        .build_call(
                            map_assoc_fn,
                            &[map_val.into(), key_val.into(), val_val.into()],
                            &format!("sq_map_assoc_{}", i),
                        )
                        .unwrap();

                    map_val = assoc_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }

                Ok(map_val)
            }

            // Nested syntax-quote: treat as data
            Expr::SyntaxQuote { expr: inner } => {
                let vec_empty_fn = self
                    .module
                    .get_function("clorus_vector_empty")
                    .ok_or("clorus_vector_empty not declared")?;
                let vec_val = self
                    .builder
                    .build_call(vec_empty_fn, &[], "nested_sq_vec")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let vec_conj_fn = self
                    .module
                    .get_function("clorus_vector_conj")
                    .ok_or("clorus_vector_conj not declared")?;

                // Add "syntax-quote" symbol
                let sq_str = self
                    .builder
                    .build_global_string_ptr("syntax-quote", "sq_symbol")
                    .unwrap();
                let sq_val = self.box_string(sq_str.as_pointer_value());
                let vec_with_sq = self
                    .builder
                    .build_call(
                        vec_conj_fn,
                        &[vec_val.into(), sq_val.into()],
                        "nested_sq_with_sym",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                // Add the inner expression
                let inner_val = self.compile_syntax_quoted(inner)?;
                let final_vec = self
                    .builder
                    .build_call(
                        vec_conj_fn,
                        &[vec_with_sq.into(), inner_val.into()],
                        "nested_sq_final",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

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
    fn compile_syntax_quoted_sequence(
        &mut self,
        elements: &[Expr],
        _as_vector: bool,
    ) -> Result<PointerValue<'ctx>, String> {
        let vec_empty_fn = self
            .module
            .get_function("clorus_vector_empty")
            .ok_or("clorus_vector_empty not declared")?;
        let mut vec_val = self
            .builder
            .build_call(vec_empty_fn, &[], "sq_seq_empty")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let vec_conj_fn = self
            .module
            .get_function("clorus_vector_conj")
            .ok_or("clorus_vector_conj not declared")?;

        for (i, elem) in elements.iter().enumerate() {
            match elem {
                Expr::UnquoteSplicing { expr: inner } => {
                    // Evaluate the inner expression and splice its elements
                    // For now, just evaluate and add as-is (proper splicing needs iteration)
                    let spliced_val = self.compile_expr(inner)?;

                    // TODO: Iterate over spliced_val and add each element
                    // For MVP, just add the whole collection
                    let conj_call = self
                        .builder
                        .build_call(
                            vec_conj_fn,
                            &[vec_val.into(), spliced_val.into()],
                            &format!("sq_seq_splice_{}", i),
                        )
                        .unwrap();

                    vec_val = conj_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }
                _ => {
                    // Regular element: recursively syntax-quote
                    let elem_val = self.compile_syntax_quoted(elem)?;

                    let conj_call = self
                        .builder
                        .build_call(
                            vec_conj_fn,
                            &[vec_val.into(), elem_val.into()],
                            &format!("sq_seq_conj_{}", i),
                        )
                        .unwrap();

                    vec_val = conj_call
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();
                }
            }
        }

        Ok(vec_val)
    }

}
