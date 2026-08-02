use super::*;

impl<'ctx> CodeGen<'ctx> {
    /// Destructure a pattern and bind all variables
    /// Returns list of (var_name, alloca) for cleanup tracking
    pub(super) fn destructure_pattern(
        &mut self,
        pattern: &Pattern,
        value: PointerValue<'ctx>,
    ) -> Result<Vec<(String, PointerValue<'ctx>)>, String> {
        let mut bindings = Vec::new();

        match pattern {
            Pattern::Symbol(name) => {
                // Simple binding - store value to variable
                let alloca = self.create_entry_block_alloca(name);
                self.builder.build_store(alloca, value).unwrap();
                self.variables.insert(name.clone(), alloca);
                bindings.push((name.clone(), alloca));
            }

            Pattern::Ignore => {
                // No binding - just evaluate for side effects
            }

            Pattern::Vector {
                elements,
                rest,
                as_binding: _,
            } => {
                // Get clorus_vector_nth function
                let nth_fn = self
                    .module
                    .get_function("clorus_vector_nth")
                    .ok_or("clorus_vector_nth not declared")?;

                // Extract each element
                for (i, elem_pattern) in elements.iter().enumerate() {
                    let index = self.context.i64_type().const_int(i as u64, false);
                    let elem_val = self
                        .builder
                        .build_call(
                            nth_fn,
                            &[value.into(), index.into()],
                            &format!("vec_elem_{}", i),
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Recursively destructure
                    let nested = self.destructure_pattern(elem_pattern, elem_val)?;
                    bindings.extend(nested);
                }

                // Handle rest parameter
                if let Some(rest_name) = rest {
                    // Get clorus_vector_rest function (takes vector and start index)
                    let rest_fn = self
                        .module
                        .get_function("clorus_vector_rest")
                        .ok_or("clorus_vector_rest not declared")?;

                    let start_index = self
                        .context
                        .i64_type()
                        .const_int(elements.len() as u64, false);
                    let rest_val = self
                        .builder
                        .build_call(rest_fn, &[value.into(), start_index.into()], "vec_rest")
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Bind rest parameter
                    let alloca = self.create_entry_block_alloca(rest_name);
                    self.builder.build_store(alloca, rest_val).unwrap();
                    self.variables.insert(rest_name.clone(), alloca);
                    bindings.push((rest_name.clone(), alloca));
                }
            }

            Pattern::Map {
                bindings: pattern_bindings,
                defaults: _,
            } => {
                // Get map_get and keyword functions
                let get_fn = self
                    .module
                    .get_function("clorus_map_get")
                    .ok_or("clorus_map_get not declared")?;
                let keyword_fn = self
                    .module
                    .get_function("clorus_keyword")
                    .ok_or("clorus_keyword not declared")?;

                for (key, value_pattern) in pattern_bindings {
                    // Get key string
                    let key_str = match key {
                        MapPatternKey::Keyword(k) => k,
                        MapPatternKey::Symbol(s) => s,
                        MapPatternKey::Str(s) => s,
                        MapPatternKey::Sym(s) => s,
                    };

                    // Create keyword for lookup
                    let keyword_c_str = self
                        .builder
                        .build_global_string_ptr(key_str, "key")
                        .unwrap();
                    let keyword_val = self
                        .builder
                        .build_call(
                            keyword_fn,
                            &[keyword_c_str.as_pointer_value().into()],
                            "keyword",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Get value from map
                    let map_val = self
                        .builder
                        .build_call(
                            get_fn,
                            &[value.into(), keyword_val.into()],
                            &format!("map_get_{}", key_str),
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .left()
                        .unwrap()
                        .into_pointer_value();

                    // Recursively destructure
                    let nested = self.destructure_pattern(value_pattern, map_val)?;
                    bindings.extend(nested);
                }
            }
        }

        Ok(bindings)
    }

    /// Collect all variable names from a pattern (for loop recur)
    pub(super) fn collect_pattern_names(pattern: &Pattern) -> Vec<String> {
        match pattern {
            Pattern::Symbol(name) => vec![name.clone()],
            Pattern::Ignore => vec![],
            Pattern::Vector {
                elements,
                rest,
                as_binding: _,
            } => {
                let mut names = Vec::new();
                for elem in elements {
                    names.extend(Self::collect_pattern_names(elem));
                }
                if let Some(rest_name) = rest {
                    names.push(rest_name.clone());
                }
                names
            }
            Pattern::Map {
                bindings,
                defaults: _,
            } => {
                let mut names = Vec::new();
                for (_, value_pattern) in bindings {
                    names.extend(Self::collect_pattern_names(value_pattern));
                }
                names
            }
        }
    }

    /// Generate a record/type constructor function: ->TypeName
    /// Creates a function that takes field values and returns a map
    /// Shared by defrecord and deftype implementations
    pub(super) fn generate_record_constructor(
        &mut self,
        type_name: &str,
        fields: &[String],
    ) -> Result<(), String> {
        let constructor_name = format!("->{}", type_name);
        let value_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // Create parameter types (one Value* for each field)
        let param_types: Vec<_> = fields.iter().map(|_| value_ptr_type.into()).collect();

        let fn_type = value_ptr_type.fn_type(&param_types, false);
        let function = self.module.add_function(&constructor_name, fn_type, None);

        // Add function to table for calls
        self.functions.insert(constructor_name.clone(), function);

        // Save current state
        let saved_vars = self.variables.clone();
        let saved_block = self.builder.get_insert_block();

        // Create entry block for constructor
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);
        self.variables.clear();

        // Create empty map to hold record fields
        let map_empty_fn = self
            .module
            .get_function("clorus_map_empty")
            .ok_or("clorus_map_empty not declared")?;
        let mut map_val = self
            .builder
            .build_call(map_empty_fn, &[], "record_map")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Get map_assoc and keyword functions
        let map_assoc_fn = self
            .module
            .get_function("clorus_map_assoc")
            .ok_or("clorus_map_assoc not declared")?;
        let keyword_fn = self
            .module
            .get_function("clorus_keyword")
            .ok_or("clorus_keyword not declared")?;

        // For each field, add keyword->value pair to map
        for (i, field_name) in fields.iter().enumerate() {
            // Get parameter value for this field
            let param_val = function
                .get_nth_param(i as u32)
                .unwrap()
                .into_pointer_value();

            // Create keyword for field name
            let field_c_str = self
                .builder
                .build_global_string_ptr(field_name, "field_name")
                .unwrap();
            let keyword_val = self
                .builder
                .build_call(
                    keyword_fn,
                    &[field_c_str.as_pointer_value().into()],
                    &format!("field_keyword_{}", i),
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();

            // Add keyword->value pair to map
            map_val = self
                .builder
                .build_call(
                    map_assoc_fn,
                    &[map_val.into(), keyword_val.into(), param_val.into()],
                    &format!("record_map_{}", i),
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .unwrap()
                .into_pointer_value();
        }

        // Add __type__ field for runtime protocol dispatch
        // This lets the runtime identify what type an instance is
        let type_keyword_str = self
            .builder
            .build_global_string_ptr("__type__", "type_keyword_name")
            .unwrap();
        let type_keyword = self
            .builder
            .build_call(
                keyword_fn,
                &[type_keyword_str.as_pointer_value().into()],
                "type_keyword",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Create string value for type name
        let type_name_str = self
            .builder
            .build_global_string_ptr(type_name, "type_name_str")
            .unwrap();
        let string_fn = self
            .module
            .get_function("clorus_value_string")
            .ok_or("clorus_value_string not declared")?;
        let type_name_val = self
            .builder
            .build_call(
                string_fn,
                &[type_name_str.as_pointer_value().into()],
                "type_name_val",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Add __type__ -> "TypeName" to map
        map_val = self
            .builder
            .build_call(
                map_assoc_fn,
                &[map_val.into(), type_keyword.into(), type_name_val.into()],
                "record_with_type",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Return the constructed record (map)
        self.builder.build_return(Some(&map_val)).unwrap();

        // Restore previous state
        self.variables = saved_vars;
        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }

        Ok(())
    }
}
