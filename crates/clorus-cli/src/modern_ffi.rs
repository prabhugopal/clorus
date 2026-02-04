/// Modern FFI wrapper generator using canonical type system
///
/// This generates FFI wrappers using FfiAnalyzer (type-safe) instead of
/// the old FfiGenerator (string-based types).

use clorus_ffi_gen::analyzer::FfiAnalyzer;
use clorus_types::{FfiFunction, FfiType};
use std::path::Path;
use std::fs;

pub struct ModernFfiWrapperGenerator {
    analyzer: FfiAnalyzer,
}

impl ModernFfiWrapperGenerator {
    pub fn new() -> Self {
        Self {
            analyzer: FfiAnalyzer::new(),
        }
    }

    /// Parse Rust source file and extract FFI functions
    pub fn parse_file(&mut self, path: &Path) -> Result<(), String> {
        self.analyzer.parse_file(path)
    }

    /// Get the analyzed functions
    pub fn functions(&self) -> &[FfiFunction] {
        &self.analyzer.functions
    }

    /// Generate C-compatible wrapper code with proper type handling
    pub fn generate_c_wrappers(&self) -> String {
        let mut code = String::new();

        code.push_str("// Auto-generated FFI wrappers using canonical type system\n");
        code.push_str("use std::ffi::{CStr, CString};\n");
        code.push_str("use std::os::raw::c_char;\n\n");

        for func in &self.analyzer.functions {
            code.push_str(&self.generate_wrapper(func));
            code.push_str("\n\n");
        }

        code
    }

    /// Generate a single FFI wrapper function
    fn generate_wrapper(&self, func: &FfiFunction) -> String {
        let wrapper_name = format!("clorus_{}", func.name);

        // Generate parameter list
        let c_params: Vec<String> = func.params.iter().map(|p| {
            let c_type = self.ffi_type_to_c_type(&p.ty);
            format!("{}: {}", p.name, c_type)
        }).collect();

        // Generate return type
        let c_return = self.ffi_type_to_c_type(&func.return_type);

        // Generate parameter conversions
        let param_conversions: Vec<String> = func.params.iter().map(|p| {
            self.c_to_rust_conversion(&p.name, &p.ty)
        }).collect();

        // Generate call parameters
        let call_params: Vec<String> = func.params.iter()
            .map(|p| format!("{}_rust", p.name))
            .collect();

        // Generate return conversion
        let return_conversion = self.rust_to_c_conversion("result", &func.return_type);

        // Add safety annotation if needed
        let safety_annotation = match func.safety {
            clorus_types::Safety::Unsafe => "// SAFETY: Caller must ensure pointer validity\n",
            clorus_types::Safety::Safe => "",
        };

        format!(
            "{}#[no_mangle]\npub extern \"C\" fn {}({}) -> {} {{\n{}\n    let result = {}({});\n{}\n}}",
            safety_annotation,
            wrapper_name,
            c_params.join(", "),
            c_return,
            param_conversions.join("\n    "),
            func.name,
            call_params.join(", "),
            return_conversion
        )
    }

    /// Convert FfiType to C type string
    fn ffi_type_to_c_type(&self, ffi_type: &FfiType) -> String {
        match ffi_type {
            FfiType::Void => "()".to_string(),
            FfiType::Bool => "bool".to_string(),
            FfiType::I64 => "i64".to_string(),
            FfiType::F64 => "f64".to_string(),
            FfiType::String => "*mut c_char".to_string(),
            FfiType::OpaquePointer { .. } => "*mut u8".to_string(),
            FfiType::Vector { .. } => "*mut u8".to_string(),
            FfiType::Option { .. } => "*mut u8".to_string(),
            FfiType::Result { .. } => "*mut u8".to_string(),
            FfiType::Struct { .. } => "*mut u8".to_string(),
            FfiType::Function { .. } => "*mut u8".to_string(),
        }
    }

    /// Generate C to Rust conversion code
    fn c_to_rust_conversion(&self, name: &str, ffi_type: &FfiType) -> String {
        match ffi_type {
            FfiType::F64 | FfiType::I64 | FfiType::Bool => {
                format!("    let {}_rust = {};", name, name)
            }
            FfiType::String => {
                format!(
                    "    let {}_rust = unsafe {{ CStr::from_ptr({} as *const c_char).to_string_lossy().to_string() }};",
                    name, name
                )
            }
            FfiType::OpaquePointer { .. } => {
                // Pointer passed through as-is
                format!("    let {}_rust = {};", name, name)
            }
            _ => {
                // Other complex types passed as opaque pointers
                format!("    let {}_rust = {};", name, name)
            }
        }
    }

    /// Generate Rust to C conversion code for return value
    fn rust_to_c_conversion(&self, name: &str, ffi_type: &FfiType) -> String {
        match ffi_type {
            FfiType::Void => "".to_string(),
            FfiType::F64 | FfiType::I64 | FfiType::Bool => {
                format!("    {}", name)
            }
            FfiType::String => {
                format!(
                    "    unsafe {{ CString::new({}).unwrap().into_raw() }}",
                    name
                )
            }
            FfiType::OpaquePointer { .. } => {
                // Cast pointer to *mut u8
                format!("    {} as *mut u8", name)
            }
            _ => {
                // Other complex types return as opaque pointers
                format!("    {} as *mut u8", name)
            }
        }
    }

    /// Generate JSON metadata for runtime
    pub fn generate_metadata_json(&self) -> Result<String, String> {
        self.analyzer.generate_metadata_json()
            .map_err(|e| format!("Failed to generate JSON: {}", e))
    }

    /// Save JSON metadata to file
    pub fn save_metadata_json(&self, output_path: &Path) -> Result<(), String> {
        let json = self.generate_metadata_json()?;
        fs::write(output_path, json)
            .map_err(|e| format!("Failed to write metadata: {}", e))
    }
}

impl Default for ModernFfiWrapperGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_generate_wrapper() {
        let temp_file = "/tmp/test_modern_wrapper.rs";
        let mut file = std::fs::File::create(temp_file).unwrap();
        file.write_all(b"pub fn move_to(ctx: *mut u8, x: f64, y: f64) { }").unwrap();

        let mut generator = ModernFfiWrapperGenerator::new();
        generator.parse_file(Path::new(temp_file)).unwrap();

        assert_eq!(generator.functions().len(), 1);

        let wrappers = generator.generate_c_wrappers();
        assert!(wrappers.contains("clorus_move_to"));
        assert!(wrappers.contains("ctx: *mut u8"));
        assert!(wrappers.contains("x: f64"));
        assert!(wrappers.contains("y: f64"));
    }

    #[test]
    fn test_generate_json_metadata() {
        let temp_file = "/tmp/test_json_metadata.rs";
        let mut file = std::fs::File::create(temp_file).unwrap();
        file.write_all(b"pub fn add(x: f64, y: f64) -> f64 { x + y }").unwrap();

        let mut generator = ModernFfiWrapperGenerator::new();
        generator.parse_file(Path::new(temp_file)).unwrap();

        let json = generator.generate_metadata_json().unwrap();
        assert!(json.contains("\"name\":\"add\""));
        assert!(json.contains("\"F64\""));
    }
}
