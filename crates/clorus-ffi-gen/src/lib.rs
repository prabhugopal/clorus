/// Clorus FFI Generator
///
/// Automatically generates FFI bindings for Rust functions
/// so they can be called from Clorus with zero wrapper code.
use std::fs;
use std::path::Path;
use quote::ToTokens;
use syn::{parse_file, FnArg, ImplItem, Item, ItemFn, ItemImpl, ReturnType, Type};

// Modern type-safe FFI analyzer
pub mod analyzer;

pub struct FfiGenerator {
    /// Functions found in the Rust source
    pub functions: Vec<FunctionInfo>,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    /// For a plain top-level free function, this is the function name
    /// itself, directly callable. For an impl-block method (associated
    /// function or instance method), this is `TypeName::method_name` --
    /// itself a valid Rust call expression via UFCS (`Type::method(recv,
    /// args)`), so no separate "call path" field is needed: `name` IS the
    /// call path in both cases.
    pub name: String,
    pub params: Vec<ParamInfo>,
    pub return_type: String,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    pub type_name: String,
}

impl FfiGenerator {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }

    /// Parse a Rust source file and extract public functions
    pub fn parse_file(&mut self, path: &Path) -> Result<(), String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        let ast = parse_file(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

        for item in ast.items {
            match item {
                Item::Fn(func) => {
                    // Only process public functions
                    if self.is_public(&func) {
                        if let Some(info) = self.extract_function_info(&func) {
                            self.functions.push(info);
                        }
                    }
                }
                Item::Impl(item_impl) => {
                    self.functions.extend(Self::extract_impl_methods(&item_impl));
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn is_public(&self, func: &ItemFn) -> bool {
        matches!(func.vis, syn::Visibility::Public(_))
    }

    /// Extracts associated functions and instance methods from an inherent
    /// `impl TypeName { ... }` block. Trait impls (`impl Trait for TypeName`)
    /// are intentionally NOT handled here: calling a trait method by UFCS
    /// path (`TypeName::method(...)`) generally needs the trait brought into
    /// scope, which requires resolving the trait's fully qualified path --
    /// real name resolution the analyzer doesn't do. Getting that wrong
    /// would silently generate a wrapper that fails to compile or, worse,
    /// resolves to the wrong trait; better to not auto-discover it at all.
    fn extract_impl_methods(item_impl: &ItemImpl) -> Vec<FunctionInfo> {
        // Trait impls are out of scope for this pass -- see doc comment above.
        if item_impl.trait_.is_some() {
            return Vec::new();
        }
        // Generic impls (`impl<T> Foo<T>`) aren't concretely callable across
        // an extern "C" boundary -- there's no single monomorphization to pick.
        if !item_impl.generics.params.is_empty() {
            return Vec::new();
        }

        let Type::Path(self_type_path) = &*item_impl.self_ty else {
            return Vec::new();
        };
        let Some(self_segment) = self_type_path.path.segments.last() else {
            return Vec::new();
        };
        // Generic Self type (e.g. `impl Foo<Bar>`) -- same reasoning as above.
        if !self_segment.arguments.is_empty() {
            return Vec::new();
        }
        let type_name = self_segment.ident.to_string();

        let mut methods = Vec::new();
        for impl_item in &item_impl.items {
            let ImplItem::Fn(method) = impl_item else {
                continue;
            };
            if !matches!(method.vis, syn::Visibility::Public(_)) {
                continue;
            }
            if method.sig.asyncness.is_some() {
                continue;
            }
            // Generic methods (`fn foo<T>(...)`, or a `impl Trait` param
            // desugared the same way) can't be called across an extern "C"
            // boundary without picking one concrete instantiation -- skip.
            if !method.sig.generics.params.is_empty() {
                continue;
            }

            let mut inputs = method.sig.inputs.iter();
            let mut params = Vec::new();

            if let Some(FnArg::Receiver(receiver)) = inputs.clone().next() {
                inputs.next();
                match &receiver.reference {
                    Some(_) => {
                        let prefix = if receiver.mutability.is_some() {
                            "&mut "
                        } else {
                            "&"
                        };
                        // Named "self_recv", not "self": the generated wrapper
                        // is a plain extern "C" fn, not a method, and Rust
                        // only allows a parameter literally named `self`
                        // inside impl/trait method signatures.
                        params.push(ParamInfo {
                            name: "self_recv".to_string(),
                            type_name: format!("{}{}", prefix, type_name),
                        });
                    }
                    // By-value `self` (consuming) is unsupported in this pass:
                    // Clorus opaque pointers have no ownership tracking yet, so
                    // there is no safe way to prevent the handle being used
                    // again after the value is moved into the method.
                    None => continue,
                }
            }

            let mut signature_ok = true;
            for input in inputs {
                if let FnArg::Typed(pat_type) = input {
                    if let syn::Pat::Ident(ident) = &*pat_type.pat {
                        let param_type = Self::type_to_string(&pat_type.ty);
                        params.push(ParamInfo {
                            name: ident.ident.to_string(),
                            type_name: param_type,
                        });
                        continue;
                    }
                }
                signature_ok = false;
                break;
            }
            if !signature_ok {
                continue;
            }

            let return_type = match &method.sig.output {
                ReturnType::Default => "()".to_string(),
                ReturnType::Type(_, ty) => Self::type_to_string(ty),
            };

            methods.push(FunctionInfo {
                name: format!("{}::{}", type_name, method.sig.ident),
                params,
                return_type,
            });
        }

        methods
    }

    fn extract_function_info(&self, func: &ItemFn) -> Option<FunctionInfo> {
        // Skip async functions - they return Futures, not concrete values
        // Users should provide blocking wrappers for async functions
        if func.sig.asyncness.is_some() {
            return None;
        }

        // Skip extern "C" functions - they're already C-compatible
        // and don't need wrapping
        if let Some(abi) = &func.sig.abi {
            if abi.name.as_ref().map(|n| n.value()) == Some("C".to_string()) {
                return None;
            }
        }

        let name = func.sig.ident.to_string();

        // Extract parameters
        let mut params = Vec::new();
        for input in &func.sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                if let syn::Pat::Ident(ident) = &*pat_type.pat {
                    let param_name = ident.ident.to_string();
                    let type_name = Self::type_to_string(&pat_type.ty);
                    params.push(ParamInfo {
                        name: param_name,
                        type_name,
                    });
                }
            }
        }

        // Extract return type
        let return_type = match &func.sig.output {
            ReturnType::Default => "()".to_string(),
            ReturnType::Type(_, ty) => Self::type_to_string(ty),
        };

        Some(FunctionInfo {
            name,
            params,
            return_type,
        })
    }

    fn type_to_string(ty: &Type) -> String {
        match ty {
            Type::Path(path) => {
                let Some(segment) = path.path.segments.last() else {
                    return "unknown".to_string();
                };
                let name = segment.ident.to_string();

                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    let type_args: Vec<&Type> = args
                        .args
                        .iter()
                        .filter_map(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(t),
                            _ => None,
                        })
                        .collect();

                    match (name.as_str(), type_args.as_slice()) {
                        ("Vec", [elem]) => {
                            return format!("Vec<{}>", Self::type_to_string(elem));
                        }
                        ("Option", [inner]) => {
                            return format!("Option<{}>", Self::type_to_string(inner));
                        }
                        ("Result", [ok, err]) => {
                            return format!(
                                "Result<{}, {}>",
                                Self::type_to_string(ok),
                                Self::type_to_string(err)
                            );
                        }
                        _ => {}
                    }
                }

                name
            }
            Type::Reference(reference) => {
                let pointee = reference
                    .elem
                    .to_token_stream()
                    .to_string()
                    .replace(' ', "");
                if reference.mutability.is_none() && pointee == "str" {
                    "&str".to_string()
                } else if pointee.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    // Opaque reference to a named type, e.g. &Counter / &mut Counter.
                    // Treated as a borrowed opaque pointer: the wrapper dereferences
                    // it back to a real Rust reference before calling the original fn.
                    let prefix = if reference.mutability.is_some() { "&mut " } else { "&" };
                    format!("{}{}", prefix, pointee)
                } else {
                    "unknown".to_string()
                }
            }
            Type::Ptr(ptr) => {
                // Preserve pointer mutability and pointee so unsupported pointer
                // signatures can be rejected with specific diagnostics upstream.
                let pointee = ptr.elem.to_token_stream().to_string().replace(' ', "");
                let ptr_prefix = if ptr.mutability.is_some() { "*mut " } else { "*const " };
                format!("{}{}", ptr_prefix, pointee)
            }
            _ => "unknown".to_string(),
        }
    }

    /// Generate C-compatible wrapper code
    pub fn generate_c_wrappers(&self) -> String {
        let mut code = String::new();

        code.push_str("// Auto-generated FFI wrappers\n");
        code.push_str("use std::ffi::{CStr, CString};\n");
        code.push_str("use std::os::raw::c_char;\n\n");

        for func in &self.functions {
            code.push_str(&self.generate_wrapper(func));
            code.push_str("\n\n");
        }

        code
    }

    fn generate_wrapper(&self, func: &FunctionInfo) -> String {
        let wrapper_name = format!("clorus_{}", func.name);

        // Convert parameters to C types
        let c_params: Vec<String> = func
            .params
            .iter()
            .map(|p| {
                let c_type = self.rust_to_c_type(&p.type_name);
                format!("{}: {}", p.name, c_type)
            })
            .collect();

        let c_return = self.rust_to_c_type(&func.return_type);

        // Generate wrapper body
        let param_conversions: Vec<String> = func
            .params
            .iter()
            .map(|p| self.c_to_rust_conversion(&p.name, &p.type_name))
            .collect();

        let call_params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}_rust", p.name))
            .collect();

        let return_conversion = self.rust_to_c_conversion("result", &func.return_type);

        format!(
            "#[no_mangle]\npub extern \"C\" fn {}({}) -> {} {{\n{}\n    let result = {}({});\n{}\n}}",
            wrapper_name,
            c_params.join(", "),
            c_return,
            param_conversions.join("\n    "),
            func.name,
            call_params.join(", "),
            return_conversion
        )
    }

    fn rust_to_c_type(&self, rust_type: &str) -> String {
        match rust_type {
            "f32" => "f32".to_string(),
            "f64" => "f64".to_string(),
            "i8" => "i8".to_string(),
            "u8" => "u8".to_string(),
            "i16" => "i16".to_string(),
            "u16" => "u16".to_string(),
            "i32" => "i32".to_string(),
            "u32" => "u32".to_string(),
            "i64" => "i64".to_string(),
            "u64" => "u64".to_string(),
            "isize" => "i64".to_string(),
            "usize" => "u64".to_string(),
            "bool" => "bool".to_string(),
            "String" => "*mut c_char".to_string(), // CString::into_raw() returns *mut c_char
            "&str" => "*mut c_char".to_string(),
            "()" => "()".to_string(),
            "*mut u8" => "*mut u8".to_string(),   // Pointer types pass through as-is
            "*const u8" => "*mut u8".to_string(), // C carrier for const raw pointer
            _ => "*mut u8".to_string(),         // Generic pointer for complex types
        }
    }

    fn c_to_rust_conversion(&self, name: &str, rust_type: &str) -> String {
        match rust_type {
            "f32"
            | "f64"
            | "i8"
            | "u8"
            | "i16"
            | "u16"
            | "i32"
            | "u32"
            | "i64"
            | "u64"
            | "bool"
            | "*mut u8" => {
                format!("    let {}_rust = {};", name, name)
            }
            "*const u8" => format!("    let {}_rust = {} as *const u8;", name, name),
            "isize" => format!("    let {}_rust = {} as isize;", name, name),
            "usize" => format!("    let {}_rust = {} as usize;", name, name),
            "String" => format!(
                "    let {}_rust = unsafe {{ CStr::from_ptr({} as *const c_char).to_string_lossy().to_string() }};",
                name, name
            ),
            "&str" => format!(
                "    let {}_rust_owned = unsafe {{ CStr::from_ptr({} as *const c_char).to_string_lossy().to_string() }};\n    let {}_rust = {}_rust_owned.as_str();",
                name, name, name, name
            ),
            _ => format!("    let {}_rust = {};", name, name),
        }
    }

    fn rust_to_c_conversion(&self, name: &str, rust_type: &str) -> String {
        match rust_type {
            "f32"
            | "f64"
            | "i8"
            | "u8"
            | "i16"
            | "u16"
            | "i32"
            | "u32"
            | "i64"
            | "u64"
            | "bool"
            | "*mut u8" => {
                format!("    {}", name)
            }
            "*const u8" => format!("    {} as *mut u8", name),
            "isize" => format!("    {} as i64", name),
            "usize" => format!("    {} as u64", name),
            "String" => format!(
                "    unsafe {{ CString::new({}).unwrap().into_raw() }}",
                name
            ),
            "&str" => format!(
                "    unsafe {{ CString::new({}).unwrap().into_raw() }}",
                name
            ),
            "()" => "".to_string(),
            _ => format!("    {} as *mut u8", name),
        }
    }

    /// Generate LLVM declarations for Clorus codegen
    pub fn generate_llvm_declarations(&self) -> String {
        let mut code = String::new();

        code.push_str("// LLVM function declarations for Clorus\n");
        code.push_str("// Add this method to your CodeGen impl:\n\n");
        code.push_str("fn declare_rust_ffi_functions(&mut self) {\n");
        code.push_str("    let f64_type = self.context.f64_type();\n");
        code.push_str(
            "    let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());\n\n",
        );

        for func in &self.functions {
            let wrapper_name = format!("clorus_{}", func.name);

            code.push_str(&format!("    // {}\n", func.name));

            // Generate parameter types
            let param_types: Vec<String> = func
                .params
                .iter()
                .map(|p| {
                    match p.type_name.as_str() {
                        "f32" => "self.context.f32_type().into()".to_string(),
                        "f64" => "f64_type.into()".to_string(),
                        "i8" => "self.context.i8_type().into()".to_string(),
                        "u8" => "self.context.i8_type().into()".to_string(),
                        "i16" => "self.context.i16_type().into()".to_string(),
                        "u16" => "self.context.i16_type().into()".to_string(),
                        "i32" => "self.context.i32_type().into()".to_string(),
                        "u32" => "self.context.i32_type().into()".to_string(),
                        "i64" => "self.context.i64_type().into()".to_string(),
                        "u64" => "self.context.i64_type().into()".to_string(),
                        "bool" => "self.context.bool_type().into()".to_string(),
                        "*mut u8" => "i8_ptr_type.into()".to_string(),
                        "*const u8" => "i8_ptr_type.into()".to_string(),
                        "isize" => "self.context.i64_type().into()".to_string(),
                        "usize" => "self.context.i64_type().into()".to_string(),
                        "String" => "i8_ptr_type.into()".to_string(),
                        _ => "i8_ptr_type.into()".to_string(), // Generic pointer for complex types
                    }
                })
                .collect();

            // Generate return type
            let return_type = match func.return_type.as_str() {
                "f32" => "self.context.f32_type()",
                "f64" => "f64_type",
                "i8" => "self.context.i8_type()",
                "u8" => "self.context.i8_type()",
                "i16" => "self.context.i16_type()",
                "u16" => "self.context.i16_type()",
                "i32" => "self.context.i32_type()",
                "u32" => "self.context.i32_type()",
                "i64" => "self.context.i64_type()",
                "u64" => "self.context.i64_type()",
                "bool" => "self.context.bool_type()",
                "*mut u8" => "i8_ptr_type",
                "*const u8" => "i8_ptr_type",
                "isize" => "self.context.i64_type()",
                "usize" => "self.context.i64_type()",
                "String" => "i8_ptr_type",
                "()" => "self.context.void_type()",
                _ => "i8_ptr_type",
            };

            if param_types.is_empty() {
                code.push_str(&format!(
                    "    let {}_type = {}.fn_type(&[], false);\n",
                    wrapper_name, return_type
                ));
            } else {
                code.push_str(&format!(
                    "    let {}_type = {}.fn_type(&[{}], false);\n",
                    wrapper_name,
                    return_type,
                    param_types.join(", ")
                ));
            }

            code.push_str(&format!(
                "    self.module.add_function(\"{}\", {}_type, None);\n\n",
                wrapper_name, wrapper_name
            ));
        }

        code.push_str("}\n");
        code
    }

    /// Save C wrappers to a file
    pub fn save_c_wrappers(&self, output_path: &Path) -> Result<(), String> {
        let wrappers = self.generate_c_wrappers();
        fs::write(output_path, wrappers).map_err(|e| {
            format!(
                "Failed to write wrappers to {}: {}",
                output_path.display(),
                e
            )
        })
    }

    /// Save LLVM declarations to a file
    pub fn save_llvm_declarations(&self, output_path: &Path) -> Result<(), String> {
        let declarations = self.generate_llvm_declarations();
        fs::write(output_path, declarations).map_err(|e| {
            format!(
                "Failed to write declarations to {}: {}",
                output_path.display(),
                e
            )
        })
    }

    /// Generate a complete Rust module file with both original functions and wrappers
    pub fn generate_ffi_module(&self, original_source: &Path) -> Result<String, String> {
        let mut module = String::new();

        // Add the original source
        let original_content = fs::read_to_string(original_source)
            .map_err(|e| format!("Failed to read {}: {}", original_source.display(), e))?;

        module.push_str(&original_content);
        module.push_str("\n\n");
        module.push_str("// Auto-generated FFI wrappers\n");
        module.push_str(&self.generate_c_wrappers());

        Ok(module)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_simple_function() {
        let temp_file = "/tmp/test_simple.rs";
        let mut file = std::fs::File::create(temp_file).unwrap();
        file.write_all(b"pub fn add(x: f64, y: f64) -> f64 { x + y }")
            .unwrap();

        let mut generator = FfiGenerator::new();
        generator.parse_file(Path::new(temp_file)).unwrap();

        assert_eq!(generator.functions.len(), 1);
        assert_eq!(generator.functions[0].name, "add");
        assert_eq!(generator.functions[0].params.len(), 2);
        assert_eq!(generator.functions[0].return_type, "f64");
    }

    #[test]
    fn test_parse_pointer_mutability_and_pointee_types() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/test_ptr_types_{}.rs", unique);
        let mut file = std::fs::File::create(&temp_file).unwrap();
        file.write_all(
            b"pub fn ptrs(a: *mut u8, b: *const u8, c: *mut i32, d: *const i64) -> *const u8 { b }",
        )
        .unwrap();

        let mut generator = FfiGenerator::new();
        generator.parse_file(Path::new(&temp_file)).unwrap();

        assert_eq!(generator.functions.len(), 1);
        let params = &generator.functions[0].params;
        assert_eq!(params[0].type_name, "*mut u8");
        assert_eq!(params[1].type_name, "*const u8");
        assert_eq!(params[2].type_name, "*mut i32");
        assert_eq!(params[3].type_name, "*const i64");
        assert_eq!(generator.functions[0].return_type, "*const u8");

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_generate_wrapper_supports_narrow_ints_and_const_pointer() {
        let mut generator = FfiGenerator::new();
        generator.functions.push(FunctionInfo {
            name: "demo".to_string(),
            params: vec![
                ParamInfo {
                    name: "a".to_string(),
                    type_name: "i8".to_string(),
                },
                ParamInfo {
                    name: "b".to_string(),
                    type_name: "u16".to_string(),
                },
                ParamInfo {
                    name: "p".to_string(),
                    type_name: "*const u8".to_string(),
                },
            ],
            return_type: "*const u8".to_string(),
        });

        let wrappers = generator.generate_c_wrappers();
        assert!(wrappers.contains("pub extern \"C\" fn clorus_demo(a: i8, b: u16, p: *mut u8) -> *mut u8"));
        assert!(wrappers.contains("let p_rust = p as *const u8;"));
        assert!(wrappers.contains("result as *mut u8"));
    }

    #[test]
    fn test_parse_borrowed_str_signature() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = format!("/tmp/test_borrowed_str_{}.rs", unique);
        let mut file = std::fs::File::create(&temp_file).unwrap();
        file.write_all(b"pub fn echo_str(s: &str) -> &str { s }").unwrap();

        let mut generator = FfiGenerator::new();
        generator.parse_file(Path::new(&temp_file)).unwrap();

        assert_eq!(generator.functions.len(), 1);
        assert_eq!(generator.functions[0].name, "echo_str");
        assert_eq!(generator.functions[0].params[0].type_name, "&str");
        assert_eq!(generator.functions[0].return_type, "&str");

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_generate_wrapper_supports_borrowed_str() {
        let mut generator = FfiGenerator::new();
        generator.functions.push(FunctionInfo {
            name: "echo_str".to_string(),
            params: vec![ParamInfo {
                name: "s".to_string(),
                type_name: "&str".to_string(),
            }],
            return_type: "&str".to_string(),
        });

        let wrappers = generator.generate_c_wrappers();
        assert!(wrappers.contains("pub extern \"C\" fn clorus_echo_str(s: *mut c_char) -> *mut c_char"));
        assert!(wrappers.contains("let s_rust_owned = unsafe { CStr::from_ptr(s as *const c_char).to_string_lossy().to_string() };"));
        assert!(wrappers.contains("let s_rust = s_rust_owned.as_str();"));
        assert!(wrappers.contains("unsafe { CString::new(result).unwrap().into_raw() }"));
    }
}
