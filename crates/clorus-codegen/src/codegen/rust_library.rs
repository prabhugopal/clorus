use super::*;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn rust_library_lookup_keys(name: &str) -> Vec<String> {
        let mut keys = Vec::new();
        let raw = name.to_string();
        let no_prefix = raw.strip_prefix("rust.").unwrap_or(&raw).to_string();
        let hyphen = no_prefix.replace('_', "-");
        let underscore = no_prefix.replace('-', "_");

        keys.push(raw.clone());
        keys.push(no_prefix.clone());
        keys.push(format!("rust.{}", no_prefix));

        if hyphen != no_prefix {
            keys.push(hyphen.clone());
            keys.push(format!("rust.{}", hyphen));
        }
        if underscore != no_prefix {
            keys.push(underscore.clone());
            keys.push(format!("rust.{}", underscore));
        }

        let mut unique = Vec::new();
        for key in keys {
            if !unique.contains(&key) {
                unique.push(key);
            }
        }
        unique
    }

    pub(super) fn resolve_rust_library(&self, name: &str) -> Option<RustLibrary> {
        for key in Self::rust_library_lookup_keys(name) {
            if let Some(lib) = self.rust_libraries.get(&key) {
                return Some(lib.clone());
            }
        }
        None
    }

    pub(super) fn rust_ffi_symbol_name(lib_name: &str, func_name: &str) -> String {
        fn normalize(raw: &str) -> String {
            raw.chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect()
        }

        format!("clorus_{}__{}", normalize(lib_name), normalize(func_name))
    }

    /// Register a Rust FFI library so its functions can be used
    pub fn register_rust_library(&mut self, lib: RustLibrary) {
        for key in Self::rust_library_lookup_keys(&lib.name) {
            self.rust_libraries.insert(key, lib.clone());
        }
    }

    /// Register a namespace as coming from a .clip package (Phase 4)
    /// This prevents the compiler from looking for source files for this namespace
    pub fn register_clip_namespace(&mut self, namespace: &str) {
        self.clip_namespaces.insert(namespace.to_string());
    }

    /// Check if a namespace is provided by a .clip package
    pub fn is_clip_namespace(&self, namespace: &str) -> bool {
        // Check if namespace starts with any registered .clip package name
        self.clip_namespaces
            .iter()
            .any(|clip_ns| namespace.starts_with(clip_ns))
    }

    /// Declare all functions from a Rust FFI library based on metadata
    pub fn declare_rust_library_functions(&mut self, lib: &RustLibrary) -> Result<(), String> {
        let f32_type = self.context.f32_type();
        let f64_type = self.context.f64_type();
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        for func in &lib.functions {
            // Convert parameter types
            let mut param_types = Vec::new();
            for param in &func.params {
                let llvm_type = match param.type_name.as_str() {
                    "String" => i8_ptr_type.into(),
                    "f32" => f32_type.into(),
                    "f64" => f64_type.into(),
                    "i32" | "u32" => i32_type.into(),
                    "i64" | "u64" | "isize" | "usize" => i64_type.into(),
                    "bool" => self.context.bool_type().into(),
                    "*mut u8" | "*const u8" => i8_ptr_type.into(), // Opaque pointers
                    other => return Err(format!("Unsupported parameter type in FFI: {}", other)),
                };
                param_types.push(llvm_type);
            }

            // Convert return type
            let return_type = match func.return_type.as_str() {
                "String" => i8_ptr_type.fn_type(&param_types, false),
                "f32" => f32_type.fn_type(&param_types, false),
                "f64" => f64_type.fn_type(&param_types, false),
                "i32" | "u32" => i32_type.fn_type(&param_types, false),
                "i64" | "u64" | "isize" | "usize" => i64_type.fn_type(&param_types, false),
                "bool" => self.context.bool_type().fn_type(&param_types, false),
                "()" => self.context.void_type().fn_type(&param_types, false),
                "*mut u8" | "*const u8" => i8_ptr_type.fn_type(&param_types, false), // Opaque pointers
                other => return Err(format!("Unsupported return type in FFI: {}", other)),
            };

            // Declare function with dependency-scoped prefix to avoid collisions with core runtime FFI symbols.
            let ffi_name = Self::rust_ffi_symbol_name(&lib.name, &func.name);
            self.module.add_function(&ffi_name, return_type, None);
        }

        Ok(())
    }

    /// Extract a raw pointer from a Value* (for opaque pointers)
    pub(super) fn extract_pointer_from_value(&self, value_ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        // Use runtime function to properly extract opaque pointer from Value*
        let extract_fn = self
            .module
            .get_function("clorus_extract_opaque_pointer")
            .expect(
                "clorus_extract_opaque_pointer not declared - runtime functions not initialized",
            );
        let call_result = self
            .builder
            .build_call(extract_fn, &[value_ptr.into()], "extract_ptr")
            .unwrap();
        call_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }

    /// Box a raw pointer into a Value* (for opaque pointers)
    pub(super) fn box_pointer(&self, ptr: PointerValue<'ctx>) -> PointerValue<'ctx> {
        // Use runtime function to properly create Value* with OpaquePointer tag
        let box_fn = self
            .module
            .get_function("clorus_value_opaque_pointer")
            .expect("clorus_value_opaque_pointer not declared - runtime functions not initialized");
        let call_result = self
            .builder
            .build_call(box_fn, &[ptr.into()], "box_ptr")
            .unwrap();
        call_result
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }
}
