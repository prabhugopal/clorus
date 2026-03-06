/// Clorus FFI Generator
///
/// Automatically generates FFI bindings for Rust functions
/// so they can be called from Clorus with zero wrapper code.
use std::fs;
use std::path::Path;
use syn::{parse_file, FnArg, Item, ItemFn, ReturnType, Type};

// Modern type-safe FFI analyzer
pub mod analyzer;

pub struct FfiGenerator {
    /// Functions found in the Rust source
    pub functions: Vec<FunctionInfo>,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
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
            if let Item::Fn(func) = item {
                // Only process public functions
                if self.is_public(&func) {
                    if let Some(info) = self.extract_function_info(&func) {
                        self.functions.push(info);
                    }
                }
            }
        }

        Ok(())
    }

    fn is_public(&self, func: &ItemFn) -> bool {
        matches!(func.vis, syn::Visibility::Public(_))
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
                    let type_name = self.type_to_string(&pat_type.ty);
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
            ReturnType::Type(_, ty) => self.type_to_string(ty),
        };

        Some(FunctionInfo {
            name,
            params,
            return_type,
        })
    }

    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Path(path) => path
                .path
                .segments
                .last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            Type::Ptr(ptr) => {
                // Handle pointer types: *mut T or *const T
                // We represent all pointers as "*mut u8" for FFI
                "*mut u8".to_string()
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
            "i32" => "i32".to_string(),
            "u32" => "u32".to_string(),
            "i64" => "i64".to_string(),
            "u64" => "u64".to_string(),
            "isize" => "i64".to_string(),
            "usize" => "u64".to_string(),
            "bool" => "bool".to_string(),
            "String" => "*mut c_char".to_string(), // CString::into_raw() returns *mut c_char
            "()" => "()".to_string(),
            "*mut u8" => "*mut u8".to_string(), // Pointer types pass through as-is
            _ => "*mut u8".to_string(),         // Generic pointer for complex types
        }
    }

    fn c_to_rust_conversion(&self, name: &str, rust_type: &str) -> String {
        match rust_type {
            "f32" | "f64" | "i32" | "u32" | "i64" | "u64" | "bool" | "*mut u8" => {
                format!("    let {}_rust = {};", name, name)
            }
            "isize" => format!("    let {}_rust = {} as isize;", name, name),
            "usize" => format!("    let {}_rust = {} as usize;", name, name),
            "String" => format!(
                "    let {}_rust = unsafe {{ CStr::from_ptr({} as *const c_char).to_string_lossy().to_string() }};",
                name, name
            ),
            _ => format!("    let {}_rust = {};", name, name),
        }
    }

    fn rust_to_c_conversion(&self, name: &str, rust_type: &str) -> String {
        match rust_type {
            "f32" | "f64" | "i32" | "u32" | "i64" | "u64" | "bool" | "*mut u8" => {
                format!("    {}", name)
            }
            "isize" => format!("    {} as i64", name),
            "usize" => format!("    {} as u64", name),
            "String" => format!(
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
                        "i32" => "self.context.i32_type().into()".to_string(),
                        "u32" => "self.context.i32_type().into()".to_string(),
                        "i64" => "self.context.i64_type().into()".to_string(),
                        "u64" => "self.context.i64_type().into()".to_string(),
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
                "i32" => "self.context.i32_type()",
                "u32" => "self.context.i32_type()",
                "i64" => "self.context.i64_type()",
                "u64" => "self.context.i64_type()",
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
}
